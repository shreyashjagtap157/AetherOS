//! Bounded executable host model of `AetherOS` component manifests and
//! dependency resolution (RFC-0002, RFC-0004; RFC-0005 profile inputs).
//!
//! Scope, deliberately bounded:
//!
//! - **Class of model:** single-domain, host-reference, deterministic
//!   evaluation of manifest validation and dependency resolution. It is not
//!   the Composition Root, not a package manager, and not an implementation
//!   of RFC-0002 canonical serialization.
//! - **Content addressing:** `component-id` is modeled as a host `u64`
//!   newtype rather than RFC-0002.1's SHA-3-512-of-canonical-representation;
//!   real content resolution requires a cryptographic hash, which this
//!   repository's dependency-free workspace deliberately does not carry.
//! - **Strategy:** exactly one provider-selection strategy is declared,
//!   `HighestVersion` (RFC-0004.3). Pinned-via-override and lowest-version
//!   are not implemented. Ties on `(component-id, version)` fail closed with
//!   [`ResolutionError::AmbiguousProvider`].
//! - **Profiles (RFC-0005):** a requirement is compatible with a provider
//!   only when the provider declares at least one profile in common with
//!   its **dependent**; a provider whose tier is weaker than its dependent's
//!   tier is rejected. Mechanism activation decisions are modeled separately
//!   in the `profile` module (RFC-0005); dependency resolution itself never
//!   activates anything.
//! - **Topology:** transitive resolution with cycle detection (Kahn). Cycle
//!   members are reported sorted; no component is returned when a cycle
//!   exists (RFC-0004.5/.6).
//! - **Conservative update behaviour (RFC-0004.8) and instance semantics
//!   (RFC-0004.10)** are documented but not implemented; instantiation is
//!   a pure function of (target, candidate pool) and has no side effects.
//!
//! Failure precedence is `version -> profile -> tier` (RFC-0004.4 refusal of
//! invalid candidates precedes all three). When several constraints fail for
//! one requirement, only the first reason code is reported; the codes are not
//! mutually exclusive in general. A requirement naming the target's own id is
//! not a manifest error: it resolves to a released candidate of that id, or
//! closes a dependency cycle (reported with the cycle's members, sorted).
//! The candidate pool contract is `pool key == manifest.id` for every entry;
//! a violation fails closed with `ResolutionError::InconsistentCandidatePool`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Composition conformance declaration model (RFC-0006).
pub mod conformance;
/// Deployment profile selection model (RFC-0005).
pub mod profile;
/// Capability routing decision model (RFC-0003).
pub mod routing;

/// Host-model component identity (RFC-0002.1 placeholder; see module docs).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ComponentId(pub u64);

/// Verification tier of a component (RFC-0040), strongest first.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Tier {
    /// Tier A - strongest verification expectations.
    A,
    /// Tier B.
    B,
    /// Tier C.
    C,
    /// Tier D - weakest.
    D,
}

/// Deployment profiles (`Profiles-Specification.md`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Profile {
    /// Embedded profile.
    Embedded,
    /// Desktop profile.
    Desktop,
    /// Cloud profile.
    Cloud,
    /// HPC profile.
    HPC,
    /// Safety-Critical profile.
    SafetyCritical,
}

/// Narrowing policy for a provided capability grant (RFC-0002.6, RFC-0003).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NarrowingPolicy {
    /// Grant only narrowed rights relative to the grantor's own grant.
    Attenuate,
    /// Grant with the exact rights the grantor holds (no attenuation).
    Exact,
}

/// A dependency declaration (RFC-0002.5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    /// The provider component that must be resolved.
    pub component: ComponentId,
    /// Minimum accepted provider version (inclusive).
    pub min_version: u64,
    /// Maximum accepted provider version (inclusive).
    pub max_version: u64,
}

/// A capability grant declaration (RFC-0002.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grant {
    /// Capability name (non-empty; syntax checked at ingress).
    pub name: String,
    /// Narrowing policy applied at grant time.
    pub narrowing: NarrowingPolicy,
    /// Rights ceiling for the grant (RFC-0003.1 narrowing bound). `None`
    /// grants no rights; a grant without a ceiling is a declaration, not
    /// authority.
    pub rights_ceiling: Option<aether_types::Rights>,
}

/// Component manifest (RFC-0002), host-model subset of the full contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    /// Content-addressed identity (host `u64` newtype in this model).
    pub id: ComponentId,
    /// Manifest schema version; this model validates version 1 only.
    pub schema_version: u32,
    /// Component version, resolved against `Requirement` ranges.
    pub version: u64,
    /// Declared dependencies (RFC-0002.5).
    pub requires: Vec<Requirement>,
    /// Declared grants to dependents (RFC-0002.6).
    pub provides: Vec<Grant>,
    /// Profiles under which this component may operate (RFC-0002.14).
    pub profiles: Vec<Profile>,
    /// Overall verification tier (RFC-0002.15, RFC-0040).
    pub tier: Tier,
    /// Maximum concurrent instances; must be >= 1 (RFC-0002.13).
    pub max_instances: u32,
    /// Rollback policy declaration (RFC-0002.17).
    pub rollback_on_failure: bool,
    /// Lifecycle entrypoint (relative address); must be non-empty (RFC-0002.11).
    pub start: String,
    /// Graceful shutdown address; must be non-empty (RFC-0002.12).
    pub stop: String,
}

/// Manifest validation failure (RFC-0002.19).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestError {
    /// RFC-0002.1 host convention: component id must be non-zero.
    ZeroComponentId,
    /// RFC-0002.2: only schema version 1 is supported by this model.
    UnsupportedSchemaVersion(u32),
    /// RFC-0002.13: `max-instances` must be >= 1.
    InvalidMaxInstances,
    /// RFC-0002.14: at least one profile must be declared.
    NoProfiles,
    /// RFC-0002.11/.12: lifecycle addresses must be non-empty.
    EmptyLifecycleAddress,
    /// RFC-0002.5: requirement id must be non-zero and `min_version <= max_version`.
    InvalidRequirement,
    /// RFC-0002.6: a grant must have a non-empty capability name.
    InvalidGrant,
}

/// Why a dependency could not be resolved (RFC-0004.6 reason codes).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolutionError {
    /// The target manifest itself is invalid (RFC-0002.19).
    InvalidTargetManifest(ManifestError),
    /// RFC-0004.6 / RFC-0002.5: no candidate carries the required id.
    MissingProvider(ComponentId),
    /// RFC-0004.6: id matched but no version overlaps the declared range.
    VersionMismatch(ComponentId),
    /// RFC-0004.6: version matched but no profile is shared with the dependent.
    ProfileMismatch(ComponentId),
    /// RFC-0004.6: profile matched but the provider tier is weaker than the dependent's.
    TierMismatch(ComponentId),
    /// RFC-0004.6 / RFC-0004.5: a dependency cycle exists; members are sorted.
    Cycle(Vec<ComponentId>),
    /// Same provider id and version declared by two distinct candidates.
    AmbiguousProvider(ComponentId),
    /// A candidate manifest's `id` does not equal the key under which the
    /// pool filed it (fail closed; the key and the manifest must agree).
    InconsistentCandidatePool(ComponentId),
}

