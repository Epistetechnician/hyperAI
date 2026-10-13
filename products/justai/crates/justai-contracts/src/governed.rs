//! Pure-data contracts for the governed discovery loop.
//!
//! This module deliberately contains no model, evaluator, policy, custody-root,
//! signer, release-authority, or kill-path selection. Those values are inputs
//! supplied by an independently governed caller.

use crate::{canonical_bytes, content_digest, ContractError, ContractResult};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const GOVERNED_STATE_SLICE: &str = "justai-governed-discovery-monorepo-v1";

const STATE_SLICES: &[&str] = &[
    "justai-governed-discovery-monorepo-v1",
    "justai-contracts-v1",
    "justai-discovery-v1",
    "justai-continual-learning-v1",
    "justai-custody-v1",
    "justai-platform-v1",
    "justai-operations-v1",
    "justai-infrastructure-v1",
];

const DISPOSITIONS: &[&str] = &[
    "proposed",
    "sampled",
    "reviewed",
    "dissent",
    "retrospective",
    "promote",
    "unknown",
    "quarantine",
];

const CLAIM_CEILINGS: &[(&str, u8)] = &[
    ("source-description-only", 0),
    ("bounded-description-only", 0),
    ("locator-only-development-pilot", 0),
    ("reviewer-identity-only", 0),
    ("identity-and-role-description-only", 0),
    ("local-proposal", 1),
    ("local-development-only", 1),
    ("bounded-public-observatory", 1),
    ("local-replay", 2),
    ("engineering-evidence", 3),
    ("independent-review-pending", 4),
    ("scientific-evidence", 5),
    ("production-authorized", 6),
];

const REQUIRED_NONCLAIMS: &[&str] = &[
    "not independent review",
    "not scientific evidence",
    "not production readiness",
    "not authorization",
    "not profitability",
    "not hosted enforcement",
];

