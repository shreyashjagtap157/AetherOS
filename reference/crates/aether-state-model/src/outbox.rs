//! Deterministic host model of the durable-intent outbox protocol
//! (`PERSISTENCE-AND-EXTERNAL-EFFECTS.md` §4, steps 5–8).
//!
//! A transaction commits state mutations, an audit commitment, and *intents*
//! atomically. This module models the part after commit: a dispatcher offers
//! pending intents to a receiver; the receiver deduplicates processing by
//! stable operation id and returns evidence; the dispatcher records the
//! outcome durably in the state engine; reconciliation after a crash is the
//! same idempotent exchange.
//!
//! It is **not** a network stack, a driver, a payment rail, or an actuator.
//! The receiver is a host-side test double with deterministic controls
//! (`rejecting`, `crashing_after_send_on`). It models at-most-once delivery
//! with possible loss, at-least-once delivery with deduplication, and explicit
//! uncertainty per the document's contract; it never claims exactly-once.
//!
//! Honest boundaries (not enforced or modeled here):
//!
//! - **Capability confinement (§4 step 5)** is NOT enforced: any caller holding
//!   `&mut StateEngine` can dispatch. `aether-capspace` does not gate dispatch.
//! - **Retry classes and deadlines (§4 step 8)** are unimplemented: `EffectClass`
//!   is recorded but never read; `reconcile()` is an explicit caller-driven
//!   re-attempt, not a policy engine.
//! - **Irreversible-effect authorization interlock (§3 R class)** is absent;
//!   an `Irreversible` intent takes the same path as a deduplicated one.
//! - **Receiver dedup durability** is session-scoped: the double's `seen` set
//!   lives in memory. Real external receivers dedup on their own durable
//!   state.
//! - **CompensationRequired** is recorded by the engine but the dispatcher
//!   never produces it (no compensation dispatch exists).
//! - If `record_effect` fails during a dispatch pass, the receiver may have
//!   processed operations whose outcomes are not yet durable; recovery re-offers
//!   only `Pending` intents, which the receiver then deduplicates to
//!   `Acknowledged`. Callers should re-dispatch after an engine error.

use crate::{EffectStatus, OperationId, StateEngine, StateError};
use std::collections::HashSet;

/// Outcome of one delivery attempt by the receiver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Deliver {
    /// Receiver accepted and processed the operation (fresh).
    Delivered,
    /// Receiver already processed this operation id; deduplicated.
    Duplicate,
    /// Receiver conclusively rejected the operation.
    Rejected,
    /// Receiver applied the operation but the acknowledgement was lost
    /// (simulated crash after send). Outcome cannot currently be determined.
    Uncertain,
}

/// Host-side double of the external receiver.
///
/// Retains the operation ids it has processed so duplicate deliveries are
/// deduplicated deterministically. `crashing_after_send_on` simulates the
/// receiver applying the operation and then losing the acknowledgement —
/// the classic uncertain-outcome injection.
#[derive(Default)]
pub struct Receiver {
    seen: HashSet<OperationId>,
    reject: HashSet<OperationId>,
    crash_after_send_on: Option<OperationId>,
    crashed: bool,
}

impl Receiver {
    /// Creates an empty receiver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares operations the receiver will conclusively reject.
    #[must_use]
    pub fn rejecting(mut self, operation: OperationId) -> Self {
        self.reject.insert(operation);
        self
    }

    /// Declares the operation whose processing will apply but whose
    /// acknowledgement will be lost (crash after send).
    #[must_use]
    pub fn crashing_after_send_on(mut self, operation: OperationId) -> Self {
        self.crash_after_send_on = Some(operation);
        self
    }

    /// Whether the receiver crashed this session (informational only).
    #[must_use]
    pub const fn crashed(&self) -> bool {
        self.crashed
    }

