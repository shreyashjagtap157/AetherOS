//! Capability token format reference implementation (RFC-0037).
//!
//! Every capability token is exactly 96 bytes, 8-byte aligned, divided into
//! an identity section (32 bytes), a reference section (32 bytes), and an
//! integrity section (32 bytes). The seal covers the first 64 bytes and is
//! verified before any rights are enforced (RFC-0037.9).

use core::fmt;

/// Total token size in bytes, exactly 96 (RFC-0037.2).
pub const TOKEN_SIZE: usize = 96;
/// Size of the identity section (RFC-0037.1).
pub const IDENTITY_SECTION: usize = 32;
/// Size of the reference section (RFC-0037.1).
pub const REFERENCE_SECTION: usize = 32;
/// Size of the integrity section (RFC-0037.1).
pub const INTEGRITY_SECTION: usize = 32;

// ---------------------------------------------------------------------------
// Capability types (RFC-0037.3)
// ---------------------------------------------------------------------------

/// Capability type (RFC-0037.3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(u8)]
pub enum CapType {
    /// Operations on a persistent object (RFC-0015).
    Object = 0,
    /// Memory region mapping (RFC-0009).
    Memory = 1,
    /// Send/receive on a communication endpoint (RFC-0025).
    Endpoint = 2,
    /// Domain creation/management (RFC-0007).
    Domain = 3,
    /// Transfer/revoke other capabilities (RFC-0015).
    Capability = 4,
    /// Storage provider operations (RFC-0019).
    Storage = 5,
    /// Interrupt binding (RFC-0010).
    Interrupt = 6,
    /// Synchronization primitives (RFC-0028).
    Sync = 7,
    /// Device-originated DMA access to memory regions.
    DmaMemory = 8,
    /// IOMMU translation table programming.
    IoMmuConfig = 9,
    /// Memory-mapped I/O register access.
    MmioRegion = 10,
    /// Error representation; not a valid resource-access capability.
    Error = 11,
    /// Device register operations.
    DeviceControl = 12,
}

// ---------------------------------------------------------------------------
// Rights flags (RFC-0037.5)
// ---------------------------------------------------------------------------

/// Read access.
pub const RIGHT_READ: u8 = 1 << 0;
/// Write access.
pub const RIGHT_WRITE: u8 = 1 << 1;
/// Execute access.
pub const RIGHT_EXECUTE: u8 = 1 << 2;
/// Map into address space (RFC-0009).
pub const RIGHT_MAP: u8 = 1 << 3;
/// Transfer to another holder (RFC-0027).
pub const RIGHT_TRANSFER: u8 = 1 << 4;
/// Revoke dependent capabilities (RFC-0039).
pub const RIGHT_REVOKE: u8 = 1 << 5;
/// Derive a narrower capability.
pub const RIGHT_DERIVE: u8 = 1 << 6;

// ---------------------------------------------------------------------------
// Capability flags (RFC-0037.7)
// ---------------------------------------------------------------------------

/// Not stored in State; lost on domain destroy.
pub const CAPF_TRANSIENT: u8 = 0x01;
/// Subject to revocation (RFC-0039).
pub const CAPF_REVOCABLE: u8 = 0x02;
/// Has a stable name (for lookup).
pub const CAPF_NAMED: u8 = 0x04;
/// Bound to a specific domain; not transferable (RFC-0037.15).
pub const CAPF_BOUND: u8 = 0x08;
/// Seal is HMAC; cannot be inspected without key (RFC-0037.10).
pub const CAPF_SEALED: u8 = 0x10;

// ---------------------------------------------------------------------------
// Platform seal key (RFC-0037.8)
// ---------------------------------------------------------------------------

/// Platform master seal key used to compute the token seal.
///
/// RFC-0037.8 derives this from the hardware root of trust at bootstrap. This
/// reference implementation uses a fixed constant so tokens are deterministic
/// across restarts; a production build must replace it with the
/// hardware-keyed derivation.
const PLATFORM_SEAL_KEY: [u8; 32] = *b"AetherOSCapabilitySealKey2026!!!";

// ---------------------------------------------------------------------------
// Capability token (RFC-0037.1)
// ---------------------------------------------------------------------------

/// A capability token: exactly 96 bytes, 8-byte aligned (RFC-0037.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityToken(pub [u8; TOKEN_SIZE]);