const LOCAL_MAX_CLAIM_CEILING_RANK: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceRef {
    pub source_repository: String,
    pub source_commit: String,
    pub source_path: String,
    pub source_digest: String,
    pub license_basis: String,
    pub custody_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceFields {
    pub state_slice: String,
    pub proposal_digest: String,
    pub claim_ceiling: String,
    pub provenance: Vec<ProvenanceRef>,
    pub disposition: String,
    pub evidence_refs: Vec<String>,
    pub content_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionRecord {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub algorithmic_proposal: String,
    pub model_role: String,
    pub reversible: bool,
    pub confidence: u8,
    pub alternative_refs: Vec<String>,
    pub requested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SerendipityRecord {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub sampling_method: String,
    pub candidate_refs: Vec<String>,
    pub preserved_alternatives: Vec<String>,
    pub candidate_digests: Vec<String>,
    pub budget: String,
    pub selected_candidate_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntuitionReview {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub hypothesis: String,
    pub confidence: u8,
    pub observed_evidence: Vec<String>,
    pub rejected_alternatives: Vec<String>,
    pub recommended_action_or_restraint: String,
    pub reviewer_identity: String,
    pub later_outcome: Option<String>,
    pub intuition_added_value: Option<bool>,
    pub human_disposition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DissentRecord {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub dissenting_reviewer: String,
    pub reason: String,
    pub alternatives_preserved: Vec<String>,
    pub resolution: Option<String>,
    pub resolution_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionDecision {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub decision: String,
    pub reason: String,
    pub validator_identity: String,
    pub validator_independence: String,
    pub human_disposition_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnknownRecord {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub unknown_reason: String,
    pub missing_evidence: Vec<String>,
    pub next_review_condition: String,
    pub preserved_alternatives: Vec<String>,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceManifest {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub artifact_refs: Vec<String>,
    pub source_commit_refs: Vec<String>,
    pub custody_root_ref: String,
    pub integrity_status: String,
    pub excluded_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationResult {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub phase_policy: String,
    pub math: String,
    pub linguistics: String,
    pub evidence: String,
    pub observed_ids: Vec<String>,
    pub required_digest_refs: Vec<String>,
    pub nonclaims: Vec<String>,
    pub external_seal_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationHandoffSeal {
    pub schema: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(flatten)]
    pub governance: GovernanceFields,
    pub result_digest: String,
    pub sealed_payload_digest: String,
    pub external_reviewer_id: String,
    pub external_public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernedRecord {
    Decision(DecisionRecord),
    Serendipity(SerendipityRecord),
    Intuition(IntuitionReview),
    Dissent(DissentRecord),
    Promotion(PromotionDecision),
    Unknown(UnknownRecord),
    Evidence(EvidenceManifest),
    Integration(IntegrationResult),
    HandoffSeal(IntegrationHandoffSeal),
}

impl GovernedRecord {
    pub fn governance(&self) -> &GovernanceFields {
        match self {
            Self::Decision(record) => &record.governance,
            Self::Serendipity(record) => &record.governance,
            Self::Intuition(record) => &record.governance,
            Self::Dissent(record) => &record.governance,
            Self::Promotion(record) => &record.governance,
            Self::Unknown(record) => &record.governance,
            Self::Evidence(record) => &record.governance,
            Self::Integration(record) => &record.governance,
            Self::HandoffSeal(record) => &record.governance,
        }
    }

    pub fn record_id(&self) -> &str {
        match self {
            Self::Decision(record) => &record.record_id,
            Self::Serendipity(record) => &record.record_id,
            Self::Intuition(record) => &record.record_id,
            Self::Dissent(record) => &record.record_id,
            Self::Promotion(record) => &record.record_id,
            Self::Unknown(record) => &record.record_id,
            Self::Evidence(record) => &record.record_id,
            Self::Integration(record) => &record.record_id,
            Self::HandoffSeal(record) => &record.record_id,
        }
    }
}

fn require_text(value: &str, field: &str) -> ContractResult<()> {
    if value.trim().is_empty() {
        Err(ContractError::Invalid(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_digest(value: &str, field: &str) -> ContractResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ContractError::Invalid(format!(
            "{field} must be lowercase SHA-256"
        )));
    }
    Ok(())
}

fn require_git_commit(value: &str, field: &str) -> ContractResult<()> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ContractError::Invalid(format!(
            "{field} must be a lowercase 40-character Git commit"
        )));
    }
    Ok(())
}

fn require_list(values: &[String], field: &str) -> ContractResult<()> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        Err(ContractError::Invalid(format!("{field} must contain text")))
    } else {
        Ok(())
    }
}

fn validate_provenance(values: &[ProvenanceRef]) -> ContractResult<()> {
    require_list(
        &values
            .iter()
            .map(|value| value.source_repository.clone())
            .collect::<Vec<_>>(),
        "provenance",
    )?;
    for value in values {
        for (field, text) in [
            (
                "provenance.source_repository",
                value.source_repository.as_str(),
            ),
            ("provenance.source_commit", value.source_commit.as_str()),
            ("provenance.source_path", value.source_path.as_str()),
            ("provenance.license_basis", value.license_basis.as_str()),
            ("provenance.custody_ref", value.custody_ref.as_str()),
        ] {
            require_text(text, field)?;
        }
        require_git_commit(&value.source_commit, "provenance.source_commit")?;
        require_digest(&value.source_digest, "provenance.source_digest")?;
        if value.source_path.starts_with('/') || value.source_path.contains("..") {
            return Err(ContractError::Invalid(
                "provenance.source_path must be repository-relative".into(),
            ));
        }
    }
    Ok(())
}

fn validate_governance_fields(fields: &GovernanceFields) -> ContractResult<()> {
    if !STATE_SLICES.contains(&fields.state_slice.as_str()) {
        return Err(ContractError::Invalid(format!(
            "unsupported state slice: {}",
            fields.state_slice
        )));
    }
    require_digest(&fields.proposal_digest, "proposal_digest")?;
    require_text(&fields.claim_ceiling, "claim_ceiling")?;
    if !CLAIM_CEILINGS
        .iter()
        .any(|(ceiling, _)| *ceiling == fields.claim_ceiling)
    {
        return Err(ContractError::Invalid(format!(
            "unknown claim ceiling: {}",
            fields.claim_ceiling
        )));
    }
    let claim_rank = CLAIM_CEILINGS
        .iter()
        .find(|(ceiling, _)| *ceiling == fields.claim_ceiling)
        .map(|(_, rank)| *rank)
        .expect("claim ceiling was checked above");
    if claim_rank > LOCAL_MAX_CLAIM_CEILING_RANK {
        return Err(ContractError::Invalid(
            "local governed contracts cannot emit a claim above local-replay".into(),
        ));
    }
    validate_provenance(&fields.provenance)?;
    if !DISPOSITIONS.contains(&fields.disposition.as_str()) {
        return Err(ContractError::Invalid(format!(
            "unsupported disposition: {}",
            fields.disposition
        )));
    }
    require_list(&fields.evidence_refs, "evidence_refs")?;
    require_digest(&fields.content_digest, "content_digest")
}

fn validate_header(
    schema: &str,
    expected_schema: &str,
    record_id: &str,
    created_at: &str,
) -> ContractResult<()> {
    if schema != expected_schema {
        return Err(ContractError::Invalid(format!(
            "schema must be {expected_schema}"
        )));
    }
    require_text(record_id, "record_id")?;
    require_text(created_at, "created_at")
}

fn validate_digest_bound(value: &Value, fields: &GovernanceFields) -> ContractResult<()> {
    let actual = content_digest(value)?;
    if actual != fields.content_digest {
        return Err(ContractError::Invalid(format!(
            "content_digest mismatch: expected {actual}, got {}",
            fields.content_digest
        )));
    }
    Ok(())
}

fn validate_decision(record: &DecisionRecord, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.decision.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    require_text(&record.algorithmic_proposal, "algorithmic_proposal")?;
    require_text(&record.requested_action, "requested_action")?;
    if record.model_role != "proposal-only" {
        return Err(ContractError::Invalid(
            "model_role must be proposal-only".into(),
        ));
    }
    require_list(&record.alternative_refs, "alternative_refs")?;
    if record.confidence > 100 {
        return Err(ContractError::Invalid("confidence must be 0..=100".into()));
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_serendipity(record: &SerendipityRecord, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.serendipity.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    require_text(&record.sampling_method, "sampling_method")?;
    require_list(&record.candidate_refs, "candidate_refs")?;
    require_list(&record.preserved_alternatives, "preserved_alternatives")?;
    require_list(&record.candidate_digests, "candidate_digests")?;
    if record.candidate_digests.len() != record.candidate_refs.len()
        || record
            .candidate_digests
            .iter()
            .any(|digest| require_digest(digest, "candidate_digests").is_err())
    {
        return Err(ContractError::Invalid(
            "candidate_digests must bind every candidate reference".into(),
        ));
    }
    require_text(&record.budget, "budget")?;
    if let Some(selected) = &record.selected_candidate_ref {
        if !record.candidate_refs.contains(selected) {
            return Err(ContractError::Invalid(
                "selected_candidate_ref must be sampled".into(),
            ));
        }
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_intuition(record: &IntuitionReview, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.intuition_review.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    require_text(&record.hypothesis, "hypothesis")?;
    require_list(&record.observed_evidence, "observed_evidence")?;
    require_list(&record.rejected_alternatives, "rejected_alternatives")?;
    require_text(
        &record.recommended_action_or_restraint,
        "recommended_action_or_restraint",
    )?;
    require_text(&record.reviewer_identity, "reviewer_identity")?;
    if record.confidence > 100 {
        return Err(ContractError::Invalid("confidence must be 0..=100".into()));
    }
    if let Some(disposition) = &record.human_disposition {
        if !["promote", "unknown", "quarantine"].contains(&disposition.as_str()) {
            return Err(ContractError::Invalid(
                "unsupported human disposition".into(),
            ));
        }
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_dissent(record: &DissentRecord, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.dissent.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    require_text(&record.dissenting_reviewer, "dissenting_reviewer")?;
    require_text(&record.reason, "reason")?;
    require_list(&record.alternatives_preserved, "alternatives_preserved")?;
    if let Some(resolution) = &record.resolution {
        require_text(resolution, "resolution")?;
    }
    if record
        .resolution_evidence_refs
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(ContractError::Invalid(
            "resolution_evidence_refs must not contain empty ids".into(),
        ));
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_promotion(record: &PromotionDecision, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.promotion_decision.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    if !["promote", "unknown", "quarantine"].contains(&record.decision.as_str()) {
        return Err(ContractError::Invalid(format!(
            "unsupported promotion decision: {}",
            record.decision
        )));
    }
    require_text(&record.reason, "reason")?;
    require_text(&record.validator_identity, "validator_identity")?;
    require_text(&record.validator_independence, "validator_independence")?;
    if record.decision != record.governance.disposition {
        return Err(ContractError::Invalid(
            "promotion decision and disposition must agree".into(),
        ));
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_unknown(record: &UnknownRecord, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.unknown.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    if record.governance.disposition != "unknown" {
        return Err(ContractError::Invalid(
            "UnknownRecord disposition must be unknown".into(),
        ));
    }
    require_text(&record.unknown_reason, "unknown_reason")?;
    require_list(&record.missing_evidence, "missing_evidence")?;
    require_text(&record.next_review_condition, "next_review_condition")?;
    require_list(&record.preserved_alternatives, "preserved_alternatives")?;
    require_text(&record.owner, "owner")?;
    validate_digest_bound(value, &record.governance)
}

fn validate_evidence(record: &EvidenceManifest, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.evidence_manifest.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    require_list(&record.artifact_refs, "artifact_refs")?;
    require_list(&record.source_commit_refs, "source_commit_refs")?;
    require_text(&record.custody_root_ref, "custody_root_ref")?;
    require_text(&record.integrity_status, "integrity_status")?;
    if record
        .excluded_artifacts
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(ContractError::Invalid(
            "excluded_artifacts must not contain empty values".into(),
        ));
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_integration(record: &IntegrationResult, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.integration_result.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    for (field, lane) in [
        ("phase_policy", &record.phase_policy),
        ("math", &record.math),
        ("linguistics", &record.linguistics),
        ("evidence", &record.evidence),
    ] {
        require_text(lane, field)?;
    }
    require_list(&record.observed_ids, "observed_ids")?;
    require_list(&record.required_digest_refs, "required_digest_refs")?;
    require_list(&record.nonclaims, "nonclaims")?;
    if record
        .required_digest_refs
        .iter()
        .any(|digest| require_digest(digest, "required_digest_refs").is_err())
    {
        return Err(ContractError::Invalid(
            "required_digest_refs must contain lowercase SHA-256 digests".into(),
        ));
    }
    let observed_ids = record.observed_ids.iter().collect::<HashSet<_>>();
    if observed_ids.len() != record.observed_ids.len()
        || record
            .nonclaims
            .iter()
            .any(|claim| !REQUIRED_NONCLAIMS.contains(&claim.as_str()))
        || REQUIRED_NONCLAIMS
            .iter()
            .any(|claim| !record.nonclaims.iter().any(|value| value == claim))
    {
        return Err(ContractError::Invalid(
            "integration result must preserve unique observed ids and mandatory nonclaims".into(),
        ));
    }
    if let Some(reference) = &record.external_seal_ref {
        require_text(reference, "external_seal_ref")?;
    }
    validate_digest_bound(value, &record.governance)
}

fn validate_handoff_seal(record: &IntegrationHandoffSeal, value: &Value) -> ContractResult<()> {
    validate_header(
        &record.schema,
        "justai.integration_handoff_seal.v1",
        &record.record_id,
        &record.created_at,
    )?;
    validate_governance_fields(&record.governance)?;
    require_digest(&record.result_digest, "result_digest")?;
    require_digest(&record.sealed_payload_digest, "sealed_payload_digest")?;
    require_text(&record.external_reviewer_id, "external_reviewer_id")?;
    require_text(&record.external_public_key, "external_public_key")?;
    require_text(&record.signature, "signature")?;
    if record.external_public_key.len() != 64
        || record.signature.len() != 128
        || !record
            .external_public_key
            .bytes()
            .chain(record.signature.bytes())
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ContractError::Invalid(
            "external seal key and signature must be lowercase hexadecimal".into(),
        ));
    }
    validate_digest_bound(value, &record.governance)
}

fn reject_unknown_fields(value: &Value, allowed: &[&str]) -> ContractResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| ContractError::Invalid("governed record must be an object".into()))?;
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(ContractError::Invalid(format!(
            "unknown governed record field: {field}"
        )));
    }
    if let Some(field) = allowed.iter().find(|field| !object.contains_key(**field)) {
        return Err(ContractError::Invalid(format!(
            "missing governed record field: {field}"
        )));
    }
    Ok(())
}

pub fn validate_governed_record(value: &Value) -> ContractResult<GovernedRecord> {
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| ContractError::Invalid("governed record schema is required".into()))?;
    const COMMON: &[&str] = &[
        "schema",
        "record_id",
        "created_at",
        "state_slice",
        "proposal_digest",
        "claim_ceiling",
        "provenance",
        "disposition",
        "evidence_refs",
        "content_digest",
    ];
    let specific: &[&str] = match schema {
        "justai.decision.v1" => &[
            "algorithmic_proposal",
            "model_role",
            "reversible",
            "confidence",
            "alternative_refs",
            "requested_action",
        ],
        "justai.serendipity.v1" => &[
            "sampling_method",
            "candidate_refs",
            "preserved_alternatives",
            "candidate_digests",
            "budget",
            "selected_candidate_ref",
        ],
        "justai.intuition_review.v1" => &[
            "hypothesis",
            "confidence",
            "observed_evidence",
            "rejected_alternatives",
            "recommended_action_or_restraint",
            "reviewer_identity",
            "later_outcome",
            "intuition_added_value",
            "human_disposition",
        ],
        "justai.dissent.v1" => &[
            "dissenting_reviewer",
            "reason",
            "alternatives_preserved",
            "resolution",
            "resolution_evidence_refs",
        ],
        "justai.promotion_decision.v1" => &[
            "decision",
            "reason",
            "validator_identity",
            "validator_independence",
            "human_disposition_ref",
        ],
        "justai.unknown.v1" => &[
            "unknown_reason",
            "missing_evidence",
            "next_review_condition",
            "preserved_alternatives",
            "owner",
        ],
        "justai.evidence_manifest.v1" => &[
            "artifact_refs",
            "source_commit_refs",
            "custody_root_ref",
            "integrity_status",
            "excluded_artifacts",
        ],
        "justai.integration_result.v1" => &[
            "phase_policy",
            "math",
            "linguistics",
            "evidence",
            "observed_ids",
            "required_digest_refs",
            "nonclaims",
            "external_seal_ref",
        ],
        "justai.integration_handoff_seal.v1" => &[
            "result_digest",
            "sealed_payload_digest",
            "external_reviewer_id",
            "external_public_key",
            "signature",
        ],
        _ => &[],
    };
    let allowed = COMMON
        .iter()
        .chain(specific.iter())
        .copied()
        .collect::<Vec<_>>();
    reject_unknown_fields(value, &allowed)?;
    match schema {
        "justai.decision.v1" => {
            let record: DecisionRecord =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("decision record shape: {error}"))
                })?;
            validate_decision(&record, value)?;
            Ok(GovernedRecord::Decision(record))
        }
        "justai.serendipity.v1" => {
            let record: SerendipityRecord =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("serendipity record shape: {error}"))
                })?;
            validate_serendipity(&record, value)?;
            Ok(GovernedRecord::Serendipity(record))
        }
        "justai.intuition_review.v1" => {
            let record: IntuitionReview =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("intuition review shape: {error}"))
                })?;
            validate_intuition(&record, value)?;
            Ok(GovernedRecord::Intuition(record))
        }
        "justai.dissent.v1" => {
            let record: DissentRecord = serde_json::from_value(value.clone()).map_err(|error| {
                ContractError::Invalid(format!("dissent record shape: {error}"))
            })?;
            validate_dissent(&record, value)?;
            Ok(GovernedRecord::Dissent(record))
        }
        "justai.promotion_decision.v1" => {
            let record: PromotionDecision =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("promotion decision shape: {error}"))
                })?;
            validate_promotion(&record, value)?;
            Ok(GovernedRecord::Promotion(record))
        }
        "justai.unknown.v1" => {
            let record: UnknownRecord = serde_json::from_value(value.clone()).map_err(|error| {
                ContractError::Invalid(format!("unknown record shape: {error}"))
            })?;
            validate_unknown(&record, value)?;
            Ok(GovernedRecord::Unknown(record))
        }
        "justai.evidence_manifest.v1" => {
            let record: EvidenceManifest =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("evidence manifest shape: {error}"))
                })?;
            validate_evidence(&record, value)?;
            Ok(GovernedRecord::Evidence(record))
        }
        "justai.integration_result.v1" => {
            let record: IntegrationResult =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("integration result shape: {error}"))
                })?;
            validate_integration(&record, value)?;
            Ok(GovernedRecord::Integration(record))
        }
        "justai.integration_handoff_seal.v1" => {
            let record: IntegrationHandoffSeal =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ContractError::Invalid(format!("integration handoff seal shape: {error}"))
                })?;
            validate_handoff_seal(&record, value)?;
            Ok(GovernedRecord::HandoffSeal(record))
        }
        other => Err(ContractError::Invalid(format!(
            "unsupported governed record schema: {other}"
        ))),
    }
}

