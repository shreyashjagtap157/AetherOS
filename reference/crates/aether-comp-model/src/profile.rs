//! Profile selection model (RFC-0005), bounded host reference.
//!
//! Profiles never relax architectural invariants (I-11); they select
//! mechanism subsets and declare conformance floors. This module models the
//! decision half: mechanism activation, forbidden-union composition, and
//! tier-floor rejection. It does not activate anything in a running system,
//! does not execute conformance tests, and makes no claim of kernel or boot
//! integration.
//!
//! Declared semantics:
//!
//! - `mandatory` mechanisms of a declaration are always activated.
//! - `declared_optional` mechanisms listed in the declaration are activated.
//! - Optional **layered extensions** are activated only by explicit opt-in
//!   (the declaration's `extensions` list); defaults activate nothing extra
//!   (RFC-0005.5).
//! - Layered extensions or optional mechanisms that the declaration marks
//!   `forbidden` are rejected even if explicitly opted in (RFC-0005.6, most
//!   restrictive profile wins).
//! - A manifest whose overall tier is weaker than the declaration's
//!   `minimum_tier` is rejected (RFC-0005.3/.7).
//! - Combination of multiple profiles: mandatory and declared optional sets
//!   are unioned; forbidden sets are unioned; the strongest tier floor
//!   applies; any mandatory mechanism forbidden by *any* combined profile is
//!   dropped from activation with a reasoning record (most restrictive wins,
//!   RFC-0005.6). Cross-profile opt-in (declared optional / extension)
//!   overlaps with another profile's forbidden set are likewise suppressed,
//!   not rejected; self-conflicts (one declaration's own mandatory, optional,
//!   or extension set overlapping its own forbidden set) are rejected with
//!   typed errors.
//! - Mechanism names are opaque strings; the model does not enumerate the
//!   official mechanism catalog (`Profiles-Specification.md`) and treats
//!   names lexically, which is sufficient for activation, conflict, and
//!   audit computation.
//! - Events are not emitted (RFC-0005.9 is out of scope); the decision
//!   carries its reasoning vector instead.

use crate::{ComponentId, Profile, Tier};
use std::collections::BTreeSet;

/// An opaque mechanism name, as used by profile declarations.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Mechanism(String);

impl Mechanism {
    /// Creates a mechanism from a non-empty name.
    ///
    /// # Errors
    /// Returns [`ProfileError::EmptyMechanism`] for an empty name.
    pub fn new(name: &str) -> Result<Self, ProfileError> {
        if name.trim().is_empty() {
            return Err(ProfileError::EmptyMechanism);
        }
        Ok(Self(name.to_string()))
    }
}

/// A deployment's profile declaration (RFC-0005.1/.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileDeclaration {
    /// The declared profile (RFC-0005.1: at least one must be declared).
    pub profile: Profile,
    /// Mechanisms that must be active for the profile.
    pub mandatory: Vec<Mechanism>,
    /// Optional mechanisms the declaration explicitly enables.
    pub declared_optional: Vec<Mechanism>,
    /// Mechanisms forbidden under the declaration.
    pub forbidden: Vec<Mechanism>,
    /// Layered extensions the deployment explicitly opts in to (RFC-0005.5).
    pub extensions: Vec<Mechanism>,
    /// Weakest tier a claiming component may occupy (RFC-0005.3/.7).
    pub minimum_tier: Tier,
}

/// Why profile selection failed (RFC-0005.4/.6/.7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileError {
    /// A mechanism name was empty.
    EmptyMechanism,
    /// The declared mandatory set is empty; a profile must mandate something.
    EmptyMandatory,
    /// No profile declarations were provided where at least one is required.
    EmptyDeclarations,
    /// A declared optional mechanism or extension is also forbidden by the declaration.
    ForbiddenDeclared(Mechanism),
    /// A mandatory mechanism is also forbidden by the same declaration (RFC-0005.6).
    ForbiddenMandatory(Mechanism),
    /// RFC-0005.7: the manifest tier is weaker than the declaration's floor.
    TierConflict {
        /// Component whose tier claim was rejected.
        component: ComponentId,
        /// The manifest's declared tier.
        declared: Tier,
        /// The declaration's weakest acceptable tier.
        minimum: Tier,
    },
}

/// A profile-activation decision (RFC-0005.3 output, §3.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileDecision {
    /// Profiles that produced this decision.
    pub profiles: Vec<Profile>,
    /// Mechanisms activated, sorted and deduplicated.
    pub activated: Vec<Mechanism>,
    /// Mechanisms that would have been activated but were forbidden by a
    /// combined declaration; each carries a reasoning record (RFC-0005.6).
    pub suppressed: Vec<Mechanism>,
    /// Human-auditable reasoning, sorted and deduplicated.
    pub reasoning: Vec<String>,
}

fn collect_set(list: &[Mechanism]) -> BTreeSet<Mechanism> {
    list.iter().cloned().collect()
}

fn activate(declaration: &ProfileDeclaration) -> BTreeSet<Mechanism> {
    let mut activated = collect_set(&declaration.mandatory);
    activated.extend(collect_set(&declaration.declared_optional));
    activated.extend(collect_set(&declaration.extensions));
    activated
}

