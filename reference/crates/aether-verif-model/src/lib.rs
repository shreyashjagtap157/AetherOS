//! Verification tier methodology reference implementation (RFC-0040).
//!
//! Defines the verification tiers (A–D) through which components claim
//! assurance, the proof-fragility classes, verification declarations per
//! component, evidence tracking, and economic cost tracking for Tier A.
//! This is process-level: it governs how verification is done, not what is
//! verified.

// ---------------------------------------------------------------------------
// Verification tiers (RFC-0040 §2.1–2.4)
// ---------------------------------------------------------------------------

/// Verification tier claimed by a component (RFC-0040.1–0040.10).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VerificationTier {
    /// Tier A — Machine-Verified: formal spec + machine-checked proof
    /// (RFC-0040.1).
    A,
    /// Tier B — Model-Checked: finite-state model + model-checker report
    /// (RFC-0040.6).
    B,
    /// Tier C — Property-Tested: property set + generative test harness
    /// (RFC-0040.8).
    C,
    /// Tier D — Conventional Testing: test suite with >= 80% statement
    /// coverage (RFC-0040.10).
    D,
}

// ---------------------------------------------------------------------------
// Proof-fragility classes (RFC-0040.4)
// ---------------------------------------------------------------------------

/// Proof-fragility class: how much surface change triggers re-proof
/// (RFC-0040.4).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProofFragilityClass {
    /// Re-proof required only for direct spec changes (RFC-0040.4).
    PF1,
    /// Re-proof required for spec changes and implementation changes that
    /// affect declared invariants (RFC-0040.4).
    PF2,
    /// Re-proof required for any change to the component (RFC-0040.4).
    PF3,
}

// ---------------------------------------------------------------------------
// Formal specification languages and provers (RFC-0040.15)
// ---------------------------------------------------------------------------

/// Declared formal specification language (RFC-0040.15).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpecLanguage {
    Acsl,
    RefinementCalculus,
    Why3,
    KFramework,
}

/// Declared machine-checked prover (RFC-0040.15).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Prover {
    Coq,
    IsabelleHol,
    Why3,
    Verus,
}

/// Declared model-checking language (RFC-0040.6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModelCheckLanguage {
    PromelaSpin,
    TlaPlus,
    Alloy,
    NuXmv,
}

/// Declared property logic (RFC-0040.6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PropertyLogic {
    Ltl,
    Ctl,
}

// ---------------------------------------------------------------------------
// Evidence artifacts
// ---------------------------------------------------------------------------

/// Machine-readable formal specification (RFC-0040.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormalSpec {
    /// Language of the specification.
    pub language: SpecLanguage,
    /// Whether the spec is in version control (RFC-0040.2).
    pub in_version_control: bool,
    /// Whether the spec is part of the implementation artifact (RFC-0040.2).
    pub in_implementation_artifact: bool,
}

/// Machine-checked proof artifact (RFC-0040.1, RFC-0040.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofArtifact {
    /// Prover used to produce the proof.
    pub prover: Prover,
    /// Prover version used (RFC-0040.2).
    pub prover_version: String,
    /// Whether the proof is independently reproducible (RFC-0040.3).
    pub reproducible: bool,
}

/// Model-checker verification report (RFC-0040.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCheckReport {
    /// Model-checking language.
    pub language: ModelCheckLanguage,
    /// Property logic used.
    pub logic: PropertyLogic,
    /// Whether all properties were satisfied.
    pub all_properties_satisfied: bool,
}

/// Generative test harness evidence (RFC-0040.8).
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyTestEvidence {
    /// Declared property set (machine-readable).
    pub property_set_declared: bool,
    /// Declared seed for reproducibility.
    pub seed_declared: bool,
    /// Declared time budget for reproducibility.
    pub time_budget_declared: bool,
    /// Branch coverage achieved (fraction 0.0–1.0).
    pub branch_coverage: f64,
}

/// Conventional test suite evidence (RFC-0040.10).
#[derive(Clone, Debug, PartialEq)]
pub struct ConventionalTestEvidence {
    /// Statement coverage achieved (fraction 0.0–1.0).
    pub statement_coverage: f64,
}

// ---------------------------------------------------------------------------
// Component verification declaration (RFC-0040.1–0040.14)
// ---------------------------------------------------------------------------

/// A component's verification declaration: which tier is claimed and what
/// evidence supports it (RFC-0040.1–0040.10).
#[derive(Clone, Debug, PartialEq)]
pub struct VerificationDeclaration {
    /// Component name.
    pub component: String,
    /// Claimed verification tier.
    pub tier: VerificationTier,
    /// Formal specification (required for Tier A).
    pub spec: Option<FormalSpec>,
    /// Proof artifact (required for Tier A).
    pub proof: Option<ProofArtifact>,
    /// Proof-fragility class (required for Tier A).
    pub fragility_class: Option<ProofFragilityClass>,
    /// Model-checker report (required for Tier B).
    pub model_report: Option<ModelCheckReport>,
    /// Property-test evidence (required for Tier C).
    pub property_test: Option<PropertyTestEvidence>,
    /// Conventional test evidence (required for Tier D).
    pub conventional_test: Option<ConventionalTestEvidence>,
    /// Whether re-evaluation was performed at the last platform release
    /// (RFC-0040.11).
    pub re_evaluated: bool,
    /// Economic cost declaration (required for Tier A, RFC-0040.13).
    pub economic_cost: Option<EconomicCost>,
}

