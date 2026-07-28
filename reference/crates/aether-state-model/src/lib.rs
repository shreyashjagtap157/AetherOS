//! Crash-recoverable transactional state and durable-effect reference model.
//!
//! The format in this crate is a prototype test format, not an `AetherOS` disk
//! format. A complete checksummed frame is the model's atomic persistence unit.

use aether_types::ObjectId;
use std::collections::{HashMap, HashSet};

const MAGIC: u32 = 0x4154_584e; // "ATXN"
const HEADER_LEN: usize = 16;
const MUTATION_ENCODED_LEN: usize = 16 + 8 + 32;
const EFFECT_ENCODED_LEN: usize = 16 + 1 + 32;

/// Immutable version identifier in the reference state model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionId(pub u64);

/// Stable transaction identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransactionId(pub u64);

/// Stable identity for an external effect across retry and recovery.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationId(pub u128);

/// Classification controlling whether and how an external effect may retry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EffectClass {
    /// Repeating the operation has the same semantic outcome.
    Idempotent = 1,
    /// Receiver must deduplicate using the operation identifier.
    Deduplicated = 2,
    /// Failure may require an explicit compensating operation.
    Compensatable = 3,
    /// Operation cannot be rolled back after dispatch.
    Irreversible = 4,
}

/// Durable lifecycle of an external-effect intent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EffectStatus {
    /// Committed locally but not yet conclusively dispatched.
    Pending = 1,
    /// Receiver supplied positive completion evidence.
    Acknowledged = 2,
    /// Receiver conclusively rejected the operation.
    Rejected = 3,
    /// Physical or remote outcome cannot currently be determined.
    Uncertain = 4,
    /// A compensating operation is required.
    CompensationRequired = 5,
}

/// Object mutation staged in a transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mutation {
    /// Object whose visible version changes.
    pub object: ObjectId,
    /// New immutable version.
    pub version: VersionId,
    /// Caller-provided digest of the logical payload.
    pub content_digest: [u8; 32],
}

/// External-effect intent atomically committed with state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectIntent {
    /// Stable idempotency and reconciliation key.
    pub operation: OperationId,
    /// Retry/compensation classification.
    pub class: EffectClass,
    /// Digest of the canonical effect request, never the secret payload.
    pub request_digest: [u8; 32],
}

/// Transaction before its single atomic commit frame is appended.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedTransaction {
    id: TransactionId,
    actor: u128,
    authority_reference: u64,
    mutations: Vec<Mutation>,
    effects: Vec<EffectIntent>,
}

impl PreparedTransaction {
    /// Adds an object mutation to the not-yet-visible transaction.
    pub fn mutate(&mut self, mutation: Mutation) {
        self.mutations.push(mutation);
    }

    /// Adds an external-effect intent to the atomic commit group.
    pub fn intend(&mut self, intent: EffectIntent) {
        self.effects.push(intent);
    }

    /// Returns the transaction identifier.
    #[must_use]
    pub const fn id(&self) -> TransactionId {
        self.id
    }
}

/// Evidence returned after the complete commit frame is appended and applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    /// Committed transaction.
    pub transaction: TransactionId,
    /// Deterministic commitment covering actor, authority, mutations and intents.
    pub audit_commitment: [u8; 32],
}

/// Recovery or validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateError {
    /// A complete frame has an invalid checksum or impossible encoding.
    CorruptFrame,
    /// Record contains an unknown enum discriminant.
    UnsupportedRecord,
    /// A transaction contains no state mutation and no effect intent.
    EmptyTransaction,
    /// Effect outcome references an unknown operation.
    UnknownOperation,
    /// Effect transition is not legal from the current status.
    InvalidEffectTransition,
    /// Frame length cannot be represented.
    FrameTooLarge,
    /// Stored audit commitment does not cover the canonical transaction body.
    AuditMismatch,
    /// A transaction identifier was already committed.
    DuplicateTransaction,
    /// A transaction contains more than one mutation for the same object.
    DuplicateMutation,
    /// An operation identifier already exists or occurs twice in one transaction.
    DuplicateOperation,
    /// No unused transaction identifier remains in the reference identifier space.
    TransactionIdExhausted,
}

/// Deterministic state engine backed by an append-only byte log.
pub struct StateEngine {
    log: Vec<u8>,
    next_transaction: u64,
    visible: HashMap<ObjectId, VersionId>,
    effects: HashMap<OperationId, (EffectClass, EffectStatus)>,
    applied_transactions: HashSet<TransactionId>,
}

