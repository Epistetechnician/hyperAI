#![allow(missing_docs)]

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{CanonicalizationError, ClaimId, Sha256Digest, UtcTimestamp, domain_separated_digest};

/// Canonical schema identifier for the first challenge descriptor.
pub const CHALLENGE_DESCRIPTOR_SCHEMA_V1: &str = "hsai.challenge-descriptor.v1";
/// Canonical schema identifier for the first claim specification.
pub const CLAIM_SPEC_SCHEMA_V1: &str = "hsai.claim-spec.v1";
/// Digest domain used to derive the immutable claim identity.
pub const CLAIM_SPEC_DIGEST_DOMAIN_V1: &str = "hsai.claim-spec.v1";

pub type Extensions = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChallengeDescriptorV1 {
    pub schema_version: String,
    pub challenge_id: String,
    pub source_platform: String,
    pub source_namespace: String,
    pub source_challenge_ref: String,
    pub display_name: String,
    pub canonical_spec_uri: String,
    pub canonical_spec_digest: Sha256Digest,
    pub source_terms_uri: String,
    pub source_timezone: String,
    pub source_adapter_kind: String,
    pub source_adapter_version: String,
    pub created_at: UtcTimestamp,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub display_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PredicateKind {
    ExistsEligibleValidatorPass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PredicateV1 {
    pub kind: PredicateKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DuplicateSemantics {
    IgnoreExactDuplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReplacementSemantics {
    ImmutableSourceRevision,
    LatestObservedBeforeDeadline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EligibleSubmissionPolicyV1 {
    pub accepted_source_namespaces: Vec<String>,
    pub allowed_artifact_media_types: Vec<String>,
    pub duplicate_semantics: DuplicateSemantics,
    pub replacement_semantics: ReplacementSemantics,
    pub withdrawn_submissions_count: bool,
    pub max_artifact_bytes: u64,
    pub disclosure_constraints: Vec<String>,
    pub allow_fetch_after_deadline_if_observed_before: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateIdentityPolicyV1 {
    pub source_identity_required: bool,
    pub content_digest_algorithm: String,
    pub author_identity_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationPolicyV1 {
    pub source_adapter_id: String,
    pub source_adapter_version: String,
    pub cursor_required: bool,
    pub raw_response_digest_required: bool,
    pub pagination_completeness_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkPolicy {
    Disabled,
    Allowlisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RandomnessPolicy {
    Forbidden,
    DeterministicSeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClockPolicy {
    InjectedLogicalTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeterminismTier {
    ByteIdentical,
    SemanticIdentical,
    QuorumOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimitsV1 {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub process_count: u32,
    pub file_descriptor_count: u32,
    pub output_bytes: u64,
    pub disk_bytes: u64,
    pub wall_clock_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResultMappingV1 {
    pub accepted_status: String,
    pub rejected_status: String,
    pub inconclusive_status: String,
    pub unspecified_is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationPolicyV1 {
    pub validator_artifact_digest: Sha256Digest,
    pub validator_interface_version: String,
    pub environment_digest: Sha256Digest,
    pub argv: Vec<String>,
    pub cpu_architecture: String,
    pub required_instruction_sets: Vec<String>,
    pub os_runtime_versions: BTreeMap<String, String>,
    pub filesystem_layout_version: String,
    pub network_policy: NetworkPolicy,
    pub network_allowlist: Vec<String>,
    pub environment_allowlist: Vec<String>,
    pub locale: String,
    pub timezone: String,
    pub randomness_policy: RandomnessPolicy,
    pub seed_derivation_domain: Option<String>,
    pub clock_policy: ClockPolicy,
    pub resource_limits: ResourceLimitsV1,
    pub retry_count: u16,
    pub output_normalization_version: String,
    pub result_mapping: ResultMappingV1,
    pub determinism_tier: DeterminismTier,
    pub required_replay_count: u16,
    pub required_runner_quorum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct ResolutionPolicyV1 {
    pub policy_version: String,
    pub true_requires_eligible_pass: bool,
    pub false_requires_deadline_and_grace: bool,
    pub false_requires_complete_observation: bool,
    pub false_requires_terminal_candidates: bool,
    pub infrastructure_failure_maps_to_false: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeadlineBoundary {
    Inclusive,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnavailableSourceBehavior {
    RemainUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeadlinePolicyV1 {
    pub authoritative_time_source: String,
    pub boundary: DeadlineBoundary,
    pub submission_deadline: UtcTimestamp,
    pub permitted_source_clock_skew_seconds: u64,
    pub ingestion_grace_period_seconds: u64,
    pub required_source_finality_depth: u64,
    pub unavailable_source_behavior: UnavailableSourceBehavior,
    pub no_solution_requires_completeness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DisputePolicyV1 {
    pub authority_id: String,
    pub window_seconds: u64,
    pub allowed_reason_codes: Vec<String>,
    pub counterexample_candidate_allowed: bool,
    pub unresolved_dispute_blocks_finality: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttestationPolicyV1 {
    pub policy_version: String,
    pub threshold: u16,
    pub registered_attestors: u16,
    pub distinct_logical_signers_required: bool,
    pub allowed_algorithms: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceClassification {
    Public,
    Embargoed,
    OperatorOnly,
    SecretReferenceOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidencePublicationPolicyV1 {
    pub default_classification: EvidenceClassification,
    pub public_verification_required: bool,
    pub hidden_artifact_commitment_required: bool,
    pub disclosure_policy_digest: Option<Sha256Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutcomeMappingV1 {
    pub true_settlement_fraction: String,
    pub false_settlement_fraction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimSpecDraftV1 {
    pub schema_version: String,
    pub challenge_descriptor_digest: Sha256Digest,
    pub question: String,
    pub predicate: PredicateV1,
    pub eligible_submission_policy: EligibleSubmissionPolicyV1,
    pub candidate_identity_policy: CandidateIdentityPolicyV1,
    pub observation_policy: ObservationPolicyV1,
    pub verification_policy: VerificationPolicyV1,
    pub resolution_policy: ResolutionPolicyV1,
    pub deadline_policy: DeadlinePolicyV1,
    pub dispute_policy: DisputePolicyV1,
    pub attestation_policy: AttestationPolicyV1,
    pub evidence_publication_policy: EvidencePublicationPolicyV1,
    pub outcome_mapping: OutcomeMappingV1,
    pub created_at: UtcTimestamp,
    pub freeze_not_before: UtcTimestamp,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimSpecV1 {
    pub schema_version: String,
    pub claim_id: ClaimId,
    pub challenge_descriptor_digest: Sha256Digest,
    pub question: String,
    pub predicate: PredicateV1,
    pub eligible_submission_policy: EligibleSubmissionPolicyV1,
    pub candidate_identity_policy: CandidateIdentityPolicyV1,
    pub observation_policy: ObservationPolicyV1,
    pub verification_policy: VerificationPolicyV1,
    pub resolution_policy: ResolutionPolicyV1,
    pub deadline_policy: DeadlinePolicyV1,
    pub dispute_policy: DisputePolicyV1,
    pub attestation_policy: AttestationPolicyV1,
    pub evidence_publication_policy: EvidencePublicationPolicyV1,
    pub outcome_mapping: OutcomeMappingV1,
    pub created_at: UtcTimestamp,
    pub freeze_not_before: UtcTimestamp,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: Extensions,
}

#[derive(Debug, Error)]
pub enum ClaimSpecError {
    #[error("unsupported claim schema version {0}")]
    UnsupportedSchema(String),
    #[error("question must be non-empty human-readable presentation text")]
    EmptyQuestion,
    #[error("at least one accepted source namespace is required")]
    MissingSourceNamespace,
    #[error("at least one allowed artifact media type is required")]
    MissingArtifactType,
    #[error("maximum artifact size must be positive")]
    InvalidArtifactLimit,
    #[error("validator argv must contain a non-empty executable and no empty argument")]
    InvalidArgv,
    #[error("validator resource limits must all be positive")]
    InvalidResourceLimits,
    #[error("deterministic replay and runner quorum counts must be positive")]
    InvalidReplayPolicy,
    #[error("network allowlist must be empty when network policy is disabled")]
    InvalidNetworkPolicy,
    #[error("attestation threshold must be positive and no greater than registered attestors")]
    InvalidAttestationPolicy,
    #[error("resolution policy may not map infrastructure failure to false")]
    UnsafeFailureMapping,
    #[error("negative resolution must require deadline, completeness, and terminal candidates")]
    UnsafeNegativeResolution,
    #[error("outcome settlement fractions must be distinct canonical decimals in [0,1]")]
    InvalidOutcomeMapping,
    #[error("extension key {0} is not namespaced")]
    UnnamespacedExtension(String),
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    #[error("claim ID does not match the canonical claim specification")]
    IdentityMismatch,
}

impl ClaimSpecDraftV1 {
    /// Validate frozen semantic invariants before identity derivation.
    ///
    /// # Errors
    ///
    /// Returns a typed policy or schema error when any fail-closed invariant is
    /// absent or unsafe.
    pub fn validate(&self) -> Result<(), ClaimSpecError> {
        if self.schema_version != CLAIM_SPEC_SCHEMA_V1 {
            return Err(ClaimSpecError::UnsupportedSchema(
                self.schema_version.clone(),
            ));
        }
        if self.question.trim().is_empty() {
            return Err(ClaimSpecError::EmptyQuestion);
        }
        if self
            .eligible_submission_policy
            .accepted_source_namespaces
            .is_empty()
        {
            return Err(ClaimSpecError::MissingSourceNamespace);
        }
        if self
            .eligible_submission_policy
            .allowed_artifact_media_types
            .is_empty()
        {
            return Err(ClaimSpecError::MissingArtifactType);
        }
        if self.eligible_submission_policy.max_artifact_bytes == 0 {
            return Err(ClaimSpecError::InvalidArtifactLimit);
        }
        if self.verification_policy.argv.is_empty()
            || self.verification_policy.argv.iter().any(String::is_empty)
        {
            return Err(ClaimSpecError::InvalidArgv);
        }
        let limits = &self.verification_policy.resource_limits;
        if limits.cpu_millis == 0
            || limits.memory_bytes == 0
            || limits.process_count == 0
            || limits.file_descriptor_count == 0
            || limits.output_bytes == 0
            || limits.disk_bytes == 0
            || limits.wall_clock_millis == 0
        {
            return Err(ClaimSpecError::InvalidResourceLimits);
        }
        if self.verification_policy.required_replay_count == 0
            || self.verification_policy.required_runner_quorum == 0
        {
            return Err(ClaimSpecError::InvalidReplayPolicy);
        }
        if self.verification_policy.network_policy == NetworkPolicy::Disabled
            && !self.verification_policy.network_allowlist.is_empty()
        {
            return Err(ClaimSpecError::InvalidNetworkPolicy);
        }
        if self.attestation_policy.threshold == 0
            || self.attestation_policy.threshold > self.attestation_policy.registered_attestors
        {
            return Err(ClaimSpecError::InvalidAttestationPolicy);
        }
        if self.resolution_policy.infrastructure_failure_maps_to_false {
            return Err(ClaimSpecError::UnsafeFailureMapping);
        }
        if !self.resolution_policy.false_requires_deadline_and_grace
            || !self.resolution_policy.false_requires_complete_observation
            || !self.resolution_policy.false_requires_terminal_candidates
        {
            return Err(ClaimSpecError::UnsafeNegativeResolution);
        }
        let true_fraction = canonical_fraction(&self.outcome_mapping.true_settlement_fraction);
        let false_fraction = canonical_fraction(&self.outcome_mapping.false_settlement_fraction);
        if !true_fraction
            || !false_fraction
            || self.outcome_mapping.true_settlement_fraction
                == self.outcome_mapping.false_settlement_fraction
        {
            return Err(ClaimSpecError::InvalidOutcomeMapping);
        }
        if let Some(key) = self.extensions.keys().find(|key| !is_namespaced(key)) {
            return Err(ClaimSpecError::UnnamespacedExtension(key.clone()));
        }
        Ok(())
    }

    /// Validate and freeze the draft with a derived content identity.
    ///
    /// # Errors
    ///
    /// Returns a validation or canonicalization error when the draft cannot be
    /// frozen safely.
    pub fn freeze(self) -> Result<ClaimSpecV1, ClaimSpecError> {
        self.validate()?;
        let digest = domain_separated_digest(CLAIM_SPEC_DIGEST_DOMAIN_V1, &self)?;
        let claim_id = ClaimId::from_digest(digest);
        Ok(ClaimSpecV1 {
            schema_version: self.schema_version,
            claim_id,
            challenge_descriptor_digest: self.challenge_descriptor_digest,
            question: self.question,
            predicate: self.predicate,
            eligible_submission_policy: self.eligible_submission_policy,
            candidate_identity_policy: self.candidate_identity_policy,
            observation_policy: self.observation_policy,
            verification_policy: self.verification_policy,
            resolution_policy: self.resolution_policy,
            deadline_policy: self.deadline_policy,
            dispute_policy: self.dispute_policy,
            attestation_policy: self.attestation_policy,
            evidence_publication_policy: self.evidence_publication_policy,
            outcome_mapping: self.outcome_mapping,
            created_at: self.created_at,
            freeze_not_before: self.freeze_not_before,
            extensions: self.extensions,
        })
    }
}

impl ClaimSpecV1 {
    /// Reconstruct the semantic payload that excludes the derived claim ID.
    #[must_use]
    pub fn identity_payload(&self) -> ClaimSpecDraftV1 {
        ClaimSpecDraftV1 {
            schema_version: self.schema_version.clone(),
            challenge_descriptor_digest: self.challenge_descriptor_digest,
            question: self.question.clone(),
            predicate: self.predicate.clone(),
            eligible_submission_policy: self.eligible_submission_policy.clone(),
            candidate_identity_policy: self.candidate_identity_policy.clone(),
            observation_policy: self.observation_policy.clone(),
            verification_policy: self.verification_policy.clone(),
            resolution_policy: self.resolution_policy.clone(),
            deadline_policy: self.deadline_policy.clone(),
            dispute_policy: self.dispute_policy.clone(),
            attestation_policy: self.attestation_policy.clone(),
            evidence_publication_policy: self.evidence_publication_policy.clone(),
            outcome_mapping: self.outcome_mapping.clone(),
            created_at: self.created_at.clone(),
            freeze_not_before: self.freeze_not_before.clone(),
            extensions: self.extensions.clone(),
        }
    }

    /// Recompute the canonical domain-separated claim-spec digest.
    ///
    /// # Errors
    ///
    /// Returns a validation or canonicalization error when the stored semantics
    /// are no longer a valid v1 claim specification.
    pub fn claim_spec_digest(&self) -> Result<Sha256Digest, ClaimSpecError> {
        let payload = self.identity_payload();
        payload.validate()?;
        Ok(domain_separated_digest(
            CLAIM_SPEC_DIGEST_DOMAIN_V1,
            &payload,
        )?)
    }

    /// Verify that the stored derived ID matches the frozen semantics.
    ///
    /// # Errors
    ///
    /// Returns [`ClaimSpecError::IdentityMismatch`] for a changed derived ID and
    /// forwards semantic validation failures.
    pub fn verify_identity(&self) -> Result<(), ClaimSpecError> {
        let expected = ClaimId::from_digest(self.claim_spec_digest()?);
        if expected == self.claim_id {
            Ok(())
        } else {
            Err(ClaimSpecError::IdentityMismatch)
        }
    }
}

fn canonical_fraction(value: &str) -> bool {
    if matches!(value, "0" | "1") {
        return true;
    }
    let Some(digits) = value.strip_prefix("0.") else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) && !digits.ends_with('0')
}

fn is_namespaced(key: &str) -> bool {
    let mut parts = key.split('.');
    matches!((parts.next(), parts.next()), (Some(first), Some(second)) if !first.is_empty() && !second.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: &str) -> UtcTimestamp {
        match value.parse() {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("test timestamp must be valid: {error}"),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn fixture_draft() -> ClaimSpecDraftV1 {
        ClaimSpecDraftV1 {
            schema_version: CLAIM_SPEC_SCHEMA_V1.to_owned(),
            challenge_descriptor_digest: Sha256Digest::from_bytes([1; 32]),
            question: "Did an eligible submission pass the pinned verifier before the deadline?"
                .to_owned(),
            predicate: PredicateV1 {
                kind: PredicateKind::ExistsEligibleValidatorPass,
            },
            eligible_submission_policy: EligibleSubmissionPolicyV1 {
                accepted_source_namespaces: vec!["local.demo".to_owned()],
                allowed_artifact_media_types: vec!["application/octet-stream".to_owned()],
                duplicate_semantics: DuplicateSemantics::IgnoreExactDuplicate,
                replacement_semantics: ReplacementSemantics::ImmutableSourceRevision,
                withdrawn_submissions_count: false,
                max_artifact_bytes: 1_048_576,
                disclosure_constraints: Vec::new(),
                allow_fetch_after_deadline_if_observed_before: true,
            },
            candidate_identity_policy: CandidateIdentityPolicyV1 {
                source_identity_required: true,
                content_digest_algorithm: "sha256".to_owned(),
                author_identity_required: false,
            },
            observation_policy: ObservationPolicyV1 {
                source_adapter_id: "local-submission-ledger".to_owned(),
                source_adapter_version: "1.0.0".to_owned(),
                cursor_required: true,
                raw_response_digest_required: true,
                pagination_completeness_required: true,
            },
            verification_policy: VerificationPolicyV1 {
                validator_artifact_digest: Sha256Digest::from_bytes([2; 32]),
                validator_interface_version: "hsai.validator.result.v1".to_owned(),
                environment_digest: Sha256Digest::from_bytes([3; 32]),
                argv: vec!["/validator/check".to_owned(), "/input/candidate".to_owned()],
                cpu_architecture: "aarch64".to_owned(),
                required_instruction_sets: Vec::new(),
                os_runtime_versions: BTreeMap::from([(
                    "linux".to_owned(),
                    "fixture-v1".to_owned(),
                )]),
                filesystem_layout_version: "hsai.runner.fs.v1".to_owned(),
                network_policy: NetworkPolicy::Disabled,
                network_allowlist: Vec::new(),
                environment_allowlist: vec!["LANG".to_owned(), "TZ".to_owned()],
                locale: "C.UTF-8".to_owned(),
                timezone: "UTC".to_owned(),
                randomness_policy: RandomnessPolicy::DeterministicSeed,
                seed_derivation_domain: Some("hsai.validator.seed.v1".to_owned()),
                clock_policy: ClockPolicy::InjectedLogicalTime,
                resource_limits: ResourceLimitsV1 {
                    cpu_millis: 10_000,
                    memory_bytes: 268_435_456,
                    process_count: 16,
                    file_descriptor_count: 64,
                    output_bytes: 1_048_576,
                    disk_bytes: 134_217_728,
                    wall_clock_millis: 30_000,
                },
                retry_count: 0,
                output_normalization_version: "hsai.validator.normalize.v1".to_owned(),
                result_mapping: ResultMappingV1 {
                    accepted_status: "PASS".to_owned(),
                    rejected_status: "FAIL".to_owned(),
                    inconclusive_status: "INCONCLUSIVE".to_owned(),
                    unspecified_is_error: true,
                },
                determinism_tier: DeterminismTier::SemanticIdentical,
                required_replay_count: 2,
                required_runner_quorum: 2,
            },
            resolution_policy: ResolutionPolicyV1 {
                policy_version: "hsai.existential-resolution.v1".to_owned(),
                true_requires_eligible_pass: true,
                false_requires_deadline_and_grace: true,
                false_requires_complete_observation: true,
                false_requires_terminal_candidates: true,
                infrastructure_failure_maps_to_false: false,
            },
            deadline_policy: DeadlinePolicyV1 {
                authoritative_time_source: "local-ledger-receipt-time".to_owned(),
                boundary: DeadlineBoundary::Inclusive,
                submission_deadline: timestamp("2026-07-10T12:00:00Z"),
                permitted_source_clock_skew_seconds: 0,
                ingestion_grace_period_seconds: 300,
                required_source_finality_depth: 1,
                unavailable_source_behavior: UnavailableSourceBehavior::RemainUnresolved,
                no_solution_requires_completeness: true,
            },
            dispute_policy: DisputePolicyV1 {
                authority_id: "local-demo-operator".to_owned(),
                window_seconds: 600,
                allowed_reason_codes: vec!["COUNTEREXAMPLE".to_owned()],
                counterexample_candidate_allowed: true,
                unresolved_dispute_blocks_finality: true,
            },
            attestation_policy: AttestationPolicyV1 {
                policy_version: "hsai.attestation-policy.v1".to_owned(),
                threshold: 2,
                registered_attestors: 3,
                distinct_logical_signers_required: true,
                allowed_algorithms: vec!["Ed25519".to_owned()],
            },
            evidence_publication_policy: EvidencePublicationPolicyV1 {
                default_classification: EvidenceClassification::Public,
                public_verification_required: true,
                hidden_artifact_commitment_required: true,
                disclosure_policy_digest: None,
            },
            outcome_mapping: OutcomeMappingV1 {
                true_settlement_fraction: "1".to_owned(),
                false_settlement_fraction: "0".to_owned(),
            },
            created_at: timestamp("2026-07-10T10:00:00Z"),
            freeze_not_before: timestamp("2026-07-10T10:00:00Z"),
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn contract_freeze_derives_and_verifies_identity() {
        let claim = fixture_draft().freeze();
        assert!(
            claim
                .as_ref()
                .is_ok_and(|claim| claim.verify_identity().is_ok())
        );
    }

    #[test]
    fn contract_identity_changes_when_semantics_change() {
        let first = fixture_draft().freeze();
        let mut changed = fixture_draft();
        changed.deadline_policy.submission_deadline = timestamp("2026-07-10T12:00:01Z");
        let second = changed.freeze();
        assert!(
            matches!((first, second), (Ok(left), Ok(right)) if left.claim_id != right.claim_id)
        );
    }

    #[test]
    fn contract_unknown_resolution_field_is_rejected() {
        let claim = fixture_draft().freeze();
        let mut json = match claim.and_then(|value| {
            serde_json::to_value(value)
                .map_err(CanonicalizationError::from)
                .map_err(ClaimSpecError::from)
        }) {
            Ok(value) => value,
            Err(error) => unreachable!("fixture must serialize: {error}"),
        };
        if let Some(object) = json.as_object_mut() {
            object.insert("silent_override".to_owned(), Value::Bool(true));
        }
        assert!(serde_json::from_value::<ClaimSpecV1>(json).is_err());
    }

    #[test]
    fn contract_infrastructure_error_cannot_map_to_false() {
        let mut draft = fixture_draft();
        draft.resolution_policy.infrastructure_failure_maps_to_false = true;
        assert!(matches!(
            draft.freeze(),
            Err(ClaimSpecError::UnsafeFailureMapping)
        ));
    }

    #[test]
    fn contract_extensions_must_be_namespaced() {
        let mut draft = fixture_draft();
        draft
            .extensions
            .insert("unsafe".to_owned(), Value::Bool(true));
        assert!(matches!(
            draft.freeze(),
            Err(ClaimSpecError::UnnamespacedExtension(_))
        ));
    }
}
