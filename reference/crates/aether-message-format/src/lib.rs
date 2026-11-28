//! Message format reference implementation (RFC-0024).
//!
//! Canonical binary message format for all inter-component and inter-domain
//! communication in the platform. Every message — whether a synchronous IPC
//! call, an asynchronous event, a capability transfer, or a stream chunk —
//! uses this format as its wire representation.
//!
//! The header (RFC-0024.1) is a fixed layout followed by `payload_len` bytes
//! of payload and `trailer_len` bytes of trailer. All multi-byte header
//! fields use network byte order (big-endian) per RFC-0024.8.

/// Platform signature in every message header: `0x434F4D50` ("COMP")
/// (RFC-0024.1).
pub const MAGIC: u32 = 0x434F4D50;

/// Format version defined by this RFC (RFC-0024.1).
pub const VERSION: u16 = 1;

/// Fixed header size in bytes (RFC-0024.1).
pub const HEADER_LEN: usize = 61;

/// Maximum message size (payload + trailer) enforced per endpoint
/// (RFC-0024.6).
pub const MAX_MESSAGE_SIZE: usize = 1 << 20; // 1 MiB

// ---------------------------------------------------------------------------
// Flags (RFC-0024.2)
// ---------------------------------------------------------------------------

/// `MSG_FAST` — fast path; skip logging if possible.
pub const MSG_FAST: u8 = 0x01;
/// `MSG_CAP_TRANSFER` — trailer contains capability tokens.
pub const MSG_CAP_TRANSFER: u8 = 0x02;
/// `MSG_STREAM` — partial message; more chunks follow.
pub const MSG_STREAM: u8 = 0x04;
/// `MSG_NO_REPLY` — sender does not expect a reply.
pub const MSG_NO_REPLY: u8 = 0x08;
/// `MSG_ERROR` — this message is an error response.
pub const MSG_ERROR: u8 = 0x10;

// ---------------------------------------------------------------------------
// Endpoint identifier
// ---------------------------------------------------------------------------

/// Endpoint identifier. RFC-0025 defines the endpoint model in full; this is
/// the minimal fixed-size form used by the message header until RFC-0025 is
/// implemented as a separate crate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct EndpointId(pub [u8; 16]);

impl EndpointId {
    /// Creates a new endpoint id from a 16-byte array.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the zero endpoint id (never a valid destination).
    #[must_use]
    pub const fn none() -> Self {
        Self([0; 16])
    }

    /// Returns true if this is the zero endpoint id.
    #[must_use]
    pub fn is_none(self) -> bool {
        self.0 == [0; 16]
    }
}

// ---------------------------------------------------------------------------
// Message header (RFC-0024.1)
// ---------------------------------------------------------------------------

/// The fixed message header. All multi-byte fields are stored in network
/// byte order (big-endian) on the wire (RFC-0024.8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageHeader {
    /// Platform signature, always `MAGIC`.
    pub magic: u32,
    /// Format version; this RFC defines version 1.
    pub version: u16,
    /// Length of this header in bytes (always `HEADER_LEN`).
    pub header_len: u16,
    /// Globally unique per-source sequence number.
    pub message_id: u64,
    /// Source endpoint (RFC-0025).
    pub src_endpoint: EndpointId,
    /// Destination endpoint (RFC-0025).
    pub dst_endpoint: EndpointId,
    /// Length of payload in bytes.
    pub payload_len: u32,
    /// Length of trailer in bytes.
    pub trailer_len: u32,
    /// Message flags (see `MSG_*` constants).
    pub flags: u8,
    /// Urgency: 0 = lowest, 255 = highest.
    pub urgency: u8,
    /// Reserved, must be zero.
    pub reserved: u16,
    /// Number of capability tokens in the trailer.
    pub capability_count: u8,
}

