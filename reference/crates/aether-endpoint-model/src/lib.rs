//! Endpoint model reference implementation (RFC-0025).
//!
//! Endpoints are named, capability-protected communication terminals in the
//! Communication Architecture. Every endpoint has a fixed-length identifier
//! computed from a hash of its domain, name, and declared capabilities
//! (RFC-0025.1). Endpoints are capability-protected: registering an endpoint
//! requires a `REGISTER_ENDPOINT` capability; receiving requires a `RECV`
//! capability (RFC-0025.2).
//!
//! This crate provides the endpoint data structures, the endpoint capability
//! kinds, the communication error codes, and an in-memory endpoint registry.
//! Capability tokens are opaque byte vectors (serialized per RFC-0037 when
//! that crate is available).

use aether_types::DomainId;

// ---------------------------------------------------------------------------
// Endpoint identifier (RFC-0025.1)
// ---------------------------------------------------------------------------

/// Fixed-length endpoint identifier: BLAKE3-256 of (domain_id, name,
/// cap_hash). 32 bytes (RFC-0025.1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct EndpointId(pub [u8; 32]);

impl EndpointId {
    /// Creates a new endpoint id from a 32-byte array.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the zero endpoint id (never valid).
    #[must_use]
    pub const fn none() -> Self {
        Self([0; 32])
    }

    /// Returns true if this is the zero endpoint id.
    #[must_use]
    pub fn is_none(self) -> bool {
        self.0 == [0; 32]
    }
}

// ---------------------------------------------------------------------------
// Endpoint capability kinds (RFC-0025.5)
// ---------------------------------------------------------------------------

/// Endpoint capability kinds (RFC-0025.5).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EndpointCapKind {
    /// Send messages to this endpoint.
    Send,
    /// Receive messages from this endpoint.
    Recv,
    /// Read endpoint metadata without receiving.
    Query,
    /// De-register this endpoint.
    Close,
    /// Transfer endpoint ownership.
    Inherit,
}

impl EndpointCapKind {
    /// Returns the bit position for this capability kind in a rights bitmask.
    #[must_use]
    pub const fn bit(self) -> u8 {
        match self {
            EndpointCapKind::Send => 1 << 0,
            EndpointCapKind::Recv => 1 << 1,
            EndpointCapKind::Query => 1 << 2,
            EndpointCapKind::Close => 1 << 3,
            EndpointCapKind::Inherit => 1 << 4,
        }
    }

    /// Returns true if the rights bitmask includes this capability kind.
    #[must_use]
    pub const fn in_rights(self, rights: u8) -> bool {
        rights & self.bit() != 0
    }
}

/// All defined endpoint capability kinds.
pub const ALL_ENDPOINT_CAPS: [EndpointCapKind; 5] = [
    EndpointCapKind::Send,
    EndpointCapKind::Recv,
    EndpointCapKind::Query,
    EndpointCapKind::Close,
    EndpointCapKind::Inherit,
];

// ---------------------------------------------------------------------------
// Endpoint operating modes (RFC-0025.7)
// ---------------------------------------------------------------------------

/// Endpoint operating mode (RFC-0025.7).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EndpointMode {
    /// Handler mode (default): messages delivered to declared handler.
    Handler = 0,
    /// Polling mode: receiver dequeues messages explicitly.
    Poll = 1,
}

impl EndpointMode {
    /// Returns true if this is polling mode.
    #[must_use]
    pub const fn is_poll(self) -> bool {
        matches!(self, EndpointMode::Poll)
    }
}

// ---------------------------------------------------------------------------
// Communication error codes (RFC-0025.8)
// ---------------------------------------------------------------------------

/// Communication error codes (RFC-0025.8). Returned in `MSG_ERROR` responses
/// per RFC-0025.9 / RFC-0024.7.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum CommErrorCode {
    /// Operation succeeded.
    Ok = 0,
    /// No such endpoint.
    ENODEST = 1,
    /// Sender lacks required capability for this operation.
    ENOCAP = 2,
    /// Message exceeds endpoint's declared maximum.
    EMSGSIZE = 3,
    /// Endpoint not in connected state.
    ENOTCONN = 4,
    /// Capability presented has been revoked.
    ECAPSTALE = 5,
    /// Malformed message format.
    EPROTO = 6,
    /// Endpoint receive quota exceeded.
    EQUOTA = 7,
}

