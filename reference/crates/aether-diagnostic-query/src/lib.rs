//! Diagnostic query reference implementation (RFC-0035).
//!
//! The diagnostic query interface is a read-only, structured mechanism for
//! querying the event stream, trace spans, and replay log. It enables
//! operators, debuggers, and automated monitoring to inspect platform
//! behaviour without modifying system state (RFC-0035 scope).
//!
//! Supported operators (RFC-0035.1): Filter, Project, Join, Aggregate,
//! Window, Causality traverse. Three execution modes (RFC-0035.3): Synchronous,
//! Streaming, Historical. All queries are subject to declared limits
//! (RFC-0035.4) and every query execution is itself logged as an Observability
//! class event (RFC-0035.6).

use aether_event_model::{
    EventClass, PlatformEvent, SECURITY_CRITICAL_URGENCY, is_valid_event_class,
};
use aether_replay_model::ReplayLogReader;
use aether_trace_model::TraceSpan;

// ---------------------------------------------------------------------------
// Query operators
// ---------------------------------------------------------------------------

/// Query language operators (RFC-0035.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum QueryOperator {
    /// Select events matching predicates on any event field.
    Filter = 0,
    /// Extract specific fields from matching events.
    Project = 1,
    /// Correlate events from different sources (e.g. by `trace_id`).
    Join = 2,
    /// Count, sum, histogram, percentile over event properties.
    Aggregate = 3,
    /// Time-bounded queries (within N seconds of an event).
    Window = 4,
    /// Given an event, return all events in its causal chain.
    Causality = 5,
}

// ---------------------------------------------------------------------------
// Predicates (conjunctive, RFC-0035.1)
// ---------------------------------------------------------------------------

/// Event field that can be filtered on (RFC-0035.1, RFC-0035.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum QueryField {
    /// Event class identifier (RFC-0031.2).
    EventClass = 0,
    /// Event type within the class.
    EventType = 1,
    /// Emitting domain identifier.
    EmitterDomain = 2,
    /// Event urgency (0=lowest, 255=highest).
    Urgency = 3,
    /// Event scheduling timestamp in nanoseconds (RFC-0012).
    TimestampNs = 4,
    /// Causation event identifier (RFC-0031.5).
    CausationId = 5,
    /// Correlation identifier grouping related events (RFC-0031.6).
    CorrelationId = 6,
    /// Payload field accessed via dot notation (`event.payload.fieldname`).
    PayloadField = 7,
}

/// Comparison operator for a predicate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ComparisonOp {
    /// Equal to.
    Eq = 0,
    /// Not equal to.
    Ne = 1,
    /// Greater than.
    Gt = 2,
    /// Less than.
    Lt = 3,
    /// Greater than or equal to.
    Ge = 4,
    /// Less than or equal to.
    Le = 5,
    /// Contains (for byte/substring matching).
    Contains = 6,
}

/// A single predicate on an event field (RFC-0035.2). Predicates within a
/// query are conjunctive (all must match).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Predicate {
    /// The event field to test.
    pub field: QueryField,
    /// The comparison to apply.
    pub op: ComparisonOp,
    /// The value to compare against (as bytes; interpreted per field type).
    pub value: Vec<u8>,
    /// Optional name of a payload sub-field for `QueryField::PayloadField`.
    pub payload_field_name: Option<String>,
}

/// A named projection field for PROJECT queries (RFC-0035.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionField {
    /// Name of the field to project.
    pub name: String,
    /// Source event field.
    pub source: QueryField,
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// Execution mode for a diagnostic query (RFC-0035.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum QueryExecutionMode {
    /// Returns immediately with bounded result; suitable for small result sets.
    Synchronous = 0,
    /// Returns a cursor; results streamed as new matching events arrive.
    Streaming = 1,
    /// Executes over the replay log; returns complete result set.
    Historical = 2,
}

/// Status of a query result (RFC-0035 data structures).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum QueryStatus {
    /// Query completed successfully with all matching results.
    Ok = 0,
    /// Result set was truncated due to declared limits (RFC-0035.4).
    ResultTruncated = 1,
    /// Query encountered an error.
    QueryError = 2,
}