impl MessageHeader {
    /// Creates a new header with the given fields. `magic`, `version`, and
    /// `header_len` are set to the canonical values.
    #[must_use]
    pub const fn new(
        message_id: u64,
        src_endpoint: EndpointId,
        dst_endpoint: EndpointId,
        payload_len: u32,
        trailer_len: u32,
        flags: u8,
        urgency: u8,
        capability_count: u8,
    ) -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            header_len: HEADER_LEN as u16,
            message_id,
            src_endpoint,
            dst_endpoint,
            payload_len,
            trailer_len,
            flags,
            urgency,
            reserved: 0,
            capability_count,
        }
    }

    /// Serializes the header into a 55-byte buffer in network byte order.
    #[must_use]
    pub fn serialize(&self) -> [u8; HEADER_LEN] {
        let mut buf = [0u8; HEADER_LEN];
        let mut i = 0;
        // magic (u32, big-endian)
        buf[i..i + 4].copy_from_slice(&self.magic.to_be_bytes());
        i += 4;
        // version (u16, big-endian)
        buf[i..i + 2].copy_from_slice(&self.version.to_be_bytes());
        i += 2;
        // header_len (u16, big-endian)
        buf[i..i + 2].copy_from_slice(&self.header_len.to_be_bytes());
        i += 2;
        // message_id (u64, big-endian)
        buf[i..i + 8].copy_from_slice(&self.message_id.to_be_bytes());
        i += 8;
        // src_endpoint (16 bytes)
        buf[i..i + 16].copy_from_slice(&self.src_endpoint.0);
        i += 16;
        // dst_endpoint (16 bytes)
        buf[i..i + 16].copy_from_slice(&self.dst_endpoint.0);
        i += 16;
        // payload_len (u32, big-endian)
        buf[i..i + 4].copy_from_slice(&self.payload_len.to_be_bytes());
        i += 4;
        // trailer_len (u32, big-endian)
        buf[i..i + 4].copy_from_slice(&self.trailer_len.to_be_bytes());
        i += 4;
        // flags (u8)
        buf[i] = self.flags;
        i += 1;
        // urgency (u8)
        buf[i] = self.urgency;
        i += 1;
        // reserved (u16, big-endian)
        buf[i..i + 2].copy_from_slice(&self.reserved.to_be_bytes());
        i += 2;
        // capability_count (u8)
        buf[i] = self.capability_count;
        buf
    }

    /// Deserializes a header from a 55-byte buffer in network byte order.
    /// Returns an error if the magic or version are invalid, or if the
    /// buffer is too short.
    pub fn deserialize(buf: &[u8]) -> Result<Self, MessageError> {
        if buf.len() < HEADER_LEN {
            return Err(MessageError::TruncatedHeader);
        }
        let mut i = 0;
        let magic = u32::from_be_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
        i += 4;
        if magic != MAGIC {
            return Err(MessageError::InvalidMagic);
        }
        let version = u16::from_be_bytes([buf[i], buf[i + 1]]);
        i += 2;
        if version != VERSION {
            return Err(MessageError::VersionMismatch);
        }
        let header_len = u16::from_be_bytes([buf[i], buf[i + 1]]);
        i += 2;
        if header_len as usize != HEADER_LEN {
            return Err(MessageError::InvalidHeaderLength);
        }
        let message_id = u64::from_be_bytes([
            buf[i], buf[i + 1], buf[i + 2], buf[i + 3],
            buf[i + 4], buf[i + 5], buf[i + 6], buf[i + 7],
        ]);
        i += 8;
        let mut src_bytes = [0u8; 16];
        src_bytes.copy_from_slice(&buf[i..i + 16]);
        i += 16;
        let mut dst_bytes = [0u8; 16];
        dst_bytes.copy_from_slice(&buf[i..i + 16]);
        i += 16;
        let payload_len = u32::from_be_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
        i += 4;
        let trailer_len = u32::from_be_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
        i += 4;
        let flags = buf[i];
        i += 1;
        let urgency = buf[i];
        i += 1;
        let reserved = u16::from_be_bytes([buf[i], buf[i + 1]]);
        i += 2;
        let capability_count = buf[i];

        Ok(Self {
            magic,
            version,
            header_len,
            message_id,
            src_endpoint: EndpointId(src_bytes),
            dst_endpoint: EndpointId(dst_bytes),
            payload_len,
            trailer_len,
            flags,
            urgency,
            reserved,
            capability_count,
        })
    }
}

// ---------------------------------------------------------------------------
// Complete message
// ---------------------------------------------------------------------------

/// A complete message: fixed header plus variable payload and trailer.
/// The payload is opaque to the Communication layer (RFC-0024.5). The
/// trailer contains capability tokens (serialized per RFC-0037) and
/// optional inline error data (RFC-0024.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Vec<u8>,
    pub trailer: Vec<u8>,
}

