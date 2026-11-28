//! Storage provider interface reference implementation (RFC-0019).
//!
//! The storage provider interface is the abstraction layer between the State
//! Architecture's object model and physical storage backends. It enables
//! replaceable storage mechanisms (invariant I-9): changing the backend from
//! `NVMe` to persistent memory to cloud storage to tape must not require
//! changing the object model.
//!
//! Mandatory operations (RFC-0019.2): [`StorageProvider::put`],
//! [`StorageProvider::get`], [`StorageProvider::delete`],
//! [`StorageProvider::list`], [`StorageProvider::get_metadata`].
//! Provider registration (RFC-0019.3) via [`StorageRegistry`].
//! Tiering (RFC-0019.5/0019.6) via [`Tier`].
//! Removability test (RFC-0019.7) via [`StorageRegistry::remove`].
//! Two-phase commit participation (RFC-0019.9, RFC-0016) via
//! [`TransactionalParticipant`], used by [`InMemoryProvider`].
//! Health events (RFC-0019.11, RFC-0031.44) emitted by
//! [`InMemoryProvider`] on capacity, latency, error-rate, and offline
//! conditions. Content integrity checksums (RFC-0017 cross-ref) via
//! [`MerkleDAG`](aether_integrity_model::MerkleDAG).

use aether_event_model::{EventClass, EventEmitter, PlatformEvent};
use aether_integrity_model::MerkleDAG;
use aether_transaction_model::{TransactionCoordinator, TransactionId};
use aether_types::{DomainId, ObjectId};
use std::collections::{BTreeMap, HashMap};

const STORAGE_EVENT_CLASS: u64 = EventClass::Storage.id();

/// Health event types for storage providers (RFC-0019.11, RFC-0031.44).
/// Class: Storage (0x0500).
mod storage_event_type {
    /// Provider capacity dropped below 10% remaining (RFC-0019.11).
    pub const LOW_CAPACITY: u64 = 1;
    /// Latency exceeded declared maximum by more than 50% (RFC-0019.11).
    pub const LATENCY_EXCEEDED: u64 = 2;
    /// Error rate exceeded declared threshold (RFC-0019.11).
    pub const ERROR_RATE: u64 = 3;
    /// Provider went offline (RFC-0019.11).
    pub const OFFLINE: u64 = 4;
}

/// Threshold: capacity remaining below this fraction triggers a low-capacity
/// health event (RFC-0019.11).
const LOW_CAPACITY_THRESHOLD: f64 = 0.10;

/// Threshold: measured latency above this multiple of the declared maximum
/// triggers a latency-exceeded health event (RFC-0019.11).
const LATENCY_EXCEEDED_MULTIPLIER: f64 = 1.50;

/// Threshold: error rate above this fraction triggers an error-rate health
/// event (RFC-0019.11).
const ERROR_RATE_THRESHOLD: f64 = 0.05;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by storage providers (RFC-0019.4).
///
/// A provider that cannot satisfy an operation returns a structured error and
/// MUST NOT corrupt state (RFC-0019.4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    /// The object does not exist in this provider.
    NotFound,
    /// The provider has insufficient capacity for the requested operation.
    CapacityExhausted,
    /// The provider is offline or unreachable.
    Offline,
    /// The object's integrity check failed on read; corrupted data was not
    /// returned.
    Corrupted,
    /// The provider cannot participate in the requested transaction.
    TransactionUnsupported,
    /// The requested tier is not supported by this provider.
    TierNotSupported,
    /// The provider is already registered or not registered in the registry.
    Registry,
}

// ---------------------------------------------------------------------------
// Provider type
// ---------------------------------------------------------------------------

/// Media type declared by a provider at registration (RFC-0019.3).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ProviderType {
    /// Block-addressable media (`NVMe`, SSD, HDD).
    Block = 0,
    /// Byte-addressable persistent memory.
    ByteAddressable = 1,
    /// Object storage (cloud, S3-like).
    Object = 2,
    /// Tape / archival media.
    Tape = 3,
    /// Network-attached storage.
    Network = 4,
}

// ---------------------------------------------------------------------------
// Latency bounds
// ---------------------------------------------------------------------------

/// Declared latency bounds for a provider (RFC-0019.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LatencyBounds {
    /// Minimum observed latency in nanoseconds.
    pub min_latency_ns: u64,
    /// Typical latency in nanoseconds.
    pub typical_latency_ns: u64,
    /// Maximum declared latency in nanoseconds.
    pub max_latency_ns: u64,
}