/// Why a provider was selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionReason {
    /// Sole valid candidate in the pool for the required id, decision-free.
    SoleCandidate,
    /// RFC-0004.3 `HighestVersion` strategy selected the highest eligible version.
    HighestVersion,
}

/// One selection event, mirroring the audit content of RFC-0004.7.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolutionEvent {
    /// The requirement's component id.
    pub component: ComponentId,
    /// The selected provider's id (always equal to `component` in this model).
    pub provider: ComponentId,
    /// Version of the selected provider.
    pub version: u64,
    /// Why it was selected.
    pub reason: SelectionReason,
}

/// Result of a successful resolution (RFC-0004).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    /// Instantiation order: dependencies before dependents, target last.
    /// When the target shares its id with a selected provider, both instances
    /// appear; the provider (a distinct released instance) appears first.
    pub order: Vec<ComponentId>,
    /// Selection events, one per **unique** selected provider (memoized;
    /// repeat consumers of the same requirement add no events). Discarded
    /// entirely when resolution fails.
    pub events: Vec<ResolutionEvent>,
}

/// Declared provider-selection strategy (RFC-0004.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionStrategy {
    /// Highest provider version wins (default, conformance-declared).
    HighestVersion,
}

/// Validates a manifest against the RFC-0002.1–18 contract subset.
///
/// # Errors
/// Returns the precise typed failure; an invalid manifest is never usable
/// for resolution or instantiation (RFC-0002.19).
pub fn validate_manifest(manifest: &Manifest) -> Result<(), ManifestError> {
    if manifest.id.0 == 0 {
        return Err(ManifestError::ZeroComponentId);
    }
    if manifest.schema_version != 1 {
        return Err(ManifestError::UnsupportedSchemaVersion(
            manifest.schema_version,
        ));
    }
    if manifest.max_instances == 0 {
        return Err(ManifestError::InvalidMaxInstances);
    }
    if manifest.profiles.is_empty() {
        return Err(ManifestError::NoProfiles);
    }
    if manifest.start.trim().is_empty() || manifest.stop.trim().is_empty() {
        return Err(ManifestError::EmptyLifecycleAddress);
    }
    for requirement in &manifest.requires {
        if requirement.component.0 == 0 || requirement.min_version > requirement.max_version {
            return Err(ManifestError::InvalidRequirement);
        }
    }
    for grant in &manifest.provides {
        if grant.name.trim().is_empty() {
            return Err(ManifestError::InvalidGrant);
        }
    }
    Ok(())
}

/// The resolved graph has one node per selected provider plus the target
/// itself; both may legitimately share a component id (a component may depend
/// on a released older provider of its own id), so they are distinct nodes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Node {
    /// The resolution root.
    Target,
    /// A selected provider, keyed by its component id.
    Provider(ComponentId),
}

impl Node {
    fn id(self, target_id: ComponentId) -> ComponentId {
        match self {
            Self::Target => target_id,
            Self::Provider(id) => id,
        }
    }
}

/// Finds one cycle deterministically via DFS over the residual dependency
/// edges. Members (the back-edge slice) are reported sorted. The start node
/// is the target, whose subgraph covers every selected node.
fn find_cycle(edges: &BTreeMap<Node, Vec<Node>>, target_id: ComponentId) -> Vec<ComponentId> {
    const WHITE: u8 = 0;
    const GRAY: u8 = 1;
    const BLACK: u8 = 2;

    fn dfs(
        node: Node,
        color: &mut BTreeMap<Node, u8>,
        stack: &mut Vec<Node>,
        edges: &BTreeMap<Node, Vec<Node>>,
        target_id: ComponentId,
    ) -> Option<Vec<ComponentId>> {
        color.insert(node, GRAY);
        stack.push(node);
        for &next in edges.get(&node).into_iter().flatten() {
            match color.get(&next).copied().unwrap_or(WHITE) {
                WHITE => {
                    if let Some(cycle) = dfs(next, color, stack, edges, target_id) {
                        return Some(cycle);
                    }
                }
                GRAY => {
                    let start = stack.iter().position(|&member| member == next)?;
                    let mut members: Vec<ComponentId> = stack[start..]
                        .iter()
                        .map(|member| member.id(target_id))
                        .collect();
                    members.sort();
                    members.dedup();
                    return Some(members);
                }
                _ => {}
            }
        }
        stack.pop();
        color.insert(node, BLACK);
        None
    }

    let mut color = BTreeMap::new();
    let mut stack = Vec::new();
    dfs(Node::Target, &mut color, &mut stack, edges, target_id).unwrap_or_default()
}

/// Resolves the dependency graph for `target` against a candidate pool.
///
/// The pool maps provider id to its manifest. Candidates that fail
/// validation are refused for selection (RFC-0004.4) and, when every
/// candidate for a requirement is invalid or absent, resolution halts for
/// that requirement with the reason code from RFC-0004.6.
///
/// Instantiation order is dependencies before dependents; the target is
/// last. Cycles are reported with sorted members and no order is returned
/// (RFC-0004.5).
///
/// # Errors
/// Returns the precise RFC-0004.6 reason code for failure.
pub fn resolve<'m>(
    target: &'m Manifest,
    candidates: &'m BTreeMap<ComponentId, Vec<Manifest>>,
    strategy: SelectionStrategy,
) -> Result<Resolution, ResolutionError> {
    validate_manifest(target).map_err(ResolutionError::InvalidTargetManifest)?;

    // Pool contract: the candidate's manifest id must equal the map key.
    for (id, manifests) in candidates {
        for manifest in manifests {
            if &manifest.id != id {
                return Err(ResolutionError::InconsistentCandidatePool(*id));
            }
        }
    }

    // Admit only valid candidates into the selectable pool (RFC-0004.4).
    let pool: BTreeMap<ComponentId, Vec<&Manifest>> = candidates
        .iter()
        .map(|(id, manifests)| {
            (
                *id,
                manifests
                    .iter()
                    .filter(|manifest| validate_manifest(manifest).is_ok())
                    .collect(),
            )
        })
        .collect();

    // BFS over the transitive requirement closure; memoized per provider id.
    let mut selected: BTreeMap<ComponentId, &Manifest> = BTreeMap::new();
    let mut events: Vec<ResolutionEvent> = Vec::new();
    let mut frontier: VecDeque<&Manifest> = VecDeque::from([target]);
    while let Some(node) = frontier.pop_front() {
        for requirement in &node.requires {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                selected.entry(requirement.component)
            {
                let chosen = select_provider(requirement, node, &pool, strategy)?;
                entry.insert(chosen);
                events.push(resolved_event(requirement, chosen, strategy, &pool));
                frontier.push_back(chosen);
            }
        }
    }

    // Topological order (Kahn) over the selected graph plus the target.
    // The target and a provider may share a component id (the provider is a
    // distinct instance), so nodes are keyed by kind, not id.
    let mut in_degree: BTreeMap<Node, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<Node, Vec<Node>> = BTreeMap::new();
    let mut edges: BTreeMap<Node, Vec<Node>> = BTreeMap::new();
    in_degree.insert(Node::Target, target.requires.len());
    edges.insert(
        Node::Target,
        target
            .requires
            .iter()
            .map(|requirement| Node::Provider(requirement.component))
            .collect(),
    );
    for requirement in &target.requires {
        dependents
            .entry(Node::Provider(requirement.component))
            .or_default()
            .push(Node::Target);
    }
    for (id, chosen) in &selected {
        in_degree.insert(Node::Provider(*id), chosen.requires.len());
        edges.insert(
            Node::Provider(*id),
            chosen
                .requires
                .iter()
                .map(|requirement| Node::Provider(requirement.component))
                .collect(),
        );
        for requirement in &chosen.requires {
            dependents
                .entry(Node::Provider(requirement.component))
                .or_default()
                .push(Node::Provider(*id));
        }
    }

    let mut ready: BTreeSet<Node> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| *node)
        .collect();
    let mut order = Vec::with_capacity(in_degree.len());
    let mut processed = 0_usize;
    while let Some(node) = ready.pop_first() {
        order.push(node.id(target.id));
        processed += 1;
        if let Some(consumers) = dependents.get(&node) {
            for consumer in consumers {
                if let Some(degree) = in_degree.get_mut(consumer) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(*consumer);
                    }
                }
            }
        }
    }
    if processed != in_degree.len() {
        return Err(ResolutionError::Cycle(find_cycle(&edges, target.id)));
    }

    Ok(Resolution { order, events })
}