    /// Delivers one intent offering, applying the dedup contract.
    ///
    /// Crash injection is one-shot: the declared crash fires only when the
    /// declared operation is delivered, and not before. A rejecting operation
    /// is rejected identically on redelivery (idempotent rejection); the
    /// dedup memory `seen` records processing, not outcome eligibility.
    pub fn deliver(&mut self, operation: OperationId) -> Deliver {
        if self.crash_after_send_on == Some(operation) {
            self.crash_after_send_on = None;
            self.seen.insert(operation);
            self.crashed = true;
            return Deliver::Uncertain;
        }
        if self.reject.contains(&operation) {
            self.seen.insert(operation);
            return Deliver::Rejected;
        }
        if !self.seen.insert(operation) {
            return Deliver::Duplicate;
        }
        Deliver::Delivered
    }
}

/// One dispatcher for one intent outbox.
///
/// "Capability-confined" is the architecture's target shape, not enforced
/// here: any caller holding `&mut StateEngine` may dispatch.
#[derive(Default)]
pub struct OutboxDispatcher {
    receiver: Receiver,
}

impl OutboxDispatcher {
    /// Creates the dispatcher around one receiver.
    #[must_use]
    pub fn new(receiver: Receiver) -> Self {
        Self { receiver }
    }

    /// Consumes every pending operation in `engine`: delivers each to the
    /// receiver, records the outcome durably in the engine, and returns a
    /// per-call accounting.
    ///
    /// # Errors
    /// Returns [`StateError`] only on an engine failure while recording an
    /// outcome; the receiver side is infallible in this model.
    pub fn dispatch(&mut self, engine: &mut StateEngine) -> Result<DispatchReport, StateError> {
        let mut report = DispatchReport::default();
        for (operation, _) in engine.pending_effects() {
            match self.receiver.deliver(operation) {
                Deliver::Delivered => {
                    engine.record_effect(operation, EffectStatus::Acknowledged)?;
                    report.delivered = report.delivered.saturating_add(1);
                }
                Deliver::Duplicate => {
                    engine.record_effect(operation, EffectStatus::Acknowledged)?;
                    report.deduplicated = report.deduplicated.saturating_add(1);
                }
                Deliver::Rejected => {
                    engine.record_effect(operation, EffectStatus::Rejected)?;
                    report.rejected = report.rejected.saturating_add(1);
                }
                Deliver::Uncertain => {
                    engine.record_effect(operation, EffectStatus::Uncertain)?;
                    report.uncertain = report.uncertain.saturating_add(1);
                }
            }
        }
        Ok(report)
    }

    /// Re-attempts every operation whose outcome is uncertain. Outcomes from
    /// earlier processes may precede this call; redelivery of an uncertain
    /// operation is deduplicated by the receiver. The caller decides when to
    /// reconcile.
    ///
    /// # Errors
    /// Returns [`StateError`] only on an engine failure while recording.
    pub fn reconcile(&mut self, engine: &mut StateEngine) -> Result<DispatchReport, StateError> {
        let mut report = DispatchReport::default();
        for (operation, _) in engine.uncertain_effects() {
            match self.receiver.deliver(operation) {
                Deliver::Delivered | Deliver::Duplicate => {
                    engine.record_effect(operation, EffectStatus::Acknowledged)?;
                    report.acknowledged = report.acknowledged.saturating_add(1);
                }
                Deliver::Rejected => {
                    engine.record_effect(operation, EffectStatus::Rejected)?;
                    report.rejected = report.rejected.saturating_add(1);
                }
                Deliver::Uncertain => {
                    // Still uncertain: leave as Uncertain and report.
                    report.uncertain = report.uncertain.saturating_add(1);
                }
            }
        }
        Ok(report)
    }
}

/// Per-call accounting for a dispatch or reconciliation pass.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DispatchReport {
    /// Fresh deliveries accepted (delivered count).
    pub delivered: u32,
    /// Deliveries deduplicated as already seen (dedup count).
    pub deduplicated: u32,
    /// Operations conclusively rejected.
    pub rejected: u32,
    /// Operations left uncertain (crash after send before acknowledgement).
    pub uncertain: u32,
    /// Uncertain operations resolved to acknowledged during reconciliation.
    pub acknowledged: u32,
}
