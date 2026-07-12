//! Content-addressed evidence bundles, signatures, and public verification.

// Wire fields are documented in the generated schemas and evidence protocol guide.
#![allow(missing_docs)]

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    hash::BuildHasher,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use outcomes_domain::{
    ClaimId, ClaimTruth, ContentBlobRefV1, OutcomeEvidenceBundleV1, ResolutionAttestationV1,
    ResolutionFinality, Sha256Digest, UtcTimestamp, canonical_json, domain_separated_digest,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use zeroize::Zeroize;

/// Fields used to construct an evidence bundle before its derived digest exists.
#[derive(Debug, Clone)]
pub struct EvidenceBundleInput {
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
}

/// Evidence construction and verification failures.
#[derive(Debug, Error)]
pub enum EvidenceError {
    #[error("evidence payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    #[error("unsupported evidence bundle schema version {0}")]
    UnsupportedBundleSchema(String),
    #[error("evidence bundle digest mismatch")]
    BundleDigestMismatch,
    #[error("referenced blob {0} is unavailable")]
    BlobUnavailable(Sha256Digest),
    #[error("blob {digest} length mismatch: expected {expected}, found {actual}")]
    BlobLengthMismatch {
        digest: Sha256Digest,
        expected: u64,
        actual: u64,
    },
    #[error("blob digest mismatch: expected {expected}, found {actual}")]
    BlobDigestMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("unknown attestor or key {signer_id}/{key_id}")]
    UnknownAttestor { signer_id: String, key_id: String },
    #[error("attestor key is not active at the attestation time")]
    InactiveKey,
    #[error("unsupported attestation algorithm {0}")]
    UnsupportedAlgorithm(String),
    #[error("attestation signature is malformed")]
    MalformedSignature,
    #[error("attestation signature verification failed")]
    InvalidSignature,
    #[error("attestation quorum contains mixed claim, policy, evidence, or verdict digests")]
    MixedAttestationSemantics,
    #[error("duplicate logical signer {0} cannot count toward quorum")]
    DuplicateSigner(String),
    #[error("attestation quorum requires {required}, only {valid} valid distinct signers found")]
    InsufficientQuorum { required: u16, valid: u16 },
    #[error("operating-system randomness is unavailable")]
    RandomnessUnavailable,
}

/// Build a bundle whose digest excludes only the derived `bundle_digest` field.
///
/// # Errors
///
/// Returns an error when the versioned input cannot be canonicalized.
pub fn build_evidence_bundle(
    input: EvidenceBundleInput,
) -> Result<OutcomeEvidenceBundleV1, EvidenceError> {
    let digest = bundle_digest_from_input(&input)?;
    Ok(OutcomeEvidenceBundleV1 {
        schema_version: "hsai.outcome-evidence-bundle.v1".to_owned(),
        claim_id: input.claim_id,
        claim_spec_digest: input.claim_spec_digest,
        challenge_descriptor_digest: input.challenge_descriptor_digest,
        observation_snapshot_digests: input.observation_snapshot_digests,
        completeness_assessment_digest: input.completeness_assessment_digest,
        qualifying_candidate_digests: input.qualifying_candidate_digests,
        admission_decision_digests: input.admission_decision_digests,
        execution_manifest_digests: input.execution_manifest_digests,
        resolution_aggregation_digest: input.resolution_aggregation_digest,
        dispute_record_digests: input.dispute_record_digests,
        code_build_provenance_digest: input.code_build_provenance_digest,
        evidence_publication_policy_digest: input.evidence_publication_policy_digest,
        blobs: input.blobs,
        bundle_digest: digest,
    })
}

/// Verify bundle identity and every supplied content-addressed blob without a database.
///
/// # Errors
///
/// Returns the first exact missing, length, digest, or bundle identity failure.
pub fn verify_evidence_bundle<S: BuildHasher>(
    bundle: &OutcomeEvidenceBundleV1,
    blobs: &HashMap<Sha256Digest, Vec<u8>, S>,
) -> Result<(), EvidenceError> {
    if bundle.schema_version != "hsai.outcome-evidence-bundle.v1" {
        return Err(EvidenceError::UnsupportedBundleSchema(
            bundle.schema_version.clone(),
        ));
    }
    let input = EvidenceBundleInput::from(bundle);
    if bundle_digest_from_input(&input)? != bundle.bundle_digest {
        return Err(EvidenceError::BundleDigestMismatch);
    }
    for reference in &bundle.blobs {
        let bytes = blobs
            .get(&reference.digest)
            .ok_or(EvidenceError::BlobUnavailable(reference.digest))?;
        let actual_length = bytes.len() as u64;
        if actual_length != reference.byte_length {
            return Err(EvidenceError::BlobLengthMismatch {
                digest: reference.digest,
                expected: reference.byte_length,
                actual: actual_length,
            });
        }
        let actual = plain_digest(bytes);
        if actual != reference.digest {
            return Err(EvidenceError::BlobDigestMismatch {
                expected: reference.digest,
                actual,
            });
        }
    }
    Ok(())
}