fn select_provider<'m>(
    requirement: &Requirement,
    dependent: &Manifest,
    pool: &BTreeMap<ComponentId, Vec<&'m Manifest>>,
    strategy: SelectionStrategy,
) -> Result<&'m Manifest, ResolutionError> {
    let candidates = pool
        .get(&requirement.component)
        .map_or(&[][..], Vec::as_slice);
    if candidates.is_empty() {
        return Err(ResolutionError::MissingProvider(requirement.component));
    }
    let overlapping: Vec<&'m Manifest> = candidates
        .iter()
        .filter(|manifest| {
            manifest.version >= requirement.min_version
                && manifest.version <= requirement.max_version
        })
        .copied()
        .collect();
    if overlapping.is_empty() {
        return Err(ResolutionError::VersionMismatch(requirement.component));
    }
    let profiled: Vec<&'m Manifest> = overlapping
        .iter()
        .filter(|manifest| shares_profile(manifest, dependent))
        .copied()
        .collect();
    if profiled.is_empty() {
        return Err(ResolutionError::ProfileMismatch(requirement.component));
    }
    let strong_enough: Vec<&'m Manifest> = profiled
        .iter()
        .filter(|manifest| manifest.tier <= dependent.tier)
        .copied()
        .collect();
    if strong_enough.is_empty() {
        return Err(ResolutionError::TierMismatch(requirement.component));
    }
    match strategy {
        SelectionStrategy::HighestVersion => {
            let mut sorted = strong_enough.clone();
            sorted.sort_by_key(|manifest| std::cmp::Reverse(manifest.version));
            let first = sorted[0];
            if sorted.len() > 1 && sorted[1].version == first.version {
                return Err(ResolutionError::AmbiguousProvider(requirement.component));
            }
            Ok(first)
        }
    }
}

fn shares_profile(provider: &Manifest, dependent: &Manifest) -> bool {
    provider
        .profiles
        .iter()
        .any(|profile| dependent.profiles.contains(profile))
}

fn resolved_event(
    requirement: &Requirement,
    chosen: &Manifest,
    strategy: SelectionStrategy,
    pool: &BTreeMap<ComponentId, Vec<&Manifest>>,
) -> ResolutionEvent {
    let reason = match pool.get(&requirement.component).map_or(0, Vec::len) {
        1 => SelectionReason::SoleCandidate,
        _ => match strategy {
            SelectionStrategy::HighestVersion => SelectionReason::HighestVersion,
        },
    };
    ResolutionEvent {
        component: requirement.component,
        provider: chosen.id,
        version: chosen.version,
        reason,
    }
}

#[cfg(test)]
mod conformance_tests {
    use super::conformance::{
        ArtifactId, CORE_COMPOSITION_TESTS, ConformanceError, DeclarationInput, FailureMode,
        FixtureResult, OmittedTest, Substitution, declare_composition,
    };
    use super::*;

    const GOLDEN_ARTIFACT: u64 = 15_100_182_307_975_615_485;
    const GOLDEN_FAIL: u64 = 1_964_222_086_005_321_787;
    const GOLDEN_SUB: u64 = 16_645_544_297_799_217_052;
    const GOLDEN_OMIT: u64 = 8_022_494_014_290_163_001;
    const GOLDEN_PROF: u64 = 18_232_138_673_669_227_618;

    fn pass_all_core() -> Vec<FixtureResult> {
        CORE_COMPOSITION_TESTS
            .iter()
            .map(|fixture| FixtureResult::pass(fixture))
            .collect()
    }

    fn base_input() -> DeclarationInput {
        DeclarationInput {
            cts_version: "cts-0.1.0".to_string(),
            implementation: "aether-ref-0.1.0".to_string(),
            profile: None,
            core_results: pass_all_core(),
            profile_subset: Vec::new(),
            substitutions: Vec::new(),
            omitted: Vec::new(),
            manifests_tested: Vec::new(),
            tier_claims: Vec::new(),
        }
    }

    #[test]
    fn full_passing_core_produces_a_conformant_declaration() {
        let declaration = declare_composition(base_input()).unwrap();
        assert!(declaration.conformant);
        assert!(declaration.gaps.is_empty());
        assert_eq!(declaration.core_results.len(), CORE_COMPOSITION_TESTS.len());
        // Deterministic artifact for identical inputs.
        let second = declare_composition(base_input()).unwrap();
        assert_eq!(declaration.artifact, second.artifact);
    }