pub fn seal_governed_record(value: &mut Value) -> ContractResult<()> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| ContractError::Invalid("governed record must be an object".into()))?;
    object.insert("content_digest".into(), Value::String(String::new()));
    let digest = content_digest(value)?;
    value
        .as_object_mut()
        .ok_or_else(|| ContractError::Invalid("governed record must be an object".into()))?
        .insert("content_digest".into(), Value::String(digest));
    Ok(())
}

pub fn enforce_claim_ceiling(actual: &str, maximum: &str) -> ContractResult<()> {
    let actual_rank = CLAIM_CEILINGS
        .iter()
        .find(|(name, _)| *name == actual)
        .map(|(_, rank)| *rank)
        .ok_or_else(|| ContractError::Invalid(format!("unknown claim ceiling: {actual}")))?;
    let maximum_rank = CLAIM_CEILINGS
        .iter()
        .find(|(name, _)| *name == maximum)
        .map(|(_, rank)| *rank)
        .ok_or_else(|| {
            ContractError::Invalid(format!("unknown maximum claim ceiling: {maximum}"))
        })?;
    if actual_rank > maximum_rank {
        return Err(ContractError::Invalid(format!(
            "claim ceiling {actual} exceeds allowed maximum {maximum}"
        )));
    }
    Ok(())
}

