use std::{collections::BTreeMap, fmt};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signature, VerifyingKey};
use hyperintel_core::{
    InferenceCapability, ProtocolId, Sha256Digest, UtcTimestamp, canonical_json,
    domain_separated_digest,
};
use serde::{Deserialize, Serialize};

use crate::{
    ED25519_ALGORITHM_V1, MAX_READINESS_ASSERTION_LIFETIME_SECONDS,
    MAX_REVOCATION_SNAPSHOT_LIFETIME_SECONDS, MAX_SERVICE_MANIFEST_LIFETIME_SECONDS,
    PROVIDER_AGENT_KEY_SCHEMA_V1, PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1,
    PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1, PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1,
    ProviderAgentError, SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1,
    SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1,
    SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1,
};

const MANIFEST_STATEMENT_DOMAIN_V1: &str = "hyperintel.provider-agent-manifest-statement.v1";
const MANIFEST_DIGEST_DOMAIN_V1: &str = "hyperintel.provider-agent-manifest-digest.v1";
const READINESS_STATEMENT_DOMAIN_V1: &str = "hyperintel.provider-agent-readiness-statement.v1";
const READINESS_DIGEST_DOMAIN_V1: &str = "hyperintel.provider-agent-readiness-digest.v1";
const REVOCATION_STATEMENT_DOMAIN_V1: &str = "hyperintel.provider-agent-revocation-statement.v1";
const REVOCATION_DIGEST_DOMAIN_V1: &str = "hyperintel.provider-agent-revocation-digest.v1";
const KEY_DIGEST_DOMAIN_V1: &str = "hyperintel.provider-agent-key-digest.v1";
const CHECKPOINT_DIGEST_DOMAIN_V1: &str = "hyperintel.provider-agent-revocation-checkpoint.v1";
const BINDING_DIGEST_DOMAIN_V1: &str = "hyperintel.provider-agent-binding.v1";

/// Dedicated purpose assigned to one provider-agent verification key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderAgentKeyPurposeV1 {
    /// Verify signed service manifests only.
    ServiceManifest,
    /// Verify signed readiness assertions only.
    ReadinessAssertion,
    /// Verify signed revocation snapshots only.
    RevocationSnapshot,
}

/// Closed local request/stream interface admitted by V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderTextStreamInterfaceV1 {
    /// Existing text-only OpenAI-shaped streaming harness interface.
    OpenAiTextStreamingChatCompletionsV1,
}

/// Provider-asserted readiness state; this is not an observed health signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderAgentReadinessStatusV1 {
    /// Provider asserts this exact instance is ready for the bound service.
    Ready,
    /// Provider asserts the instance is draining and must not receive new work.
    Draining,
    /// Provider asserts the instance is unavailable.
    Unavailable,
}

/// Exact nonsecret provider-agent deployment binding.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAgentBindingV1 {
    /// Selected provider.
    pub provider_id: ProtocolId,
    /// Exact agent process or workload identity asserted by the provider.
    pub agent_instance_id: ProtocolId,
    /// Selected service specification.
    pub service_spec_digest: Sha256Digest,
    /// Selected signed capacity-offer identity.
    pub selected_offer_digest: Sha256Digest,
    /// Validated runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Exact model artifact.
    pub model_artifact_digest: Sha256Digest,
    /// Exact tokenizer artifact.
    pub tokenizer_digest: Sha256Digest,
    /// Exact runtime image.
    pub runtime_image_digest: Sha256Digest,
    /// Strictly sorted closed capability set.
    pub capabilities: Vec<InferenceCapability>,
    /// Configured receipt-signing key identity; never key material.
    pub receipt_key_id: ProtocolId,
    /// Exact text-stream interface.
    pub interface: ProviderTextStreamInterfaceV1,
    /// Immutable secret-reference identity; never secret material.
    pub secret_reference_digest: Sha256Digest,
}

impl fmt::Debug for ProviderAgentBindingV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAgentBindingV1")
            .field("provider_id", &self.provider_id)
            .field("agent_instance_id", &self.agent_instance_id)
            .field("service_spec_digest", &"[REDACTED]")
            .field("selected_offer_digest", &"[REDACTED]")
            .field("runtime_configuration_digest", &"[REDACTED]")
            .field("artifacts", &"[REDACTED]")
            .field("capabilities", &self.capabilities)
            .field("receipt_key_id", &"[REDACTED]")
            .field("interface", &self.interface)
            .field("secret_reference_digest", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl ProviderAgentBindingV1 {
    /// Return the complete nonsecret operational-binding identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, ProviderAgentError> {
        Ok(domain_separated_digest(BINDING_DIGEST_DOMAIN_V1, self)?)
    }
}

/// One caller-trusted provider-agent verification key.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredProviderAgentKeyV1 {
    /// Fixed key schema.
    pub schema_version: String,
    /// Provider trust namespace.
    pub provider_id: ProtocolId,
    /// Exact key identity.
    pub key_id: ProtocolId,
    /// Dedicated key purpose.
    pub purpose: ProviderAgentKeyPurposeV1,
    /// Lowercase hexadecimal Ed25519 public key.
    pub public_key_hex: String,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

impl fmt::Debug for RegisteredProviderAgentKeyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredProviderAgentKeyV1")
            .field("provider_id", &self.provider_id)
            .field("key_id", &self.key_id)
            .field("purpose", &self.purpose)
            .field("public_key_hex", &"[REDACTED]")
            .field("valid_from", &self.valid_from)
            .field("valid_until", &self.valid_until)
            .field("revoked_at", &self.revoked_at)
            .finish_non_exhaustive()
    }
}

impl RegisteredProviderAgentKeyV1 {
    /// Return the caller-trusted key-record identity without exposing key bytes.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, ProviderAgentError> {
        Ok(domain_separated_digest(KEY_DIGEST_DOMAIN_V1, self)?)
    }
}

/// Caller-owned purpose-separated provider-agent trust registry.
#[derive(Clone, Debug, Default)]
pub struct ProviderAgentKeyRegistryV1 {
    keys:
        BTreeMap<(ProtocolId, ProtocolId, ProviderAgentKeyPurposeV1), RegisteredProviderAgentKeyV1>,
}

impl ProviderAgentKeyRegistryV1 {
    /// Register one exact key without overwrite or purpose fallback.
    ///
    /// # Errors
    ///
    /// Returns for schema, interval, encoding, or duplicate failures.
    pub fn register(
        &mut self,
        key: RegisteredProviderAgentKeyV1,
    ) -> Result<(), ProviderAgentError> {
        if key.schema_version != PROVIDER_AGENT_KEY_SCHEMA_V1 {
            return Err(ProviderAgentError::UnsupportedSchema);
        }
        validate_interval(&key.valid_from, &key.valid_until, None)?;
        if key
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| revoked_at < &key.valid_from)
        {
            return Err(ProviderAgentError::InvalidValidity);
        }
        decode_verifying_key(&key.public_key_hex)?;
        let identity = (key.provider_id.clone(), key.key_id.clone(), key.purpose);
        if self.keys.contains_key(&identity) {
            return Err(ProviderAgentError::DuplicateIdentity);
        }
        self.keys.insert(identity, key);
        Ok(())
    }

    fn get(
        &self,
        provider_id: &ProtocolId,
        key_id: &ProtocolId,
        purpose: ProviderAgentKeyPurposeV1,
    ) -> Option<&RegisteredProviderAgentKeyV1> {
        self.keys
            .get(&(provider_id.clone(), key_id.clone(), purpose))
    }
}