    #[test]
    fn missing_core_fixture_is_rejected() {
        let mut input = base_input();
        input.core_results.pop();
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::MissingCoreTest("CTS-COMP-PROF-4"))
        );
    }

    #[test]
    fn failure_must_be_classified_and_is_then_reported_as_a_gap() {
        let mut input = base_input();
        *input
            .core_results
            .iter_mut()
            .find(|result| result.fixture == "CTS-COMP-RT-2")
            .unwrap() = FixtureResult::fail("CTS-COMP-RT-2", FailureMode::ImplementationBug);
        let declaration = declare_composition(input).unwrap();
        assert!(!declaration.conformant);
        assert_eq!(declaration.gaps.len(), 1);
        assert!(declaration.gaps[0].contains("CTS-COMP-RT-2"));

        // Unclassified failure is rejected outright.
        let mut unclassified = base_input();
        let failing = FixtureResult {
            fixture: "CTS-COMP-RT-2",
            passed: false,
            failure: None,
        };
        let index = unclassified
            .core_results
            .iter()
            .position(|result| result.fixture == "CTS-COMP-RT-2")
            .unwrap();
        unclassified.core_results[index] = failing;
        assert_eq!(
            declare_composition(unclassified),
            Err(ConformanceError::FailureNotClassified("CTS-COMP-RT-2"))
        );
    }

    #[test]
    fn passing_fixture_with_a_failure_mode_is_rejected() {
        let mut input = base_input();
        *input
            .core_results
            .iter_mut()
            .find(|result| result.fixture == "CTS-COMP-MAN-1")
            .unwrap() = FixtureResult {
            fixture: "CTS-COMP-MAN-1",
            passed: true,
            failure: Some(FailureMode::TestBug),
        };
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::InconsistentFailureMode("CTS-COMP-MAN-1"))
        );
    }

    #[test]
    fn substitution_must_be_executed_and_covers_the_core_fixture() {
        let mut input = base_input();
        // Remove the core fixture and omit it with justification; substitute it.
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-RT-1");
        input.substitutions.push(Substitution {
            test: "CTS-COMP-RT-1",
            substitute: "CTS-PROF-RT-1",
            justification: "host substitute with stricter checks".to_string(),
        });
        // Substitute not executed: rejected.
        assert_eq!(
            declare_composition(input.clone()),
            Err(ConformanceError::SubstitutionNotExecuted("CTS-PROF-RT-1"))
        );
        // Execute the substitute: accepted.
        input
            .core_results
            .push(FixtureResult::pass("CTS-PROF-RT-1"));
        let declaration = declare_composition(input).unwrap();
        assert!(declaration.conformant);
        assert_eq!(declaration.substitutions.len(), 1);
    }

    #[test]
    fn profile_claim_requires_a_non_empty_subset_and_it_must_pass() {
        // Empty subset with a profile claim is not a claim.
        let mut input = base_input();
        input.profile = Some(Profile::Desktop);
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::MissingProfileCoverage(Profile::Desktop))
        );

        // A failing subset fixture is reported as a gap, not conformant.
        let mut input = base_input();
        input.profile = Some(Profile::Desktop);
        input.profile_subset = vec![FixtureResult::fail(
            "CTS-PROF-DESKTOP-1",
            FailureMode::ConformanceGap,
        )];
        let declaration = declare_composition(input).unwrap();
        assert!(!declaration.conformant);
        assert!(
            declaration
                .gaps
                .iter()
                .any(|gap| gap.contains("CTS-PROF-DESKTOP-1"))
        );
    }

    #[test]
    fn omitted_core_fixture_with_justification_is_recorded_as_a_gap() {
        let mut input = base_input();
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-MAN-5");
        input.omitted.push(OmittedTest {
            fixture: "CTS-COMP-MAN-5",
            justification: "not applicable to single-node host reference".to_string(),
        });
        let declaration = declare_composition(input).unwrap();
        assert!(!declaration.conformant);
        assert!(
            declaration
                .gaps
                .iter()
                .any(|gap| gap.contains("CTS-COMP-MAN-5"))
        );
    }

    #[test]
    fn empty_substitution_reuse_across_records_is_rejected() {
        let mut input = base_input();
        input.core_results.retain(|result| {
            result.fixture != "CTS-COMP-RT-1" && result.fixture != "CTS-COMP-RT-2"
        });
        input.substitutions.push(Substitution {
            test: "CTS-COMP-RT-1",
            substitute: "CTS-PROF-RT-1",
            justification: "stricter substitute".to_string(),
        });
        input.substitutions.push(Substitution {
            test: "CTS-COMP-RT-2",
            substitute: "CTS-PROF-RT-1",
            justification: "reused substitute must be rejected".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::InvalidSubstitution("CTS-COMP-RT-2"))
        );
    }

    #[test]
    fn artifact_vector_is_stable() {
        let declaration = declare_composition(base_input()).unwrap();
        // Golden value: fails loudly on any framing change (field set,
        // encoding, canonicalization), by design.
        assert_eq!(declaration.artifact, ArtifactId(GOLDEN_ARTIFACT));
    }

    #[test]
    fn artifact_vector_with_failing_fixture_is_stable() {
        let mut input = base_input();
        let position = input
            .core_results
            .iter()
            .position(|result| result.fixture == "CTS-COMP-MAN-4")
            .unwrap();
        input.core_results[position] =
            FixtureResult::fail("CTS-COMP-MAN-4", FailureMode::ImplementationBug);
        let declaration = declare_composition(input).unwrap();
        assert_eq!(declaration.artifact, ArtifactId(GOLDEN_FAIL));
    }

    #[test]
    fn artifact_vector_with_substitutions_is_stable() {
        let mut input = base_input();
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-RT-1");
        input.substitutions.push(Substitution {
            test: "CTS-COMP-RT-1",
            substitute: "CTS-PROF-RT-1",
            justification: "stricter substitute".to_string(),
        });
        input
            .core_results
            .push(FixtureResult::pass("CTS-PROF-RT-1"));
        let declaration = declare_composition(input).unwrap();
        assert_eq!(declaration.artifact, ArtifactId(GOLDEN_SUB));
    }

    #[test]
    fn artifact_vector_with_omissions_is_stable() {
        let mut input = base_input();
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-MAN-2");
        input.omitted.push(OmittedTest {
            fixture: "CTS-COMP-MAN-2",
            justification: "single-node host reference".to_string(),
        });
        let declaration = declare_composition(input).unwrap();
        assert_eq!(declaration.artifact, ArtifactId(GOLDEN_OMIT));
    }

    #[test]
    fn artifact_vector_with_profile_claim_is_stable() {
        let mut input = base_input();
        input.profile = Some(Profile::Desktop);
        input.profile_subset = vec![
            FixtureResult::pass("CTS-PROF-DESKTOP-1"),
            FixtureResult::pass("CTS-PROF-DESKTOP-2"),
        ];
        let declaration = declare_composition(input).unwrap();
        assert_eq!(declaration.artifact, ArtifactId(GOLDEN_PROF));
    }

    #[test]
    fn contradictory_accounting_is_rejected() {
        // Executed and omitted: contradiction.
        let mut input = base_input();
        input.omitted.push(OmittedTest {
            fixture: "CTS-COMP-MAN-1",
            justification: "duplicate claim".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::DuplicateAccounting("CTS-COMP-MAN-1"))
        );

        // Substituted and omitted: contradiction.
        let mut input = base_input();
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-RT-1");
        input.substitutions.push(Substitution {
            test: "CTS-COMP-RT-1",
            substitute: "CTS-PROF-RT-1",
            justification: "stricter substitute".to_string(),
        });
        input.omitted.push(OmittedTest {
            fixture: "CTS-COMP-RT-1",
            justification: "contradictory omission".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::DuplicateAccounting("CTS-COMP-RT-1"))
        );

        // Duplicate omissions.
        let mut input = base_input();
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-MAN-2");
        input.omitted.push(OmittedTest {
            fixture: "CTS-COMP-MAN-2",
            justification: "first".to_string(),
        });
        input.omitted.push(OmittedTest {
            fixture: "CTS-COMP-MAN-2",
            justification: "second".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::DuplicateAccounting("CTS-COMP-MAN-2"))
        );
    }

    #[test]
    fn substitution_records_are_validated() {
        // Empty justification rejected.
        let mut input = base_input();
        input
            .core_results
            .retain(|result| result.fixture != "CTS-COMP-RT-1");
        input.substitutions.push(Substitution {
            test: "CTS-COMP-RT-1",
            substitute: "CTS-PROF-RT-1",
            justification: "   ".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::UnjustifiedSubstitution("CTS-COMP-RT-1"))
        );

        // Substituting a non-core fixture, a self-substitution, and a
        // duplicate record are all rejected.
        let mut input = base_input();
        input.substitutions.push(Substitution {
            test: "NOT-A-CORE-FIXTURE",
            substitute: "CTS-PROF-RT-1",
            justification: "not core".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::InvalidSubstitution("NOT-A-CORE-FIXTURE"))
        );

        let mut input = base_input();
        input.substitutions.push(Substitution {
            test: "CTS-COMP-MAN-1",
            substitute: "CTS-COMP-MAN-1",
            justification: "self".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::InvalidSubstitution("CTS-COMP-MAN-1"))
        );
    }

    #[test]
    fn non_core_results_and_omissions_are_rejected() {
        // A core result for an unknown fixture is rejected.
        let mut input = base_input();
        input
            .core_results
            .push(FixtureResult::pass("CTS-MYSTERY-1"));
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::UnexpectedFixture("CTS-MYSTERY-1"))
        );

        // An omission naming a non-core fixture is rejected.
        let mut input = base_input();
        input.omitted.push(OmittedTest {
            fixture: "CTS-MYSTERY-1",
            justification: "not part of the core set".to_string(),
        });
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::OmittedNonCoreFixture("CTS-MYSTERY-1"))
        );
    }

    #[test]
    fn report_cites_artifact_and_fixtures_rfc_0006_4() {
        let declaration = declare_composition(base_input()).unwrap();
        assert!(declaration.artifact_is_cited());
        let report = declaration.report();
        assert_eq!(report.artifact, declaration.artifact);
        assert_eq!(report.artifacts_tested.len(), CORE_COMPOSITION_TESTS.len());
        assert!(report.artifacts_tested.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn report_aggregates_forensics_buckets_exactly() {
        let mut input = base_input();
        let modes = [
            FailureMode::ConformanceViolation,
            FailureMode::ConformanceGap,
            FailureMode::ImplementationBug,
            FailureMode::TestBug,
            FailureMode::EnvironmentalFailure,
        ];
        let fixtures: Vec<&'static str> = CORE_COMPOSITION_TESTS.iter().take(5).copied().collect();
        for (index, mode) in modes.iter().enumerate() {
            let fixture = fixtures[index];
            let position = input
                .core_results
                .iter()
                .position(|result| result.fixture == fixture)
                .unwrap();
            input.core_results[position] = FixtureResult::fail(fixture, *mode);
        }
        let declaration = declare_composition(input).unwrap();
        let counts = declaration.report().forensics;
        assert_eq!(counts.conformance_violations, 1);
        assert_eq!(counts.conformance_gaps, 1);
        assert_eq!(counts.implementation_bugs, 1);
        assert_eq!(counts.test_bugs, 1);
        assert_eq!(counts.environmental_failures, 1);
    }

    #[test]
    fn report_lists_manifests_and_tier_claims() {
        let mut input = base_input();
        input.manifests_tested = vec!["component-a".to_string(), "component-b".to_string()];
        input.tier_claims = vec![
            ("component-a".to_string(), Tier::A),
            ("component-b".to_string(), Tier::B),
        ];
        let declaration = declare_composition(input).unwrap();
        let report = declaration.report();
        assert_eq!(report.manifests_tested, ["component-a", "component-b"]);
        assert_eq!(
            report.tier_claims,
            [
                ("component-a".to_string(), Tier::A),
                ("component-b".to_string(), Tier::B),
            ]
        );
    }

    #[test]
    fn artifact_citation_fails_for_zero_artifact() {
        let mut declaration = declare_composition(base_input()).unwrap();
        assert!(declaration.artifact_is_cited());
        declaration.artifact = super::conformance::ArtifactId(0);
        assert!(!declaration.artifact_is_cited());

        // An entirely-omitted declaration (no fixtures executed, no profile
        // subset) is likewise not a cited artifact.
        let empty = {
            let core = pass_all_core();
            let _ = core; // keep helper used elsewhere
            DeclarationInput {
                core_results: Vec::new(),
                omitted: CORE_COMPOSITION_TESTS
                    .iter()
                    .map(|fixture| OmittedTest {
                        fixture,
                        justification: "out of scope for this host-reference run".to_string(),
                    })
                    .collect(),
                ..base_input()
            }
        };
        let declaration = declare_composition(empty).unwrap();
        assert!(!declaration.artifact_is_cited());
    }

    #[test]
    fn inconsistent_results_and_duplicates_are_rejected() {
        let mut input = base_input();
        let duplicate = input.core_results[0];
        input.core_results.push(duplicate);
        assert_eq!(
            declare_composition(input),
            Err(ConformanceError::DuplicateResult("CTS-COMP-MAN-1"))
        );

        let mut unjustified = base_input();
        unjustified.omitted.push(OmittedTest {
            fixture: "CTS-COMP-MAN-1",
            justification: "   ".to_string(),
        });
        assert_eq!(
            declare_composition(unjustified),
            Err(ConformanceError::UnjustifiedOmission("CTS-COMP-MAN-1"))
        );
    }
}

#[cfg(test)]
mod routing_tests {
    use super::routing::{DenialReason, RoutingOutcome, RoutingRequest, route};
    use super::*;
    use aether_types::{Bounds, Capability, ObjectId, ObjectType, Rights};

    fn component(id: u64) -> ComponentId {
        ComponentId(id)
    }

    fn manifest_with_provides(id: u64, provides: Vec<Grant>) -> Manifest {
        Manifest {
            id: component(id),
            schema_version: 1,
            version: 1,
            requires: Vec::new(),
            provides,
            profiles: vec![Profile::Desktop],
            tier: Tier::B,
            max_instances: 1,
            rollback_on_failure: true,
            start: "main".to_string(),
            stop: "main".to_string(),
        }
    }

    fn attenuated_grant(name: &str, ceiling: Option<Rights>) -> Grant {
        Grant {
            name: name.to_string(),
            narrowing: NarrowingPolicy::Attenuate,
            rights_ceiling: ceiling,
        }
    }

    fn authority() -> Capability {
        Capability {
            object: ObjectId(42),
            object_type: ObjectType::Memory,
            rights: Rights::READ.union(Rights::WRITE).union(Rights::MAP),
            bounds: Bounds::new(0, 65536).unwrap(),
            flags: aether_types::CapabilityFlags::NONE,
            expires_at_ns: 0,
        }
    }

    #[test]
    fn attenuated_grant_bounded_by_ceiling_and_authority() {
        let grant = attenuated_grant("memory.read", Some(Rights::READ.union(Rights::WRITE)));
        let grantor = manifest_with_provides(1, vec![grant]);
        let requestor = manifest_with_provides(2, Vec::new());

        // Request covered by the ceiling and held by the grantor: granted.
        let outcome = route(
            &grantor,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "memory.read".to_string(),
                requested_rights: Rights::READ,
            },
        );
        assert_eq!(
            outcome,
            RoutingOutcome::Granted(Capability {
                rights: Rights::READ,
                ..authority()
            })
        );

        // Request exceeds the ceiling: no grant.
        let outcome = route(
            &grantor,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "memory.read".to_string(),
                requested_rights: Rights::READ.union(Rights::MAP),
            },
        );
        assert_eq!(
            outcome,
            RoutingOutcome::Denied(DenialReason::NarrowingFailed)
        );
    }

    #[test]
    fn ceiling_the_grantor_does_not_hold_is_not_derivable() {
        let grant = attenuated_grant("memory.all", Some(Rights::INVOKE));
        let grantor = manifest_with_provides(1, vec![grant]);
        let requestor = manifest_with_provides(2, Vec::new());
        let outcome = route(
            &grantor,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "memory.all".to_string(),
                requested_rights: Rights::INVOKE,
            },
        );
        assert_eq!(
            outcome,
            RoutingOutcome::Denied(DenialReason::GrantNotDerivable)
        );
    }

    #[test]
    fn exact_policy_grants_the_ceiling_and_rejects_a_mismatched_request() {
        let grant = Grant {
            name: "memory.rw".to_string(),
            narrowing: NarrowingPolicy::Exact,
            rights_ceiling: Some(Rights::READ.union(Rights::WRITE)),
        };
        let grantor = manifest_with_provides(1, vec![grant]);
        let requestor = manifest_with_provides(2, Vec::new());

        let mismatched = route(
            &grantor,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "memory.rw".to_string(),
                requested_rights: Rights::READ,
            },
        );
        assert_eq!(
            mismatched,
            RoutingOutcome::Denied(DenialReason::NarrowingFailed)
        );

        let exact = route(
            &grantor,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "memory.rw".to_string(),
                requested_rights: Rights::READ.union(Rights::WRITE),
            },
        );
        assert_eq!(
            exact,
            RoutingOutcome::Granted(Capability {
                rights: Rights::READ.union(Rights::WRITE),
                ..authority()
            })
        );
    }

    #[test]
    fn missing_grant_and_none_ceiling_fail_closed() {
        let grantor = manifest_with_provides(1, Vec::new());
        let requestor = manifest_with_provides(2, Vec::new());
        let outcome = route(
            &grantor,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "absent".to_string(),
                requested_rights: Rights::READ,
            },
        );
        assert_eq!(
            outcome,
            RoutingOutcome::Denied(DenialReason::GrantNotDerivable)
        );

        // A grant with no ceiling grants nothing.
        let ceilingless = manifest_with_provides(1, vec![attenuated_grant("any", None)]);
        let outcome = route(
            &ceilingless,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "any".to_string(),
                requested_rights: Rights::READ,
            },
        );
        assert_eq!(
            outcome,
            RoutingOutcome::Denied(DenialReason::NarrowingFailed)
        );
    }

    #[test]
    fn invalid_manifests_are_rejected() {
        let mut broken = manifest_with_provides(1, Vec::new());
        broken.max_instances = 0;
        let requestor = manifest_with_provides(2, Vec::new());
        let outcome = route(
            &broken,
            &requestor,
            &authority(),
            &RoutingRequest {
                capability: "any".to_string(),
                requested_rights: Rights::READ,
            },
        );
        assert_eq!(
            outcome,
            RoutingOutcome::Denied(DenialReason::ManifestInvalid)
        );
    }

    #[test]
    fn environmental_independence_holds() {
        let grant = attenuated_grant("memory.read", Some(Rights::READ.union(Rights::WRITE)));
        let grantor = manifest_with_provides(1, vec![grant]);
        let requestor = manifest_with_provides(2, Vec::new());
        let request = RoutingRequest {
            capability: "memory.read".to_string(),
            requested_rights: Rights::READ,
        };
        let first = route(&grantor, &requestor, &authority(), &request);
        for _ in 0..100 {
            assert_eq!(route(&grantor, &requestor, &authority(), &request), first);
        }
    }
}

