//! Replay log format reference implementation (RFC-0033).
//!
//! The replay log is an append-only, linear file of event records that enables
//! deterministic replay of platform behaviour for debugging, auditing, and
//! forensic analysis. It is built on the RFC-0031 event model and RFC-0032
//! trace spans.
//!
//! Log layout:
//!   [`replay_log_header`][`record_1`][`record_2`]...[`record_N`]
//! where each record is a 4-byte length prefix followed by a serialized
//! `PlatformEvent` and its payload bytes.

use aether_event_model::PlatformEvent;
use aether_types::DomainId;

// ---------------------------------------------------------------------------
// CRC32 (ISO 3309, reflected, polynomial 0xEDB8_8320)
// ---------------------------------------------------------------------------

const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        #[allow(clippy::cast_possible_truncation)]
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            crc = if crc & 1 == 1 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// Computes the CRC32 of `data` (ISO 3309).
#[must_use]
pub const fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    let mut i = 0;
    while i < data.len() {
        #[allow(clippy::cast_possible_truncation)]
        let index = ((crc as u8) ^ data[i]) as usize;
        crc = CRC32_TABLE[index] ^ (crc >> 8);
        i += 1;
    }
    !crc
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Magic number for the replay log header: `"REPL"` = 0x5245504C (RFC-0033.2).
pub const REPLAY_LOG_MAGIC: u32 = 0x5245_504C;

/// Format version for this RFC (RFC-0033.2).
pub const REPLAY_LOG_FORMAT_VERSION: u16 = 1;

/// Serialized size of the fixed part of a `PlatformEvent` record (64 bytes).
const PLATFORM_EVENT_FIXED_SIZE: usize = 64;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Header of a replay log file (RFC-0033.2).
///
/// The `checksum` field is a CRC32 over the entire header excluding the
/// checksum field itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayLogHeader {
    /// Magic: `0x5245504C` ("REPL").
    pub magic: u32,
    /// Format version (1 for this RFC).
    pub format_version: u16,
    /// Total events in this log file.
    pub event_count: u32,
    /// Scheduling time of the first event (RFC-0012).
    pub start_timestamp_ns: u64,
    /// Scheduling time of the last event.
    pub end_timestamp_ns: u64,
    /// Global event counter at log start.
    pub first_event_id: u64,
    /// CRC32 of the header bytes excluding this field.
    pub checksum: u64,
}

impl ReplayLogHeader {
    /// Returns the serialized size of the header (42 bytes).
    #[must_use]
    pub const fn serialized_size() -> usize {
        42
    }

    /// Serializes the header to bytes (little-endian), with the checksum
    /// field filled in over the preceding bytes.
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::serialized_size());
        buf.extend_from_slice(&self.magic.to_le_bytes());
        buf.extend_from_slice(&self.format_version.to_le_bytes());
        buf.extend_from_slice(&self.event_count.to_le_bytes());
        buf.extend_from_slice(&self.start_timestamp_ns.to_le_bytes());
        buf.extend_from_slice(&self.end_timestamp_ns.to_le_bytes());
        buf.extend_from_slice(&self.first_event_id.to_le_bytes());
        // Placeholder for checksum.
        buf.extend_from_slice(&0u64.to_le_bytes());
        let checksum = crc32(&buf[..Self::serialized_size() - 8]);
        buf[Self::serialized_size() - 8..].copy_from_slice(&u64::from(checksum).to_le_bytes());
        buf
    }

    /// Deserializes a header from bytes, validating magic, version, and
    /// checksum. Returns an error if the header is corrupt.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is shorter than [`Self::serialized_size`], because the
    /// fixed-size slices indexed below would be out of bounds.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError::HeaderTooShort`] if `buf` is shorter than the
    /// header size, [`ReplayError::BadMagic`] if the magic number is wrong,
    /// [`ReplayError::UnsupportedVersion`] if the format version is not 1, or
    /// [`ReplayError::ChecksumMismatch`] if the stored CRC32 does not match.
    pub fn deserialize(buf: &[u8]) -> Result<Self, ReplayError> {
        if buf.len() < Self::serialized_size() {
            return Err(ReplayError::HeaderTooShort);
        }
        let magic = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        if magic != REPLAY_LOG_MAGIC {
            return Err(ReplayError::BadMagic);
        }
        let format_version = u16::from_le_bytes(buf[4..6].try_into().unwrap());
        if format_version != REPLAY_LOG_FORMAT_VERSION {
            return Err(ReplayError::UnsupportedVersion(format_version));
        }
        let event_count = u32::from_le_bytes(buf[6..10].try_into().unwrap());
        let start_timestamp_ns = u64::from_le_bytes(buf[10..18].try_into().unwrap());
        let end_timestamp_ns = u64::from_le_bytes(buf[18..26].try_into().unwrap());
        let first_event_id = u64::from_le_bytes(buf[26..34].try_into().unwrap());
        let stored_checksum = u64::from_le_bytes(buf[34..42].try_into().unwrap());
        let computed = crc32(&buf[0..34]);
        if u32::try_from(stored_checksum).unwrap_or(u32::MAX) != computed {
            return Err(ReplayError::ChecksumMismatch);
        }
        Ok(Self {
            magic,
            format_version,
            event_count,
            start_timestamp_ns,
            end_timestamp_ns,
            first_event_id,
            checksum: stored_checksum,
        })
    }
}