impl StateEngine {
    /// Creates an empty state engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            next_transaction: 1,
            visible: HashMap::new(),
            effects: HashMap::new(),
            applied_transactions: HashSet::new(),
        }
    }

    /// Reconstructs state from complete valid frames and ignores only a torn tail.
    ///
    /// # Errors
    /// Returns an error for complete corrupt frames, unsupported records,
    /// invalid transitions, or unknown effect operations.
    pub fn recover(log: &[u8]) -> Result<Self, StateError> {
        let mut engine = Self::new();
        let mut cursor = 0;
        while log.len().saturating_sub(cursor) >= HEADER_LEN {
            let magic = read_u32(log, cursor)?;
            if magic != MAGIC {
                return Err(StateError::CorruptFrame);
            }
            let payload_len = usize::try_from(read_u32(log, cursor + 4)?)
                .map_err(|_| StateError::CorruptFrame)?;
            let frame_len = HEADER_LEN
                .checked_add(payload_len)
                .ok_or(StateError::CorruptFrame)?;
            if log.len().saturating_sub(cursor) < frame_len {
                break;
            }
            let expected = read_u64(log, cursor + 8)?;
            let payload = &log[cursor + HEADER_LEN..cursor + frame_len];
            if checksum(payload) != expected {
                return Err(StateError::CorruptFrame);
            }
            engine.apply_payload(payload)?;
            engine
                .log
                .extend_from_slice(&log[cursor..cursor + frame_len]);
            cursor += frame_len;
        }
        Ok(engine)
    }

    /// Opens a transaction. Nothing becomes visible until `commit` succeeds.
    ///
    /// # Errors
    /// Returns [`StateError::TransactionIdExhausted`] before an identifier could
    /// wrap or be reused.
    pub fn prepare(
        &mut self,
        actor: u128,
        authority_reference: u64,
    ) -> Result<PreparedTransaction, StateError> {
        let id = TransactionId(self.next_transaction);
        self.next_transaction = self
            .next_transaction
            .checked_add(1)
            .ok_or(StateError::TransactionIdExhausted)?;
        Ok(PreparedTransaction {
            id,
            actor,
            authority_reference,
            mutations: Vec::new(),
            effects: Vec::new(),
        })
    }

    /// Atomically appends and applies a transaction frame in the reference model.
    ///
    /// # Errors
    /// Returns an error for empty transactions, duplicate object mutations,
    /// duplicate operation identifiers, or unrepresentable reference frames.
    pub fn commit(
        &mut self,
        transaction: PreparedTransaction,
    ) -> Result<CommitReceipt, StateError> {
        if transaction.mutations.is_empty() && transaction.effects.is_empty() {
            return Err(StateError::EmptyTransaction);
        }
        self.validate_prepared(&transaction)?;
        let transaction_id = transaction.id;
        let (payload, audit_commitment) = encode_commit(transaction)?;
        self.append_frame(&payload)?;
        self.apply_payload(&payload)?;
        Ok(CommitReceipt {
            transaction: transaction_id,
            audit_commitment,
        })
    }

    /// Records a conclusive or uncertain effect outcome as a separate durable frame.
    ///
    /// # Errors
    /// Returns an error for unknown operations, illegal transitions, or an
    /// unrepresentable frame.
    pub fn record_effect(
        &mut self,
        operation: OperationId,
        status: EffectStatus,
    ) -> Result<(), StateError> {
        let (_, current) = self
            .effects
            .get(&operation)
            .ok_or(StateError::UnknownOperation)?;
        if !valid_transition(*current, status) {
            return Err(StateError::InvalidEffectTransition);
        }
        let payload = encode_effect(operation, status);
        self.append_frame(&payload)?;
        self.apply_payload(&payload)
    }

    /// Returns the visible version of an object.
    #[must_use]
    pub fn visible_version(&self, object: ObjectId) -> Option<VersionId> {
        self.visible.get(&object).copied()
    }

    /// Returns the durable effect status.
    #[must_use]
    pub fn effect_status(&self, operation: OperationId) -> Option<EffectStatus> {
        self.effects.get(&operation).map(|(_, status)| *status)
    }

    /// Returns the complete durable log used for deterministic crash simulation.
    #[must_use]
    pub fn durable_log(&self) -> &[u8] {
        &self.log
    }

    fn append_frame(&mut self, payload: &[u8]) -> Result<(), StateError> {
        let len = u32::try_from(payload.len()).map_err(|_| StateError::FrameTooLarge)?;
        self.log.extend_from_slice(&MAGIC.to_le_bytes());
        self.log.extend_from_slice(&len.to_le_bytes());
        self.log.extend_from_slice(&checksum(payload).to_le_bytes());
        self.log.extend_from_slice(payload);
        Ok(())
    }

    fn apply_payload(&mut self, payload: &[u8]) -> Result<(), StateError> {
        let tag = *payload.first().ok_or(StateError::CorruptFrame)?;
        match tag {
            1 => self.apply_commit(payload),
            2 => self.apply_effect(payload),
            _ => Err(StateError::UnsupportedRecord),
        }
    }

    fn apply_commit(&mut self, payload: &[u8]) -> Result<(), StateError> {
        let mut cursor = 1;
        let stored_commitment: [u8; 32] = take(payload, &mut cursor, 32)?
            .try_into()
            .map_err(|_| StateError::CorruptFrame)?;
        let body = payload.get(cursor..).ok_or(StateError::CorruptFrame)?;
        if stored_commitment != commitment(body) {
            return Err(StateError::AuditMismatch);
        }
        let transaction = TransactionId(read_u64_advance(payload, &mut cursor)?);
        let _actor = read_u128_advance(payload, &mut cursor)?;
        let _authority = read_u64_advance(payload, &mut cursor)?;
        let mutation_count = read_u32_advance(payload, &mut cursor)?;
        let mutation_count =
            usize::try_from(mutation_count).map_err(|_| StateError::CorruptFrame)?;
        let mutation_bytes = mutation_count
            .checked_mul(MUTATION_ENCODED_LEN)
            .ok_or(StateError::CorruptFrame)?;
        let mutations_and_effect_count = mutation_bytes
            .checked_add(4)
            .ok_or(StateError::CorruptFrame)?;
        if payload.len().saturating_sub(cursor) < mutations_and_effect_count {
            return Err(StateError::CorruptFrame);
        }
        let mut mutations = Vec::with_capacity(mutation_count);
        let mut mutation_ids = HashSet::with_capacity(mutation_count);
        for _ in 0..mutation_count {
            let object = ObjectId(read_u128_advance(payload, &mut cursor)?);
            let version = VersionId(read_u64_advance(payload, &mut cursor)?);
            take(payload, &mut cursor, 32)?;
            if !mutation_ids.insert(object) {
                return Err(StateError::DuplicateMutation);
            }
            mutations.push((object, version));
        }
        let effect_count = read_u32_advance(payload, &mut cursor)?;
        let effect_count = usize::try_from(effect_count).map_err(|_| StateError::CorruptFrame)?;
        let effect_bytes = effect_count
            .checked_mul(EFFECT_ENCODED_LEN)
            .ok_or(StateError::CorruptFrame)?;
        if payload.len().saturating_sub(cursor) != effect_bytes {
            return Err(StateError::CorruptFrame);
        }
        if mutation_count == 0 && effect_count == 0 {
            return Err(StateError::EmptyTransaction);
        }
        let mut effects = Vec::with_capacity(effect_count);
        let mut operation_ids = HashSet::with_capacity(effect_count);
        for _ in 0..effect_count {
            let operation = OperationId(read_u128_advance(payload, &mut cursor)?);
            let class = decode_class(
                *take(payload, &mut cursor, 1)?
                    .first()
                    .ok_or(StateError::CorruptFrame)?,
            )?;
            take(payload, &mut cursor, 32)?;
            if !operation_ids.insert(operation) {
                return Err(StateError::DuplicateOperation);
            }
            effects.push((operation, class));
        }
        if cursor != payload.len() {
            return Err(StateError::CorruptFrame);
        }
        if self.applied_transactions.contains(&transaction) {
            return Err(StateError::DuplicateTransaction);
        }
        if effects
            .iter()
            .any(|(operation, _)| self.effects.contains_key(operation))
        {
            return Err(StateError::DuplicateOperation);
        }

        for (object, version) in mutations {
            self.visible.insert(object, version);
        }
        for (operation, class) in effects {
            self.effects
                .insert(operation, (class, EffectStatus::Pending));
        }
        self.applied_transactions.insert(transaction);
        self.next_transaction = self.next_transaction.max(transaction.0.saturating_add(1));
        Ok(())
    }

    fn validate_prepared(&self, transaction: &PreparedTransaction) -> Result<(), StateError> {
        if self.applied_transactions.contains(&transaction.id) {
            return Err(StateError::DuplicateTransaction);
        }
        let mut objects = HashSet::with_capacity(transaction.mutations.len());
        for mutation in &transaction.mutations {
            if !objects.insert(mutation.object) {
                return Err(StateError::DuplicateMutation);
            }
        }
        let mut operations = HashSet::with_capacity(transaction.effects.len());
        for effect in &transaction.effects {
            if self.effects.contains_key(&effect.operation) || !operations.insert(effect.operation)
            {
                return Err(StateError::DuplicateOperation);
            }
        }
        Ok(())
    }

    fn apply_effect(&mut self, payload: &[u8]) -> Result<(), StateError> {
        let mut cursor = 1;
        let operation = OperationId(read_u128_advance(payload, &mut cursor)?);
        let status = decode_status(
            *take(payload, &mut cursor, 1)?
                .first()
                .ok_or(StateError::CorruptFrame)?,
        )?;
        if cursor != payload.len() {
            return Err(StateError::CorruptFrame);
        }
        let (_, current) = self
            .effects
            .get_mut(&operation)
            .ok_or(StateError::UnknownOperation)?;
        if !valid_transition(*current, status) {
            return Err(StateError::InvalidEffectTransition);
        }
        *current = status;
        Ok(())
    }
}

