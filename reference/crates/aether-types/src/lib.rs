//! Provider-neutral capability types for the executable reference model.

use core::fmt;

/// A capability domain. Domain zero is reserved and never valid.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DomainId(u16);

impl DomainId {
    /// Creates a non-zero domain identifier.
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    /// Returns the integer representation.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Stable identity of a resource in the reference model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(pub u128);

/// Resource type checked independently from rights.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ObjectType {
    /// Memory region.
    Memory = 1,
    /// IPC endpoint.
    Endpoint = 2,
    /// Persistent state object.
    State = 3,
}

/// Compact host-provider capability-right set. Unknown bits are rejected.
///
/// These are semantic labels used inside the executable reference model, not
/// the canonical RFC-0037 serialized rights bit positions. Canonical import
/// and export must translate explicitly at the serialization boundary.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Rights(u16);

impl Rights {
    /// Read resource contents.
    pub const READ: Self = Self(1 << 0);
    /// Modify resource contents.
    pub const WRITE: Self = Self(1 << 1);
    /// Invoke an endpoint or operation.
    pub const INVOKE: Self = Self(1 << 2);
    /// Provider-reference grant authority for derivation and transfer.
    ///
    /// This is a host-model aggregate, not a canonical RFC-0037 token bit.
    pub const GRANT: Self = Self(1 << 3);
    /// Execute instructions from a memory region.
    pub const EXECUTE: Self = Self(1 << 4);
    /// Create or remove an address-space mapping for a memory region.
    pub const MAP: Self = Self(1 << 5);
    /// Explicit authority to revoke a revocable capability.
    pub const REVOKE: Self = Self(1 << 6);
    /// Empty rights set.
    pub const NONE: Self = Self(0);
    const KNOWN: u16 = Self::READ.0
        | Self::WRITE.0
        | Self::INVOKE.0
        | Self::GRANT.0
        | Self::EXECUTE.0
        | Self::MAP.0
        | Self::REVOKE.0;

    /// Creates a rights set if every bit is defined.
    #[must_use]
    pub const fn from_bits(bits: u16) -> Option<Self> {
        if bits & !Self::KNOWN == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the bit representation.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Returns whether `self` contains every requested right.
    #[must_use]
    pub const fn contains(self, requested: Self) -> bool {
        self.0 & requested.0 == requested.0
    }

    /// Returns the union. Used to build requested sets, not elevate authority.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl fmt::Debug for Rights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rights({:#06x})", self.0)
    }
}

/// Scheduling-clock instant supplied by the scheduler/reference harness.
///
/// This is deliberately not backed by a host wall clock. RFC-0037 expiration
/// uses scheduling time; the caller must provide the current scheduler value.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchedulingTime(u64);

impl SchedulingTime {
    /// Zero scheduling time, useful only for explicit unbounded test fixtures.
    pub const ZERO: Self = Self(0);

    /// Constructs a scheduling-time instant in nanoseconds.
    #[must_use]
    pub const fn from_nanos(value: u64) -> Self { Self(value) }

    /// Returns the nanosecond value.
    #[must_use]
    pub const fn as_nanos(self) -> u64 { self.0 }
}

/// Behaviour flags carried by a capability token (RFC-0037.7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityFlags(u8);

impl CapabilityFlags {
    /// Capability is transient and is lost when its domain is destroyed.
    pub const TRANSIENT: Self = Self(0x01);
    /// Capability is subject to explicit revocation.
    pub const REVOCABLE: Self = Self(0x02);
    /// Capability has a stable lookup name.
    pub const NAMED: Self = Self(0x04);
    /// Capability is bound to one holder domain and cannot be transferred.
    pub const BOUND: Self = Self(0x08);
    /// Capability is sealed with the platform integrity key.
    pub const SEALED: Self = Self(0x10);
    /// No behaviour flags.
    pub const NONE: Self = Self(0);
    const KNOWN: u8 = 0x1f;

    /// Creates a flag set only when no unknown RFC-0037 flag bits are present.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !Self::KNOWN == 0 { Some(Self(bits)) } else { None }
    }

    /// Returns the serialized flag bits.
    #[must_use]
    pub const fn bits(self) -> u8 { self.0 }

    /// Returns whether every flag in other is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns the union of two flag sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
}

/// Lifecycle metadata corresponding to RFC-0037 token fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityLifecycle {
    /// Token behaviour flags.
    pub flags: CapabilityFlags,
    /// Creation timestamp in scheduler time.
    pub creation_timestamp_ns: u64,
    /// Absolute scheduling-time deadline; zero means no expiration.
    pub expires_at_ns: u64,
}