// ---------------------------------------------------------------------------
// Serialization of PlatformEvent
// ---------------------------------------------------------------------------

/// Serializes a `PlatformEvent` to little-endian bytes (fixed part + payload).
/// Returns the fixed-size part and payload separately so callers can compute
/// `record_len`.
///
/// # Panics
///
/// Panics if `event.payload.len()` exceeds `u32::MAX`, because the payload
/// length field is a `u32`.
#[must_use]
pub fn serialize_event(event: &PlatformEvent) -> Vec<u8> {
    let mut buf = Vec::with_capacity(PLATFORM_EVENT_FIXED_SIZE + event.payload.len());
    buf.extend_from_slice(&event.event_id.to_le_bytes());
    buf.extend_from_slice(&event.event_class.to_le_bytes());
    buf.extend_from_slice(&event.event_type.to_le_bytes());
    buf.extend_from_slice(&event.timestamp_ns.to_le_bytes());
    buf.extend_from_slice(&event.emitter_thread.to_le_bytes());
    buf.extend_from_slice(&event.emitter_domain.get().to_le_bytes());
    buf.push(event.urgency);
    buf.push(event.flags);
    buf.extend_from_slice(&event.causation_id.to_le_bytes());
    buf.extend_from_slice(&event.correlation_id.to_le_bytes());
    buf.extend_from_slice(&u32::try_from(event.payload.len()).unwrap().to_le_bytes());
    buf.extend_from_slice(&event.payload);
    buf
}

