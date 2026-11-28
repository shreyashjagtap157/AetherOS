//! Key–value projection reference implementation (RFC-0022).
//!
//! Maps arbitrary byte-string keys to persistent object identities, providing
//! get/put/delete/list with optional transactional grouping and atomic
//! compare-and-swap.
//!
//! Core design (RFC-0022):
//! - Keys are arbitrary byte strings, stored as-is; ordering is lexicographic
//!   over the byte representation (RFC-0022.2 / RFC-0022.3).
//! - Values are persistent objects (RFC-0015): a key maps to an `object_id`
//!   and the value's content is the object's bytes (RFC-0022.4).
//! - Value lifecycle follows the RFC-0018 Lifecycle FSM (RFC-0022.4).
//! - Key–value transactions are State Architecture transactions: multiple
//!   `kv_put`/`kv_delete` within a `kv_txn` commit atomically (RFC-0022.5).
//! - `kv_txn_commit` implements two-phase commit (RFC-0022.6).
//! - `kv_cas` atomically replaces a value only if the current value matches
//!   the expected identity, returning a conflict indicator otherwise
//!   (RFC-0022.7 / RFC-0022.8).
//!
//! Keys and the key→value index are tracked in memory by the projection;
//! value content is stored through a [`StorageProvider`] (RFC-0019).

use aether_lifecycle_model::LifecycleState;
use aether_object_identity::ObjectIdentity;
use aether_storage_provider::{StorageError, StorageProvider, Tier, apply_staged};
use aether_transaction_model::TransactionId;
use aether_types::ObjectId;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Key–value projection errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvError {
    /// The key does not exist.
    NotFound,
    /// The key already exists (put without overwrite semantics).
    AlreadyExists,
    /// CAS conflict: the current value does not match the expected value.
    Conflict,
    /// The transaction handle is unknown or already resolved.
    UnknownTransaction,
    /// Underlying storage provider error.
    Storage(StorageError),
}

/// Result type returned by all key–value projection operations.
pub type KvResult<T> = Result<T, KvError>;