impl From<&OutcomeEvidenceBundleV1> for EvidenceBundleInput {
    fn from(bundle: &OutcomeEvidenceBundleV1) -> Self {
        Self {
            claim_id: bundle.claim_id.clone(),
            claim_spec_digest: bundle.claim_spec_digest,
            challenge_descriptor_digest: bundle.challenge_descriptor_digest,
            observation_snapshot_digests: bundle.observation_snapshot_digests.clone(),
            completeness_assessment_digest: bundle.completeness_assessment_digest,
            qualifying_candidate_digests: bundle.qualifying_candidate_digests.clone(),
            admission_decision_digests: bundle.admission_decision_digests.clone(),
            execution_manifest_digests: bundle.execution_manifest_digests.clone(),
            resolution_aggregation_digest: bundle.resolution_aggregation_digest,
            dispute_record_digests: bundle.dispute_record_digests.clone(),
            code_build_provenance_digest: bundle.code_build_provenance_digest,
            evidence_publication_policy_digest: bundle.evidence_publication_policy_digest,
            blobs: bundle.blobs.clone(),
        }
    }
}

#[derive(Serialize)]
struct BundleIdentity<'a> {
    schema_version: &'static str,
    claim_id: &'a ClaimId,
    claim_spec_digest: Sha256Digest,
    challenge_descriptor_digest: Sha256Digest,
    observation_snapshot_digests: &'a [Sha256Digest],
    completeness_assessment_digest: Sha256Digest,
    qualifying_candidate_digests: &'a [Sha256Digest],
    admission_decision_digests: &'a [Sha256Digest],
    execution_manifest_digests: &'a [Sha256Digest],
    resolution_aggregation_digest: Sha256Digest,
    dispute_record_digests: &'a [Sha256Digest],
    code_build_provenance_digest: Sha256Digest,
    evidence_publication_policy_digest: Sha256Digest,
    blobs: &'a [ContentBlobRefV1],
}

fn bundle_digest_from_input(input: &EvidenceBundleInput) -> Result<Sha256Digest, EvidenceError> {
    Ok(domain_separated_digest(
        "hsai.outcome-evidence-bundle.v1",
        &BundleIdentity {
            schema_version: "hsai.outcome-evidence-bundle.v1",
            claim_id: &input.claim_id,
            claim_spec_digest: input.claim_spec_digest,
            challenge_descriptor_digest: input.challenge_descriptor_digest,
            observation_snapshot_digests: &input.observation_snapshot_digests,
            completeness_assessment_digest: input.completeness_assessment_digest,
            qualifying_candidate_digests: &input.qualifying_candidate_digests,
            admission_decision_digests: &input.admission_decision_digests,
            execution_manifest_digests: &input.execution_manifest_digests,
            resolution_aggregation_digest: input.resolution_aggregation_digest,
            dispute_record_digests: &input.dispute_record_digests,
            code_build_provenance_digest: input.code_build_provenance_digest,
            evidence_publication_policy_digest: input.evidence_publication_policy_digest,
            blobs: &input.blobs,
        },
    )?)
}

/// Runtime-generated Ed25519 development signer. Debug output never exposes key bytes.
pub struct DevelopmentSigner {
    signer_id: String,
    key_id: String,
    key: SigningKey,
}

impl fmt::Debug for DevelopmentSigner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DevelopmentSigner")
            .field("signer_id", &self.signer_id)
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

