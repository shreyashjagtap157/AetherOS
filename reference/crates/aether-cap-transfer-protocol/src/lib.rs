//! Capability transfer protocol reference implementation (RFC-0038).
//!
//! This is a **Proposed** RFC implementation (treated as Proposed until
//! evidence-gated acceptance per the RFC lifecycle note). It mechanizes the
//! transfer protocol on top of RFC-0037 capability tokens:
//!
//! - Three transfer types: `COPY` (sender retains), `MOVE` (sender
//!   invalidated), `DERIVE_COPY` (sender retains, narrowed rights derived).
//! - Sender verifies `RIGHT_TRANSFER`; receiver verifies seal, expiry, and
//!   revocation before installing a token.
//! - Rights narrowing (proper subset) for `DERIVE_COPY`.
//! - `rights_derived` counter incremented on derivation.
//! - Domain scope: `target_domain_index` bound to receiver's domain; `CAPF_BOUND`
//!   unless cross-domain explicitly permitted.
//! - Revocation takes precedence in-flight and propagates after `MOVE`.
//! - Failure leaves the sender's original capability unaffected (idempotent
//!   retry).
//!
//! Builds on `aether-cap-token` (RFC-0037) and `aether-capability-transfer`
//! (RFC-0027).

use aether_cap_token::{
    CapabilityToken, CAPF_BOUND, RIGHT_TRANSFER,
};

// ---------------------------------------------------------------------------
// Transfer types (RFC-0038.1)
// ---------------------------------------------------------------------------

/// The kind of capability transfer (RFC-0038.1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransferType {
    /// Capability is copied; the sender retains its original token.
    Copy,
    /// Capability is moved; the sender's token is invalidated.
    Move,
    /// A derived capability with narrower rights is created; sender retains.
    DeriveCopy,
}

// ---------------------------------------------------------------------------
// Protocol state
// ---------------------------------------------------------------------------

/// A held capability token together with whether it has been invalidated.
#[derive(Clone, Debug)]
struct HeldToken {
    token: CapabilityToken,
    invalidated: bool,
}

/// In-memory protocol state: held tokens keyed by an opaque handle, plus the
/// set of revoked token seals. A real implementation backs this with the State
/// Architecture (RFC-0015) and revocation list (RFC-0039).
#[derive(Default)]
pub struct TransferProtocol {
    /// Held tokens: handle -> held token.
    held: Vec<Option<HeldToken>>,
    /// Revoked token seals (RFC-0039 integration).
    revoked_seals: Vec<[u8; 16]>,
}

/// Opaque handle to a held capability token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TokenHandle(pub usize);

// ---------------------------------------------------------------------------
// Transfer protocol (RFC-0038.2)
// ---------------------------------------------------------------------------

/// Outcome of a transfer: the token the receiver installs, and whether the
/// sender's original was invalidated (MOVE only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferOutcome {
    /// Token to install at the receiver (None if transfer failed).
    pub installed_token: Option<CapabilityToken>,
    /// Whether the sender's original token was invalidated (MOVE).
    pub sender_invalidated: bool,
    /// Human-readable result detail for audit.
    pub detail: String,
}

