use std::collections::{BTreeMap, BTreeSet};

use hyperintel_core::{CanonicalizationError, Sha256Digest, UtcTimestamp, domain_separated_digest};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

use crate::{
    AssuranceEnvelopeV1, SignatureError, SignedStatementV1, VerificationKeyRegistry,
    verify_signed_statement_v1,
};

/// Visibility classification for evidence content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceClassification {
    /// Safe for public distribution.
    Public,
    /// Restricted to authorized participants.
    Restricted,
}

/// Content-addressed evidence reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBlobRefV1 {
    /// Must be `sha256` in v1.
    pub digest_algorithm: String,
    /// Content digest.
    pub digest: Sha256Digest,
    /// Exact byte length.
    pub byte_length: u64,
    /// Bounded ASCII media type.
    pub media_type: String,
    /// Evidence visibility.
    pub classification: EvidenceClassification,
    /// Portable `cas://sha256/<digest>` locator.
    pub storage_locator: String,
}

/// Generic assurance set. It contains no route or settlement records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceEvidenceSetV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Typed assurance envelope.
    pub envelope: AssuranceEnvelopeV1,
    /// Referenced evidence content.
    pub evidence_blobs: BTreeSet<EvidenceBlobRefV1>,
    /// Attributable signed statements.
    pub signed_statements: Vec<SignedStatementV1>,
    /// Code or build provenance commitment.
    pub code_provenance_digest: Sha256Digest,
    /// Digest of every preceding field.
    pub evidence_set_digest: Sha256Digest,
}

#[derive(Serialize)]
struct EvidenceSetIdentity<'a> {
    schema_version: &'a str,
    envelope: &'a AssuranceEnvelopeV1,
    evidence_blobs: &'a BTreeSet<EvidenceBlobRefV1>,
    signed_statements: &'a [SignedStatementV1],
    code_provenance_digest: Sha256Digest,
}