#[cfg(test)]
mod profile_tests {
    use super::profile::{
        Mechanism, ProfileDeclaration, ProfileError, apply_profile, combine_profiles,
    };
    use super::*;

    fn mechanism(name: &str) -> Mechanism {
        Mechanism::new(name).unwrap()
    }

    fn manifest_with_tier(id: u64, tier: Tier) -> Manifest {
        Manifest {
            id: ComponentId(id),
            schema_version: 1,
            version: 1,
            requires: Vec::new(),
            provides: Vec::new(),
            profiles: vec![Profile::Desktop],
            tier,
            max_instances: 1,
            rollback_on_failure: true,
            start: "main".to_string(),
            stop: "main".to_string(),
        }
    }

    fn desktop() -> ProfileDeclaration {
        ProfileDeclaration {
            profile: Profile::Desktop,
            mandatory: vec![mechanism("RFC-0011: layer1-scheduler")],
            declared_optional: vec![mechanism("RFC-0020: posix-projection")],
            forbidden: vec![mechanism("multi-domain-federation")],
            extensions: Vec::new(),
            minimum_tier: Tier::B,
        }
    }

    fn safety_critical() -> ProfileDeclaration {
        ProfileDeclaration {
            profile: Profile::SafetyCritical,
            mandatory: vec![mechanism("locked-manifests")],
            declared_optional: Vec::new(),
            forbidden: vec![mechanism("RFC-0011: layer3-offline-learning")],
            extensions: Vec::new(),
            minimum_tier: Tier::A,
        }
    }