impl LatencyBounds {
    /// Creates latency bounds, rejecting `min > typical > max` violations.
    #[must_use]
    pub const fn new(min: u64, typical: u64, max: u64) -> Option<Self> {
        if min <= typical && typical <= max {
            Some(Self {
                min_latency_ns: min,
                typical_latency_ns: typical,
                max_latency_ns: max,
            })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tier
// ---------------------------------------------------------------------------

/// Storage tier representing a cost/performance tradeoff (RFC-0019.6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Tier {
    /// Lowest latency; highest cost.
    Hot = 0,
    /// Moderate latency and cost.
    Warm = 1,
    /// High latency; lowest cost.
    Cold = 2,
    /// Offline; highest latency; very low cost.
    Archive = 3,
}

impl Tier {
    /// Returns the tier's display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Tier::Hot => "HOT",
            Tier::Warm => "WARM",
            Tier::Cold => "COLD",
            Tier::Archive => "ARCHIVE",
        }
    }
}

// ---------------------------------------------------------------------------
// Storage metadata
// ---------------------------------------------------------------------------

/// Storage metadata for an object (RFC-0019.2: `get_metadata` returns size,
/// tier, replica count, plus content integrity hash and modification time).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageMetadata {
    /// Object identity.
    pub object_id: ObjectId,
    /// Content size in bytes.
    pub size_bytes: u64,
    /// Assigned storage tier.
    pub tier: Tier,
    /// Number of durable replicas.
    pub replica_count: u32,
    /// Content integrity hash (BLAKE3-256 of content, folded to `ObjectId`,
    /// RFC-0017 cross-ref).
    pub content_hash: ObjectId,
    /// Last modification time in scheduling nanoseconds (RFC-0012).
    pub modified_at_ns: u64,
}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

/// Result type for storage provider operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Mandatory storage provider interface (RFC-0019.2).
///
/// Every storage provider implements these operations. The State Architecture
/// interacts with storage exclusively through this trait and MUST NOT make
/// assumptions about physical media, block size, latency, durability, or
/// connectivity (RFC-0019.1).
pub trait StorageProvider {
    /// Returns the provider's human-readable name.
    fn name(&self) -> &str;

    /// Returns the provider type (media type, RFC-0019.3).
    fn provider_type(&self) -> ProviderType;

    /// Returns the total capacity in bytes.
    fn capacity_bytes(&self) -> u64;

    /// Returns the declared latency bounds (RFC-0019.3).
    fn latency_bounds(&self) -> LatencyBounds;

    /// Returns the durability level (0.0–1.0 annual uptime fraction).
    fn durability(&self) -> f64;

    /// Returns supported encryption modes as a bitmask (RFC-0019.3).
    fn encryption_modes(&self) -> u32;

    /// Store content under identity (RFC-0019.2 `put`).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::CapacityExhausted`] if there is insufficient
    /// space, or [`StorageError::Offline`] if the provider is offline.
    fn put(&mut self, object_id: ObjectId, content: &[u8], tier: Tier) -> StorageResult<()>;

    /// Retrieve content by identity (RFC-0019.2 `get`).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`] if the object does not exist, or
    /// [`StorageError::Corrupted`] if the integrity check fails.
    fn get(&self, object_id: ObjectId) -> StorageResult<Vec<u8>>;

    /// Remove content by identity (RFC-0019.2 `delete`, subject to lifecycle
    /// FSM).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`] if the object does not exist,
    /// or [`StorageError::Offline`] if the provider is offline.
    fn delete(&mut self, object_id: ObjectId) -> StorageResult<()>;

    /// Enumerate objects by identity prefix (RFC-0019.2 `list`).
    ///
    /// Returns at most `limit` object identities whose big-endian byte
    /// representation starts with `prefix`'s significant bytes. A prefix of
    /// `ObjectId(0)` matches every object (no filter).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Offline`] if the provider is offline.
    fn list(&self, prefix: ObjectId, limit: u32) -> StorageResult<Vec<ObjectId>>;

    /// Retrieve storage metadata (RFC-0019.2 `get_metadata`).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`] if the object does not exist.
    fn get_metadata(&self, object_id: ObjectId) -> StorageResult<StorageMetadata>;
}

// ---------------------------------------------------------------------------
// Storage registry
// ---------------------------------------------------------------------------

/// Registry of storage providers (RFC-0019.3).
///
/// Providers are registered at platform initialization with their
/// characteristics. The registry enables the removability test
/// (RFC-0019.7): removing a provider must not violate any State Architecture
/// invariant, or the provider is in the trusted core and cannot be removed.
#[derive(Default)]
pub struct StorageRegistry {
    providers: BTreeMap<String, Box<dyn StorageProvider>>,
    by_type: BTreeMap<ProviderType, Vec<String>>,
}

