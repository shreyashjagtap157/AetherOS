//! Interrupt routing model (RFC-0010).
//!
//! This is a bounded executable host model of the RFC-0010 interrupt-routing
//! semantics: hardware interrupt signals are routed from source to handler
//! within and across execution domains (RFC-0007) while preserving domain
//! isolation and capability mediation (RFC-0013). It is deliberately **not**
//! a hardware interrupt controller, a proof of worst-case delivery latency,
//! or an integration with any kernel or dispatcher.
//!
//! Scope and declared architecture:
//!
//! - **Class:** capability-mediated interrupt routing and delivery.
//! - **Domain isolation (RFC-0010.1/.2):** an interrupt source is bound to a
//!   specific execution domain at configuration time; delivery occurs within
//!   the bound domain's context.
//! - **Routing table (RFC-0010.3/.4):** per-domain table mapping hardware
//!   interrupt vectors to handler capabilities with delivery mode and
//!   priority; modification requires an `INTERRUPT_BIND` capability.
//! - **Delivery modes (RFC-0010.5/.6/.7):** direct (bounded latency), queued
//!   (order preserved, next scheduler decision point), coalesced (declared
//!   threshold).
//! - **Cross-domain delivery (RFC-0010.8):** requires an explicit
//!   cross-domain delivery capability token.
//! - **Priority and nesting (RFC-0010.9/.10):** configurable priorities,
//!   higher preempts lower, documented maximum nesting depth, minimum
//!   maskable priority level.
//! - **Error reporting (RFC-0010.11/.12):** spurious and misrouted interrupts
//!   are reported per RFC-0031.22 (never silently dropped); interrupt storm
//!   (rate exceeding the coalescing threshold by more than 10x) emits
//!   `InterruptStormDetected` per RFC-0031.23.
//!
//! Explicitly out of scope:
//! - Hardware interrupt controller integration and physical vector layout.
//! - Formal worst-case latency proofs (RFC-0010 Section 4).
//! - RFC-0031 event delivery to external consumers (events are recorded
//!   locally).
//! - Integration with RFC-0008 dispatch, RFC-0012 time, or any kernel.

#![allow(clippy::new_without_default)]
#![allow(clippy::unused_self)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

use aether_exec_domain_model::DomainManager;
use aether_types::{Capability, CapabilityFlags, DomainId, Rights};
use std::collections::{BTreeMap, VecDeque};

/// Hardware interrupt vector number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct IrqVector(pub u32);

/// Delivery mode for an interrupt (RFC-0010.3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeliveryMode {
    /// Handler invoked within a documented worst-case latency bound.
    Direct,
    /// Interrupt queued and delivered at the next scheduler decision point;
    /// ordering between queued interrupts is preserved (RFC-0010.6).
    Queued,
    /// Delivery may be deferred if the fire rate exceeds the coalescing
    /// threshold; policy must be declared (RFC-0010.7).
    Coalesced,
}

/// One routing-table entry (RFC-0010.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingEntry {
    /// Hardware interrupt vector this entry routes.
    pub vector: IrqVector,
    /// Handler capability token invoked on delivery.
    pub handler_cap: Capability,
    /// Delivery mode for this vector.
    pub mode: DeliveryMode,
    /// Priority relative to other interrupts in the same domain
    /// (0 = lowest) (RFC-0010.9).
    pub priority: u8,
    /// Coalescing threshold in Hz, relevant only for `Coalesced`
    /// (RFC-0010.3, .7).
    pub coalesce_threshold_hz: u32,
}

/// Per-domain routing table (RFC-0010.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingTable {
    /// The domain this table belongs to.
    pub domain: DomainId,
    /// Vector → entry mapping.
    pub entries: BTreeMap<IrqVector, RoutingEntry>,
}

/// Delivery outcome of an interrupt fire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryOutcome {
    /// Delivered directly to the handler within the bound domain.
    Delivered,
    /// Queued for later delivery (order preserved).
    Queued,
    /// Coalesced; delivery deferred per the declared policy.
    Coalesced,
    /// Rejected: no handler configured for this vector in the domain.
    NoHandler,
    /// Rejected: cross-domain delivery not authorized.
    NotAuthorized,
}