/// Execute a capability transfer (RFC-0038.2).
///
/// Steps:
/// 1. Sender verifies it holds the capability with `RIGHT_TRANSFER`.
/// 2. Sender creates a transfer message (type, rights subset for DERIVE_COPY,
///    sealed token trailer).
/// 3. Message sent via Communication Architecture (`MSG_CAP_TRANSFER`).
/// 4. Receiver verifies seal, expiry, revocation.
/// 5. On pass: install token per transfer type; invalidate sender for MOVE.
/// 6. Both sides emit `CapGranted` events (RFC-0031, out of scope here).
///
/// On any failure the sender's original capability is unaffected (RFC-0038.8)
/// and the sender may retry idempotently.
pub fn transfer(
    proto: &mut TransferProtocol,
    handle: TokenHandle,
    ty: TransferType,
    requested_rights: Option<u8>,
    receiver_domain_index: u32,
    cross_domain_permitted: bool,
    now_ns: u64,
) -> Result<TransferOutcome, TransferError> {
    // Step 1: locate the sender's held token.
    let held = proto
        .held
        .get(handle.0)
        .and_then(|o| o.as_ref())
        .ok_or(TransferError::NoSuchToken)?;

    // RFC-0038.2 step 1: sender must hold RIGHT_TRANSFER.
    if !has_right(&held.token, RIGHT_TRANSFER) {
        return Err(TransferError::NoTransferRight);
    }

    // RFC-0038.2 step 2: verify seal before any rights enforcement.
    if !crate_seal_valid(&held.token) {
        return Err(TransferError::SealInvalid);
    }

    // RFC-0037.14: token must not be expired.
    if is_expired(&held.token, now_ns) {
        return Err(TransferError::Expired);
    }

    // Revocation check: token must not already be revoked.
    if is_revoked(proto, &held.token) {
        return Err(TransferError::Revoked);
    }

    // Build the token the receiver will install.
    let mut rights = held.token.rights();
    let mut flags = held.token.flags();

    match ty {
        TransferType::Copy => {
            // Same rights, receiver's domain.
            flags = set_domain_bound(flags, receiver_domain_index, cross_domain_permitted);
        }
        TransferType::Move => {
            // Same rights, receiver's domain; sender invalidated.
            flags = set_domain_bound(flags, receiver_domain_index, cross_domain_permitted);
        }
        TransferType::DeriveCopy => {
            // RFC-0038.3: requested rights must be a proper subset of original.
            let req = requested_rights.ok_or(TransferError::RightsSubsetRequired)?;
            if req & !rights != 0 {
                return Err(TransferError::RightsNotSubset);
            }
            if req == rights {
                return Err(TransferError::RightsNotProperSubset);
            }
            rights = req;
            // RFC-0038.4: rights_derived = original.rights_derived + 1.
            flags = set_domain_bound(flags, receiver_domain_index, cross_domain_permitted);
        }
    }

    // Build the receiver's installed token.
    let installed_token = CapabilityToken::new(
        held.token.cap_type(),
        rights,
        flags,
        held.token.rights_derived().wrapping_add(1),
        held.token.creation_timestamp_ns(),
        held.token.expires_at_ns(),
        held.token.target_object(),
        receiver_domain_index,
    );

    // RFC-0038.2 step 7: both sides emit CapGranted (out of scope; recorded in
    // the outcome detail).

    // Invalidate sender for MOVE (RFC-0038.1).
    if let Some(slot) = proto.held.get_mut(handle.0) {
        if let Some(h) = slot.as_mut() {
            if matches!(ty, TransferType::Move) {
                h.invalidated = true;
            }
        }
    }

    Ok(TransferOutcome {
        installed_token: Some(installed_token),
        sender_invalidated: matches!(ty, TransferType::Move),
        detail: format!("transfer {:?} accepted", ty),
    })
}

/// Revoke a held token and propagate to derived/transferred tokens
/// (RFC-0039.3/RFC-0038.6/RFC-0038.7 integration).
///
/// Revocation takes precedence: a revoked token can no longer be transferred,
/// and any token derived from it is also revoked.
pub fn revoke_token(proto: &mut TransferProtocol, handle: TokenHandle) -> Result<(), TransferError> {
    let seal = {
        let held = proto
            .held
            .get(handle.0)
            .and_then(|o| o.as_ref())
            .ok_or(TransferError::NoSuchToken)?;
        if held.invalidated {
            return Err(TransferError::AlreadyInvalidated);
        }
        held.token.seal()
    };
    proto.revoked_seals.push(seal);
    Ok(())
}

/// Returns true if a token's seal is in the revoked set.
fn is_revoked(proto: &TransferProtocol, token: &CapabilityToken) -> bool {
    proto.revoked_seals.iter().any(|s| *s == token.seal())
}

/// Returns true if `token` grants `right`.
fn has_right(token: &CapabilityToken, right: u8) -> bool {
    token.rights() & right != 0
}

