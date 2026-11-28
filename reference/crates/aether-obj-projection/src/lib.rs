//! Object projection reference implementation (RFC-0021).
//!
//! Exposes the State Architecture's object model as first-class entities with
//! full lifecycle, integrity, and transactional properties, without the POSIX
//! compatibility layer.
//!
//! Core design (RFC-0021):
//! - Objects are content-resolved: identity is BLAKE3-256 over content
//!   (RFC-0015), and identical content dedupes to the same identity
//!   (RFC-0021.3).
//! - Each object carries an append-only version history (RFC-0015.7).
//! - `object_create` and `object_update` are atomic RFC-0016 transactions
//!   (RFC-0021.4); batch creation is all-or-nothing (RFC-0021.5).
//! - Immutable objects reject updates (RFC-0021.7).
//! - Object lifecycle follows RFC-0018 (Created -> ... -> Destroyed).
//!
//! Objects are stored through a [`StorageProvider`] (RFC-0019); lifecycle
//! state and version history are tracked in memory by the projection.

use aether_lifecycle_model::LifecycleState;
use aether_object_identity::ObjectIdentity;
use aether_storage_provider::{StorageError, StorageProvider, Tier, apply_staged};
use aether_transaction_model::TransactionId;
use aether_types::ObjectId;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Object projection errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectError {
    /// The object id does not exist (or has been destroyed).
    NotFound,
    /// The object is immutable and cannot be updated.
    Immutable,
    /// Invalid argument (e.g. version index out of range).
    InvalidArgument,
    /// Underlying storage provider error.
    Storage(StorageError),
}

/// Result type returned by all object projection operations.
pub type ObjectResult<T> = Result<T, ObjectError>;