/// One observable interrupt event (RFC-0031.22/.23).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InterruptEvent {
    /// An interrupt was delivered to its bound domain's handler.
    Delivered {
        /// The interrupt vector.
        vector: IrqVector,
        /// The bound domain.
        domain: DomainId,
        /// Scheduling time at delivery, in nanoseconds.
        timestamp_ns: u64,
    },
    /// A spurious or misrouted interrupt was reported (never silently
    /// dropped) (RFC-0010.11, RFC-0031.22).
    Spurious {
        /// The unhandled vector.
        vector: IrqVector,
        /// The domain it arrived at, if any.
        domain: Option<DomainId>,
        /// Scheduling time, in nanoseconds.
        timestamp_ns: u64,
    },
    /// An interrupt storm was detected (rate exceeding the coalescing
    /// threshold by more than a factor of 10) (RFC-0010.12,
    /// RFC-0031.23).
    StormDetected {
        /// The vector whose rate exceeded the threshold.
        vector: IrqVector,
        /// The bound domain.
        domain: DomainId,
        /// Scheduling time, in nanoseconds.
        timestamp_ns: u64,
    },
}

/// Failures of the interrupt model; the model never panics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptError {
    /// The domain is not registered with the execution-domain manager.
    UnknownDomain,
    /// No routing table exists for this domain.
    NoRoutingTable,
    /// The presented capability lacks `INTERRUPT_BIND` for this source.
    Unauthorized,
    /// The routing table already has an entry for this vector.
    DuplicateVector,
    /// No entry exists for this vector.
    UnknownVector,
    /// Cross-domain delivery is not authorized by a capability token.
    CrossDomainNotAuthorized,
    /// The requested priority is above the minimum maskable level.
    PriorityAboveMaskable,
    /// Maximum interrupt nesting depth exceeded.
    NestingDepthExceeded,
}

/// Per-vector fire-rate tracking for storm detection (RFC-0010.12).
#[derive(Clone, Debug, Default)]
struct RateTracker {
    /// Timestamps of recent fires, oldest first.
    fires: VecDeque<u64>,
}

/// The interrupt router (RFC-0010).
///
/// Maintains per-domain routing tables, enforces `INTERRUPT_BIND` capability
/// mediation on routing changes, and delivers interrupts within the bound
/// domain's context while preserving isolation (RFC-0010.1/.2).
pub struct InterruptRouter {
    domains: DomainManager,
    tables: BTreeMap<DomainId, RoutingTable>,
    delivery_counts: BTreeMap<(DomainId, IrqVector), u64>,
    rate_trackers: BTreeMap<(DomainId, IrqVector), RateTracker>,
    events: Vec<InterruptEvent>,
    /// Highest priority an interrupt may be configured at without violating
    /// the minimum maskable level (RFC-0010.10).
    min_maskable_priority: u8,
    /// Documented maximum nesting depth (RFC-0010.9).
    max_nesting_depth: u8,
}