/// Deserializes a `PlatformEvent` from bytes. `bytes` must contain the fixed
/// part plus exactly `payload_len` payload bytes.
fn deserialize_event(bytes: &[u8]) -> Result<PlatformEvent, ReplayError> {
    if bytes.len() < PLATFORM_EVENT_FIXED_SIZE {
        return Err(ReplayError::RecordTooShort);
    }
    let event_id = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    let event_class = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let event_type = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let timestamp_ns = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
    let emitter_thread = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
    let emitter_domain = DomainId::new(u16::from_le_bytes(bytes[40..42].try_into().unwrap()))
        .ok_or(ReplayError::InvalidDomain)?;
    let urgency = bytes[42];
    let flags = bytes[43];
    let causation_id = u64::from_le_bytes(bytes[44..52].try_into().unwrap());
    let correlation_id = u64::from_le_bytes(bytes[52..60].try_into().unwrap());
    let payload_len = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    if bytes.len() < 64 + payload_len {
        return Err(ReplayError::RecordTooShort);
    }
    let payload = bytes[64..64 + payload_len].to_vec();
    Ok(PlatformEvent {
        event_id,
        event_class,
        event_type,
        timestamp_ns,
        emitter_thread,
        emitter_domain,
        urgency,
        flags,
        causation_id,
        correlation_id,
        payload,
    })
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors encountered while reading or validating a replay log (RFC-0033.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayError {
    /// The header bytes are shorter than the expected header size.
    HeaderTooShort,
    /// The magic number is not `0x5245504C` ("REPL").
    BadMagic,
    /// The format version is not supported.
    UnsupportedVersion(u16),
    /// The header checksum does not match the stored value.
    ChecksumMismatch,
    /// A record is shorter than expected.
    RecordTooShort,
    /// The emitter domain id is invalid (zero).
    InvalidDomain,
    /// The log ended before the expected number of events were read.
    UnexpectedEndOfLog,
    /// A replay diverged from the recorded state (RFC-0033 failure semantics).
    ReplayDiverged,
    /// A gap was detected in the replayed event sequence (RFC-0033 failure semantics).
    ReplayGap,
}

// ---------------------------------------------------------------------------
// Replay log writer
// ---------------------------------------------------------------------------

/// Appends event records to a replay log in memory (RFC-0033.1).
///
/// New events are appended to the log in chronological order. The header is
/// written once when the log is created and updated on close.
pub struct ReplayLogWriter {
    /// Records appended so far, in order.
    records: Vec<Vec<u8>>,
    /// Start timestamp of the log.
    start_timestamp_ns: u64,
    /// End timestamp of the log (updated on each append).
    end_timestamp_ns: u64,
    /// First event id at log start.
    first_event_id: u64,
}

impl ReplayLogWriter {
    /// Creates a new replay log writer starting at `first_event_id`.
    #[must_use]
    pub fn new(first_event_id: u64, start_timestamp_ns: u64) -> Self {
        Self {
            records: Vec::new(),
            start_timestamp_ns,
            end_timestamp_ns: start_timestamp_ns,
            first_event_id,
        }
    }

    /// Appends a `PlatformEvent` to the log (RFC-0033.1).
    pub fn append(&mut self, event: &PlatformEvent) {
        let serialized = serialize_event(event);
        self.records.push(serialized);
        self.end_timestamp_ns = event.timestamp_ns;
    }

    /// Returns the number of records appended.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Finalizes the log into a byte vector: header followed by records.
    /// Each record is prefixed by a 4-byte little-endian `record_len`
    /// (RFC-0033.1).
    ///
    /// # Panics
    ///
    /// Panics if `self.records.len()` or any record length exceeds `u32::MAX`,
    /// because those fields are `u32` in the log format.
    #[must_use]
    pub fn finalize(self) -> Vec<u8> {
        let header = ReplayLogHeader {
            magic: REPLAY_LOG_MAGIC,
            format_version: REPLAY_LOG_FORMAT_VERSION,
            event_count: u32::try_from(self.records.len()).expect("log too large for u32"),
            start_timestamp_ns: self.start_timestamp_ns,
            end_timestamp_ns: self.end_timestamp_ns,
            first_event_id: self.first_event_id,
            checksum: 0, // filled in by serialize()
        };
        let mut buf = header.serialize();
        for record in &self.records {
            let record_len: u32 = u32::try_from(record.len()).expect("record too large for u32");
            buf.extend_from_slice(&record_len.to_le_bytes());
            buf.extend_from_slice(record);
        }
        buf
    }
}

// ---------------------------------------------------------------------------
// Replay log reader
// ---------------------------------------------------------------------------

/// Reads and validates a replay log (RFC-0033.1).
pub struct ReplayLogReader {
    /// Validated header.
    pub header: ReplayLogHeader,
    /// Deserialized events, in order.
    events: Vec<PlatformEvent>,
}

impl ReplayLogReader {
    /// Reads and validates a replay log from `buf`.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is shorter than the replay log header size, because the
    /// fixed-size header slice indexed below would be out of bounds.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError::HeaderTooShort`] if `buf` is shorter than the
    /// header size, [`ReplayError::UnexpectedEndOfLog`] if the log ends
    /// before all declared records are read, or a [`ReplayError`] variant from
    /// [`ReplayLogHeader::deserialize`] if the header is corrupt.
    pub fn read(buf: &[u8]) -> Result<Self, ReplayError> {
        if buf.len() < ReplayLogHeader::serialized_size() {
            return Err(ReplayError::HeaderTooShort);
        }
        let header = ReplayLogHeader::deserialize(&buf[..ReplayLogHeader::serialized_size()])?;
        let mut offset = ReplayLogHeader::serialized_size();
        #[allow(clippy::cast_possible_truncation)]
        let mut events = Vec::with_capacity(header.event_count as usize);

        for _ in 0..header.event_count {
            if offset + 4 > buf.len() {
                return Err(ReplayError::UnexpectedEndOfLog);
            }
            let record_len: usize =
                u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            if offset + record_len > buf.len() {
                return Err(ReplayError::UnexpectedEndOfLog);
            }
            let event = deserialize_event(&buf[offset..offset + record_len])?;
            events.push(event);
            offset += record_len;
        }

        Ok(Self { header, events })
    }

    /// Returns all events in chronological order.
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        &self.events
    }

    /// Returns the number of events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

// ---------------------------------------------------------------------------
// Deterministic replay engine (RFC-0033.3, RFC-0033.5)
// ---------------------------------------------------------------------------

/// Result of stepping the replay engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayStep {
    /// An event was replayed successfully.
    Replayed,
    /// The replay reached the requested pause point (`replay_to`).
    Paused,
    /// All events have been replayed.
    Completed,
}

/// A deterministic replay engine that replays recorded events against an
/// initial platform state (RFC-0033.3).
///
/// Non-deterministic inputs (hardware interrupts, timers, async I/O
/// completions) are represented as explicit events in the log; the replay
/// substitutes synthetic events for actual hardware events (RFC-0033.4).
pub struct ReplayEngine<S: ReplayableState> {
    /// Events to replay, in order.
    events: Vec<PlatformEvent>,
    /// Current index into `events`.
    index: usize,
    /// The platform state being evolved by replay.
    state: S,
    /// Whether replay has been paused (reached `replay_to` target).
    paused: bool,
    /// Whether replay is complete.
    completed: bool,
    /// Whether non-deterministic inputs are suppressed (RFC-0033.4).
    hardware_inputs_suppressed: bool,
}

impl<S: ReplayableState + Default> ReplayEngine<S> {
    /// Creates a replay engine over `events` starting from default state.
    #[must_use]
    pub fn new(events: Vec<PlatformEvent>) -> Self {
        Self {
            events,
            index: 0,
            state: S::default(),
            paused: false,
            completed: false,
            hardware_inputs_suppressed: false,
        }
    }

    /// Creates a replay engine over `events` starting from a given state.
    #[must_use]
    pub fn with_state(events: Vec<PlatformEvent>, state: S) -> Self {
        Self {
            events,
            index: 0,
            state,
            paused: false,
            completed: false,
            hardware_inputs_suppressed: false,
        }
    }

    /// Returns whether replay is complete.
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        self.completed
    }

    /// Returns whether replay is paused.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    /// Returns the current platform state.
    #[must_use]
    pub const fn state(&self) -> &S {
        &self.state
    }

    /// Returns the index of the next event to replay.
    #[must_use]
    pub const fn next_index(&self) -> usize {
        self.index
    }

    /// Suppresses or enables actual hardware input events during replay
    /// (RFC-0033.4). When suppressed, non-deterministic hardware events are
    /// replaced by explicit synthetic events from the log.
    pub fn set_hardware_inputs_suppressed(&mut self, suppressed: bool) {
        self.hardware_inputs_suppressed = suppressed;
    }

    /// Returns whether non-deterministic hardware inputs are suppressed
    /// during replay (RFC-0033.4).
    #[must_use]
    pub const fn is_hardware_inputs_suppressed(&self) -> bool {
        self.hardware_inputs_suppressed
    }

    /// Replays events up to and including `target_event_id`, then pauses
    /// (RFC-0033.5). Returns `ReplayStep::Paused` when the target is reached,
    /// `ReplayStep::Completed` when the log is exhausted, or `ReplayStep::Replayed`
    /// if the target is beyond the log.
    pub fn replay_to(&mut self, target_event_id: u64) -> ReplayStep {
        while self.index < self.events.len() {
            let event = self.events[self.index].clone();
            self.state.apply_event(&event);
            self.index += 1;
            if event.event_id == target_event_id {
                self.paused = true;
                return ReplayStep::Paused;
            }
        }
        self.completed = true;
        ReplayStep::Completed
    }

    /// Steps forward by one event. Returns `ReplayStep::Replayed` if an event
    /// was applied, `ReplayStep::Completed` if the log is exhausted, or
    /// `ReplayStep::Paused` if previously paused.
    pub fn step(&mut self) -> ReplayStep {
        if self.paused || self.completed {
            return if self.completed {
                ReplayStep::Completed
            } else {
                ReplayStep::Paused
            };
        }
        if self.index >= self.events.len() {
            self.completed = true;
            return ReplayStep::Completed;
        }
        let event = self.events[self.index].clone();
        self.state.apply_event(&event);
        self.index += 1;
        ReplayStep::Replayed
    }

    /// Applies a single event to the platform state. By default, events are
    /// recorded but do not mutate state; concrete states override
    /// `ReplayableState::apply_event` to evolve the platform state
    /// deterministically (RFC-0033.3).
    #[allow(dead_code)]
    fn apply_event(&mut self, event: &PlatformEvent) {
        self.state.apply_event(event);
    }
}

