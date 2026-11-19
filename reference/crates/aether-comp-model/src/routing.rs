//! Capability routing decision model (RFC-0003 §3), bounded host reference.
//!
//! This module implements the *decision* half of RFC-0003: given a grantor's
//! manifest (its `provides` declarations with narrowing policy and rights
//! ceiling), the requesting component's manifest, the grantor's presented
//! capability authority, and a request, it selects the attenuated authority to
//! grant or denies with the RFC-0003.10 reason codes. It does **not** mint
//! canonical RFC-0037 tokens (no 96-byte layout, no seal), does not sign or
//! serialize, and is not invoked by any Composition Root.
//!
//! Declared semantics:
//!
//! - RFC-0003.3/.4: the grantor can grant only what it holds. A grant's
//!   declared ceiling must be fully contained in the grantor authority's
//!   rights (all-or-nothing containment, not intersection); a partially
//!   held ceiling is not derivable (`GrantNotDerivable`).
//! - RFC-0003.1 narrowing policies:
//!   - `Attenuate`: the granted rights are exactly the requested rights, and
//!     every requested right must be covered by the effective ceiling,
//!     otherwise `NarrowingFailed` (no partial grants).
//!   - `Exact`: the request must match the effective ceiling exactly;
//!     otherwise `NarrowingFailed`.
//!   - A `None` ceiling is a declaration, not authority: it grants nothing
//!     (`NarrowingFailed`).
//! - A request for zero rights is denied (`NarrowingFailed`) under both
//!   policies. This is deliberate: a Granted-but-empty authority is never
//!   useful, and denying it is fail-closed and in line with the "a grant
//!   without a ceiling is a declaration, not authority" rule.
//! - RFC-0003.7 (environmental independence): the decision is a pure function
//!   of the four inputs; there is no ambient input. Same inputs produce the
//!   same decision on every platform, every run.
//! - The granted authority inherits the grantor authority's resource,
//!   type, bounds, flags and expiry (a grantee never outlives its grantor);
//!   minting a live token from this decision is out of scope here.
//! - Revocation propagation (RFC-0003.9/.11) is exercised and tested at the
//!   capability-provider layer (`aether-cap-model` / `aether-capspace`), not
//!   here; see the revocation tests in both crates and `CTS-CAP-REVOKE-*`.

use crate::{Manifest, NarrowingPolicy, validate_manifest};
use aether_types::{Capability, Rights};

/// A capability request (RFC-0003.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingRequest {
    /// Capability name as declared in the grantor's `provides` list.
    pub capability: String,
    /// Rights the requestor asks for.
    pub requested_rights: Rights,
}

/// Why routing denied the request (RFC-0003.10 reason codes).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DenialReason {
    /// A presented manifest failed validation (RFC-0002.19).
    ManifestInvalid,
    /// RFC-0003.3/.4: no grant can derive the requested capability —
    /// no matching `provides` entry, or the grantor's authority does
    /// not cover the declared ceiling.
    GrantNotDerivable,
    /// RFC-0003.4/.10: narrowing failed — the requested rights are not a
    /// subset of the effective ceiling (or, for `Exact`, do not match it).
    NarrowingFailed,
}

/// Outcome of a routing decision (RFC-0003.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutingOutcome {
    /// Granted with the attenuated capability authority.
    Granted(Capability),
    /// Denied with the reason code.
    Denied(DenialReason),
}

/// Computes the routing decision for one `(grantor, requestor, authority, request)` tuple.
///
/// The decision is pure: no I/O, no clocks, no global state (RFC-0003.7).
///
/// # Errors
/// None — denial is a value, not an error; API misuse is out of scope.
#[must_use]
pub fn route(
    grantor: &Manifest,
    requestor: &Manifest,
    authority: &Capability,
    request: &RoutingRequest,
) -> RoutingOutcome {
    if validate_manifest(grantor).is_err() || validate_manifest(requestor).is_err() {
        return RoutingOutcome::Denied(DenialReason::ManifestInvalid);
    }
    let Some(grant) = grantor
        .provides
        .iter()
        .find(|grant| grant.name == request.capability)
    else {
        return RoutingOutcome::Denied(DenialReason::GrantNotDerivable);
    };
    let ceiling = grant.rights_ceiling.unwrap_or(Rights::NONE);
    if !authority.rights.contains(ceiling) {
        // RFC-0003.4: the grantor cannot grant what it does not hold.
        return RoutingOutcome::Denied(DenialReason::GrantNotDerivable);
    }
    let effective = ceiling;
    let (granted, allowed) = match grant.narrowing {
        NarrowingPolicy::Attenuate => {
            let granted = request.requested_rights;
            (granted, effective.contains(granted))
        }
        NarrowingPolicy::Exact => {
            let granted = effective;
            (granted, granted == request.requested_rights)
        }
    };
    if !allowed || granted == Rights::NONE {
        return RoutingOutcome::Denied(DenialReason::NarrowingFailed);
    }
    RoutingOutcome::Granted(Capability {
        rights: granted,
        object: authority.object,
        object_type: authority.object_type,
        bounds: authority.bounds,
        flags: authority.flags,
        expires_at_ns: authority.expires_at_ns,
    })
}