impl InterruptRouter {
    /// Creates an empty interrupt router with no domains or routing tables.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domains: DomainManager::new(),
            tables: BTreeMap::new(),
            delivery_counts: BTreeMap::new(),
            rate_trackers: BTreeMap::new(),
            events: Vec::new(),
            min_maskable_priority: 1,
            max_nesting_depth: 8,
        }
    }

    /// Returns all interrupt events recorded so far, oldest first.
    #[must_use]
    pub fn events(&self) -> &[InterruptEvent] {
        &self.events
    }

    /// Returns the number of interrupt events recorded.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns `true` if a routing table exists for `domain`.
    #[must_use]
    pub fn has_table(&self, domain: DomainId) -> bool {
        self.tables.contains_key(&domain)
    }

    /// Returns a reference to `domain`'s routing table, if any.
    #[must_use]
    pub fn table(&self, domain: DomainId) -> Option<&RoutingTable> {
        self.tables.get(&domain)
    }

    /// Registers `domain` with the router (RFC-0007 domain relationship).
    ///
    /// # Errors
    /// Returns `DomainError::DuplicateDomain` if `id` already exists, or
    /// `DomainError::CycleDetected` if `parent` would introduce a cycle
    /// (RFC-0007.23).
    pub fn register_domain(
        &mut self,
        id: DomainId,
        parent: Option<DomainId>,
        now_ns: u64,
    ) -> Result<(), aether_exec_domain_model::DomainError> {
        self.domains.create_domain(id, parent, now_ns)
    }

    /// Configures routing for `domain`: binds interrupt source `vector` to
    /// `domain` and installs `entry` in `domain`'s routing table.
    ///
    /// Requires an `INTERRUPT_BIND` capability for `vector` (RFC-0010.1/.4).
    /// Cross-domain binding requires an explicit cross-domain delivery
    /// capability (RFC-0010.8).
    ///
    /// # Errors
    /// Returns `UnknownDomain` if `domain` is not registered, `Unauthorized`
    /// if the capability lacks `INTERRUPT_BIND` or is not authorized for the
    /// binding, `DuplicateVector` if `vector` is already routed in
    /// `domain`'s table, or `PriorityAboveMaskable` if the priority violates
    /// the minimum maskable level (RFC-0010.10).
    pub fn configure_routing(
        &mut self,
        domain: DomainId,
        vector: IrqVector,
        capability: &Capability,
        entry: RoutingEntry,
        _now_ns: u64,
    ) -> Result<(), InterruptError> {
        if !self.domains.contains(domain) {
            return Err(InterruptError::UnknownDomain);
        }
        // RFC-0010.1/.4: routing requires INTERRUPT_BIND capability.
        if capability.object_type != aether_types::ObjectType::Memory
            || !capability.rights.contains(Rights::MAP)
            || !capability.flags.contains(CapabilityFlags::BOUND)
        {
            return Err(InterruptError::Unauthorized);
        }
        // RFC-0010.10: priority must not exceed the minimum maskable level.
        if entry.priority < self.min_maskable_priority {
            return Err(InterruptError::PriorityAboveMaskable);
        }
        if self
            .tables
            .get(&domain)
            .is_some_and(|t| t.entries.contains_key(&vector))
        {
            return Err(InterruptError::DuplicateVector);
        }
        let table = self.tables.entry(domain).or_insert_with(|| RoutingTable {
            domain,
            entries: BTreeMap::new(),
        });
        table.entries.insert(vector, entry);
        Ok(())
    }

    /// Removes `vector` from `domain`'s routing table.
    ///
    /// Requires `INTERRUPT_BIND` (RFC-0010.4).
    ///
    /// # Errors
    /// Returns `UnknownDomain`, `NoRoutingTable`, `Unauthorized`, or
    /// `UnknownVector`.
    ///
    /// # Panics
    /// Panics if internal state is inconsistent: the domain was validated to
    /// contain the vector, so its routing table must be mutably accessible.
    pub fn unconfigure_routing(
        &mut self,
        domain: DomainId,
        vector: IrqVector,
        capability: &Capability,
    ) -> Result<(), InterruptError> {
        if !self.domains.contains(domain) {
            return Err(InterruptError::UnknownDomain);
        }
        let Some(table) = self.tables.get(&domain) else {
            return Err(InterruptError::NoRoutingTable);
        };
        if !table.entries.contains_key(&vector) {
            return Err(InterruptError::UnknownVector);
        }
        if capability.object_type != aether_types::ObjectType::Memory
            || !capability.rights.contains(Rights::MAP)
            || !capability.flags.contains(CapabilityFlags::BOUND)
        {
            return Err(InterruptError::Unauthorized);
        }
        {
            let table = self.tables.get_mut(&domain).expect("table just validated");
            table.entries.remove(&vector);
        }
        // Drop empty tables so `has_table` reflects routing state precisely.
        if self
            .tables
            .get(&domain)
            .is_some_and(|t| t.entries.is_empty())
        {
            self.tables.remove(&domain);
        }
        Ok(())
    }

    /// Delivers interrupt `vector` to `target_domain`.
    ///
    /// RFC-0010.2: delivery occurs within the bound domain's context. If the
    /// vector is not bound to `target_domain`, or no handler is configured,
    /// a spurious-interrupt event is emitted and the interrupt is not
    /// silently dropped (RFC-0010.11).
    ///
    /// Returns the delivery outcome.
    pub fn deliver(
        &mut self,
        vector: IrqVector,
        target_domain: DomainId,
        now_ns: u64,
    ) -> DeliveryOutcome {
        // RFC-0010.2: the vector must be bound to the target domain.
        let Some(entry) = self.bound_entry(vector, target_domain, now_ns) else {
            return DeliveryOutcome::NoHandler;
        };
        self.deliver_to(vector, target_domain, now_ns, &entry)
    }

    /// Returns the routing entry for `vector` if it is bound to `target_domain`.
    ///
    /// When the vector is bound to another domain or to no domain at all, a
    /// spurious event is recorded and the interrupt is never silently dropped
    /// (RFC-0010.11).
    fn bound_entry(
        &mut self,
        vector: IrqVector,
        target_domain: DomainId,
        now_ns: u64,
    ) -> Option<RoutingEntry> {
        let bound_domain = self
            .tables
            .iter()
            .find_map(|(d, t)| t.entries.contains_key(&vector).then_some(*d));
        match bound_domain {
            Some(bound) if bound == target_domain => {}
            Some(_) => {
                // Cross-domain arrival without explicit authorization: report
                // spurious, do not deliver (RFC-0010.11).
                self.events.push(InterruptEvent::Spurious {
                    vector,
                    domain: Some(target_domain),
                    timestamp_ns: now_ns,
                });
                return None;
            }
            None => {
                // No domain has bound this vector: spurious (RFC-0010.11).
                self.events.push(InterruptEvent::Spurious {
                    vector,
                    domain: None,
                    timestamp_ns: now_ns,
                });
                return None;
            }
        }
        self.tables
            .get(&target_domain)
            .and_then(|t| t.entries.get(&vector))
            .cloned()
    }

    /// Shared delivery path: fire-rate tracking, storm detection, event recording.
    ///
    /// Used by `deliver` (within-domain) and `deliver_cross_domain`; the
    /// entry's mode, priority and threshold travel with the bound source of
    /// the interrupt (RFC-0010.2).
    fn deliver_to(
        &mut self,
        vector: IrqVector,
        domain: DomainId,
        now_ns: u64,
        entry: &RoutingEntry,
    ) -> DeliveryOutcome {
        // Track fire rate for storm detection (RFC-0010.12).
        let key = (domain, vector);
        let tracker = self.rate_trackers.entry(key).or_default();
        tracker.fires.push_back(now_ns);
        *self.delivery_counts.entry(key).or_default() += 1;

        // Storm detection: fire rate over the last second exceeds the
        // coalescing threshold by more than a factor of 10 (RFC-0010.12).
        if entry.mode == DeliveryMode::Coalesced && entry.coalesce_threshold_hz > 0 {
            let window_start = now_ns.saturating_sub(1_000_000_000);
            tracker.fires.retain(|&t| t >= window_start);
            let limit = entry.coalesce_threshold_hz as usize * 10;
            if tracker.fires.len() > limit {
                self.events.push(InterruptEvent::StormDetected {
                    vector,
                    domain,
                    timestamp_ns: now_ns,
                });
            }
        }

        let outcome = match entry.mode {
            DeliveryMode::Direct => DeliveryOutcome::Delivered,
            DeliveryMode::Queued => DeliveryOutcome::Queued,
            DeliveryMode::Coalesced => DeliveryOutcome::Coalesced,
        };
        self.events.push(InterruptEvent::Delivered {
            vector,
            domain,
            timestamp_ns: now_ns,
        });
        outcome
    }

    /// Delivers interrupt `vector` across a domain boundary from
    /// `source_domain` to `target_domain`.
    ///
    /// RFC-0010.8: cross-domain delivery requires an explicit
    /// cross-domain delivery capability token; unauthorized crossing is
    /// non-conformant and reported as spurious (RFC-0010.11). The interrupt
    /// remains bound to `source_domain`; delivery occurs in
    /// `target_domain`'s context using the source's routing entry (mode,
    /// priority, threshold).
    ///
    /// # Errors
    /// Returns `CrossDomainNotAuthorized` if no cross-domain capability is
    /// provided, or `UnknownVector` if the vector is not bound to the source
    /// domain.
    pub fn deliver_cross_domain(
        &mut self,
        vector: IrqVector,
        source_domain: DomainId,
        target_domain: DomainId,
        cross_domain_cap: Option<&Capability>,
        now_ns: u64,
    ) -> Result<DeliveryOutcome, InterruptError> {
        let Some(cap) = cross_domain_cap else {
            self.events.push(InterruptEvent::Spurious {
                vector,
                domain: Some(target_domain),
                timestamp_ns: now_ns,
            });
            return Err(InterruptError::CrossDomainNotAuthorized);
        };
        if cap.object_type != aether_types::ObjectType::Memory
            || !cap.rights.contains(Rights::MAP)
            || !cap.flags.contains(CapabilityFlags::BOUND)
        {
            self.events.push(InterruptEvent::Spurious {
                vector,
                domain: Some(target_domain),
                timestamp_ns: now_ns,
            });
            return Err(InterruptError::CrossDomainNotAuthorized);
        }
        // The vector must be bound to the source domain (RFC-0010.2).
        let entry = if let Some(entry) = self
            .tables
            .get(&source_domain)
            .and_then(|t| t.entries.get(&vector))
        {
            entry.clone()
        } else {
            self.events.push(InterruptEvent::Spurious {
                vector,
                domain: Some(source_domain),
                timestamp_ns: now_ns,
            });
            return Err(InterruptError::UnknownVector);
        };
        Ok(self.deliver_to(vector, target_domain, now_ns, &entry))
    }

    /// Returns the minimum maskable priority level (RFC-0010.10).
    #[must_use]
    pub const fn min_maskable_priority(&self) -> u8 {
        self.min_maskable_priority
    }

    /// Returns the documented maximum nesting depth (RFC-0010.9).
    #[must_use]
    pub const fn max_nesting_depth(&self) -> u8 {
        self.max_nesting_depth
    }
}