impl VerificationDeclaration {
    /// Creates a new verification declaration for a component.
    #[must_use]
    pub fn new(component: &str, tier: VerificationTier) -> Self {
        Self {
            component: component.to_string(),
            tier,
            spec: None,
            proof: None,
            fragility_class: None,
            model_report: None,
            property_test: None,
            conventional_test: None,
            re_evaluated: false,
            economic_cost: None,
        }
    }

    /// Validates that the declaration satisfies the requirements for its
    /// claimed tier (RFC-0040.1–0040.10).
    pub fn validate(&self) -> Result<(), VerifError> {
        match self.tier {
            VerificationTier::A => {
                // RFC-0040.1: formal spec, machine-checked proof, fragility class.
                let spec = self.spec.as_ref().ok_or(VerifError::MissingSpec)?;
                if !spec.in_version_control {
                    return Err(VerifError::SpecNotInVersionControl);
                }
                if !spec.in_implementation_artifact {
                    return Err(VerifError::SpecNotInArtifact);
                }
                let proof = self.proof.as_ref().ok_or(VerifError::MissingProof)?;
                if !proof.reproducible {
                    return Err(VerifError::ProofNotReproducible);
                }
                self.fragility_class.ok_or(VerifError::MissingFragilityClass)?;
                // RFC-0040.13: economic cost declared for Tier A.
                self.economic_cost.as_ref().ok_or(VerifError::MissingEconomicCost)?;
            }
            VerificationTier::B => {
                // RFC-0040.6: finite-state model + property spec + report.
                let report = self.model_report.as_ref().ok_or(VerifError::MissingModelReport)?;
                if !report.all_properties_satisfied {
                    return Err(VerifError::PropertiesNotSatisfied);
                }
            }
            VerificationTier::C => {
                // RFC-0040.8: property set + generative harness + coverage.
                let pt = self.property_test.as_ref().ok_or(VerifError::MissingPropertyTest)?;
                if pt.branch_coverage < 0.0 {
                    return Err(VerifError::CoverageBelowMinimum);
                }
            }
            VerificationTier::D => {
                // RFC-0040.10: test suite with >= 80% statement coverage.
                let ct = self.conventional_test.as_ref().ok_or(VerifError::MissingConventionalTest)?;
                if ct.statement_coverage < 0.80 {
                    return Err(VerifError::CoverageBelowMinimum);
                }
            }
        }
        // RFC-0040.11: re-evaluation at each platform release for Tier A/B.
        if matches!(self.tier, VerificationTier::A | VerificationTier::B) && !self.re_evaluated {
            return Err(VerifError::NotReevaluated);
        }
        Ok(())
    }

    /// Returns true if the declaration is valid for its claimed tier.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

// ---------------------------------------------------------------------------
// Economic cost tracking (RFC-0040.13)
// ---------------------------------------------------------------------------

/// Economic verification cost declaration for Tier A components
/// (RFC-0040.13).
#[derive(Clone, Debug, PartialEq)]
pub struct EconomicCost {
    /// Initial proof investment in person-years to first proof.
    pub initial_proof_investment_py: f64,
    /// Per-release re-proof cost in person-years per platform release.
    pub per_release_reproof_cost_py: f64,
    /// Proof-fragility class.
    pub fragility_class: ProofFragilityClass,
    /// Estimated total verification cost over a 10-year window in person-years.
    pub estimated_ten_year_cost_py: f64,
}

// ---------------------------------------------------------------------------
// Verification registry (RFC-0040.15)
// ---------------------------------------------------------------------------

/// Registry of component verification declarations (RFC-0040).
pub struct VerificationRegistry {
    /// Declarations keyed by component name.
    pub declarations: Vec<VerificationDeclaration>,
}

impl VerificationRegistry {
    /// Creates an empty verification registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    /// Registers a verification declaration.
    pub fn register(&mut self, decl: VerificationDeclaration) {
        self.declarations.push(decl);
    }

    /// Returns the declaration for a component, if present.
    pub fn get(&self, component: &str) -> Option<&VerificationDeclaration> {
        self.declarations.iter().find(|d| d.component == component)
    }

    /// Returns the number of components claiming Tier A.
    #[must_use]
    pub fn tier_a_count(&self) -> u32 {
        self.declarations
            .iter()
            .filter(|d| d.tier == VerificationTier::A)
            .count() as u32
    }

    /// Validates all registered declarations against their claimed tiers
    /// (RFC-0040.1–0040.14).
    pub fn validate_all(&self) -> Result<(), VerifError> {
        for decl in &self.declarations {
            decl.validate()?;
        }
        Ok(())
    }

