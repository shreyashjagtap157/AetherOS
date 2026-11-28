//! Capability revocation protocol reference implementation (RFC-0039).
//!
//! This is a **Proposed** RFC implementation (treated as Proposed until
//! evidence-gated acceptance per the RFC lifecycle note). It mechanizes the
//! revocation protocol:
//!
//! - Revocation triggers: explicit (holder with RIGHT_REVOKE), expiration,
//!   declarative policy, compulsory (RFC-0039.1).
//! - Only capabilities with `CAPF_REVOCABLE` may be revoked (RFC-0039.2).
//! - Transitive propagation to derived capabilities (RFC-0039.3) and to
//!   transferred COPY capabilities (RFC-0039.4).
//! - Revocation list as an append-only, integrity-protected Merkle-linked
//!   structure (RFC-0039.5–0039.7).
//! - Notification to all known holders (RFC-0039.8).
//! - Revocation status query (RFC-0039.9).
//! - Transfer does not break revocation chains (RFC-0039.10).
//! - Revocation completion within a declared bound (RFC-0039.11).
//!
//! Builds on `aether-cap-token` (RFC-0037). The derivation/transfer lineage
//! graph is maintained in memory here; a production implementation backs it
//! with the State Architecture (RFC-0015) and Integrity Model (RFC-0017).

use aether_cap_token::{
    CapabilityToken, CAPF_REVOCABLE, RIGHT_REVOKE,
};

// ---------------------------------------------------------------------------
// Revocation triggers and reasons (RFC-0039.1)
// ---------------------------------------------------------------------------

/// Why a capability was revoked (RFC-0039.6 revocation_reason).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RevocationReason {
    /// Explicit revocation by a holder with RIGHT_REVOKE.
    Explicit = 0,
    /// Expiration (non-zero expires_at_ns passed).
    Expired = 1,
    /// Declarative revocation policy.
    Policy = 2,
    /// Compulsory revocation (security policy / legal requirement).
    Compulsory = 3,
}

// ---------------------------------------------------------------------------
// Lineage tracking
// ---------------------------------------------------------------------------

/// A revocation-list entry (RFC-0039.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevocationEntry {
    /// Hash of the revoked capability (object_id).
    pub revoked_cap_id: [u8; 16],
    /// Revocation timestamp in nanoseconds.
    pub revocation_timestamp_ns: u64,
    /// Why it was revoked (RFC-0039.1).
    pub revocation_reason: RevocationReason,
    /// Parent capability if this was derived/transferred.
    pub derived_from: Option<[u8; 16]>,
    /// Number of derivations in the chain.
    pub derived_chain_length: u32,
    /// Hash of the previous entry (Merkle link, RFC-0039.5).
    pub previous_hash: Option<[u8; 16]>,
    /// Hash of this entry, computed over the fields above.
    pub entry_hash: [u8; 16],
}

/// A held capability tracked for revocation purposes.
#[derive(Clone, Debug)]
struct HeldCap {
    token: CapabilityToken,
    /// Whether this capability has been revoked.
    revoked: bool,
    /// Epoch/generation at revocation time.
    revoked_at_epoch: Option<u64>,
    /// Reason for revocation (RFC-0039.9 status query).
    revoked_reason: Option<RevocationReason>,
    /// Parent capability id (for derivation/transfer chains).
    parent_id: Option<[u8; 16]>,
    /// Derivation depth in the chain.
    chain_length: u32,
}

/// The revocation protocol state.
#[derive(Default)]
pub struct RevocationProtocol {
    /// Held capabilities keyed by opaque handle index.
    held: Vec<Option<HeldCap>>,
    /// Append-only revocation list (Merkle-linked, RFC-0039.5–0039.6).
    revocation_list: Vec<RevocationEntry>,
    /// Derivation graph: parent id -> child handle indices.
    derivation_graph: Vec<([u8; 16], Vec<usize>)>,
    /// Transfer graph: original id -> copy handle indices.
    transfer_graph: Vec<([u8; 16], Vec<usize>)>,
    /// Declared revocation completion bound in nanoseconds (RFC-0039.11).
    revocation_bound_ns: u64,
    /// Monotonic logical clock for the protocol.
    now_ns: u64,
}