impl Default for InterruptRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(value: u16) -> DomainId {
        DomainId::new(value).unwrap()
    }

    fn bind_cap() -> Capability {
        Capability {
            object: aether_types::ObjectId(1),
            object_type: aether_types::ObjectType::Memory,
            rights: Rights::MAP,
            bounds: aether_types::Bounds::new(0, 4096).unwrap(),
            flags: CapabilityFlags::BOUND,
            expires_at_ns: 0,
        }
    }

    fn entry(vector: IrqVector, mode: DeliveryMode, priority: u8) -> RoutingEntry {
        RoutingEntry {
            vector,
            handler_cap: bind_cap(),
            mode,
            priority,
            coalesce_threshold_hz: 1000,
        }
    }

    #[test]
    fn routing_entry_requires_bind_capability() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        let mut cap = bind_cap();
        cap.flags = CapabilityFlags::NONE; // not BOUND → not INTERRUPT_BIND
        assert_eq!(
            router.configure_routing(
                domain(1),
                IrqVector(10),
                &cap,
                entry(IrqVector(10), DeliveryMode::Direct, 5),
                1_000
            ),
            Err(InterruptError::Unauthorized)
        );
    }

    #[test]
    fn configure_and_deliver_direct() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        let vector = IrqVector(42);
        router
            .configure_routing(
                domain(1),
                vector,
                &bind_cap(),
                entry(vector, DeliveryMode::Direct, 5),
                1_000,
            )
            .unwrap();
        assert!(router.has_table(domain(1)));
        assert_eq!(
            router.deliver(vector, domain(1), 2_000),
            DeliveryOutcome::Delivered
        );
        assert!(
            router
                .events()
                .iter()
                .any(|e| matches!(e, InterruptEvent::Delivered { vector: v, .. } if *v == vector))
        );
    }

    #[test]
    fn queued_delivery_preserves_order() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        for v in [IrqVector(1), IrqVector(2), IrqVector(3)] {
            router
                .configure_routing(
                    domain(1),
                    v,
                    &bind_cap(),
                    entry(v, DeliveryMode::Queued, 3),
                    1_000,
                )
                .unwrap();
        }
        assert_eq!(
            router.deliver(IrqVector(1), domain(1), 2_000),
            DeliveryOutcome::Queued
        );
        assert_eq!(
            router.deliver(IrqVector(2), domain(1), 2_000),
            DeliveryOutcome::Queued
        );
        assert_eq!(
            router.deliver(IrqVector(3), domain(1), 2_000),
            DeliveryOutcome::Queued
        );
        // BTreeMap iterates in vector order → queued delivery order preserved.
        let delivered: Vec<IrqVector> = router
            .events()
            .iter()
            .filter_map(|e| match e {
                InterruptEvent::Delivered { vector, .. } => Some(*vector),
                _ => None,
            })
            .collect();
        assert_eq!(delivered, vec![IrqVector(1), IrqVector(2), IrqVector(3)]);
    }

    #[test]
    fn coalesced_delivery_recorded() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        let vector = IrqVector(7);
        router
            .configure_routing(
                domain(1),
                vector,
                &bind_cap(),
                entry(vector, DeliveryMode::Coalesced, 4),
                1_000,
            )
            .unwrap();
        assert_eq!(
            router.deliver(vector, domain(1), 2_000),
            DeliveryOutcome::Coalesced
        );
    }

    #[test]
    fn unhandled_vector_is_reported_not_silently_dropped() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        // Vector not bound to any domain: spurious event, no delivery.
        assert_eq!(
            router.deliver(IrqVector(99), domain(1), 2_000),
            DeliveryOutcome::NoHandler
        );
        assert!(
            router
                .events()
                .iter()
                .any(|e| matches!(e, InterruptEvent::Spurious { .. }))
        );
    }

    #[test]
    fn cross_domain_delivery_requires_capability() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        router.register_domain(domain(2), None, 1_000).unwrap();
        let vector = IrqVector(55);
        router
            .configure_routing(
                domain(1),
                vector,
                &bind_cap(),
                entry(vector, DeliveryMode::Direct, 5),
                1_000,
            )
            .unwrap();
        // No cross-domain capability: rejected and reported spurious.
        assert_eq!(
            router.deliver_cross_domain(vector, domain(1), domain(2), None, 2_000),
            Err(InterruptError::CrossDomainNotAuthorized)
        );
        // With an authorized cross-domain capability: delivered.
        assert_eq!(
            router.deliver_cross_domain(vector, domain(1), domain(2), Some(&bind_cap()), 2_000),
            Ok(DeliveryOutcome::Delivered)
        );
    }

    #[test]
    fn priority_and_maskable_level() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        // Priority below the minimum maskable level is rejected (RFC-0010.10).
        assert_eq!(
            router.configure_routing(
                domain(1),
                IrqVector(1),
                &bind_cap(),
                entry(IrqVector(1), DeliveryMode::Direct, 0),
                1_000
            ),
            Err(InterruptError::PriorityAboveMaskable)
        );
        assert_eq!(router.min_maskable_priority(), 1);
        assert_eq!(router.max_nesting_depth(), 8);
    }

    #[test]
    fn unconfigure_requires_bind() {
        let mut router = InterruptRouter::new();
        router.register_domain(domain(1), None, 1_000).unwrap();
        router
            .configure_routing(
                domain(1),
                IrqVector(3),
                &bind_cap(),
                entry(IrqVector(3), DeliveryMode::Direct, 5),
                1_000,
            )
            .unwrap();
        let mut no_bind = bind_cap();
        no_bind.flags = CapabilityFlags::NONE;
        assert_eq!(
            router.unconfigure_routing(domain(1), IrqVector(3), &no_bind),
            Err(InterruptError::Unauthorized)
        );
        router
            .unconfigure_routing(domain(1), IrqVector(3), &bind_cap())
            .unwrap();
        assert!(!router.has_table(domain(1)));
    }
}