/// A state that can be evolved by replaying events (RFC-0033.3).
///
/// The default implementation does not mutate state; concrete states override
/// `apply_event` to evolve deterministically as events are replayed.
pub trait ReplayableState: Clone + Default {
    /// Applies an event to this state, evolving it deterministically.
    fn apply_event(&mut self, _event: &PlatformEvent) {}
}

/// A simple replayable state: a counter of how many events have been applied.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReplayState {
    /// Number of events applied.
    pub events_applied: u64,
    /// The id of the last applied event.
    pub last_event_id: u64,
}

impl ReplayableState for ReplayState {
    fn apply_event(&mut self, event: &PlatformEvent) {
        self.events_applied += 1;
        self.last_event_id = event.event_id;
    }
}

// ---------------------------------------------------------------------------
// Checkpoint support (RFC-0033.6, RFC-0033.7)
// ---------------------------------------------------------------------------

/// A state checkpoint embedded in the replay log (RFC-0033.6).
///
/// Checkpoints are taken at consistent points where all transactions have
/// committed and no partial operations are in flight (RFC-0033.7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    /// Scheduling time at which the checkpoint was taken.
    pub timestamp_ns: u64,
    /// Compressed state dump (opaque bytes).
    pub state_dump: Vec<u8>,
    /// Whether the checkpoint is consistent (all transactions committed).
    pub consistent: bool,
}