/// Exact signed service-manifest claims.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAgentServiceManifestClaimsV1 {
    /// Fixed claims schema.
    pub schema_version: String,
    /// Nonzero logical manifest sequence.
    pub sequence: u64,
    /// Complete provider-agent binding.
    pub binding: ProviderAgentBindingV1,
    /// Issuance time.
    pub issued_at: UtcTimestamp,
    /// Expiration time.
    pub expires_at: UtcTimestamp,
}

/// Signed provider-agent service manifest.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProviderAgentServiceManifestV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact signed claims.
    pub claims: ProviderAgentServiceManifestClaimsV1,
    /// Must equal the binding provider.
    pub signer_id: ProtocolId,
    /// Dedicated manifest key.
    pub key_id: ProtocolId,
    /// Fixed Ed25519 label.
    pub algorithm: String,
    /// Lowercase hexadecimal signature.
    pub signature_hex: String,
}

impl fmt::Debug for SignedProviderAgentServiceManifestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedProviderAgentServiceManifestV1")
            .field("provider_id", &self.claims.binding.provider_id)
            .field("agent_instance_id", &self.claims.binding.agent_instance_id)
            .field("binding", &"[REDACTED]")
            .field("key_id", &self.key_id)
            .field("signature_hex", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Exact short-lived readiness assertion claims.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAgentReadinessClaimsV1 {
    /// Fixed claims schema.
    pub schema_version: String,
    /// Nonzero logical readiness sequence.
    pub sequence: u64,
    /// Complete provider-agent binding.
    pub binding: ProviderAgentBindingV1,
    /// Complete signed service-manifest identity.
    pub service_manifest_digest: Sha256Digest,
    /// Provider assertion evaluated fail-closed; only `Ready` is admitted.
    pub status: ProviderAgentReadinessStatusV1,
    /// Issuance time.
    pub issued_at: UtcTimestamp,
    /// Expiration time.
    pub expires_at: UtcTimestamp,
}

/// Signed provider/operator readiness assertion.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProviderAgentReadinessV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact signed claims.
    pub claims: ProviderAgentReadinessClaimsV1,
    /// Must equal the binding provider.
    pub signer_id: ProtocolId,
    /// Dedicated readiness key.
    pub key_id: ProtocolId,
    /// Fixed Ed25519 label.
    pub algorithm: String,
    /// Lowercase hexadecimal signature.
    pub signature_hex: String,
}

impl fmt::Debug for SignedProviderAgentReadinessV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedProviderAgentReadinessV1")
            .field("provider_id", &self.claims.binding.provider_id)
            .field("agent_instance_id", &self.claims.binding.agent_instance_id)
            .field("binding", &"[REDACTED]")
            .field("service_manifest_digest", &"[REDACTED]")
            .field("key_id", &self.key_id)
            .field("signature_hex", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Revocable signed artifact kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderAgentArtifactKindV1 {
    /// Complete signed manifest digest.
    ServiceManifest,
    /// Complete signed readiness digest.
    ReadinessAssertion,
}

/// Inclusive revocation of one exact signed control artifact.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAgentAssertionRevocationV1 {
    /// Provider namespace.
    pub provider_id: ProtocolId,
    /// Exact artifact kind.
    pub artifact_kind: ProviderAgentArtifactKindV1,
    /// Complete signed artifact digest.
    pub artifact_digest: Sha256Digest,
    /// Inclusive revocation instant.
    pub revoked_at: UtcTimestamp,
}

/// Inclusive revocation of one provider-agent key.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAgentKeyRevocationV1 {
    /// Provider namespace.
    pub provider_id: ProtocolId,
    /// Exact key identity.
    pub key_id: ProtocolId,
    /// Dedicated key purpose.
    pub purpose: ProviderAgentKeyPurposeV1,
    /// Inclusive revocation instant.
    pub revoked_at: UtcTimestamp,
}

/// Signed freshness-bounded provider-agent revocation state.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAgentRevocationSnapshotV1 {
    /// Fixed snapshot schema.
    pub schema_version: String,
    /// Provider namespace.
    pub provider_id: ProtocolId,
    /// Exact runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Nonzero monotonic sequence.
    pub sequence: u64,
    /// Snapshot issuance time.
    pub issued_at: UtcTimestamp,
    /// First represented evaluation instant.
    pub effective_at: UtcTimestamp,
    /// Last represented evaluation instant.
    pub expires_at: UtcTimestamp,
    /// Strictly sorted assertion revocations.
    pub assertion_revocations: Vec<ProviderAgentAssertionRevocationV1>,
    /// Strictly sorted key revocations.
    pub key_revocations: Vec<ProviderAgentKeyRevocationV1>,
}

impl fmt::Debug for ProviderAgentRevocationSnapshotV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAgentRevocationSnapshotV1")
            .field("provider_id", &self.provider_id)
            .field("sequence", &self.sequence)
            .field("revocations", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Signed provider-agent revocation snapshot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProviderAgentRevocationSnapshotV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact signed snapshot.
    pub snapshot: ProviderAgentRevocationSnapshotV1,
    /// Must equal the snapshot provider.
    pub signer_id: ProtocolId,
    /// Dedicated revocation key.
    pub key_id: ProtocolId,
    /// Fixed Ed25519 label.
    pub algorithm: String,
    /// Lowercase hexadecimal signature.
    pub signature_hex: String,
}