impl Message {
    /// Creates a new message with the given header, payload, and trailer.
    /// Returns `PayloadTooLarge` if payload + trailer exceeds
    /// `MAX_MESSAGE_SIZE` (RFC-0024.6).
    pub fn new(
        header: MessageHeader,
        payload: Vec<u8>,
        trailer: Vec<u8>,
    ) -> Result<Self, MessageError> {
        if (payload.len() + trailer.len()) > MAX_MESSAGE_SIZE {
            return Err(MessageError::PayloadTooLarge);
        }
        Ok(Self {
            header,
            payload,
            trailer,
        })
    }

    /// Serializes the complete message into a byte vector: header, then
    /// payload, then trailer, all in network byte order.
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = self.header.serialize().to_vec();
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.trailer);
        buf
    }

    /// Deserializes a complete message from a byte buffer. Returns an error
    /// if the header is invalid or the buffer does not contain the declared
    /// payload and trailer bytes.
    pub fn deserialize(buf: &[u8]) -> Result<Self, MessageError> {
        let header = MessageHeader::deserialize(buf)?;
        let payload_start = HEADER_LEN;
        let payload_end = payload_start
            .checked_add(header.payload_len as usize)
            .ok_or(MessageError::PayloadTooLarge)?;
        let trailer_end = payload_end
            .checked_add(header.trailer_len as usize)
            .ok_or(MessageError::PayloadTooLarge)?;
        if buf.len() < trailer_end {
            return Err(MessageError::TruncatedHeader);
        }
        let payload = buf[payload_start..payload_end].to_vec();
        let trailer = buf[payload_end..trailer_end].to_vec();
        Ok(Self {
            header,
            payload,
            trailer,
        })
    }

    /// Returns true if the `MSG_ERROR` flag is set (RFC-0024.7).
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.header.flags & MSG_ERROR != 0
    }

    /// Returns true if the `MSG_NO_REPLY` flag is set (RFC-0024.2).
    #[must_use]
    pub fn is_no_reply(&self) -> bool {
        self.header.flags & MSG_NO_REPLY != 0
    }

    /// Returns true if the `MSG_STREAM` flag is set (RFC-0024.2).
    #[must_use]
    pub fn is_stream(&self) -> bool {
        self.header.flags & MSG_STREAM != 0
    }

    /// Returns true if the `MSG_CAP_TRANSFER` flag is set (RFC-0024.2).
    #[must_use]
    pub fn has_capability_transfer(&self) -> bool {
        self.header.flags & MSG_CAP_TRANSFER != 0
    }

    /// Creates an error response message (RFC-0024.7). The error code is
    /// placed in the first 4 bytes of the payload. `src_endpoint` and
    /// `dst_endpoint` are swapped relative to the original request.
    pub fn error_response(
        message_id: u64,
        src_endpoint: EndpointId,
        dst_endpoint: EndpointId,
        error_code: u32,
    ) -> Self {
        let payload = error_code.to_be_bytes().to_vec();
        let header = MessageHeader::new(
            message_id,
            src_endpoint,
            dst_endpoint,
            payload.len() as u32,
            0,
            MSG_ERROR,
            0,
            0,
        );
        Self {
            header,
            payload,
            trailer: Vec::new(),
        }
    }

    /// Creates a reply message for a request, per RFC-0024.9: the reply MUST
    /// have the same `message_id` as the request, with `src_endpoint` and
    /// `dst_endpoint` swapped.
    #[must_use]
    pub fn reply_for(
        message_id: u64,
        request_src: EndpointId,
        request_dst: EndpointId,
        payload: Vec<u8>,
    ) -> Result<Self, MessageError> {
        let header = MessageHeader::new(
            message_id,
            request_dst, // reply source = request destination
            request_src, // reply destination = request source
            payload.len() as u32,
            0,
            0,
            0,
            0,
        );
        Self::new(header, payload, Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Message format errors (RFC-0024.4 through RFC-0024.7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageError {
    /// Header magic did not match `MAGIC`.
    InvalidMagic,
    /// Header version is not supported by this implementation.
    VersionMismatch,
    /// Buffer is too short to contain a complete header.
    TruncatedHeader,
    /// Payload + trailer exceeds `MAX_MESSAGE_SIZE`.
    PayloadTooLarge,
    /// Header length field does not match `HEADER_LEN`.
    InvalidHeaderLength,
}