impl From<StorageError> for KvError {
    fn from(e: StorageError) -> Self {
        KvError::Storage(e)
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// One pending write within a transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
struct KvPendingWrite {
    /// The key the write targets.
    key: Vec<u8>,
    /// The new value object id, or `None` to delete the key.
    value_id: Option<ObjectId>,
}

/// An open key–value transaction handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KvTransaction(pub TransactionId);

// ---------------------------------------------------------------------------
// KvProjection
// ---------------------------------------------------------------------------

/// Key–value projection over a [`StorageProvider`] (RFC-0022).
///
/// A key maps to a value object identity; value content lives in the
/// provider. The key index is maintained in memory.
pub struct KvProjection<P: StorageProvider> {
    /// The storage provider backend (RFC-0019).
    pub provider: P,
    /// Transactional participant driving two-phase commit (RFC-0019.9).
    participant: aether_storage_provider::TransactionalParticipant,
    /// Key → value object id index (RFC-0022.4).
    pub index: HashMap<Vec<u8>, ObjectId>,
    /// Lifecycle state per value object id (RFC-0018 / RFC-0022.4).
    lifecycle: HashMap<ObjectId, LifecycleState>,
    /// Pending writes keyed by open transaction (RFC-0022.5).
    pending: HashMap<TransactionId, Vec<KvPendingWrite>>,
    /// Next transaction id nonce.
    next_xid_nonce: u64,
}

impl<P: StorageProvider> KvProjection<P> {
    /// Creates a new projection over `provider`.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            participant: aether_storage_provider::TransactionalParticipant::new(true),
            index: HashMap::new(),
            lifecycle: HashMap::new(),
            pending: HashMap::new(),
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
    fn begin_fresh_txn(&mut self, timestamp_ns: u64) -> KvResult<TransactionId> {
        self.participant = aether_storage_provider::TransactionalParticipant::new(true);
        let xid = self.next_xid();
        self.participant
            .begin_transaction(xid, timestamp_ns)
            .map_err(KvError::from)?;
        Ok(xid)
    }

    /// Commits the current transaction, applies staged mutations, and resets
    /// the participant for the next operation.
    fn commit_current_txn(&mut self, timestamp_ns: u64) -> KvResult<()> {
        self.participant
            .commit_transaction(timestamp_ns)
            .map_err(KvError::from)?;
        apply_staged(&mut self.provider, &self.participant).map_err(KvError::from)?;
        self.participant.mark_committed();
        Ok(())
    }

    /// Stores `content` as a value object and returns its identity. Empty
    /// content is written directly because `apply_staged` treats empty content
    /// as a delete.
    fn put_value(&mut self, content: &[u8]) -> KvResult<ObjectId> {
        let id = ObjectId::from(ObjectIdentity::compute(content));
        // Content-addressed objects are immutable and idempotent, so they can
        // be stored directly without a transaction. Empty content is fine at
        // the provider level (only `apply_staged` treats empty content as a
        // delete).
        self.provider
            .put(id, content, Tier::Hot)
            .map_err(KvError::from)?;
        self.lifecycle.insert(id, LifecycleState::Active);
        Ok(id)
    }

    // ------------------------------------------------------------------
    // Operations (RFC-0022.1)
    // ------------------------------------------------------------------

    /// `kv_put(key, value)` (RFC-0022.1).
    ///
    /// Stores `value` under `key`, creating or updating the mapping.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::Storage`] if the value cannot be stored.
    pub fn kv_put(&mut self, key: &[u8], value: &[u8]) -> KvResult<ObjectId> {
        let value_id = self.put_value(value)?;
        let now = 0;

        // Stage the key→value mapping update in its own transaction so the
        // index update is atomic (RFC-0022.5).
        self.begin_fresh_txn(now)?;
        let key_bytes = key.to_vec();
        // Rebuild index in memory with the new binding first so serialization
        // reflects the put.
        self.index.insert(key_bytes.clone(), value_id);
        let content = self.serialize_index();
        self.participant
            .stage_put(Self::OBJECT_INDEX_ID, content, Tier::Hot)
            .map_err(KvError::from)?;
        self.commit_current_txn(now)?;
        Ok(value_id)
    }

    /// `kv_get(key)` (RFC-0022.1).
    ///
    /// Returns the value bytes for `key`.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::NotFound`] if `key` does not exist.
    pub fn kv_get(&self, key: &[u8]) -> KvResult<Vec<u8>> {
        let value_id = self.index.get(key).copied().ok_or(KvError::NotFound)?;
        let content = self.provider.get(value_id).map_err(KvError::from)?;
        Ok(content)
    }

    /// `kv_delete(key)` (RFC-0022.1).
    ///
    /// Deletes `key` and its associated value object.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::NotFound`] if `key` does not exist.
    pub fn kv_delete(&mut self, key: &[u8]) -> KvResult<()> {
        let value_id = self.index.get(key).copied().ok_or(KvError::NotFound)?;
        let now = 0;

        self.begin_fresh_txn(now)?;
        self.index.remove(key);
        let content = self.serialize_index();
        self.participant
            .stage_put(Self::OBJECT_INDEX_ID, content, Tier::Hot)
            .map_err(KvError::from)?;
        // Delete the value object from the provider.
        self.participant
            .stage_delete(value_id)
            .map_err(KvError::from)?;
        self.commit_current_txn(now)?;
        self.lifecycle.remove(&value_id);
        Ok(())
    }

    /// `kv_exists(key)` (RFC-0022.1).
    ///
    /// Returns `true` if `key` is present.
    #[must_use]
    pub fn kv_exists(&self, key: &[u8]) -> bool {
        self.index.contains_key(key)
    }

    /// `kv_list(prefix, limit)` (RFC-0022.1).
    ///
    /// Returns keys matching `prefix` in lexicographic byte order, at most
    /// `limit` entries (RFC-0022.3).
    #[must_use]
    pub fn kv_list(&self, prefix: &[u8], limit: u32) -> Vec<Vec<u8>> {
        let mut keys: Vec<Vec<u8>> = self
            .index
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        keys.sort();
        keys.truncate(limit as usize);
        keys
    }

    // ------------------------------------------------------------------
    // Transactions (RFC-0022.5 / RFC-0022.6)
    // ------------------------------------------------------------------

    /// `kv_txn_open()` (RFC-0022.5).
    ///
    /// Opens a new key–value transaction handle. Returns the handle.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::Storage`] if a fresh transaction cannot be begun.
    pub fn kv_txn_open(&mut self) -> KvResult<KvTransaction> {
        let xid = self.begin_fresh_txn(0)?;
        self.pending.insert(xid, Vec::new());
        Ok(KvTransaction(xid))
    }

    /// `kv_txn_put(txn, key, value)` (RFC-0022.5).
    ///
    /// Stages a put within `txn`. The change becomes visible only on
    /// `kv_txn_commit(txn)`.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::UnknownTransaction`] if `txn` is not open.
    pub fn kv_txn_put(
        &mut self,
        txn: KvTransaction,
        key: &[u8],
        value: &[u8],
    ) -> KvResult<ObjectId> {
        let value_id = self.put_value(value)?;
        let writes = self
            .pending
            .get_mut(&txn.0)
            .ok_or(KvError::UnknownTransaction)?;
        writes.push(KvPendingWrite {
            key: key.to_vec(),
            value_id: Some(value_id),
        });
        Ok(value_id)
    }

    /// `kv_txn_delete(txn, key)` (RFC-0022.5).
    ///
    /// Stages a delete within `txn`. The change becomes visible only on
    /// `kv_txn_commit(txn)`.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::UnknownTransaction`] if `txn` is not open.
    pub fn kv_txn_delete(&mut self, txn: KvTransaction, key: &[u8]) -> KvResult<()> {
        let writes = self
            .pending
            .get_mut(&txn.0)
            .ok_or(KvError::UnknownTransaction)?;
        writes.push(KvPendingWrite {
            key: key.to_vec(),
            value_id: None,
        });
        Ok(())
    }

    /// `kv_txn_commit(txn)` (RFC-0022.6).
    ///
    /// Commits `txn` atomically via two-phase commit: all staged puts and
    /// deletes apply, or none do.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::UnknownTransaction`] if `txn` is not open, or
    /// [`KvError::Storage`] if two-phase commit fails.
    pub fn kv_txn_commit(&mut self, txn: KvTransaction) -> KvResult<()> {
        let writes = self
            .pending
            .remove(&txn.0)
            .ok_or(KvError::UnknownTransaction)?;
        if writes.is_empty() {
            return Ok(());
        }

        let now = 0;
        self.begin_fresh_txn(now)?;

        // Apply all pending writes to the in-memory index first, then stage
        // the serialized index plus any value-object deletes.
        for write in &writes {
            match write.value_id {
                Some(value_id) => {
                    self.index.insert(write.key.clone(), value_id);
                    self.lifecycle.insert(value_id, LifecycleState::Active);
                }
                None => {
                    if let Some(value_id) = self.index.remove(&write.key) {
                        self.lifecycle.remove(&value_id);
                        self.participant
                            .stage_delete(value_id)
                            .map_err(KvError::from)?;
                    }
                }
            }
        }
        let content = self.serialize_index();
        self.participant
            .stage_put(Self::OBJECT_INDEX_ID, content, Tier::Hot)
            .map_err(KvError::from)?;
        self.commit_current_txn(now)?;
        Ok(())
    }

    /// `kv_txn_abort(txn)` (RFC-0022.6).
    ///
    /// Aborts `txn`, discarding all staged writes.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::UnknownTransaction`] if `txn` is not open.
    pub fn kv_txn_abort(&mut self, txn: KvTransaction) -> KvResult<()> {
        self.pending
            .remove(&txn.0)
            .ok_or(KvError::UnknownTransaction)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Atomic compare-and-swap (RFC-0022.7 / RFC-0022.8)
    // ------------------------------------------------------------------

    /// `kv_cas(key, old_value, new_value)` (RFC-0022.7).
    ///
    /// Atomically replaces the value of `key` with `new_value` only if the
    /// current value identity equals `old_value`. Values are content-resolved
    /// (RFC-0015), so comparing identities is equivalent to comparing content.
    ///
    /// Returns `Ok(true)` if the swap succeeded, or `Ok(false)` if the current
    /// value does not match `old_value` (a conflict — no state was modified,
    /// RFC-0022.8).
    ///
    /// # Errors
    ///
    /// Returns [`KvError::NotFound`] if `key` does not exist, or
    /// [`KvError::Storage`] if the commit fails.
    pub fn kv_cas(&mut self, key: &[u8], old_value: ObjectId, new_value: &[u8]) -> KvResult<bool> {
        let current_id = *self.index.get(key).ok_or(KvError::NotFound)?;
        if current_id != old_value {
            // Conflict: current value does not match expected. No state change.
            return Ok(false);
        }

        let new_id = self.put_value(new_value)?;
        let now = 0;

        self.begin_fresh_txn(now)?;
        self.index.insert(key.to_vec(), new_id);
        self.lifecycle.insert(new_id, LifecycleState::Active);
        let content = self.serialize_index();
        self.participant
            .stage_put(Self::OBJECT_INDEX_ID, content, Tier::Hot)
            .map_err(KvError::from)?;
        self.commit_current_txn(now)?;
        Ok(true)
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// The well-known object id under which the serialized key index is kept
    /// in the provider (RFC-0022.2).
    const OBJECT_INDEX_ID: ObjectId = ObjectId(1);

    /// Serializes the in-memory key index: `[u64:key_len][key bytes]...`
    /// repeated, followed by a terminating count of zero. Keys are stored
    /// as-is (RFC-0022.2).
    fn serialize_index(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut keys: Vec<&Vec<u8>> = self.index.keys().collect();
        keys.sort();
        for key in keys {
            let key_len = u64::try_from(key.len()).expect("key length must fit in u64");
            buf.extend_from_slice(&key_len.to_le_bytes());
            buf.extend_from_slice(key);
        }
        // Terminator: zero-length entry marks end of index.
        buf.extend_from_slice(&0u64.to_le_bytes());
        buf
    }
}

// ---------------------------------------------------------------------------
// Unit tests (CTS-STATE-KV-1..5 coverage)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider() -> aether_storage_provider::InMemoryProvider {
        aether_storage_provider::InMemoryProvider::new(
            "kv-proj-test",
            aether_storage_provider::ProviderType::Block,
            1024 * 1024,
            aether_storage_provider::LatencyBounds::new(100, 1000, 10_000).unwrap(),
            0.999,
            0x01,
        )
    }

    /// CTS-STATE-KV-1 (RFC-0022.1): full KV API — put, get, delete, exists,
    /// list work end to end.
    #[test]
    fn cts_kv_1_full_api() {
        let mut proj = KvProjection::new(make_provider());

        // Initially empty.
        assert!(!proj.kv_exists(b"foo"));
        assert_eq!(proj.kv_list(b"", 10).len(), 0);

        // Put.
        let id1 = proj.kv_put(b"foo", b"value-one").unwrap();
        assert!(proj.kv_exists(b"foo"));
        assert_eq!(proj.kv_get(b"foo").unwrap(), b"value-one");

        // Put another key.
        let id2 = proj.kv_put(b"bar", b"value-two").unwrap();
        assert_ne!(id1, id2);

        // List in lexicographic order.
        let listed = proj.kv_list(b"", 10);
        assert_eq!(listed, vec![b"bar".to_vec(), b"foo".to_vec()]);

        // Prefix list.
        let prefixed = proj.kv_list(b"f", 10);
        assert_eq!(prefixed, vec![b"foo".to_vec()]);

        // Update an existing key.
        proj.kv_put(b"foo", b"value-one-updated").unwrap();
        assert_eq!(proj.kv_get(b"foo").unwrap(), b"value-one-updated");

        // Delete.
        proj.kv_delete(b"bar").unwrap();
        assert!(!proj.kv_exists(b"bar"));
        assert_eq!(proj.kv_list(b"", 10), vec![b"foo".to_vec()]);
        assert_eq!(proj.kv_delete(b"bar"), Err(KvError::NotFound));
        assert_eq!(proj.kv_get(b"bar"), Err(KvError::NotFound));
    }

    /// CTS-STATE-KV-2 (RFC-0022.5): transactional grouping — multiple puts
    /// and deletes in one txn commit atomically.
    #[test]
    fn cts_kv_2_transactional_grouping() {
        let mut proj = KvProjection::new(make_provider());
        proj.kv_put(b"a", b"one").unwrap();
        proj.kv_put(b"b", b"two").unwrap();

        let txn = proj.kv_txn_open().unwrap();
        proj.kv_txn_put(txn, b"a", b"one-changed").unwrap();
        proj.kv_txn_put(txn, b"c", b"three").unwrap();
        proj.kv_txn_delete(txn, b"b").unwrap();

        // Uncommitted: reads still see pre-txn state.
        assert_eq!(proj.kv_get(b"a").unwrap(), b"one");
        assert!(!proj.kv_exists(b"c"));
        assert_eq!(proj.kv_get(b"b").unwrap(), b"two");

        proj.kv_txn_commit(txn).unwrap();

        // Committed: all changes visible atomically.
        assert_eq!(proj.kv_get(b"a").unwrap(), b"one-changed");
        assert_eq!(proj.kv_get(b"c").unwrap(), b"three");
        assert!(!proj.kv_exists(b"b"));
        assert_eq!(proj.kv_list(b"", 10), vec![b"a".to_vec(), b"c".to_vec()]);
    }

    /// CTS-STATE-KV-3 (RFC-0022.6): two-phase commit — abort discards all
    /// staged writes; none are applied.
    #[test]
    fn cts_kv_3_two_phase_commit_abort() {
        let mut proj = KvProjection::new(make_provider());
        proj.kv_put(b"a", b"one").unwrap();

        let txn = proj.kv_txn_open().unwrap();
        proj.kv_txn_put(txn, b"a", b"should-not-apply").unwrap();
        proj.kv_txn_put(txn, b"b", b"also-not").unwrap();
        proj.kv_txn_abort(txn).unwrap();

        // State unchanged after abort.
        assert_eq!(proj.kv_get(b"a").unwrap(), b"one");
        assert!(!proj.kv_exists(b"b"));
        assert_eq!(proj.kv_list(b"", 10), vec![b"a".to_vec()]);

        // The aborted handle can no longer be used.
        assert_eq!(proj.kv_txn_commit(txn), Err(KvError::UnknownTransaction));
        assert_eq!(
            proj.kv_txn_delete(txn, b"a"),
            Err(KvError::UnknownTransaction)
        );
    }

    /// CTS-STATE-KV-4 (RFC-0022.7): atomic CAS — replaces value only if the
    /// current value matches the expected identity.
    #[test]
    fn cts_kv_4_atomic_cas() {
        let mut proj = KvProjection::new(make_provider());
        let id1 = proj.kv_put(b"k", b"value-one").unwrap();

        // CAS with matching expected value succeeds.
        let new_id = proj.kv_put(b"k", b"value-two").unwrap();
        assert_ne!(new_id, id1);
        assert!(proj.kv_cas(b"k", new_id, b"value-three").unwrap());
        assert_eq!(proj.kv_get(b"k").unwrap(), b"value-three");

        // The key now points at the CAS'd value.
        assert_eq!(
            proj.index.get(b"k".as_slice()).copied(),
            Some(ObjectId::from(
                aether_object_identity::ObjectIdentity::compute(b"value-three")
            ))
        );
    }

    /// CTS-STATE-KV-5 (RFC-0022.8): CAS conflict — returns false without
    /// modifying state when the expected value does not match.
    #[test]
    fn cts_kv_5_cas_conflict() {
        let mut proj = KvProjection::new(make_provider());
        proj.kv_put(b"k", b"original").unwrap();
        let current = proj.index.get(b"k".as_slice()).copied().unwrap();

        // CAS with a non-matching expected value fails without changing state.
        let wrong = ObjectId(999);
        assert_ne!(wrong, current);
        assert!(!proj.kv_cas(b"k", wrong, b"replacement").unwrap());
        assert_eq!(proj.kv_get(b"k").unwrap(), b"original");
        assert_eq!(proj.index.get(b"k".as_slice()).copied(), Some(current));

        // CAS on a missing key returns NotFound.
        assert_eq!(
            proj.kv_cas(b"missing", current, b"x"),
            Err(KvError::NotFound)
        );
    }

    /// CTS-STATE-KV-2: empty transaction commits cleanly.
    #[test]
    fn cts_kv_2_empty_txn_commits() {
        let mut proj = KvProjection::new(make_provider());
        let txn = proj.kv_txn_open().unwrap();
        proj.kv_txn_commit(txn).unwrap();
        assert!(proj.kv_list(b"", 10).is_empty());
    }

    /// CTS-STATE-KV-1: list limit is respected.
    #[test]
    fn cts_kv_1_list_limit() {
        let mut proj = KvProjection::new(make_provider());
        for i in 0..5 {
            proj.kv_put(&[i], &[i]).unwrap();
        }
        let listed = proj.kv_list(b"", 2);
        assert_eq!(listed.len(), 2);
    }
}