impl Checkpoint {
    /// Creates a consistent checkpoint at `timestamp_ns` with `state_dump`.
    #[must_use]
    pub fn consistent(timestamp_ns: u64, state_dump: Vec<u8>) -> Self {
        Self {
            timestamp_ns,
            state_dump,
            consistent: true,
        }
    }

    /// Returns whether this checkpoint is consistent (RFC-0033.7).
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        self.consistent
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use aether_event_model::{EventClass, EventFlag, SECURITY_CRITICAL_URGENCY};

    fn make_event(event_id: u64, timestamp_ns: u64) -> PlatformEvent {
        PlatformEvent {
            event_id,
            event_class: EventClass::State.id(),
            event_type: 1,
            timestamp_ns,
            emitter_thread: 1,
            emitter_domain: DomainId::new(7).unwrap(),
            urgency: 100,
            flags: EventFlag::Persistent as u8,
            causation_id: 0,
            correlation_id: 0,
            payload: b"replay event payload".to_vec(),
        }
    }

    #[test]
    fn crc32_known_vector() {
        // CRC32 of "123456789" is 0xCBF43926 (ISO 3309).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn header_serialize_deserialize_round_trip() {
        let header = ReplayLogHeader {
            magic: REPLAY_LOG_MAGIC,
            format_version: REPLAY_LOG_FORMAT_VERSION,
            event_count: 7,
            start_timestamp_ns: 1_000_000,
            end_timestamp_ns: 5_000_000,
            first_event_id: 1,
            checksum: 0,
        };
        let bytes = header.serialize();
        assert_eq!(bytes.len(), ReplayLogHeader::serialized_size());
        let round_tripped = ReplayLogHeader::deserialize(&bytes).unwrap();
        assert_eq!(round_tripped.magic, REPLAY_LOG_MAGIC);
        assert_eq!(round_tripped.format_version, REPLAY_LOG_FORMAT_VERSION);
        assert_eq!(round_tripped.event_count, 7);
        assert_eq!(round_tripped.start_timestamp_ns, 1_000_000);
        assert_eq!(round_tripped.end_timestamp_ns, 5_000_000);
        assert_eq!(round_tripped.first_event_id, 1);
        assert_eq!(round_tripped.checksum as u32, crc32(&bytes[0..34]));
    }

    #[test]
    fn header_bad_magic_rejected() {
        let bytes = ReplayLogHeader {
            magic: 0xDEAD_BEEF,
            format_version: REPLAY_LOG_FORMAT_VERSION,
            event_count: 0,
            start_timestamp_ns: 0,
            end_timestamp_ns: 0,
            first_event_id: 0,
            checksum: 0,
        }
        .serialize();
        assert!(ReplayLogHeader::deserialize(&bytes).is_err());
    }

    #[test]
    fn header_checksum_mismatch_rejected() {
        let mut bytes = ReplayLogHeader {
            magic: REPLAY_LOG_MAGIC,
            format_version: REPLAY_LOG_FORMAT_VERSION,
            event_count: 0,
            start_timestamp_ns: 0,
            end_timestamp_ns: 0,
            first_event_id: 0,
            checksum: 0,
        }
        .serialize();
        // Corrupt a non-checksum byte.
        bytes[10] ^= 0xFF;
        assert!(ReplayLogHeader::deserialize(&bytes).is_err());
    }

