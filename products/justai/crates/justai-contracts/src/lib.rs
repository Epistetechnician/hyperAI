use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use thiserror::Error;

pub mod governed;
pub mod records;
pub use governed::{
    canonical_governed_bytes, enforce_claim_ceiling, integration_handoff_payload,
    seal_governed_record, validate_governed_record, verify_integration_handoff_seal,
    DecisionRecord, DissentRecord, EvidenceManifest, GovernanceFields, GovernedRecord,
    IntegrationHandoffSeal, IntegrationResult, IntuitionReview, PromotionDecision, ProvenanceRef,
    SerendipityRecord, UnknownRecord, GOVERNED_STATE_SLICE,
};
pub use records::{
    validate_typed_record, ArtifactManifest, ClaimProjection, ClaimRecord, CommitmentRecord,
    DisclosureDecision, DisputeRecord, EvaluationReceipt, IncidentRecord, PolicyTraceRecord,
    ResourceLimits, ReviewReceipt, SourceRecord, SyndicateManifest, TypedRecord,
};

pub const RELEASE_SCHEMA: &str = "justai.release.v1";
pub const RELEASE_PURPOSE: &str = "release";

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("duplicate JSON object key: {0}")]
    DuplicateKey(String),
    #[error("invalid contract: {0}")]
    Invalid(String),
    #[error("invalid hexadecimal value for {field}")]
    Hex { field: &'static str },
    #[error("signature verification failed for key {key_id}")]
    Signature { key_id: String },
}

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureRef {
    pub key_id: String,
    pub purpose: String,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub schema: String,
    pub record_id: String,
    pub state_slice: String,
    pub status: String,
    pub claim_ceiling: String,
    pub created_at: String,
    pub publisher_id: String,
    pub source_digests: Vec<String>,
    pub content_digest: String,
    pub signature_refs: Vec<SignatureRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub schema: String,
    pub record_id: String,
    pub state_slice: String,
    pub status: String,
    pub claim_ceiling: String,
    pub created_at: String,
    pub publisher_id: String,
    pub source_digests: Vec<String>,
    pub content_digest: String,
    pub signature_refs: Vec<SignatureRef>,
    pub records: Vec<Value>,
    pub supersedes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Submitted,
    Quarantined,
    Corroborated,
    IndependentlyVerified,
    Contested,
    Withdrawn,
}

impl EvidenceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "Submitted",
            Self::Quarantined => "Quarantined",
            Self::Corroborated => "Corroborated",
            Self::IndependentlyVerified => "IndependentlyVerified",
            Self::Contested => "Contested",
            Self::Withdrawn => "Withdrawn",
        }
    }
}

pub fn parse_json_no_duplicates(input: &[u8]) -> ContractResult<Value> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = deserializer.deserialize_any(NoDuplicateVisitor)?;
    deserializer.end()?;
    Ok(value)
}

struct NoDuplicateSeed;

impl<'de> DeserializeSeed<'de> for NoDuplicateSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateVisitor)
    }
}

struct NoDuplicateVisitor;

impl<'de> Visitor<'de> for NoDuplicateVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with unique object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateVisitor)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(NoDuplicateSeed)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut keys = HashSet::new();
        while let Some(key) = access.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate JSON object key: {key}"
                )));
            }
            let value = access.next_value_seed(NoDuplicateSeed)?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

pub fn canonical_bytes(value: &Value) -> ContractResult<Vec<u8>> {
    fn reject_non_integer_numbers(value: &Value) -> ContractResult<()> {
        match value {
            Value::Array(values) => values.iter().try_for_each(reject_non_integer_numbers),
            Value::Object(values) => {
                for (key, nested) in values {
                    if !key.is_ascii() {
                        return Err(ContractError::Invalid(
                            "non-ASCII JSON object keys are not supported by the cross-language digest contract"
                                .into(),
                        ));
                    }
                    reject_non_integer_numbers(nested)?;
                }
                Ok(())
            }
            Value::Number(number) if number.as_i64().is_none() && number.as_u64().is_none() => {
                Err(ContractError::Invalid(
                    "floating-point JSON is not supported by the cross-language digest contract"
                        .into(),
                ))
            }
            _ => Ok(()),
        }
    }

    reject_non_integer_numbers(value)?;
    serde_jcs::to_vec(value).map_err(|error| ContractError::Invalid(format!("JCS: {error}")))
}

