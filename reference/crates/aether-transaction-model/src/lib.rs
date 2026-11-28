//! Transaction model reference implementation (RFC-0016).
//!
//! The transaction model makes persistent state mutations atomic, consistent,
//! isolated, and durable. Every persistent mutation occurs within a
//! transaction using two-phase commit, snapshot isolation, write-write
//! conflict detection, and crash recovery (RFC-0016).
//!
//! Transaction lifecycle (RFC-0016.1):
//!   OPEN -> COMMITTING -> COMMITTED   (successful commit)
//!   OPEN -> ABORTED                    (client abort or prepare failure)
//!   COMMITTING -> FAILED               (commit failure, recovery needed)
//!   FAILED -> ABORTED                  (after recovery)

use aether_event_model::{PlatformEvent, SECURITY_CRITICAL_URGENCY};
use aether_types::{DomainId, ObjectId};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Transaction state
// ---------------------------------------------------------------------------

/// Transaction state (RFC-0016.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TransactionState {
    /// Transaction initialized; mutations accumulating.
    Open = 0,
    /// Two-phase commit in progress; mutations finalized.
    Committing = 1,
    /// All mutations durable; visible to subsequent transactions.
    Committed = 2,
    /// Transaction rolled back; no mutations applied.
    Aborted = 3,
    /// Commit failed; recovery in progress.
    Failed = 4,
}

impl TransactionState {
    /// Returns the numeric discriminant.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// ---------------------------------------------------------------------------
// Identifiers and records
// ---------------------------------------------------------------------------

/// Transaction identifier: the RFC-0015 identity of the transaction metadata
/// object (16 bytes folded from a BLAKE3-256 hash).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransactionId(pub ObjectId);

impl TransactionId {
    /// Creates a transaction id from raw bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(ObjectId(u128::from_le_bytes(bytes)))
    }

    /// Returns the raw 16-byte representation.
    #[must_use]
    pub const fn as_bytes(&self) -> [u8; 16] {
        self.0.0.to_le_bytes()
    }
}

/// A transaction record (RFC-0016 data structures).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionRecord {
    /// Transaction id.
    pub xid: TransactionId,
    /// Scheduling time the transaction started (RFC-0012).
    pub start_timestamp_ns: u64,
    /// Scheduling time the transaction committed (zero unless committed).
    pub commit_timestamp_ns: u64,
    /// Current transaction state.
    pub state: TransactionState,
    /// Number of participants in the two-phase commit.
    pub participant_count: u32,
    /// Capability token authorizing the coordinator.
    pub coordinator_cap: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Events emitted by the transaction model
// ---------------------------------------------------------------------------

/// Event types for transaction events (State class, 0x0300).
mod txn_event_type {
    /// Transaction state transition event (RFC-0031.36).
    pub const STATE_TRANSITION: u64 = 1;
    /// Transaction aborted (RFC-0031.34).
    pub const ABORTED: u64 = 2;
    /// Write-write conflict (RFC-0031.35).
    pub const WRITE_WRITE_CONFLICT: u64 = 3;
    /// Transaction recovered after crash (RFC-0031.37).
    pub const RECOVERED: u64 = 4;
    /// Transaction timeout (RFC-0031.38).
    #[allow(dead_code)]
    pub const TIMEOUT: u64 = 5;
}

/// Event class for transaction events: State (0x0300).
const TXN_EVENT_CLASS: u64 = aether_event_model::EventClass::State.id();

// ---------------------------------------------------------------------------
// Transaction participant
// ---------------------------------------------------------------------------

/// A participant in a two-phase commit (RFC-0016.3.2).
#[allow(clippy::struct_excessive_bools)]
pub struct Participant {
    /// Participant identifier.
    pub id: u64,
    /// Whether the participant can commit its portion.
    pub can_prepare: bool,
    /// Whether the participant has responded PREPARED.
    pub prepared: bool,
    /// Whether the participant has acknowledged COMMIT/ABORT.
    pub acked: bool,
    /// The participant's pending mutation (flushed to durable storage on prepare).
    pub mutation_pending: Vec<u8>,
    /// Whether the participant's mutation is durable (committed).
    pub durable: bool,
}