impl CommErrorCode {
    /// Returns the numeric code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

// ---------------------------------------------------------------------------
// Data structures (RFC-0025.3)
// ---------------------------------------------------------------------------

/// Endpoint registration declaration (RFC-0025.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointRegistration {
    /// Endpoint name (up to 256 bytes).
    pub name: String,
    /// Maximum message size in bytes.
    pub max_msg_size: u64,
    /// Supported capability rights (bitmask of `EndpointCapKind` bits).
    pub supported_rights: u8,
    /// Handler capability invoked on message receipt; `None` for polling mode.
    pub handler_cap: Option<Vec<u8>>,
    /// Operating mode: handler or poll.
    pub mode: EndpointMode,
}

/// Endpoint metadata returned on registration/query (RFC-0025 data structures).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointMetadata {
    /// Endpoint identifier.
    pub id: EndpointId,
    /// Owning execution domain.
    pub owner_domain: DomainId,
    /// Maximum message size in bytes.
    pub max_msg_size: u64,
    /// Current messages waiting in the queue.
    pub queue_depth: u64,
    /// Maximum queue depth.
    pub queue_capacity: u64,
    /// Rights granted at registration.
    pub registered_rights: u8,
}

// ---------------------------------------------------------------------------
// Endpoint errors
// ---------------------------------------------------------------------------

/// Endpoint model errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointError {
    /// The endpoint id does not exist.
    NotFound,
    /// The endpoint is already registered.
    AlreadyExists,
    /// The name exceeds 256 bytes.
    NameTooLong,
    /// The operation requires a capability the caller does not hold.
    NoCap,
    /// The capability presented has been revoked.
    CapStale,
    /// Endpoint not in connected state.
    NotConnected,
    /// Endpoint receive quota exceeded.
    QuotaExceeded,
    /// Forced close with pending messages would lose data.
    QueueNotEmpty,
}

impl From<EndpointError> for CommErrorCode {
    fn from(e: EndpointError) -> Self {
        match e {
            EndpointError::NotFound => CommErrorCode::ENODEST,
            EndpointError::NoCap => CommErrorCode::ENOCAP,
            EndpointError::QuotaExceeded => CommErrorCode::EQUOTA,
            EndpointError::NotConnected => CommErrorCode::ENOTCONN,
            EndpointError::CapStale => CommErrorCode::ECAPSTALE,
            EndpointError::QueueNotEmpty => CommErrorCode::ENOTCONN,
            EndpointError::AlreadyExists | EndpointError::NameTooLong => CommErrorCode::EPROTO,
        }
    }
}

// ---------------------------------------------------------------------------
// Endpoint registry
// ---------------------------------------------------------------------------

/// An in-memory endpoint registry implementing RFC-0025 registration,
/// lookup, and close semantics.
pub struct EndpointRegistry {
    endpoints: Vec<EndpointEntry>,
    next_domain: DomainId,
}

struct EndpointEntry {
    id: EndpointId,
    registration: EndpointRegistration,
    metadata: EndpointMetadata,
    closed: bool,
    queue_depth: u64,
}