pub fn integration_handoff_payload(
    result: &IntegrationResult,
    seal: &IntegrationHandoffSeal,
) -> ContractResult<Vec<u8>> {
    serde_jcs::to_vec(&serde_json::json!({
        "domain": "justai:integration-handoff:v1",
        "result_digest": result.governance.content_digest,
        "seal_id": seal.record_id,
        "result_state_slice": result.governance.state_slice,
        "result_proposal_digest": result.governance.proposal_digest,
        "result_claim_ceiling": result.governance.claim_ceiling,
        "result_disposition": result.governance.disposition,
        "result_provenance": result.governance.provenance,
        "result_evidence_refs": result.governance.evidence_refs,
        "external_seal_ref": result.external_seal_ref,
        "seal_state_slice": seal.governance.state_slice,
        "seal_proposal_digest": seal.governance.proposal_digest,
        "seal_claim_ceiling": seal.governance.claim_ceiling,
        "seal_disposition": seal.governance.disposition,
        "seal_provenance": seal.governance.provenance,
        "seal_evidence_refs": seal.governance.evidence_refs,
        "external_reviewer_id": seal.external_reviewer_id,
    }))
    .map_err(|error| ContractError::Invalid(format!("JCS handoff payload: {error}")))
}

pub fn verify_integration_handoff_seal(
    result: &IntegrationResult,
    seal: &IntegrationHandoffSeal,
    authorized_reviewer_id: &str,
    authorized_public_key: &str,
) -> ContractResult<()> {
    let result_value = serde_json::to_value(result)?;
    validate_integration(result, &result_value)?;
    let seal_value = serde_json::to_value(seal)?;
    validate_handoff_seal(seal, &seal_value)?;
    if seal.result_digest != result.governance.content_digest {
        return Err(ContractError::Invalid(
            "external seal result_digest does not match integration result".into(),
        ));
    }
    if result.external_seal_ref.as_deref() != Some(seal.record_id.as_str()) {
        return Err(ContractError::Invalid(
            "integration result external_seal_ref does not match seal".into(),
        ));
    }
    if seal.external_reviewer_id != authorized_reviewer_id
        || seal.external_public_key != authorized_public_key
    {
        return Err(ContractError::Invalid(
            "external seal reviewer is not bound to the authorized key".into(),
        ));
    }
    if seal.governance.proposal_digest != result.governance.proposal_digest
        || seal.governance.claim_ceiling != result.governance.claim_ceiling
    {
        return Err(ContractError::Invalid(
            "external seal governance context does not match integration result".into(),
        ));
    }
    let payload = integration_handoff_payload(result, seal)?;
    let payload_digest = hex::encode(Sha256::digest(&payload));
    if seal.sealed_payload_digest != payload_digest {
        return Err(ContractError::Invalid(
            "external seal payload digest mismatch".into(),
        ));
    }
    let public_key_bytes: [u8; 32] = hex::decode(&seal.external_public_key)
        .map_err(|_| ContractError::Invalid("invalid external public key".into()))?
        .try_into()
        .map_err(|_| ContractError::Invalid("invalid external public key length".into()))?;
    let signature_bytes = hex::decode(&seal.signature)
        .map_err(|_| ContractError::Invalid("invalid external signature".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| ContractError::Invalid("invalid external public key".into()))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| ContractError::Invalid("invalid external signature".into()))?;
    verifying_key
        .verify(&payload, &signature)
        .map_err(|_| ContractError::Invalid("external seal verification failed".into()))
}