impl DevelopmentSigner {
    /// Generate a new process-local development signer through the OS RNG.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError::RandomnessUnavailable`] if the operating system
    /// cannot supply 32 cryptographically secure random bytes.
    pub fn generate(
        signer_id: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Result<Self, EvidenceError> {
        let mut seed = [0_u8; 32];
        getrandom::fill(&mut seed).map_err(|_| EvidenceError::RandomnessUnavailable)?;
        let key = SigningKey::from_bytes(&seed);
        seed.zeroize();
        Ok(Self {
            signer_id: signer_id.into(),
            key_id: key_id.into(),
            key,
        })
    }

    /// Return the public verification key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    /// Sign a domain-separated canonical resolution attestation.
    ///
    /// # Errors
    ///
    /// Returns an error when the unsigned attestation payload cannot be
    /// canonicalized.
    #[allow(clippy::too_many_arguments)]
    pub fn sign_attestation(
        &self,
        claim_id: ClaimId,
        claim_spec_digest: Sha256Digest,
        truth: ClaimTruth,
        finality: ResolutionFinality,
        resolution_policy_digest: Sha256Digest,
        evidence_bundle_digest: Sha256Digest,
        qualifying_run_digests: Vec<Sha256Digest>,
        completeness_evidence_digest: Option<Sha256Digest>,
        dispute_window_start: UtcTimestamp,
        dispute_window_end: UtcTimestamp,
        issued_at: UtcTimestamp,
        logical_sequence: u64,
    ) -> Result<ResolutionAttestationV1, EvidenceError> {
        self.sign_attestation_superseding(
            claim_id,
            claim_spec_digest,
            truth,
            finality,
            resolution_policy_digest,
            evidence_bundle_digest,
            qualifying_run_digests,
            completeness_evidence_digest,
            dispute_window_start,
            dispute_window_end,
            issued_at,
            logical_sequence,
            None,
        )
    }

    /// Sign an attestation that explicitly replaces a non-settling record.
    ///
    /// # Errors
    ///
    /// Returns an error when the unsigned attestation payload cannot be
    /// canonicalized.
    #[allow(clippy::too_many_arguments)]
    pub fn sign_attestation_superseding(
        &self,
        claim_id: ClaimId,
        claim_spec_digest: Sha256Digest,
        truth: ClaimTruth,
        finality: ResolutionFinality,
        resolution_policy_digest: Sha256Digest,
        evidence_bundle_digest: Sha256Digest,
        qualifying_run_digests: Vec<Sha256Digest>,
        completeness_evidence_digest: Option<Sha256Digest>,
        dispute_window_start: UtcTimestamp,
        dispute_window_end: UtcTimestamp,
        issued_at: UtcTimestamp,
        logical_sequence: u64,
        supersedes_non_settling_record: Option<Sha256Digest>,
    ) -> Result<ResolutionAttestationV1, EvidenceError> {
        let payload = AttestationPayload {
            schema_version: "hsai.resolution-attestation.v1",
            claim_id: &claim_id,
            claim_spec_digest,
            truth,
            finality,
            resolution_policy_digest,
            evidence_bundle_digest,
            qualifying_run_digests: &qualifying_run_digests,
            completeness_evidence_digest,
            dispute_window_start: &dispute_window_start,
            dispute_window_end: &dispute_window_end,
            issued_at: &issued_at,
            logical_sequence,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: "Ed25519",
            supersedes_non_settling_record,
        };
        let message = attestation_signing_message(&payload)?;
        let signature = self.key.sign(&message);
        Ok(ResolutionAttestationV1 {
            schema_version: payload.schema_version.to_owned(),
            claim_id,
            claim_spec_digest,
            truth,
            finality,
            resolution_policy_digest,
            evidence_bundle_digest,
            qualifying_run_digests,
            completeness_evidence_digest,
            dispute_window_start,
            dispute_window_end,
            issued_at,
            logical_sequence,
            signer_id: self.signer_id.clone(),
            key_id: self.key_id.clone(),
            algorithm: "Ed25519".to_owned(),
            signature: STANDARD.encode(signature.to_bytes()),
            supersedes_non_settling_record,
        })
    }
}

/// Registered public attestor key with activation and optional revocation bounds.
#[derive(Debug, Clone)]
pub struct RegisteredAttestor {
    pub signer_id: String,
    pub key_id: String,
    pub verifying_key: VerifyingKey,
    pub active_from: UtcTimestamp,
    pub revoked_at: Option<UtcTimestamp>,
}

/// Public-key registry used for standalone attestation verification.
#[derive(Debug, Clone, Default)]
pub struct AttestorRegistry {
    entries: BTreeMap<(String, String), RegisteredAttestor>,
}

impl AttestorRegistry {
    /// Register a versioned public verification key.
    pub fn register(&mut self, attestor: RegisteredAttestor) {
        self.entries.insert(
            (attestor.signer_id.clone(), attestor.key_id.clone()),
            attestor,
        );
    }