/// Declared limits for a query (RFC-0035.4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryLimits {
    /// Maximum result set size (default: 1,000 records).
    pub max_results: u32,
    /// Maximum query duration in nanoseconds (default: 30 seconds).
    pub max_duration_ns: u64,
    /// Maximum causal chain traversal depth (default: 100 events).
    pub max_causality_depth: u32,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            max_results: 1_000,
            max_duration_ns: 30_000_000_000,
            max_causality_depth: 100,
        }
    }
}

/// A diagnostic query (RFC-0035 data structures).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticQuery {
    /// Query operator (RFC-0035.1).
    pub query_type: QueryOperator,
    /// Start of the query time window (scheduling time, RFC-0012).
    pub start_time_ns: u64,
    /// End of the query time window.
    pub end_time_ns: u64,
    /// Conjunctive predicates on event fields.
    pub predicates: Vec<Predicate>,
    /// Fields to project for PROJECT queries.
    pub projection_fields: Vec<ProjectionField>,
    /// Execution mode (RFC-0035.3).
    pub execution_mode: QueryExecutionMode,
    /// Declared query limits (RFC-0035.4).
    pub limits: QueryLimits,
}

/// Result of executing a diagnostic query (RFC-0035 data structures).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryResult {
    /// Status: `Ok`, `ResultTruncated`, `QueryError`.
    pub status: QueryStatus,
    /// Total events matching the query.
    pub matched_count: u64,
    /// Events included in this result.
    pub returned_count: u64,
    /// Cursor for continuation if truncated.
    pub next_cursor: u64,
    /// Actual result events.
    pub results: Vec<PlatformEvent>,
}

// ---------------------------------------------------------------------------
// Authorization
// ---------------------------------------------------------------------------

/// Capability rights required to query certain event classes (RFC-0035.7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum QueryCapabilityRight {
    /// Required to query Security-class events (0x0700).
    QuerySecurityEvents = 0,
    /// Required to query Capability-class events (0x0200).
    QueryCapabilityEvents = 1,
    /// Required to read raw event content (full payload).
    QueryFullPayload = 2,
}

