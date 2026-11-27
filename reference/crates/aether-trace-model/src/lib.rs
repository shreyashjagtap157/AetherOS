//! Trace span reference implementation (RFC-0032).
//!
//! Trace spans correlate events produced by different components during a
//! single logical operation (service call, transaction, stream transfer),
//! forming a parent-child tree keyed by `trace_id`. Spans derive from the
//! RFC-0031 event model: every event produced during a span carries the
//! span's `trace_id` in its `correlation_id` field (RFC-0032.5).
//!
//! Sampling (RFC-0032.6) controls whether a span is fully instrumented or
//! lightweight; high-urgency spans (urgency >= 200 per RFC-0031.7) are
//! always fully recorded regardless of sampling policy (RFC-0032.7).

use aether_event_model::{EventEmitter, PlatformEvent, SECURITY_CRITICAL_URGENCY};
use core::fmt;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Span class (RFC-0032.4)
// ---------------------------------------------------------------------------

/// Class of a trace span (RFC-0032.4).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SpanClass {
    /// Inbound call; span opened on receive, closed on reply.
    Request = 0,
    /// Outbound call; span opened on send, closed on receive of reply/error.
    Response = 1,
    /// Within-component operation; opened and closed in the same component.
    Internal = 2,
    /// Async background work; may live indefinitely.
    Background = 3,
}

impl SpanClass {
    /// Returns the numeric discriminant.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for SpanClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpanClass::Request => write!(f, "REQUEST"),
            SpanClass::Response => write!(f, "RESPONSE"),
            SpanClass::Internal => write!(f, "INTERNAL"),
            SpanClass::Background => write!(f, "BACKGROUND"),
        }
    }
}

// ---------------------------------------------------------------------------
// Span outcome (RFC-0032.8)
// ---------------------------------------------------------------------------

/// Outcome of a completed span (RFC-0032.8).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SpanOutcome {
    /// Outcome undetermined (e.g. span abandoned without explicit close).
    Unknown = 0,
    /// Operation completed successfully.
    Ok = 1,
    /// Operation failed with a declared error code.
    Error = 2,
    /// Operation timed out.
    Timeout = 3,
}

impl SpanOutcome {
    /// Returns the numeric discriminant.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Locally unique span identifier within a trace.
pub type SpanId = u64;
/// Globally unique trace identifier shared by all spans in one trace.
pub type TraceId = u64;
/// Component endpoint where a trace starts (RFC-0032 data structures).
pub type EndpointId = u64;

/// Root spans have no parent (RFC-0032 data structures: `parent_span_id`).
pub const ROOT_SPAN_ID: SpanId = 0;

// ---------------------------------------------------------------------------
// Span structure (RFC-0032.1)
// ---------------------------------------------------------------------------

/// A trace span correlating events of a single logical operation (RFC-0032.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceSpan {
    /// Locally unique within a trace.
    pub span_id: SpanId,
    /// Globally unique trace identifier shared by all spans of one trace.
    pub trace_id: TraceId,
    /// Component endpoint where the trace starts.
    pub source: EndpointId,
    /// First event id in this span.
    pub start_event_id: u64,
    /// Last event id in this span (filled on close).
    pub end_event_id: Option<u64>,
    /// Scheduling time the span started (RFC-0012).
    pub start_timestamp_ns: u64,
    /// Scheduling time the span closed (filled on close).
    pub end_timestamp_ns: Option<u64>,
    /// Class of the span (RFC-0032.4).
    pub span_class: SpanClass,
    /// Outcome, set on span close (RFC-0032.8).
    pub outcome: Option<SpanOutcome>,
    /// Number of child spans (RFC-0032 data structures).
    pub child_span_count: u32,
    /// Parent span id; `ROOT_SPAN_ID` (0) if this is a root span (RFC-0032.3).
    pub parent_span_id: SpanId,
}

impl TraceSpan {
    /// Returns whether the span has been closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.end_timestamp_ns.is_some()
    }

    /// Returns whether this is a root span (no parent).
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.parent_span_id == ROOT_SPAN_ID
    }
}

// ---------------------------------------------------------------------------
// Sampling policy (RFC-0032.6)
// ---------------------------------------------------------------------------