    #[test]
    fn mandatory_and_declared_optional_activate_defaults_do_not() {
        let manifest = manifest_with_tier(1, Tier::A);
        let decision = apply_profile(&desktop(), &manifest).unwrap();
        assert_eq!(
            decision.activated,
            vec![
                mechanism("RFC-0011: layer1-scheduler"),
                mechanism("RFC-0020: posix-projection"),
            ]
        );
        assert!(decision.suppressed.is_empty());
    }

    #[test]
    fn weak_manifest_tier_is_rejected() {
        let manifest = manifest_with_tier(1, Tier::D);
        assert_eq!(
            apply_profile(&desktop(), &manifest),
            Err(ProfileError::TierConflict {
                component: ComponentId(1),
                declared: Tier::D,
                minimum: Tier::B,
            })
        );
    }

    #[test]
    fn opting_in_to_a_forbidden_mechanism_is_rejected() {
        let mut declaration = desktop();
        declaration
            .extensions
            .push(mechanism("multi-domain-federation"));
        let manifest = manifest_with_tier(1, Tier::A);
        assert_eq!(
            apply_profile(&declaration, &manifest),
            Err(ProfileError::ForbiddenDeclared(mechanism(
                "multi-domain-federation"
            )))
        );
    }

    #[test]
    fn combination_unions_forbidden_and_applies_strictest_tier_floor() {
        let manifest = manifest_with_tier(1, Tier::B);
        // Desktop allows Tier B; Safety-Critical demands Tier A.
        assert_eq!(
            combine_profiles(&[desktop(), safety_critical()], &manifest),
            Err(ProfileError::TierConflict {
                component: ComponentId(1),
                declared: Tier::B,
                minimum: Tier::A,
            })
        );

        let strong = manifest_with_tier(1, Tier::A);
        let decision = combine_profiles(&[desktop(), safety_critical()], &strong).unwrap();
        assert!(decision.activated.contains(&mechanism("locked-manifests")));
        assert!(
            decision
                .activated
                .contains(&mechanism("RFC-0011: layer1-scheduler"))
        );
        assert!(
            !decision
                .activated
                .contains(&mechanism("RFC-0011: layer3-offline-learning"))
        );
    }

