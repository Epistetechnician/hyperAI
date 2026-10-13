use std::collections::BTreeSet;

use hyperintel_core::{
    AssuranceMaturity, AssuranceProperty, CanonicalizationError, ProtocolId, Sha256Digest,
    UtcTimestamp, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Assurance statement about one committed subject.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceClaimV1 {
    /// Subject bytes committed by this claim.
    pub subject_digest: Sha256Digest,
    /// Non-compensable property being asserted.
    pub property: AssuranceProperty,
}

/// Class of trust root. Classes remain visible and are never averaged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssuranceRootClassV1 {
    /// Provider-controlled signing key.
    ProviderKey,
    /// Hardware or confidential-compute vendor.
    HardwareVendor,
    /// Independent verifier key.
    VerifierKey,
    /// Buyer-owned policy root.
    BuyerPolicy,
}

/// Named trust root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceRootV1 {
    /// Root class.
    pub class: AssuranceRootClassV1,
    /// Stable root identifier.
    pub root_id: ProtocolId,
}

/// Inclusive validity window.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidityWindowV1 {
    /// First valid instant.
    pub start: UtcTimestamp,
    /// Last valid instant.
    pub end: UtcTimestamp,
}

impl ValidityWindowV1 {
    /// Return whether the window is ordered and contains the evaluation time.
    #[must_use]
    pub fn contains(&self, at: &UtcTimestamp) -> bool {
        self.start <= self.end && &self.start <= at && at <= &self.end
    }

    fn intersect(&self, other: &Self) -> Self {
        Self {
            start: self.start.clone().max(other.start.clone()),
            end: self.end.clone().min(other.end.clone()),
        }
    }
}

/// Typed assurance facts with explicit assumptions and exclusions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceEnvelopeV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Facts asserted by the evidence lane.
    pub guarantees: BTreeSet<AssuranceClaimV1>,
    /// Facts that must hold but are not established here.
    pub assumptions: BTreeSet<AssuranceClaimV1>,
    /// Facts explicitly not established.
    pub exclusions: BTreeSet<AssuranceClaimV1>,
    /// Evidence maturity.
    pub maturity: AssuranceMaturity,
    /// Explicit trust roots.
    pub trust_roots: BTreeSet<AssuranceRootV1>,
    /// Inclusive validity.
    pub valid: ValidityWindowV1,
    /// Source evidence digests composing this envelope.
    pub source_digests: BTreeSet<Sha256Digest>,
}

impl AssuranceEnvelopeV1 {
    /// Validate internal consistency.
    ///
    /// # Errors
    ///
    /// Returns typed rejection reasons for invalid schema, validity, overlap,
    /// or empty provenance.
    pub fn validate(&self) -> Result<(), AssuranceRejectionV1> {
        if self.schema_version != "hyperintel.assurance-envelope.v1" {
            return Err(AssuranceRejectionV1::UnsupportedSchema);
        }
        if self.valid.start > self.valid.end {
            return Err(AssuranceRejectionV1::InvalidValidityWindow);
        }
        if self.source_digests.is_empty() || self.source_digests.iter().any(Sha256Digest::is_zero) {
            return Err(AssuranceRejectionV1::MissingProvenance);
        }
        if self
            .guarantees
            .iter()
            .any(|claim| self.exclusions.contains(claim))
        {
            return Err(AssuranceRejectionV1::GuaranteeExcluded);
        }
        Ok(())
    }

    /// Compute the canonical envelope digest.
    ///
    /// # Errors
    ///
    /// Returns when validation or canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, AssuranceRejectionV1> {
        self.validate()?;
        domain_separated_digest("hyperintel.assurance-envelope.v1", self)
            .map_err(AssuranceRejectionV1::Canonicalization)
    }
}