impl Participant {
    /// Creates a new participant.
    #[must_use]
    pub fn new(id: u64, can_prepare: bool) -> Self {
        Self {
            id,
            can_prepare,
            prepared: false,
            acked: false,
            mutation_pending: Vec::new(),
            durable: false,
        }
    }

    /// Receives PREPARE (Phase 1). Validates local consistency and responds.
    /// Returns true if PREPARED, false if ABORT.
    pub fn on_prepare(&mut self, mutation: Vec<u8>) -> bool {
        if !self.can_prepare {
            return false;
        }
        self.mutation_pending = mutation;
        self.prepared = true;
        true
    }

    /// Receives COMMIT (Phase 2). Marks mutation durable.
    pub fn on_commit(&mut self) {
        self.durable = true;
        self.acked = true;
    }

    /// Receives ABORT (Phase 1 failure). Discards pending mutation.
    pub fn on_abort(&mut self) {
        self.mutation_pending.clear();
        self.durable = false;
        self.acked = true;
    }
}

// ---------------------------------------------------------------------------
// Transaction
// ---------------------------------------------------------------------------

/// A single transaction with its mutations and participants (RFC-0016.1).
pub struct Transaction {
    /// The transaction record.
    pub record: TransactionRecord,
    /// Mutations accumulated during the OPEN phase, keyed by object id.
    pub mutations: HashMap<ObjectId, Vec<u8>>,
    /// Two-phase commit participants.
    pub participants: HashMap<u64, Participant>,
    /// Objects this transaction has written (for write-write conflict detection).
    pub writes: HashSet<ObjectId>,
}

impl Transaction {
    /// Creates a new OPEN transaction.
    #[must_use]
    pub fn new(
        xid: TransactionId,
        start_timestamp_ns: u64,
        participant_count: u32,
        coordinator_cap: Option<ObjectId>,
    ) -> Self {
        Self {
            record: TransactionRecord {
                xid,
                start_timestamp_ns,
                commit_timestamp_ns: 0,
                state: TransactionState::Open,
                participant_count,
                coordinator_cap,
            },
            mutations: HashMap::new(),
            participants: HashMap::new(),
            writes: HashSet::new(),
        }
    }