/// Sampling policy declared in the component manifest (RFC-0032.6).
///
/// Controls whether a span is fully instrumented (all events recorded) or
/// lightweight (only `span_id`, `trace_id`, `start`, `end`).
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSamplingPolicy {
    /// Fraction of spans to fully record, in the range `0.0..=1.0`.
    pub sample_rate: f64,
    /// When true, high-urgency spans are always fully recorded regardless of
    /// `sample_rate` (RFC-0032.7).
    pub always_record_high_urgency: bool,
}

impl Default for TraceSamplingPolicy {
    fn default() -> Self {
        Self {
            sample_rate: 1.0,
            always_record_high_urgency: true,
        }
    }
}

impl TraceSamplingPolicy {
    /// Returns whether a span with the given `is_high_urgency` flag should be
    /// fully recorded under this policy (RFC-0032.6, RFC-0032.7).
    #[must_use]
    pub fn should_record_fully(&self, is_high_urgency: bool) -> bool {
        if is_high_urgency && self.always_record_high_urgency {
            return true; // RFC-0032.7: urgency >= 200 always recorded
        }
        self.sample_rate >= 1.0
    }
}

// ---------------------------------------------------------------------------
// Span outcome helpers
// ---------------------------------------------------------------------------

/// Whether an urgency level is high-urgency (>= 200, RFC-0031.7).
#[must_use]
pub const fn is_high_urgency(urgency: u8) -> bool {
    urgency >= SECURITY_CRITICAL_URGENCY
}

// ---------------------------------------------------------------------------
// Trace emitter (RFC-0032.2, RFC-0032.3, RFC-0032.5)
// ---------------------------------------------------------------------------

/// Creates and closes trace spans and correlates events to traces.
///
/// Spans form a tree: children share the parent's `trace_id` and reference
/// it via `parent_span_id` (RFC-0032.3). Every event emitted while a span is
/// active carries the span's `trace_id` in its `correlation_id` field
/// (RFC-0032.5), linking events to traces.
pub struct TraceEmitter {
    /// Next span identifier to assign.
    next_span_id: SpanId,
    /// Spans keyed by span id.
    spans: BTreeMap<SpanId, TraceSpan>,
    /// Underlying RFC-0031 event emitter used to record events.
    event_emitter: EventEmitter,
    /// Sampling policy declared in the component manifest (RFC-0032.6).
    sampling: TraceSamplingPolicy,
}

impl Default for TraceEmitter {
    fn default() -> Self {
        Self::new(TraceSamplingPolicy::default())
    }
}

impl TraceEmitter {
    /// Creates a trace emitter with the given sampling policy.
    #[must_use]
    pub fn new(sampling: TraceSamplingPolicy) -> Self {
        Self {
            next_span_id: 1,
            spans: BTreeMap::new(),
            event_emitter: EventEmitter::new(),
            sampling,
        }
    }

    /// Returns the sampling policy in effect (RFC-0032.6).
    #[must_use]
    pub fn sampling_policy(&self) -> &TraceSamplingPolicy {
        &self.sampling
    }

    /// Returns the number of spans currently tracked.
    #[must_use]
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Returns the span with `span_id`, if tracked.
    #[must_use]
    pub fn span(&self, span_id: SpanId) -> Option<&TraceSpan> {
        self.spans.get(&span_id)
    }

    /// Returns all spans.
    #[must_use]
    pub fn spans(&self) -> &BTreeMap<SpanId, TraceSpan> {
        &self.spans
    }