impl Default for StateEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_commit(transaction: PreparedTransaction) -> Result<(Vec<u8>, [u8; 32]), StateError> {
    let PreparedTransaction {
        id,
        actor,
        authority_reference,
        mutations,
        effects,
    } = transaction;
    let mut body = Vec::new();
    body.extend_from_slice(&id.0.to_le_bytes());
    body.extend_from_slice(&actor.to_le_bytes());
    body.extend_from_slice(&authority_reference.to_le_bytes());
    let mutation_count = u32::try_from(mutations.len()).map_err(|_| StateError::FrameTooLarge)?;
    body.extend_from_slice(&mutation_count.to_le_bytes());
    for mutation in &mutations {
        body.extend_from_slice(&mutation.object.0.to_le_bytes());
        body.extend_from_slice(&mutation.version.0.to_le_bytes());
        body.extend_from_slice(&mutation.content_digest);
    }
    let effect_count = u32::try_from(effects.len()).map_err(|_| StateError::FrameTooLarge)?;
    body.extend_from_slice(&effect_count.to_le_bytes());
    for effect in &effects {
        body.extend_from_slice(&effect.operation.0.to_le_bytes());
        body.push(effect.class as u8);
        body.extend_from_slice(&effect.request_digest);
    }
    let audit_commitment = commitment(&body);
    let mut payload = Vec::with_capacity(1 + audit_commitment.len() + body.len());
    payload.push(1);
    payload.extend_from_slice(&audit_commitment);
    payload.extend_from_slice(&body);
    Ok((payload, audit_commitment))
}