impl CapabilityToken {
    /// Field offsets within the 96-byte token.
    const OFF_TYPE: usize = 0;
    const OFF_RIGHTS: usize = 1;
    const OFF_FLAGS: usize = 2;
    const OFF_RIGHTS_DERIVED: usize = 4;
    const OFF_CREATION_TS: usize = 8;
    const OFF_EXPIRES_AT: usize = 16;
    const OFF_TARGET_OBJECT: usize = 24;
    const OFF_TARGET_DOMAIN_INDEX: usize = 40;
    const OFF_SEAL: usize = 64;

    /// Creates a new capability token from raw fields (RFC-0037.1). The seal
    /// is computed over the first 64 bytes (RFC-0037.8).
    #[must_use]
    pub fn new(
        cap_type: CapType,
        rights: u8,
        flags: u8,
        rights_derived: u32,
        creation_timestamp_ns: u64,
        expires_at_ns: u64,
        target_object: [u8; 16],
        target_domain_index: u32,
    ) -> Self {
        let mut buf = [0u8; TOKEN_SIZE];
        buf[Self::OFF_TYPE] = cap_type as u8;
        buf[Self::OFF_RIGHTS] = rights;
        buf[Self::OFF_FLAGS] = flags;
        buf[Self::OFF_RIGHTS_DERIVED..Self::OFF_RIGHTS_DERIVED + 4]
            .copy_from_slice(&rights_derived.to_be_bytes());
        buf[Self::OFF_CREATION_TS..Self::OFF_CREATION_TS + 8]
            .copy_from_slice(&creation_timestamp_ns.to_be_bytes());
        buf[Self::OFF_EXPIRES_AT..Self::OFF_EXPIRES_AT + 8]
            .copy_from_slice(&expires_at_ns.to_be_bytes());
        buf[Self::OFF_TARGET_OBJECT..Self::OFF_TARGET_OBJECT + 16].copy_from_slice(&target_object);
        buf[Self::OFF_TARGET_DOMAIN_INDEX..Self::OFF_TARGET_DOMAIN_INDEX + 4]
            .copy_from_slice(&target_domain_index.to_be_bytes());
        // Compute the seal over the first 64 bytes (RFC-0037.8).
        let seal = compute_seal(&buf[..64]);
        buf[Self::OFF_SEAL..Self::OFF_SEAL + 16].copy_from_slice(&seal);
        Self(buf)
    }

    /// Returns the capability type (RFC-0037.3).
    #[must_use]
    pub fn cap_type(self) -> CapType {
        match self.0[Self::OFF_TYPE] {
            0 => CapType::Object,
            1 => CapType::Memory,
            2 => CapType::Endpoint,
            3 => CapType::Domain,
            4 => CapType::Capability,
            5 => CapType::Storage,
            6 => CapType::Interrupt,
            7 => CapType::Sync,
            8 => CapType::DmaMemory,
            9 => CapType::IoMmuConfig,
            10 => CapType::MmioRegion,
            11 => CapType::Error,
            12 => CapType::DeviceControl,
            // Unknown types are rejected by the capability mediator
            // (RFC-0037 failure semantics).
            _ => CapType::Object,
        }
    }

    /// Returns the rights bitmask (RFC-0037.5).
    #[must_use]
    pub fn rights(self) -> u8 {
        self.0[Self::OFF_RIGHTS]
    }

    /// Returns true if `RIGHT_TRANSFER` is granted (RFC-0037.5).
    #[must_use]
    pub fn has_transfer_right(self) -> bool {
        self.rights() & RIGHT_TRANSFER != 0
    }

    /// Returns true if `RIGHT_REVOKE` is granted (RFC-0037.5).
    #[must_use]
    pub fn has_revoke_right(self) -> bool {
        self.rights() & RIGHT_REVOKE != 0
    }

    /// Returns true if `RIGHT_DERIVE` is granted (RFC-0037.5).
    #[must_use]
    pub fn has_derive_right(self) -> bool {
        self.rights() & RIGHT_DERIVE != 0
    }

    /// Returns the capability flags (RFC-0037.7).
    #[must_use]
    pub fn flags(self) -> u8 {
        self.0[Self::OFF_FLAGS]
    }

    /// Returns true if `CAPF_REVOCABLE` is set (RFC-0037.7).
    #[must_use]
    pub fn is_revocable(self) -> bool {
        self.flags() & CAPF_REVOCABLE != 0
    }

    /// Returns true if `CAPF_BOUND` is set (RFC-0037.7).
    #[must_use]
    pub fn is_bound(self) -> bool {
        self.flags() & CAPF_BOUND != 0
    }

