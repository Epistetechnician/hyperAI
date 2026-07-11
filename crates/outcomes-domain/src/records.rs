#![allow(missing_docs)]
// Capability booleans are explicit versioned wire facts, not mutable state.
#![allow(clippy::struct_excessive_bools)]

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ClaimId, ClaimTruth, DeterminismTier, EvidenceClassification, ExecutionOutcome,
    ResolutionFinality, SettlementLifecycle, Sha256Digest, UtcTimestamp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterHealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementReadiness {
    SettlementReady,
    EvaluationOnly,
    CatalogOnly,
    ManualOnly,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RightsStatus {
    Permitted,
    Restricted,
    Ambiguous,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdapterCapabilityRecordV1 {
    pub schema_version: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub discovery_catalog: bool,
    pub immutable_snapshot: bool,
    pub local_materialization: bool,
    pub local_rehosting: bool,
    pub verifier_available: bool,
    pub determinism_tier: Option<DeterminismTier>,
    pub secret_isolation: bool,
    pub candidate_ingestion: bool,
    pub live_observation: bool,
    pub deadline_evidence: bool,
    pub completeness_attestation: bool,
    pub authenticated_access_required: bool,
    pub rights_status: RightsStatus,
    pub supported_categories: Vec<String>,
    pub supported_architectures: Vec<String>,
    pub external_network_required: bool,
    pub ci_fixture_available: bool,
    pub settlement_readiness: SettlementReadiness,
    pub blocking_reason: Option<String>,
    pub verified_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdapterHealthV1 {
    pub adapter_id: String,
    pub status: AdapterHealthStatus,
    pub checked_at: UtcTimestamp,
    pub exact_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusDescriptorV1 {
    pub schema_version: String,
    pub corpus_id: String,
    pub adapter_id: String,
    pub upstream_uri: String,
    pub discovery_ref: String,
    pub rights_status: RightsStatus,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusSnapshotV1 {
    pub schema_version: String,
    pub corpus_id: String,
    pub resolved_commit: String,
    pub snapshot_digest: Sha256Digest,
    pub submodule_digests: BTreeMap<String, Sha256Digest>,
    pub lfs_object_digests: Vec<Sha256Digest>,
    pub image_digests: Vec<Sha256Digest>,
    pub external_artifact_digests: Vec<Sha256Digest>,
    pub adapter_version: String,
    pub received_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PackageVisibility {
    SolverPublic,
    RuntimePrivate,
    VerifierOnly,
    ContaminationSensitive,
    BuildInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageArtifactV1 {
    pub logical_path: String,
    pub digest: Sha256Digest,
    pub byte_length: u64,
    pub media_type: String,
    pub visibility: PackageVisibility,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChallengePackageV1 {
    pub schema_version: String,
    pub package_id: String,
    pub corpus_snapshot_digest: Sha256Digest,
    pub challenge_ref: String,
    pub artifacts: Vec<PackageArtifactV1>,
    pub runtime_manifest_digest: Sha256Digest,
    pub verifier_manifest_digest: Sha256Digest,
    pub solver_view_manifest_digest: Sha256Digest,
    pub rights_assessment_digest: Sha256Digest,
    pub package_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SolverViewManifestV1 {
    pub schema_version: String,
    pub challenge_package_digest: Sha256Digest,
    pub exposed_artifact_digests: Vec<Sha256Digest>,
    pub excluded_secret_digests: Vec<Sha256Digest>,
    pub manifest_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RightsAndProvenanceAssessmentV1 {
    pub schema_version: String,
    pub package_digest: Sha256Digest,
    pub upstream_attribution: Vec<String>,
    pub repository_license: Option<String>,
    pub item_license: Option<String>,
    pub rights_status: RightsStatus,
    pub redistribution_allowed: bool,
    pub execution_allowed: bool,
    pub commercial_market_use_allowed: bool,
    pub exact_limitation: Option<String>,
    pub assessed_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompletenessStatus {
    Complete,
    Incomplete,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationSnapshotV1 {
    pub schema_version: String,
    pub snapshot_id: String,
    pub claim_id: ClaimId,
    pub source_adapter_id: String,
    pub source_adapter_version: String,
    pub cursor_start: Option<String>,
    pub cursor_end: String,
    pub cutoff: UtcTimestamp,
    pub request_parameters_digest: Sha256Digest,
    pub raw_response_digests: Vec<Sha256Digest>,
    pub received_at: UtcTimestamp,
    pub latest_source_time: Option<UtcTimestamp>,
    pub pagination_complete: bool,
    pub retry_count: u16,
    pub completeness: CompletenessStatus,
    pub completeness_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateEnvelopeV1 {
    pub schema_version: String,
    pub candidate_id: String,
    pub claim_id: ClaimId,
    pub source_namespace: String,
    pub source_candidate_ref: String,
    pub source_revision: String,
    pub author_or_team_id: Option<String>,
    pub observed_source_time: UtcTimestamp,
    pub received_at: UtcTimestamp,
    pub artifact_digest: Sha256Digest,
    pub artifact_byte_length: u64,
    pub artifact_media_type: String,
    pub source_provenance_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentHarnessDescriptorV1 {
    pub schema_version: String,
    pub harness_id: String,
    pub harness_version: String,
    pub source_commit: String,
    pub container_digest: Sha256Digest,
    pub supported_entrypoints: Vec<String>,
    pub live_model_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentRunPlanV1 {
    pub schema_version: String,
    pub plan_id: String,
    pub harness_id: String,
    pub challenge_package_digest: Sha256Digest,
    pub solver_view_manifest_digest: Sha256Digest,
    pub configuration_digest: Sha256Digest,
    pub prompt_digests: Vec<Sha256Digest>,
    pub model_provider_identity: String,
    pub tool_allowlist: Vec<String>,
    pub token_budget: u64,
    pub wall_clock_budget_millis: u64,
    pub live_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentRunResultV1 {
    pub schema_version: String,
    pub run_id: String,
    pub plan_id: String,
    pub candidate_digests: Vec<Sha256Digest>,
    pub harness_reward_microunits: Option<i64>,
    pub token_usage: u64,
    pub tool_call_count: u64,
    pub runtime_millis: u64,
    pub completed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentTraceBundleV1 {
    pub schema_version: String,
    pub run_id: String,
    pub trace_blob_digests: Vec<Sha256Digest>,
    pub public_summary_digest: Sha256Digest,
    pub bundle_digest: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionDecisionKind {
    Allow,
    Deny,
    ManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdmissionDecisionV1 {
    pub schema_version: String,
    pub decision_id: String,
    pub claim_spec_digest: Sha256Digest,
    pub candidate_digest: Sha256Digest,
    pub validator_digest: Sha256Digest,
    pub environment_digest: Sha256Digest,
    pub source_provenance_status: String,
    pub artifact_policy_digest: Sha256Digest,
    pub sandbox_policy_digest: Sha256Digest,
    pub resource_budget_digest: Sha256Digest,
    pub secrets_policy_digest: Sha256Digest,
    pub risk_classification: String,
    pub decision: AdmissionDecisionKind,
    pub reason_codes: Vec<String>,
    pub policy_version: String,
    pub decision_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceUsageV1 {
    pub cpu_millis: u64,
    pub peak_memory_bytes: u64,
    pub output_bytes: u64,
    pub disk_bytes: u64,
    pub wall_clock_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionManifestV1 {
    pub schema_version: String,
    pub run_id: String,
    pub claim_spec_digest: Sha256Digest,
    pub candidate_digest: Sha256Digest,
    pub admission_decision_digest: Sha256Digest,
    pub verifier_digest: Sha256Digest,
    pub environment_digest: Sha256Digest,
    pub runner_implementation: String,
    pub runner_build_version: String,
    pub host_compatibility_facts: BTreeMap<String, String>,
    pub argv: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub sandbox_policy_digest: Sha256Digest,
    pub input_manifest_digest: Sha256Digest,
    pub output_manifest_digest: Option<Sha256Digest>,
    pub resource_limits_digest: Sha256Digest,
    pub resource_usage: ResourceUsageV1,
    pub started_at: UtcTimestamp,
    pub ended_at: UtcTimestamp,
    pub logical_time: UtcTimestamp,
    pub raw_exit_status: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    pub out_of_memory: bool,
    pub infrastructure_classification: Option<String>,
    pub stdout_digest: Sha256Digest,
    pub stderr_digest: Sha256Digest,
    pub normalized_outcome: ExecutionOutcome,
    pub normalization_policy_version: String,
    pub determinism_tier: DeterminismTier,
    pub declared_nondeterminism: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentBlobRefV1 {
    pub digest_algorithm: String,
    pub digest: Sha256Digest,
    pub byte_length: u64,
    pub media_type: String,
    pub classification: EvidenceClassification,
    pub storage_locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutcomeEvidenceBundleV1 {
    pub schema_version: String,
    pub claim_id: ClaimId,
    pub claim_spec_digest: Sha256Digest,
    pub challenge_descriptor_digest: Sha256Digest,
    pub observation_snapshot_digests: Vec<Sha256Digest>,
    pub completeness_assessment_digest: Sha256Digest,
    pub qualifying_candidate_digests: Vec<Sha256Digest>,
    pub admission_decision_digests: Vec<Sha256Digest>,
    pub execution_manifest_digests: Vec<Sha256Digest>,
    pub resolution_aggregation_digest: Sha256Digest,
    pub dispute_record_digests: Vec<Sha256Digest>,
    pub code_build_provenance_digest: Sha256Digest,
    pub evidence_publication_policy_digest: Sha256Digest,
    pub blobs: Vec<ContentBlobRefV1>,
    pub bundle_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolutionAttestationV1 {
    pub schema_version: String,
    pub claim_id: ClaimId,
    pub claim_spec_digest: Sha256Digest,
    pub truth: ClaimTruth,
    pub finality: ResolutionFinality,
    pub resolution_policy_digest: Sha256Digest,
    pub evidence_bundle_digest: Sha256Digest,
    pub qualifying_run_digests: Vec<Sha256Digest>,
    pub completeness_evidence_digest: Option<Sha256Digest>,
    pub dispute_window_start: UtcTimestamp,
    pub dispute_window_end: UtcTimestamp,
    pub issued_at: UtcTimestamp,
    pub logical_sequence: u64,
    pub signer_id: String,
    pub key_id: String,
    pub algorithm: String,
    pub signature: String,
    pub supersedes_non_settling_record: Option<Sha256Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolutionRecordV1 {
    pub schema_version: String,
    pub resolution_id: String,
    pub claim_id: ClaimId,
    pub truth: ClaimTruth,
    pub finality: ResolutionFinality,
    pub evidence_bundle_digest: Sha256Digest,
    pub attestation_digests: Vec<Sha256Digest>,
    pub dispute_window_start: UtcTimestamp,
    pub dispute_window_end: UtcTimestamp,
    pub record_digest: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisputeStatus {
    Open,
    Rejected,
    AcceptedSupersededBeforeFinality,
    UnresolvedEscalated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DisputeRecordV1 {
    pub schema_version: String,
    pub dispute_id: String,
    pub claim_id: ClaimId,
    pub provisional_resolution_digest: Sha256Digest,
    pub opened_at: UtcTimestamp,
    pub closes_at: UtcTimestamp,
    pub authenticated_actor_id: String,
    pub reason_code: String,
    pub evidence_digests: Vec<Sha256Digest>,
    pub counterexample_candidate_digest: Option<Sha256Digest>,
    pub status: DisputeStatus,
    pub decision_rationale: Option<String>,
    pub record_digest: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementNetwork {
    Mock,
    HyperliquidTestnet,
    HyperliquidMainnetReadonly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExternalSideV1 {
    pub side_index: u8,
    pub side_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketBindingV1 {
    pub schema_version: String,
    pub backend: String,
    pub network: SettlementNetwork,
    pub external_market_id: String,
    pub external_question_id: Option<String>,
    pub sides: Vec<ExternalSideV1>,
    pub true_side_index: u8,
    pub false_side_index: u8,
    pub true_settlement_fraction: String,
    pub false_settlement_fraction: String,
    pub quote_token: Option<String>,
    pub external_metadata_digest: Sha256Digest,
    pub raw_payload_digest: Sha256Digest,
    pub claim_id: ClaimId,
    pub claim_spec_digest: Sha256Digest,
    pub external_claim_commitment: Option<String>,
    pub expected_settlement_authority: Option<String>,
    pub capability_snapshot_digest: Sha256Digest,
    pub capability_version: String,
    pub created_by: String,
    pub created_at: UtcTimestamp,
    pub binding_signature: String,
    pub binding_digest: Sha256Digest,
    pub invalidation_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnsignedSettlementIntentV1 {
    pub schema_version: String,
    pub intent_id: String,
    pub backend: String,
    pub network: SettlementNetwork,
    pub binding_digest: Sha256Digest,
    pub final_resolution_digest: Sha256Digest,
    pub action_type: String,
    pub canonical_payload: String,
    pub payload_digest: Sha256Digest,
    pub idempotency_key: String,
    pub expires_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SettlementHandoffV1 {
    pub schema_version: String,
    pub backend: String,
    pub network: SettlementNetwork,
    pub binding_digest: Sha256Digest,
    pub resolution_digest: Sha256Digest,
    pub expected_settlement_mapping: BTreeMap<String, String>,
    pub unsigned_intent_digest: Sha256Digest,
    pub evidence_bundle_uri: String,
    pub evidence_bundle_digest: Sha256Digest,
    pub required_authority_role: String,
    pub operator_preflight_checklist: Vec<String>,
    pub expires_at: UtcTimestamp,
    pub replay_protection: String,
    pub hsai_handoff_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmissionReceiptV1 {
    pub schema_version: String,
    pub intent_id: String,
    pub idempotency_key: String,
    pub backend_receipt_id: String,
    pub accepted_at: UtcTimestamp,
    pub request_digest: Sha256Digest,
    pub response_digest: Sha256Digest,
    pub external_sequence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReconciliationStatus {
    Confirmed,
    Pending,
    Mismatch,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationResultV1 {
    pub schema_version: String,
    pub binding_digest: Sha256Digest,
    pub final_resolution_digest: Sha256Digest,
    pub receipt_digest: Option<Sha256Digest>,
    pub external_status_digest: Sha256Digest,
    pub status: ReconciliationStatus,
    pub settlement_lifecycle: SettlementLifecycle,
    pub checked_at: UtcTimestamp,
    pub exact_reason: String,
}
