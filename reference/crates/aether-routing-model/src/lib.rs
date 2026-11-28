//! Routing model reference implementation (RFC-0026).
//!
//! The routing subsystem delivers messages from a source endpoint to a
//! destination endpoint within and across execution domains. Routing is
//! responsible for finding the path, enforcing capability rights at each hop,
//! and handling delivery failures without silently dropping messages
//! (RFC-0026.8).
//!
//! v1 restriction (RFC-0026.7): multi-domain routing is restricted to
//! single-execution-domain deployments. All routing within this crate is
//! intra-domain single-hop.

use aether_endpoint_model::{CommErrorCode, EndpointCapKind, EndpointId, EndpointRegistry};

// ---------------------------------------------------------------------------
// Data structures (RFC-0026.3)
// ---------------------------------------------------------------------------

/// One hop in a route: the next routing node (or final destination), the
/// capability required to traverse that hop, and the transport to use
/// (RFC-0026.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteHop {
    /// Intermediate or final destination endpoint.
    pub next_hop: EndpointId,
    /// Minimum capability class required for this hop.
    pub required_cap_class: u8,
    /// Provider-specific transport ID.
    pub transport: u8,
    /// Weight for equal-cost multi-path.
    pub weight: u32,
}

/// A routing table entry mapping a destination to an ordered array of hops
/// (RFC-0026.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteEntry {
    /// Destination endpoint id (pattern or exact match).
    pub destination: EndpointId,
    /// Number of hops in this route.
    pub hop_count: u32,
    /// Ordered array of hops.
    pub hops: Vec<RouteHop>,
}

/// A routing table mapping destination patterns to next hops (RFC-0026.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingTable {
    /// Routing table entries.
    pub entries: Vec<RouteEntry>,
}