impl fmt::Debug for SignedProviderAgentRevocationSnapshotV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedProviderAgentRevocationSnapshotV1")
            .field("provider_id", &self.snapshot.provider_id)
            .field("sequence", &self.snapshot.sequence)
            .field("revocations", &"[REDACTED]")
            .field("key_id", &self.key_id)
            .field("signature_hex", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Caller-pinned rollback and same-sequence-fork checkpoint.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ProviderAgentRevocationCheckpointV1 {
    /// Exact provider namespace.
    pub provider_id: ProtocolId,
    /// Exact runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Exact asserted provider-agent instance.
    pub agent_instance_id: ProtocolId,
    /// Complete expected provider-agent binding identity.
    pub binding_digest: Sha256Digest,
    /// Minimum accepted manifest sequence.
    pub manifest_minimum_sequence: u64,
    /// Exact manifest digest required at its minimum sequence.
    pub manifest_digest_at_minimum_sequence: Sha256Digest,
    /// Minimum accepted readiness sequence.
    pub readiness_minimum_sequence: u64,
    /// Exact readiness digest required at its minimum sequence.
    pub readiness_digest_at_minimum_sequence: Sha256Digest,
    /// Minimum accepted revocation sequence.
    pub revocation_minimum_sequence: u64,
    /// Exact revocation digest required at its minimum sequence.
    pub revocation_digest_at_minimum_sequence: Sha256Digest,
}

impl fmt::Debug for ProviderAgentRevocationCheckpointV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAgentRevocationCheckpointV1")
            .field("provider_id", &self.provider_id)
            .field("sequences", &"[REDACTED]")
            .field("binding", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl ProviderAgentRevocationCheckpointV1 {
    /// Return the exact caller-pinned checkpoint identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, ProviderAgentError> {
        Ok(domain_separated_digest(CHECKPOINT_DIGEST_DOMAIN_V1, self)?)
    }
}

#[derive(Serialize)]
struct SignedMessage<'a, T> {
    statement_domain: &'static str,
    wrapper_schema: &'a str,
    body: &'a T,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

macro_rules! signed_methods {
    ($type:ty, $body:ident, $statement:expr, $digest:expr) => {
        impl $type {
            /// Return canonical domain-bound signing bytes.
            ///
            /// # Errors
            ///
            /// Returns when canonicalization fails.
            pub fn signing_message(&self) -> Result<Vec<u8>, ProviderAgentError> {
                Ok(canonical_json(&SignedMessage {
                    statement_domain: $statement,
                    wrapper_schema: &self.schema_version,
                    body: &self.$body,
                    signer_id: &self.signer_id,
                    key_id: &self.key_id,
                    algorithm: &self.algorithm,
                })?)
            }

            /// Return the complete signed artifact identity.
            ///
            /// # Errors
            ///
            /// Returns when canonicalization fails.
            pub fn digest(&self) -> Result<Sha256Digest, ProviderAgentError> {
                Ok(domain_separated_digest($digest, self)?)
            }
        }
    };
}

signed_methods!(
    SignedProviderAgentServiceManifestV1,
    claims,
    MANIFEST_STATEMENT_DOMAIN_V1,
    MANIFEST_DIGEST_DOMAIN_V1
);
signed_methods!(
    SignedProviderAgentReadinessV1,
    claims,
    READINESS_STATEMENT_DOMAIN_V1,
    READINESS_DIGEST_DOMAIN_V1
);
signed_methods!(
    SignedProviderAgentRevocationSnapshotV1,
    snapshot,
    REVOCATION_STATEMENT_DOMAIN_V1,
    REVOCATION_DIGEST_DOMAIN_V1
);

pub(crate) struct VerifiedProviderAgentControlV1 {
    pub(crate) manifest: Sha256Digest,
    pub(crate) readiness: Sha256Digest,
    pub(crate) revocation: Sha256Digest,
    pub(crate) manifest_key: Sha256Digest,
    pub(crate) readiness_key: Sha256Digest,
    pub(crate) revocation_key: Sha256Digest,
}

pub(crate) fn verify_provider_agent_control_v1(
    manifest: &SignedProviderAgentServiceManifestV1,
    readiness: &SignedProviderAgentReadinessV1,
    revocation: &SignedProviderAgentRevocationSnapshotV1,
    checkpoint: &ProviderAgentRevocationCheckpointV1,
    registry: &ProviderAgentKeyRegistryV1,
    expected: &ProviderAgentBindingV1,
    evaluated_at: &UtcTimestamp,
) -> Result<VerifiedProviderAgentControlV1, ProviderAgentError> {
    validate_binding(expected)?;
    if checkpoint.provider_id != expected.provider_id
        || checkpoint.runtime_configuration_digest != expected.runtime_configuration_digest
        || checkpoint.agent_instance_id != expected.agent_instance_id
        || checkpoint.binding_digest != expected.digest()?
    {
        return Err(ProviderAgentError::ControlDenied);
    }
    let (manifest_digest, manifest_key_digest) =
        verify_manifest(manifest, registry, expected, evaluated_at)?;
    let (readiness_digest, readiness_key_digest) = verify_readiness(
        readiness,
        registry,
        expected,
        manifest_digest,
        &manifest.claims,
        evaluated_at,
    )?;
    let (revocation_digest, revocation_key_digest) =
        verify_revocation_snapshot(revocation, checkpoint, registry, expected, evaluated_at)?;
    verify_checkpoint_artifact(
        manifest.claims.sequence,
        manifest_digest,
        checkpoint.manifest_minimum_sequence,
        checkpoint.manifest_digest_at_minimum_sequence,
    )?;
    verify_checkpoint_artifact(
        readiness.claims.sequence,
        readiness_digest,
        checkpoint.readiness_minimum_sequence,
        checkpoint.readiness_digest_at_minimum_sequence,
    )?;
    verify_not_revoked(
        manifest,
        readiness,
        revocation,
        manifest_digest,
        readiness_digest,
        evaluated_at,
    )?;
    Ok(VerifiedProviderAgentControlV1 {
        manifest: manifest_digest,
        readiness: readiness_digest,
        revocation: revocation_digest,
        manifest_key: manifest_key_digest,
        readiness_key: readiness_key_digest,
        revocation_key: revocation_key_digest,
    })
}

fn verify_manifest(
    signed: &SignedProviderAgentServiceManifestV1,
    registry: &ProviderAgentKeyRegistryV1,
    expected: &ProviderAgentBindingV1,
    evaluated_at: &UtcTimestamp,
) -> Result<(Sha256Digest, Sha256Digest), ProviderAgentError> {
    if signed.schema_version != SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1
        || signed.claims.schema_version != PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1
    {
        return Err(ProviderAgentError::UnsupportedSchema);
    }
    validate_signed_header(
        &signed.algorithm,
        &signed.signer_id,
        &signed.claims.binding.provider_id,
    )?;
    if signed.claims.sequence == 0 || signed.claims.binding != *expected {
        return Err(ProviderAgentError::ControlDenied);
    }
    validate_interval(
        &signed.claims.issued_at,
        &signed.claims.expires_at,
        Some(MAX_SERVICE_MANIFEST_LIFETIME_SECONDS),
    )?;
    validate_evaluated_at(
        &signed.claims.issued_at,
        &signed.claims.expires_at,
        evaluated_at,
    )?;
    let key_digest = verify_keyed_signature(KeyedSignatureInput {
        registry,
        provider_id: &signed.signer_id,
        key_id: &signed.key_id,
        purpose: ProviderAgentKeyPurposeV1::ServiceManifest,
        issued_at: &signed.claims.issued_at,
        expires_at: &signed.claims.expires_at,
        evaluated_at,
        signature_hex: &signed.signature_hex,
        message: &signed.signing_message()?,
    })?;
    Ok((signed.digest()?, key_digest))
}

fn verify_readiness(
    signed: &SignedProviderAgentReadinessV1,
    registry: &ProviderAgentKeyRegistryV1,
    expected: &ProviderAgentBindingV1,
    manifest_digest: Sha256Digest,
    manifest_claims: &ProviderAgentServiceManifestClaimsV1,
    evaluated_at: &UtcTimestamp,
) -> Result<(Sha256Digest, Sha256Digest), ProviderAgentError> {
    if signed.schema_version != SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1
        || signed.claims.schema_version != PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1
    {
        return Err(ProviderAgentError::UnsupportedSchema);
    }
    validate_signed_header(
        &signed.algorithm,
        &signed.signer_id,
        &signed.claims.binding.provider_id,
    )?;
    if signed.claims.sequence == 0
        || signed.claims.binding != *expected
        || signed.claims.service_manifest_digest != manifest_digest
        || signed.claims.status != ProviderAgentReadinessStatusV1::Ready
    {
        return Err(ProviderAgentError::ControlDenied);
    }
    validate_interval(
        &signed.claims.issued_at,
        &signed.claims.expires_at,
        Some(MAX_READINESS_ASSERTION_LIFETIME_SECONDS),
    )?;
    if signed.claims.issued_at < manifest_claims.issued_at
        || signed.claims.expires_at > manifest_claims.expires_at
    {
        return Err(ProviderAgentError::InvalidValidity);
    }
    validate_evaluated_at(
        &signed.claims.issued_at,
        &signed.claims.expires_at,
        evaluated_at,
    )?;
    let key_digest = verify_keyed_signature(KeyedSignatureInput {
        registry,
        provider_id: &signed.signer_id,
        key_id: &signed.key_id,
        purpose: ProviderAgentKeyPurposeV1::ReadinessAssertion,
        issued_at: &signed.claims.issued_at,
        expires_at: &signed.claims.expires_at,
        evaluated_at,
        signature_hex: &signed.signature_hex,
        message: &signed.signing_message()?,
    })?;
    Ok((signed.digest()?, key_digest))
}

fn verify_revocation_snapshot(
    signed: &SignedProviderAgentRevocationSnapshotV1,
    checkpoint: &ProviderAgentRevocationCheckpointV1,
    registry: &ProviderAgentKeyRegistryV1,
    expected: &ProviderAgentBindingV1,
    evaluated_at: &UtcTimestamp,
) -> Result<(Sha256Digest, Sha256Digest), ProviderAgentError> {
    if signed.schema_version != SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1
        || signed.snapshot.schema_version != PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1
    {
        return Err(ProviderAgentError::UnsupportedSchema);
    }
    validate_signed_header(
        &signed.algorithm,
        &signed.signer_id,
        &signed.snapshot.provider_id,
    )?;
    let snapshot = &signed.snapshot;
    if snapshot.sequence == 0
        || snapshot.runtime_configuration_digest.is_zero()
        || snapshot.provider_id != expected.provider_id
        || snapshot.runtime_configuration_digest != expected.runtime_configuration_digest
        || checkpoint.manifest_minimum_sequence == 0
        || checkpoint.manifest_digest_at_minimum_sequence.is_zero()
        || checkpoint.readiness_minimum_sequence == 0
        || checkpoint.readiness_digest_at_minimum_sequence.is_zero()
        || checkpoint.revocation_minimum_sequence == 0
        || checkpoint.revocation_digest_at_minimum_sequence.is_zero()
    {
        return Err(ProviderAgentError::ControlDenied);
    }
    validate_interval(
        &snapshot.issued_at,
        &snapshot.expires_at,
        Some(MAX_REVOCATION_SNAPSHOT_LIFETIME_SECONDS),
    )?;
    if snapshot.effective_at < snapshot.issued_at
        || snapshot.effective_at > snapshot.expires_at
        || evaluated_at < &snapshot.effective_at
        || evaluated_at > &snapshot.expires_at
    {
        return Err(ProviderAgentError::InvalidValidity);
    }
    validate_revocation_entries(snapshot)?;
    let key_digest = verify_keyed_signature(KeyedSignatureInput {
        registry,
        provider_id: &signed.signer_id,
        key_id: &signed.key_id,
        purpose: ProviderAgentKeyPurposeV1::RevocationSnapshot,
        issued_at: &snapshot.issued_at,
        expires_at: &snapshot.expires_at,
        evaluated_at,
        signature_hex: &signed.signature_hex,
        message: &signed.signing_message()?,
    })?;
    let digest = signed.digest()?;
    verify_checkpoint_artifact(
        snapshot.sequence,
        digest,
        checkpoint.revocation_minimum_sequence,
        checkpoint.revocation_digest_at_minimum_sequence,
    )?;
    Ok((digest, key_digest))
}

fn verify_checkpoint_artifact(
    sequence: u64,
    digest: Sha256Digest,
    minimum: u64,
    digest_at_minimum: Sha256Digest,
) -> Result<(), ProviderAgentError> {
    if sequence < minimum || (sequence == minimum && digest != digest_at_minimum) {
        return Err(ProviderAgentError::ControlDenied);
    }
    Ok(())
}

fn validate_revocation_entries(
    snapshot: &ProviderAgentRevocationSnapshotV1,
) -> Result<(), ProviderAgentError> {
    if !strictly_sorted(&snapshot.assertion_revocations)
        || !strictly_sorted(&snapshot.key_revocations)
        || snapshot.assertion_revocations.windows(2).any(|pair| {
            pair[0].provider_id == pair[1].provider_id
                && pair[0].artifact_kind == pair[1].artifact_kind
                && pair[0].artifact_digest == pair[1].artifact_digest
        })
        || snapshot.key_revocations.windows(2).any(|pair| {
            pair[0].provider_id == pair[1].provider_id
                && pair[0].key_id == pair[1].key_id
                && pair[0].purpose == pair[1].purpose
        })
        || snapshot
            .assertion_revocations
            .iter()
            .any(|item| item.provider_id != snapshot.provider_id || item.artifact_digest.is_zero())
        || snapshot
            .key_revocations
            .iter()
            .any(|item| item.provider_id != snapshot.provider_id)
    {
        return Err(ProviderAgentError::DuplicateIdentity);
    }
    Ok(())
}

fn verify_not_revoked(
    manifest: &SignedProviderAgentServiceManifestV1,
    readiness: &SignedProviderAgentReadinessV1,
    revocation: &SignedProviderAgentRevocationSnapshotV1,
    manifest_digest: Sha256Digest,
    readiness_digest: Sha256Digest,
    evaluated_at: &UtcTimestamp,
) -> Result<(), ProviderAgentError> {
    let snapshot = &revocation.snapshot;
    let artifact_revoked = snapshot.assertion_revocations.iter().any(|item| {
        item.provider_id == snapshot.provider_id
            && evaluated_at >= &item.revoked_at
            && matches!(
                item.artifact_kind,
                ProviderAgentArtifactKindV1::ServiceManifest
                    if item.artifact_digest == manifest_digest
            )
    }) || snapshot.assertion_revocations.iter().any(|item| {
        item.provider_id == snapshot.provider_id
            && evaluated_at >= &item.revoked_at
            && matches!(
                item.artifact_kind,
                ProviderAgentArtifactKindV1::ReadinessAssertion
                    if item.artifact_digest == readiness_digest
            )
    });
    let key_revoked = [
        (&manifest.key_id, ProviderAgentKeyPurposeV1::ServiceManifest),
        (
            &readiness.key_id,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
        ),
        (
            &revocation.key_id,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
        ),
    ]
    .iter()
    .any(|(key_id, purpose)| {
        snapshot.key_revocations.iter().any(|item| {
            item.provider_id == snapshot.provider_id
                && &item.key_id == *key_id
                && item.purpose == *purpose
                && evaluated_at >= &item.revoked_at
        })
    });
    if artifact_revoked || key_revoked {
        return Err(ProviderAgentError::ControlDenied);
    }
    Ok(())
}

fn validate_binding(binding: &ProviderAgentBindingV1) -> Result<(), ProviderAgentError> {
    if binding.service_spec_digest.is_zero()
        || binding.selected_offer_digest.is_zero()
        || binding.runtime_configuration_digest.is_zero()
        || binding.model_artifact_digest.is_zero()
        || binding.tokenizer_digest.is_zero()
        || binding.runtime_image_digest.is_zero()
        || binding.secret_reference_digest.is_zero()
    {
        return Err(ProviderAgentError::ZeroIdentity);
    }
    if binding.capabilities.is_empty()
        || !strictly_sorted(&binding.capabilities)
        || !binding
            .capabilities
            .contains(&InferenceCapability::Streaming)
    {
        return Err(ProviderAgentError::ControlDenied);
    }
    Ok(())
}

fn validate_signed_header(
    algorithm: &str,
    signer_id: &ProtocolId,
    provider_id: &ProtocolId,
) -> Result<(), ProviderAgentError> {
    if algorithm != ED25519_ALGORITHM_V1 {
        return Err(ProviderAgentError::UnsupportedAlgorithm);
    }
    if signer_id != provider_id {
        return Err(ProviderAgentError::ControlDenied);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct KeyedSignatureInput<'a> {
    registry: &'a ProviderAgentKeyRegistryV1,
    provider_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    purpose: ProviderAgentKeyPurposeV1,
    issued_at: &'a UtcTimestamp,
    expires_at: &'a UtcTimestamp,
    evaluated_at: &'a UtcTimestamp,
    signature_hex: &'a str,
    message: &'a [u8],
}

fn verify_keyed_signature(
    input: KeyedSignatureInput<'_>,
) -> Result<Sha256Digest, ProviderAgentError> {
    let key = input
        .registry
        .get(input.provider_id, input.key_id, input.purpose)
        .ok_or(ProviderAgentError::ControlDenied)?;
    if input.issued_at < &key.valid_from
        || input.expires_at > &key.valid_until
        || input.evaluated_at < &key.valid_from
        || input.evaluated_at > &key.valid_until
        || key
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| input.evaluated_at >= revoked_at)
    {
        return Err(ProviderAgentError::ControlDenied);
    }
    verify_signature(&key.public_key_hex, input.signature_hex, input.message)?;
    key.digest()
}

fn validate_interval(
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    maximum_seconds: Option<u64>,
) -> Result<(), ProviderAgentError> {
    if issued_at > expires_at {
        return Err(ProviderAgentError::InvalidValidity);
    }
    if let Some(maximum) = maximum_seconds
        && expires_at > &issued_at.checked_add_seconds(maximum)?
    {
        return Err(ProviderAgentError::InvalidValidity);
    }
    Ok(())
}

fn validate_evaluated_at(
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    evaluated_at: &UtcTimestamp,
) -> Result<(), ProviderAgentError> {
    if evaluated_at < issued_at || evaluated_at > expires_at {
        return Err(ProviderAgentError::InvalidValidity);
    }
    Ok(())
}

fn strictly_sorted<T: Ord>(items: &[T]) -> bool {
    items.windows(2).all(|pair| pair[0] < pair[1])
}

fn decode_verifying_key(public_key_hex: &str) -> Result<VerifyingKey, ProviderAgentError> {
    let bytes = HEXLOWER
        .decode(public_key_hex.as_bytes())
        .map_err(|_| ProviderAgentError::InvalidEncoding)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ProviderAgentError::InvalidEncoding)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| ProviderAgentError::InvalidEncoding)
}

fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), ProviderAgentError> {
    let key = decode_verifying_key(public_key_hex)?;
    let bytes = HEXLOWER
        .decode(signature_hex.as_bytes())
        .map_err(|_| ProviderAgentError::InvalidEncoding)?;
    let signature =
        Signature::try_from(bytes.as_slice()).map_err(|_| ProviderAgentError::InvalidEncoding)?;
    key.verify_strict(message, &signature)
        .map_err(|_| ProviderAgentError::ControlDenied)
}

#[cfg(test)]
mod tests {
    use data_encoding::HEXLOWER;
    use ed25519_dalek::{Signer, SigningKey};
    use hyperintel_core::{InferenceCapability, ProtocolId, Sha256Digest, UtcTimestamp};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type FixtureMutation = Box<dyn Fn(&mut Fixture) -> Result<(), ProviderAgentError>>;

    struct Fixture {
        manifest_key: SigningKey,
        readiness_key: SigningKey,
        revocation_key: SigningKey,
        registry: ProviderAgentKeyRegistryV1,
        binding: ProviderAgentBindingV1,
        manifest: SignedProviderAgentServiceManifestV1,
        readiness: SignedProviderAgentReadinessV1,
        revocation: SignedProviderAgentRevocationSnapshotV1,
        checkpoint: ProviderAgentRevocationCheckpointV1,
        evaluated_at: UtcTimestamp,
    }

