//! Content-resolved object identity reference model (RFC-0015).
//!
//! Every persistent object receives a unique, immutable, content-resolved
//! identifier computed as BLAKE3-256 over the object's canonical byte
//! representation. Identity is independent of location, metadata, and
//! capability authority (RFC-0015.11, I-13).
//!
//! The Merkle-DAG construction from RFC-0017 is not yet implemented; composite
//! objects hash their raw content bytes directly (RFC-0015.5).

use aether_types::ObjectId;
use core::fmt;

/// Reason a new version was committed (RFC-0015.7, `version_record.reason`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum VersionReason {
    /// Object was first created.
    Create = 0,
    /// Object content was updated, producing a new identity.
    Update = 1,
    /// Object was reconstructed from an uncorrupted replica (RFC-0015.13).
    Reconstruct = 2,
}

/// Consistency guarantee declared for a replica (RFC-0015.10).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ConsistencyLevel {
    /// All replicas are guaranteed byte-identical.
    Strong = 0,
    /// Replicas may diverge within documented bounds (eventual consistency).
    Eventual = 1,
}

/// Fixed-width, content-resolved object identity (RFC-0015.1, RFC-0015.2).
///
/// A 32-byte BLAKE3-256 output over the object's canonical representation.
/// Two objects with byte-identical canonical representations produce identical
/// identities (RFC-0015.3). Identity is immutable: changing content changes the
/// identity (RFC-0015.4).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectIdentity(pub [u8; 32]);

const IDENTITY_LEN: usize = 32;

impl ObjectIdentity {
    /// Computes the content-resolved identity of `content` using BLAKE3-256
    /// (RFC-0015.1, RFC-0015.2, RFC-0015.5).
    #[must_use]
    pub fn compute(content: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(content);
        let mut bytes = [0u8; IDENTITY_LEN];
        bytes.copy_from_slice(hasher.finalize().as_bytes());
        Self(bytes)
    }

    /// Returns the raw 32-byte hash.
    #[must_use]
    pub const fn bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl AsRef<[u8]> for ObjectIdentity {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for ObjectIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

/// One entry in an object's append-only version history (RFC-0015.7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionRecord {
    /// Identity of the object at this version.
    pub id: ObjectIdentity,
    /// Scheduling-time commit timestamp in nanoseconds (not wall-clock).
    pub commit_timestamp_ns: u64,
    /// Why this version was committed.
    pub reason: VersionReason,
}

/// Append-only, immutable version history for an object (RFC-0015.7, RFC-0015.8).
///
/// Entries are stored oldest first. No entry may be removed or reordered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionHistory {
    /// The object's current (latest) identity.
    pub current_id: ObjectIdentity,
    /// Number of entries in the history.
    pub version_count: u32,
    /// Version records, oldest first.
    pub entries: Vec<VersionRecord>,
}

impl VersionHistory {
    /// Creates a new history with a single initial version.
    #[must_use]
    pub fn initial(id: ObjectIdentity, commit_timestamp_ns: u64) -> Self {
        Self {
            current_id: id,
            version_count: 1,
            entries: vec![VersionRecord {
                id,
                commit_timestamp_ns,
                reason: VersionReason::Create,
            }],
        }
    }

    /// Appends a new version to the history. The new identity becomes current.
    pub fn append(&mut self, id: ObjectIdentity, commit_timestamp_ns: u64, reason: VersionReason) {
        self.entries.push(VersionRecord {
            id,
            commit_timestamp_ns,
            reason,
        });
        self.current_id = id;
        self.version_count += 1;
    }

    /// Returns the full identity sequence in chronological order with commit
    /// timestamps (RFC-0015.8).
    #[must_use]
    pub fn sequence(&self) -> Vec<(ObjectIdentity, u64)> {
        self.entries
            .iter()
            .map(|r| (r.id, r.commit_timestamp_ns))
            .collect()
    }
}

/// Location-specific replica descriptor (RFC-0015.9, RFC-0015.10).
///
/// Identity does not distinguish replicas; this structure provides the
/// separate replica identifier for location-specific operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplicaDescriptor {
    /// The shared object identity of all replicas.
    pub object_id: ObjectIdentity,
    /// Per-replica sequence number.
    pub replica_serial: u64,
    /// Provider-specific storage location (opaque bytes).
    pub location: Vec<u8>,
    /// Consistency guarantee for this replica.
    pub consistency_level: ConsistencyLevel,
}

/// Convert an [`ObjectIdentity`] into an `aether_types::ObjectId` for use in
/// the capability and state reference models. Identity and capability remain
/// orthogonal (RFC-0015.11): this is a naming anchor, not authority.
impl From<ObjectIdentity> for ObjectId {
    fn from(identity: ObjectIdentity) -> Self {
        let bytes = identity.bytes();
        // ObjectId is u128; fold the first 16 bytes of the hash.
        let mut value: u128 = 0;
        for &b in bytes.iter().take(16) {
            value = (value << 8) | u128::from(b);
        }
        ObjectId(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_content_produces_identical_identity() {
        let a = ObjectIdentity::compute(b"canonical object bytes");
        let b = ObjectIdentity::compute(b"canonical object bytes");
        assert_eq!(a, b);
    }

    #[test]
    fn differing_content_produces_differing_identity() {
        let a = ObjectIdentity::compute(b"version one");
        let b = ObjectIdentity::compute(b"version two");
        assert_ne!(a, b);
    }

    #[test]
    fn identity_is_32_bytes() {
        let id = ObjectIdentity::compute(b"x");
        assert_eq!(id.bytes().len(), 32);
    }

    #[test]
    fn version_history_is_append_only_and_chronological() {
        let id1 = ObjectIdentity::compute(b"v1");
        let id2 = ObjectIdentity::compute(b"v2");
        let mut history = VersionHistory::initial(id1, 100);
        assert_eq!(history.version_count, 1);
        assert_eq!(history.current_id, id1);

        history.append(id2, 200, VersionReason::Update);
        assert_eq!(history.version_count, 2);
        assert_eq!(history.current_id, id2);

        let seq = history.sequence();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0], (id1, 100));
        assert_eq!(seq[1], (id2, 200));
    }

    #[test]
    fn replica_shares_identity_but_has_own_serial_and_location() {
        let id = ObjectIdentity::compute(b"replicated object");
        let r1 = ReplicaDescriptor {
            object_id: id,
            replica_serial: 1,
            location: b"/provider/a/slot-1".to_vec(),
            consistency_level: ConsistencyLevel::Strong,
        };
        let r2 = ReplicaDescriptor {
            object_id: id,
            replica_serial: 2,
            location: b"/provider/b/slot-2".to_vec(),
            consistency_level: ConsistencyLevel::Eventual,
        };
        assert_eq!(r1.object_id, r2.object_id);
        assert_ne!(r1.replica_serial, r2.replica_serial);
        assert_ne!(r1.location, r2.location);
    }

    #[test]
    fn identity_to_object_id_is_deterministic() {
        let id = ObjectIdentity::compute(b"deterministic");
        let oid: ObjectId = id.into();
        let oid2: ObjectId = id.into();
        assert_eq!(oid, oid2);
    }
}
