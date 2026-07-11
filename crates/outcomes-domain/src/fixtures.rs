//! Deterministic public fixtures used by generators and hermetic tests.

use std::collections::BTreeMap;

use crate::{
    AttestationPolicyV1, CLAIM_SPEC_SCHEMA_V1, CandidateIdentityPolicyV1, ClaimSpecDraftV1,
    ClockPolicy, DeadlineBoundary, DeadlinePolicyV1, DeterminismTier, DisputePolicyV1,
    DuplicateSemantics, EligibleSubmissionPolicyV1, EvidenceClassification,
    EvidencePublicationPolicyV1, NetworkPolicy, ObservationPolicyV1, OutcomeMappingV1,
    PredicateKind, PredicateV1, RandomnessPolicy, ReplacementSemantics, ResolutionPolicyV1,
    ResourceLimitsV1, ResultMappingV1, Sha256Digest, TimestampError, UnavailableSourceBehavior,
    UtcTimestamp, VerificationPolicyV1,
};

/// Build the deterministic local-demo claim used by schemas, goldens, and E2E tests.
///
/// # Errors
///
/// Returns a timestamp validation error if a checked-in fixture timestamp is
/// accidentally changed to a non-canonical representation.
#[allow(clippy::too_many_lines)]
pub fn local_demo_claim_draft() -> Result<ClaimSpecDraftV1, TimestampError> {
    Ok(ClaimSpecDraftV1 {
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
            os_runtime_versions: BTreeMap::from([("linux".to_owned(), "fixture-v1".to_owned())]),
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
            submission_deadline: parse_timestamp("2026-07-10T12:00:00Z")?,
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
        created_at: parse_timestamp("2026-07-10T10:00:00Z")?,
        freeze_not_before: parse_timestamp("2026-07-10T10:00:00Z")?,
        extensions: BTreeMap::new(),
    })
}

fn parse_timestamp(value: &str) -> Result<UtcTimestamp, TimestampError> {
    value.parse()
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs, path::Path};

    use schemars::schema_for;
    use serde::Serialize;

    use super::*;
    use crate::{
        AdapterCapabilityRecordV1, AdapterHealthV1, AdmissionDecisionV1, AgentHarnessDescriptorV1,
        AgentRunPlanV1, AgentRunResultV1, AgentTraceBundleV1, CandidateEnvelopeV1,
        ChallengeDescriptorV1, ChallengePackageV1, ClaimSpecV1, ContentBlobRefV1,
        CorpusDescriptorV1, CorpusSnapshotV1, DisputeDecisionRecordV1, DisputeRecordV1,
        ExecutionManifestV1, MarketBindingV1, ObservationSnapshotV1, OutcomeEvidenceBundleV1,
        ReconciliationResultV1, ResolutionAttestationV1, ResolutionRecordV1,
        ResolutionSupersessionRecordV1, RightsAndProvenanceAssessmentV1, SettlementHandoffV1,
        SolverViewManifestV1, SubmissionReceiptV1, UnsignedSettlementIntentV1, canonical_json,
    };

    #[test]
    fn contract_generated_assets_match_typed_sources() -> Result<(), Box<dyn Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        macro_rules! assert_schema {
            ($type:ty, $name:literal) => {
                assert_pretty(&root.join(concat!("schemas/", $name)), &schema_for!($type))?;
            };
        }
        assert_schema!(ChallengeDescriptorV1, "challenge-descriptor-v1.schema.json");
        assert_schema!(ClaimSpecV1, "claim-spec-v1.schema.json");
        assert_schema!(
            AdapterCapabilityRecordV1,
            "adapter-capability-record-v1.schema.json"
        );
        assert_schema!(AdapterHealthV1, "adapter-health-v1.schema.json");
        assert_schema!(CorpusDescriptorV1, "corpus-descriptor-v1.schema.json");
        assert_schema!(CorpusSnapshotV1, "corpus-snapshot-v1.schema.json");
        assert_schema!(ChallengePackageV1, "challenge-package-v1.schema.json");
        assert_schema!(SolverViewManifestV1, "solver-view-manifest-v1.schema.json");
        assert_schema!(
            RightsAndProvenanceAssessmentV1,
            "rights-and-provenance-assessment-v1.schema.json"
        );
        assert_schema!(ObservationSnapshotV1, "observation-snapshot-v1.schema.json");
        assert_schema!(CandidateEnvelopeV1, "candidate-envelope-v1.schema.json");
        assert_schema!(
            AgentHarnessDescriptorV1,
            "agent-harness-descriptor-v1.schema.json"
        );
        assert_schema!(AgentRunPlanV1, "agent-run-plan-v1.schema.json");
        assert_schema!(AgentRunResultV1, "agent-run-result-v1.schema.json");
        assert_schema!(AgentTraceBundleV1, "agent-trace-bundle-v1.schema.json");
        assert_schema!(AdmissionDecisionV1, "admission-decision-v1.schema.json");
        assert_schema!(ExecutionManifestV1, "execution-manifest-v1.schema.json");
        assert_schema!(ContentBlobRefV1, "content-blob-ref-v1.schema.json");
        assert_schema!(
            OutcomeEvidenceBundleV1,
            "outcome-evidence-bundle-v1.schema.json"
        );
        assert_schema!(
            ResolutionAttestationV1,
            "resolution-attestation-v1.schema.json"
        );
        assert_schema!(ResolutionRecordV1, "resolution-record-v1.schema.json");
        assert_schema!(DisputeRecordV1, "dispute-record-v1.schema.json");
        assert_schema!(
            DisputeDecisionRecordV1,
            "dispute-decision-record-v1.schema.json"
        );
        assert_schema!(
            ResolutionSupersessionRecordV1,
            "resolution-supersession-record-v1.schema.json"
        );
        assert_schema!(MarketBindingV1, "market-binding-v1.schema.json");
        assert_schema!(
            UnsignedSettlementIntentV1,
            "unsigned-settlement-intent-v1.schema.json"
        );
        assert_schema!(SettlementHandoffV1, "settlement-handoff-v1.schema.json");
        assert_schema!(SubmissionReceiptV1, "submission-receipt-v1.schema.json");
        assert_schema!(
            ReconciliationResultV1,
            "reconciliation-result-v1.schema.json"
        );

        let claim = local_demo_claim_draft()?.freeze()?;
        assert_pretty(
            &root.join("fixtures/claims/local-demo-claim-spec-v1.json"),
            &claim,
        )?;
        assert_eq!(
            fs::read(root.join("fixtures/claims/local-demo-claim-spec-v1.canonical.json"))?,
            canonical_json(&claim)?
        );
        let identity = serde_json::json!({
            "schema_version": "hsai.claim-identity-fixture.v1",
            "claim_id": claim.claim_id,
            "claim_spec_digest": claim.claim_spec_digest()?,
        });
        assert_pretty(
            &root.join("fixtures/claims/local-demo-claim-identity-v1.json"),
            &identity,
        )?;
        Ok(())
    }

    fn assert_pretty(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
        let mut expected = serde_json::to_vec_pretty(value)?;
        expected.push(b'\n');
        assert_eq!(fs::read(path)?, expected);
        Ok(())
    }
}