impl StorageRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a provider (RFC-0019.3). The provider declares its type and
    /// characteristics at registration.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Registry`] if a provider with the same name is
    /// already registered.
    pub fn register(&mut self, provider: Box<dyn StorageProvider>) -> Result<(), StorageError> {
        let name = provider.name().to_string();
        let provider_type = provider.provider_type();
        if self.providers.contains_key(&name) {
            return Err(StorageError::Registry);
        }
        self.by_type
            .entry(provider_type)
            .or_default()
            .push(name.clone());
        self.providers.insert(name, provider);
        Ok(())
    }

    /// Removes a provider by name (RFC-0019.7 removability test).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Registry`] if no provider with that name is
    /// registered.
    pub fn remove(&mut self, name: &str) -> Result<Box<dyn StorageProvider>, StorageError> {
        let provider = self.providers.remove(name).ok_or(StorageError::Registry)?;
        let provider_type = provider.provider_type();
        if let Some(names) = self.by_type.get_mut(&provider_type) {
            names.retain(|n| n != name);
            if names.is_empty() {
                self.by_type.remove(&provider_type);
            }
        }
        Ok(provider)
    }

    /// Returns a reference to the named provider, if registered.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn StorageProvider> {
        self.providers.get(name).map(AsRef::as_ref)
    }

    /// Returns all registered providers of the given type.
    #[must_use]
    pub fn providers_of_type(&self, provider_type: ProviderType) -> Vec<&dyn StorageProvider> {
        self.by_type
            .get(&provider_type)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|n| self.providers.get(n).map(AsRef::as_ref))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the names of all registered providers.
    #[must_use]
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.keys().map(String::as_str).collect()
    }

    /// Returns the number of registered providers.
    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

// ---------------------------------------------------------------------------
// Two-phase commit participation
// ---------------------------------------------------------------------------

/// Two-phase commit participation for a storage provider (RFC-0019.9, RFC-0016).
///
/// A provider registers itself with a [`TransactionCoordinator`] and routes
/// its mutations through it, so that put/delete operations are atomic across
/// all participants (RFC-0016.3 all-or-nothing invariant). Providers that
/// cannot participate in two-phase commit MAY be used as write-through caches
/// with a conforming primary provider handling transactions (RFC-0019.9).
pub struct TransactionalParticipant {
    /// The coordinator driving two-phase commit.
    coordinator: TransactionCoordinator,
    /// The transaction this provider is currently participating in, if any.
    current_xid: Option<TransactionId>,
    /// Mutations staged for the current transaction, not yet applied.
    staged: HashMap<ObjectId, Vec<u8>>,
    /// Tiers assigned to staged puts.
    staged_tiers: HashMap<ObjectId, Tier>,
    /// Whether this provider can prepare (vote YES in phase 1).
    can_prepare: bool,
}

impl TransactionalParticipant {
    /// Creates a new transactional participant.
    #[must_use]
    pub fn new(can_prepare: bool) -> Self {
        Self {
            coordinator: TransactionCoordinator::new(),
            current_xid: None,
            staged: HashMap::new(),
            staged_tiers: HashMap::new(),
            can_prepare,
        }
    }

    /// Begins a new transaction and registers this provider as a participant
    /// (RFC-0016.1, RFC-0019.9).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::TransactionUnsupported`] if a transaction is
    /// already in progress.
    pub fn begin_transaction(
        &mut self,
        xid: TransactionId,
        timestamp_ns: u64,
    ) -> StorageResult<TransactionId> {
        if self.current_xid.is_some() {
            return Err(StorageError::TransactionUnsupported);
        }
        self.coordinator
            .begin(xid, timestamp_ns, 1, None)
            .map_err(|_| StorageError::TransactionUnsupported)?;
        self.coordinator
            .register_participant(&xid, self.can_prepare)
            .map_err(|_| StorageError::TransactionUnsupported)?;
        self.current_xid = Some(xid);
        Ok(xid)
    }

    /// Stages a put mutation for the current transaction.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::TransactionUnsupported`] if no transaction is
    /// in progress.
    pub fn stage_put(
        &mut self,
        object_id: ObjectId,
        content: Vec<u8>,
        tier: Tier,
    ) -> StorageResult<()> {
        let xid = self
            .current_xid
            .ok_or(StorageError::TransactionUnsupported)?;
        self.coordinator
            .add_mutation(&xid, object_id, content.clone())
            .map_err(|_| StorageError::TransactionUnsupported)?;
        self.staged.insert(object_id, content);
        self.staged_tiers.insert(object_id, tier);
        Ok(())
    }

    /// Stages a delete mutation for the current transaction.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::TransactionUnsupported`] if no transaction is
    /// in progress.
    pub fn stage_delete(&mut self, object_id: ObjectId) -> StorageResult<()> {
        let xid = self
            .current_xid
            .ok_or(StorageError::TransactionUnsupported)?;
        self.coordinator
            .add_mutation(&xid, object_id, Vec::new())
            .map_err(|_| StorageError::TransactionUnsupported)?;
        self.staged.insert(object_id, Vec::new());
        self.staged_tiers.remove(&object_id);
        Ok(())
    }