/// Assurance evidence verification failures.
#[derive(Debug, Error)]
pub enum AssuranceEvidenceError {
    /// Schema is not supported.
    #[error("unsupported assurance evidence schema")]
    UnsupportedSchema,
    /// Evidence metadata is malformed.
    #[error("invalid evidence reference")]
    InvalidReference,
    /// Envelope provenance does not exactly match supplied evidence references.
    #[error("assurance source digests do not match evidence references")]
    SourceDigestMismatch,
    /// Code provenance is absent.
    #[error("assurance code provenance digest is missing")]
    MissingCodeProvenance,
    /// Referenced bytes are absent.
    #[error("referenced evidence blob is missing")]
    MissingBlob,
    /// Referenced byte length differs.
    #[error("evidence byte length mismatch")]
    BlobLengthMismatch,
    /// Referenced digest differs.
    #[error("evidence digest mismatch")]
    BlobDigestMismatch,
    /// Set contains no attributable statement.
    #[error("assurance evidence set requires at least one signed statement")]
    MissingSignedStatement,
    /// A signed statement does not bind the envelope.
    #[error("signed statement does not bind the assurance envelope")]
    StatementBindingMismatch,
    /// A statement subject is absent from the assurance envelope.
    #[error("signed statement subject is absent from the assurance envelope")]
    StatementSubjectMismatch,
    /// A statement key is not declared under its caller-assigned trust class.
    #[error("signed statement key does not match an envelope trust root")]
    StatementTrustRootMismatch,
    /// The claimed maturity is not supported by the signed root composition.
    #[error("assurance maturity is not established by eligible evidence roots")]
    MaturityNotEstablished,
    /// This evidence schema cannot establish the requested maturity tier.
    #[error("assurance maturity tier is unsupported by evidence v1")]
    UnsupportedMaturity,
    /// Final evidence-set identity differs.
    #[error("assurance evidence-set digest mismatch")]
    EvidenceSetDigestMismatch,
    /// Envelope validation failed.
    #[error("assurance envelope rejected: {0}")]
    Envelope(#[from] crate::AssuranceRejectionV1),
    /// Signature validation failed.
    #[error("signed assurance statement rejected: {0}")]
    Signature(#[from] SignatureError),
    /// Canonicalization failed.
    #[error("evidence canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Build a generic assurance evidence set and its self-digest.
///
/// # Errors
///
/// Returns on invalid envelope, evidence metadata, or canonicalization.
pub fn build_assurance_evidence_set_v1(
    envelope: AssuranceEnvelopeV1,
    evidence_blobs: BTreeSet<EvidenceBlobRefV1>,
    signed_statements: Vec<SignedStatementV1>,
    code_provenance_digest: Sha256Digest,
) -> Result<AssuranceEvidenceSetV1, AssuranceEvidenceError> {
    envelope.validate()?;
    validate_references(&evidence_blobs)?;
    validate_source_binding(&envelope, &evidence_blobs, code_provenance_digest)?;
    if signed_statements.is_empty() {
        return Err(AssuranceEvidenceError::MissingSignedStatement);
    }
    let mut set = AssuranceEvidenceSetV1 {
        schema_version: "hyperintel.assurance-evidence-set.v1".to_owned(),
        envelope,
        evidence_blobs,
        signed_statements,
        code_provenance_digest,
        evidence_set_digest: Sha256Digest::from_bytes([0; 32]),
    };
    set.evidence_set_digest = evidence_set_digest(&set)?;
    Ok(set)
}

/// Verify the complete set without a database.
///
/// # Errors
///
/// Returns on any schema, metadata, blob, signature, binding, or digest failure.
pub fn verify_assurance_evidence_set_v1(
    set: &AssuranceEvidenceSetV1,
    blobs: &BTreeMap<Sha256Digest, Vec<u8>>,
    registry: &VerificationKeyRegistry,
    evaluated_at: &UtcTimestamp,
) -> Result<(), AssuranceEvidenceError> {
    if set.schema_version != "hyperintel.assurance-evidence-set.v1" {
        return Err(AssuranceEvidenceError::UnsupportedSchema);
    }
    set.envelope.validate()?;
    validate_references(&set.evidence_blobs)?;
    validate_source_binding(
        &set.envelope,
        &set.evidence_blobs,
        set.code_provenance_digest,
    )?;
    let envelope_digest = set.envelope.digest()?;
    if set.signed_statements.is_empty() {
        return Err(AssuranceEvidenceError::MissingSignedStatement);
    }
    let mut has_independent_verifier = false;
    for statement in &set.signed_statements {
        if statement.assurance_envelope_digest != envelope_digest {
            return Err(AssuranceEvidenceError::StatementBindingMismatch);
        }
        let subject_is_bound = set
            .envelope
            .guarantees
            .iter()
            .chain(&set.envelope.assumptions)
            .chain(&set.envelope.exclusions)
            .any(|claim| claim.subject_digest == statement.subject_digest);
        if !subject_is_bound {
            return Err(AssuranceEvidenceError::StatementSubjectMismatch);
        }
        let root_class = registry
            .trust_root_class(&statement.signer_id, &statement.key_id)
            .ok_or(AssuranceEvidenceError::StatementTrustRootMismatch)?;
        if !set
            .envelope
            .trust_roots
            .iter()
            .any(|root| root.root_id == statement.key_id && root.class == root_class)
        {
            return Err(AssuranceEvidenceError::StatementTrustRootMismatch);
        }
        has_independent_verifier |= root_class == crate::AssuranceRootClassV1::VerifierKey;
        verify_signed_statement_v1(statement, registry, evaluated_at)?;
    }
    match set.envelope.maturity {
        hyperintel_core::AssuranceMaturity::Declared => {}
        hyperintel_core::AssuranceMaturity::LocallyChecked if has_independent_verifier => {}
        hyperintel_core::AssuranceMaturity::LocallyChecked => {
            return Err(AssuranceEvidenceError::MaturityNotEstablished);
        }
        hyperintel_core::AssuranceMaturity::Attested
        | hyperintel_core::AssuranceMaturity::MachineChecked
        | hyperintel_core::AssuranceMaturity::IndependentlyReproduced => {
            return Err(AssuranceEvidenceError::UnsupportedMaturity);
        }
    }
    for reference in &set.evidence_blobs {
        let bytes = blobs
            .get(&reference.digest)
            .ok_or(AssuranceEvidenceError::MissingBlob)?;
        if u64::try_from(bytes.len()).ok() != Some(reference.byte_length) {
            return Err(AssuranceEvidenceError::BlobLengthMismatch);
        }
        let actual = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
        if actual != reference.digest {
            return Err(AssuranceEvidenceError::BlobDigestMismatch);
        }
    }
    if evidence_set_digest(set)? != set.evidence_set_digest {
        return Err(AssuranceEvidenceError::EvidenceSetDigestMismatch);
    }
    Ok(())
}

fn validate_source_binding(
    envelope: &AssuranceEnvelopeV1,
    references: &BTreeSet<EvidenceBlobRefV1>,
    code_provenance_digest: Sha256Digest,
) -> Result<(), AssuranceEvidenceError> {
    if code_provenance_digest.is_zero() {
        return Err(AssuranceEvidenceError::MissingCodeProvenance);
    }
    let referenced_digests = references
        .iter()
        .map(|reference| reference.digest)
        .collect::<BTreeSet<_>>();
    if envelope.source_digests != referenced_digests {
        return Err(AssuranceEvidenceError::SourceDigestMismatch);
    }
    Ok(())
}

fn evidence_set_digest(
    set: &AssuranceEvidenceSetV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.assurance-evidence-set.v1",
        &EvidenceSetIdentity {
            schema_version: &set.schema_version,
            envelope: &set.envelope,
            evidence_blobs: &set.evidence_blobs,
            signed_statements: &set.signed_statements,
            code_provenance_digest: set.code_provenance_digest,
        },
    )
}

fn validate_references(
    references: &BTreeSet<EvidenceBlobRefV1>,
) -> Result<(), AssuranceEvidenceError> {
    for reference in references {
        if reference.digest_algorithm != "sha256"
            || reference.digest.is_zero()
            || reference.byte_length == 0
            || reference.byte_length > 16 * 1024 * 1024
            || reference.media_type.is_empty()
            || reference.media_type.len() > 127
            || !reference.media_type.is_ascii()
            || reference.storage_locator != format!("cas://sha256/{}", reference.digest.to_hex())
        {
            return Err(AssuranceEvidenceError::InvalidReference);
        }
    }
    Ok(())
}

/// Helper for constructing a portable SHA-256 evidence reference.
///
/// # Errors
///
/// Returns if identifiers or metadata violate v1 bounds.
pub fn evidence_blob_ref_v1(
    bytes: &[u8],
    media_type: &str,
    classification: EvidenceClassification,
) -> Result<EvidenceBlobRefV1, AssuranceEvidenceError> {
    let digest = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
    let reference = EvidenceBlobRefV1 {
        digest_algorithm: "sha256".to_owned(),
        digest,
        byte_length: u64::try_from(bytes.len())
            .map_err(|_| AssuranceEvidenceError::InvalidReference)?,
        media_type: media_type.to_owned(),
        classification,
        storage_locator: format!("cas://sha256/{}", digest.to_hex()),
    };
    validate_references(&[reference.clone()].into_iter().collect())?;
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use data_encoding::HEXLOWER;
    use ed25519_dalek::{Signer as _, SigningKey};

    use super::*;
    use crate::{AssuranceClaimV1, AssuranceRootClassV1, AssuranceRootV1, ValidityWindowV1};
    use hyperintel_core::{AssuranceMaturity, AssuranceProperty};

    #[test]
    fn contract_evidence_set_verifies_offline_and_rejects_tamper()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence_bytes = b"authorized provider declaration".to_vec();
        let evidence_ref = evidence_blob_ref_v1(
            &evidence_bytes,
            "application/json",
            EvidenceClassification::Restricted,
        )?;
        let envelope = AssuranceEnvelopeV1 {
            schema_version: "hyperintel.assurance-envelope.v1".to_owned(),
            guarantees: [AssuranceClaimV1 {
                subject_digest: Sha256Digest::from_bytes([1; 32]),
                property: AssuranceProperty::ProviderAuthorization,
            }]
            .into_iter()
            .collect(),
            assumptions: BTreeSet::new(),
            exclusions: BTreeSet::new(),
            maturity: AssuranceMaturity::Declared,
            trust_roots: [AssuranceRootV1 {
                class: AssuranceRootClassV1::ProviderKey,
                root_id: "key:alpha-1".parse()?,
            }]
            .into_iter()
            .collect(),
            valid: ValidityWindowV1 {
                start: "2026-07-20T00:00:00Z".parse()?,
                end: "2026-07-21T00:00:00Z".parse()?,
            },
            source_digests: [evidence_ref.digest].into_iter().collect(),
        };
        let mut wrong_source = envelope.clone();
        wrong_source.source_digests = [Sha256Digest::from_bytes([99; 32])].into_iter().collect();
        assert!(matches!(
            build_assurance_evidence_set_v1(
                wrong_source,
                [evidence_ref.clone()].into_iter().collect(),
                Vec::new(),
                Sha256Digest::from_bytes([8; 32])
            ),
            Err(AssuranceEvidenceError::SourceDigestMismatch)
        ));
        let signing_key = SigningKey::from_bytes(&[11; 32]);
        let mut registry = VerificationKeyRegistry::default();
        let signer_id: hyperintel_core::ProtocolId = "provider:alpha".parse()?;
        let key_id: hyperintel_core::ProtocolId = "key:alpha-1".parse()?;
        registry.register(crate::RegisteredVerificationKeyV1 {
            signer_id: signer_id.clone(),
            key_id: key_id.clone(),
            trust_root_class: AssuranceRootClassV1::ProviderKey,
            public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
            valid_from: "2026-07-20T00:00:00Z".parse()?,
            valid_until: "2026-07-22T00:00:00Z".parse()?,
            revoked_at: None,
        })?;
        let mut statement = SignedStatementV1 {
            schema_version: "hyperintel.signed-statement.v1".to_owned(),
            subject_kind: "provider-identity".parse()?,
            subject_digest: Sha256Digest::from_bytes([1; 32]),
            assurance_envelope_digest: envelope.digest()?,
            issued_at: "2026-07-20T10:00:00Z".parse()?,
            expires_at: "2026-07-21T00:00:00Z".parse()?,
            sequence: 1,
            signer_id,
            key_id,
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        statement.signature_hex =
            HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
        let set = build_assurance_evidence_set_v1(
            envelope,
            [evidence_ref.clone()].into_iter().collect(),
            vec![statement],
            Sha256Digest::from_bytes([8; 32]),
        )?;
        let mut blobs = BTreeMap::from([(evidence_ref.digest, evidence_bytes)]);
        let evaluated_at = "2026-07-20T12:00:00Z".parse()?;
        verify_assurance_evidence_set_v1(&set, &blobs, &registry, &evaluated_at)?;
        if let Some(bytes) = blobs.get_mut(&evidence_ref.digest) {
            bytes[0] ^= 1;
        }
        assert!(matches!(
            verify_assurance_evidence_set_v1(&set, &blobs, &registry, &evaluated_at),
            Err(AssuranceEvidenceError::BlobDigestMismatch)
        ));
        Ok(())
    }
}