    /// Returns all child span ids of `parent_span_id` (RFC-0032.3).
    #[must_use]
    pub fn children_of(&self, parent_span_id: SpanId) -> Vec<SpanId> {
        self.spans
            .iter()
            .filter_map(|(id, span)| {
                if span.parent_span_id == parent_span_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Span lifecycle (RFC-0032.2)
    // -----------------------------------------------------------------------

    /// Starts a new span for `trace_id` originating at `source` (RFC-0032.2).
    ///
    /// If `parent_span_id` is `ROOT_SPAN_ID` (0) this is a root span
    /// (RFC-0032.3). Returns the assigned span id. A span is lightweight or
    /// fully recorded according to the sampling policy (RFC-0032.6).
    pub fn start_span(
        &mut self,
        trace_id: TraceId,
        source: EndpointId,
        span_class: SpanClass,
        parent_span_id: SpanId,
        start_timestamp_ns: u64,
    ) -> SpanId {
        let span_id = self.next_span_id;
        self.next_span_id += 1;

        let span = TraceSpan {
            span_id,
            trace_id,
            source,
            start_event_id: 0,
            end_event_id: None,
            start_timestamp_ns,
            end_timestamp_ns: None,
            span_class,
            outcome: None,
            child_span_count: 0,
            parent_span_id,
        };

        // Increment the parent's child count (RFC-0032.3).
        if parent_span_id != ROOT_SPAN_ID {
            if let Some(parent) = self.spans.get_mut(&parent_span_id) {
                parent.child_span_count += 1;
            }
        }

        self.spans.insert(span_id, span);
        span_id
    }

    /// Closes `span_id` with `outcome` at `end_timestamp_ns` (RFC-0032.2,
    /// RFC-0032.8). Returns an error if the span is unknown or already closed.
    ///
    /// # Errors
    ///
    /// Returns [`CloseSpanError::UnknownSpan`] if `span_id` is not tracked, or
    /// [`CloseSpanError::AlreadyClosed`] if the span has already been closed.
    pub fn close_span(
        &mut self,
        span_id: SpanId,
        outcome: SpanOutcome,
        end_timestamp_ns: u64,
    ) -> Result<(), CloseSpanError> {
        let span = self
            .spans
            .get_mut(&span_id)
            .ok_or(CloseSpanError::UnknownSpan)?;
        if span.is_closed() {
            return Err(CloseSpanError::AlreadyClosed);
        }
        span.end_timestamp_ns = Some(end_timestamp_ns);
        span.outcome = Some(outcome);
        // Record the last event id seen for this span, if any events were
        // emitted while it was active.
        span.end_event_id = self.event_emitter.last_event().map(|e| e.event_id);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Event correlation (RFC-0032.5)
    // -----------------------------------------------------------------------

    /// Emits `event` while `span_id` is active, assigning the span's
    /// `trace_id` to the event's `correlation_id` field (RFC-0032.5).
    ///
    /// Returns the assigned event id. The event is recorded by the underlying
    /// RFC-0031 event emitter when the span is fully recorded or the event is
    /// high-urgency (RFC-0032.7).
    ///
    /// # Errors
    ///
    /// Returns [`EmitEventError::UnknownSpan`] if `span_id` is not tracked.
    pub fn emit_event_in_span(
        &mut self,
        span_id: SpanId,
        mut event: PlatformEvent,
    ) -> Result<EventId, EmitEventError> {
        let span = self
            .spans
            .get(&span_id)
            .ok_or(EmitEventError::UnknownSpan)?;
        let trace_id = span.trace_id;
        event.correlation_id = trace_id; // link event to span (RFC-0032.5)

        if span.start_event_id == 0 {
            // First event of the span: record it as the span's start event.
            if let Some(s) = self.spans.get_mut(&span_id) {
                s.start_event_id = self.event_emitter.event_count() as u64 + 1;
            }
        }

        let fully_recorded = self
            .sampling
            .should_record_fully(is_high_urgency(event.urgency));
        if fully_recorded {
            self.event_emitter.emit(event);
        }
        Ok(self.event_emitter.event_count() as u64)
    }

    /// Returns all RFC-0031 events emitted (fully recorded spans only,
    /// plus all high-urgency events per RFC-0032.7).
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        self.event_emitter.events()
    }

    /// Returns the number of RFC-0031 events emitted.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.event_emitter.event_count()
    }
}

/// Event identifier assigned by the underlying RFC-0031 event emitter.
pub type EventId = u64;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error returned when closing a span (RFC-0032.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseSpanError {
    /// The span id is not tracked.
    UnknownSpan,
    /// The span has already been closed.
    AlreadyClosed,
}

/// Error returned when emitting an event in a span (RFC-0032.5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmitEventError {
    /// The span id is not tracked.
    UnknownSpan,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use aether_event_model::EventClass;
    use aether_types::DomainId;

    #[test]
    fn span_lifecycle_start_and_close() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
        let span_id = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
        let span = emitter.span(span_id).unwrap();
        assert_eq!(span.trace_id, 100);
        assert_eq!(span.span_class, SpanClass::Request);
        assert_eq!(span.parent_span_id, ROOT_SPAN_ID);
        assert!(span.is_root());
        assert!(!span.is_closed());

        emitter
            .close_span(span_id, SpanOutcome::Ok, 2_000_000)
            .unwrap();
        let span = emitter.span(span_id).unwrap();
        assert!(span.is_closed());
        assert_eq!(span.outcome, Some(SpanOutcome::Ok));
        assert_eq!(span.end_timestamp_ns, Some(2_000_000));
    }

    #[test]
    fn close_unknown_or_twice_rejected() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
        assert!(emitter.close_span(999, SpanOutcome::Ok, 1_000_000).is_err());
        let span_id = emitter.start_span(100, 1, SpanClass::Internal, ROOT_SPAN_ID, 1_000_000);
        emitter
            .close_span(span_id, SpanOutcome::Ok, 2_000_000)
            .unwrap();
        assert!(
            emitter
                .close_span(span_id, SpanOutcome::Ok, 3_000_000)
                .is_err()
        );
    }