impl CapabilityLifecycle {
    /// Builds lifecycle metadata without consulting a host clock.
    #[must_use]
    pub const fn new(flags: CapabilityFlags, creation_timestamp_ns: u64, expires_at_ns: u64) -> Self {
        Self { flags, creation_timestamp_ns, expires_at_ns }
    }

    /// Compatibility policy used by the original host-reference root helper.
    /// This is an explicit host-model default, not a normative token default.
    #[must_use]
    pub const fn permanent_revocable() -> Self {
        Self::new(CapabilityFlags::REVOCABLE, 0, 0)
    }

    /// RFC-0037.14 uses strict current scheduling time greater than expiry.
    #[must_use]
    pub const fn is_expired(self, now: SchedulingTime) -> bool {
        self.expires_at_ns != 0 && now.as_nanos() > self.expires_at_ns
    }
}

/// Half-open byte range authorized by a capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bounds {
    /// First authorized byte.
    pub start: u64,
    /// Exclusive end byte.
    pub end: u64,
}

impl Bounds {
    /// Constructs a non-empty range.
    #[must_use]
    pub const fn new(start: u64, end: u64) -> Option<Self> {
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Returns whether `other` is a subset of this range.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

/// Semantic authority stored behind a live provider handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capability {
    /// Authorized resource.
    pub object: ObjectId,
    /// Resource type.
    pub object_type: ObjectType,
    /// Authorized operations.
    pub rights: Rights,
    /// Authorized byte range.
    pub bounds: Bounds,
}

/// Opaque, domain-local live handle. Its layout is not an ABI.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Handle(u64);

impl Handle {
    const PROVIDER: u64 = 1;

    /// Creates a provider handle from validated components.
    #[must_use]
    pub const fn encode(domain: DomainId, slot: u32, generation: u16) -> Option<Self> {
        if slot > 0x00ff_ffff || generation == 0 {
            return None;
        }
        Some(Self(
            (Self::PROVIDER << 56)
                | ((domain.get() as u64) << 40)
                | ((slot as u64) << 16)
                | generation as u64,
        ))
    }

    /// Treats attacker-controlled bits as a candidate handle for validation.
    #[must_use]
    pub const fn from_untrusted(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw representation for transport inside one domain.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Decodes components, rejecting another provider or reserved values.
    #[must_use]
    pub const fn decode(self) -> Option<(DomainId, u32, u16)> {
        if self.0 >> 56 != Self::PROVIDER {
            return None;
        }
        let domain = ((self.0 >> 40) & 0xffff) as u16;
        let slot = ((self.0 >> 16) & 0x00ff_ffff) as u32;
        let generation = (self.0 & 0xffff) as u16;
        match (DomainId::new(domain), generation) {
            (Some(domain), 1..=u16::MAX) => Some((domain, slot, generation)),
            _ => None,
        }
    }
}

/// Stable semantic failures shared by the model and provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapError {
    /// Bootstrap witness belongs to a different authority universe.
    WrongBootstrap,
    /// Handle bits are malformed or belong to another provider.
    MalformedHandle,
    /// Handle is meaningful only in another domain.
    WrongDomain,
    /// Slot does not exist or is vacant.
    Missing,
    /// Slot generation no longer matches.
    Stale,
    /// Capability or one of its ancestors was revoked.
    Revoked,
    /// Capability scheduling-time expiry has passed.
    Expired,
    /// Explicit revocation was attempted without CAPF_REVOCABLE.
    NotRevocable,
    /// Capability is bound to a different domain or cannot be transferred.
    DomainBound,
    /// Requested right is absent.
    Rights,
    /// Requested bounds exceed authority.
    Bounds,
    /// Resource type does not match.
    Type,
    /// Table cannot represent another entry.
    Exhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_round_trip_preserves_every_field() {
        let domain = DomainId::new(0xabcd).unwrap();
        let handle = Handle::encode(domain, 0x00ab_cdef, 0x1234).unwrap();
        assert_eq!(handle.decode(), Some((domain, 0x00ab_cdef, 0x1234)));
    }

    #[test]
    fn malformed_handle_and_unknown_right_are_rejected() {
        assert_eq!(Handle::from_untrusted(0).decode(), None);
        assert_eq!(Rights::from_bits(1 << 15), None);
        assert_eq!(CapabilityFlags::from_bits(0x20), None);
    }

    #[test]
    fn scheduling_expiry_uses_strict_greater_than() {
        let lifecycle = CapabilityLifecycle::new(CapabilityFlags::REVOCABLE, 5, 10);
        assert!(!lifecycle.is_expired(SchedulingTime::from_nanos(10)));
        assert!(lifecycle.is_expired(SchedulingTime::from_nanos(11)));
        assert!(!CapabilityLifecycle::new(CapabilityFlags::NONE, 5, 0)
            .is_expired(SchedulingTime::from_nanos(u64::MAX)));
    }
}