pub fn digest_without_field(value: &Value, field: &str) -> ContractResult<String> {
    let mut copy = value.clone();
    let object = copy
        .as_object_mut()
        .ok_or_else(|| ContractError::Invalid("top-level value must be an object".into()))?;
    object.remove(field);
    let canonical = canonical_bytes(&copy)?;
    let digest = Sha256::digest(canonical);
    Ok(hex::encode(digest))
}

/// Hashes release content while excluding generated digest and signature
/// metadata. This prevents circular signing while keeping substantive content
/// digest-bound.
pub fn content_digest(value: &Value) -> ContractResult<String> {
    let mut copy = value.clone();
    let object = copy
        .as_object_mut()
        .ok_or_else(|| ContractError::Invalid("top-level value must be an object".into()))?;
    object.remove("content_digest");
    object.remove("signature_refs");
    let canonical = canonical_bytes(&copy)?;
    let digest = Sha256::digest(canonical);
    Ok(hex::encode(digest))
}

pub fn seal_record(value: &mut Value) -> ContractResult<()> {
    {
        let object = value
            .as_object_mut()
            .ok_or_else(|| ContractError::Invalid("record must be an object".into()))?;
        object
            .entry("content_digest")
            .or_insert_with(|| Value::String(String::new()));
        object
            .entry("signature_refs")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    let actual_digest = content_digest(value)?;
    if let Some(supplied_digest) = value.get("content_digest").and_then(Value::as_str) {
        if !supplied_digest.is_empty() && supplied_digest != actual_digest {
            return Err(ContractError::Invalid(format!(
                "record content_digest mismatch: expected {actual_digest}, got {supplied_digest}"
            )));
        }
    }
    value
        .as_object_mut()
        .ok_or_else(|| ContractError::Invalid("record must be an object".into()))?
        .insert("content_digest".into(), Value::String(actual_digest));
    Ok(())
}

fn validate_public_release_records(records: &[Value], publisher_id: &str) -> ContractResult<()> {
    for record in records {
        reject_disclosure_fields(record)?;
    }
    let typed_records: Vec<TypedRecord> = records
        .iter()
        .map(validate_typed_record)
        .collect::<ContractResult<Vec<_>>>()?;
    let mut record_ids: HashMap<String, usize> = HashMap::new();
    for (index, typed) in typed_records.iter().enumerate() {
        let id = typed.envelope().record_id.clone();
        if record_ids.insert(id.clone(), index).is_some() {
            return Err(ContractError::Invalid(format!("duplicate record_id: {id}")));
        }
    }

    let by_id: HashMap<&str, &TypedRecord> = typed_records
        .iter()
        .map(|record| (record.envelope().record_id.as_str(), record))
        .collect();
    let mut source_digests = HashSet::new();
    let mut record_digests = HashSet::new();
    let mut reviews_by_record: HashMap<&str, Vec<&ReviewReceipt>> = HashMap::new();
    let mut manifests_by_record_id: HashMap<&str, &SyndicateManifest> = HashMap::new();
    let mut has_publisher_manifest = false;
    for typed in &typed_records {
        record_digests.insert(typed.envelope().content_digest.clone());
        match typed {
            TypedRecord::Source(source) => {
                source_digests.extend(source.envelope.source_digests.iter().cloned());
            }
            TypedRecord::ArtifactManifest(artifact) => {
                source_digests.insert(artifact.artifact_digest.clone());
                source_digests.extend(artifact.envelope.source_digests.iter().cloned());
            }
            TypedRecord::Review(review) => {
                reviews_by_record
                    .entry(review.reviewed_record_id.as_str())
                    .or_default()
                    .push(review);
            }
            TypedRecord::Syndicate(syndicate) => {
                if manifests_by_record_id
                    .insert(syndicate.envelope.record_id.as_str(), syndicate)
                    .is_some()
                {
                    return Err(ContractError::Invalid(format!(
                        "duplicate syndicate manifest: {}",
                        syndicate.syndicate_id
                    )));
                }
                if syndicate.syndicate_id == publisher_id {
                    has_publisher_manifest = true;
                }
            }
            _ => {}
        }
    }
    if !has_publisher_manifest {
        return Err(ContractError::Invalid(
            "published release requires a syndicate manifest for its publisher".into(),
        ));
    }

    for typed in &typed_records {
        if let TypedRecord::Review(review) = typed {
            let target = by_id
                .get(review.reviewed_record_id.as_str())
                .ok_or_else(|| {
                    ContractError::Invalid(format!(
                        "review references unknown record: {}",
                        review.reviewed_record_id
                    ))
                })?;
            if review.reviewed_digest != target.envelope().content_digest {
                return Err(ContractError::Invalid(format!(
                    "review receipt {} digest does not match {}",
                    review.envelope.record_id, review.reviewed_record_id
                )));
            }
            let manifest = manifests_by_record_id
                .get(review.syndicate_manifest_ref.as_str())
                .ok_or_else(|| {
                    ContractError::Invalid(format!(
                        "review receipt {} references missing syndicate manifest {}",
                        review.envelope.record_id, review.syndicate_manifest_ref
                    ))
                })?;
            if manifest.syndicate_id != review.reviewer_org {
                return Err(ContractError::Invalid(format!(
                    "review receipt {} is not bound to reviewer organization {}",
                    review.envelope.record_id, review.reviewer_org
                )));
            }
            if manifest.envelope.signature_refs.is_empty() {
                return Err(ContractError::Invalid(format!(
                    "reviewer syndicate manifest {} must be signed",
                    manifest.envelope.record_id
                )));
            }
            verify_record_signatures(&serde_json::to_value(manifest)?)?;
        }
    }

    let require_record_ref = |owner: &str, reference: &str| -> ContractResult<&TypedRecord> {
        by_id.get(reference).copied().ok_or_else(|| {
            ContractError::Invalid(format!(
                "record {owner} references unknown record: {reference}"
            ))
        })
    };
    let require_schema = |owner: &str, reference: &str, schema: &str| -> ContractResult<()> {
        let record = require_record_ref(owner, reference)?;
        if record.envelope().schema != schema {
            return Err(ContractError::Invalid(format!(
                "record {owner} reference {reference} must use schema {schema}"
            )));
        }
        Ok(())
    };

    for typed in &typed_records {
        let (envelope, source_refs, review_refs) = match typed {
            TypedRecord::Claim(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            TypedRecord::Incident(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            TypedRecord::Commitment(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            TypedRecord::Evaluation(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            TypedRecord::Disclosure(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            TypedRecord::Projection(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            TypedRecord::PolicyTrace(record) => (
                &record.envelope,
                &record.envelope.source_digests,
                &record.review_receipt_refs,
            ),
            _ => continue,
        };
        if envelope.publisher_id != publisher_id {
            return Err(ContractError::Invalid(format!(
                "published record {} has a different publisher",
                envelope.record_id
            )));
        }
        if source_refs.is_empty()
            || source_refs
                .iter()
                .any(|digest| !source_digests.contains(digest) && !record_digests.contains(digest))
        {
            return Err(ContractError::Invalid(format!(
                "record {} has unresolved source provenance",
                envelope.record_id
            )));
        }
        if review_refs.len() < 2 {
            return Err(ContractError::Invalid(format!(
                "record {} requires at least two review receipts",
                envelope.record_id
            )));
        }
        let mut review_ids = HashSet::new();
        let mut reviewer_orgs = HashSet::new();
        for review_id in review_refs {
            if !review_ids.insert(review_id) {
                return Err(ContractError::Invalid(format!(
                    "record {} repeats review receipt {}",
                    envelope.record_id, review_id
                )));
            }
            let review = reviews_by_record
                .get(envelope.record_id.as_str())
                .and_then(|reviews| {
                    reviews
                        .iter()
                        .find(|review| review.envelope.record_id == review_id.as_str())
                })
                .ok_or_else(|| {
                    ContractError::Invalid(format!(
                        "record {} references missing review receipt {}",
                        envelope.record_id, review_id
                    ))
                })?;
            if review.reviewed_digest != envelope.content_digest
                || review.result != "accept"
                || review.conflict_of_interest
                || review.reviewer_org == publisher_id
            {
                return Err(ContractError::Invalid(format!(
                    "review receipt {} is not an independent acceptance of {}",
                    review.envelope.record_id, envelope.record_id
                )));
            }
            reviewer_orgs.insert(review.reviewer_org.clone());
        }
        if reviewer_orgs.len() < 2 {
            return Err(ContractError::Invalid(format!(
                "record {} requires reviewers from two organizations",
                envelope.record_id
            )));
        }
    }

    for typed in &typed_records {
        match typed {
            TypedRecord::Claim(record) => {
                for reference in record
                    .support_refs
                    .iter()
                    .chain(record.contradiction_refs.iter())
                {
                    require_record_ref(&record.envelope.record_id, reference)?;
                }
            }
            TypedRecord::Commitment(record) => {
                for reference in &record.evidence_refs {
                    require_record_ref(&record.envelope.record_id, reference)?;
                }
            }
            TypedRecord::Incident(_) => {}
            TypedRecord::Dispute(record) => {
                require_record_ref(&record.envelope.record_id, &record.disputed_record_id)?;
            }
            TypedRecord::Evaluation(record) => {
                for reference in &record.artifact_manifest_refs {
                    let artifact = require_record_ref(&record.envelope.record_id, reference)?;
                    require_schema(
                        &record.envelope.record_id,
                        reference,
                        "justai.artifact_manifest.v1",
                    )?;
                    if let TypedRecord::ArtifactManifest(artifact) = artifact {
                        if !record
                            .envelope
                            .source_digests
                            .contains(&artifact.artifact_digest)
                        {
                            return Err(ContractError::Invalid(format!(
                                "evaluation {} does not bind artifact digest {}",
                                record.envelope.record_id, artifact.artifact_digest
                            )));
                        }
                    }
                }
                for reference in &record.disclosure_decision_refs {
                    require_schema(
                        &record.envelope.record_id,
                        reference,
                        "justai.disclosure_decision.v1",
                    )?;
                }
            }
            TypedRecord::Disclosure(record) => {
                for reference in &record.subject_refs {
                    require_record_ref(&record.envelope.record_id, reference)?;
                }
            }
            TypedRecord::Projection(record) => {
                for reference in &record.evidence_refs {
                    require_record_ref(&record.envelope.record_id, reference)?;
                }
                for reference in &record.disclosure_decision_refs {
                    require_schema(
                        &record.envelope.record_id,
                        reference,
                        "justai.disclosure_decision.v1",
                    )?;
                }
            }
            TypedRecord::PolicyTrace(record) => {
                for reference in &record.artifact_refs {
                    let artifact = require_record_ref(&record.envelope.record_id, reference)?;
                    require_schema(
                        &record.envelope.record_id,
                        reference,
                        "justai.artifact_manifest.v1",
                    )?;
                    if let TypedRecord::ArtifactManifest(artifact) = artifact {
                        if !record
                            .envelope
                            .source_digests
                            .contains(&artifact.artifact_digest)
                        {
                            return Err(ContractError::Invalid(format!(
                                "policy trace {} does not bind artifact digest {}",
                                record.envelope.record_id, artifact.artifact_digest
                            )));
                        }
                    }
                }
                for reference in &record.evaluation_receipt_refs {
                    require_schema(
                        &record.envelope.record_id,
                        reference,
                        "justai.evaluation_receipt.v1",
                    )?;
                }
                for reference in &record.claim_projection_refs {
                    require_schema(
                        &record.envelope.record_id,
                        reference,
                        "justai.claim_projection.v1",
                    )?;
                }
            }
            TypedRecord::Source(_)
            | TypedRecord::ArtifactManifest(_)
            | TypedRecord::Review(_)
            | TypedRecord::Syndicate(_) => {}
        }
    }
    Ok(())
}

fn reject_disclosure_fields(value: &Value) -> ContractResult<()> {
    const FORBIDDEN_FIELDS: &[&str] = &[
        "raw_payload",
        "raw_bytes",
        "private_locator",
        "secret",
        "credential",
        "restricted_payload",
    ];
    match value {
        Value::Array(values) => {
            for value in values {
                reject_disclosure_fields(value)?;
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                if FORBIDDEN_FIELDS.contains(&key.as_str()) {
                    return Err(ContractError::Invalid(format!(
                        "public release contains forbidden disclosure field: {key}"
                    )));
                }
                reject_disclosure_fields(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn assemble_release(
    mut records: Vec<Value>,
    publisher_id: &str,
    created_at: &str,
    release_id: &str,
) -> ContractResult<Value> {
    if publisher_id.trim().is_empty()
        || created_at.trim().is_empty()
        || release_id.trim().is_empty()
    {
        return Err(ContractError::Invalid(
            "publisher_id, created_at, and release_id are required".into(),
        ));
    }
    if records.is_empty() {
        return Err(ContractError::Invalid(
            "release must contain records".into(),
        ));
    }
    for record in &mut records {
        seal_record(record)?;
        validate_typed_record(record)?;
    }
    validate_public_release_records(&records, publisher_id)?;
    records.sort_by(|left, right| {
        let left_id = left
            .get("record_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right_id = right
            .get("record_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        left_id.cmp(right_id)
    });
    let mut source_digest_set = BTreeSet::new();
    for record in &records {
        if record.get("schema").and_then(Value::as_str) == Some("justai.source.v1") {
            if let Some(digests) = record.get("source_digests").and_then(Value::as_array) {
                for digest in digests.iter().filter_map(Value::as_str) {
                    source_digest_set.insert(digest.to_string());
                }
            }
        }
    }
    let mut release = serde_json::json!({
        "schema": RELEASE_SCHEMA,
        "record_id": release_id,
        "state_slice": "justai-public-observatory-v1",
        "status": "Published",
        "claim_ceiling": "bounded-public-observatory",
        "created_at": created_at,
        "publisher_id": publisher_id,
        "source_digests": source_digest_set.into_iter().collect::<Vec<_>>(),
        "content_digest": "",
        "signature_refs": [],
        "records": records,
        "supersedes": [],
    });
    let digest = content_digest(&release)?;
    release["content_digest"] = Value::String(digest);
    validate_release(&release, false)?;
    Ok(release)
}

pub fn signature_payload(schema: &str, digest: &str, purpose: &str) -> Vec<u8> {
    format!("justai:{schema}:{digest}:{purpose}").into_bytes()
}

pub fn validate_envelope(value: &Value) -> ContractResult<Envelope> {
    let envelope: Envelope = serde_json::from_value(value.clone())
        .map_err(|error| ContractError::Invalid(format!("envelope shape: {error}")))?;
    validate_envelope_fields(&envelope)?;
    let actual_digest = content_digest(value)?;
    if envelope.content_digest != actual_digest {
        return Err(ContractError::Invalid(format!(
            "content_digest mismatch: expected {actual_digest}, got {}",
            envelope.content_digest
        )));
    }
    Ok(envelope)
}

fn validate_envelope_fields(envelope: &Envelope) -> ContractResult<()> {
    for (name, value) in [
        ("schema", envelope.schema.as_str()),
        ("record_id", envelope.record_id.as_str()),
        ("state_slice", envelope.state_slice.as_str()),
        ("status", envelope.status.as_str()),
        ("claim_ceiling", envelope.claim_ceiling.as_str()),
        ("created_at", envelope.created_at.as_str()),
        ("publisher_id", envelope.publisher_id.as_str()),
    ] {
        if value.is_empty() {
            return Err(ContractError::Invalid(format!("{name} must not be empty")));
        }
    }
    if !envelope.schema.starts_with("justai.") {
        return Err(ContractError::Invalid(
            "schema must start with justai.".into(),
        ));
    }
    validate_digest(&envelope.content_digest, "content_digest")?;
    for digest in &envelope.source_digests {
        validate_digest(digest, "source_digest")?;
    }
    validate_status(&envelope.status)?;
    for signature in &envelope.signature_refs {
        if signature.key_id.is_empty() || signature.purpose.is_empty() {
            return Err(ContractError::Invalid(
                "signature key_id and purpose are required".into(),
            ));
        }
        validate_hex(&signature.public_key, "public_key", 32)?;
        validate_hex(&signature.signature, "signature", 64)?;
    }
    Ok(())
}

fn validate_digest(value: &str, field: &'static str) -> ContractResult<()> {
    validate_hex(value, field, 32)
}

fn validate_hex(value: &str, field: &'static str, bytes: usize) -> ContractResult<()> {
    if value.len() != bytes * 2
        || !value
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(ContractError::Hex { field });
    }
    Ok(())
}

fn validate_status(status: &str) -> ContractResult<()> {
    const ALLOWED: &[&str] = &[
        "Submitted",
        "Quarantined",
        "Corroborated",
        "IndependentlyVerified",
        "Contested",
        "Withdrawn",
        "Draft",
        "Published",
        "Superseded",
        "Declared",
        "PartiallyVerified",
        "Fulfilled",
        "Overdue",
        "NotReady",
        "Unknown",
        "NoCandidate",
    ];
    if ALLOWED.contains(&status) {
        Ok(())
    } else {
        Err(ContractError::Invalid(format!(
            "unsupported status: {status}"
        )))
    }
}

pub fn validate_release(value: &Value, require_signature: bool) -> ContractResult<ReleaseManifest> {
    let release: ReleaseManifest = serde_json::from_value(value.clone())
        .map_err(|error| ContractError::Invalid(format!("release shape: {error}")))?;
    if release.schema != RELEASE_SCHEMA {
        return Err(ContractError::Invalid(format!(
            "release schema must be {RELEASE_SCHEMA}"
        )));
    }
    let envelope_value = serde_json::to_value(&release)?;
    validate_envelope_fields(&Envelope {
        schema: release.schema.clone(),
        record_id: release.record_id.clone(),
        state_slice: release.state_slice.clone(),
        status: release.status.clone(),
        claim_ceiling: release.claim_ceiling.clone(),
        created_at: release.created_at.clone(),
        publisher_id: release.publisher_id.clone(),
        source_digests: release.source_digests.clone(),
        content_digest: release.content_digest.clone(),
        signature_refs: release.signature_refs.clone(),
    })?;
    let actual_digest = content_digest(&envelope_value)?;
    if actual_digest != release.content_digest {
        return Err(ContractError::Invalid(format!(
            "release content_digest mismatch: expected {actual_digest}, got {}",
            release.content_digest
        )));
    }
    if require_signature && release.signature_refs.is_empty() {
        return Err(ContractError::Invalid(
            "release requires at least one signature".into(),
        ));
    }
    let mut record_ids = HashSet::new();
    for record in &release.records {
        let typed = validate_typed_record(record)?;
        let envelope = typed.envelope();
        if !record_ids.insert(envelope.record_id.clone()) {
            return Err(ContractError::Invalid(format!(
                "duplicate record_id: {}",
                envelope.record_id
            )));
        }
    }
    if release.status == "Published" {
        validate_public_release_records(&release.records, &release.publisher_id)?;
    }
    Ok(release)
}

pub fn verify_signatures(value: &Value) -> ContractResult<()> {
    let release = validate_release(value, true)?;
    for signature_ref in release.signature_refs {
        let public_key_bytes =
            hex::decode(&signature_ref.public_key).map_err(|_| ContractError::Hex {
                field: "public_key",
            })?;
        let signature_bytes = hex::decode(&signature_ref.signature)
            .map_err(|_| ContractError::Hex { field: "signature" })?;
        let public_key_array: [u8; 32] =
            public_key_bytes
                .try_into()
                .map_err(|_| ContractError::Hex {
                    field: "public_key",
                })?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key_array).map_err(|_| ContractError::Signature {
                key_id: signature_ref.key_id.clone(),
            })?;
        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| ContractError::Signature {
                key_id: signature_ref.key_id.clone(),
            })?;
        verifying_key
            .verify(
                &signature_payload(
                    &release.schema,
                    &release.content_digest,
                    &signature_ref.purpose,
                ),
                &signature,
            )
            .map_err(|_| ContractError::Signature {
                key_id: signature_ref.key_id,
            })?;
    }
    Ok(())
}

pub fn verify_record_signatures(value: &Value) -> ContractResult<()> {
    let envelope = validate_envelope(value)?;
    if envelope.signature_refs.is_empty() {
        return Err(ContractError::Invalid(format!(
            "record {} requires at least one signature",
            envelope.record_id
        )));
    }
    for signature_ref in envelope.signature_refs {
        let public_key_bytes =
            hex::decode(&signature_ref.public_key).map_err(|_| ContractError::Hex {
                field: "public_key",
            })?;
        let signature_bytes = hex::decode(&signature_ref.signature)
            .map_err(|_| ContractError::Hex { field: "signature" })?;
        let public_key_array: [u8; 32] =
            public_key_bytes
                .try_into()
                .map_err(|_| ContractError::Hex {
                    field: "public_key",
                })?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key_array).map_err(|_| ContractError::Signature {
                key_id: signature_ref.key_id.clone(),
            })?;
        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| ContractError::Signature {
                key_id: signature_ref.key_id.clone(),
            })?;
        verifying_key
            .verify(
                &signature_payload(
                    &envelope.schema,
                    &envelope.content_digest,
                    &signature_ref.purpose,
                ),
                &signature,
            )
            .map_err(|_| ContractError::Signature {
                key_id: signature_ref.key_id,
            })?;
    }
    Ok(())
}

pub fn sign_release(
    value: &mut Value,
    key_id: &str,
    signing_key: &SigningKey,
) -> ContractResult<()> {
    {
        let object = value
            .as_object_mut()
            .ok_or_else(|| ContractError::Invalid("release must be an object".into()))?;
        object.insert("schema".into(), Value::String(RELEASE_SCHEMA.into()));
        object.insert("content_digest".into(), Value::String(String::new()));
        object.insert("signature_refs".into(), Value::Array(Vec::new()));
    }
    let digest = content_digest(value)?;
    let signature = signing_key.sign(&signature_payload(RELEASE_SCHEMA, &digest, RELEASE_PURPOSE));
    let public_key = hex::encode(signing_key.verifying_key().to_bytes());
    {
        let object = value
            .as_object_mut()
            .ok_or_else(|| ContractError::Invalid("release must be an object".into()))?;
        object.insert("content_digest".into(), Value::String(digest));
        object.insert(
            "signature_refs".into(),
            serde_json::json!([{
                "key_id": key_id,
                "purpose": RELEASE_PURPOSE,
                "public_key": public_key,
                "signature": hex::encode(signature.to_bytes())
            }]),
        );
    }
    Ok(())
}

pub fn parse_seed_hex(seed_hex: &str) -> ContractResult<SigningKey> {
    let bytes = hex::decode(seed_hex).map_err(|_| ContractError::Hex { field: "seed" })?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ContractError::Hex { field: "seed" })?;
    Ok(SigningKey::from_bytes(&array))
}

pub fn validate_transition(from: &str, to: &str) -> ContractResult<()> {
    let allowed = match from {
        "Submitted" => &["Quarantined"][..],
        "Quarantined" => &["Corroborated", "Withdrawn"][..],
        "Corroborated" => &["IndependentlyVerified", "Contested", "Withdrawn"][..],
        "IndependentlyVerified" => &["Contested", "Withdrawn"][..],
        "Contested" => &["Corroborated", "IndependentlyVerified", "Withdrawn"][..],
        "Withdrawn" => &[][..],
        _ => {
            return Err(ContractError::Invalid(format!(
                "unknown source status: {from}"
            )))
        }
    };
    if allowed.contains(&to) {
        Ok(())
    } else {
        Err(ContractError::Invalid(format!(
            "invalid evidence transition: {from} -> {to}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    fn record() -> Value {
        let mut value = serde_json::json!({
            "schema": "justai.claim.v1",
            "record_id": "claim-001",
            "state_slice": "justai-observatory-contracts-v1",
            "status": "Submitted",
            "claim_ceiling": "bounded-description-only",
            "created_at": "2026-09-09T12:00:00Z",
            "publisher_id": "syndicate-example",
            "source_digests": [],
            "content_digest": "",
            "signature_refs": [],
            "claim": "This is a development fixture, not public evidence.",
            "claim_type": "fact",
            "scope": "fixture-only",
            "uncertainty": "The fixture has no evidentiary value.",
            "affected_party_response": "Not applicable to a fixture.",
            "support_refs": ["fixture-source"],
            "contradiction_refs": [],
            "review_receipt_refs": []
        });
        let digest = content_digest(&value).unwrap();
        value["content_digest"] = Value::String(digest);
        value
    }

    fn release() -> Value {
        let record = record();
        let mut value = serde_json::json!({
            "schema": RELEASE_SCHEMA,
            "record_id": "release-001",
            "state_slice": "justai-public-observatory-v1",
            "status": "NotReady",
            "claim_ceiling": "bounded-description-only",
            "created_at": "2026-09-09T12:00:00Z",
            "publisher_id": "syndicate-example",
            "source_digests": [],
            "content_digest": "",
            "signature_refs": [],
            "records": [record],
            "supersedes": []
        });
        let digest = content_digest(&value).unwrap();
        value["content_digest"] = Value::String(digest);
        value
    }

    #[test]
    fn rejects_duplicate_keys_before_mapping_collapse() {
        let result = parse_json_no_duplicates(br#"{"a":1,"a":2}"#);
        assert!(matches!(result, Err(ContractError::Json(_))));
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON object key"));
    }

    #[test]
    fn canonical_digest_is_stable_for_reordered_keys() {
        let left = parse_json_no_duplicates(br#"{"b":"two","a":"one"}"#).unwrap();
        let right = parse_json_no_duplicates(br#"{"a":"one","b":"two"}"#).unwrap();
        assert_eq!(
            canonical_bytes(&left).unwrap(),
            canonical_bytes(&right).unwrap()
        );
        assert_eq!(
            content_digest(&left).unwrap(),
            content_digest(&right).unwrap()
        );
    }

    #[test]
    fn rejects_cross_language_digest_ambiguities() {
        let floating = serde_json::json!({"value": 1.5});
        assert!(canonical_bytes(&floating).is_err());
        let non_ascii_key = serde_json::json!({"clé": "value"});
        assert!(canonical_bytes(&non_ascii_key).is_err());
    }

    #[test]
    fn validates_signed_release_and_rejects_tampering() {
        let mut value = release();
        let signing_key = SigningKey::generate(&mut OsRng);
        sign_release(&mut value, "test-key", &signing_key).unwrap();
        validate_release(&value, true).unwrap();
        verify_signatures(&value).unwrap();
        value["records"][0]["status"] = Value::String("Withdrawn".into());
        assert!(verify_signatures(&value).is_err());
    }

    #[test]
    fn evidence_state_machine_is_fail_closed() {
        assert!(validate_transition("Submitted", "Quarantined").is_ok());
        assert!(validate_transition("Quarantined", "Corroborated").is_ok());
        assert!(validate_transition("IndependentlyVerified", "Submitted").is_err());
        assert!(validate_transition("Withdrawn", "Submitted").is_err());
    }

    #[test]
    fn unsigned_release_is_not_publishable() {
        assert!(validate_release(&release(), false).is_ok());
        assert!(validate_release(&release(), true).is_err());
    }

    #[test]
    fn unknown_custody_cannot_claim_a_verified_status() {
        let mut value = serde_json::json!({
            "schema": "justai.artifact_manifest.v1",
            "record_id": "artifact-unknown",
            "state_slice": "justai-evidence-bridge-v1",
            "status": "Submitted",
            "claim_ceiling": "bounded-observation-only",
            "created_at": "2026-09-09T12:00:00Z",
            "publisher_id": "syndicate-example",
            "source_digests": [],
            "content_digest": "",
            "signature_refs": [],
            "artifact_kind": "trace",
            "access_class": "restricted",
            "custody_mode": "unknown",
            "license_basis": "synthetic",
            "custodian_id": "custodian-example",
            "acquisition_timestamp": "2026-09-09T12:00:00Z",
            "artifact_digest": "1111111111111111111111111111111111111111111111111111111111111111",
            "permitted_uses": ["bounded-evaluation"],
            "retention_and_disclosure_rules": "withhold raw material",
            "evidence_basis": "unknown"
        });
        seal_record(&mut value).unwrap();
        assert!(validate_typed_record(&value).is_err());
        value["status"] = Value::String("Unknown".into());
        value["content_digest"] = Value::String(String::new());
        seal_record(&mut value).unwrap();
        assert!(validate_typed_record(&value).is_ok());
    }

    #[test]
    fn public_release_rejects_raw_restricted_payload_fields() {
        let mut value = serde_json::json!({
            "schema": "justai.artifact_manifest.v1",
            "record_id": "artifact-leak",
            "state_slice": "justai-evidence-bridge-v1",
            "status": "Unknown",
            "claim_ceiling": "bounded-observation-only",
            "created_at": "2026-09-09T12:00:00Z",
            "publisher_id": "syndicate-example",
            "source_digests": [],
            "content_digest": "",
            "signature_refs": [],
            "artifact_kind": "trace",
            "access_class": "restricted",
            "custody_mode": "unknown",
            "license_basis": "synthetic",
            "custodian_id": "custodian-example",
            "acquisition_timestamp": "2026-09-09T12:00:00Z",
            "artifact_digest": "1111111111111111111111111111111111111111111111111111111111111111",
            "permitted_uses": ["bounded-evaluation"],
            "retention_and_disclosure_rules": "withhold raw material",
            "evidence_basis": "unknown",
            "raw_payload": "must never enter a public receipt"
        });
        seal_record(&mut value).unwrap();
        let release = serde_json::json!([value]);
        assert!(reject_disclosure_fields(&release).is_err());
    }
}