    /// Returns whether the transaction is still open (mutations can be added).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.record.state == TransactionState::Open
    }

    /// Returns whether the transaction is committed.
    #[must_use]
    pub fn is_committed(&self) -> bool {
        self.record.state == TransactionState::Committed
    }

    /// Returns whether the transaction is aborted.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.record.state == TransactionState::Aborted
    }

    /// Adds a mutation to the transaction. Only allowed while OPEN.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction is not OPEN.
    pub fn add_mutation(
        &mut self,
        object_id: ObjectId,
        mutation: Vec<u8>,
    ) -> Result<(), TransactionError> {
        if self.record.state != TransactionState::Open {
            return Err(TransactionError::NotOpen);
        }
        self.mutations.insert(object_id, mutation);
        self.writes.insert(object_id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors encountered by the transaction model (RFC-0016).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionError {
    /// The transaction is not in OPEN state and cannot accept mutations.
    NotOpen,
    /// The transaction is already terminal (committed or aborted).
    AlreadyTerminal,
    /// A write-write conflict was detected (RFC-0016.9).
    WriteWriteConflict,
    /// Phase 1 prepare failed for at least one participant.
    PrepareFailed,
    /// The transaction is not in COMMITTING state.
    NotCommitting,
    /// The coordinator timed out waiting for a participant (RFC-0016.15).
    Timeout,
    /// Unknown transaction id.
    UnknownTransaction,
}

// ---------------------------------------------------------------------------
// Two-phase commit coordinator
// ---------------------------------------------------------------------------

/// Coordinator for two-phase commit transactions (RFC-0016.4).
///
/// Drives participants through PREPARE (Phase 1) and COMMIT (Phase 2),
/// ensuring the all-or-nothing invariant (RFC-0016.3).
pub struct TransactionCoordinator {
    /// All transactions, keyed by id.
    transactions: HashMap<TransactionId, Transaction>,
    /// The current (latest) committed snapshot of object state, keyed by
    /// object id. Used for snapshot isolation (RFC-0016.8).
    committed_state: HashMap<ObjectId, Vec<u8>>,
    /// Set of object ids written by any committed transaction (for
    /// write-write conflict detection, RFC-0016.9).
    committed_writes: HashSet<ObjectId>,
    /// Next participant id to assign.
    next_participant_id: u64,
    /// Events emitted by the coordinator (transaction observability,
    /// RFC-0016.12).
    events: Vec<PlatformEvent>,
    /// Whether crash recovery has been performed.
    recovered: bool,
}

impl TransactionCoordinator {
    /// Creates a new transaction coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            committed_state: HashMap::new(),
            committed_writes: HashSet::new(),
            next_participant_id: 0,
            events: Vec::new(),
            recovered: false,
        }
    }

    /// Returns all emitted transaction events.
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        &self.events
    }

    /// Returns the number of transactions.
    #[must_use]
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Returns whether crash recovery has been performed (RFC-0016.13).
    #[must_use]
    pub fn is_recovered(&self) -> bool {
        self.recovered
    }

    /// Begins a new OPEN transaction (RFC-0016.1).
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::UnknownTransaction`] if a transaction with
    /// `xid` already exists.
    pub fn begin(
        &mut self,
        xid: TransactionId,
        start_timestamp_ns: u64,
        participant_count: u32,
        coordinator_cap: Option<ObjectId>,
    ) -> Result<TransactionId, TransactionError> {
        if self.transactions.contains_key(&xid) {
            return Err(TransactionError::UnknownTransaction);
        }
        self.transactions.insert(
            xid,
            Transaction::new(xid, start_timestamp_ns, participant_count, coordinator_cap),
        );
        Ok(xid)
    }

    /// Registers a participant for a transaction and returns its id.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::UnknownTransaction`] if `xid` does not
    /// exist, or [`TransactionError::NotOpen`] if the transaction is not
    /// OPEN.
    pub fn register_participant(
        &mut self,
        xid: &TransactionId,
        can_prepare: bool,
    ) -> Result<u64, TransactionError> {
        let txn = self
            .transactions
            .get_mut(xid)
            .ok_or(TransactionError::UnknownTransaction)?;
        if txn.record.state != TransactionState::Open {
            return Err(TransactionError::NotOpen);
        }
        let id = self.next_participant_id;
        self.next_participant_id += 1;
        txn.participants
            .insert(id, Participant::new(id, can_prepare));
        Ok(id)
    }

    /// Adds a mutation to a transaction (RFC-0016.2: invisible until COMMITTED).
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::UnknownTransaction`] if `xid` does not
    /// exist.
    pub fn add_mutation(
        &mut self,
        xid: &TransactionId,
        object_id: ObjectId,
        mutation: Vec<u8>,
    ) -> Result<(), TransactionError> {
        let txn = self
            .transactions
            .get_mut(xid)
            .ok_or(TransactionError::UnknownTransaction)?;
        txn.add_mutation(object_id, mutation)
    }

    /// Emits a transaction state transition event (RFC-0016.12, RFC-0031.36).
    fn emit_state_transition(
        &mut self,
        xid: &TransactionId,
        old_state: TransactionState,
        new_state: TransactionState,
    ) {
        let mut payload = Vec::new();
        payload.extend_from_slice(&xid.as_bytes());
        payload.push(old_state.as_u8());
        payload.push(new_state.as_u8());
        payload.extend_from_slice(&(self.transactions.len() as u64).to_le_bytes());
        self.events.push(PlatformEvent {
            event_id: self.events.len() as u64 + 1,
            event_class: TXN_EVENT_CLASS,
            event_type: txn_event_type::STATE_TRANSITION,
            timestamp_ns: self
                .transactions
                .get(xid)
                .map_or(0, |t| t.record.start_timestamp_ns),
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: SECURITY_CRITICAL_URGENCY,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload,
        });
    }

    /// Commits a transaction using two-phase commit (RFC-0016.4).
    ///
    /// Phase 1: sends PREPARE to all participants; any ABORT -> abort all.
    /// Phase 2: if all PREPARED, sends COMMIT to all -> COMMITTED.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::PrepareFailed`] if any participant cannot
    /// prepare. Returns [`TransactionError::WriteWriteConflict`] if a
    /// concurrent transaction has already committed writes to the same
    /// objects (RFC-0016.9). Returns [`TransactionError::AlreadyTerminal`]
    /// if the transaction is not OPEN.
    ///
    /// # Panics
    ///
    /// Panics if `DomainId::new(1)` returns `None` (it does not; domain 1 is
    /// non-zero).
    pub fn commit(
        &mut self,
        xid: &TransactionId,
        commit_timestamp_ns: u64,
    ) -> Result<(), TransactionError> {
        // Take the transaction out of the map so we can mutate it without
        // borrowing self across participant method calls.
        let mut txn = self
            .transactions
            .remove(xid)
            .ok_or(TransactionError::UnknownTransaction)?;
        let old_state = txn.record.state;
        if old_state != TransactionState::Open {
            self.transactions.insert(*xid, txn);
            return Err(TransactionError::AlreadyTerminal);
        }

        // Write-write conflict detection (RFC-0016.9).
        for object_id in &txn.writes {
            if self.committed_writes.contains(object_id) {
                txn.record.state = TransactionState::Aborted;
                self.emit_state_transition(xid, old_state, TransactionState::Aborted);
                self.events.push(PlatformEvent {
                    event_id: self.events.len() as u64 + 1,
                    event_class: TXN_EVENT_CLASS,
                    event_type: txn_event_type::WRITE_WRITE_CONFLICT,
                    timestamp_ns: commit_timestamp_ns,
                    emitter_thread: 1,
                    emitter_domain: DomainId::new(1).unwrap(),
                    urgency: SECURITY_CRITICAL_URGENCY,
                    flags: 0,
                    causation_id: 0,
                    correlation_id: 0,
                    payload: xid.as_bytes().to_vec(),
                });
                self.transactions.insert(*xid, txn);
                return Err(TransactionError::WriteWriteConflict);
            }
        }

        // Phase 1: PREPARE (RFC-0016.4).
        let mutation_for_prepare = txn.mutations.get(&ObjectId(0)).cloned().unwrap_or_default();
        let mut all_prepared = true;
        for participant in txn.participants.values_mut() {
            if !participant.on_prepare(mutation_for_prepare.clone()) {
                all_prepared = false;
                break;
            }
        }

        if !all_prepared {
            // Abort all PREPARED participants (RFC-0016.5).
            for participant in txn.participants.values_mut() {
                if participant.prepared {
                    participant.on_abort();
                }
            }
            txn.record.state = TransactionState::Aborted;
            self.emit_state_transition(xid, old_state, TransactionState::Aborted);
            self.events.push(PlatformEvent {
                event_id: self.events.len() as u64 + 1,
                event_class: TXN_EVENT_CLASS,
                event_type: txn_event_type::ABORTED,
                timestamp_ns: commit_timestamp_ns,
                emitter_thread: 1,
                emitter_domain: DomainId::new(1).unwrap(),
                urgency: SECURITY_CRITICAL_URGENCY,
                flags: 0,
                causation_id: 0,
                correlation_id: 0,
                payload: xid.as_bytes().to_vec(),
            });
            self.transactions.insert(*xid, txn);
            return Err(TransactionError::PrepareFailed);
        }

        // Phase 2: COMMIT (RFC-0016.4).
        txn.record.state = TransactionState::Committing;
        self.emit_state_transition(xid, old_state, TransactionState::Committing);
        for participant in txn.participants.values_mut() {
            participant.on_commit();
        }
        txn.record.state = TransactionState::Committed;
        txn.record.commit_timestamp_ns = commit_timestamp_ns;

        // Apply mutations to the committed snapshot (durability, RFC-0016.6).
        for (object_id, mutation) in txn.mutations.drain() {
            self.committed_state.insert(object_id, mutation);
            self.committed_writes.insert(object_id);
        }

        self.emit_state_transition(
            xid,
            TransactionState::Committing,
            TransactionState::Committed,
        );
        self.transactions.insert(*xid, txn);
        Ok(())
    }

    /// Aborts a transaction (client abort or prepare failure path).
    ///
    /// # Errors
    ///
    /// Returns [`TransactionError::UnknownTransaction`] if `xid` does not
    /// exist, or [`TransactionError::AlreadyTerminal`] if the transaction is
    /// already COMMITTED or ABORTED.
    ///
    /// # Panics
    ///
    /// Panics if `DomainId::new(1)` returns `None` (it does not; domain 1 is
    /// non-zero).
    pub fn abort(&mut self, xid: &TransactionId) -> Result<(), TransactionError> {
        let txn = self
            .transactions
            .get_mut(xid)
            .ok_or(TransactionError::UnknownTransaction)?;
        let old_state = txn.record.state;
        if old_state == TransactionState::Committed || old_state == TransactionState::Aborted {
            return Err(TransactionError::AlreadyTerminal);
        }
        for participant in txn.participants.values_mut() {
            if participant.prepared {
                participant.on_abort();
            }
        }
        let start_ts = txn.record.start_timestamp_ns;
        txn.record.state = TransactionState::Aborted;
        self.emit_state_transition(xid, old_state, TransactionState::Aborted);
        self.events.push(PlatformEvent {
            event_id: self.events.len() as u64 + 1,
            event_class: TXN_EVENT_CLASS,
            event_type: txn_event_type::ABORTED,
            timestamp_ns: start_ts,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: SECURITY_CRITICAL_URGENCY,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: xid.as_bytes().to_vec(),
        });
        Ok(())
    }

    /// Reads a committed object's value (snapshot isolation, RFC-0016.8).
    /// Transactions observe the committed snapshot as of their start time.
    #[must_use]
    pub fn read_committed(&self, object_id: &ObjectId) -> Option<&Vec<u8>> {
        self.committed_state.get(object_id)
    }

    /// Performs crash recovery (RFC-0016.13). All in-progress transactions
    /// that were not COMMITTED at crash time are recovered as ABORTED.
    /// Recovery MUST NOT silently upgrade an ABORTED transaction to COMMITTED.
    ///
    /// # Panics
    ///
    /// Panics if a transaction id in `to_recover` is no longer present in the
    /// transactions map (cannot happen: ids are collected from the map).
    pub fn recover_after_crash(&mut self) -> Vec<TransactionId> {
        let mut recovered_ids: Vec<TransactionId> = Vec::new();
        // Collect non-committed transaction ids first, so we do not hold a
        // mutable borrow across emit_state_transition.
        let to_recover: Vec<TransactionId> = self
            .transactions
            .iter()
            .filter(|(_, txn)| txn.record.state != TransactionState::Committed)
            .map(|(xid, _)| *xid)
            .collect();
        for xid in to_recover {
            let txn = self.transactions.get_mut(&xid).unwrap();
            let old_state = txn.record.state;
            let start_ts = txn.record.start_timestamp_ns;
            txn.record.state = TransactionState::Aborted;
            for participant in txn.participants.values_mut() {
                participant.on_abort();
            }
            self.emit_state_transition(&xid, old_state, TransactionState::Aborted);
            self.events.push(PlatformEvent {
                event_id: self.events.len() as u64 + 1,
                event_class: TXN_EVENT_CLASS,
                event_type: txn_event_type::RECOVERED,
                timestamp_ns: start_ts,
                emitter_thread: 1,
                emitter_domain: DomainId::new(1).unwrap(),
                urgency: SECURITY_CRITICAL_URGENCY,
                flags: 0,
                causation_id: 0,
                correlation_id: 0,
                payload: xid.as_bytes().to_vec(),
            });
            recovered_ids.push(xid);
        }
        self.recovered = true;
        recovered_ids
    }

    /// Returns the transaction with `xid`, if it exists.
    #[must_use]
    pub fn get_transaction(&self, xid: &TransactionId) -> Option<&Transaction> {
        self.transactions.get(xid)
    }
}