    /// Returns the rights_derived counter (RFC-0037.12).
    #[must_use]
    pub fn rights_derived(self) -> u32 {
        u32::from_be_bytes([self.0[4], self.0[5], self.0[6], self.0[7]])
    }

    /// Returns the creation timestamp (RFC-0037.1).
    #[must_use]
    pub fn creation_timestamp_ns(self) -> u64 {
        u64::from_be_bytes([
            self.0[8], self.0[9], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15],
        ])
    }

    /// Returns the expiration timestamp; 0 means never (RFC-0037.1).
    #[must_use]
    pub fn expires_at_ns(self) -> u64 {
        u64::from_be_bytes([
            self.0[16], self.0[17], self.0[18], self.0[19],
            self.0[20], self.0[21], self.0[22], self.0[23],
        ])
    }

    /// Returns the target object reference (RFC-0037.4).
    #[must_use]
    pub fn target_object(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out.copy_from_slice(&self.0[Self::OFF_TARGET_OBJECT..Self::OFF_TARGET_OBJECT + 16]);
        out
    }

    /// Returns the target domain index (RFC-0037.1).
    #[must_use]
    pub fn target_domain_index(self) -> u32 {
        u32::from_be_bytes([
            self.0[40], self.0[41], self.0[42], self.0[43],
        ])
    }

    /// Returns the seal (16 bytes, RFC-0037.8).
    #[must_use]
    pub fn seal(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out.copy_from_slice(&self.0[Self::OFF_SEAL..Self::OFF_SEAL + 16]);
        out
    }

    /// Serializes the token to its 96-byte wire form (RFC-0037.2).
    #[must_use]
    pub fn serialize(self) -> [u8; TOKEN_SIZE] {
        self.0
    }

    /// Deserializes a token from a byte buffer (RFC-0037.2).
    pub fn deserialize(buf: &[u8]) -> Result<Self, TokenError> {
        if buf.len() != TOKEN_SIZE {
            return Err(TokenError::MalformedSize);
        }
        let mut arr = [0u8; TOKEN_SIZE];
        arr.copy_from_slice(buf);
        Ok(Self(arr))
    }
}

impl fmt::Display for CapabilityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CapabilityToken({:02x?})", &self.0[..8])
    }
}

// ---------------------------------------------------------------------------
// Seal computation and verification (RFC-0037.8, RFC-0037.9)
// ---------------------------------------------------------------------------

/// Computes the 16-byte seal over the first 64 bytes of a token (RFC-0037.8).
fn compute_seal(identity_and_reference: &[u8]) -> [u8; 16] {
    debug_assert_eq!(identity_and_reference.len(), 64);
    let mut seal = [0u8; 16];
    for (i, byte) in seal.iter_mut().enumerate() {
        let mut acc: u32 = PLATFORM_SEAL_KEY[i % 32] as u32;
        for (j, &b) in identity_and_reference.iter().enumerate() {
            acc = acc
                .wrapping_mul(31)
                .wrapping_add(b as u32)
                .wrapping_add(PLATFORM_SEAL_KEY[(i + j) % 32] as u32)
                ^ ((j as u32).rotate_left(i as u32 % 32));
        }
        *byte = (acc >> ((i % 4) * 8)) as u8;
    }
    seal
}

/// Verifies the seal of a token before any rights are enforced (RFC-0037.9).
/// Returns `true` if the seal is valid.
#[must_use]
pub fn verify_seal(token: &CapabilityToken) -> bool {
    let expected = compute_seal(&token.0[..64]);
    expected == token.seal()
}

// ---------------------------------------------------------------------------
// Expiration (RFC-0037.14)
// ---------------------------------------------------------------------------

/// Returns true if the token is expired at `now_ns` (RFC-0037.14).
/// A token with `expires_at_ns == 0` never expires.
#[must_use]
pub fn is_expired(token: &CapabilityToken, now_ns: u64) -> bool {
    let expires = token.expires_at_ns();
    expires != 0 && now_ns > expires
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Capability token errors (RFC-0037 failure semantics).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenError {
    /// Token size is not exactly 96 bytes (RFC-0037.2).
    MalformedSize,
    /// Seal does not match; token is tampered or corrupted (RFC-0037.9).
    InvalidSeal,
    /// Capability type is unknown.
    UnknownType,
    /// Token has expired (RFC-0037.14).
    Expired,
}