    /// Commits the current transaction using two-phase commit
    /// (RFC-0016.4, RFC-0019.9). On success, staged mutations are ready to be
    /// applied to the provider's store by the caller via [`apply_staged`].
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::TransactionUnsupported`] if no transaction is
    /// in progress, or if two-phase commit fails (prepare failure,
    /// write-write conflict).
    pub fn commit_transaction(&mut self, timestamp_ns: u64) -> StorageResult<()> {
        let xid = self
            .current_xid
            .ok_or(StorageError::TransactionUnsupported)?;
        self.coordinator
            .commit(&xid, timestamp_ns)
            .map_err(|_| StorageError::TransactionUnsupported)?;
        Ok(())
    }

    /// Aborts the current transaction, discarding all staged mutations
    /// (RFC-0016.1, RFC-0016.5).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::TransactionUnsupported`] if no transaction is
    /// in progress.
    pub fn abort_transaction(&mut self) -> StorageResult<()> {
        let xid = self
            .current_xid
            .ok_or(StorageError::TransactionUnsupported)?;
        self.coordinator
            .abort(&xid)
            .map_err(|_| StorageError::TransactionUnsupported)?;
        self.staged.clear();
        self.staged_tiers.clear();
        self.current_xid = None;
        Ok(())
    }

    /// Returns the staged mutations awaiting commit.
    #[must_use]
    pub fn staged_mutations(&self) -> &HashMap<ObjectId, Vec<u8>> {
        &self.staged
    }

    /// Returns the staged tiers awaiting commit.
    #[must_use]
    pub fn staged_tiers(&self) -> &HashMap<ObjectId, Tier> {
        &self.staged_tiers
    }

    /// Clears staged state after a successful commit and marks the
    /// transaction complete. Call this after applying staged mutations to
    /// the store.
    pub fn mark_committed(&mut self) {
        self.staged.clear();
        self.staged_tiers.clear();
        self.current_xid = None;
    }

    /// Returns whether a transaction is currently in progress.
    #[must_use]
    pub fn in_transaction(&self) -> bool {
        self.current_xid.is_some()
    }

    /// Returns all emitted transaction events.
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        self.coordinator.events()
    }
}