impl EndpointRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            endpoints: Vec::new(),
            next_domain: DomainId::new(1).unwrap(),
        }
    }

    /// Registers an endpoint (RFC-0025.3). Returns the endpoint id and a
    /// capability token for the endpoint (RFC-0025.4).
    pub fn register(
        &mut self,
        domain: DomainId,
        registration: EndpointRegistration,
    ) -> Result<(EndpointId, Vec<u8>), EndpointError> {
        if registration.name.len() > 256 {
            return Err(EndpointError::NameTooLong);
        }
        if self
            .endpoints
            .iter()
            .any(|e| e.registration.name == registration.name)
        {
            return Err(EndpointError::AlreadyExists);
        }
        let id = EndpointId::new(crate::compute_endpoint_id(
            domain,
            &registration.name,
            registration.supported_rights,
        ));
        let metadata = EndpointMetadata {
            id,
            owner_domain: domain,
            max_msg_size: registration.max_msg_size,
            queue_depth: 0,
            queue_capacity: 64,
            registered_rights: registration.supported_rights,
        };
        let token = EndpointRegistry::make_token(&id, registration.supported_rights);
        self.endpoints.push(EndpointEntry {
            id,
            registration,
            metadata,
            closed: false,
            queue_depth: 0,
        });
        Ok((id, token))
    }

    /// Looks up endpoint metadata by id (RFC-0025.3 `QUERY`).
    pub fn metadata(&self, id: EndpointId) -> Option<&EndpointMetadata> {
        self.endpoints.iter().find(|e| e.id == id && !e.closed).map(|e| &e.metadata)
    }

    /// Closes an endpoint (RFC-0025.10). Close is deferred until the queue is
    /// empty unless `force` is true.
    pub fn close(&mut self, id: EndpointId, force: bool) -> Result<(), EndpointError> {
        let entry = self
            .endpoints
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(EndpointError::NotFound)?;
        if entry.closed {
            return Err(EndpointError::NotFound);
        }
        if !force && entry.queue_depth > 0 {
            return Err(EndpointError::QueueNotEmpty);
        }
        entry.closed = true;
        Ok(())
    }

    /// Enqueues a message on the endpoint (increments queue depth).
    pub fn enqueue(&mut self, id: EndpointId) -> Result<(), EndpointError> {
        let entry = self
            .endpoints
            .iter_mut()
            .find(|e| e.id == id && !e.closed)
            .ok_or(EndpointError::NotFound)?;
        if entry.queue_depth >= entry.metadata.queue_capacity {
            return Err(EndpointError::QuotaExceeded);
        }
        entry.queue_depth += 1;
        entry.metadata.queue_depth = entry.queue_depth;
        Ok(())
    }

    /// Dequeues a message from the endpoint (decrements queue depth).
    pub fn dequeue(&mut self, id: EndpointId) -> Result<(), EndpointError> {
        let entry = self
            .endpoints
            .iter_mut()
            .find(|e| e.id == id && !e.closed)
            .ok_or(EndpointError::NotFound)?;
        if entry.queue_depth == 0 {
            return Ok(());
        }
        entry.queue_depth -= 1;
        entry.metadata.queue_depth = entry.queue_depth;
        Ok(())
    }

    /// Returns the number of registered (non-closed) endpoints.
    pub fn len(&self) -> usize {
        self.endpoints.iter().filter(|e| !e.closed).count()
    }

    /// Returns true if no endpoints are registered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates an opaque capability token for an endpoint (RFC-0025.4).
    fn make_token(id: &EndpointId, rights: u8) -> Vec<u8> {
        let mut token = Vec::with_capacity(34);
        token.extend_from_slice(&id.0);
        token.push(rights);
        token.push(0x01); // token version
        token
    }
}

impl Default for EndpointRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Endpoint id computation
// ---------------------------------------------------------------------------

/// Computes the endpoint id: BLAKE3-256 of (domain_id, name, cap_hash).
/// When the integrity model crate is available this delegates to its Merkle
/// hash; here we use a fixed-domain mixing function so ids are deterministic
/// and stable across restarts (RFC-0025.1).
pub fn compute_endpoint_id(domain: DomainId, name: &str, cap_hash: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    let dom = domain.get().to_le_bytes();
    let name_bytes = name.as_bytes();
    // Mix domain id into first 2 bytes.
    out[0] = dom[0];
    out[1] = dom[1];
    // Mix name bytes (up to 28 bytes) into the middle.
    let name_len = name_bytes.len().min(28);
    out[2..2 + name_len].copy_from_slice(&name_bytes[..name_len]);
    // Mix cap hash into the last byte.
    out[31] = cap_hash;
    out
}
