//! Composition conformance declaration model (RFC-0006; methodology §8/§10).
//!
//! This module models the *declaration* a conforming implementation must
//! publish: the mandatory composition fixtures that ran, their outcomes and
//! failure modes, substitutions declared per RFC-0006.6, required tests
//! skipped with justification, and the content-derived artifact identifier
//! cite required by RFC-0006.4.
//!
//! It is **not** a CTS executor, does not run any fixture, does not emit
//! RFC-0034 audit records, and is not a conformance claim itself. It is a
//! host-reference data model whose outputs another tool would consume to
//! produce a declaration document. The RFC-0006.4 artifact citation is
//! exposed via `CompositionConformance::artifact_is_cited` (CTS-COMP-CONF-3),
//! the §4 reporting obligations via `CompositionConformance::report`
//! (manifests tested, active profile declarations, tier claims, substitution
//! table, failure-forensics category counts, cited artifact and fixture IDs),
//! with forensics aggregation pinned by CTS-COMP-CONF-4.
//! RFC-0006.2's profile to `Profiles-Specification.md` section 4 binding,
//! RFC-0006.3's RFC-0034 audit-entry obligation, and §3-6 beyond this model
//! remain **not implemented** and not claimed.

//! Declared semantics:
//!
//! - `CORE_COMPOSITION_TESTS` is the RFC-0006.1 fixture set (RFC-0002 MAN-1..5,
//!   RFC-0003 RT-1..4, RFC-0004 DEPR-1..5, RFC-0005 PROF-1..4).
//! - Every core fixture must be executed (or substituted with the substitute
//!   executed, RFC-0006.6); otherwise the declaration is rejected
//!   (`MissingCoreTest` / `SubstitutionNotExecuted`).
//! - A failed fixture MUST carry a failure mode from the methodology §10
//!   taxonomy; an unclassified failure rejects the declaration
//!   (`FailureNotClassified`). A passing fixture MUST NOT carry one.
//! - A profile claim requires a non-empty profile subset and all of it must
//!   pass (`MissingProfileCoverage` / `conformant = false` with gap records).
//! - Required tests skipped must carry a justification; substitutions must be
//!   declared and their substitutes executed.
//! - `artifact` is a 64-bit FNV-1a content hash over (CTS version,
//!   implementation version, claimed profile, the full fixture vector including
//!   substitutions and omissions with their justifications). Every byte
//!   sequence is length-prefixed and result vectors are canonicalized by
//!   fixture id, so identical runs hash identically regardless of input order.
//!   Non-cryptographic, consistent with the repository's other host-model
//!   digests; good enough for bookkeeping, not for release attestation.
//!   Methodology §8's timestamp and forensics capture are **not** bound (a
//!   golden vector pins the current framing).
//! - Events are not emitted; the declaration is a value, not a log.
//! - `conformant` reports whether the run supports a conformance claim (all
//!   core passed, all subset passed, no omitted core); it is a statement
//!   about THIS fixture vector only, not about any real deployment.

use crate::Profile;
use std::collections::HashSet;

/// Failure modes from the Conformance-Test-Suite-Methodology §10 taxonomy.
///
/// Discriminants are pinned: artifact framing encodes them with `as u8`, so
/// reordering or renumbering variants would change every artifact identifier.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureMode {
    /// Fails the relevant MUST or invariant.
    ConformanceViolation = 0,
    /// Fails a SHOULD or profile-specific requirement.
    ConformanceGap = 1,
    /// Test correctly specified; implementation fails.
    ImplementationBug = 2,
    /// Test itself non-conformant; must be re-specified.
    TestBug = 3,
    /// Test environment fails; reruns may differ.
    EnvironmentalFailure = 4,
}

/// Content-derived identifier of one CTS run (methodology §8).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArtifactId(pub u64);

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut acc: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(FNV_PRIME);
    }
    acc
}

fn push_bytes(mut acc: u64, bytes: &[u8]) -> u64 {
    acc = fnv(acc, &(bytes.len() as u64).to_le_bytes());
    fnv(acc, bytes)
}

const fn profile_tag(profile: Option<Profile>) -> u8 {
    match profile {
        None => 0,
        Some(Profile::Embedded) => 1,
        Some(Profile::Desktop) => 2,
        Some(Profile::Cloud) => 3,
        Some(Profile::HPC) => 4,
        Some(Profile::SafetyCritical) => 5,
    }
}