/// Applies one profile declaration to a component manifest (RFC-0005.3).
///
/// # Errors
/// Returns [`ProfileError::TierConflict`] when the manifest's tier is weaker
/// than the declaration's floor, [`ProfileError::ForbiddenDeclared`] when the
/// declaration opted into a mechanism it also forbids, or
/// [`ProfileError::EmptyMandatory`] for an empty mandatory set, or
/// [`ProfileError::ForbiddenMandatory`] for a mandatory set overlapping `forbidden`.
pub fn apply_profile(
    declaration: &ProfileDeclaration,
    manifest: &crate::Manifest,
) -> Result<ProfileDecision, ProfileError> {
    let forbidden = collect_set(&declaration.forbidden);
    if declaration.mandatory.is_empty() {
        return Err(ProfileError::EmptyMandatory);
    }
    for declared in declaration
        .declared_optional
        .iter()
        .chain(&declaration.extensions)
    {
        if forbidden.contains(declared) {
            return Err(ProfileError::ForbiddenDeclared(declared.clone()));
        }
    }
    for declared in &declaration.mandatory {
        if forbidden.contains(declared) {
            return Err(ProfileError::ForbiddenMandatory(declared.clone()));
        }
    }
    if manifest.tier > declaration.minimum_tier {
        return Err(ProfileError::TierConflict {
            component: manifest.id,
            declared: manifest.tier,
            minimum: declaration.minimum_tier,
        });
    }
    let activated = activate(declaration);
    let activated_len = activated.len();
    Ok(ProfileDecision {
        profiles: vec![declaration.profile],
        activated: activated.into_iter().collect(),
        suppressed: Vec::new(),
        reasoning: vec![format!(
            "profile {:?} accepted component {:?}: {activated_len} mechanism(s) activated",
            declaration.profile, manifest.id,
        )],
    })
}

/// Combines several declarations into one decision (RFC-0005.6).
///
/// Forbidden sets are unioned (most restrictive wins); a mandatory or
/// declared optional mechanism forbidden by any declaration is suppressed
/// rather than activated, with a reasoning record. The strongest tier floor
/// across declarations applies: every profile in the combination rejects
/// manifests weaker than its own floor, and combination succeeds only for a
/// manifest satisfying the strictest floor.
///
/// # Errors
/// Returns [`ProfileError::EmptyDeclarations`] when given no declarations;
/// [`ProfileError::TierConflict`] when the manifest fails the
/// strictest floor; [`ProfileError::ForbiddenDeclared`] when a declaration
/// opts into a mechanism the same declaration forbids;
/// [`ProfileError::ForbiddenMandatory`] when a declaration's mandatory set
/// overlaps its own `forbidden` set; [`ProfileError::EmptyMandatory`] when a
/// declaration's mandatory set is empty.
pub fn combine_profiles(
    declarations: &[ProfileDeclaration],
    manifest: &crate::Manifest,
) -> Result<ProfileDecision, ProfileError> {
    if declarations.is_empty() {
        return Err(ProfileError::EmptyDeclarations);
    }
    // Strictest tier floor (lowest ordinal A < B < C < D) applies.
    let minimum = declarations
        .iter()
        .map(|declaration| declaration.minimum_tier)
        .min();
    let forbidden: BTreeSet<Mechanism> = declarations
        .iter()
        .flat_map(|declaration| declaration.forbidden.iter().cloned())
        .collect();

    for declaration in declarations {
        if declaration.mandatory.is_empty() {
            return Err(ProfileError::EmptyMandatory);
        }
        for declared in declaration
            .declared_optional
            .iter()
            .chain(&declaration.extensions)
        {
            // Self-conflict within one declaration is rejected; cross-profile
            // conflicts are suppressed downstream (most restrictive wins).
            if declaration.forbidden.contains(declared) {
                return Err(ProfileError::ForbiddenDeclared(declared.clone()));
            }
        }
        for declared in &declaration.mandatory {
            if declaration.forbidden.contains(declared) {
                return Err(ProfileError::ForbiddenMandatory(declared.clone()));
            }
        }
        if manifest.tier > declaration.minimum_tier {
            return Err(ProfileError::TierConflict {
                component: manifest.id,
                declared: manifest.tier,
                minimum: declaration.minimum_tier,
            });
        }
    }

    let mut activated = BTreeSet::new();
    let mut suppressed = BTreeSet::new();
    let mut reasoning = Vec::new();
    let mut profiles = Vec::new();
    for declaration in declarations {
        profiles.push(declaration.profile);
        for mechanism in activate(declaration) {
            if forbidden.contains(&mechanism) {
                suppressed.insert(mechanism.clone());
                reasoning.push(format!(
                    "mechanism {:?} (mandatory or declared opt-in) from {:?} is forbidden by a combined profile; suppressed (RFC-0005.6)",
                    mechanism, declaration.profile
                ));
            } else {
                activated.insert(mechanism);
            }
        }
    }
    profiles.sort();
    profiles.dedup();
    if let Some(floor) = minimum {
        reasoning.push(format!(
            "combined profile floor is {floor:?}; manifest tier {:?} accepted",
            manifest.tier
        ));
    }
    // Deduplicated and sorted: reasoning is an audit surface, not a trace.
    reasoning.sort();
    reasoning.dedup();

    Ok(ProfileDecision {
        profiles,
        activated: activated.into_iter().collect(),
        suppressed: suppressed.into_iter().collect(),
        reasoning,
    })
}