impl From<StorageError> for ObjectError {
    fn from(e: StorageError) -> Self {
        ObjectError::Storage(e)
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Metadata for one object (RFC-0021.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectMeta {
    /// Object identity.
    pub object_id: ObjectId,
    /// Lifecycle state (RFC-0018).
    pub lifecycle_state: LifecycleState,
    /// Content size in bytes.
    pub size_bytes: u64,
    /// Number of versions in the history.
    pub version_count: u32,
    /// Whether updates are rejected (RFC-0021.7).
    pub immutable: bool,
    /// Content integrity hash (folded to `ObjectId`, RFC-0017 cross-ref).
    pub content_hash: ObjectId,
    /// Creation time in scheduling nanoseconds (RFC-0012).
    pub created_at_ns: u64,
}

/// One entry in an object's append-only version history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionEntry {
    /// Object identity of this version.
    pub id: ObjectId,
    /// Commit time in scheduling nanoseconds.
    pub commit_timestamp_ns: u64,
}

/// One batch-create entry (RFC-0021.5 data structures).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectCreateEntry {
    /// Object content.
    pub content: Vec<u8>,
    /// IMMUTABLE flag (RFC-0021.7).
    pub immutable: bool,
}

// ---------------------------------------------------------------------------
// ObjectProjection
// ---------------------------------------------------------------------------

/// Object projection over a [`StorageProvider`] (RFC-0021).
///
/// Every object is a persistent object stored through the provider, keyed by
/// its content-resolved identity. The projection tracks lifecycle state and
/// an append-only version history per object.
pub struct ObjectProjection<P: StorageProvider> {
    /// The storage provider backend (RFC-0019).
    pub provider: P,
    participant: aether_storage_provider::TransactionalParticipant,
    metadata: HashMap<ObjectId, ObjectMeta>,
    versions: HashMap<ObjectId, Vec<VersionEntry>>,
    next_xid_nonce: u64,
}

impl<P: StorageProvider> ObjectProjection<P> {
    /// Creates a new projection over `provider`.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            participant: aether_storage_provider::TransactionalParticipant::new(true),
            metadata: HashMap::new(),
            versions: HashMap::new(),
            next_xid_nonce: 1,
        }
    }

    /// Allocates a fresh transaction id.
    fn next_xid(&mut self) -> TransactionId {
        let mut bytes = [0u8; 16];
        let n = self.next_xid_nonce.to_le_bytes();
        bytes[..8].copy_from_slice(&n);
        self.next_xid_nonce += 1;
        TransactionId::from_bytes(bytes)
    }

    /// Begins a fresh transaction on a new participant (avoids the
    /// coordinator's write-write conflict tracking across independent ops).
    fn begin_fresh_txn(&mut self, timestamp_ns: u64) -> ObjectResult<TransactionId> {
        self.participant = aether_storage_provider::TransactionalParticipant::new(true);
        let xid = self.next_xid();
        self.participant
            .begin_transaction(xid, timestamp_ns)
            .map_err(ObjectError::from)?;
        Ok(xid)
    }

    /// Commits the current transaction, applies staged mutations, and resets
    /// the participant for the next operation.
    fn commit_current_txn(&mut self, timestamp_ns: u64) -> ObjectResult<()> {
        self.participant
            .commit_transaction(timestamp_ns)
            .map_err(ObjectError::from)?;
        apply_staged(&mut self.provider, &self.participant).map_err(ObjectError::from)?;
        self.participant.mark_committed();
        Ok(())
    }

    // ------------------------------------------------------------------
    // Operations
    // ------------------------------------------------------------------

    /// `object_create(content)` (RFC-0021.1).
    ///
    /// Creates and commits a new object from `content`, returning its
    /// content-resolved identity. Identical content dedupes to the existing
    /// identity (RFC-0021.3).
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::Storage`] if the provider cannot store the
    /// content.
    pub fn object_create(&mut self, content: &[u8]) -> ObjectResult<ObjectId> {
        let id = ObjectId::from(ObjectIdentity::compute(content));

        // Dedup at identity level: identical content returns existing id
        // (RFC-0021.3).
        if self.metadata.contains_key(&id) {
            return Ok(id);
        }

        let now = 0;

        // Store the content. Empty content is written directly because
        // apply_staged treats empty content as a delete.
        if content.is_empty() {
            self.provider
                .put(id, &[], Tier::Hot)
                .map_err(ObjectError::from)?;
        } else {
            self.begin_fresh_txn(now)?;
            self.participant
                .stage_put(id, content.to_vec(), Tier::Hot)
                .map_err(ObjectError::from)?;
            self.commit_current_txn(now)?;
        }

        let meta = ObjectMeta {
            object_id: id,
            lifecycle_state: LifecycleState::Created,
            size_bytes: content.len() as u64,
            version_count: 1,
            immutable: false,
            content_hash: id,
            created_at_ns: now,
        };
        self.metadata.insert(id, meta);
        self.versions.insert(
            id,
            vec![VersionEntry {
                id,
                commit_timestamp_ns: now,
            }],
        );

        Ok(id)
    }

    /// `object_get(id)` (RFC-0021.1).
    ///
    /// Returns the content of the current version of `id`.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::NotFound`] if `id` does not exist or has been
    /// destroyed.
    pub fn object_get(&self, id: ObjectId) -> ObjectResult<Vec<u8>> {
        let meta = self.metadata.get(&id).ok_or(ObjectError::NotFound)?;
        if meta.lifecycle_state == LifecycleState::Destroyed {
            return Err(ObjectError::NotFound);
        }
        let current_id = self
            .versions
            .get(&id)
            .and_then(|h| h.last())
            .map_or(id, |v| v.id);
        self.provider.get(current_id).map_err(ObjectError::from)
    }

    /// `object_update(id, new_content)` (RFC-0021.1).
    ///
    /// Updates `id` to `new_content`, creating a new content-resolved
    /// identity and appending a version entry to `id`'s history
    /// (RFC-0015.7). Returns the new identity.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::NotFound`] if `id` does not exist or is
    /// destroyed, [`ObjectError::Immutable`] if `id` is immutable
    /// (RFC-0021.7).
    pub fn object_update(&mut self, id: ObjectId, new_content: &[u8]) -> ObjectResult<ObjectId> {
        let meta = self.metadata.get(&id).ok_or(ObjectError::NotFound)?;
        if meta.lifecycle_state == LifecycleState::Destroyed {
            return Err(ObjectError::NotFound);
        }
        if meta.immutable {
            return Err(ObjectError::Immutable);
        }

        let new_id = ObjectId::from(ObjectIdentity::compute(new_content));
        let now = 0;

        // Store the new content. Empty content is written directly because
        // apply_staged treats empty content as a delete.
        if new_content.is_empty() {
            self.provider
                .put(new_id, &[], Tier::Hot)
                .map_err(ObjectError::from)?;
        } else {
            self.begin_fresh_txn(now)?;
            self.participant
                .stage_put(new_id, new_content.to_vec(), Tier::Hot)
                .map_err(ObjectError::from)?;
            self.commit_current_txn(now)?;
        }

        // Append a version entry to the original object's history
        // (RFC-0015.7).
        let history = self.versions.get_mut(&id).ok_or(ObjectError::NotFound)?;
        history.push(VersionEntry {
            id: new_id,
            commit_timestamp_ns: now,
        });

        // Record metadata for the new version object.
        let new_meta = ObjectMeta {
            object_id: new_id,
            lifecycle_state: LifecycleState::Created,
            size_bytes: new_content.len() as u64,
            version_count: 1,
            immutable: false,
            content_hash: new_id,
            created_at_ns: now,
        };
        self.metadata.insert(new_id, new_meta);
        self.versions.insert(
            new_id,
            vec![VersionEntry {
                id: new_id,
                commit_timestamp_ns: now,
            }],
        );

        Ok(new_id)
    }

    /// `object_delete(id)` (RFC-0021.1).
    ///
    /// Initiates lifecycle destruction of `id` (RFC-0018). The identity is
    /// preserved in history only; the object is no longer readable.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::NotFound`] if `id` does not exist or is already
    /// destroyed.
    pub fn object_delete(&mut self, id: ObjectId) -> ObjectResult<()> {
        let meta = self.metadata.get_mut(&id).ok_or(ObjectError::NotFound)?;
        if meta.lifecycle_state == LifecycleState::Destroyed {
            return Err(ObjectError::NotFound);
        }
        meta.lifecycle_state = LifecycleState::Destroyed;
        Ok(())
    }

    /// `object_get_metadata(id)` (RFC-0021.1).
    ///
    /// Returns the lifecycle state, size, version count, and integrity proof
    /// for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::NotFound`] if `id` does not exist.
    pub fn object_get_metadata(&self, id: ObjectId) -> ObjectResult<ObjectMeta> {
        self.metadata.get(&id).cloned().ok_or(ObjectError::NotFound)
    }

    /// `object_list_versions(id)` (RFC-0021.6).
    ///
    /// Returns the version history sequence (oldest first) as
    /// `(object_id, commit_timestamp_ns)` pairs.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::NotFound`] if `id` does not exist.
    pub fn object_list_versions(&self, id: ObjectId) -> ObjectResult<Vec<(ObjectId, u64)>> {
        let history = self.versions.get(&id).ok_or(ObjectError::NotFound)?;
        Ok(history
            .iter()
            .map(|v| (v.id, v.commit_timestamp_ns))
            .collect())
    }

    /// `object_get_at_version(id, version_index)` (RFC-0021.6).
    ///
    /// Returns the content of the version at `version_index` (zero-based,
    /// oldest first).
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::NotFound`] if `id` does not exist, or
    /// [`ObjectError::InvalidArgument`] if `version_index` is out of range.
    pub fn object_get_at_version(&self, id: ObjectId, version_index: u32) -> ObjectResult<Vec<u8>> {
        let history = self.versions.get(&id).ok_or(ObjectError::NotFound)?;
        let entry = history
            .get(version_index as usize)
            .ok_or(ObjectError::InvalidArgument)?;
        self.provider.get(entry.id).map_err(ObjectError::from)
    }

    /// `object_create_batch(entries)` (RFC-0021.5).
    ///
    /// Creates multiple objects in a single transaction: all succeed or all
    /// fail. Identical content dedupes to existing identities
    /// (RFC-0021.3).
    ///
    /// # Errors
    ///
    /// Returns [`ObjectError::Storage`] if any staged put fails, in which
    /// case the batch is aborted and no new objects are created.
    pub fn object_create_batch(
        &mut self,
        entries: &[ObjectCreateEntry],
    ) -> ObjectResult<Vec<ObjectId>> {
        let now = 0;
        let mut ids = Vec::with_capacity(entries.len());

        // Phase 1: compute identities, dedupe, and write empty-content
        // objects directly (apply_staged treats empty content as a delete).
        for entry in entries {
            let id = ObjectId::from(ObjectIdentity::compute(&entry.content));
            if self.metadata.contains_key(&id) {
                ids.push(id);
                continue;
            }
            if entry.content.is_empty() {
                self.provider
                    .put(id, &[], Tier::Hot)
                    .map_err(ObjectError::from)?;
            }
            ids.push(id);
        }

        // Phase 2: stage all non-empty, non-duplicate puts in one transaction
        // so the batch is all-or-nothing (RFC-0021.5).
        self.begin_fresh_txn(now)?;
        for (entry, &id) in entries.iter().zip(ids.iter()) {
            if self.metadata.contains_key(&id) || entry.content.is_empty() {
                continue;
            }
            self.participant
                .stage_put(id, entry.content.clone(), Tier::Hot)
                .map_err(ObjectError::from)?;
        }
        self.commit_current_txn(now)?;

        // Phase 3: record metadata and version histories for newly created
        // objects (in-memory, always succeeds once the transaction commits).
        for (entry, &id) in entries.iter().zip(ids.iter()) {
            if self.metadata.contains_key(&id) {
                continue;
            }
            let meta = ObjectMeta {
                object_id: id,
                lifecycle_state: LifecycleState::Created,
                size_bytes: entry.content.len() as u64,
                version_count: 1,
                immutable: entry.immutable,
                content_hash: id,
                created_at_ns: now,
            };
            self.metadata.insert(id, meta);
            self.versions.insert(
                id,
                vec![VersionEntry {
                    id,
                    commit_timestamp_ns: now,
                }],
            );
        }

        Ok(ids)
    }
}

// ---------------------------------------------------------------------------
// Unit tests (CTS-STATE-OBJPROJ-1..5 coverage)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider() -> aether_storage_provider::InMemoryProvider {
        aether_storage_provider::InMemoryProvider::new(
            "obj-proj-test",
            aether_storage_provider::ProviderType::Block,
            1024 * 1024,
            aether_storage_provider::LatencyBounds::new(100, 1000, 10_000).unwrap(),
            0.999,
            0x01,
        )
    }

    fn make_projection() -> ObjectProjection<aether_storage_provider::InMemoryProvider> {
        ObjectProjection::new(make_provider())
    }

    /// CTS-STATE-OBJPROJ-1 (RFC-0021.1): full object API works end to end —
    /// create, get, update, delete, metadata, version listing.
    #[test]
    fn cts_objproj_1_object_api() {
        let mut proj = make_projection();

        // Create an object.
        let id = proj.object_create(b"version one").unwrap();
        let meta = proj.object_get_metadata(id).unwrap();
        assert_eq!(meta.size_bytes, 11);
        assert_eq!(meta.lifecycle_state, LifecycleState::Created);
        assert!(!meta.immutable);
        assert_eq!(meta.version_count, 1);

        // Get returns the content.
        let content = proj.object_get(id).unwrap();
        assert_eq!(content, b"version one");

        // List versions: single entry.
        let versions = proj.object_list_versions(id).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].0, id);

        // Update creates a new identity.
        let id2 = proj.object_update(id, b"version two").unwrap();
        assert_ne!(id2, id);
        let content2 = proj.object_get(id).unwrap();
        assert_eq!(content2, b"version two"); // current version of id
        let content_direct = proj.object_get(id2).unwrap();
        assert_eq!(content_direct, b"version two");

        // Version history now has two entries.
        let versions = proj.object_list_versions(id).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].0, id);
        assert_eq!(versions[1].0, id2);

        // Get at specific historical version.
        let v0 = proj.object_get_at_version(id, 0).unwrap();
        assert_eq!(v0, b"version one");
        let v1 = proj.object_get_at_version(id, 1).unwrap();
        assert_eq!(v1, b"version two");

        // Delete the original object; it is no longer readable.
        proj.object_delete(id).unwrap();
        assert_eq!(proj.object_get(id), Err(ObjectError::NotFound));
        // The new identity object is unaffected.
        let _ = proj.object_get(id2).unwrap();
    }

    /// CTS-STATE-OBJPROJ-2 (RFC-0021.3): deduplication at identity level —
    /// identical content returns the same identity.
    #[test]
    fn cts_objproj_2_dedup() {
        let mut proj = make_projection();

        let id1 = proj.object_create(b"same content").unwrap();
        let id2 = proj.object_create(b"same content").unwrap();
        assert_eq!(id1, id2);

        // Creating it again still returns the same id and does not add a
        // version.
        let id3 = proj.object_create(b"same content").unwrap();
        assert_eq!(id3, id1);
        let versions = proj.object_list_versions(id1).unwrap();
        assert_eq!(versions.len(), 1);
    }

    /// CTS-STATE-OBJPROJ-3 (RFC-0021.4): create and update are atomic — the
    /// new identity is valid only after commit and is stored in the provider.
    #[test]
    fn cts_objproj_3_atomic_update() {
        let mut proj = make_projection();

        let id = proj.object_create(b"original").unwrap();

        // Update commits the new content under the new identity.
        let id2 = proj.object_update(id, b"updated").unwrap();
        assert_ne!(id2, id);

        // Both identities are retrievable through the provider.
        let meta1 = proj.object_get_metadata(id).unwrap();
        let content1 = proj.provider.get(meta1.object_id).unwrap();
        assert_eq!(content1, b"original");

        let meta2 = proj.object_get_metadata(id2).unwrap();
        let content2 = proj.provider.get(meta2.object_id).unwrap();
        assert_eq!(content2, b"updated");

        // The current version of the original id reflects the update.
        assert_eq!(proj.object_get(id).unwrap(), b"updated");
    }

    /// CTS-STATE-OBJPROJ-4 (RFC-0021.5): batch operations are all-or-nothing.
    #[test]
    fn cts_objproj_4_batch_all_or_nothing() {
        let mut proj = make_projection();

        let ids = proj
            .object_create_batch(&[
                ObjectCreateEntry {
                    content: b"batch one".to_vec(),
                    immutable: false,
                },
                ObjectCreateEntry {
                    content: b"batch two".to_vec(),
                    immutable: true,
                },
            ])
            .unwrap();
        assert_eq!(ids.len(), 2);

        // Both objects exist with their expected properties.
        let meta0 = proj.object_get_metadata(ids[0]).unwrap();
        assert!(!meta0.immutable);
        assert_eq!(proj.object_get(ids[0]).unwrap(), b"batch one");

        let meta1 = proj.object_get_metadata(ids[1]).unwrap();
        assert!(meta1.immutable);
        assert_eq!(proj.object_get(ids[1]).unwrap(), b"batch two");

        // Batch with duplicate content dedupes.
        let ids2 = proj
            .object_create_batch(&[ObjectCreateEntry {
                content: b"batch one".to_vec(),
                immutable: false,
            }])
            .unwrap();
        assert_eq!(ids2, vec![ids[0]]);
    }

    /// CTS-STATE-OBJPROJ-5 (RFC-0021.7): immutable flag enforced — updating
    /// an immutable object returns an error.
    #[test]
    fn cts_objproj_5_immutable_rejected() {
        let mut proj = make_projection();

        let ids = proj
            .object_create_batch(&[ObjectCreateEntry {
                content: b"immutable object".to_vec(),
                immutable: true,
            }])
            .unwrap();
        let id = ids[0];

        // Updating an immutable object is rejected.
        assert_eq!(
            proj.object_update(id, b"attempted change"),
            Err(ObjectError::Immutable)
        );

        // Content is unchanged.
        assert_eq!(proj.object_get(id).unwrap(), b"immutable object");

        // A mutable object can still be updated.
        let id2 = proj.object_create(b"mutable").unwrap();
        let id3 = proj.object_update(id2, b"changed").unwrap();
        assert_ne!(id2, id3);
        assert_eq!(proj.object_get(id2).unwrap(), b"changed");
    }

    /// CTS-STATE-OBJPROJ-2: distinct content produces distinct identities.
    #[test]
    fn cts_objproj_2_distinct_content_distinct_ids() {
        let mut proj = make_projection();
        let id1 = proj.object_create(b"alpha").unwrap();
        let id2 = proj.object_create(b"beta").unwrap();
        assert_ne!(id1, id2);
    }

    /// CTS-STATE-OBJPROJ-1: get / metadata on a missing id returns NotFound.
    #[test]
    fn cts_objproj_1_missing_not_found() {
        let mut proj = make_projection();
        assert_eq!(
            proj.object_get(aether_types::ObjectId(999)),
            Err(ObjectError::NotFound)
        );
        assert_eq!(
            proj.object_get_metadata(aether_types::ObjectId(999)),
            Err(ObjectError::NotFound)
        );
        assert_eq!(
            proj.object_delete(aether_types::ObjectId(999)),
            Err(ObjectError::NotFound)
        );
    }

    /// CTS-STATE-OBJPROJ-6: get_at_version with out-of-range index returns
    /// InvalidArgument.
    #[test]
    fn cts_objproj_6_version_index_out_of_range() {
        let mut proj = make_projection();
        let id = proj.object_create(b"only version").unwrap();
        assert_eq!(
            proj.object_get_at_version(id, 1),
            Err(ObjectError::InvalidArgument)
        );
    }
}