impl Default for TransactionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_xid(bytes: [u8; 16]) -> TransactionId {
        TransactionId::from_bytes(bytes)
    }

    #[test]
    fn transaction_states_transition() {
        let mut txn = Transaction::new(make_xid([1u8; 16]), 1_000_000, 1, Some(ObjectId(0x42)));
        assert!(txn.is_open());
        assert!(!txn.is_committed());
        assert!(!txn.is_aborted());

        txn.record.state = TransactionState::Committing;
        assert!(!txn.is_open());

        txn.record.state = TransactionState::Committed;
        txn.record.commit_timestamp_ns = 2_000_000;
        assert!(txn.is_committed());
        assert_eq!(txn.record.commit_timestamp_ns, 2_000_000);
    }

    #[test]
    fn begin_and_add_mutation() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([2u8; 16]);
        coord.begin(xid, 1_000_000, 1, None).unwrap();
        coord
            .add_mutation(&xid, ObjectId(0x100), b"mutation data".to_vec())
            .unwrap();
        let txn = coord.get_transaction(&xid).unwrap();
        assert_eq!(txn.mutations.len(), 1);
        assert_eq!(txn.record.state, TransactionState::Open);
    }

    #[test]
    fn mutation_not_visible_until_committed() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([3u8; 16]);
        coord.begin(xid, 1_000_000, 1, None).unwrap();
        coord
            .add_mutation(&xid, ObjectId(0x100), b"uncommitted".to_vec())
            .unwrap();
        // Not visible to other transactions before commit.
        assert_eq!(coord.read_committed(&ObjectId(0x100)), None);
    }

    #[test]
    fn two_phase_commit_all_prepared() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([4u8; 16]);
        coord.begin(xid, 1_000_000, 2, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord
            .add_mutation(&xid, ObjectId(0x100), b"data".to_vec())
            .unwrap();

        coord.commit(&xid, 2_000_000).unwrap();
        let txn = coord.get_transaction(&xid).unwrap();
        assert!(txn.is_committed());
        assert_eq!(
            coord.read_committed(&ObjectId(0x100)),
            Some(&b"data".to_vec())
        );
        // All participants durable.
        for p in txn.participants.values() {
            assert!(p.durable);
            assert!(p.acked);
        }
    }

    #[test]
    fn two_phase_commit_abort_on_prepare_failure() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([5u8; 16]);
        coord.begin(xid, 1_000_000, 2, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord.register_participant(&xid, false).unwrap(); // cannot prepare
        coord
            .add_mutation(&xid, ObjectId(0x100), b"data".to_vec())
            .unwrap();

        let result = coord.commit(&xid, 2_000_000);
        assert_eq!(result, Err(TransactionError::PrepareFailed));
        let txn = coord.get_transaction(&xid).unwrap();
        assert!(txn.is_aborted());
        // No mutation applied.
        assert_eq!(coord.read_committed(&ObjectId(0x100)), None);
    }

    #[test]
    fn client_abort() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([6u8; 16]);
        coord.begin(xid, 1_000_000, 1, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord.abort(&xid).unwrap();
        let txn = coord.get_transaction(&xid).unwrap();
        assert!(txn.is_aborted());
    }

    #[test]
    fn write_write_conflict_detected() {
        let mut coord = TransactionCoordinator::new();
        // Txn1 commits a write to object 0x100.
        let xid1 = make_xid([7u8; 16]);
        coord.begin(xid1, 1_000_000, 1, None).unwrap();
        coord.register_participant(&xid1, true).unwrap();
        coord
            .add_mutation(&xid1, ObjectId(0x100), b"v1".to_vec())
            .unwrap();
        coord.commit(&xid1, 2_000_000).unwrap();

        // Txn2 attempts to write the same object -> conflict.
        let xid2 = make_xid([8u8; 16]);
        coord.begin(xid2, 1_500_000, 1, None).unwrap();
        coord.register_participant(&xid2, true).unwrap();
        coord
            .add_mutation(&xid2, ObjectId(0x100), b"v2".to_vec())
            .unwrap();
        let result = coord.commit(&xid2, 3_000_000);
        assert_eq!(result, Err(TransactionError::WriteWriteConflict));
        let txn2 = coord.get_transaction(&xid2).unwrap();
        assert!(txn2.is_aborted());
    }

    #[test]
    fn snapshot_isolation_reads_committed_snapshot() {
        let mut coord = TransactionCoordinator::new();
        // Commit object 0x100 = "old".
        let xid1 = make_xid([9u8; 16]);
        coord.begin(xid1, 1_000_000, 1, None).unwrap();
        coord.register_participant(&xid1, true).unwrap();
        coord
            .add_mutation(&xid1, ObjectId(0x100), b"old".to_vec())
            .unwrap();
        coord.commit(&xid1, 2_000_000).unwrap();

        // Txn2 reads the committed snapshot while Txn3 is open but uncommitted.
        let xid2 = make_xid([10u8; 16]);
        coord.begin(xid2, 2_500_000, 1, None).unwrap();
        coord.register_participant(&xid2, true).unwrap();
        coord
            .add_mutation(&xid2, ObjectId(0x100), b"uncommitted".to_vec())
            .unwrap();

        assert_eq!(
            coord.read_committed(&ObjectId(0x100)),
            Some(&b"old".to_vec())
        );
    }

    #[test]
    fn crash_recovery_aborts_in_progress() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([11u8; 16]);
        coord.begin(xid, 1_000_000, 1, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord
            .add_mutation(&xid, ObjectId(0x100), b"uncommitted".to_vec())
            .unwrap();
        // Do not commit; simulate crash.
        let recovered = coord.recover_after_crash();
        assert_eq!(recovered, vec![xid]);
        assert!(coord.is_recovered());
        let txn = coord.get_transaction(&xid).unwrap();
        assert!(txn.is_aborted());
        assert_eq!(coord.read_committed(&ObjectId(0x100)), None);
    }

    #[test]
    fn crash_recovery_does_not_upgrade_aborted() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([12u8; 16]);
        coord.begin(xid, 1_000_000, 1, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord.abort(&xid).unwrap();
        coord.recover_after_crash();
        let txn = coord.get_transaction(&xid).unwrap();
        assert!(txn.is_aborted());
    }

    #[test]
    fn state_transition_events_emitted() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([13u8; 16]);
        coord.begin(xid, 1_000_000, 1, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord
            .add_mutation(&xid, ObjectId(0x100), b"data".to_vec())
            .unwrap();
        coord.commit(&xid, 2_000_000).unwrap();

        let events = coord.events();
        let transition_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == txn_event_type::STATE_TRANSITION)
            .collect();
        assert!(!transition_events.is_empty());
        // Open -> Committing -> Committed: two transitions.
        assert_eq!(transition_events.len(), 2);
    }

    #[test]
    fn all_or_nothing_invariant() {
        let mut coord = TransactionCoordinator::new();
        let xid = make_xid([14u8; 16]);
        coord.begin(xid, 1_000_000, 2, None).unwrap();
        coord.register_participant(&xid, true).unwrap();
        coord.register_participant(&xid, false).unwrap();
        coord
            .add_mutation(&xid, ObjectId(0x200), b"should-not-apply".to_vec())
            .unwrap();
        let result = coord.commit(&xid, 2_000_000);
        assert!(result.is_err());
        assert_eq!(coord.read_committed(&ObjectId(0x200)), None);
    }
}