fn encode_effect(operation: OperationId, status: EffectStatus) -> Vec<u8> {
    let mut payload = vec![2];
    payload.extend_from_slice(&operation.0.to_le_bytes());
    payload.push(status as u8);
    payload
}

fn valid_transition(from: EffectStatus, to: EffectStatus) -> bool {
    matches!(
        (from, to),
        (
            EffectStatus::Pending | EffectStatus::Uncertain,
            EffectStatus::Acknowledged
                | EffectStatus::Rejected
                | EffectStatus::Uncertain
                | EffectStatus::CompensationRequired
        )
    )
}

fn decode_class(value: u8) -> Result<EffectClass, StateError> {
    match value {
        1 => Ok(EffectClass::Idempotent),
        2 => Ok(EffectClass::Deduplicated),
        3 => Ok(EffectClass::Compensatable),
        4 => Ok(EffectClass::Irreversible),
        _ => Err(StateError::UnsupportedRecord),
    }
}

fn decode_status(value: u8) -> Result<EffectStatus, StateError> {
    match value {
        1 => Ok(EffectStatus::Pending),
        2 => Ok(EffectStatus::Acknowledged),
        3 => Ok(EffectStatus::Rejected),
        4 => Ok(EffectStatus::Uncertain),
        5 => Ok(EffectStatus::CompensationRequired),
        _ => Err(StateError::UnsupportedRecord),
    }
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn commitment(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0; 32];
    for (chunk, seed) in output.chunks_exact_mut(8).zip([0_u8, 1, 2, 3]) {
        let mut material = Vec::with_capacity(bytes.len() + 1);
        material.push(seed);
        material.extend_from_slice(bytes);
        chunk.copy_from_slice(&checksum(&material).to_le_bytes());
    }
    output
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, StateError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(StateError::CorruptFrame)?;
    Ok(u32::from_le_bytes(
        slice.try_into().map_err(|_| StateError::CorruptFrame)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, StateError> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or(StateError::CorruptFrame)?;
    Ok(u64::from_le_bytes(
        slice.try_into().map_err(|_| StateError::CorruptFrame)?,
    ))
}

fn read_u32_advance(bytes: &[u8], cursor: &mut usize) -> Result<u32, StateError> {
    let value = read_u32(bytes, *cursor)?;
    *cursor += 4;
    Ok(value)
}

fn read_u64_advance(bytes: &[u8], cursor: &mut usize) -> Result<u64, StateError> {
    let value = read_u64(bytes, *cursor)?;
    *cursor += 8;
    Ok(value)
}

fn read_u128_advance(bytes: &[u8], cursor: &mut usize) -> Result<u128, StateError> {
    let slice = take(bytes, cursor, 16)?;
    Ok(u128::from_le_bytes(
        slice.try_into().map_err(|_| StateError::CorruptFrame)?,
    ))
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], StateError> {
    let end = cursor.checked_add(len).ok_or(StateError::CorruptFrame)?;
    let slice = bytes.get(*cursor..end).ok_or(StateError::CorruptFrame)?;
    *cursor = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transaction(engine: &mut StateEngine) -> PreparedTransaction {
        let mut transaction = engine.prepare(7, 11).unwrap();
        transaction.mutate(Mutation {
            object: ObjectId(1),
            version: VersionId(2),
            content_digest: [3; 32],
        });
        transaction.intend(EffectIntent {
            operation: OperationId(4),
            class: EffectClass::Deduplicated,
            request_digest: [5; 32],
        });
        transaction
    }

    #[test]
    fn every_torn_commit_is_invisible() {
        let mut engine = StateEngine::new();
        let prepared = transaction(&mut engine);
        engine.commit(prepared).unwrap();
        let log = engine.durable_log();
        for cut in 0..log.len() {
            let recovered = StateEngine::recover(&log[..cut]).unwrap();
            assert_eq!(recovered.visible_version(ObjectId(1)), None, "cut {cut}");
            assert_eq!(recovered.effect_status(OperationId(4)), None, "cut {cut}");
        }
        let recovered = StateEngine::recover(log).unwrap();
        assert_eq!(recovered.visible_version(ObjectId(1)), Some(VersionId(2)));
        assert_eq!(
            recovered.effect_status(OperationId(4)),
            Some(EffectStatus::Pending)
        );
    }

    #[test]
    fn corruption_is_not_treated_as_a_torn_tail() {
        let mut engine = StateEngine::new();
        let prepared = transaction(&mut engine);
        engine.commit(prepared).unwrap();
        let mut log = engine.durable_log().to_vec();
        *log.last_mut().unwrap() ^= 1;
        assert!(matches!(
            StateEngine::recover(&log),
            Err(StateError::CorruptFrame)
        ));
    }

    #[test]
    fn audit_commitment_is_persisted_and_independently_validated() {
        let mut engine = StateEngine::new();
        let prepared = transaction(&mut engine);
        let receipt = engine.commit(prepared).unwrap();
        assert_ne!(receipt.audit_commitment, [0; 32]);

        let mut log = engine.durable_log().to_vec();
        log[HEADER_LEN + 1] ^= 1;
        let payload_checksum = checksum(&log[HEADER_LEN..]);
        log[8..16].copy_from_slice(&payload_checksum.to_le_bytes());
        assert!(matches!(
            StateEngine::recover(&log),
            Err(StateError::AuditMismatch)
        ));
    }

    #[test]
    fn uncertain_effect_survives_recovery_and_can_reconcile() {
        let mut engine = StateEngine::new();
        let prepared = transaction(&mut engine);
        engine.commit(prepared).unwrap();
        engine
            .record_effect(OperationId(4), EffectStatus::Uncertain)
            .unwrap();
        let mut recovered = StateEngine::recover(engine.durable_log()).unwrap();
        assert_eq!(
            recovered.effect_status(OperationId(4)),
            Some(EffectStatus::Uncertain)
        );
        recovered
            .record_effect(OperationId(4), EffectStatus::Acknowledged)
            .unwrap();
        assert_eq!(
            recovered.effect_status(OperationId(4)),
            Some(EffectStatus::Acknowledged)
        );
    }

    #[test]
    fn duplicate_content_is_rejected_before_log_append() {
        let mut engine = StateEngine::new();
        let mut duplicate_mutation = engine.prepare(1, 1).unwrap();
        let mutation = Mutation {
            object: ObjectId(9),
            version: VersionId(1),
            content_digest: [0; 32],
        };
        duplicate_mutation.mutate(mutation);
        duplicate_mutation.mutate(mutation);
        assert_eq!(
            engine.commit(duplicate_mutation),
            Err(StateError::DuplicateMutation)
        );
        assert!(engine.durable_log().is_empty());

        let prepared = transaction(&mut engine);
        engine.commit(prepared).unwrap();
        let length = engine.durable_log().len();
        let mut duplicate_operation = engine.prepare(1, 1).unwrap();
        duplicate_operation.intend(EffectIntent {
            operation: OperationId(4),
            class: EffectClass::Deduplicated,
            request_digest: [1; 32],
        });
        assert_eq!(
            engine.commit(duplicate_operation),
            Err(StateError::DuplicateOperation)
        );
        assert_eq!(engine.durable_log().len(), length);
    }

    #[test]
    fn replayed_commit_frame_is_rejected() {
        let mut engine = StateEngine::new();
        let prepared = transaction(&mut engine);
        engine.commit(prepared).unwrap();
        let mut replayed = engine.durable_log().to_vec();
        replayed.extend_from_slice(engine.durable_log());
        assert!(matches!(
            StateEngine::recover(&replayed),
            Err(StateError::DuplicateTransaction)
        ));
    }

    #[test]
    fn transaction_identifiers_do_not_wrap_or_alias() {
        let mut engine = StateEngine {
            next_transaction: u64::MAX - 1,
            ..StateEngine::new()
        };
        let first = engine.prepare(1, 1).unwrap();
        assert_eq!(first.id(), TransactionId(u64::MAX - 1));
        assert_eq!(
            engine.prepare(1, 1),
            Err(StateError::TransactionIdExhausted)
        );
    }

    #[test]
    fn default_engine_uses_the_same_valid_identifier_origin_as_new() {
        let mut engine = StateEngine::default();
        assert_eq!(engine.prepare(1, 1).unwrap().id(), TransactionId(1));
    }

    #[test]
    fn recovery_rejects_impossible_counts_before_allocating() {
        let mut body = Vec::new();
        body.extend_from_slice(&1_u64.to_le_bytes());
        body.extend_from_slice(&1_u128.to_le_bytes());
        body.extend_from_slice(&1_u64.to_le_bytes());
        body.extend_from_slice(&u32::MAX.to_le_bytes());
        let audit = commitment(&body);
        let mut payload = vec![1];
        payload.extend_from_slice(&audit);
        payload.extend_from_slice(&body);
        let mut log = Vec::new();
        log.extend_from_slice(&MAGIC.to_le_bytes());
        log.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
        log.extend_from_slice(&checksum(&payload).to_le_bytes());
        log.extend_from_slice(&payload);
        assert_eq!(
            StateEngine::recover(&log).err(),
            Some(StateError::CorruptFrame)
        );

        let mut empty_body = Vec::new();
        empty_body.extend_from_slice(&2_u64.to_le_bytes());
        empty_body.extend_from_slice(&1_u128.to_le_bytes());
        empty_body.extend_from_slice(&1_u64.to_le_bytes());
        empty_body.extend_from_slice(&0_u32.to_le_bytes());
        empty_body.extend_from_slice(&0_u32.to_le_bytes());
        let empty_audit = commitment(&empty_body);
        let mut empty_payload = vec![1];
        empty_payload.extend_from_slice(&empty_audit);
        empty_payload.extend_from_slice(&empty_body);
        let mut empty_log = Vec::new();
        empty_log.extend_from_slice(&MAGIC.to_le_bytes());
        empty_log.extend_from_slice(&u32::try_from(empty_payload.len()).unwrap().to_le_bytes());
        empty_log.extend_from_slice(&checksum(&empty_payload).to_le_bytes());
        empty_log.extend_from_slice(&empty_payload);
        assert_eq!(
            StateEngine::recover(&empty_log).err(),
            Some(StateError::EmptyTransaction)
        );
    }
}
