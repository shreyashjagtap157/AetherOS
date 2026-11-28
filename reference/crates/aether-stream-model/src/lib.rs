//! Streaming communication reference implementation (RFC-0029).
//!
//! Stream communication uses the `MSG_STREAM` flag in the message header
//! (RFC-0024.2) to indicate partial delivery. Stream chunks are grouped by a
//! client-selected `stream_id`, delivered in order, reassembled by the
//! receiver, and terminated by a final message without the `MSG_STREAM` flag.

use aether_message_format::{MSG_STREAM, Message, MessageHeader};

// ---------------------------------------------------------------------------
// Stream lifecycle
// ---------------------------------------------------------------------------

/// Stream state (RFC-0029.4).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StreamState {
    /// Stream established, accepting chunks.
    Open,
    /// Stream complete (final chunk received).
    Closed,
    /// Stream abandoned (final chunk not received within timeout).
    Abandoned,
    /// Stream cancelled by either endpoint (RFC-0029.7).
    Cancelled,
}

/// Chunk metadata carried in a stream message trailer (RFC-0029 §3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StreamHeader {
    /// Client-assigned stream identifier, unique per sender.
    pub stream_id: u64,
    /// 0-based chunk index, monotonically increasing.
    pub chunk_index: u64,
    /// Total expected size (0 if unknown, RFC-0029.2).
    pub total_size: u64,
    /// Bytes in this chunk.
    pub chunk_size: u64,
}

/// A stream session: tracks received chunks in order (RFC-0029.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stream {
    /// Stream identifier.
    pub stream_id: u64,
    /// Current state.
    pub state: StreamState,
    /// Highest contiguous chunk index received (reassembly cursor).
    pub next_expected_index: u64,
    /// Total size declared in the first chunk (0 if unknown).
    pub total_size: u64,
    /// Bytes buffered so far.
    pub bytes_received: u64,
    /// Receive window: maximum total bytes buffered before backpressure
    /// (RFC-0029.5).
    pub receive_window: u64,
}

impl Stream {
    /// Creates a new stream from its first chunk (RFC-0029.1).
    #[must_use]
    pub fn new(stream_id: u64, total_size: u64, receive_window: u64) -> Self {
        Self {
            stream_id,
            state: StreamState::Open,
            next_expected_index: 0,
            total_size,
            bytes_received: 0,
            receive_window,
        }
    }

    /// Receives a chunk: returns `Ok` if it is the next expected chunk
    /// (in-order delivery, RFC-0029.3), or `StreamOutOfOrder` otherwise.
    pub fn receive_chunk(
        &mut self,
        chunk_index: u64,
        chunk_size: u64,
    ) -> Result<(), StreamError> {
        if self.state != StreamState::Open {
            return Err(StreamError::NotOpen);
        }
        if chunk_index != self.next_expected_index {
            return Err(StreamError::OutOfOrder);
        }
        self.next_expected_index = self.next_expected_index.checked_add(1).ok_or(StreamError::Overflow)?;
        self.bytes_received = self.bytes_received.checked_add(chunk_size).ok_or(StreamError::Overflow)?;
        Ok(())
    }

    /// Marks the stream complete: final message without `MSG_STREAM`
    /// (RFC-0029.4).
    pub fn complete(&mut self) {
        self.state = StreamState::Closed;
    }

    /// Cancels the stream (RFC-0029.7): discards all previously received
    /// chunks.
    pub fn cancel(&mut self) {
        self.state = StreamState::Cancelled;
        self.next_expected_index = 0;
        self.bytes_received = 0;
    }

    /// Returns true if buffered bytes are within the receive window.
    /// Exceeding the window triggers backpressure (RFC-0029.5).
    #[must_use]
    pub fn within_window(&self) -> bool {
        self.bytes_received <= self.receive_window
    }

    /// Returns true if the stream is complete.
    #[must_use]
    pub fn is_complete(self) -> bool {
        self.state == StreamState::Closed
    }
}

impl Default for Stream {
    fn default() -> Self {
        Self {
            stream_id: 0,
            state: StreamState::Open,
            next_expected_index: 0,
            total_size: 0,
            bytes_received: 0,
            receive_window: u64::MAX,
        }
    }
}

// ---------------------------------------------------------------------------
// Stream manager (RFC-0029.8 multiplexing)
// ---------------------------------------------------------------------------

/// Manages multiple streams multiplexed over a single endpoint connection,
/// distinguished by `stream_id` (RFC-0029.8).
pub struct StreamManager {
    /// Active streams keyed by stream id.
    pub streams: Vec<Stream>,
}

