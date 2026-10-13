use std::collections::BTreeMap;

use hyperintel_assurance::{
    AssuranceRootClassV1, Ed25519MessageVerificationInputV1, SignatureError,
    VerificationKeyRegistry, verify_ed25519_message_v1,
};
use hyperintel_core::{
    CanonicalizationError, CapacityOfferV1, ExactArithmeticError, PartsPerMillion, ProtocolId,
    Sha256Digest, UtcTimestamp, canonical_json, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Frozen conversion from observed counts to a success rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReliabilityProbabilityMethodV1 {
    /// `floor(successful_observations * 1_000_000 / total_observations)`.
    ObservedSuccessFractionFloorPpm,
}

/// Inputs used to construct a typed reliability artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReliabilityArtifactDraftV1 {
    /// Provider whose observations were counted.
    pub provider_id: ProtocolId,
    /// Exact service whose observations were counted.
    pub service_spec_digest: Sha256Digest,
    /// Commitment to the counting and inclusion procedure.
    pub observation_policy_digest: Sha256Digest,
    /// Commitment to the source observation set.
    pub source_observation_digest: Sha256Digest,
    /// Inclusive beginning of the observation window.
    pub observation_window_start: UtcTimestamp,
    /// Inclusive end of the observation window.
    pub observation_window_end: UtcTimestamp,
    /// First instant at which the artifact may be used.
    pub valid_from: UtcTimestamp,
    /// Last instant at which the artifact may be used.
    pub valid_until: UtcTimestamp,
    /// Observations classified as successful by the committed policy.
    pub successful_observations: u64,
    /// All observations admitted by the committed policy.
    pub total_observations: u64,
}

/// Canonical reliability statement whose probability is derived from exact counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReliabilityArtifactV1 {
    /// Fixed artifact schema.
    pub schema_version: String,
    /// Provider whose observations were counted.
    pub provider_id: ProtocolId,
    /// Exact service whose observations were counted.
    pub service_spec_digest: Sha256Digest,
    /// Commitment to the counting and inclusion procedure.
    pub observation_policy_digest: Sha256Digest,
    /// Commitment to the source observation set.
    pub source_observation_digest: Sha256Digest,
    /// Inclusive beginning of the observation window.
    pub observation_window_start: UtcTimestamp,
    /// Inclusive end of the observation window.
    pub observation_window_end: UtcTimestamp,
    /// First instant at which the artifact may be used.
    pub valid_from: UtcTimestamp,
    /// Last instant at which the artifact may be used.
    pub valid_until: UtcTimestamp,
    /// Observations classified as successful by the committed policy.
    pub successful_observations: u64,
    /// All observations admitted by the committed policy.
    pub total_observations: u64,
    /// Frozen count-to-rate conversion.
    pub probability_method: ReliabilityProbabilityMethodV1,
    /// Exact probability derived under `probability_method`.
    pub observed_success_probability: PartsPerMillion,
    /// Domain-separated digest of every preceding field.
    pub artifact_digest: Sha256Digest,
}

#[derive(Serialize)]
struct ReliabilityArtifactIdentity<'a> {
    schema_version: &'a str,
    provider_id: &'a ProtocolId,
    service_spec_digest: Sha256Digest,
    observation_policy_digest: Sha256Digest,
    source_observation_digest: Sha256Digest,
    observation_window_start: &'a UtcTimestamp,
    observation_window_end: &'a UtcTimestamp,
    valid_from: &'a UtcTimestamp,
    valid_until: &'a UtcTimestamp,
    successful_observations: u64,
    total_observations: u64,
    probability_method: ReliabilityProbabilityMethodV1,
    observed_success_probability: PartsPerMillion,
}

impl ReliabilityArtifactV1 {
    /// Recompute the artifact identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        reliability_artifact_digest(self)
    }
}

/// Reliability-specific verifier signature metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReliabilityVerifierStatementV1 {
    /// Fixed reliability statement schema.
    pub schema_version: String,
    /// Exact typed artifact identity being signed.
    pub artifact_digest: Sha256Digest,
    /// Exact counting and inclusion procedure commitment being signed.
    pub observation_policy_digest: Sha256Digest,
    /// First valid instant.
    pub issued_at: UtcTimestamp,
    /// Last valid instant.
    pub expires_at: UtcTimestamp,
    /// Monotonic logical sequence chosen by the reliability verifier.
    pub sequence: u64,
    /// Reliability verifier identity.
    pub signer_id: ProtocolId,
    /// Caller-classified verifier key identity.
    pub key_id: ProtocolId,
    /// Fixed signature algorithm.
    pub algorithm: String,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