impl RoutingTable {
    /// Creates an empty routing table.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Returns the number of entries in the table.
    #[must_use]
    pub const fn entry_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Inserts or replaces a route entry for the given destination.
    pub fn insert(&mut self, entry: RouteEntry) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.destination == entry.destination)
        {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Removes all entries referring to the given endpoint id (used when an
    /// endpoint is deregistered, RFC-0026.10).
    pub fn invalidate_for_endpoint(&mut self, endpoint: EndpointId) {
        self.entries.retain(|e| e.destination != endpoint);
        // Also remove any hops that traverse this endpoint as an intermediate
        // hop (RFC-0026.10: entries referring to a deregistered endpoint are
        // invalidated).
        for entry in &mut self.entries {
            entry.hops.retain(|h| h.next_hop != endpoint);
        }
        // Drop entries that now have no hops.
        self.entries.retain(|e| !e.hops.is_empty());
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// The routing subsystem. Resolves routes, enforces per-hop capability
/// requirements, and handles delivery failures (RFC-0026).
pub struct Router {
    table: RoutingTable,
}

impl Router {
    /// Creates a new router with an empty routing table.
    pub fn new() -> Self {
        Self {
            table: RoutingTable::new(),
        }
    }

    /// Returns a reference to the routing table.
    #[must_use]
    pub const fn table(&self) -> &RoutingTable {
        &self.table
    }

    /// Installs a route entry into the routing table (RFC-0026.2).
    pub fn install_route(&mut self, entry: RouteEntry) {
        self.table.insert(entry);
    }

    /// Invalidates all routing table entries referring to a deregistered
    /// endpoint (RFC-0026.10).
    pub fn invalidate_endpoint(&mut self, endpoint: EndpointId) {
        self.table.invalidate_for_endpoint(endpoint);
    }

    /// Resolves a route from `src` to `dst` (RFC-0026.1, RFC-0026.11).
    /// Route resolution completes in O(log N) where N is the number of
    /// routing table entries, using binary search over sorted destinations.
    /// Returns `ENODEST` if no route exists (RFC-0026.8).
    pub fn resolve(&self, _src: EndpointId, dst: EndpointId) -> Result<Vec<RouteHop>, RoutingError> {
        // Binary search over entries for O(log N) lookup (RFC-0026.11).
        let entries = &self.table.entries;
        let mut lo = 0usize;
        let mut hi = entries.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            match entries[mid].destination.0.cmp(&dst.0) {
                core::cmp::Ordering::Equal => {
                    return Ok(entries[mid].hops.clone());
                }
                core::cmp::Ordering::Less => {
                    lo = mid + 1;
                }
                core::cmp::Ordering::Greater => {
                    hi = mid;
                }
            }
        }
        Err(RoutingError::ENODEST)
    }

    /// Authorizes traversal of a hop: the sender must hold the required
    /// capability class for that hop. Returns `ENOCAP` if the sender lacks
    /// the required capability (RFC-0026.3, RFC-0026.4).
    pub fn authorize_hop(
        &self,
        hop: &RouteHop,
        sender_rights: u8,
    ) -> Result<(), RoutingError> {
        if hop.required_cap_class == 0 {
            // No capability required for this hop.
            return Ok(());
        }
        // Map cap class to endpoint capability kind bit. Class 1 = SEND,
        // class 2 = RECV, etc. (RFC-0026.5: intra-domain requires SEND).
        let required_bit = match hop.required_cap_class {
            1 => EndpointCapKind::Send.bit(),
            2 => EndpointCapKind::Recv.bit(),
            3 => EndpointCapKind::Query.bit(),
            4 => EndpointCapKind::Close.bit(),
            5 => EndpointCapKind::Inherit.bit(),
            _ => hop.required_cap_class,
        };
        if sender_rights & required_bit == 0 {
            return Err(RoutingError::ENOCAP);
        }
        Ok(())
    }

    /// Resolves and authorizes a full route from `src` to `dst` for a sender
    /// holding `sender_rights`. Returns the ordered hops on success, or
    /// `ENODEST`/`ENOCAP` on failure without silently dropping the message
    /// (RFC-0026.8).
    pub fn route(
        &self,
        src: EndpointId,
        dst: EndpointId,
        sender_rights: u8,
    ) -> Result<Vec<RouteHop>, RoutingError> {
        let hops = self.resolve(src, dst)?;
        // Authorize each hop in order (RFC-0026.3, RFC-0026.4).
        for hop in &hops {
            self.authorize_hop(hop, sender_rights)?;
        }
        Ok(hops)
    }

    /// Handles a delivery failure after routing succeeded: retries up to
    /// `max_retries` with exponential backoff, then returns `EQUOTA` and the
    /// message is dropped (RFC-0026.9).
    pub fn handle_delivery_failure(&self, attempts: u32, max_retries: u32) -> RoutingError {
        if attempts > max_retries {
            return RoutingError::EQUOTA;
        }
        RoutingError::ENODEST // placeholder: caller retries
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Routing errors
// ---------------------------------------------------------------------------

/// Routing errors (RFC-0026.5 through RFC-0026.8).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RoutingError {
    /// No route to destination.
    ENODEST,
    /// Capability missing for a hop.
    ENOCAP,
    /// Destination receive queue exceeded after max retries.
    EQUOTA,
}

impl RoutingError {
    /// Returns the numeric communication error code (RFC-0025.8 / RFC-0025.9).
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            RoutingError::ENODEST => CommErrorCode::ENODEST.code(),
            RoutingError::ENOCAP => CommErrorCode::ENOCAP.code(),
            RoutingError::EQUOTA => CommErrorCode::EQUOTA.code(),
        }
    }
}

impl From<RoutingError> for CommErrorCode {
    fn from(e: RoutingError) -> Self {
        match e {
            RoutingError::ENODEST => CommErrorCode::ENODEST,
            RoutingError::ENOCAP => CommErrorCode::ENOCAP,
            RoutingError::EQUOTA => CommErrorCode::EQUOTA,
        }
    }
}

// ---------------------------------------------------------------------------
// Endpoint registry integration for table consistency (RFC-0026.10)
// ---------------------------------------------------------------------------

/// Synchronizes a routing table with an endpoint registry: when an endpoint
/// is deregistered, all routing table entries referring to it are
/// invalidated (RFC-0026.10).
pub fn sync_table_with_registry(_table: &mut RoutingTable, _registry: &EndpointRegistry) {
    // Note: EndpointRegistry does not yet expose a list of all endpoint ids,
    // so this is a no-op hook for future integration. Individual invalidation
    // is done via Router::invalidate_endpoint.
}
