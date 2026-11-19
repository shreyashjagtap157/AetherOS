//! Deterministic crash-boundary fault injection (PERSISTENCE-AND-EXTERNAL-EFFECTS.md §6).
//!
//! A crash at any persistence boundary is indistinguishable from truncating
//! the durable log at some byte offset: a frame either fully exists or it
//! does not (or is torn). This module sweeps *every* byte cut of a durable
//! log and reports every violated crash-consistency invariant. It is
//! deterministic, exhaustive at the byte granularity, and does not read
//! clocks, files, or environment.
//!
//! It is a verification harness, not part of the durable format and not a
//! storage provider. It complements (and does not replace) hand-written
//! fault-injection cases.
//!
//! Invariants checked, for the sequence of cuts in ascending byte order:
//!
//! - Recovery must succeed at every cut (a complete corrupt frame must not
//!   be silently accepted, and a torn tail must be ignored).
//! - Objects visible at one cut remain visible at every later cut, and
//!   versions never regress.
//! - An effect whose status is terminal (`Acknowledged`, `Rejected`,
//!   `CompensationRequired`) remains so identically; a recorded effect can
//!   never be deleted or downgraded by a crash.
//!
//! Note on reachability: `MissingObject` and `EffectRegressed` cannot fire
//! on a log this engine can recover (visible state and effects are
//! append-only in the reference model; contradicting effect frames fail
//! `valid_transition` at recovery instead). Both arms are kept as
//! defense-in-depth against a future engine change that mis-accepts a torn
//! or hand-forged frame, and are deliberately untested for their unreachable
//! shape. `VersionRegressed` *can* fire: the engine intentionally does not
//! enforce version monotonicity, so the sweep flags such logs.

use crate::{EffectStatus, OperationId, StateEngine};
use aether_types::ObjectId;

/// One violated crash-consistency invariant found at one cut.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrashViolation {
    /// Recovery reported an error at this byte cut.
    RecoverFailed(usize),
    /// A version visible at an earlier cut is different (smaller) here;
    /// a commit was silently rolled back by a crash.
    VersionRegressed(usize, ObjectId),
    /// An object visible at an earlier cut is absent here; a commit
    /// vanished from history without a trace.
    MissingObject(usize, ObjectId),
    /// An effect in a terminal state at an earlier cut is absent or in a
    /// different terminal state here.
    EffectRegressed(usize, OperationId),
}

/// Sweep every byte cut of a durable log and surface violations.
pub struct CrashSweep {
    log: Vec<u8>,
}

impl CrashSweep {
    /// Creates the sweep for one durable log.
    #[must_use]
    pub const fn new(log: Vec<u8>) -> Self {
        Self { log }
    }

    /// Returns the violated invariants across every cut, in ascending byte
    /// order. An empty vector means the log is crash-consistent at every
    /// byte boundary.
    #[must_use]
    pub fn violations(&self) -> Vec<CrashViolation> {
        let mut violations = Vec::new();
        let mut previous_objects: Vec<(ObjectId, crate::VersionId)> = Vec::new();
        let mut previous_effects: Vec<(OperationId, EffectStatus)> = Vec::new();

        for cut in 0..=self.log.len() {
            let Ok(recovered) = StateEngine::recover(&self.log[..cut]) else {
                violations.push(CrashViolation::RecoverFailed(cut));
                continue;
            };

            for (object, version) in &previous_objects {
                match recovered.visible_version(*object) {
                    Some(current) if current.0 < version.0 => {
                        violations.push(CrashViolation::VersionRegressed(cut, *object));
                    }
                    None => violations.push(CrashViolation::MissingObject(cut, *object)),
                    Some(_) => {}
                }
            }
            for (operation, status) in &previous_effects {
                if is_terminal(*status) && recovered.effect_status(*operation) != Some(*status) {
                    violations.push(CrashViolation::EffectRegressed(cut, *operation));
                }
            }

            previous_objects = recovered.objects();
            previous_effects = recovered.operations();
        }
        violations
    }
}

fn is_terminal(status: EffectStatus) -> bool {
    matches!(
        status,
        EffectStatus::Acknowledged | EffectStatus::Rejected | EffectStatus::CompensationRequired
    )
}
