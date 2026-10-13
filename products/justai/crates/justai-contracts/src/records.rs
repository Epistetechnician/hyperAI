use crate::{validate_envelope, ContractError, ContractResult, Envelope};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRecord {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub title: String,
    pub url: String,
    pub accessed_at: String,
    pub publication_date: String,
    pub jurisdiction: String,
    pub source_class: String,
    pub license_or_terms: String,
    pub access_class: String,
    pub custody_mode: String,
    pub license_basis: String,
    pub custodian_id: String,
    pub acquisition_timestamp: String,
    pub permitted_uses: Vec<String>,
    pub retention_and_disclosure_rules: String,
    pub snapshot_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactManifest {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub artifact_kind: String,
    pub access_class: String,
    pub custody_mode: String,
    pub license_basis: String,
    pub custodian_id: String,
    pub acquisition_timestamp: String,
    pub artifact_digest: String,
    pub permitted_uses: Vec<String>,
    pub retention_and_disclosure_rules: String,
    pub evidence_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceLimits {
    pub cpu_seconds: u64,
    pub memory_bytes: u64,
    pub network_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationReceipt {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub artifact_manifest_refs: Vec<String>,
    pub authorization_ref: String,
    pub evaluation_scope: String,
    pub environment_digest: String,
    pub runner_digest: String,
    pub resource_limits: ResourceLimits,
    pub observed_result_digest: String,
    pub observed_result_summary: String,
    pub evidence_basis: String,
    pub uncertainty: String,
    pub nonclaims: Vec<String>,
    pub review_receipt_refs: Vec<String>,
    pub disclosure_decision_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisclosureDecision {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub subject_refs: Vec<String>,
    pub decision: String,
    pub reason: String,
    pub public_summary: String,
    pub withheld_material: String,
    pub missing_information: String,
    pub contestation: String,
    pub review_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimProjection {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub evidence_refs: Vec<String>,
    pub public_summary: String,
    pub uncertainty: String,
    pub nonclaims: Vec<String>,
    pub missing_information: String,
    pub contestation: String,
    pub disclosure_decision_refs: Vec<String>,
    pub review_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyTraceRecord {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub artifact_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub evaluation_receipt_refs: Vec<String>,
    pub review_receipt_refs: Vec<String>,
    pub claim_projection_refs: Vec<String>,
    pub demand: String,
    pub lawful_target: String,
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimRecord {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub claim: String,
    pub claim_type: String,
    pub scope: String,
    pub uncertainty: String,
    pub affected_party_response: String,
    pub support_refs: Vec<String>,
    pub contradiction_refs: Vec<String>,
    pub review_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncidentRecord {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub system_id: String,
    pub operator_id: String,
    pub occurred_at: String,
    pub environment: String,
    pub observed_action: String,
    pub harm_status: String,
    pub affected_parties: Vec<String>,
    pub reproducibility: String,
    pub response: String,
    pub disclosure_state: String,
    pub review_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitmentRecord {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub issuer: String,
    pub commitment: String,
    pub metric: String,
    pub deadline: String,
    pub verification_status: String,
    pub evidence_refs: Vec<String>,
    pub review_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewReceipt {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub reviewer_id: String,
    pub reviewer_org: String,
    pub reviewed_record_id: String,
    pub reviewed_digest: String,
    pub syndicate_manifest_ref: String,
    pub result: String,
    pub conflict_of_interest: bool,
    pub independence_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisputeRecord {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub challenger_id: String,
    pub disputed_record_id: String,
    pub basis: String,
    pub affected_party_response: String,
    pub resolution: String,
    pub contestation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyndicateManifest {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub syndicate_id: String,
    pub syndicate_name: String,
    pub jurisdiction: String,
    pub roles: Vec<String>,
    pub public_key: String,
    pub charter_version: String,
    pub funding_disclosure: String,
    pub conflict_disclosure: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedRecord {
    Source(SourceRecord),
    ArtifactManifest(ArtifactManifest),
    Evaluation(EvaluationReceipt),
    Disclosure(DisclosureDecision),
    Projection(ClaimProjection),
    PolicyTrace(PolicyTraceRecord),
    Claim(ClaimRecord),
    Incident(IncidentRecord),
    Commitment(CommitmentRecord),
    Review(ReviewReceipt),
    Dispute(DisputeRecord),
    Syndicate(SyndicateManifest),
}

impl TypedRecord {
    pub fn envelope(&self) -> &Envelope {
        match self {
            Self::Source(record) => &record.envelope,
            Self::ArtifactManifest(record) => &record.envelope,
            Self::Evaluation(record) => &record.envelope,
            Self::Disclosure(record) => &record.envelope,
            Self::Projection(record) => &record.envelope,
            Self::PolicyTrace(record) => &record.envelope,
            Self::Claim(record) => &record.envelope,
            Self::Incident(record) => &record.envelope,
            Self::Commitment(record) => &record.envelope,
            Self::Review(record) => &record.envelope,
            Self::Dispute(record) => &record.envelope,
            Self::Syndicate(record) => &record.envelope,
        }
    }
}

fn parse<T: DeserializeOwned>(value: &Value, kind: &str) -> ContractResult<T> {
    serde_json::from_value(value.clone())
        .map_err(|error| ContractError::Invalid(format!("{kind} record shape: {error}")))
}

fn require_text(value: &str, field: &str) -> ContractResult<()> {
    if value.trim().is_empty() {
        Err(ContractError::Invalid(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_list(values: &[String], field: &str) -> ContractResult<()> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        Err(ContractError::Invalid(format!("{field} must contain text")))
    } else {
        Ok(())
    }
}

fn require_digest(value: &str, field: &str) -> ContractResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(ContractError::Invalid(format!(
            "{field} must be lowercase SHA-256"
        )));
    }
    Ok(())
}

fn require_one_of(value: &str, field: &str, allowed: &[&str]) -> ContractResult<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ContractError::Invalid(format!(
            "unsupported {field}: {value}"
        )))
    }
}

fn validate_custody_metadata(
    access_class: &str,
    custody_mode: &str,
    license_basis: &str,
    custodian_id: &str,
    acquisition_timestamp: &str,
    permitted_uses: &[String],
    retention_and_disclosure_rules: &str,
) -> ContractResult<()> {
    require_one_of(
        access_class,
        "access_class",
        &["public", "restricted", "protected"],
    )?;
    require_one_of(
        custody_mode,
        "custody_mode",
        &["reproducible", "attested", "corroborated", "unknown"],
    )?;
    for (field, value) in [
        ("license_basis", license_basis),
        ("custodian_id", custodian_id),
        ("acquisition_timestamp", acquisition_timestamp),
        (
            "retention_and_disclosure_rules",
            retention_and_disclosure_rules,
        ),
    ] {
        require_text(value, field)?;
    }
    require_list(permitted_uses, "permitted_uses")
}

fn validate_bridge_state(envelope: &Envelope) -> ContractResult<()> {
    if envelope.state_slice != "justai-evidence-bridge-v1" {
        return Err(ContractError::Invalid(format!(
            "bridge record {} must use state slice justai-evidence-bridge-v1",
            envelope.record_id
        )));
    }
    Ok(())
}

fn validate_source(record: &SourceRecord) -> ContractResult<()> {
    for (field, value) in [
        ("title", &record.title),
        ("url", &record.url),
        ("accessed_at", &record.accessed_at),
        ("publication_date", &record.publication_date),
        ("jurisdiction", &record.jurisdiction),
        ("source_class", &record.source_class),
        ("license_or_terms", &record.license_or_terms),
        ("snapshot_status", &record.snapshot_status),
    ] {
        require_text(value, field)?;
    }
    if ![
        "primary",
        "independent",
        "institutional",
        "community",
        "secondary",
    ]
    .contains(&record.source_class.as_str())
    {
        return Err(ContractError::Invalid(format!(
            "unsupported source_class: {}",
            record.source_class
        )));
    }
    validate_custody_metadata(
        &record.access_class,
        &record.custody_mode,
        &record.license_basis,
        &record.custodian_id,
        &record.acquisition_timestamp,
        &record.permitted_uses,
        &record.retention_and_disclosure_rules,
    )?;
    Ok(())
}

fn validate_artifact_manifest(record: &ArtifactManifest) -> ContractResult<()> {
    validate_bridge_state(&record.envelope)?;
    require_one_of(
        &record.artifact_kind,
        "artifact_kind",
        &["code", "model", "dataset", "document", "trace", "testimony"],
    )?;
    validate_custody_metadata(
        &record.access_class,
        &record.custody_mode,
        &record.license_basis,
        &record.custodian_id,
        &record.acquisition_timestamp,
        &record.permitted_uses,
        &record.retention_and_disclosure_rules,
    )?;
    if record.custody_mode == "unknown"
        && record.envelope.status != "Unknown"
        && record.envelope.status != "NoCandidate"
    {
        return Err(ContractError::Invalid(
            "unknown custody requires Unknown or NoCandidate status".into(),
        ));
    }
    require_digest(&record.artifact_digest, "artifact_digest")?;
    require_text(&record.evidence_basis, "evidence_basis")?;
    Ok(())
}

fn validate_resource_limits(limits: &ResourceLimits) -> ContractResult<()> {
    require_text(&limits.network_policy, "resource_limits.network_policy")
}

fn validate_evaluation(record: &EvaluationReceipt) -> ContractResult<()> {
    validate_bridge_state(&record.envelope)?;
    require_list(&record.artifact_manifest_refs, "artifact_manifest_refs")?;
    for (field, value) in [
        ("authorization_ref", &record.authorization_ref),
        ("evaluation_scope", &record.evaluation_scope),
        ("observed_result_summary", &record.observed_result_summary),
        ("evidence_basis", &record.evidence_basis),
        ("uncertainty", &record.uncertainty),
    ] {
        require_text(value, field)?;
    }
    require_digest(&record.environment_digest, "environment_digest")?;
    require_digest(&record.runner_digest, "runner_digest")?;
    require_digest(&record.observed_result_digest, "observed_result_digest")?;
    validate_resource_limits(&record.resource_limits)?;
    require_list(&record.nonclaims, "nonclaims")?;
    for field in [
        &record.review_receipt_refs,
        &record.disclosure_decision_refs,
    ] {
        if field.iter().any(|value| value.trim().is_empty()) {
            return Err(ContractError::Invalid(
                "reference lists must not contain empty ids".into(),
            ));
        }
    }
    Ok(())
}

fn validate_disclosure(record: &DisclosureDecision) -> ContractResult<()> {
    validate_bridge_state(&record.envelope)?;
    require_list(&record.subject_refs, "subject_refs")?;
    require_one_of(
        &record.decision,
        "disclosure decision",
        &["public", "aggregate_only", "withhold", "unknown"],
    )?;
    for (field, value) in [
        ("reason", &record.reason),
        ("public_summary", &record.public_summary),
        ("withheld_material", &record.withheld_material),
        ("missing_information", &record.missing_information),
        ("contestation", &record.contestation),
    ] {
        require_text(value, field)?;
    }
    if record
        .review_receipt_refs
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(ContractError::Invalid(
            "review_receipt_refs must not contain empty ids".into(),
        ));
    }
    Ok(())
}

fn validate_projection(record: &ClaimProjection) -> ContractResult<()> {
    validate_bridge_state(&record.envelope)?;
    require_list(&record.evidence_refs, "evidence_refs")?;
    for (field, value) in [
        ("public_summary", &record.public_summary),
        ("uncertainty", &record.uncertainty),
        ("missing_information", &record.missing_information),
        ("contestation", &record.contestation),
    ] {
        require_text(value, field)?;
    }
    require_list(&record.nonclaims, "nonclaims")?;
    for field in [
        &record.disclosure_decision_refs,
        &record.review_receipt_refs,
    ] {
        if field.is_empty() || field.iter().any(|value| value.trim().is_empty()) {
            return Err(ContractError::Invalid(
                "claim projection review and disclosure references are required".into(),
            ));
        }
    }
    Ok(())
}

fn validate_policy_trace(record: &PolicyTraceRecord) -> ContractResult<()> {
    validate_bridge_state(&record.envelope)?;
    for (field, values) in [
        ("artifact_refs", &record.artifact_refs),
        ("evidence_refs", &record.evidence_refs),
        ("evaluation_receipt_refs", &record.evaluation_receipt_refs),
        ("review_receipt_refs", &record.review_receipt_refs),
        ("claim_projection_refs", &record.claim_projection_refs),
    ] {
        require_list(values, field)?;
    }
    for (field, value) in [
        ("demand", &record.demand),
        ("lawful_target", &record.lawful_target),
        ("response", &record.response),
    ] {
        require_text(value, field)?;
    }
    Ok(())
}

fn validate_claim(record: &ClaimRecord) -> ContractResult<()> {
    for (field, value) in [
        ("claim", &record.claim),
        ("claim_type", &record.claim_type),
        ("scope", &record.scope),
        ("uncertainty", &record.uncertainty),
        ("affected_party_response", &record.affected_party_response),
    ] {
        require_text(value, field)?;
    }
    require_list(&record.support_refs, "support_refs")?;
    if record
        .review_receipt_refs
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(ContractError::Invalid(
            "review_receipt_refs must not contain empty ids".into(),
        ));
    }
    Ok(())
}

fn validate_incident(record: &IncidentRecord) -> ContractResult<()> {
    for (field, value) in [
        ("system_id", &record.system_id),
        ("operator_id", &record.operator_id),
        ("occurred_at", &record.occurred_at),
        ("environment", &record.environment),
        ("observed_action", &record.observed_action),
        ("harm_status", &record.harm_status),
        ("reproducibility", &record.reproducibility),
        ("response", &record.response),
        ("disclosure_state", &record.disclosure_state),
    ] {
        require_text(value, field)?;
    }
    require_list(&record.affected_parties, "affected_parties")?;
    if record
        .review_receipt_refs
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(ContractError::Invalid(
            "review_receipt_refs must not contain empty ids".into(),
        ));
    }
    Ok(())
}

fn validate_commitment(record: &CommitmentRecord) -> ContractResult<()> {
    for (field, value) in [
        ("issuer", &record.issuer),
        ("commitment", &record.commitment),
        ("metric", &record.metric),
        ("deadline", &record.deadline),
        ("verification_status", &record.verification_status),
    ] {
        require_text(value, field)?;
    }
    require_list(&record.evidence_refs, "evidence_refs")?;
    if record
        .review_receipt_refs
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(ContractError::Invalid(
            "review_receipt_refs must not contain empty ids".into(),
        ));
    }
    Ok(())
}

fn validate_review(record: &ReviewReceipt) -> ContractResult<()> {
    for (field, value) in [
        ("reviewer_id", &record.reviewer_id),
        ("reviewer_org", &record.reviewer_org),
        ("reviewed_record_id", &record.reviewed_record_id),
        ("reviewed_digest", &record.reviewed_digest),
        ("syndicate_manifest_ref", &record.syndicate_manifest_ref),
        ("result", &record.result),
        ("independence_basis", &record.independence_basis),
    ] {
        require_text(value, field)?;
    }
    if record.result != "accept" && record.result != "needs_revision" && record.result != "reject" {
        return Err(ContractError::Invalid(format!(
            "unsupported review result: {}",
            record.result
        )));
    }
    if record.reviewed_digest.len() != 64
        || !record
            .reviewed_digest
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(ContractError::Invalid(
            "reviewed_digest must be lowercase SHA-256".into(),
        ));
    }
    Ok(())
}

fn validate_dispute(record: &DisputeRecord) -> ContractResult<()> {
    for (field, value) in [
        ("challenger_id", &record.challenger_id),
        ("disputed_record_id", &record.disputed_record_id),
        ("basis", &record.basis),
        ("affected_party_response", &record.affected_party_response),
        ("resolution", &record.resolution),
        ("contestation", &record.contestation),
    ] {
        require_text(value, field)?;
    }
    Ok(())
}

fn validate_syndicate(record: &SyndicateManifest) -> ContractResult<()> {
    for (field, value) in [
        ("syndicate_id", &record.syndicate_id),
        ("syndicate_name", &record.syndicate_name),
        ("jurisdiction", &record.jurisdiction),
        ("public_key", &record.public_key),
        ("charter_version", &record.charter_version),
        ("funding_disclosure", &record.funding_disclosure),
        ("conflict_disclosure", &record.conflict_disclosure),
    ] {
        require_text(value, field)?;
    }
    require_list(&record.roles, "roles")?;
    if record.public_key.len() != 64
        || !record
            .public_key
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(ContractError::Invalid(
            "public_key must be lowercase Ed25519 hex".into(),
        ));
    }
    Ok(())
}

pub fn validate_typed_record(value: &Value) -> ContractResult<TypedRecord> {
    let envelope = validate_envelope(value)?;
    match envelope.schema.as_str() {
        "justai.source.v1" => {
            let record: SourceRecord = parse(value, "source")?;
            validate_source(&record)?;
            Ok(TypedRecord::Source(record))
        }
        "justai.artifact_manifest.v1" => {
            let record: ArtifactManifest = parse(value, "artifact_manifest")?;
            validate_artifact_manifest(&record)?;
            Ok(TypedRecord::ArtifactManifest(record))
        }
        "justai.evaluation_receipt.v1" => {
            let record: EvaluationReceipt = parse(value, "evaluation_receipt")?;
            validate_evaluation(&record)?;
            Ok(TypedRecord::Evaluation(record))
        }
        "justai.disclosure_decision.v1" => {
            let record: DisclosureDecision = parse(value, "disclosure_decision")?;
            validate_disclosure(&record)?;
            Ok(TypedRecord::Disclosure(record))
        }
        "justai.claim_projection.v1" => {
            let record: ClaimProjection = parse(value, "claim_projection")?;
            validate_projection(&record)?;
            Ok(TypedRecord::Projection(record))
        }
        "justai.policy_trace.v1" => {
            let record: PolicyTraceRecord = parse(value, "policy_trace")?;
            validate_policy_trace(&record)?;
            Ok(TypedRecord::PolicyTrace(record))
        }
        "justai.claim.v1" => {
            let record: ClaimRecord = parse(value, "claim")?;
            validate_claim(&record)?;
            Ok(TypedRecord::Claim(record))
        }
        "justai.incident.v1" => {
            let record: IncidentRecord = parse(value, "incident")?;
            validate_incident(&record)?;
            Ok(TypedRecord::Incident(record))
        }
        "justai.commitment.v1" => {
            let record: CommitmentRecord = parse(value, "commitment")?;
            validate_commitment(&record)?;
            Ok(TypedRecord::Commitment(record))
        }
        "justai.review_receipt.v1" => {
            let record: ReviewReceipt = parse(value, "review_receipt")?;
            validate_review(&record)?;
            Ok(TypedRecord::Review(record))
        }
        "justai.dispute.v1" => {
            let record: DisputeRecord = parse(value, "dispute")?;
            validate_dispute(&record)?;
            Ok(TypedRecord::Dispute(record))
        }
        "justai.syndicate.v1" => {
            let record: SyndicateManifest = parse(value, "syndicate")?;
            validate_syndicate(&record)?;
            Ok(TypedRecord::Syndicate(record))
        }
        schema => Err(ContractError::Invalid(format!(
            "unsupported public record schema: {schema}"
        ))),
    }
}