    /// Verify one attestation's key validity and Ed25519 signature.
    ///
    /// # Errors
    ///
    /// Returns a typed unknown-key, validity-window, algorithm, encoding, or
    /// signature failure.
    pub fn verify(&self, attestation: &ResolutionAttestationV1) -> Result<(), EvidenceError> {
        if attestation.algorithm != "Ed25519" {
            return Err(EvidenceError::UnsupportedAlgorithm(
                attestation.algorithm.clone(),
            ));
        }
        let registered = self
            .entries
            .get(&(attestation.signer_id.clone(), attestation.key_id.clone()))
            .ok_or_else(|| EvidenceError::UnknownAttestor {
                signer_id: attestation.signer_id.clone(),
                key_id: attestation.key_id.clone(),
            })?;
        if attestation.issued_at < registered.active_from
            || registered
                .revoked_at
                .as_ref()
                .is_some_and(|revoked| &attestation.issued_at >= revoked)
        {
            return Err(EvidenceError::InactiveKey);
        }
        let decoded = STANDARD
            .decode(&attestation.signature)
            .map_err(|_| EvidenceError::MalformedSignature)?;
        let signature =
            Signature::from_slice(&decoded).map_err(|_| EvidenceError::MalformedSignature)?;
        let payload = AttestationPayload::from(attestation);
        let message = attestation_signing_message(&payload)?;
        registered
            .verifying_key
            .verify(&message, &signature)
            .map_err(|_| EvidenceError::InvalidSignature)
    }
}

/// Verify a homogeneous threshold of distinct valid logical signers.
///
/// # Errors
///
/// Returns signature/key failures, duplicate signers, mixed semantic payloads,
/// or insufficient distinct quorum.
pub fn verify_quorum(
    attestations: &[ResolutionAttestationV1],
    registry: &AttestorRegistry,
    required: u16,
) -> Result<(), EvidenceError> {
    let Some(first) = attestations.first() else {
        return Err(EvidenceError::InsufficientQuorum { required, valid: 0 });
    };
    let semantics = attestation_semantics(first);
    let mut signers = BTreeSet::new();
    for attestation in attestations {
        if attestation_semantics(attestation) != semantics {
            return Err(EvidenceError::MixedAttestationSemantics);
        }
        if !signers.insert(attestation.signer_id.clone()) {
            return Err(EvidenceError::DuplicateSigner(
                attestation.signer_id.clone(),
            ));
        }
        registry.verify(attestation)?;
    }
    let valid = u16::try_from(signers.len()).unwrap_or(u16::MAX);
    if valid < required {
        return Err(EvidenceError::InsufficientQuorum { required, valid });
    }
    Ok(())
}

#[derive(Serialize)]
struct AttestationPayload<'a> {
    schema_version: &'a str,
    claim_id: &'a ClaimId,
    claim_spec_digest: Sha256Digest,
    truth: ClaimTruth,
    finality: ResolutionFinality,
    resolution_policy_digest: Sha256Digest,
    evidence_bundle_digest: Sha256Digest,
    qualifying_run_digests: &'a [Sha256Digest],
    completeness_evidence_digest: Option<Sha256Digest>,
    dispute_window_start: &'a UtcTimestamp,
    dispute_window_end: &'a UtcTimestamp,
    issued_at: &'a UtcTimestamp,
    logical_sequence: u64,
    signer_id: &'a str,
    key_id: &'a str,
    algorithm: &'a str,
    supersedes_non_settling_record: Option<Sha256Digest>,
}

impl<'a> From<&'a ResolutionAttestationV1> for AttestationPayload<'a> {
    fn from(attestation: &'a ResolutionAttestationV1) -> Self {
        Self {
            schema_version: &attestation.schema_version,
            claim_id: &attestation.claim_id,
            claim_spec_digest: attestation.claim_spec_digest,
            truth: attestation.truth,
            finality: attestation.finality,
            resolution_policy_digest: attestation.resolution_policy_digest,
            evidence_bundle_digest: attestation.evidence_bundle_digest,
            qualifying_run_digests: &attestation.qualifying_run_digests,
            completeness_evidence_digest: attestation.completeness_evidence_digest,
            dispute_window_start: &attestation.dispute_window_start,
            dispute_window_end: &attestation.dispute_window_end,
            issued_at: &attestation.issued_at,
            logical_sequence: attestation.logical_sequence,
            signer_id: &attestation.signer_id,
            key_id: &attestation.key_id,
            algorithm: &attestation.algorithm,
            supersedes_non_settling_record: attestation.supersedes_non_settling_record,
        }
    }
}