/// Buyer or router assurance admission policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceAcceptancePolicyV1 {
    /// Required guarantees.
    pub required_guarantees: BTreeSet<AssuranceClaimV1>,
    /// Minimum maturity.
    pub minimum_maturity: AssuranceMaturity,
    /// Whether every assumption must be discharged.
    pub require_closed_assumptions: bool,
    /// Trust-root classes that cause rejection.
    pub forbidden_root_classes: BTreeSet<AssuranceRootClassV1>,
    /// Explicit evaluation time.
    pub evaluated_at: UtcTimestamp,
}

/// Closed assurance rejection taxonomy.
#[derive(Debug, Error)]
pub enum AssuranceRejectionV1 {
    /// Schema is not supported.
    #[error("unsupported assurance schema")]
    UnsupportedSchema,
    /// Validity window is inverted.
    #[error("invalid assurance validity window")]
    InvalidValidityWindow,
    /// Evidence provenance is missing or zero.
    #[error("assurance provenance is missing")]
    MissingProvenance,
    /// A claim is both guaranteed and excluded.
    #[error("assurance claim is simultaneously guaranteed and excluded")]
    GuaranteeExcluded,
    /// One or more guarantees are absent.
    #[error("required assurance guarantee is absent")]
    MissingGuarantee,
    /// Evidence maturity is too weak.
    #[error("assurance maturity is below policy")]
    InsufficientMaturity,
    /// An assumption remains open.
    #[error("assurance contains open assumptions")]
    OpenAssumption,
    /// A forbidden trust-root class is present.
    #[error("assurance uses a forbidden trust-root class")]
    ForbiddenTrustRoot,
    /// The evaluation time is outside the evidence window.
    #[error("assurance is not valid at evaluation time")]
    OutsideValidity,
    /// Canonical identity could not be produced.
    #[error("assurance canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Opaque proof that one envelope passed one policy evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedAssuranceV1 {
    envelope_digest: Sha256Digest,
}

impl ValidatedAssuranceV1 {
    /// Return the admitted envelope digest.
    #[must_use]
    pub const fn envelope_digest(&self) -> Sha256Digest {
        self.envelope_digest
    }
}

/// Compose two envelopes without increasing their maturity.
///
/// Guarantees and exclusions accumulate, mutually satisfied assumptions are
/// discharged, trust roots accumulate, validity intersects, and maturity takes
/// the weaker value.
///
/// # Errors
///
/// Returns if either input or the composed envelope is invalid.
pub fn conjoin_envelopes_v1(
    left: &AssuranceEnvelopeV1,
    right: &AssuranceEnvelopeV1,
) -> Result<AssuranceEnvelopeV1, AssuranceRejectionV1> {
    left.validate()?;
    right.validate()?;
    let guarantees = left
        .guarantees
        .union(&right.guarantees)
        .cloned()
        .collect::<BTreeSet<_>>();
    let assumptions = left
        .assumptions
        .union(&right.assumptions)
        .filter(|claim| !guarantees.contains(*claim))
        .cloned()
        .collect();
    let envelope = AssuranceEnvelopeV1 {
        schema_version: "hyperintel.assurance-envelope.v1".to_owned(),
        guarantees,
        assumptions,
        exclusions: left.exclusions.union(&right.exclusions).cloned().collect(),
        maturity: left.maturity.min(right.maturity),
        trust_roots: left
            .trust_roots
            .union(&right.trust_roots)
            .cloned()
            .collect(),
        valid: left.valid.intersect(&right.valid),
        source_digests: left
            .source_digests
            .union(&right.source_digests)
            .copied()
            .collect(),
    };
    envelope.validate()?;
    Ok(envelope)
}