    #[test]
    fn contract_exact_control_lineage_verifies() -> TestResult {
        let fixture = fixture()?;
        let verified = verify_fixture(&fixture)?;
        assert_eq!(verified.manifest, fixture.manifest.digest()?);
        assert_eq!(verified.readiness, fixture.readiness.digest()?);
        assert_eq!(verified.revocation, fixture.revocation.digest()?);
        Ok(())
    }

    #[test]
    fn contract_non_ready_assertions_fail_closed() -> TestResult {
        for status in [
            ProviderAgentReadinessStatusV1::Draining,
            ProviderAgentReadinessStatusV1::Unavailable,
        ] {
            let mut fixture = fixture()?;
            fixture.readiness.claims.status = status;
            sign_readiness(&mut fixture.readiness, &fixture.readiness_key)?;
            assert!(verify_fixture(&fixture).is_err());
        }
        Ok(())
    }

    #[test]
    fn contract_signed_binding_and_original_signature_tampering_fail() -> TestResult {
        let mut cases: Vec<FixtureMutation> = vec![
            Box::new(|fixture| {
                fixture.manifest.schema_version = "wrong".to_owned();
                Ok(())
            }),
            Box::new(|fixture| {
                fixture.manifest.algorithm = "wrong".to_owned();
                Ok(())
            }),
            Box::new(|fixture| {
                fixture.manifest.claims.binding.selected_offer_digest = digest(99);
                Ok(())
            }),
            Box::new(|fixture| {
                fixture.readiness.claims.binding.secret_reference_digest = digest(98);
                Ok(())
            }),
            Box::new(|fixture| {
                fixture.revocation.snapshot.runtime_configuration_digest = digest(97);
                Ok(())
            }),
            Box::new(|fixture| {
                fixture.manifest.signature_hex = "not-hex".to_owned();
                Ok(())
            }),
        ];
        for mutate in &mut cases {
            let mut fixture = fixture()?;
            mutate(&mut fixture)?;
            assert!(verify_fixture(&fixture).is_err());
        }

        let mut fixture = fixture()?;
        fixture.manifest.signature_hex = HEXLOWER.encode(
            &SigningKey::from_bytes(&[0x44; 32])
                .sign(&fixture.manifest.signing_message()?)
                .to_bytes(),
        );
        assert!(verify_fixture(&fixture).is_err());
        Ok(())
    }