/// Applies staged committed mutations to a provider's store.
///
/// Used after [`TransactionalParticipant::commit_transaction`] succeeds to
/// durably apply the staged put/delete operations.
///
/// # Errors
///
/// Returns [`StorageError`] if any staged put or delete fails on the
/// provider.
pub fn apply_staged<P: StorageProvider>(
    provider: &mut P,
    participant: &TransactionalParticipant,
) -> StorageResult<()> {
    for (object_id, content) in participant.staged_mutations() {
        let tier = *participant
            .staged_tiers()
            .get(object_id)
            .unwrap_or(&Tier::Hot);
        if content.is_empty() {
            provider.delete(*object_id)?;
        } else {
            provider.put(*object_id, content, tier)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Health statistics
// ---------------------------------------------------------------------------

/// Operation statistics used for health event emission (RFC-0019.11).
#[derive(Debug, Default)]
struct HealthStats {
    operation_count: u64,
    error_count: u64,
    total_latency_ns: u64,
}

// ---------------------------------------------------------------------------
// In-memory provider
// ---------------------------------------------------------------------------

/// In-memory storage provider implementing [`StorageProvider`] with optional
/// two-phase commit participation and health event emission.
///
/// All content is held in memory (RFC-0019.2). It registers as a two-phase
/// commit participant (RFC-0019.9, RFC-0016) and emits health events
/// (RFC-0019.11, RFC-0031.44) when capacity drops below 10%, latency exceeds
/// the declared maximum by more than 50%, the error rate exceeds the
/// threshold, or the provider goes offline.
pub struct InMemoryProvider {
    /// Provider name.
    name: String,
    /// Media type.
    provider_type: ProviderType,
    /// Total capacity in bytes.
    capacity_bytes: u64,
    /// Declared latency bounds.
    latency: LatencyBounds,
    /// Durability (0.0–1.0 annual uptime fraction).
    durability: f64,
    /// Supported encryption modes bitmask.
    encryption_modes: u32,
    /// Object store: identity -> content bytes.
    store: HashMap<ObjectId, Vec<u8>>,
    /// Object metadata: identity -> metadata.
    metadata: HashMap<ObjectId, StorageMetadata>,
    /// Bytes currently consumed.
    used_bytes: u64,
    /// Event emitter for health events (RFC-0019.11).
    emitter: EventEmitter,
    /// Operation statistics for health monitoring.
    stats: HealthStats,
    /// Whether the provider is offline.
    offline: bool,
    /// Optional two-phase commit participation (RFC-0019.9).
    txn: Option<TransactionalParticipant>,
}

impl InMemoryProvider {
    /// Creates a new in-memory provider with the given characteristics
    /// (RFC-0019.3).
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        provider_type: ProviderType,
        capacity_bytes: u64,
        latency: LatencyBounds,
        durability: f64,
        encryption_modes: u32,
    ) -> Self {
        Self {
            name: name.into(),
            provider_type,
            capacity_bytes,
            latency,
            durability,
            encryption_modes,
            store: HashMap::new(),
            metadata: HashMap::new(),
            used_bytes: 0,
            emitter: EventEmitter::new(),
            stats: HealthStats::default(),
            offline: false,
            txn: None,
        }
    }

    /// Enables two-phase commit participation for this provider
    /// (RFC-0019.9, RFC-0016).
    #[must_use]
    pub fn with_transactional_participant(mut self, can_prepare: bool) -> Self {
        self.txn = Some(TransactionalParticipant::new(can_prepare));
        self
    }

    /// Returns a mutable reference to the transactional participant, if enabled.
    #[must_use]
    pub fn txn_participant_mut(&mut self) -> Option<&mut TransactionalParticipant> {
        self.txn.as_mut()
    }

    /// Returns the provider's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of objects currently stored.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.store.len()
    }

    /// Returns the number of bytes currently consumed.
    #[must_use]
    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    /// Returns the fraction of capacity remaining (0.0–1.0).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn capacity_remaining_fraction(&self) -> f64 {
        if self.capacity_bytes == 0 {
            return 0.0;
        }
        let used = self.used_bytes.min(self.capacity_bytes) as f64;
        let total = self.capacity_bytes as f64;
        (total - used) / total
    }

    /// Sets whether the provider is offline, emitting a health event on
    /// transition (RFC-0019.11).
    pub fn set_offline(&mut self, offline: bool) {
        if self.offline == offline {
            return;
        }
        self.offline = offline;
        if offline {
            self.emit_health_event(storage_event_type::OFFLINE, b"provider offline");
        }
    }

    /// Records an observed operation for health monitoring, emitting health
    /// events when thresholds are exceeded (RFC-0019.11).
    #[allow(clippy::cast_precision_loss)]
    pub fn record_operation(&mut self, latency_ns: u64, success: bool) {
        self.stats.operation_count += 1;
        self.stats.total_latency_ns = self.stats.total_latency_ns.saturating_add(latency_ns);
        if !success {
            self.stats.error_count += 1;
        }

        // Low capacity check.
        if self.capacity_remaining_fraction() < LOW_CAPACITY_THRESHOLD {
            self.emit_health_event(
                storage_event_type::LOW_CAPACITY,
                b"capacity below 10% remaining",
            );
        }

        // Latency exceedance check.
        let max = self.latency.max_latency_ns as f64;
        if max > 0.0 && (latency_ns as f64) > max * LATENCY_EXCEEDED_MULTIPLIER {
            self.emit_health_event(
                storage_event_type::LATENCY_EXCEEDED,
                b"latency exceeds declared maximum by more than 50%",
            );
        }

        // Error rate check.
        if self.stats.operation_count > 0 {
            let error_rate = self.stats.error_count as f64 / self.stats.operation_count as f64;
            if error_rate > ERROR_RATE_THRESHOLD {
                self.emit_health_event(
                    storage_event_type::ERROR_RATE,
                    b"error rate exceeds declared threshold",
                );
            }
        }
    }

    /// Returns all emitted health events.
    #[must_use]
    pub fn health_events(&self) -> &[PlatformEvent] {
        self.emitter.events()
    }

    /// Returns the number of emitted health events.
    #[must_use]
    pub fn health_event_count(&self) -> usize {
        self.emitter.event_count()
    }

    fn emit_health_event(&mut self, event_type: u64, message: &[u8]) {
        let mut payload = Vec::new();
        payload.extend_from_slice(message);
        payload.extend_from_slice(&self.used_bytes.to_le_bytes());
        payload.extend_from_slice(&self.capacity_bytes.to_le_bytes());
        let event = PlatformEvent {
            event_id: 0,
            event_class: STORAGE_EVENT_CLASS,
            event_type,
            timestamp_ns: 0,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 50,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload,
        };
        let _ = self.emitter.emit(event);
    }

    fn content_hash(content: &[u8]) -> ObjectId {
        let identity = MerkleDAG::leaf_identity(content);
        ObjectId::from(identity)
    }

    fn prefix_bytes(prefix: ObjectId) -> Vec<u8> {
        let bytes = prefix.0.to_be_bytes();
        bytes
            .iter()
            .position(|&b| b != 0)
            .map(|pos| bytes[pos..].to_vec())
            .unwrap_or_default()
    }

    fn matches_prefix(id: ObjectId, prefix: ObjectId) -> bool {
        let p = Self::prefix_bytes(prefix);
        if p.is_empty() {
            return true;
        }
        let id_bytes = Self::prefix_bytes(id);
        id_bytes.starts_with(&p)
    }
}