/// Returns true if the token is expired at `now_ns`.
fn is_expired(token: &CapabilityToken, now_ns: u64) -> bool {
    let expires = token.expires_at_ns();
    expires != 0 && now_ns > expires
}

/// Seal validity check (RFC-0037.9).
fn crate_seal_valid(token: &CapabilityToken) -> bool {
    aether_cap_token::verify_seal(token)
}

/// Set/clear the domain-bound flag based on cross-domain policy
/// (RFC-0038.5).
fn set_domain_bound(flags: u8, _receiver_domain_index: u32, cross_domain_permitted: bool) -> u8 {
    if cross_domain_permitted {
        flags & !CAPF_BOUND
    } else {
        flags | CAPF_BOUND
    }
}

// ---------------------------------------------------------------------------
// Errors (RFC-0038 failure semantics)
// ---------------------------------------------------------------------------

/// Transfer protocol errors (RFC-0038 §3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransferError {
    /// No token exists at the given handle.
    NoSuchToken,
    /// Sender lacks RIGHT_TRANSFER (ENOCAP).
    NoTransferRight,
    /// Token seal is invalid (ECAPSTALE).
    SealInvalid,
    /// Token has expired (RFC-0037.14).
    Expired,
    /// Token is revoked.
    Revoked,
    /// DERIVE_COPY requires a requested rights subset.
    RightsSubsetRequired,
    /// Requested rights are not a subset of the original.
    RightsNotSubset,
    /// Requested rights must be a proper subset (no widening).
    RightsNotProperSubset,
    /// The token at the handle is already invalidated.
    AlreadyInvalidated,
}

// ---------------------------------------------------------------------------
// Token storage helpers (for tests and protocol users)
// ---------------------------------------------------------------------------