#[derive(Serialize)]
struct ReliabilityVerifierSigningMessage<'a> {
    schema_version: &'a str,
    artifact_digest: Sha256Digest,
    observation_policy_digest: Sha256Digest,
    issued_at: &'a UtcTimestamp,
    expires_at: &'a UtcTimestamp,
    sequence: u64,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl ReliabilityVerifierStatementV1 {
    /// Return canonical domain-specific signing bytes, excluding the signature field.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, CanonicalizationError> {
        canonical_json(&ReliabilityVerifierSigningMessage {
            schema_version: &self.schema_version,
            artifact_digest: self.artifact_digest,
            observation_policy_digest: self.observation_policy_digest,
            issued_at: &self.issued_at,
            expires_at: &self.expires_at,
            sequence: self.sequence,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        })
    }
}

/// Reliability artifact and verifier signature over its exact typed identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReliabilityArtifactV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact typed reliability artifact.
    pub artifact: ReliabilityArtifactV1,
    /// Caller-trusted verifier statement over `artifact_digest`.
    pub statement: ReliabilityVerifierStatementV1,
}

/// Closed reliability-artifact validation failures.
#[derive(Debug, Error)]
pub enum ReliabilityArtifactError {
    /// Artifact or wrapper schema is unsupported.
    #[error("unsupported reliability artifact schema")]
    UnsupportedSchema,
    /// Count-to-probability conversion is unsupported.
    #[error("unsupported reliability probability method")]
    UnsupportedProbabilityMethod,
    /// The referenced artifact is absent.
    #[error("referenced reliability artifact is absent")]
    MissingArtifact,
    /// A required policy, source, service, or artifact digest is zero.
    #[error("reliability artifact contains a zero required digest")]
    ZeroDigest,
    /// The caller's content-addressed map key differs from artifact identity.
    #[error("reliability artifact map key mismatch")]
    MapKeyMismatch,
    /// The embedded artifact digest is not its canonical identity.
    #[error("reliability artifact self-digest mismatch")]
    ArtifactDigestMismatch,
    /// Artifact provider differs from the signed capacity offer.
    #[error("reliability artifact provider binding mismatch")]
    ProviderBindingMismatch,
    /// Artifact service differs from the signed capacity offer.
    #[error("reliability artifact service binding mismatch")]
    ServiceBindingMismatch,
    /// The offer does not reference the artifact identity.
    #[error("capacity offer reliability digest mismatch")]
    OfferReliabilityDigestMismatch,
    /// Observation window is empty, inverted, or ends after validity begins.
    #[error("invalid reliability observation window")]
    InvalidObservationWindow,
    /// Artifact validity is empty or evaluation is outside it.
    #[error("reliability artifact is outside its validity window")]
    OutsideValidity,
    /// Offer validity exceeds the reliability artifact validity.
    #[error("capacity offer validity is not contained by reliability validity")]
    OfferValidityNotContained,
    /// No observations were supplied.
    #[error("reliability artifact has no observations")]
    EmptyObservationSet,
    /// Successful observations exceed all observations.
    #[error("reliability successes exceed total observations")]
    SuccessCountExceedsTotal,
    /// Artifact or offer probability differs from the exact count-derived value.
    #[error("reliability probability does not match exact observed counts")]
    ProbabilityMismatch,
    /// Verifier statement schema, artifact identity, or observation-policy binding differs.
    #[error("reliability verifier statement binding mismatch")]
    StatementBindingMismatch,
    /// Reliability statement sequence is unusable for future replay controls.
    #[error("reliability verifier statement sequence must be nonzero")]
    InvalidSequence,
    /// Reliability verifier statement validity differs from artifact validity.
    #[error("reliability verifier statement validity mismatch")]
    StatementValidityMismatch,
    /// Caller did not classify the statement key as an independent verifier key.
    #[error("reliability statement key is not a caller-trusted verifier key")]
    TrustRootMismatch,
    /// Exact rate construction failed.
    #[error("reliability arithmetic failed: {0}")]
    Arithmetic(#[from] ExactArithmeticError),
    /// Canonical identity construction failed.
    #[error("reliability canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Signature, key validity, or revocation verification failed.
    #[error("reliability signature verification failed: {0}")]
    Signature(#[from] SignatureError),
}

/// Derive a deterministic floor-rounded parts-per-million success rate.
///
/// # Errors
///
/// Returns when the observation set is empty or successes exceed total observations.
pub fn derive_observed_success_probability_v1(
    successful_observations: u64,
    total_observations: u64,
) -> Result<PartsPerMillion, ReliabilityArtifactError> {
    if total_observations == 0 {
        return Err(ReliabilityArtifactError::EmptyObservationSet);
    }
    if successful_observations > total_observations {
        return Err(ReliabilityArtifactError::SuccessCountExceedsTotal);
    }
    let value =
        (u128::from(successful_observations) * 1_000_000_u128) / u128::from(total_observations);
    let value = u32::try_from(value).map_err(|_| ExactArithmeticError::Overflow)?;
    Ok(PartsPerMillion::new(value)?)
}

/// Construct a self-digested artifact and derive its rate from exact counts.
///
/// # Errors
///
/// Returns on invalid counts or canonical identity construction failure. Full temporal,
/// binding, trust, and signature validation is performed by
/// [`verify_signed_reliability_artifact_v1`].
pub fn build_reliability_artifact_v1(
    draft: ReliabilityArtifactDraftV1,
) -> Result<ReliabilityArtifactV1, ReliabilityArtifactError> {
    let observed_success_probability = derive_observed_success_probability_v1(
        draft.successful_observations,
        draft.total_observations,
    )?;
    let mut artifact = ReliabilityArtifactV1 {
        schema_version: "hyperintel.reliability-artifact.v1".to_owned(),
        provider_id: draft.provider_id,
        service_spec_digest: draft.service_spec_digest,
        observation_policy_digest: draft.observation_policy_digest,
        source_observation_digest: draft.source_observation_digest,
        observation_window_start: draft.observation_window_start,
        observation_window_end: draft.observation_window_end,
        valid_from: draft.valid_from,
        valid_until: draft.valid_until,
        successful_observations: draft.successful_observations,
        total_observations: draft.total_observations,
        probability_method: ReliabilityProbabilityMethodV1::ObservedSuccessFractionFloorPpm,
        observed_success_probability,
        artifact_digest: Sha256Digest::from_bytes([0; 32]),
    };
    artifact.artifact_digest = reliability_artifact_digest(&artifact)?;
    Ok(artifact)
}

/// Verify exact artifact identity, offer bindings, counts, validity, caller trust, and signature.
///
/// This does not establish that source observations are true, representative, independently
/// reproduced, stationary, calibrated, or predictive of future provider performance.
///
/// # Errors
///
/// Returns a closed failure for any unsupported, missing, stale, tampered, mismatched,
/// arithmetically inconsistent, untrusted, expired, revoked, or invalidly signed input.
pub fn verify_signed_reliability_artifact_v1(
    signed: &SignedReliabilityArtifactV1,
    map_key: Sha256Digest,
    offer: &CapacityOfferV1,
    evaluated_at: &UtcTimestamp,
    registry: &VerificationKeyRegistry,
) -> Result<(), ReliabilityArtifactError> {
    let artifact = &signed.artifact;
    if signed.schema_version != "hyperintel.signed-reliability-artifact.v1"
        || artifact.schema_version != "hyperintel.reliability-artifact.v1"
    {
        return Err(ReliabilityArtifactError::UnsupportedSchema);
    }
    if artifact.probability_method
        != ReliabilityProbabilityMethodV1::ObservedSuccessFractionFloorPpm
    {
        return Err(ReliabilityArtifactError::UnsupportedProbabilityMethod);
    }
    if artifact.service_spec_digest.is_zero()
        || artifact.observation_policy_digest.is_zero()
        || artifact.source_observation_digest.is_zero()
        || artifact.artifact_digest.is_zero()
    {
        return Err(ReliabilityArtifactError::ZeroDigest);
    }
    let rebuilt_digest = reliability_artifact_digest(artifact)?;
    if rebuilt_digest != artifact.artifact_digest {
        return Err(ReliabilityArtifactError::ArtifactDigestMismatch);
    }
    if map_key != artifact.artifact_digest {
        return Err(ReliabilityArtifactError::MapKeyMismatch);
    }
    if offer.reliability_evidence_digest != artifact.artifact_digest {
        return Err(ReliabilityArtifactError::OfferReliabilityDigestMismatch);
    }
    if artifact.provider_id != offer.provider_id {
        return Err(ReliabilityArtifactError::ProviderBindingMismatch);
    }
    if artifact.service_spec_digest != offer.service_spec_digest {
        return Err(ReliabilityArtifactError::ServiceBindingMismatch);
    }
    if artifact.observation_window_start >= artifact.observation_window_end
        || artifact.observation_window_end > artifact.valid_from
    {
        return Err(ReliabilityArtifactError::InvalidObservationWindow);
    }
    if artifact.valid_from >= artifact.valid_until
        || evaluated_at < &artifact.valid_from
        || evaluated_at > &artifact.valid_until
    {
        return Err(ReliabilityArtifactError::OutsideValidity);
    }
    if offer.valid_from < artifact.valid_from || offer.valid_until > artifact.valid_until {
        return Err(ReliabilityArtifactError::OfferValidityNotContained);
    }
    let expected_probability = derive_observed_success_probability_v1(
        artifact.successful_observations,
        artifact.total_observations,
    )?;
    if artifact.observed_success_probability != expected_probability
        || offer.success_probability != expected_probability
    {
        return Err(ReliabilityArtifactError::ProbabilityMismatch);
    }
    let statement = &signed.statement;
    if statement.schema_version != "hyperintel.reliability-verifier-statement.v1"
        || statement.artifact_digest != artifact.artifact_digest
        || statement.observation_policy_digest != artifact.observation_policy_digest
    {
        return Err(ReliabilityArtifactError::StatementBindingMismatch);
    }
    if statement.sequence == 0 {
        return Err(ReliabilityArtifactError::InvalidSequence);
    }
    if statement.issued_at != artifact.valid_from || statement.expires_at != artifact.valid_until {
        return Err(ReliabilityArtifactError::StatementValidityMismatch);
    }
    if registry.trust_root_class(&statement.signer_id, &statement.key_id)
        != Some(AssuranceRootClassV1::VerifierKey)
    {
        return Err(ReliabilityArtifactError::TrustRootMismatch);
    }
    let message = statement.signing_message()?;
    verify_ed25519_message_v1(
        &Ed25519MessageVerificationInputV1 {
            signer_id: &statement.signer_id,
            key_id: &statement.key_id,
            issued_at: &statement.issued_at,
            expires_at: &statement.expires_at,
            algorithm: &statement.algorithm,
            signature_hex: &statement.signature_hex,
            message: &message,
        },
        registry,
        evaluated_at,
    )?;
    Ok(())
}

/// Resolve and verify the exact reliability artifact referenced by one capacity offer.
///
/// # Errors
///
/// Returns [`ReliabilityArtifactError::MissingArtifact`] when the content-addressed input is
/// absent, then preserves the closed typed failure from full artifact verification.
pub fn verify_offer_reliability_v1<'a>(
    artifacts: &'a BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    offer: &CapacityOfferV1,
    evaluated_at: &UtcTimestamp,
    registry: &VerificationKeyRegistry,
) -> Result<&'a SignedReliabilityArtifactV1, ReliabilityArtifactError> {
    let artifact = artifacts
        .get(&offer.reliability_evidence_digest)
        .ok_or(ReliabilityArtifactError::MissingArtifact)?;
    verify_signed_reliability_artifact_v1(
        artifact,
        offer.reliability_evidence_digest,
        offer,
        evaluated_at,
        registry,
    )?;
    Ok(artifact)
}

fn reliability_artifact_digest(
    artifact: &ReliabilityArtifactV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.reliability-artifact.v1",
        &ReliabilityArtifactIdentity {
            schema_version: &artifact.schema_version,
            provider_id: &artifact.provider_id,
            service_spec_digest: artifact.service_spec_digest,
            observation_policy_digest: artifact.observation_policy_digest,
            source_observation_digest: artifact.source_observation_digest,
            observation_window_start: &artifact.observation_window_start,
            observation_window_end: &artifact.observation_window_end,
            valid_from: &artifact.valid_from,
            valid_until: &artifact.valid_until,
            successful_observations: artifact.successful_observations,
            total_observations: artifact.total_observations,
            probability_method: artifact.probability_method,
            observed_success_probability: artifact.observed_success_probability,
        },
    )
}