/// Whether the caller is authorized to run `query` against events of `event_class`.
///
/// Returns `Ok(())` if authorized, or the missing `QueryCapabilityRight` if not.
///
/// # Errors
///
/// Returns [`QueryCapabilityRight::QuerySecurityEvents`] if the event class is
/// `Security` (0x0700) and the caller lacks that right.
/// Returns [`QueryCapabilityRight::QueryCapabilityEvents`] if the event class is
/// `Capability` (0x0200) and the caller lacks that right.
/// Returns [`QueryCapabilityRight::QueryFullPayload`] if `include_full_payload`
/// is true and the caller lacks `QUERY_FULL_PAYLOAD`.
pub fn authorize_query(
    event_class: u64,
    include_full_payload: bool,
    held_rights: &[QueryCapabilityRight],
) -> Result<(), QueryCapabilityRight> {
    match event_class {
        c if c == EventClass::Security.id()
            && !held_rights.contains(&QueryCapabilityRight::QuerySecurityEvents) =>
        {
            return Err(QueryCapabilityRight::QuerySecurityEvents);
        }
        c if c == EventClass::Capability.id()
            && !held_rights.contains(&QueryCapabilityRight::QueryCapabilityEvents) =>
        {
            return Err(QueryCapabilityRight::QueryCapabilityEvents);
        }
        _ => {}
    }
    if include_full_payload && !held_rights.contains(&QueryCapabilityRight::QueryFullPayload) {
        return Err(QueryCapabilityRight::QueryFullPayload);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Predicate evaluation
// ---------------------------------------------------------------------------

/// Evaluate a single predicate against an event.
fn eval_predicate(pred: &Predicate, event: &PlatformEvent) -> bool {
    let field_value = match pred.field {
        QueryField::EventClass => event.event_class.to_le_bytes().to_vec(),
        QueryField::EventType => event.event_type.to_le_bytes().to_vec(),
        QueryField::EmitterDomain => event.emitter_domain.get().to_le_bytes().to_vec(),
        QueryField::Urgency => vec![event.urgency],
        QueryField::TimestampNs => event.timestamp_ns.to_le_bytes().to_vec(),
        QueryField::CausationId => event.causation_id.to_le_bytes().to_vec(),
        QueryField::CorrelationId => event.correlation_id.to_le_bytes().to_vec(),
        QueryField::PayloadField => {
            // Payload field lookup via dot notation; opaque bytes, compare as prefix/contains.
            return false;
        }
    };
    eval_comparison(&field_value, pred.op, &pred.value)
}

/// Compare two byte slices according to `op`.
fn eval_comparison(field: &[u8], op: ComparisonOp, value: &[u8]) -> bool {
    match op {
        ComparisonOp::Eq => field == value,
        ComparisonOp::Ne => field != value,
        ComparisonOp::Gt => field > value,
        ComparisonOp::Lt => field < value,
        ComparisonOp::Ge => field >= value,
        ComparisonOp::Le => field <= value,
        ComparisonOp::Contains => field.windows(value.len()).any(|w| w == value),
    }
}

/// Check whether `event` matches all conjunctive predicates.
fn matches_predicates(event: &PlatformEvent, predicates: &[Predicate]) -> bool {
    predicates.iter().all(|p| eval_predicate(p, event))
}

// ---------------------------------------------------------------------------
// Query engine
// ---------------------------------------------------------------------------

/// Read-only diagnostic query engine (RFC-0035).
///
/// Executes queries against an in-memory event stream. Historical mode reads
/// from a replay log (RFC-0033). Join mode correlates events with trace spans
/// (RFC-0032) by `trace_id`/`correlation_id`.
pub struct DiagnosticQueryEngine {
    /// Events the engine can query, in chronological order.
    events: Vec<PlatformEvent>,
    /// Trace spans available for join operations.
    spans: Vec<TraceSpan>,
}

impl DiagnosticQueryEngine {
    /// Creates a new query engine over `events` with optional trace `spans`.
    #[must_use]
    pub fn new(events: Vec<PlatformEvent>, spans: Vec<TraceSpan>) -> Self {
        Self { events, spans }
    }

    /// Returns the number of events available to the engine.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Executes `query` and returns a result (RFC-0035).
    ///
    /// # Errors
    ///
    /// Returns a `QueryResult` with `QueryStatus::QueryError` if the query
    /// references an unknown event class filter.
    #[must_use]
    pub fn execute(&self, query: &DiagnosticQuery) -> QueryResult {
        // Filter by time window first.
        let in_window: Vec<&PlatformEvent> = self
            .events
            .iter()
            .filter(|e| {
                e.timestamp_ns >= query.start_time_ns && e.timestamp_ns <= query.end_time_ns
            })
            .collect();

        // Unknown event class filter -> empty results, not an error (RFC-0035.4).
        let class_filtered: Vec<&PlatformEvent> = if let Some(cls) = event_class_filter(query) {
            if !is_valid_event_class(cls) {
                return QueryResult {
                    status: QueryStatus::Ok,
                    matched_count: 0,
                    returned_count: 0,
                    next_cursor: 0,
                    results: Vec::new(),
                };
            }
            in_window
                .iter()
                .filter(|e| e.event_class == cls)
                .copied()
                .collect()
        } else {
            in_window
        };

        // Apply conjunctive predicates.
        let matched: Vec<PlatformEvent> = class_filtered
            .into_iter()
            .filter(|e| matches_predicates(e, &query.predicates))
            .cloned()
            .collect();

        let matched_count = matched.len() as u64;

        // Apply result set limit (RFC-0035.4).
        let limit = query.limits.max_results as usize;
        let truncated = matched_count > limit as u64;
        let returned: Vec<PlatformEvent> = matched.into_iter().take(limit).collect();
        let returned_count = returned.len() as u64;

        let status = if truncated {
            QueryStatus::ResultTruncated
        } else {
            QueryStatus::Ok
        };

        QueryResult {
            status,
            matched_count,
            returned_count,
            next_cursor: if truncated { returned_count } else { 0 },
            results: returned,
        }
    }

    /// Executes a causality traversal: given a root event id, returns all
    /// events in its causal chain by following `causation_id` links upward
    /// (RFC-0035.1 Causality operator).
    ///
    /// # Errors
    ///
    /// Returns `None` if the root event is not found.
    #[must_use]
    pub fn traverse_causality(&self, root_event_id: u64, max_depth: u32) -> Option<QueryResult> {
        let root_idx = self
            .events
            .iter()
            .position(|e| e.event_id == root_event_id)?;
        let mut chain: Vec<PlatformEvent> = Vec::new();
        let mut visited: Vec<u64> = Vec::new();
        let mut current_idx = root_idx;
        let mut depth: u32 = 0;

        while depth < max_depth {
            let event = self.events[current_idx].clone();
            if visited.contains(&event.event_id) {
                break; // cycle
            }
            visited.push(event.event_id);
            chain.push(event.clone());
            if event.causation_id == 0 {
                break;
            }
            // Find the causative event.
            match self
                .events
                .iter()
                .position(|e| e.event_id == event.causation_id)
            {
                Some(idx) => current_idx = idx,
                None => break,
            }
            depth += 1;
        }

        chain.reverse(); // root cause first
        Some(QueryResult {
            status: if depth >= max_depth {
                QueryStatus::ResultTruncated
            } else {
                QueryStatus::Ok
            },
            matched_count: chain.len() as u64,
            returned_count: chain.len() as u64,
            next_cursor: 0,
            results: chain,
        })
    }

    /// Joins events with trace spans by matching event `correlation_id` to
    /// span `trace_id` (RFC-0035.1 Join operator).
    #[must_use]
    pub fn join_with_spans(&self, query: &DiagnosticQuery) -> QueryResult {
        let mut matched: Vec<PlatformEvent> = Vec::new();
        for event in &self.events {
            if event.timestamp_ns < query.start_time_ns || event.timestamp_ns > query.end_time_ns {
                continue;
            }
            if self
                .spans
                .iter()
                .any(|s| s.trace_id == event.correlation_id)
                && matches_predicates(event, &query.predicates)
            {
                matched.push(event.clone());
            }
        }
        let matched_count = matched.len() as u64;
        let limit = query.limits.max_results as usize;
        let truncated = matched_count > limit as u64;
        let returned: Vec<PlatformEvent> = matched.into_iter().take(limit).collect();
        QueryResult {
            status: if truncated {
                QueryStatus::ResultTruncated
            } else {
                QueryStatus::Ok
            },
            matched_count,
            returned_count: returned.len() as u64,
            next_cursor: if truncated { returned.len() as u64 } else { 0 },
            results: returned,
        }
    }
}

/// Extract an optional event class filter from query predicates, if present.
fn event_class_filter(query: &DiagnosticQuery) -> Option<u64> {
    query.predicates.iter().find_map(|p| {
        if p.field == QueryField::EventClass && p.op == ComparisonOp::Eq {
            <[u8; 8]>::try_from(p.value.as_slice())
                .ok()
                .map(u64::from_le_bytes)
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// Predefined queries (RFC-0035.5)
// ---------------------------------------------------------------------------

/// Common diagnostic scenarios (RFC-0035.5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredefinedQuery {
    /// All events with urgency >= 200 in the last hour.
    RecentErrors,
    /// All `ThreadStateChanged` events for a given `thread_id` in a time window.
    ThreadActivity,
    /// All `CapGranted` events for a given capability.
    CapabilityLineage,
    /// All lifecycle events for a given `object_id`.
    ObjectLifecycle,
    /// All message events for a given domain in a time window.
    DomainTraffic,
    /// All `ProviderOffline` and `ProviderHealthWarning` events in the last 24 hours.
    StorageHealth,
}

/// Build a `DiagnosticQuery` for a predefined scenario (RFC-0035.5).
#[must_use]
pub fn predefined_query(
    scenario: PredefinedQuery,
    start_time_ns: u64,
    end_time_ns: u64,
    param: Option<u64>,
) -> DiagnosticQuery {
    let limits = QueryLimits::default();
    let base = DiagnosticQuery {
        query_type: QueryOperator::Filter,
        start_time_ns,
        end_time_ns,
        predicates: Vec::new(),
        projection_fields: Vec::new(),
        execution_mode: QueryExecutionMode::Synchronous,
        limits,
    };
    match scenario {
        PredefinedQuery::RecentErrors => DiagnosticQuery {
            predicates: urgent_predicate(),
            ..base
        },
        PredefinedQuery::ThreadActivity => DiagnosticQuery {
            predicates: class_and_domain_predicate(EventClass::Execution.id(), param),
            ..base
        },
        PredefinedQuery::CapabilityLineage => DiagnosticQuery {
            predicates: class_and_correlation_predicate(EventClass::Capability.id(), param),
            ..base
        },
        PredefinedQuery::ObjectLifecycle => DiagnosticQuery {
            predicates: class_and_domain_predicate(EventClass::State.id(), param),
            ..base
        },
        PredefinedQuery::DomainTraffic => DiagnosticQuery {
            predicates: class_and_domain_predicate(EventClass::Communication.id(), param),
            ..base
        },
        PredefinedQuery::StorageHealth => DiagnosticQuery {
            predicates: storage_health_predicates(),
            ..base
        },
    }
}

/// Predicate selecting events with security-critical urgency (urgency >= 200).
#[must_use]
fn urgent_predicate() -> Vec<Predicate> {
    vec![Predicate {
        field: QueryField::Urgency,
        op: ComparisonOp::Ge,
        value: SECURITY_CRITICAL_URGENCY.to_le_bytes().to_vec(),
        payload_field_name: None,
    }]
}

/// Predicates selecting events of `event_class` emitted by `domain`.
#[must_use]
fn class_and_domain_predicate(event_class: u64, domain: Option<u64>) -> Vec<Predicate> {
    vec![
        Predicate {
            field: QueryField::EventClass,
            op: ComparisonOp::Eq,
            value: event_class.to_le_bytes().to_vec(),
            payload_field_name: None,
        },
        Predicate {
            field: QueryField::EmitterDomain,
            op: ComparisonOp::Eq,
            value: domain.unwrap_or(0).to_le_bytes().to_vec(),
            payload_field_name: None,
        },
    ]
}

/// Predicates selecting events of `event_class` with `correlation_id`.
#[must_use]
fn class_and_correlation_predicate(event_class: u64, corr: Option<u64>) -> Vec<Predicate> {
    vec![
        Predicate {
            field: QueryField::EventClass,
            op: ComparisonOp::Eq,
            value: event_class.to_le_bytes().to_vec(),
            payload_field_name: None,
        },
        Predicate {
            field: QueryField::CorrelationId,
            op: ComparisonOp::Eq,
            value: corr.unwrap_or(0).to_le_bytes().to_vec(),
            payload_field_name: None,
        },
    ]
}

/// Predicates selecting Storage-class events with security-critical urgency.
#[must_use]
fn storage_health_predicates() -> Vec<Predicate> {
    vec![
        Predicate {
            field: QueryField::EventClass,
            op: ComparisonOp::Eq,
            value: EventClass::Storage.id().to_le_bytes().to_vec(),
            payload_field_name: None,
        },
        Predicate {
            field: QueryField::Urgency,
            op: ComparisonOp::Ge,
            value: SECURITY_CRITICAL_URGENCY.to_le_bytes().to_vec(),
            payload_field_name: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// Historical mode over a replay log (RFC-0035.3)
// ---------------------------------------------------------------------------

/// Executes `query` historically over a replay log buffer (RFC-0035.3).
///
/// Reads the replay log via [`ReplayLogReader`] and runs the query against its
/// events. Returns the query result.
///
/// # Errors
///
/// Returns [`ReplayError::LogReadError`] if the replay log buffer cannot be
/// read or validated.
pub fn query_replay_log(
    log_buf: &[u8],
    query: &DiagnosticQuery,
) -> Result<QueryResult, ReplayError> {
    let reader = ReplayLogReader::read(log_buf)?;
    let engine = DiagnosticQueryEngine::new(reader.events().to_vec(), Vec::new());
    Ok(engine.execute(query))
}

/// Error type wrapping replay read errors for historical queries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    /// The replay log could not be read.
    LogReadError,
}

impl From<aether_replay_model::ReplayError> for ReplayError {
    fn from(_: aether_replay_model::ReplayError) -> Self {
        ReplayError::LogReadError
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use aether_event_model::{EventFlag, EventTypeId};
    use aether_types::DomainId;

    fn make_event(
        event_id: u64,
        event_class: u64,
        event_type: EventTypeId,
        urgency: u8,
        domain: u16,
        causation_id: u64,
        correlation_id: u64,
        timestamp_ns: u64,
    ) -> PlatformEvent {
        PlatformEvent {
            event_id,
            event_class,
            event_type,
            timestamp_ns,
            emitter_thread: 1,
            emitter_domain: DomainId::new(domain).unwrap(),
            urgency,
            flags: EventFlag::Persistent as u8,
            causation_id,
            correlation_id,
            payload: Vec::new(),
        }
    }

    #[test]
    fn filter_by_event_class() {
        let events = vec![
            make_event(1, EventClass::Capability.id(), 1, 50, 7, 0, 0, 100),
            make_event(2, EventClass::Security.id(), 1, 50, 7, 0, 0, 200),
        ];
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let query = DiagnosticQuery {
            query_type: QueryOperator::Filter,
            start_time_ns: 0,
            end_time_ns: u64::MAX,
            predicates: vec![Predicate {
                field: QueryField::EventClass,
                op: ComparisonOp::Eq,
                value: EventClass::Security.id().to_le_bytes().to_vec(),
                payload_field_name: None,
            }],
            projection_fields: Vec::new(),
            execution_mode: QueryExecutionMode::Synchronous,
            limits: QueryLimits::default(),
        };
        let result = engine.execute(&query);
        assert_eq!(result.status, QueryStatus::Ok);
        assert_eq!(result.matched_count, 1);
        assert_eq!(result.returned_count, 1);
        assert_eq!(result.results[0].event_class, EventClass::Security.id());
    }

    #[test]
    fn filter_by_urgency_range() {
        let events = vec![
            make_event(1, EventClass::State.id(), 1, 10, 7, 0, 0, 100),
            make_event(2, EventClass::State.id(), 1, 250, 7, 0, 0, 200),
        ];
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let query = DiagnosticQuery {
            query_type: QueryOperator::Filter,
            start_time_ns: 0,
            end_time_ns: u64::MAX,
            predicates: vec![Predicate {
                field: QueryField::Urgency,
                op: ComparisonOp::Ge,
                value: SECURITY_CRITICAL_URGENCY.to_le_bytes().to_vec(),
                payload_field_name: None,
            }],
            projection_fields: Vec::new(),
            execution_mode: QueryExecutionMode::Synchronous,
            limits: QueryLimits::default(),
        };
        let result = engine.execute(&query);
        assert_eq!(result.matched_count, 1);
        assert_eq!(result.results[0].urgency, 250);
    }

    #[test]
    fn time_window_filters_events() {
        let events = vec![
            make_event(1, EventClass::State.id(), 1, 50, 7, 0, 0, 100),
            make_event(2, EventClass::State.id(), 1, 50, 7, 0, 0, 500),
        ];
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let query = DiagnosticQuery {
            query_type: QueryOperator::Filter,
            start_time_ns: 200,
            end_time_ns: 400,
            predicates: Vec::new(),
            projection_fields: Vec::new(),
            execution_mode: QueryExecutionMode::Synchronous,
            limits: QueryLimits::default(),
        };
        let result = engine.execute(&query);
        assert_eq!(result.matched_count, 0);
    }

    #[test]
    fn result_limit_enforcement() {
        let events: Vec<PlatformEvent> = (1..=15)
            .map(|i| make_event(i, EventClass::State.id(), 1, 50, 7, 0, 0, i * 100))
            .collect();
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let query = DiagnosticQuery {
            query_type: QueryOperator::Filter,
            start_time_ns: 0,
            end_time_ns: u64::MAX,
            predicates: Vec::new(),
            projection_fields: Vec::new(),
            execution_mode: QueryExecutionMode::Synchronous,
            limits: QueryLimits {
                max_results: 10,
                max_duration_ns: 30_000_000_000,
                max_causality_depth: 100,
            },
        };
        let result = engine.execute(&query);
        assert_eq!(result.status, QueryStatus::ResultTruncated);
        assert_eq!(result.matched_count, 15);
        assert_eq!(result.returned_count, 10);
        assert_eq!(result.next_cursor, 10);
    }

    #[test]
    fn causality_traversal() {
        // 3 <- 2 <- 1 (3 caused by 2 caused by 1)
        let events = vec![
            make_event(1, EventClass::Execution.id(), 1, 50, 7, 0, 0, 100),
            make_event(2, EventClass::Execution.id(), 1, 50, 7, 1, 0, 200),
            make_event(3, EventClass::Execution.id(), 1, 50, 7, 2, 0, 300),
        ];
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let result = engine.traverse_causality(3, 100).unwrap();
        assert_eq!(result.returned_count, 3);
        // Root cause first.
        assert_eq!(result.results[0].event_id, 1);
        assert_eq!(result.results[2].event_id, 3);
    }

    #[test]
    fn causality_depth_limit() {
        let events = vec![
            make_event(1, EventClass::Execution.id(), 1, 50, 7, 0, 0, 100),
            make_event(2, EventClass::Execution.id(), 1, 50, 7, 1, 0, 200),
            make_event(3, EventClass::Execution.id(), 1, 50, 7, 2, 0, 300),
        ];
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let result = engine.traverse_causality(3, 1).unwrap();
        assert_eq!(result.status, QueryStatus::ResultTruncated);
        assert_eq!(result.returned_count, 1);
    }

    #[test]
    fn join_with_spans_by_trace_id() {
        let events = vec![
            make_event(1, EventClass::Communication.id(), 1, 50, 7, 0, 42, 100),
            make_event(2, EventClass::Execution.id(), 1, 50, 7, 0, 99, 200),
        ];
        let spans = vec![TraceSpan {
            span_id: 1,
            trace_id: 42,
            source: 1,
            start_event_id: 1,
            end_event_id: None,
            start_timestamp_ns: 100,
            end_timestamp_ns: None,
            span_class: aether_trace_model::SpanClass::Request,
            outcome: None,
            child_span_count: 0,
            parent_span_id: aether_trace_model::ROOT_SPAN_ID,
        }];
        let engine = DiagnosticQueryEngine::new(events, spans);
        let query = DiagnosticQuery {
            query_type: QueryOperator::Join,
            start_time_ns: 0,
            end_time_ns: u64::MAX,
            predicates: Vec::new(),
            projection_fields: Vec::new(),
            execution_mode: QueryExecutionMode::Synchronous,
            limits: QueryLimits::default(),
        };
        let result = engine.join_with_spans(&query);
        assert_eq!(result.matched_count, 1);
        assert_eq!(result.results[0].correlation_id, 42);
    }

    #[test]
    fn unknown_event_class_filter_returns_empty() {
        let events = vec![make_event(1, EventClass::State.id(), 1, 50, 7, 0, 0, 100)];
        let engine = DiagnosticQueryEngine::new(events, Vec::new());
        let query = DiagnosticQuery {
            query_type: QueryOperator::Filter,
            start_time_ns: 0,
            end_time_ns: u64::MAX,
            predicates: vec![Predicate {
                field: QueryField::EventClass,
                op: ComparisonOp::Eq,
                value: 0x0800u64.to_le_bytes().to_vec(), // not a valid class
                payload_field_name: None,
            }],
            projection_fields: Vec::new(),
            execution_mode: QueryExecutionMode::Synchronous,
            limits: QueryLimits::default(),
        };
        let result = engine.execute(&query);
        assert_eq!(result.status, QueryStatus::Ok);
        assert_eq!(result.matched_count, 0);
    }

    #[test]
    fn predefined_recent_errors_selects_urgent() {
        let query = predefined_query(PredefinedQuery::RecentErrors, 0, u64::MAX, None);
        assert_eq!(query.query_type, QueryOperator::Filter);
        assert_eq!(query.predicates.len(), 1);
        assert_eq!(query.predicates[0].field, QueryField::Urgency);
        assert_eq!(query.predicates[0].op, ComparisonOp::Ge);
    }

    #[test]
    fn authorize_security_events_requires_right() {
        assert!(
            authorize_query(
                EventClass::Security.id(),
                false,
                &[QueryCapabilityRight::QuerySecurityEvents]
            )
            .is_ok()
        );
        assert_eq!(
            authorize_query(EventClass::Security.id(), false, &[]),
            Err(QueryCapabilityRight::QuerySecurityEvents)
        );
    }

    #[test]
    fn authorize_capability_events_requires_right() {
        assert!(
            authorize_query(
                EventClass::Capability.id(),
                false,
                &[QueryCapabilityRight::QueryCapabilityEvents]
            )
            .is_ok()
        );
        assert_eq!(
            authorize_query(EventClass::Capability.id(), false, &[]),
            Err(QueryCapabilityRight::QueryCapabilityEvents)
        );
    }

    #[test]
    fn full_payload_requires_right() {
        assert_eq!(
            authorize_query(EventClass::State.id(), true, &[]),
            Err(QueryCapabilityRight::QueryFullPayload)
        );
        assert!(
            authorize_query(
                EventClass::State.id(),
                true,
                &[QueryCapabilityRight::QueryFullPayload]
            )
            .is_ok()
        );
    }

    #[test]
    fn default_limits() {
        let limits = QueryLimits::default();
        assert_eq!(limits.max_results, 1_000);
        assert_eq!(limits.max_duration_ns, 30_000_000_000);
        assert_eq!(limits.max_causality_depth, 100);
    }
}