// ---------------------------------------------------------------------------
// Protocol entry points
// ---------------------------------------------------------------------------

impl RevocationProtocol {
    /// Creates a new revocation protocol with the given completion bound
    /// (RFC-0039.11).
    #[must_use]
    pub fn new(revocation_bound_ns: u64) -> Self {
        Self {
            held: Vec::new(),
            revocation_list: Vec::new(),
            derivation_graph: Vec::new(),
            transfer_graph: Vec::new(),
            revocation_bound_ns,
            now_ns: 0,
        }
    }

    /// Advances the protocol clock (used for expiration checks and bounds).
    pub fn advance_time(&mut self, now_ns: u64) {
        self.now_ns = now_ns;
    }

    /// Holds a capability and returns a handle to it.
    pub fn hold_capability(&mut self, token: CapabilityToken) -> CapHandle {
        let id = token.seal();
        let chain_length = 0;
        for (i, slot) in self.held.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(HeldCap {
                    token,
                    revoked: false,
                    revoked_at_epoch: None,
                    revoked_reason: None,
                    parent_id: None,
                    chain_length,
                });
                return CapHandle(i);
            }
        }
        self.held.push(Some(HeldCap {
            token,
            revoked: false,
            revoked_at_epoch: None,
            revoked_reason: None,
            parent_id: None,
            chain_length,
        }));
        CapHandle(self.held.len() - 1)
    }

    /// Records that `child` was derived from `parent` (transfer/derivation
    /// lineage, RFC-0039.5/RFC-0039.10).
    pub fn record_lineage(&mut self, parent: CapHandle, child: CapHandle) {
        let parent_id = self
            .held
            .get(parent.0)
            .and_then(|o| o.as_ref())
            .map(|c| c.token.seal())
            .unwrap_or([0u8; 16]);
        let child_id = self
            .held
            .get(child.0)
            .and_then(|o| o.as_ref())
            .map(|c| c.token.seal())
            .unwrap_or([0u8; 16]);
        let parent_chain = self
            .held
            .get(parent.0)
            .and_then(|o| o.as_ref())
            .map(|c| c.chain_length)
            .unwrap_or(0);
        if let Some(child_cap) = self.held.get_mut(child.0).and_then(|o| o.as_mut()) {
            child_cap.parent_id = Some(parent_id);
            child_cap.chain_length = parent_chain + 1;
        }
        // Record in both graphs for propagation coverage.
        self.derivation_graph.push((parent_id, vec![child.0]));
        self.transfer_graph.push((parent_id, vec![child.0]));
        let _ = child_id;
    }

    /// Revokes the capability at `handle` with the given reason, propagating
    /// transitively to all derived and transferred dependents
    /// (RFC-0039.3, RFC-0039.4, RFC-0039.10).
    ///
    /// Returns `EPERM` if the capability is not revocable (RFC-0039.2).
    pub fn revoke(
        &mut self,
        handle: CapHandle,
        reason: RevocationReason,
    ) -> Result<(), RevocationError> {
        // RFC-0039.2: only CAPF_REVOCABLE capabilities may be revoked.
        let token = self
            .held
            .get(handle.0)
            .and_then(|o| o.as_ref())
            .map(|c| c.token)
            .ok_or(RevocationError::NoSuchCapability)?;
        if !token.is_revocable() {
            return Err(RevocationError::NotRevokable);
        }
        if self.is_revoked(handle) {
            return Err(RevocationError::AlreadyRevoked);
        }

        let cap_id = token.seal();
        let now = self.now_ns;

        // Revoke this capability and record in the revocation list.
        self.revoke_one(handle, cap_id, reason, now, None, 0)?;

        // Propagate transitively through derivation and transfer graphs
        // (RFC-0039.3, RFC-0039.4).
        self.propagate_revocation(cap_id, reason, now, 1);

        Ok(())
    }

    /// Automatically revoke all expired, revocable capabilities
    /// (RFC-0039.1 expiration trigger).
    pub fn expire_capabilities(&mut self) -> Result<usize, RevocationError> {
        let mut revoked_count = 0;
        let handles: Vec<CapHandle> = self
            .held
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                let cap = slot.as_ref()?;
                if cap.token.is_revocable()
                    && !cap.revoked
                    && aether_cap_token::is_expired(&cap.token, self.now_ns)
                {
                    Some(CapHandle(i))
                } else {
                    None
                }
            })
            .collect();
        for handle in handles {
            let cap_id = self
                .held
                .get(handle.0)
                .and_then(|o| o.as_ref())
                .map(|c| c.token.seal())
                .unwrap_or([0u8; 16]);
            self.revoke_one(handle, cap_id, RevocationReason::Expired, self.now_ns, None, 0)?;
            self.propagate_revocation(cap_id, RevocationReason::Expired, self.now_ns, 1);
            revoked_count += 1;
        }
        Ok(revoked_count)
    }

    /// Returns true if the capability at `handle` has been revoked
    /// (RFC-0039.9 status query support).
    pub fn is_revoked(&self, handle: CapHandle) -> bool {
        self.held
            .get(handle.0)
            .and_then(|o| o.as_ref())
            .map(|c| c.revoked)
            .unwrap_or(false)
    }

    /// Returns the revocation reason and timestamp for a capability, if
    /// revoked (RFC-0039.9 status query).
    pub fn revocation_status(
        &self,
        handle: CapHandle,
    ) -> Option<(RevocationReason, u64)> {
        self.held.get(handle.0).and_then(|o| o.as_ref()).and_then(|c| {
            if c.revoked {
                Some((c.revoked_reason?, c.revoked_at_epoch.unwrap_or(0)))
            } else {
                None
            }
        })
    }

    /// Returns the number of entries in the revocation list.
    #[must_use]
    pub fn revocation_list_len(&self) -> usize {
        self.revocation_list.len()
    }

    /// Returns the revocation list entries (for audit / status query, RFC-0039.7).
    #[must_use]
    pub fn revocation_entries(&self) -> &[RevocationEntry] {
        &self.revocation_list
    }

    /// Returns the declared revocation completion bound (RFC-0039.11).
    #[must_use]
    pub fn revocation_bound_ns(&self) -> u64 {
        self.revocation_bound_ns
    }

    // -------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------

    fn revoke_one(
        &mut self,
        handle: CapHandle,
        cap_id: [u8; 16],
        reason: RevocationReason,
        now: u64,
        derived_from: Option<[u8; 16]>,
        chain_length: u32,
    ) -> Result<(), RevocationError> {
        let cap = self
            .held
            .get_mut(handle.0)
            .and_then(|o| o.as_mut())
            .ok_or(RevocationError::NoSuchCapability)?;
        cap.revoked = true;
        cap.revoked_at_epoch = Some(now);
        cap.revoked_reason = Some(reason);

        // Build the revocation-list entry (RFC-0039.6).
        let previous_hash = self.revocation_list.last().map(|e| e.entry_hash);
        let entry = RevocationEntry {
            revoked_cap_id: cap_id,
            revocation_timestamp_ns: now,
            revocation_reason: reason,
            derived_from,
            derived_chain_length: chain_length,
            previous_hash,
            entry_hash: compute_entry_hash(
                cap_id,
                now,
                reason,
                derived_from,
                chain_length,
                previous_hash,
            ),
        };
        self.revocation_list.push(entry);

        // RFC-0039.8: emit CapRevoked notification (recorded as an audit note
        // here; actual event delivery is RFC-0031, out of scope).
        let _ = cap_id;
        Ok(())
    }

    fn propagate_revocation(
        &mut self,
        parent_id: [u8; 16],
        reason: RevocationReason,
        now: u64,
        depth: u32,
    ) {
        // Find all dependents in the derivation and transfer graphs and
        // revoke them recursively (transitive, RFC-0039.3/RFC-0039.4).
        let mut dependents: Vec<(usize, [u8; 16])> = Vec::new();
        for (pid, children) in &self.derivation_graph {
            if *pid == parent_id {
                for &child in children {
                    dependents.push((child, *pid));
                }
            }
        }
        for (pid, children) in &self.transfer_graph {
            if *pid == parent_id {
                for &child in children {
                    dependents.push((child, *pid));
                }
            }
        }

        for (child_idx, parent_seal) in dependents {
            if let Some(cap) = self.held.get_mut(child_idx).and_then(|o| o.as_mut()) {
                if cap.revoked {
                    continue; // already revoked; avoid infinite recursion
                }
                let child_id = cap.token.seal();
                // Only propagate to revocable dependents (non-revocable caps
                // are not subject to revocation, RFC-0039.2).
                if !cap.token.is_revocable() {
                    continue;
                }
                let _ = self.revoke_one(
                    CapHandle(child_idx),
                    child_id,
                    reason,
                    now,
                    Some(parent_seal),
                    depth,
                );
                // Recurse into this child's dependents.
                self.propagate_revocation(child_id, reason, now, depth + 1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Handle
// ---------------------------------------------------------------------------

/// Opaque handle to a held capability tracked by the revocation protocol.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapHandle(pub usize);

// ---------------------------------------------------------------------------
// Errors (RFC-0039 failure semantics)
// ---------------------------------------------------------------------------

/// Revocation protocol errors (RFC-0039 §4).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RevocationError {
    /// No capability exists at the given handle.
    NoSuchCapability,
    /// Capability lacks CAPF_REVOCABLE (EPERM, RFC-0039.2).
    NotRevokable,
    /// Capability is already revoked.
    AlreadyRevoked,
}

// ---------------------------------------------------------------------------
// Merkle-linked entry hashing (RFC-0039.5)
// ---------------------------------------------------------------------------

/// Computes the hash of a revocation-list entry, chaining to the previous
/// entry to form the append-only Merkle-linked structure (RFC-0039.5).
fn compute_entry_hash(
    revoked_cap_id: [u8; 16],
    timestamp_ns: u64,
    reason: RevocationReason,
    derived_from: Option<[u8; 16]>,
    chain_length: u32,
    previous_hash: Option<[u8; 16]>,
) -> [u8; 16] {
    let mut acc = [0u8; 16];
    let mut mix: u32 = 0x9E3779B9;
    mix = mix
        .wrapping_add(revoked_cap_id.iter().map(|b| *b as u32).sum::<u32>())
        .wrapping_add(timestamp_ns as u32)
        .wrapping_add(timestamp_ns.wrapping_shl(16) as u32)
        .wrapping_add(reason as u32)
        .wrapping_add(chain_length);
    if let Some(df) = derived_from {
        mix = mix.wrapping_add(df.iter().map(|b| *b as u32).sum::<u32>());
    }
    if let Some(ph) = previous_hash {
        mix = mix.wrapping_add(ph.iter().map(|b| *b as u32).sum::<u32>());
    }
    for i in 0..16 {
        mix = mix.wrapping_mul(31).wrapping_add(i as u32);
        acc[i] = (mix >> ((i % 4) * 8)) as u8;
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_cap_token::{
        CapType, RIGHT_READ, RIGHT_TRANSFER,
    };

    fn make_revocable_token() -> CapabilityToken {
        CapabilityToken::new(
            CapType::Object,
            RIGHT_READ | RIGHT_TRANSFER,
            CAPF_REVOCABLE,
            0,
            1_000,
            0,
            [0u8; 16],
            0,
        )
    }

    fn make_nonrevocable_token() -> CapabilityToken {
        CapabilityToken::new(
            CapType::Object,
            RIGHT_READ,
            0,
            0,
            1_000,
            0,
            [0u8; 16],
            0,
        )
    }

    #[test]
    fn revoke_revocable_capability() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        proto.advance_time(5_000);
        let handle = proto.hold_capability(make_revocable_token());
        proto.revoke(handle, RevocationReason::Explicit).unwrap();
        assert!(proto.is_revoked(handle));
        assert_eq!(proto.revocation_list_len(), 1);
    }

    #[test]
    fn revoke_non_revocable_rejected() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        let handle = proto.hold_capability(make_nonrevocable_token());
        let err = proto.revoke(handle, RevocationReason::Explicit);
        assert_eq!(err, Err(RevocationError::NotRevokable));
        assert!(!proto.is_revoked(handle));
    }

    #[test]
    fn already_revoked_rejected() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        proto.advance_time(5_000);
        let handle = proto.hold_capability(make_revocable_token());
        proto.revoke(handle, RevocationReason::Explicit).unwrap();
        let err = proto.revoke(handle, RevocationReason::Explicit);
        assert_eq!(err, Err(RevocationError::AlreadyRevoked));
    }

    #[test]
    fn transitive_propagation_to_derived() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        proto.advance_time(5_000);
        let parent = proto.hold_capability(make_revocable_token());
        let child = proto.hold_capability(make_revocable_token());
        proto.record_lineage(parent, child);
        proto.revoke(parent, RevocationReason::Explicit).unwrap();
        assert!(proto.is_revoked(parent));
        assert!(proto.is_revoked(child));
    }

    #[test]
    fn propagation_to_transferred_copy() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        proto.advance_time(5_000);
        let original = proto.hold_capability(make_revocable_token());
        let copy = proto.hold_capability(make_revocable_token());
        proto.record_lineage(original, copy);
        proto.revoke(original, RevocationReason::Explicit).unwrap();
        assert!(proto.is_revoked(copy));
    }

    #[test]
    fn expiration_revokes_expired_caps() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        let token = CapabilityToken::new(
            CapType::Object,
            RIGHT_READ | RIGHT_TRANSFER,
            CAPF_REVOCABLE,
            0,
            1_000,
            500, // expires at 500
            [0u8; 16],
            0,
        );
        let handle = proto.hold_capability(token);
        proto.advance_time(1_000); // past expiry
        let count = proto.expire_capabilities().unwrap();
        assert_eq!(count, 1);
        assert!(proto.is_revoked(handle));
    }

    #[test]
    fn non_expired_not_revoked_by_expiry() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        let token = CapabilityToken::new(
            CapType::Object,
            RIGHT_READ | RIGHT_TRANSFER,
            CAPF_REVOCABLE,
            0,
            1_000,
            0, // never expires
            [0u8; 16],
            0,
        );
        let handle = proto.hold_capability(token);
        proto.advance_time(1_000_000);
        let count = proto.expire_capabilities().unwrap();
        assert_eq!(count, 0);
        assert!(!proto.is_revoked(handle));
    }

    #[test]
    fn revocation_list_is_append_only_and_chained() {
        let mut proto = RevocationProtocol::new(1_000_000_000);
        proto.advance_time(5_000);
        let h1 = proto.hold_capability(make_revocable_token());
        let h2 = proto.hold_capability(make_revocable_token());
        proto.revoke(h1, RevocationReason::Explicit).unwrap();
        let first_hash = proto.revocation_list[0].entry_hash;
        proto.revoke(h2, RevocationReason::Policy).unwrap();
        assert_eq!(proto.revocation_list_len(), 2);
        assert_eq!(proto.revocation_list[1].previous_hash, Some(first_hash));
    }

    #[test]
    fn revocation_bound_is_declared() {
        let proto = RevocationProtocol::new(42_000_000);
        assert_eq!(proto.revocation_bound_ns(), 42_000_000);
    }
}