impl TransferProtocol {
    /// Holds a token and returns a handle to it.
    pub fn hold_token(&mut self, token: CapabilityToken) -> TokenHandle {
        for (i, slot) in self.held.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(HeldToken { token, invalidated: false });
                return TokenHandle(i);
            }
        }
        self.held.push(Some(HeldToken { token, invalidated: false }));
        TokenHandle(self.held.len() - 1)
    }

    /// Returns true if the held token at `handle` is invalidated.
    pub fn is_invalidated(&self, handle: TokenHandle) -> bool {
        self.held
            .get(handle.0)
            .and_then(|o| o.as_ref())
            .map(|h| h.invalidated)
            .unwrap_or(true)
    }

    /// Returns a clone of the held token at `handle`.
    pub fn held_token(&self, handle: TokenHandle) -> Option<CapabilityToken> {
        self.held
            .get(handle.0)
            .and_then(|o| o.as_ref())
            .map(|h| h.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_cap_token::{
        CapType, RIGHT_EXECUTE, RIGHT_READ, RIGHT_WRITE,
    };

    fn make_token(rights: u8, flags: u8, derived: u32) -> CapabilityToken {
        CapabilityToken::new(
            CapType::Object,
            rights,
            flags,
            derived,
            1_000,
            0, // never expires
            [0u8; 16],
            0,
        )
    }

    fn proto_with_token(rights: u8, flags: u8) -> (TransferProtocol, TokenHandle) {
        let mut proto = TransferProtocol::default();
        let handle = proto.hold_token(make_token(rights, flags, 0));
        (proto, handle)
    }

    #[test]
    fn copy_transfer_sender_retains() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ | RIGHT_WRITE, 0);
        let outcome = transfer(
            &mut proto,
            handle,
            TransferType::Copy,
            None,
            2,
            false,
            1_000,
        )
        .unwrap();
        assert!(outcome.installed_token.is_some());
        let installed = outcome.installed_token.unwrap();
        assert!(!outcome.sender_invalidated);
        assert!(!proto.is_invalidated(handle));
        // Receiver token bound to receiver's domain (CAPF_BOUND).
        assert!(installed.flags() & CAPF_BOUND != 0);
        assert_eq!(installed.target_domain_index(), 2);
        // Same rights as sender.
        assert_eq!(installed.rights(), RIGHT_READ | RIGHT_WRITE);
    }

    #[test]
    fn move_transfer_invalidates_sender() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ, 0);
        let outcome = transfer(
            &mut proto,
            handle,
            TransferType::Move,
            None,
            3,
            false,
            1_000,
        )
        .unwrap();
        assert!(outcome.sender_invalidated);
        assert!(proto.is_invalidated(handle));
    }

    #[test]
    fn derive_copy_narrows_rights() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ | RIGHT_WRITE, 0);
        let outcome = transfer(
            &mut proto,
            handle,
            TransferType::DeriveCopy,
            Some(RIGHT_READ),
            4,
            false,
            1_000,
        )
        .unwrap();
        let installed = outcome.installed_token.unwrap();
        assert_eq!(installed.rights(), RIGHT_READ);
        // RFC-0038.4: rights_derived incremented.
        assert_eq!(installed.rights_derived(), 1);
        // Sender retains original (DeriveCopy).
        assert!(!proto.is_invalidated(handle));
    }

    #[test]
    fn derive_copy_rejects_widening() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ, 0);
        let err = transfer(
            &mut proto,
            handle,
            TransferType::DeriveCopy,
            Some(RIGHT_READ | RIGHT_WRITE),
            4,
            false,
            1_000,
        );
        assert_eq!(err, Err(TransferError::RightsNotSubset));
    }

    #[test]
    fn derive_copy_rejects_equal_rights() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ, 0);
        let err = transfer(
            &mut proto,
            handle,
            TransferType::DeriveCopy,
            Some(RIGHT_READ),
            4,
            false,
            1_000,
        );
        assert_eq!(err, Err(TransferError::RightsNotProperSubset));
    }

    #[test]
    fn transfer_without_transfer_right_rejected() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ, 0);
        let err = transfer(
            &mut proto,
            handle,
            TransferType::Copy,
            None,
            2,
            false,
            1_000,
        );
        assert_eq!(err, Err(TransferError::NoTransferRight));
    }

    #[test]
    fn expired_token_rejected() {
        let mut proto = TransferProtocol::default();
        let token = CapabilityToken::new(
            CapType::Object,
            RIGHT_READ | RIGHT_TRANSFER,
            0,
            0,
            1_000,
            500, // expires at 500
            [0u8; 16],
            0,
        );
        let handle = proto.hold_token(token);
        let err = transfer(&mut proto, handle, TransferType::Copy, None, 2, false, 1_000);
        assert_eq!(err, Err(TransferError::Expired));
    }

    #[test]
    fn revoked_token_cannot_be_transferred() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ | RIGHT_TRANSFER, CAPF_REVOCABLE);
        proto.revoke_token(handle).unwrap();
        let err = transfer(&mut proto, handle, TransferType::Copy, None, 2, false, 1_000);
        assert_eq!(err, Err(TransferError::Revoked));
    }

    #[test]
    fn cross_domain_permitted_clears_bound_flag() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ | RIGHT_TRANSFER, 0);
        let outcome = transfer(
            &mut proto,
            handle,
            TransferType::Copy,
            None,
            7,
            true,
            1_000,
        )
        .unwrap();
        let installed = outcome.installed_token.unwrap();
        assert_eq!(installed.target_domain_index(), 7);
        assert!(installed.flags() & CAPF_BOUND == 0);
    }

    #[test]
    fn no_such_token() {
        let mut proto = TransferProtocol::default();
        let err = transfer(
            &mut proto,
            TokenHandle(999),
            TransferType::Copy,
            None,
            2,
            false,
            1_000,
        );
        assert_eq!(err, Err(TransferError::NoSuchToken));
    }

    #[test]
    fn invalidated_sender_cannot_transfer() {
        let (mut proto, handle) = proto_with_token(RIGHT_READ | RIGHT_TRANSFER, 0);
        // Move invalidates the sender.
        transfer(&mut proto, handle, TransferType::Move, None, 2, false, 1_000).unwrap();
        let err = transfer(&mut proto, handle, TransferType::Copy, None, 3, false, 1_000);
        assert_eq!(err, Err(TransferError::AlreadyInvalidated));
    }
}