fn artifact_of(
    cts_version: &str,
    implementation: &str,
    profile: Option<Profile>,
    core_results: &[FixtureResult],
    profile_subset: &[FixtureResult],
    substitutions: &[Substitution],
    omitted: &[OmittedTest],
) -> ArtifactId {
    let mut acc = FNV_OFFSET;
    acc = push_bytes(acc, cts_version.as_bytes());
    acc = push_bytes(acc, implementation.as_bytes());
    acc = fnv(acc, &[profile_tag(profile)]);
    let mut results: Vec<FixtureResult> =
        core_results.iter().chain(profile_subset).copied().collect();
    results.sort_by(|a, b| a.fixture.cmp(b.fixture));
    for result in &results {
        acc = push_bytes(acc, result.fixture.as_bytes());
        acc = fnv(acc, &[u8::from(result.passed)]);
        acc = fnv(acc, &[result.failure.map_or(u8::MAX, |mode| mode as u8)]);
    }
    let mut substitutions = substitutions.to_vec();
    substitutions.sort_by(|a, b| a.test.cmp(b.test).then(a.substitute.cmp(b.substitute)));
    for substitution in &substitutions {
        acc = push_bytes(acc, substitution.test.as_bytes());
        acc = push_bytes(acc, substitution.substitute.as_bytes());
        acc = push_bytes(acc, substitution.justification.as_bytes());
    }
    let mut omitted = omitted.to_vec();
    omitted.sort_by(|a, b| a.fixture.cmp(b.fixture));
    for omission in &omitted {
        acc = push_bytes(acc, omission.fixture.as_bytes());
        acc = push_bytes(acc, omission.justification.as_bytes());
    }
    ArtifactId(acc)
}

/// Outcome of one fixture execution (methodology §8/§10).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixtureResult {
    /// RFC-0006 fixture identifier (e.g. `CTS-COMP-MAN-1`).
    pub fixture: &'static str,
    /// Whether the fixture passed.
    pub passed: bool,
    /// Failure mode; mandatory iff `passed` is `false` (methodology §10).
    pub failure: Option<FailureMode>,
}

impl FixtureResult {
    /// Creates a passing fixture result.
    #[must_use]
    pub const fn pass(fixture: &'static str) -> Self {
        Self {
            fixture,
            passed: true,
            failure: None,
        }
    }

    /// Creates a failing fixture result with the mandatory failure mode.
    #[must_use]
    pub const fn fail(fixture: &'static str, mode: FailureMode) -> Self {
        Self {
            fixture,
            passed: false,
            failure: Some(mode),
        }
    }
}

/// Mandatory composition core fixtures (RFC-0006.1).
pub const CORE_COMPOSITION_TESTS: &[&str] = &[
    "CTS-COMP-MAN-1",
    "CTS-COMP-MAN-2",
    "CTS-COMP-MAN-3",
    "CTS-COMP-MAN-4",
    "CTS-COMP-MAN-5",
    "CTS-COMP-RT-1",
    "CTS-COMP-RT-2",
    "CTS-COMP-RT-3",
    "CTS-COMP-RT-4",
    "CTS-COMP-DEPR-1",
    "CTS-COMP-DEPR-2",
    "CTS-COMP-DEPR-3",
    "CTS-COMP-DEPR-4",
    "CTS-COMP-DEPR-5",
    "CTS-COMP-PROF-1",
    "CTS-COMP-PROF-2",
    "CTS-COMP-PROF-3",
    "CTS-COMP-PROF-4",
];

/// A declared substitute for a core fixture (RFC-0006.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Substitution {
    /// The core fixture being substituted.
    pub test: &'static str,
    /// The substitute fixture executed instead.
    pub substitute: &'static str,
    /// Rationale, recorded alongside the substitution.
    pub justification: String,
}

/// A required fixture skipped, with the mandatory justification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OmittedTest {
    /// The skipped fixture identifier.
    pub fixture: &'static str,
    /// Why it was skipped.
    pub justification: String,
}

