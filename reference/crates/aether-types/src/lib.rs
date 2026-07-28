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

/// Compact rights set. Unknown bits are rejected.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Rights(u16);

impl Rights {
    /// Read resource contents.
    pub const READ: Self = Self(1 << 0);
    /// Modify resource contents.
    pub const WRITE: Self = Self(1 << 1);
    /// Invoke an endpoint or operation.
    pub const INVOKE: Self = Self(1 << 2);
    /// Derive or transfer attenuated authority.
    pub const GRANT: Self = Self(1 << 3);
    /// Empty rights set.
    pub const NONE: Self = Self(0);
    const KNOWN: u16 = Self::READ.0 | Self::WRITE.0 | Self::INVOKE.0 | Self::GRANT.0;

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
    }
}