    #[test]
    fn a_mandatory_mechanism_forbidden_by_a_combined_profile_is_suppressed_not_activated() {
        let mut embedded = ProfileDeclaration {
            profile: Profile::Embedded,
            mandatory: vec![mechanism("RFC-0028: multi-domain-streaming")],
            declared_optional: Vec::new(),
            forbidden: Vec::new(),
            extensions: Vec::new(),
            minimum_tier: Tier::D,
        };
        let mut cloud = safety_critical();
        cloud.mandatory = vec![mechanism("static-manifests")];
        cloud
            .forbidden
            .push(mechanism("RFC-0028: multi-domain-streaming"));
        let manifest = manifest_with_tier(1, Tier::A);
        let decision = combine_profiles(&[embedded.clone(), cloud], &manifest).unwrap();
        assert!(
            !decision
                .activated
                .contains(&mechanism("RFC-0028: multi-domain-streaming"))
        );
        assert!(
            decision
                .suppressed
                .contains(&mechanism("RFC-0028: multi-domain-streaming"))
        );
        assert!(decision.activated.contains(&mechanism("static-manifests")));

        // Rebinding works: declaring the same list in one profile only still activates it.
        embedded.forbidden = Vec::new();
        let single = apply_profile(&embedded, &manifest_with_tier(1, Tier::A)).unwrap();
        assert!(
            single
                .activated
                .contains(&mechanism("RFC-0028: multi-domain-streaming"))
        );
    }

    #[test]
    fn empty_mechanism_names_fail_closed() {
        assert_eq!(Mechanism::new("  "), Err(ProfileError::EmptyMechanism));
    }

    #[test]
    fn mandatory_overlapping_forbidden_is_rejected() {
        let declaration = ProfileDeclaration {
            profile: Profile::Embedded,
            mandatory: vec![mechanism("locked-manifests")],
            declared_optional: Vec::new(),
            forbidden: vec![mechanism("locked-manifests")],
            extensions: Vec::new(),
            minimum_tier: Tier::D,
        };
        let manifest = manifest_with_tier(1, Tier::A);
        assert_eq!(
            apply_profile(&declaration, &manifest),
            Err(ProfileError::ForbiddenMandatory(mechanism(
                "locked-manifests"
            )))
        );
    }

    #[test]
    fn empty_combination_is_rejected() {
        let manifest = manifest_with_tier(1, Tier::A);
        assert_eq!(
            combine_profiles(&[], &manifest),
            Err(ProfileError::EmptyDeclarations)
        );
    }

    #[test]
    fn combine_rejects_per_declaration_mandatory_forbidden_overlap() {
        let declaration = ProfileDeclaration {
            profile: Profile::Embedded,
            mandatory: vec![mechanism("locked-manifests")],
            declared_optional: Vec::new(),
            forbidden: vec![mechanism("locked-manifests")],
            extensions: Vec::new(),
            minimum_tier: Tier::D,
        };
        let strong = safety_critical();
        let manifest = manifest_with_tier(1, Tier::A);
        assert_eq!(
            combine_profiles(&[declaration, strong], &manifest),
            Err(ProfileError::ForbiddenMandatory(mechanism(
                "locked-manifests"
            )))
        );
    }

    #[test]
    fn explicitly_opted_in_extension_is_activated() {
        let mut declaration = desktop();
        declaration.extensions.push(mechanism("object-projection"));
        let manifest = manifest_with_tier(1, Tier::A);
        let decision = apply_profile(&declaration, &manifest).unwrap();
        assert!(decision.activated.contains(&mechanism("object-projection")));
    }

    #[test]
    fn combination_suppresses_cross_profile_forbidden_opt_in() {
        let embedded = ProfileDeclaration {
            profile: Profile::Embedded,
            mandatory: vec![mechanism("base-scheduler")],
            declared_optional: Vec::new(),
            forbidden: Vec::new(),
            extensions: vec![mechanism("object-projection")],
            minimum_tier: Tier::D,
        };
        let mut strong = safety_critical();
        strong.extensions = Vec::new();
        strong.forbidden.push(mechanism("object-projection"));
        let manifest = manifest_with_tier(1, Tier::A);
        let decision = combine_profiles(&[embedded, strong], &manifest).unwrap();
        assert!(!decision.activated.contains(&mechanism("object-projection")));
        assert!(
            decision
                .suppressed
                .contains(&mechanism("object-projection"))
        );
        assert!(decision.activated.contains(&mechanism("locked-manifests")));
    }

    #[test]
    fn empty_mandatory_set_is_rejected() {
        let declaration = ProfileDeclaration {
            profile: Profile::Embedded,
            mandatory: Vec::new(),
            declared_optional: Vec::new(),
            forbidden: Vec::new(),
            extensions: Vec::new(),
            minimum_tier: Tier::D,
        };
        let manifest = manifest_with_tier(1, Tier::A);
        assert_eq!(
            apply_profile(&declaration, &manifest),
            Err(ProfileError::EmptyMandatory)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(id: u64) -> ComponentId {
        ComponentId(id)
    }

    fn manifest(id: u64, version: u64, requires: Vec<Requirement>) -> Manifest {
        Manifest {
            id: component(id),
            schema_version: 1,
            version,
            requires,
            provides: Vec::new(),
            profiles: vec![Profile::Desktop],
            tier: Tier::B,
            max_instances: 4,
            rollback_on_failure: true,
            start: "main".to_string(),
            stop: "main".to_string(),
        }
    }

    fn requirement(component_id: u64, min: u64, max: u64) -> Requirement {
        Requirement {
            component: component(component_id),
            min_version: min,
            max_version: max,
        }
    }

    fn pool(entries: Vec<Manifest>) -> BTreeMap<ComponentId, Vec<Manifest>> {
        let mut map: BTreeMap<ComponentId, Vec<Manifest>> = BTreeMap::new();
        for manifest in entries {
            map.entry(manifest.id).or_default().push(manifest);
        }
        map
    }

    #[test]
    fn invalid_manifests_are_rejected() {
        let mut bad = manifest(1, 1, Vec::new());
        bad.max_instances = 0;
        assert_eq!(
            validate_manifest(&bad),
            Err(ManifestError::InvalidMaxInstances)
        );
        bad.max_instances = 1;
        bad.profiles = Vec::new();
        assert_eq!(validate_manifest(&bad), Err(ManifestError::NoProfiles));
        bad.profiles = vec![Profile::Desktop];
        bad.start = String::new();
        assert_eq!(
            validate_manifest(&bad),
            Err(ManifestError::EmptyLifecycleAddress)
        );

        let target = manifest(
            1,
            1,
            vec![Requirement {
                component: component(0),
                min_version: 1,
                max_version: 1,
            }],
        );
        assert_eq!(
            validate_manifest(&target),
            Err(ManifestError::InvalidRequirement)
        );
    }

    #[test]
    fn highest_version_strategy_wins_and_still_enforces_overlap() {
        let target = manifest(1, 1, vec![requirement(2, 1, 9)]);
        let candidates = pool(vec![
            manifest(2, 3, Vec::new()),
            manifest(2, 7, Vec::new()),
            manifest(2, 10, Vec::new()), // outside range
        ]);
        let resolution = resolve(&target, &candidates, SelectionStrategy::HighestVersion).unwrap();
        assert_eq!(resolution.order, vec![component(2), component(1)]);
        assert_eq!(resolution.events[0].version, 7);
        assert_eq!(resolution.events[0].reason, SelectionReason::HighestVersion);
    }

    #[test]
    fn missing_provider_is_reported_with_reason() {
        let target = manifest(1, 1, vec![requirement(2, 1, 1)]);
        let candidates = pool(Vec::new());
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::MissingProvider(component(2)))
        );
    }