fn attestation_signing_message(payload: &AttestationPayload<'_>) -> Result<Vec<u8>, EvidenceError> {
    let canonical = canonical_json(payload)?;
    let mut message = b"hsai.resolution-attestation.signature.v1\0".to_vec();
    message.extend_from_slice(&canonical);
    Ok(message)
}

fn attestation_semantics(
    value: &ResolutionAttestationV1,
) -> (
    &ClaimId,
    Sha256Digest,
    ClaimTruth,
    ResolutionFinality,
    Sha256Digest,
    Sha256Digest,
) {
    (
        &value.claim_id,
        value.claim_spec_digest,
        value.truth,
        value.finality,
        value.resolution_policy_digest,
        value.evidence_bundle_digest,
    )
}

fn plain_digest(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use outcomes_domain::{EvidenceClassification, fixtures::local_demo_claim_draft};

    use super::*;

    type TestBundle = (OutcomeEvidenceBundleV1, HashMap<Sha256Digest, Vec<u8>>);

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn test_bundle() -> Result<TestBundle, Box<dyn std::error::Error>> {
        let bytes = b"portable candidate".to_vec();
        let blob_digest = plain_digest(&bytes);
        let blob = ContentBlobRefV1 {
            digest_algorithm: "sha256".to_owned(),
            digest: blob_digest,
            byte_length: bytes.len() as u64,
            media_type: "application/octet-stream".to_owned(),
            classification: EvidenceClassification::Public,
            storage_locator: format!("cas://sha256/{blob_digest}"),
        };
        let claim = local_demo_claim_draft()?.freeze()?;
        let bundle = build_evidence_bundle(EvidenceBundleInput {
            claim_id: claim.claim_id.clone(),
            claim_spec_digest: claim.claim_spec_digest()?,
            challenge_descriptor_digest: claim.challenge_descriptor_digest,
            observation_snapshot_digests: vec![Sha256Digest::from_bytes([4; 32])],
            completeness_assessment_digest: Sha256Digest::from_bytes([5; 32]),
            qualifying_candidate_digests: vec![blob_digest],
            admission_decision_digests: vec![Sha256Digest::from_bytes([6; 32])],
            execution_manifest_digests: vec![Sha256Digest::from_bytes([7; 32])],
            resolution_aggregation_digest: Sha256Digest::from_bytes([8; 32]),
            dispute_record_digests: Vec::new(),
            code_build_provenance_digest: Sha256Digest::from_bytes([9; 32]),
            evidence_publication_policy_digest: Sha256Digest::from_bytes([10; 32]),
            blobs: vec![blob],
        })?;
        Ok((bundle, HashMap::from([(blob_digest, bytes)])))
    }

    #[test]
    fn contract_bundle_verifies_without_database() -> Result<(), Box<dyn std::error::Error>> {
        let (bundle, blobs) = test_bundle()?;
        verify_evidence_bundle(&bundle, &blobs)?;
        Ok(())
    }

    #[test]
    fn contract_one_byte_tamper_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let (bundle, mut blobs) = test_bundle()?;
        if let Some(bytes) = blobs.values_mut().next() {
            bytes[0] ^= 1;
        }
        assert!(matches!(
            verify_evidence_bundle(&bundle, &blobs),
            Err(EvidenceError::BlobDigestMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn contract_bundle_schema_tamper_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let (mut bundle, blobs) = test_bundle()?;
        bundle.schema_version = "hsai.outcome-evidence-bundle.v2".to_owned();
        assert!(matches!(
            verify_evidence_bundle(&bundle, &blobs),
            Err(EvidenceError::UnsupportedBundleSchema(version))
                if version == "hsai.outcome-evidence-bundle.v2"
        ));
        Ok(())
    }

    #[test]
    fn contract_two_of_three_distinct_attestors_verify() -> Result<(), Box<dyn std::error::Error>> {
        let (bundle, _) = test_bundle()?;
        let claim = local_demo_claim_draft()?.freeze()?;
        let issued_at = timestamp("2026-07-10T12:10:00Z")?;
        let start = timestamp("2026-07-10T12:00:00Z")?;
        let end = timestamp("2026-07-10T12:20:00Z")?;
        let policy_digest = domain_separated_digest("hsai.test-policy.v1", &"policy")?;
        let signer_a = DevelopmentSigner::generate("attestor-a", "key-a")?;
        let signer_b = DevelopmentSigner::generate("attestor-b", "key-b")?;
        let mut registry = AttestorRegistry::default();
        for (signer, signer_id, key_id) in [
            (&signer_a, "attestor-a", "key-a"),
            (&signer_b, "attestor-b", "key-b"),
        ] {
            registry.register(RegisteredAttestor {
                signer_id: signer_id.to_owned(),
                key_id: key_id.to_owned(),
                verifying_key: signer.verifying_key(),
                active_from: timestamp("2026-07-10T00:00:00Z")?,
                revoked_at: None,
            });
        }
        let mut attestations = Vec::new();
        for signer in [&signer_a, &signer_b] {
            attestations.push(signer.sign_attestation(
                claim.claim_id.clone(),
                claim.claim_spec_digest()?,
                ClaimTruth::True,
                ResolutionFinality::Provisional,
                policy_digest,
                bundle.bundle_digest,
                vec![Sha256Digest::from_bytes([7; 32])],
                None,
                start.clone(),
                end.clone(),
                issued_at.clone(),
                1,
            )?);
        }
        verify_quorum(&attestations, &registry, 2)?;
        Ok(())
    }

    #[test]
    fn contract_duplicate_signer_does_not_count_twice() -> Result<(), Box<dyn std::error::Error>> {
        let (bundle, _) = test_bundle()?;
        let claim = local_demo_claim_draft()?.freeze()?;
        let signer = DevelopmentSigner::generate("attestor-a", "key-a")?;
        let issued_at = timestamp("2026-07-10T12:10:00Z")?;
        let mut registry = AttestorRegistry::default();
        registry.register(RegisteredAttestor {
            signer_id: "attestor-a".to_owned(),
            key_id: "key-a".to_owned(),
            verifying_key: signer.verifying_key(),
            active_from: timestamp("2026-07-10T00:00:00Z")?,
            revoked_at: None,
        });
        let attestation = signer.sign_attestation(
            claim.claim_id.clone(),
            claim.claim_spec_digest()?,
            ClaimTruth::True,
            ResolutionFinality::Provisional,
            Sha256Digest::from_bytes([11; 32]),
            bundle.bundle_digest,
            Vec::new(),
            None,
            timestamp("2026-07-10T12:00:00Z")?,
            timestamp("2026-07-10T12:20:00Z")?,
            issued_at,
            1,
        )?;
        assert!(matches!(
            verify_quorum(&[attestation.clone(), attestation], &registry, 2),
            Err(EvidenceError::DuplicateSigner(_))
        ));
        Ok(())
    }

    #[test]
    fn contract_supersession_digest_is_signed() -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let signer = DevelopmentSigner::generate("attestor-a", "key-a")?;
        let mut registry = AttestorRegistry::default();
        registry.register(RegisteredAttestor {
            signer_id: "attestor-a".to_owned(),
            key_id: "key-a".to_owned(),
            verifying_key: signer.verifying_key(),
            active_from: timestamp("2026-07-10T00:00:00Z")?,
            revoked_at: None,
        });
        let predecessor = Sha256Digest::from_bytes([90; 32]);
        let mut attestation = signer.sign_attestation_superseding(
            claim.claim_id.clone(),
            claim.claim_spec_digest()?,
            ClaimTruth::True,
            ResolutionFinality::Provisional,
            Sha256Digest::from_bytes([91; 32]),
            Sha256Digest::from_bytes([92; 32]),
            vec![Sha256Digest::from_bytes([93; 32])],
            None,
            timestamp("2026-07-10T12:00:00Z")?,
            timestamp("2026-07-10T12:20:00Z")?,
            timestamp("2026-07-10T12:10:00Z")?,
            2,
            Some(predecessor),
        )?;
        verify_quorum(std::slice::from_ref(&attestation), &registry, 1)?;
        attestation.supersedes_non_settling_record = Some(Sha256Digest::from_bytes([94; 32]));
        assert!(matches!(
            verify_quorum(&[attestation], &registry, 1),
            Err(EvidenceError::InvalidSignature)
        ));
        Ok(())
    }
}