impl StorageProvider for InMemoryProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> ProviderType {
        self.provider_type
    }

    fn capacity_bytes(&self) -> u64 {
        self.capacity_bytes
    }

    fn latency_bounds(&self) -> LatencyBounds {
        self.latency
    }

    fn durability(&self) -> f64 {
        self.durability
    }

    fn encryption_modes(&self) -> u32 {
        self.encryption_modes
    }

    fn put(&mut self, object_id: ObjectId, content: &[u8], tier: Tier) -> StorageResult<()> {
        if self.offline {
            return Err(StorageError::Offline);
        }
        let content_len = content.len() as u64;
        let existing_len = self.store.get(&object_id).map_or(0, |c| c.len() as u64);
        let needed = content_len.saturating_sub(existing_len);
        if self.used_bytes.saturating_add(needed) > self.capacity_bytes {
            self.record_operation(0, false);
            return Err(StorageError::CapacityExhausted);
        }
        let hash = Self::content_hash(content);
        let metadata = StorageMetadata {
            object_id,
            size_bytes: content_len,
            tier,
            replica_count: 1,
            content_hash: hash,
            modified_at_ns: 0,
        };
        self.used_bytes = self
            .used_bytes
            .saturating_sub(existing_len)
            .saturating_add(content_len);
        self.store.insert(object_id, content.to_vec());
        self.metadata.insert(object_id, metadata);
        self.record_operation(0, true);
        Ok(())
    }

    fn get(&self, object_id: ObjectId) -> StorageResult<Vec<u8>> {
        if self.offline {
            return Err(StorageError::Offline);
        }
        let content = self.store.get(&object_id).cloned();
        match content {
            Some(content) => {
                // Integrity check: verify content hash matches metadata
                // (RFC-0017 cross-ref, RFC-0019.10 atomicity at provider level).
                if let Some(meta) = self.metadata.get(&object_id) {
                    let hash = Self::content_hash(&content);
                    if hash != meta.content_hash {
                        return Err(StorageError::Corrupted);
                    }
                }
                Ok(content)
            }
            None => Err(StorageError::NotFound),
        }
    }

    fn delete(&mut self, object_id: ObjectId) -> StorageResult<()> {
        if self.offline {
            return Err(StorageError::Offline);
        }
        if self.store.remove(&object_id).is_some() {
            self.metadata.remove(&object_id);
            // Recompute used bytes (simplified: subtract is approximate, so
            // recompute from store).
            self.used_bytes = self.store.values().map(|c| c.len() as u64).sum();
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    fn list(&self, prefix: ObjectId, limit: u32) -> StorageResult<Vec<ObjectId>> {
        if self.offline {
            return Err(StorageError::Offline);
        }
        let mut results = Vec::new();
        for id in self.store.keys() {
            if Self::matches_prefix(*id, prefix) {
                results.push(*id);
                if results.len() >= limit as usize {
                    break;
                }
            }
        }
        Ok(results)
    }

    fn get_metadata(&self, object_id: ObjectId) -> StorageResult<StorageMetadata> {
        self.metadata
            .get(&object_id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }
}

impl Default for InMemoryProvider {
    fn default() -> Self {
        let latency = LatencyBounds::new(100, 1000, 10_000).unwrap();
        Self::new(
            "default-in-memory",
            ProviderType::ByteAddressable,
            1024 * 1024 * 1024, // 1 GiB
            latency,
            0.999,
            0x01, // AES-256
        )
    }
}

// ---------------------------------------------------------------------------
// Unit tests (CTS-STATE-PROV-1..5 coverage)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_latency() -> LatencyBounds {
        LatencyBounds::new(100, 1000, 10_000).unwrap()
    }

    fn make_provider() -> InMemoryProvider {
        InMemoryProvider::new(
            "test-provider",
            ProviderType::Block,
            1024,
            test_latency(),
            0.999,
            0x01,
        )
    }

    fn make_object_id(value: u128) -> ObjectId {
        ObjectId(value)
    }

    /// CTS-STATE-PROV-1 (RFC-0019.2): mandatory operations — put, get,
    /// delete, list, get_metadata all work end to end.
    #[test]
    fn cts_prov_1_mandatory_operations() {
        let mut prov = make_provider();
        let id = make_object_id(0x4242);
        let content = b"hello storage";

        // put
        prov.put(id, content, Tier::Hot).unwrap();
        assert_eq!(prov.object_count(), 1);

        // get
        let got = prov.get(id).unwrap();
        assert_eq!(got, content.to_vec());

        // get_metadata
        let meta = prov.get_metadata(id).unwrap();
        assert_eq!(meta.object_id, id);
        assert_eq!(meta.size_bytes, content.len() as u64);
        assert_eq!(meta.tier, Tier::Hot);
        assert_eq!(meta.replica_count, 1);
        assert_eq!(meta.content_hash, InMemoryProvider::content_hash(content));

        // list
        let listed = prov.list(ObjectId(0), 10).unwrap();
        assert!(listed.contains(&id));

        // delete
        prov.delete(id).unwrap();
        assert_eq!(prov.object_count(), 0);
        assert_eq!(prov.get(id), Err(StorageError::NotFound));
        assert_eq!(prov.get_metadata(id), Err(StorageError::NotFound));
    }

    /// CTS-STATE-PROV-1 (RFC-0019.2): get on missing object returns NotFound.
    #[test]
    fn cts_prov_1_get_missing_object() {
        let mut prov = make_provider();
        assert_eq!(
            prov.get(make_object_id(0x9999)),
            Err(StorageError::NotFound)
        );
        assert_eq!(
            prov.delete(make_object_id(0x9999)),
            Err(StorageError::NotFound)
        );
    }

    /// CTS-STATE-PROV-1 (RFC-0019.2): list with prefix filtering and limit.
    #[test]
    fn cts_prov_1_list_prefix_and_limit() {
        let mut prov = make_provider();
        prov.put(make_object_id(0x1000), b"a", Tier::Hot).unwrap();
        prov.put(make_object_id(0x1001), b"b", Tier::Hot).unwrap();
        prov.put(make_object_id(0x2000), b"c", Tier::Hot).unwrap();

        // ObjectId(0) prefix matches everything.
        let all = prov.list(ObjectId(0), 10).unwrap();
        assert_eq!(all.len(), 3);

        // Prefix 0x10 matches objects whose significant bytes start with 0x10.
        let prefixed = prov.list(make_object_id(0x10), 10).unwrap();
        assert_eq!(prefixed.len(), 2);

        // Limit is respected.
        let limited = prov.list(ObjectId(0), 2).unwrap();
        assert_eq!(limited.len(), 2);

        // An exact object ID matches only itself.
        let exact = prov.list(make_object_id(0x1000), 10).unwrap();
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0], make_object_id(0x1000));
    }

    /// CTS-STATE-PROV-2 (RFC-0019.5): tier assignment per object.
    #[test]
    fn cts_prov_2_tier_assignment() {
        let mut prov = make_provider();
        for (id, tier) in [
            (make_object_id(1), Tier::Hot),
            (make_object_id(2), Tier::Warm),
            (make_object_id(3), Tier::Cold),
            (make_object_id(4), Tier::Archive),
        ] {
            prov.put(id, b"data", tier).unwrap();
            let meta = prov.get_metadata(id).unwrap();
            assert_eq!(meta.tier, tier, "tier mismatch for {}", id.0);
            assert_eq!(
                meta.tier.name(),
                match tier {
                    Tier::Hot => "HOT",
                    Tier::Warm => "WARM",
                    Tier::Cold => "COLD",
                    Tier::Archive => "ARCHIVE",
                }
            );
        }
    }

    /// CTS-STATE-PROV-3 (RFC-0019.7): removability test — removing a
    /// provider from the registry does not break the remaining providers.
    #[test]
    fn cts_prov_3_removability_test() {
        let mut registry = StorageRegistry::new();
        let primary: Box<dyn StorageProvider> = Box::new(make_provider());
        registry.register(primary).unwrap();

        let mut secondary = InMemoryProvider::new(
            "secondary",
            ProviderType::Network,
            2048,
            test_latency(),
            0.999,
            0x01,
        );
        secondary
            .put(make_object_id(0x7777), b"secondary data", Tier::Warm)
            .unwrap();
        registry.register(Box::new(secondary)).unwrap();

        assert_eq!(registry.provider_count(), 2);
        assert!(registry.get("secondary").is_some());

        // Remove the primary provider; secondary must still work.
        let removed = registry.remove("test-provider").unwrap();
        assert_eq!(removed.name(), "test-provider");
        assert_eq!(registry.provider_count(), 1);

        let secondary = registry.get("secondary").unwrap();
        let got = secondary.get(make_object_id(0x7777)).unwrap();
        assert_eq!(got, b"secondary data");
        let meta = secondary.get_metadata(make_object_id(0x7777)).unwrap();
        assert_eq!(meta.tier, Tier::Warm);
    }

    /// CTS-STATE-PROV-4 (RFC-0019.9, RFC-0016): two-phase commit
    /// participation — all prepared participants commit atomically.
    #[test]
    fn cts_prov_4_two_phase_commit_all_prepared() {
        let mut prov = make_provider().with_transactional_participant(true);
        let xid = TransactionId::from_bytes([1u8; 16]);
        let (staged, in_txn_after_commit) = {
            let txn = prov.txn.as_mut().unwrap();
            txn.begin_transaction(xid, 1_000_000).unwrap();
            assert!(txn.in_transaction());
            txn.stage_put(make_object_id(0x100), b"txn data".to_vec(), Tier::Hot)
                .unwrap();
            txn.commit_transaction(2_000_000).unwrap();
            let in_txn = txn.in_transaction();
            let staged: Vec<(ObjectId, Vec<u8>, Tier)> = txn
                .staged_mutations()
                .iter()
                .map(|(id, content)| {
                    let tier = *txn.staged_tiers().get(id).unwrap_or(&Tier::Hot);
                    (*id, content.clone(), tier)
                })
                .collect();
            (staged, in_txn)
        };
        assert!(in_txn_after_commit);
        // Apply staged mutations to the store (txn borrow ended).
        for (object_id, content, tier) in staged {
            if content.is_empty() {
                prov.delete(object_id).unwrap();
            } else {
                prov.put(object_id, &content, tier).unwrap();
            }
        }
        {
            let txn = prov.txn.as_mut().unwrap();
            txn.mark_committed();
        }
        assert!(!prov.txn.as_ref().unwrap().in_transaction());
        let got = prov.get(make_object_id(0x100)).unwrap();
        assert_eq!(got, b"txn data");
    }

    /// CTS-STATE-PROV-4 (RFC-0019.9, RFC-0016): two-phase commit abort
    /// discards all staged mutations.
    #[test]
    fn cts_prov_4_two_phase_commit_abort() {
        let mut prov = make_provider().with_transactional_participant(true);
        let txn = prov.txn.as_mut().unwrap();
        let xid = TransactionId::from_bytes([2u8; 16]);
        txn.begin_transaction(xid, 1_000_000).unwrap();
        txn.stage_put(
            make_object_id(0x200),
            b"should not apply".to_vec(),
            Tier::Hot,
        )
        .unwrap();
        txn.abort_transaction().unwrap();

        assert!(!txn.in_transaction());
        assert!(txn.staged_mutations().is_empty());
        assert_eq!(prov.get(make_object_id(0x200)), Err(StorageError::NotFound));
    }

    /// CTS-STATE-PROV-5 (RFC-0019.11, RFC-0031.44): health event on low
    /// capacity (below 10% remaining).
    #[test]
    fn cts_prov_5_health_event_low_capacity() {
        let mut prov = InMemoryProvider::new(
            "small",
            ProviderType::Block,
            100,
            test_latency(),
            0.999,
            0x01,
        );
        // Fill to 95% (95 bytes of 100).
        prov.put(make_object_id(1), &vec![0u8; 50], Tier::Hot)
            .unwrap();
        prov.put(make_object_id(2), &vec![0u8; 45], Tier::Hot)
            .unwrap();
        assert!(prov.capacity_remaining_fraction() < LOW_CAPACITY_THRESHOLD);

        // Any operation triggers the low-capacity health event.
        let _ = prov.get(make_object_id(1));
        assert!(prov.health_event_count() > 0);
        let event = prov.health_events().last().unwrap();
        assert_eq!(event.event_class, STORAGE_EVENT_CLASS);
        assert_eq!(event.event_type, storage_event_type::LOW_CAPACITY);
    }

    /// CTS-STATE-PROV-5 (RFC-0019.11, RFC-0031.44): health event when
    /// provider goes offline.
    #[test]
    fn cts_prov_5_health_event_offline() {
        let mut prov = make_provider();
        prov.set_offline(true);
        assert!(prov.offline);
        assert_eq!(prov.health_event_count(), 1);
        let event = prov.health_events().last().unwrap();
        assert_eq!(event.event_class, STORAGE_EVENT_CLASS);
        assert_eq!(event.event_type, storage_event_type::OFFLINE);

        // Operations fail while offline.
        assert_eq!(
            prov.put(make_object_id(1), b"x", Tier::Hot),
            Err(StorageError::Offline)
        );
        assert_eq!(prov.get(make_object_id(1)), Err(StorageError::Offline));
        assert_eq!(prov.delete(make_object_id(1)), Err(StorageError::Offline));
    }

    /// CTS-STATE-PROV-5 (RFC-0019.11): provider emits health events through
    /// the Storage event class (0x0500).
    #[test]
    fn cts_prov_5_health_event_storage_class() {
        assert_eq!(EventClass::Storage.id(), 0x0500);
        assert_eq!(STORAGE_EVENT_CLASS, 0x0500);
        let mut prov = make_provider();
        prov.set_offline(true);
        let event = prov.health_events().last().unwrap();
        assert_eq!(event.event_class, EventClass::Storage.id());
    }
}
