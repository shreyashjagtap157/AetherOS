//! Capability transfer reference implementation (RFC-0027).
//!
//! Communication-layer capability transfer: capability tokens are carried in
//! the message trailer (RFC-0024.3) with the `MSG_CAP_TRANSFER` flag set
//! (RFC-0024.2). Transfer is a copy operation by default: the sender retains
//! its own token (RFC-0027.2). Rights may be narrowed but never widened
//! (RFC-0027.4), and the sender must hold `RIGHT_TRANSFER` (RFC-0027.5).
//! Corrupted tokens are rejected and the message is not delivered
//! (RFC-0027.7).

use aether_cap_token::{CapabilityToken, TOKEN_SIZE, verify_seal};
use aether_endpoint_model::CommErrorCode;
use aether_message_format::Message;

// ---------------------------------------------------------------------------
// Transfer semantics
// ---------------------------------------------------------------------------

/// Transfer semantics (RFC-0027.2, RFC-0038.1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransferKind {
    /// Capability is copied; the sender retains its own token (RFC-0027.2).
    Copy,
    /// Capability is moved; the sender's token is invalidated (RFC-0038).
    Move,
}

/// Prepared result of authorizing a capability transfer.
pub struct Transfer {
    /// The (possibly narrowed) token to place in the message trailer.
    pub token: CapabilityToken,
    /// The transfer kind.
    pub kind: TransferKind,
}

// ---------------------------------------------------------------------------
// Transfer authorization (RFC-0027.4, RFC-0027.5)
// ---------------------------------------------------------------------------

/// Authorizes a capability transfer and prepares the token for the trailer
/// (RFC-0027.1, RFC-0027.4, RFC-0027.5).
///
/// `holder_token` is the capability the sender holds. `requested_rights` are
/// the rights the sender wants the receiver to acquire.
///
/// Returns `CommErrorCode::ENOCAP` if the sender lacks `RIGHT_TRANSFER`
/// (RFC-0027.5), or if rights widening is attempted (RFC-0027.4 / RFC-0013).
pub fn authorize_transfer(
    holder_token: &CapabilityToken,
    requested_rights: u8,
) -> Result<Transfer, CommErrorCode> {
    // RFC-0027.5: sender must hold RIGHT_TRANSFER on the capability.
    if !holder_token.has_transfer_right() {
        return Err(CommErrorCode::ENOCAP);
    }
    // RFC-0027.4: rights may be narrowed but never widened.
    if requested_rights & !holder_token.rights() != 0 {
        return Err(CommErrorCode::ENOCAP);
    }
    let token = CapabilityToken::new(
        holder_token.cap_type(),
        requested_rights,
        holder_token.flags(),
        holder_token.rights_derived(),
        holder_token.creation_timestamp_ns(),
        holder_token.expires_at_ns(),
        holder_token.target_object(),
        0, // target_domain_index is set by the receiver's domain (RFC-0038.5)
    );
    Ok(Transfer {
        token,
        kind: TransferKind::Copy,
    })
}

// ---------------------------------------------------------------------------
// Received token validation (RFC-0027.7, RFC-0027.8)
// ---------------------------------------------------------------------------

/// Validates a received capability token from a message trailer
/// (RFC-0027.7, RFC-0027.8).
///
/// Returns `Ok(())` if the seal is valid. A corrupted or tampered token is
/// rejected and the message MUST NOT be delivered (RFC-0027.7).
pub fn validate_received_token(token: &CapabilityToken) -> Result<(), CommErrorCode> {
    // RFC-0027.8 / RFC-0037.9: seal must be verified before rights enforced.
    if !verify_seal(token) {
        return Err(CommErrorCode::ECAPSTALE);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Token extraction from a message trailer (RFC-0027.1)
// ---------------------------------------------------------------------------

/// Extracts capability tokens from a message trailer carrying
/// `MSG_CAP_TRANSFER` (RFC-0027.1).
///
/// Each token is exactly `TOKEN_SIZE` bytes (RFC-0037.2). Returns an error if
/// the flag is set but the trailer does not contain well-formed tokens.
pub fn extract_tokens(message: &Message) -> Result<Vec<CapabilityToken>, CommErrorCode> {
    if !message.has_capability_transfer() {
        return Ok(Vec::new());
    }
    let count = message.header.capability_count as usize;
    let trailer = &message.trailer;
    if trailer.len() != count * TOKEN_SIZE {
        return Err(CommErrorCode::EPROTO);
    }
    let mut tokens = Vec::with_capacity(count);
    for chunk in trailer.chunks(TOKEN_SIZE) {
        match CapabilityToken::deserialize(chunk) {
            Ok(token) => tokens.push(token),
            Err(_) => return Err(CommErrorCode::EPROTO),
        }
    }
    Ok(tokens)
}