/// Evaluate an envelope against an explicit assurance policy.
///
/// # Errors
///
/// Returns a closed rejection rather than an authority-bearing boolean.
pub fn evaluate_assurance_v1(
    envelope: &AssuranceEnvelopeV1,
    policy: &AssuranceAcceptancePolicyV1,
) -> Result<ValidatedAssuranceV1, AssuranceRejectionV1> {
    envelope.validate()?;
    if !policy.required_guarantees.is_subset(&envelope.guarantees) {
        return Err(AssuranceRejectionV1::MissingGuarantee);
    }
    if envelope.maturity < policy.minimum_maturity {
        return Err(AssuranceRejectionV1::InsufficientMaturity);
    }
    if policy.require_closed_assumptions && !envelope.assumptions.is_empty() {
        return Err(AssuranceRejectionV1::OpenAssumption);
    }
    if envelope
        .trust_roots
        .iter()
        .any(|root| policy.forbidden_root_classes.contains(&root.class))
    {
        return Err(AssuranceRejectionV1::ForbiddenTrustRoot);
    }
    if !envelope.valid.contains(&policy.evaluated_at) {
        return Err(AssuranceRejectionV1::OutsideValidity);
    }
    Ok(ValidatedAssuranceV1 {
        envelope_digest: envelope.digest()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
        Ok(value.parse()?)
    }

    fn claim(byte: u8, property: AssuranceProperty) -> AssuranceClaimV1 {
        AssuranceClaimV1 {
            subject_digest: Sha256Digest::from_bytes([byte; 32]),
            property,
        }
    }

    fn envelope() -> Result<AssuranceEnvelopeV1, Box<dyn std::error::Error>> {
        Ok(AssuranceEnvelopeV1 {
            schema_version: "hyperintel.assurance-envelope.v1".to_owned(),
            guarantees: [claim(1, AssuranceProperty::ProviderAuthorization)]
                .into_iter()
                .collect(),
            assumptions: BTreeSet::new(),
            exclusions: [claim(1, AssuranceProperty::RuntimeMeasurement)]
                .into_iter()
                .collect(),
            maturity: AssuranceMaturity::LocallyChecked,
            trust_roots: [AssuranceRootV1 {
                class: AssuranceRootClassV1::ProviderKey,
                root_id: "key:provider-alpha".parse()?,
            }]
            .into_iter()
            .collect(),
            valid: ValidityWindowV1 {
                start: timestamp("2026-07-20T00:00:00Z")?,
                end: timestamp("2026-07-21T00:00:00Z")?,
            },
            source_digests: [Sha256Digest::from_bytes([9; 32])].into_iter().collect(),
        })
    }

    #[test]
    fn contract_assurance_admission_is_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
        let envelope = envelope()?;
        let policy = AssuranceAcceptancePolicyV1 {
            required_guarantees: envelope.guarantees.clone(),
            minimum_maturity: AssuranceMaturity::LocallyChecked,
            require_closed_assumptions: true,
            forbidden_root_classes: BTreeSet::new(),
            evaluated_at: timestamp("2026-07-20T12:00:00Z")?,
        };
        assert_eq!(
            evaluate_assurance_v1(&envelope, &policy)?.envelope_digest(),
            envelope.digest()?
        );

        let mut insufficient = policy;
        insufficient.minimum_maturity = AssuranceMaturity::Attested;
        assert!(matches!(
            evaluate_assurance_v1(&envelope, &insufficient),
            Err(AssuranceRejectionV1::InsufficientMaturity)
        ));
        Ok(())
    }

    #[test]
    fn contract_conjoin_only_weakens_and_discharges_assumptions()
    -> Result<(), Box<dyn std::error::Error>> {
        let left = envelope()?;
        let mut right = envelope()?;
        right.guarantees = [claim(2, AssuranceProperty::ServiceManifestBinding)]
            .into_iter()
            .collect();
        right.assumptions = left.guarantees.clone();
        right.maturity = AssuranceMaturity::Attested;
        let composed = conjoin_envelopes_v1(&left, &right)?;
        assert!(composed.assumptions.is_empty());
        assert_eq!(composed.maturity, AssuranceMaturity::LocallyChecked);
        Ok(())
    }
}