    #[test]
    fn version_mismatch_profile_mismatch_and_tier_mismatch_are_distinct() {
        let target = manifest(1, 1, vec![requirement(2, 1, 5)]);
        // Version mismatch: provider exists but out of range.
        let candidates = pool(vec![manifest(2, 7, Vec::new())]);
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::VersionMismatch(component(2)))
        );

        // Profile mismatch: version ok but no shared profile.
        let mut other = manifest(2, 3, Vec::new());
        other.profiles = vec![Profile::HPC];
        let candidates = pool(vec![other.clone()]);
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::ProfileMismatch(component(2)))
        );

        // Tier mismatch: profile ok but provider tier is weaker.
        let mut weak = other.clone();
        weak.profiles = vec![Profile::Desktop];
        weak.tier = Tier::D; // weaker than target's Tier::B
        let candidates = pool(vec![weak]);
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::TierMismatch(component(2)))
        );
    }

    #[test]
    fn cycles_are_reported_with_sorted_members_and_no_order() {
        let a = manifest(1, 1, vec![requirement(2, 1, 1)]);
        let b = manifest(2, 1, vec![requirement(1, 1, 1)]);
        let candidates = pool(vec![a.clone(), b]);
        assert_eq!(
            resolve(&a, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::Cycle(vec![component(1), component(2)]))
        );
    }

    #[test]
    fn self_dependency_is_a_cycle() {
        // The selected provider of a component's own id requires the same id
        // (same manifest): a genuine self-cycle in the resolution graph.
        let a = manifest(1, 1, vec![requirement(1, 1, 1)]);
        let candidates = pool(vec![a.clone()]);
        assert_eq!(
            resolve(&a, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::Cycle(vec![component(1)]))
        );
    }

    #[test]
    fn target_self_requirement_with_acyclic_provider_resolves_without_panic() {
        // Verifier repro D1: target id 1 requires both a released provider of
        // id 1 and another component; the provider has no requirements. No
        // cycle exists and no entry may be silently overwritten.
        let target = manifest(1, 2, vec![requirement(2, 1, 1), requirement(1, 1, 1)]);
        let candidates = pool(vec![manifest(2, 1, Vec::new()), manifest(1, 1, Vec::new())]);
        let resolution = resolve(&target, &candidates, SelectionStrategy::HighestVersion)
            .expect("acyclic same-id provider must resolve");
        // Dependencies are emitted before the target; deterministic pop order
        // selects the equal-degree providers in ascending id order.
        assert_eq!(
            resolution.order,
            vec![component(1), component(2), component(1)]
        );
    }

    #[test]
    fn cycle_members_exclude_innocent_downstream_dependents() {
        // 1 -> 2, 2 -> 1 is the cycle; 4 -> 1 and target 3 -> 4 are innocent.
        let one = manifest(1, 1, vec![requirement(2, 1, 1)]);
        let two = manifest(2, 1, vec![requirement(1, 1, 1)]);
        let four = manifest(4, 1, vec![requirement(1, 1, 1)]);
        let target = manifest(3, 1, vec![requirement(4, 1, 1)]);
        let candidates = pool(vec![one, two, four]);
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::Cycle(vec![component(1), component(2)]))
        );
    }

    #[test]
    fn inconsistent_pool_key_fails_closed() {
        let target = manifest(1, 1, vec![requirement(2, 1, 1)]);
        let mut candidates = BTreeMap::new();
        candidates.insert(component(2), vec![manifest(9, 1, Vec::new())]);
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::InconsistentCandidatePool(component(2)))
        );
    }

    #[test]
    fn validation_rejects_whitespace_and_wrong_schema_version() {
        let mut whitespace_start = manifest(1, 1, Vec::new());
        whitespace_start.start = "   ".to_string();
        assert_eq!(
            validate_manifest(&whitespace_start),
            Err(ManifestError::EmptyLifecycleAddress)
        );
        let mut schema = manifest(1, 1, Vec::new());
        schema.schema_version = 2;
        assert_eq!(
            validate_manifest(&schema),
            Err(ManifestError::UnsupportedSchemaVersion(2))
        );
        let whitespace_grant = manifest(1, 1, Vec::new());
        let mut with_grant = whitespace_grant;
        with_grant.provides = vec![Grant {
            name: "   ".to_string(),
            narrowing: NarrowingPolicy::Attenuate,
            rights_ceiling: None,
        }];
        assert_eq!(
            validate_manifest(&with_grant),
            Err(ManifestError::InvalidGrant)
        );
    }

    #[test]
    fn diamond_dependencies_order_deterministically() {
        let root = manifest(1, 1, vec![requirement(2, 1, 1), requirement(3, 1, 1)]);
        let a = manifest(2, 1, vec![requirement(4, 1, 1)]);
        let b = manifest(3, 1, vec![requirement(4, 1, 1)]);
        let d = manifest(4, 1, Vec::new());
        let candidates = pool(vec![d, b, a]);
        let resolution = resolve(&root, &candidates, SelectionStrategy::HighestVersion).unwrap();
        assert_eq!(
            resolution.order,
            vec![component(4), component(2), component(3), component(1)]
        );
        // D was selected once; the memoized second reference adds no event.
        let d_events = resolution
            .events
            .iter()
            .filter(|event| event.component == component(4))
            .count();
        assert_eq!(d_events, 1);
    }

    #[test]
    fn ambiguous_provider_fails_closed() {
        let target = manifest(1, 1, vec![requirement(2, 1, 1)]);
        let first = manifest(2, 1, Vec::new());
        let mut second = manifest(2, 1, Vec::new());
        second.provides = vec![Grant {
            name: "other".to_string(),
            narrowing: NarrowingPolicy::Exact,
            rights_ceiling: None,
        }];
        let candidates = pool(vec![first, second]);
        assert_eq!(
            resolve(&target, &candidates, SelectionStrategy::HighestVersion),
            Err(ResolutionError::AmbiguousProvider(component(2)))
        );
    }

    #[test]
    fn invalid_candidates_are_refused_per_rfc_0004_4() {
        let target = manifest(1, 1, vec![requirement(2, 1, 9)]);
        let mut broken = manifest(2, 9, Vec::new());
        broken.max_instances = 0;
        let ok = manifest(2, 3, Vec::new());
        let candidates = pool(vec![broken, ok]);
        let resolution = resolve(&target, &candidates, SelectionStrategy::HighestVersion).unwrap();
        // Only the valid candidate is selectable.
        assert_eq!(resolution.events[0].version, 3);
    }

    #[test]
    fn sole_selected_candidate_reports_sole_candidate_reason() {
        let target = manifest(1, 1, vec![requirement(2, 1, 1)]);
        let candidates = pool(vec![manifest(2, 1, Vec::new())]);
        let resolution = resolve(&target, &candidates, SelectionStrategy::HighestVersion).unwrap();
        assert_eq!(resolution.events[0].reason, SelectionReason::SoleCandidate);
    }
}