/// Input to a conformance declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationInput {
    /// CTS version string (methodology §8).
    pub cts_version: String,
    /// Implementation version string (methodology §8).
    pub implementation: String,
    /// Claimed profile, if the declaration includes a profile subset.
    pub profile: Option<Profile>,
    /// Results of the core fixture set.
    pub core_results: Vec<FixtureResult>,
    /// Results of the profile-specific subset (`Profiles-Specification.md` §4).
    pub profile_subset: Vec<FixtureResult>,
    /// Substitutions declared per RFC-0006.6.
    pub substitutions: Vec<Substitution>,
    /// Required tests skipped, each with justification.
    pub omitted: Vec<OmittedTest>,
    /// Components whose manifests were exercised by this fixture vector
    /// (RFC-0006 section 4).
    pub manifests_tested: Vec<String>,
    /// Tier claims for components exercised (RFC-0006 section 3/4).
    pub tier_claims: Vec<(String, crate::Tier)>,
}

/// The emitted composition conformance declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionConformance {
    /// Artifact identifier this declaration cites (methodology §8).
    pub artifact: ArtifactId,
    /// The profile claimed, if any.
    pub profile: Option<Profile>,
    /// Core fixture results as provided.
    pub core_results: Vec<FixtureResult>,
    /// Profile subset results as provided.
    pub profile_subset: Vec<FixtureResult>,
    /// Substitutions declared (RFC-0006.6).
    pub substitutions: Vec<Substitution>,
    /// Required tests skipped (methodology 8), each with
    /// justification.
    pub omitted: Vec<OmittedTest>,
    /// Components whose manifests were exercised by this fixture vector
    /// (RFC-0006 section 4).
    pub manifests_tested: Vec<String>,
    /// Tier claims for components exercised (RFC-0006 section 3/4).
    pub tier_claims: Vec<(String, crate::Tier)>,
    /// True iff every core fixture passed, the profile subset passed, and no
    /// core fixture was omitted. Statement about this fixture vector only.
    pub conformant: bool,
    /// Human-auditable gap records (failing fixtures, omitted core fixtures).
    pub gaps: Vec<String>,
}