    #[test]
    fn contract_semantic_duplicate_revocations_rollback_and_forks_fail() -> TestResult {
        let mut duplicate = fixture()?;
        duplicate.revocation.snapshot.assertion_revocations = vec![
            ProviderAgentAssertionRevocationV1 {
                provider_id: id("provider:alpha")?,
                artifact_kind: ProviderAgentArtifactKindV1::ServiceManifest,
                artifact_digest: digest(50),
                revoked_at: time("2026-07-21T10:04:00Z")?,
            },
            ProviderAgentAssertionRevocationV1 {
                provider_id: id("provider:alpha")?,
                artifact_kind: ProviderAgentArtifactKindV1::ServiceManifest,
                artifact_digest: digest(50),
                revoked_at: time("2026-07-21T10:05:00Z")?,
            },
        ];
        sign_revocation(&mut duplicate.revocation, &duplicate.revocation_key)?;
        assert!(verify_fixture(&duplicate).is_err());

        let mut rollback = fixture()?;
        rollback.checkpoint.revocation_minimum_sequence = 2;
        assert!(verify_fixture(&rollback).is_err());

        let mut manifest_rollback = fixture()?;
        manifest_rollback.checkpoint.manifest_minimum_sequence = 2;
        assert!(verify_fixture(&manifest_rollback).is_err());

        let mut readiness_rollback = fixture()?;
        readiness_rollback.checkpoint.readiness_minimum_sequence = 2;
        assert!(verify_fixture(&readiness_rollback).is_err());

        let mut fork = fixture()?;
        fork.checkpoint.revocation_digest_at_minimum_sequence = digest(77);
        assert!(verify_fixture(&fork).is_err());

        let mut manifest_fork = fixture()?;
        manifest_fork.manifest.claims.expires_at = time("2026-07-21T10:29:00Z")?;
        sign_manifest(&mut manifest_fork.manifest, &manifest_fork.manifest_key)?;
        manifest_fork.readiness.claims.service_manifest_digest = manifest_fork.manifest.digest()?;
        sign_readiness(&mut manifest_fork.readiness, &manifest_fork.readiness_key)?;
        assert!(verify_fixture(&manifest_fork).is_err());

        let mut readiness_fork = fixture()?;
        readiness_fork.readiness.claims.expires_at = time("2026-07-21T10:05:00Z")?;
        sign_readiness(&mut readiness_fork.readiness, &readiness_fork.readiness_key)?;
        assert!(verify_fixture(&readiness_fork).is_err());

        let mut cross_agent = fixture()?;
        cross_agent.checkpoint.agent_instance_id = id("agent:other")?;
        assert!(verify_fixture(&cross_agent).is_err());

        let mut cross_binding = fixture()?;
        cross_binding.checkpoint.binding_digest = digest(78);
        assert!(verify_fixture(&cross_binding).is_err());

        let mut key_duplicate = fixture()?;
        key_duplicate.revocation.snapshot.key_revocations = vec![
            ProviderAgentKeyRevocationV1 {
                provider_id: id("provider:alpha")?,
                key_id: id("key:manifest")?,
                purpose: ProviderAgentKeyPurposeV1::ServiceManifest,
                revoked_at: time("2026-07-21T10:04:00Z")?,
            },
            ProviderAgentKeyRevocationV1 {
                provider_id: id("provider:alpha")?,
                key_id: id("key:manifest")?,
                purpose: ProviderAgentKeyPurposeV1::ServiceManifest,
                revoked_at: time("2026-07-21T10:05:00Z")?,
            },
        ];
        resign_and_pin_revocation(&mut key_duplicate)?;
        assert!(verify_fixture(&key_duplicate).is_err());
        Ok(())
    }

    #[test]
    fn contract_readiness_and_revocation_headers_signatures_and_purposes_fail_closed() -> TestResult
    {
        let mut readiness_schema = fixture()?;
        readiness_schema.readiness.schema_version = "wrong".to_owned();
        assert!(verify_fixture(&readiness_schema).is_err());

        let mut readiness_claims_schema = fixture()?;
        readiness_claims_schema.readiness.claims.schema_version = "wrong".to_owned();
        assert!(verify_fixture(&readiness_claims_schema).is_err());

        let mut readiness_algorithm = fixture()?;
        readiness_algorithm.readiness.algorithm = "wrong".to_owned();
        assert!(verify_fixture(&readiness_algorithm).is_err());

        let mut readiness_signature = fixture()?;
        readiness_signature.readiness.signature_hex = "not-hex".to_owned();
        assert!(verify_fixture(&readiness_signature).is_err());

        let mut readiness_signer = fixture()?;
        readiness_signer.readiness.signer_id = id("provider:other")?;
        assert!(verify_fixture(&readiness_signer).is_err());

        let mut readiness_key_id = fixture()?;
        readiness_key_id.readiness.key_id = id("key:other")?;
        assert!(verify_fixture(&readiness_key_id).is_err());

        let mut readiness_wrong_signature = fixture()?;
        readiness_wrong_signature.readiness.signature_hex = HEXLOWER.encode(
            &SigningKey::from_bytes(&[0x71; 32])
                .sign(&readiness_wrong_signature.readiness.signing_message()?)
                .to_bytes(),
        );
        assert!(verify_fixture(&readiness_wrong_signature).is_err());

        let mut readiness_wrong_purpose = fixture()?;
        readiness_wrong_purpose.registry = registry_with_wrong_purpose(
            &readiness_wrong_purpose,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
        )?;
        assert!(verify_fixture(&readiness_wrong_purpose).is_err());

        let mut revocation_schema = fixture()?;
        revocation_schema.revocation.snapshot.schema_version = "wrong".to_owned();
        assert!(verify_fixture(&revocation_schema).is_err());

        let mut revocation_wrapper_schema = fixture()?;
        revocation_wrapper_schema.revocation.schema_version = "wrong".to_owned();
        assert!(verify_fixture(&revocation_wrapper_schema).is_err());

        let mut revocation_algorithm = fixture()?;
        revocation_algorithm.revocation.algorithm = "wrong".to_owned();
        assert!(verify_fixture(&revocation_algorithm).is_err());

        let mut revocation_signature = fixture()?;
        revocation_signature.revocation.signature_hex = "not-hex".to_owned();
        assert!(verify_fixture(&revocation_signature).is_err());

        let mut revocation_signer = fixture()?;
        revocation_signer.revocation.signer_id = id("provider:other")?;
        assert!(verify_fixture(&revocation_signer).is_err());

        let mut revocation_key_id = fixture()?;
        revocation_key_id.revocation.key_id = id("key:other")?;
        assert!(verify_fixture(&revocation_key_id).is_err());

        let mut revocation_wrong_signature = fixture()?;
        revocation_wrong_signature.revocation.signature_hex = HEXLOWER.encode(
            &SigningKey::from_bytes(&[0x72; 32])
                .sign(&revocation_wrong_signature.revocation.signing_message()?)
                .to_bytes(),
        );
        assert!(verify_fixture(&revocation_wrong_signature).is_err());

        let mut wrong_revocation_purpose = fixture()?;
        let mut registry = ProviderAgentKeyRegistryV1::default();
        register(
            &mut registry,
            &wrong_revocation_purpose.manifest_key,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            "key:manifest",
            None,
        )?;
        register(
            &mut registry,
            &wrong_revocation_purpose.readiness_key,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:readiness",
            None,
        )?;
        register(
            &mut registry,
            &wrong_revocation_purpose.revocation_key,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:revocation",
            None,
        )?;
        wrong_revocation_purpose.registry = registry;
        assert!(verify_fixture(&wrong_revocation_purpose).is_err());
        Ok(())
    }