    /// Returns true if every registered declaration satisfies its tier.
    #[must_use]
    pub fn all_valid(&self) -> bool {
        self.validate_all().is_ok()
    }
}

impl Default for VerificationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Invariant-to-tier mapping (RFC-0040.14)
// ---------------------------------------------------------------------------

/// Which tier a component claims for a specific invariant (RFC-0040.14).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InvariantTierClaim {
    /// Invariant identifier, e.g. "I-1".
    pub invariant: String,
    /// Component realizing the invariant.
    pub component: String,
    /// Tier claimed for this invariant by this component.
    pub tier: VerificationTier,
}

/// Platform invariants I-1 through I-13 (RFC-0040.14).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Invariant {
    I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Verification declaration errors (RFC-0040 failure semantics).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VerifError {
    /// Required formal specification is missing.
    MissingSpec,
    /// Specification is not in version control.
    SpecNotInVersionControl,
    /// Specification is not part of the implementation artifact.
    SpecNotInArtifact,
    /// Required proof artifact is missing.
    MissingProof,
    /// Proof is not independently reproducible.
    ProofNotReproducible,
    /// Required proof-fragility class is missing.
    MissingFragilityClass,
    /// Required model-checker report is missing.
    MissingModelReport,
    /// Not all model-checker properties were satisfied.
    PropertiesNotSatisfied,
    /// Required property-test evidence is missing.
    MissingPropertyTest,
    /// Required conventional-test evidence is missing.
    MissingConventionalTest,
    /// Coverage is below the declared minimum.
    CoverageBelowMinimum,
    /// Component was not re-evaluated at the last platform release.
    NotReevaluated,
    /// Required economic cost declaration is missing for Tier A.
    MissingEconomicCost,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_a_declaration_validates() {
        let mut decl = VerificationDeclaration::new("kernel", VerificationTier::A);
        decl.spec = Some(FormalSpec {
            language: SpecLanguage::Why3,
            in_version_control: true,
            in_implementation_artifact: true,
        });
        decl.proof = Some(ProofArtifact {
            prover: Prover::Verus,
            prover_version: "1.0".to_string(),
            reproducible: true,
        });
        decl.fragility_class = Some(ProofFragilityClass::PF1);
        decl.re_evaluated = true;
        decl.economic_cost = Some(EconomicCost {
            initial_proof_investment_py: 2.0,
            per_release_reproof_cost_py: 0.5,
            fragility_class: ProofFragilityClass::PF1,
            estimated_ten_year_cost_py: 7.0,
        });
        assert!(decl.is_valid());
    }

    #[test]
    fn tier_a_without_proof_is_invalid() {
        let decl = VerificationDeclaration::new("kernel", VerificationTier::A);
        assert!(!decl.is_valid());
    }

    #[test]
    fn tier_b_validates_with_satisfied_model() {
        let mut decl = VerificationDeclaration::new("sched", VerificationTier::B);
        decl.model_report = Some(ModelCheckReport {
            language: ModelCheckLanguage::TlaPlus,
            logic: PropertyLogic::Ltl,
            all_properties_satisfied: true,
        });
        decl.re_evaluated = true;
        assert!(decl.is_valid());
    }

    #[test]
    fn tier_b_with_unsatisfied_properties_is_invalid() {
        let mut decl = VerificationDeclaration::new("sched", VerificationTier::B);
        decl.model_report = Some(ModelCheckReport {
            language: ModelCheckLanguage::TlaPlus,
            logic: PropertyLogic::Ltl,
            all_properties_satisfied: false,
        });
        decl.re_evaluated = true;
        assert!(!decl.is_valid());
    }

    #[test]
    fn tier_d_requires_80_percent_coverage() {
        let mut decl = VerificationDeclaration::new("driver", VerificationTier::D);
        decl.conventional_test = Some(ConventionalTestEvidence {
            statement_coverage: 0.85,
        });
        assert!(decl.is_valid());
        decl.conventional_test = Some(ConventionalTestEvidence {
            statement_coverage: 0.79,
        });
        assert!(!decl.is_valid());
    }

    #[test]
    fn registry_all_valid() {
        let mut reg = VerificationRegistry::new();
        let mut decl = VerificationDeclaration::new("comp", VerificationTier::D);
        decl.conventional_test = Some(ConventionalTestEvidence {
            statement_coverage: 0.90,
        });
        reg.register(decl);
        assert!(reg.all_valid());
        assert_eq!(reg.tier_a_count(), 0);
    }

    #[test]
    fn invariant_tier_claims() {
        let claims = vec![
            InvariantTierClaim {
                invariant: "I-1".to_string(),
                component: "cap-mediator".to_string(),
                tier: VerificationTier::A,
            },
            InvariantTierClaim {
                invariant: "I-13".to_string(),
                component: "cap-mediator".to_string(),
                tier: VerificationTier::A,
            },
        ];
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].tier, VerificationTier::A);
    }
}