/// Why a declaration was rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceError {
    /// RFC-0006.1: a mandatory core fixture is absent (unexecuted and not
    /// substituted).
    MissingCoreTest(&'static str),
    /// RFC-0006.6: a declared substitute was not executed.
    SubstitutionNotExecuted(&'static str),
    /// Methodology §10: a failed fixture was not classified.
    FailureNotClassified(&'static str),
    /// A passing fixture carried a failure mode.
    InconsistentFailureMode(&'static str),
    /// Methodology §8: an omission record lacks justification.
    UnjustifiedOmission(&'static str),
    /// RFC-0006.2: a profile claim with an empty subset is not a claim.
    MissingProfileCoverage(Profile),
    /// Two results share a fixture identifier.
    DuplicateResult(&'static str),
    /// RFC-0006.6: a substitution record fails validation — empty
    /// justification, a substitute equal to the core fixture, a substitute
    /// that itself is a core fixture, a substitute reused across records,
    /// a `test` that is not a core fixture, or a duplicate substitution record.
    InvalidSubstitution(&'static str),
    /// A fixture was both accounted for and omitted (or omitted more than
    /// once, or recorded as omitted more than once).
    DuplicateAccounting(&'static str),
    /// An omission record names a fixture outside the core set.
    OmittedNonCoreFixture(&'static str),
    /// A `core_results` fixture is neither a core fixture nor a declared
    /// substitute's target.
    UnexpectedFixture(&'static str),
    /// Substitution justification is empty or whitespace-only.
    UnjustifiedSubstitution(&'static str),
}

/// Builds the declaration for one CTS run.
///
/// # Errors
/// Returns the precise typed rejection; nothing is emitted on failure.
pub fn declare_composition(
    input: DeclarationInput,
) -> Result<CompositionConformance, ConformanceError> {
    let all_results: Vec<FixtureResult> = input
        .core_results
        .iter()
        .chain(&input.profile_subset)
        .copied()
        .collect();
    validate_declaration(&input, &all_results)?;

    // Gap records: failures and omitted core fixtures.
    let mut gaps = Vec::new();
    for result in &all_results {
        if let Some(mode) = result.failure {
            gaps.push(format!("fixture {} failed ({mode:?})", result.fixture));
        }
    }
    for omission in &input.omitted {
        if CORE_COMPOSITION_TESTS.contains(&omission.fixture) {
            gaps.push(format!(
                "core fixture {} omitted: {}",
                omission.fixture, omission.justification
            ));
        }
    }

    let artifact = artifact_of(
        &input.cts_version,
        &input.implementation,
        input.profile,
        &input.core_results,
        &input.profile_subset,
        &input.substitutions,
        &input.omitted,
    );

    let conformant = all_results.iter().all(|result| result.passed) && gaps.is_empty();

    Ok(CompositionConformance {
        artifact,
        profile: input.profile,
        core_results: input.core_results,
        profile_subset: input.profile_subset,
        substitutions: input.substitutions,
        omitted: input.omitted,
        manifests_tested: input.manifests_tested,
        tier_claims: input.tier_claims,
        conformant,
        gaps,
    })
}

/// Failure-forensics category counts (methodology section 10), part of the
/// RFC-0006 section 4 reporting obligations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ForensicsCounts {
    /// `FailureMode::ConformanceViolation` count.
    pub conformance_violations: u32,
    /// `FailureMode::ConformanceGap` count.
    pub conformance_gaps: u32,
    /// `FailureMode::ImplementationBug` count.
    pub implementation_bugs: u32,
    /// `FailureMode::TestBug` count.
    pub test_bugs: u32,
    /// `FailureMode::EnvironmentalFailure` count.
    pub environmental_failures: u32,
}

impl ForensicsCounts {
    fn of_results<'a>(results: impl Iterator<Item = &'a FixtureResult>) -> Self {
        let mut counts = Self::default();
        for result in results {
            match result.failure {
                Some(FailureMode::ConformanceViolation) => {
                    counts.conformance_violations += 1;
                }
                Some(FailureMode::ConformanceGap) => counts.conformance_gaps += 1,
                Some(FailureMode::ImplementationBug) => counts.implementation_bugs += 1,
                Some(FailureMode::TestBug) => counts.test_bugs += 1,
                Some(FailureMode::EnvironmentalFailure) => {
                    counts.environmental_failures += 1;
                }
                None => {}
            }
        }
        counts
    }
}

/// The RFC-0006 section 4 reporting view of a declaration.
///
/// §3-6 beyond this model and RFC-0034 audit-entry emission remain out of
/// scope; this is the declaration document's §4 field set as a value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceReport {
    /// Components whose manifests were exercised (RFC-0006 section 4).
    pub manifests_tested: Vec<String>,
    /// Profiles whose subset ran (RFC-0006 section 4); empty when none claimed.
    pub profile_declarations: Vec<Profile>,
    /// Tier claims for components exercised (RFC-0006 section 3/4).
    pub tier_claims: Vec<(String, crate::Tier)>,
    /// Substitution table as declared (RFC-0006 section 4).
    pub substitution_table: Vec<Substitution>,
    /// Failure-forensics category counts (methodology section 10).
    pub forensics: ForensicsCounts,
    /// Artifact the claim cites (RFC-0006.4).
    pub artifact: ArtifactId,
    /// Executed fixture identifiers, sorted (RFC-0006 section 4).
    pub artifacts_tested: Vec<&'static str>,
}

impl CompositionConformance {
    /// Builds the RFC-0006 section 4 reporting view of this declaration.
    #[must_use]
    pub fn report(&self) -> ConformanceReport {
        let mut executed: Vec<&'static str> = self
            .core_results
            .iter()
            .chain(&self.profile_subset)
            .map(|result| result.fixture)
            .collect();
        executed.sort_unstable();
        let forensics =
            ForensicsCounts::of_results(self.core_results.iter().chain(&self.profile_subset));
        ConformanceReport {
            manifests_tested: self.manifests_tested.clone(),
            profile_declarations: self.profile.into_iter().collect(),
            tier_claims: self.tier_claims.clone(),
            substitution_table: self.substitutions.clone(),
            forensics,
            artifact: self.artifact,
            artifacts_tested: executed,
        }
    }

    /// RFC-0006.4 / CTS-COMP-CONF-3: the declaration cites both the executed
    /// fixture set and a non-zero artifact identifier.
    #[must_use]
    pub fn artifact_is_cited(&self) -> bool {
        self.artifact.0 != 0 && !(self.core_results.is_empty() && self.profile_subset.is_empty())
    }
}

fn validate_declaration(
    input: &DeclarationInput,
    all_results: &[FixtureResult],
) -> Result<(), ConformanceError> {
    // Duplicate fixture identifiers are a caller error, not data to merge.
    let mut seen = HashSet::new();
    for result in all_results {
        if !seen.insert(result.fixture) {
            return Err(ConformanceError::DuplicateResult(result.fixture));
        }
    }

    // Failure-mode coherence (methodology §10).
    for result in all_results {
        match (result.passed, result.failure) {
            (false, None) => {
                return Err(ConformanceError::FailureNotClassified(result.fixture));
            }
            (true, Some(_)) => {
                return Err(ConformanceError::InconsistentFailureMode(result.fixture));
            }
            _ => {}
        }
    }

    // Omission justifications are mandatory (methodology §8).
    for omission in &input.omitted {
        if omission.justification.trim().is_empty() {
            return Err(ConformanceError::UnjustifiedOmission(omission.fixture));
        }
    }

    // Every core fixture is accounted for: executed, substituted, or
    // skipped-with-justification (methodology §8); otherwise (RFC-0006.1/.6).
    let executed: HashSet<&str> = all_results.iter().map(|result| result.fixture).collect();
    let substituted: HashSet<&str> = input
        .substitutions
        .iter()
        .map(|substitution| substitution.test)
        .collect();
    let omitted: HashSet<&str> = input
        .omitted
        .iter()
        .map(|omission| omission.fixture)
        .collect();
    // Accounting sets are already computed above (executed, substituted,
    // omitted); validation happens before any coverage check.
    // Substitution validation (RFC-0006.6): justification present; target is a
    // core fixture; substitute differs; no duplicate records; one substitute
    // record per test; no substitute reused across records (one execution
    // must not silently discharge two obligations); a substitute must not
    // itself be a core fixture (discharge ambiguity).
    let mut substitution_targets = HashSet::new();
    let mut substitute_uses = HashSet::new();
    for substitution in &input.substitutions {
        if substitution.justification.trim().is_empty() {
            return Err(ConformanceError::UnjustifiedSubstitution(substitution.test));
        }
        if !CORE_COMPOSITION_TESTS.contains(&substitution.test)
            || substitution.substitute == substitution.test
            || !substitution_targets.insert(substitution.test)
        {
            return Err(ConformanceError::InvalidSubstitution(substitution.test));
        }
        if !substitute_uses.insert(substitution.substitute) {
            return Err(ConformanceError::InvalidSubstitution(substitution.test));
        }
        if CORE_COMPOSITION_TESTS.contains(&substitution.substitute) {
            return Err(ConformanceError::InvalidSubstitution(substitution.test));
        }
    }

    // Omission validation: core fixtures only, no duplicates, no settled
    // fixture both accounted for and omitted.
    let mut omission_ids = HashSet::new();
    for omission in &input.omitted {
        if !CORE_COMPOSITION_TESTS.contains(&omission.fixture) {
            return Err(ConformanceError::OmittedNonCoreFixture(omission.fixture));
        }
        if !omission_ids.insert(omission.fixture) {
            return Err(ConformanceError::DuplicateAccounting(omission.fixture));
        }
        if executed.contains(omission.fixture) || substituted.contains(omission.fixture) {
            return Err(ConformanceError::DuplicateAccounting(omission.fixture));
        }
    }

    // Core results may contain only core fixtures or declared substitutes.
    let substitute_targets: HashSet<&str> = input
        .substitutions
        .iter()
        .map(|substitution| substitution.substitute)
        .collect();
    for result in &input.core_results {
        if !CORE_COMPOSITION_TESTS.contains(&result.fixture)
            && !substitute_targets.contains(&result.fixture)
        {
            return Err(ConformanceError::UnexpectedFixture(result.fixture));
        }
    }

    for core in CORE_COMPOSITION_TESTS {
        if !executed.contains(core) && !substituted.contains(core) && !omitted.contains(core) {
            return Err(ConformanceError::MissingCoreTest(core));
        }
    }

    // Substitutes declared must have been executed (RFC-0006.6).
    for substitution in &input.substitutions {
        if !executed.contains(substitution.substitute) {
            return Err(ConformanceError::SubstitutionNotExecuted(
                substitution.substitute,
            ));
        }
    }

    // Profile coverage (RFC-0006.2): a claim requires a non-empty subset.
    if let Some(profile) = input.profile {
        if input.profile_subset.is_empty() {
            return Err(ConformanceError::MissingProfileCoverage(profile));
        }
    }

    Ok(())
}