    fn registry_with_wrong_purpose(
        fixture: &Fixture,
        readiness_purpose: ProviderAgentKeyPurposeV1,
        revocation_purpose: ProviderAgentKeyPurposeV1,
    ) -> Result<ProviderAgentKeyRegistryV1, Box<dyn std::error::Error>> {
        let mut registry = ProviderAgentKeyRegistryV1::default();
        register(
            &mut registry,
            &fixture.manifest_key,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            "key:manifest",
            None,
        )?;
        register(
            &mut registry,
            &fixture.readiness_key,
            readiness_purpose,
            "key:readiness",
            None,
        )?;
        register(
            &mut registry,
            &fixture.revocation_key,
            revocation_purpose,
            "key:revocation",
            None,
        )?;
        Ok(registry)
    }

    #[test]
    fn contract_registry_and_revocation_key_context_are_rechecked_at_evaluation_time() -> TestResult
    {
        let baseline = fixture()?;
        assert!(verify_fixture(&baseline).is_ok());
        let mut substituted = fixture()?;
        substituted.registry = ProviderAgentKeyRegistryV1::default();
        assert!(verify_fixture(&substituted).is_err());

        let mut revoked_key = fixture()?;
        let mut registry = ProviderAgentKeyRegistryV1::default();
        register(
            &mut registry,
            &revoked_key.manifest_key,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            "key:manifest",
            Some(revoked_key.evaluated_at.clone()),
        )?;
        register(
            &mut registry,
            &revoked_key.readiness_key,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:readiness",
            None,
        )?;
        register(
            &mut registry,
            &revoked_key.revocation_key,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
            "key:revocation",
            None,
        )?;
        revoked_key.registry = registry;
        assert!(verify_fixture(&revoked_key).is_err());
        Ok(())
    }

    #[test]
    fn contract_signed_manifest_and_readiness_revocations_are_inclusive() -> TestResult {
        for artifact_kind in [
            ProviderAgentArtifactKindV1::ServiceManifest,
            ProviderAgentArtifactKindV1::ReadinessAssertion,
        ] {
            let mut fixture = fixture()?;
            let artifact_digest = match artifact_kind {
                ProviderAgentArtifactKindV1::ServiceManifest => fixture.manifest.digest()?,
                ProviderAgentArtifactKindV1::ReadinessAssertion => fixture.readiness.digest()?,
            };
            fixture.revocation.snapshot.assertion_revocations =
                vec![ProviderAgentAssertionRevocationV1 {
                    provider_id: fixture.binding.provider_id.clone(),
                    artifact_kind,
                    artifact_digest,
                    revoked_at: fixture.evaluated_at.clone(),
                }];
            resign_and_pin_revocation(&mut fixture)?;
            assert!(verify_fixture(&fixture).is_err());
        }
        Ok(())
    }

    #[test]
    fn contract_all_signed_control_key_revocations_are_inclusive() -> TestResult {
        for (key_id, purpose) in [
            ("key:manifest", ProviderAgentKeyPurposeV1::ServiceManifest),
            (
                "key:readiness",
                ProviderAgentKeyPurposeV1::ReadinessAssertion,
            ),
            (
                "key:revocation",
                ProviderAgentKeyPurposeV1::RevocationSnapshot,
            ),
        ] {
            let mut fixture = fixture()?;
            fixture.revocation.snapshot.key_revocations = vec![ProviderAgentKeyRevocationV1 {
                provider_id: fixture.binding.provider_id.clone(),
                key_id: id(key_id)?,
                purpose,
                revoked_at: fixture.evaluated_at.clone(),
            }];
            resign_and_pin_revocation(&mut fixture)?;
            assert!(verify_fixture(&fixture).is_err());
        }
        Ok(())
    }

    #[test]
    fn contract_wrong_purpose_registry_entry_never_substitutes() -> TestResult {
        let mut fixture = fixture()?;
        let mut registry = ProviderAgentKeyRegistryV1::default();
        register(
            &mut registry,
            &fixture.manifest_key,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:manifest",
            None,
        )?;
        register(
            &mut registry,
            &fixture.readiness_key,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:readiness",
            None,
        )?;
        register(
            &mut registry,
            &fixture.revocation_key,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
            "key:revocation",
            None,
        )?;
        fixture.registry = registry;
        assert!(verify_fixture(&fixture).is_err());
        Ok(())
    }

    #[test]
    fn contract_duplicate_key_insert_is_rejected_and_original_remains_trusted() -> TestResult {
        let mut fixture = fixture()?;
        let attacker = SigningKey::from_bytes(&[0x7f; 32]);
        let duplicate = RegisteredProviderAgentKeyV1 {
            schema_version: PROVIDER_AGENT_KEY_SCHEMA_V1.to_owned(),
            provider_id: fixture.binding.provider_id.clone(),
            key_id: id("key:manifest")?,
            purpose: ProviderAgentKeyPurposeV1::ServiceManifest,
            public_key_hex: HEXLOWER.encode(attacker.verifying_key().as_bytes()),
            valid_from: time("2026-07-21T09:00:00Z")?,
            valid_until: time("2026-07-21T11:00:00Z")?,
            revoked_at: None,
        };
        assert!(matches!(
            fixture.registry.register(duplicate),
            Err(ProviderAgentError::DuplicateIdentity)
        ));
        assert!(verify_fixture(&fixture).is_ok());
        Ok(())
    }

    fn resign_and_pin_revocation(fixture: &mut Fixture) -> TestResult {
        sign_revocation(&mut fixture.revocation, &fixture.revocation_key)?;
        fixture.checkpoint.revocation_digest_at_minimum_sequence = fixture.revocation.digest()?;
        Ok(())
    }

    fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
        let (manifest_key, readiness_key, revocation_key, registry) = keys_and_registry()?;
        let binding = ProviderAgentBindingV1 {
            provider_id: id("provider:alpha")?,
            agent_instance_id: id("agent:alpha-1")?,
            service_spec_digest: digest(1),
            selected_offer_digest: digest(2),
            runtime_configuration_digest: digest(3),
            model_artifact_digest: digest(4),
            tokenizer_digest: digest(5),
            runtime_image_digest: digest(6),
            capabilities: vec![InferenceCapability::Streaming],
            receipt_key_id: id("key:receipt")?,
            interface: ProviderTextStreamInterfaceV1::OpenAiTextStreamingChatCompletionsV1,
            secret_reference_digest: digest(7),
        };
        let mut manifest = SignedProviderAgentServiceManifestV1 {
            schema_version: SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1.to_owned(),
            claims: ProviderAgentServiceManifestClaimsV1 {
                schema_version: PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1.to_owned(),
                sequence: 1,
                binding: binding.clone(),
                issued_at: time("2026-07-21T10:00:00Z")?,
                expires_at: time("2026-07-21T10:30:00Z")?,
            },
            signer_id: id("provider:alpha")?,
            key_id: id("key:manifest")?,
            algorithm: ED25519_ALGORITHM_V1.to_owned(),
            signature_hex: String::new(),
        };
        sign_manifest(&mut manifest, &manifest_key)?;
        let mut readiness = SignedProviderAgentReadinessV1 {
            schema_version: SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1.to_owned(),
            claims: ProviderAgentReadinessClaimsV1 {
                schema_version: PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1.to_owned(),
                sequence: 1,
                binding: binding.clone(),
                service_manifest_digest: manifest.digest()?,
                status: ProviderAgentReadinessStatusV1::Ready,
                issued_at: time("2026-07-21T10:01:00Z")?,
                expires_at: time("2026-07-21T10:06:00Z")?,
            },
            signer_id: id("provider:alpha")?,
            key_id: id("key:readiness")?,
            algorithm: ED25519_ALGORITHM_V1.to_owned(),
            signature_hex: String::new(),
        };
        sign_readiness(&mut readiness, &readiness_key)?;
        let mut revocation = SignedProviderAgentRevocationSnapshotV1 {
            schema_version: SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot: ProviderAgentRevocationSnapshotV1 {
                schema_version: PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
                provider_id: id("provider:alpha")?,
                runtime_configuration_digest: digest(3),
                sequence: 1,
                issued_at: time("2026-07-21T10:01:00Z")?,
                effective_at: time("2026-07-21T10:01:00Z")?,
                expires_at: time("2026-07-21T10:06:00Z")?,
                assertion_revocations: vec![],
                key_revocations: vec![],
            },
            signer_id: id("provider:alpha")?,
            key_id: id("key:revocation")?,
            algorithm: ED25519_ALGORITHM_V1.to_owned(),
            signature_hex: String::new(),
        };
        sign_revocation(&mut revocation, &revocation_key)?;
        let checkpoint = ProviderAgentRevocationCheckpointV1 {
            provider_id: id("provider:alpha")?,
            runtime_configuration_digest: digest(3),
            agent_instance_id: binding.agent_instance_id.clone(),
            binding_digest: binding.digest()?,
            manifest_minimum_sequence: 1,
            manifest_digest_at_minimum_sequence: manifest.digest()?,
            readiness_minimum_sequence: 1,
            readiness_digest_at_minimum_sequence: readiness.digest()?,
            revocation_minimum_sequence: 1,
            revocation_digest_at_minimum_sequence: revocation.digest()?,
        };
        Ok(Fixture {
            manifest_key,
            readiness_key,
            revocation_key,
            registry,
            binding,
            manifest,
            readiness,
            revocation,
            checkpoint,
            evaluated_at: time("2026-07-21T10:03:00Z")?,
        })
    }

    fn keys_and_registry() -> Result<
        (
            SigningKey,
            SigningKey,
            SigningKey,
            ProviderAgentKeyRegistryV1,
        ),
        Box<dyn std::error::Error>,
    > {
        let manifest = SigningKey::from_bytes(&[0x31; 32]);
        let readiness = SigningKey::from_bytes(&[0x32; 32]);
        let revocation = SigningKey::from_bytes(&[0x33; 32]);
        let mut registry = ProviderAgentKeyRegistryV1::default();
        register(
            &mut registry,
            &manifest,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            "key:manifest",
            None,
        )?;
        register(
            &mut registry,
            &readiness,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:readiness",
            None,
        )?;
        register(
            &mut registry,
            &revocation,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
            "key:revocation",
            None,
        )?;
        Ok((manifest, readiness, revocation, registry))
    }

    fn verify_fixture(
        fixture: &Fixture,
    ) -> Result<VerifiedProviderAgentControlV1, ProviderAgentError> {
        verify_provider_agent_control_v1(
            &fixture.manifest,
            &fixture.readiness,
            &fixture.revocation,
            &fixture.checkpoint,
            &fixture.registry,
            &fixture.binding,
            &fixture.evaluated_at,
        )
    }
    fn register(
        registry: &mut ProviderAgentKeyRegistryV1,
        signer: &SigningKey,
        purpose: ProviderAgentKeyPurposeV1,
        key_id: &str,
        revoked_at: Option<UtcTimestamp>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        registry.register(RegisteredProviderAgentKeyV1 {
            schema_version: PROVIDER_AGENT_KEY_SCHEMA_V1.to_owned(),
            provider_id: id("provider:alpha")?,
            key_id: id(key_id)?,
            purpose,
            public_key_hex: HEXLOWER.encode(signer.verifying_key().as_bytes()),
            valid_from: time("2026-07-21T09:00:00Z")?,
            valid_until: time("2026-07-21T11:00:00Z")?,
            revoked_at,
        })?;
        Ok(())
    }
    fn sign_manifest(
        value: &mut SignedProviderAgentServiceManifestV1,
        key: &SigningKey,
    ) -> Result<(), ProviderAgentError> {
        value.signature_hex = HEXLOWER.encode(&key.sign(&value.signing_message()?).to_bytes());
        Ok(())
    }
    fn sign_readiness(
        value: &mut SignedProviderAgentReadinessV1,
        key: &SigningKey,
    ) -> Result<(), ProviderAgentError> {
        value.signature_hex = HEXLOWER.encode(&key.sign(&value.signing_message()?).to_bytes());
        Ok(())
    }
    fn sign_revocation(
        value: &mut SignedProviderAgentRevocationSnapshotV1,
        key: &SigningKey,
    ) -> Result<(), ProviderAgentError> {
        value.signature_hex = HEXLOWER.encode(&key.sign(&value.signing_message()?).to_bytes());
        Ok(())
    }
    fn digest(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }
    fn id(value: &str) -> Result<ProtocolId, Box<dyn std::error::Error>> {
        Ok(value.parse()?)
    }
    fn time(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
        Ok(value.parse()?)
    }
}