    #[test]
    fn event_serialize_deserialize_round_trip() {
        let event = make_event(42, 1_000_000);
        let serialized = serialize_event(&event);
        let round_tripped = deserialize_event(&serialized).unwrap();
        assert_eq!(round_tripped, event);
        assert_eq!(
            serialized.len(),
            PLATFORM_EVENT_FIXED_SIZE + event.payload.len()
        );
    }

    #[test]
    fn writer_reader_round_trip() {
        let mut writer = ReplayLogWriter::new(1, 1_000_000);
        for i in 1..=5 {
            writer.append(&make_event(i, 1_000_000 + i * 100_000));
        }
        let log = writer.finalize();
        let reader = ReplayLogReader::read(&log).unwrap();
        assert_eq!(reader.event_count(), 5);
        assert_eq!(reader.header.event_count, 5);
        assert_eq!(reader.header.start_timestamp_ns, 1_000_000);
        assert_eq!(reader.header.end_timestamp_ns, 1_500_000);
        for (i, event) in reader.events().iter().enumerate() {
            assert_eq!(event.event_id, (i + 1) as u64);
        }
    }

    #[test]
    fn replay_to_pauses_at_target() {
        let mut writer = ReplayLogWriter::new(1, 1_000_000);
        for i in 1..=5 {
            writer.append(&make_event(i, 1_000_000 + i * 100_000));
        }
        let reader = ReplayLogReader::read(&writer.finalize()).unwrap();
        let mut engine: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
        let result = engine.replay_to(3);
        assert_eq!(result, ReplayStep::Paused);
        assert!(engine.is_paused());
        assert_eq!(engine.state().events_applied, 3);
        assert_eq!(engine.state().last_event_id, 3);
        assert_eq!(engine.next_index(), 3);
    }

    #[test]
    fn replay_completes_when_target_beyond_log() {
        let mut writer = ReplayLogWriter::new(1, 1_000_000);
        for i in 1..=3 {
            writer.append(&make_event(i, 1_000_000 + i * 100_000));
        }
        let reader = ReplayLogReader::read(&writer.finalize()).unwrap();
        let mut engine: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
        let result = engine.replay_to(99);
        assert_eq!(result, ReplayStep::Completed);
        assert!(engine.is_completed());
        assert_eq!(engine.state().events_applied, 3);
    }

    #[test]
    fn deterministic_replay_reproduces_state() {
        let mut writer = ReplayLogWriter::new(1, 1_000_000);
        for i in 1..=10 {
            writer.append(&make_event(i, 1_000_000 + i * 100_000));
        }
        let reader = ReplayLogReader::read(&writer.finalize()).unwrap();

        // Two independent replays of the same log produce identical state.
        let mut engine1: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
        let mut engine2: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
        while !engine1.is_completed() {
            engine1.step();
        }
        while !engine2.is_completed() {
            engine2.step();
        }
        assert_eq!(engine1.state(), engine2.state());
        assert_eq!(engine1.state().events_applied, 10);
    }

    #[test]
    fn hardware_inputs_suppressed_during_replay() {
        let mut writer = ReplayLogWriter::new(1, 1_000_000);
        writer.append(&make_event(1, 1_000_000));
        let reader = ReplayLogReader::read(&writer.finalize()).unwrap();
        let mut engine: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
        engine.set_hardware_inputs_suppressed(true);
        assert!(engine.hardware_inputs_suppressed);
        engine.step();
        assert_eq!(engine.state().events_applied, 1);
    }

    #[test]
    fn checkpoint_consistency() {
        let dump = b"compressed state snapshot".to_vec();
        let checkpoint = Checkpoint::consistent(1_000_000, dump.clone());
        assert!(checkpoint.is_consistent());
        assert_eq!(checkpoint.timestamp_ns, 1_000_000);
        assert_eq!(checkpoint.state_dump, dump);
    }

    #[test]
    fn inconsistent_checkpoint_not_allowed() {
        let dump = b"inconsistent state".to_vec();
        let mut checkpoint = Checkpoint::consistent(1_000_000, dump);
        checkpoint.consistent = false;
        assert!(!checkpoint.is_consistent());
    }
}