    #[test]
    fn parent_child_tree_structure() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
        let parent = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
        let child1 = emitter.start_span(100, 2, SpanClass::Internal, parent, 1_100_000);
        let child2 = emitter.start_span(100, 2, SpanClass::Internal, parent, 1_200_000);

        assert_eq!(emitter.span(child1).unwrap().parent_span_id, parent);
        assert_eq!(emitter.span(child2).unwrap().parent_span_id, parent);
        // Children share trace_id but have distinct span_id.
        assert_ne!(child1, child2);
        assert_eq!(
            emitter.span(child1).unwrap().trace_id,
            emitter.span(child2).unwrap().trace_id
        );
        assert_eq!(emitter.children_of(parent), vec![child1, child2]);
        assert_eq!(emitter.span(parent).unwrap().child_span_count, 2);
    }

    #[test]
    fn event_to_span_correlation() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
        let span_id = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
        let event = PlatformEvent {
            event_id: 0,
            event_class: EventClass::Execution.id(),
            event_type: 1,
            timestamp_ns: 1_050_000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 50,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        emitter.emit_event_in_span(span_id, event).unwrap();
        let event = emitter.events().last().unwrap();
        assert_eq!(event.correlation_id, 100); // linked to span's trace_id (RFC-0032.5)
    }

    #[test]
    fn sampling_policy_declared() {
        let policy = TraceSamplingPolicy {
            sample_rate: 0.5,
            always_record_high_urgency: true,
        };
        assert!((policy.sample_rate - 0.5).abs() < f64::EPSILON);
        assert!(policy.always_record_high_urgency);
        assert!(policy.should_record_fully(true)); // high urgency always recorded
        assert!(!policy.should_record_fully(false)); // 0.5 < 1.0, not fully
    }

    #[test]
    fn high_urgency_always_recorded() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy {
            sample_rate: 0.0,
            always_record_high_urgency: true,
        });
        let span_id = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
        let critical = PlatformEvent {
            event_id: 0,
            event_class: EventClass::Security.id(),
            event_type: 1,
            timestamp_ns: 1_050_000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: SECURITY_CRITICAL_URGENCY,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        emitter.emit_event_in_span(span_id, critical).unwrap();
        assert_eq!(emitter.event_count(), 1); // recorded despite sample_rate 0.0 (RFC-0032.7)
    }

    #[test]
    fn outcome_set_on_close() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
        let span_id = emitter.start_span(100, 1, SpanClass::Response, ROOT_SPAN_ID, 1_000_000);
        emitter
            .close_span(span_id, SpanOutcome::Timeout, 2_000_000)
            .unwrap();
        assert_eq!(
            emitter.span(span_id).unwrap().outcome,
            Some(SpanOutcome::Timeout)
        );
    }

    #[test]
    fn span_class_display_names() {
        assert_eq!(format!("{}", SpanClass::Request), "REQUEST");
        assert_eq!(format!("{}", SpanClass::Background), "BACKGROUND");
    }

    #[test]
    fn unknown_span_emission_rejected() {
        let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
        let event = PlatformEvent {
            event_id: 0,
            event_class: EventClass::Execution.id(),
            event_type: 1,
            timestamp_ns: 1_000_000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        assert!(emitter.emit_event_in_span(999, event).is_err());
    }
}