impl StreamManager {
    /// Creates a new empty stream manager.
    #[must_use]
    pub fn new() -> Self {
        Self { streams: Vec::new() }
    }

    /// Establishes a stream on receipt of the first chunk (RFC-0029.1).
    pub fn establish(
        &mut self,
        stream_id: u64,
        total_size: u64,
        receive_window: u64,
    ) {
        if self.streams.iter().any(|s| s.stream_id == stream_id) {
            return; // already established
        }
        self.streams.push(Stream::new(stream_id, total_size, receive_window));
    }

    /// Returns the maximum simultaneous streams supported per endpoint
    /// (RFC-0029.8: at least 256).
    #[must_use]
    pub const fn max_streams() -> u64 {
        256
    }

    /// Returns the stream with the given id, if open.
    pub fn get(&self, stream_id: u64) -> Option<&Stream> {
        self.streams.iter().find(|s| s.stream_id == stream_id)
    }

    /// Returns the stream mutably.
    pub fn get_mut(&mut self, stream_id: u64) -> Option<&mut Stream> {
        self.streams.iter_mut().find(|s| s.stream_id == stream_id)
    }

    /// Cancels a stream (RFC-0029.7).
    pub fn cancel(&mut self, stream_id: u64) -> Result<(), StreamError> {
        match self.streams.iter_mut().find(|s| s.stream_id == stream_id) {
            Some(stream) => {
                stream.cancel();
                Ok(())
            }
            None => Err(StreamError::NotFound),
        }
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Message helpers (RFC-0029.1, RFC-0029.4)
// ---------------------------------------------------------------------------

/// Returns true if a message is a stream chunk (`MSG_STREAM` flag set,
/// RFC-0024.2).
#[must_use]
pub fn is_stream_chunk(message: &Message) -> bool {
    message.header.flags & MSG_STREAM != 0
}

/// Returns true if a message is the final chunk of a stream (no `MSG_STREAM`
/// flag set, RFC-0029.4).
#[must_use]
pub fn is_final_chunk(message: &Message) -> bool {
    message.header.flags & MSG_STREAM == 0
}

/// Builds a stream chunk header (RFC-0029 §3 trailer format).
#[must_use]
pub fn stream_header(stream_id: u64, chunk_index: u64, total_size: u64, chunk_size: u64) -> StreamHeader {
    StreamHeader {
        stream_id,
        chunk_index,
        total_size,
        chunk_size,
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Stream errors (RFC-0029 failure semantics).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StreamError {
    /// Stream is not open (already closed, abandoned, or cancelled).
    NotOpen,
    /// Chunk arrived out of order (RFC-0029.3).
    OutOfOrder,
    /// Stream not found.
    NotFound,
    /// Arithmetic overflow in stream accounting.
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_message_format::{EndpointId, Message, MessageHeader};

    fn chunk_message(stream_id: u64, chunk_index: u64) -> Message {
        Message {
            header: MessageHeader::new(
                1,
                EndpointId::none(),
                EndpointId::none(),
                0,
                0,
                MSG_STREAM,
                0,
                0,
            ),
            payload: Vec::new(),
            trailer: Vec::new(),
        }
    }

    #[test]
    fn stream_receive_in_order() {
        let mut s = Stream::new(1, 0, u64::MAX);
        assert_eq!(s.receive_chunk(0, 100), Ok(()));
        assert_eq!(s.receive_chunk(1, 100), Ok(()));
        assert_eq!(s.next_expected_index, 2);
        assert_eq!(s.bytes_received, 200);
    }

    #[test]
    fn out_of_order_chunk_rejected() {
        let mut s = Stream::new(1, 0, u64::MAX);
        assert_eq!(s.receive_chunk(1, 100), Err(StreamError::OutOfOrder));
    }

    #[test]
    fn stream_complete_and_cancel() {
        let mut s = Stream::new(1, 0, u64::MAX);
        s.complete();
        assert!(s.is_complete());
        let mut s2 = Stream::new(2, 0, u64::MAX);
        s2.receive_chunk(0, 50).unwrap();
        s2.cancel();
        assert_eq!(s2.state, StreamState::Cancelled);
        assert_eq!(s2.bytes_received, 0);
    }

    #[test]
    fn max_streams_is_at_least_256() {
        assert!(StreamManager::max_streams() >= 256);
    }

    #[test]
    fn stream_chunk_detection() {
        let msg = chunk_message(1, 0);
        assert!(is_stream_chunk(&msg));
        let mut final_msg = chunk_message(1, 1);
        final_msg.header.flags &= !MSG_STREAM;
        assert!(is_final_chunk(&final_msg));
    }
}