pub fn canonical_governed_bytes(value: &Value) -> ContractResult<Vec<u8>> {
    let _ = validate_governed_record(value)?;
    canonical_bytes(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn commit(byte: char) -> String {
        std::iter::repeat_n(byte, 40).collect()
    }

    fn governance(disposition: &str) -> serde_json::Value {
        serde_json::json!({
            "state_slice": GOVERNED_STATE_SLICE,
            "proposal_digest": digest('a'),
            "claim_ceiling": "local-replay",
            "provenance": [{
                "source_repository": "source-repo",
                "source_commit": commit('b'),
                "source_path": "src/contracts.py",
                "source_digest": digest('c'),
                "license_basis": "reference-only; clean-room reimplementation",
                "custody_ref": "custody:source-repo:commit"
            }],
            "disposition": disposition,
            "evidence_refs": ["evidence-1"],
            "content_digest": ""
        })
    }

    fn integration_value() -> Value {
        let mut value = serde_json::json!({
            "schema": "justai.integration_result.v1",
            "record_id": "integration-1",
            "created_at": "2026-09-09T12:00:00Z",
            "governance": governance("promote"),
            "phase_policy": "design-only",
            "math": "proposal-lane-preserved",
            "linguistics": "prediction-lock-required",
            "evidence": "local-engineering-evidence-only",
            "observed_ids": ["decision-1"],
            "required_digest_refs": [digest('d')],
            "nonclaims": REQUIRED_NONCLAIMS,
            "external_seal_ref": "seal-1"
        });
        let metadata = value
            .get_mut("governance")
            .and_then(Value::as_object_mut)
            .expect("governance object");
        let mut fields = metadata.clone();
        fields.insert("content_digest".into(), Value::String(String::new()));
        value.as_object_mut().unwrap().remove("governance");
        for (key, field) in fields {
            value.as_object_mut().unwrap().insert(key, field);
        }
        seal_governed_record(&mut value).unwrap();
        value
    }

    #[test]
    fn validates_all_required_governance_fields() {
        let value = integration_value();
        let parsed = validate_governed_record(&value).unwrap();
        assert_eq!(parsed.record_id(), "integration-1");
        assert_eq!(parsed.governance().state_slice, GOVERNED_STATE_SLICE);
    }

    #[test]
    fn rejects_duplicate_keys_recursively_before_contract_mapping() {
        let result = crate::parse_json_no_duplicates(br#"{"outer":{"key":1,"key":2}}"#);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON object key"));
    }

    #[test]
    fn rejects_model_authority_and_bad_claim_ceiling() {
        let mut value = serde_json::json!({
            "schema": "justai.decision.v1",
            "record_id": "decision-1",
            "created_at": "2026-09-09T12:00:00Z",
            "governance": governance("proposed"),
            "algorithmic_proposal": "bounded proposal",
            "model_role": "policy-selector",
            "reversible": true,
            "confidence": 90,
            "alternative_refs": ["alternative-1"],
            "requested_action": "request independent validation"
        });
        let fields = value
            .get_mut("governance")
            .and_then(Value::as_object_mut)
            .unwrap();
        let mut flattened = fields.clone();
        value.as_object_mut().unwrap().remove("governance");
        for (key, field) in flattened.iter_mut() {
            value
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), field.clone());
        }
        seal_governed_record(&mut value).unwrap();
        assert!(validate_governed_record(&value).is_err());
        let mut high_ceiling = value.clone();
        high_ceiling["model_role"] = Value::String("proposal-only".into());
        high_ceiling["claim_ceiling"] = Value::String("scientific-evidence".into());
        seal_governed_record(&mut high_ceiling).unwrap();
        assert!(validate_governed_record(&high_ceiling).is_err());
        assert!(enforce_claim_ceiling("scientific-evidence", "local-replay").is_err());
        assert!(enforce_claim_ceiling("local-replay", "engineering-evidence").is_ok());
    }

    #[test]
    fn verifies_external_handoff_seal_without_manufacturing_one() {
        let result_value = integration_value();
        let result: IntegrationResult = serde_json::from_value(result_value).unwrap();
        let signing_key = SigningKey::generate(&mut OsRng);
        let seal_id = "seal-1";
        let result_digest = result.governance.content_digest.clone();
        let mut seal_value = serde_json::json!({
            "schema": "justai.integration_handoff_seal.v1",
            "record_id": seal_id,
            "created_at": "2026-09-09T12:00:00Z",
            "state_slice": "justai-custody-v1",
            "proposal_digest": digest('a'),
            "claim_ceiling": "local-replay",
            "provenance": [{
                "source_repository": "source-repo",
                "source_commit": commit('b'),
                "source_path": "src/contracts.py",
                "source_digest": digest('c'),
                "license_basis": "reference-only",
                "custody_ref": "custody:source-repo:commit"
            }],
            "disposition": "promote",
            "evidence_refs": ["evidence-1"],
            "content_digest": "",
            "result_digest": result_digest,
            "sealed_payload_digest": "0".repeat(64),
            "external_reviewer_id": "reviewer-external",
            "external_public_key": hex::encode(signing_key.verifying_key().to_bytes()),
            "signature": "0".repeat(128)
        });
        seal_governed_record(&mut seal_value).unwrap();
        let mut seal: IntegrationHandoffSeal = serde_json::from_value(seal_value).unwrap();
        let payload = integration_handoff_payload(&result, &seal).unwrap();
        let payload_digest = hex::encode(Sha256::digest(&payload));
        let signature = signing_key.sign(&payload);
        seal.sealed_payload_digest = payload_digest;
        seal.signature = hex::encode(signature.to_bytes());
        let mut seal_value = serde_json::to_value(&seal).unwrap();
        seal_governed_record(&mut seal_value).unwrap();
        seal = serde_json::from_value(seal_value).unwrap();
        verify_integration_handoff_seal(
            &result,
            &seal,
            "reviewer-external",
            &hex::encode(signing_key.verifying_key().to_bytes()),
        )
        .unwrap();
    }

    #[test]
    fn rejects_untrusted_external_seal_key_and_mismatched_reference() {
        let result_value = integration_value();
        let result: IntegrationResult = serde_json::from_value(result_value).unwrap();
        let signing_key = SigningKey::generate(&mut OsRng);
        let seal_id = "seal-1";
        let mut seal_value = serde_json::json!({
            "schema": "justai.integration_handoff_seal.v1",
            "record_id": seal_id,
            "created_at": "2026-09-09T12:00:00Z",
            "state_slice": "justai-custody-v1",
            "proposal_digest": digest('a'),
            "claim_ceiling": "local-replay",
            "provenance": [{
                "source_repository": "source-repo",
                "source_commit": commit('b'),
                "source_path": "src/contracts.py",
                "source_digest": digest('c'),
                "license_basis": "reference-only",
                "custody_ref": "custody:source-repo:commit"
            }],
            "disposition": "promote",
            "evidence_refs": ["evidence-1"],
            "content_digest": "",
            "result_digest": result.governance.content_digest,
            "sealed_payload_digest": "0".repeat(64),
            "external_reviewer_id": "rogue",
            "external_public_key": hex::encode(signing_key.verifying_key().to_bytes()),
            "signature": "0".repeat(128)
        });
        seal_governed_record(&mut seal_value).unwrap();
        let seal: IntegrationHandoffSeal = serde_json::from_value(seal_value).unwrap();
        assert!(verify_integration_handoff_seal(
            &result,
            &seal,
            "reviewer-external",
            &hex::encode(signing_key.verifying_key().to_bytes()),
        )
        .is_err());
    }
}
