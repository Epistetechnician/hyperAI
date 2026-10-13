use std::{collections::BTreeMap, fmt};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signature, VerifyingKey};
use hyperintel_core::{
    ProtocolId, Sha256Digest, UtcTimestamp, canonical_json, domain_separated_digest,
};
use hyperintel_runtime_config::SecretReferenceV1;
use serde::{Deserialize, Serialize};

use crate::{
    ED25519_ALGORITHM_V1, MAX_OPERATOR_SECRET_AUTHORIZATION_LIFETIME_SECONDS,
    OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1, OPERATOR_SECRET_KEY_SCHEMA_V1,
    OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1, SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1,
    SecretProviderError,
};

const AUTHORIZATION_STATEMENT_DOMAIN_V1: &str =
    "hyperintel.operator-secret-authorization-statement.v1";
const AUTHORIZATION_DIGEST_DOMAIN_V1: &str = "hyperintel.operator-secret-authorization-digest.v1";
const OPERATOR_SECRET_KEY_RECORD_DIGEST_DOMAIN_V1: &str =
    "hyperintel.operator-secret-key-record.v1";
const OPERATOR_SECRET_REVOCATION_STATE_DIGEST_DOMAIN_V1: &str =
    "hyperintel.operator-secret-revocation-state-digest.v1";
const OPERATOR_SECRET_REVOCATION_CHECKPOINT_DIGEST_DOMAIN_V1: &str =
    "hyperintel.operator-secret-revocation-checkpoint-digest.v1";
const SECRET_REFERENCE_DIGEST_DOMAIN_V1: &str = "hyperintel.secret-reference-digest.v1";

/// The only secret use authorized by this state slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecretPurposeV1 {
    /// Authenticate one selected inference-provider API interaction in a later slice.
    ProviderApiAuthentication,
}

/// Dedicated purpose for operator-secret authorization verification keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperatorSecretKeyPurposeV1 {
    /// Verify only operator-secret resolution authorizations.
    SecretResolutionAuthorization,
}

/// One caller-trusted operator-secret verification key.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredOperatorSecretKeyV1 {
    /// Fixed key-record schema.
    pub schema_version: String,
    /// Dedicated operator-secret issuer identity.
    pub operator_id: ProtocolId,
    /// Key identity within the operator-secret namespace.
    pub key_id: ProtocolId,
    /// Closed key purpose.
    pub purpose: OperatorSecretKeyPurposeV1,
    /// Lowercase hexadecimal Ed25519 public key.
    pub public_key_hex: String,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

impl fmt::Debug for RegisteredOperatorSecretKeyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredOperatorSecretKeyV1")
            .field("schema_version", &self.schema_version)
            .field("operator_id", &self.operator_id)
            .field("key_id", &self.key_id)
            .field("purpose", &self.purpose)
            .field("public_key_hex", &"[REDACTED]")
            .field("valid_from", &self.valid_from)
            .field("valid_until", &self.valid_until)
            .field("revoked_at", &self.revoked_at)
            .finish()
    }
}

/// Caller-owned dedicated operator-secret key registry.
#[derive(Clone, Debug, Default)]
pub struct OperatorSecretKeyRegistryV1 {
    keys: BTreeMap<
        (ProtocolId, ProtocolId, OperatorSecretKeyPurposeV1),
        RegisteredOperatorSecretKeyV1,
    >,
}

impl OperatorSecretKeyRegistryV1 {
    /// Register one exact key without overwrite or purpose fallback.
    ///
    /// # Errors
    ///
    /// Returns for schema, interval, encoding, or duplicate failures.
    pub fn register(
        &mut self,
        key: RegisteredOperatorSecretKeyV1,
    ) -> Result<(), SecretProviderError> {
        if key.schema_version != OPERATOR_SECRET_KEY_SCHEMA_V1 {
            return Err(SecretProviderError::UnsupportedSchema);
        }
        validate_interval(&key.valid_from, &key.valid_until, None)?;
        if key
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| revoked_at < &key.valid_from)
        {
            return Err(SecretProviderError::InvalidValidity);
        }
        decode_verifying_key(&key.public_key_hex)?;
        let identity = (key.operator_id.clone(), key.key_id.clone(), key.purpose);
        if self.keys.contains_key(&identity) {
            return Err(SecretProviderError::DuplicateIdentity);
        }
        self.keys.insert(identity, key);
        Ok(())
    }

    fn get(
        &self,
        operator_id: &ProtocolId,
        key_id: &ProtocolId,
        purpose: OperatorSecretKeyPurposeV1,
    ) -> Option<&RegisteredOperatorSecretKeyV1> {
        self.keys
            .get(&(operator_id.clone(), key_id.clone(), purpose))
    }
}

/// Exact claims authorizing one bounded local secret resolution.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSecretAuthorizationClaimsV1 {
    /// Fixed claims schema.
    pub schema_version: String,
    /// Nonzero logical authorization sequence.
    pub sequence: u64,
    /// Dedicated operator-secret issuer.
    pub operator_id: ProtocolId,
    /// Exact validated runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Exact component allowed to consume the material.
    pub consumer_id: ProtocolId,
    /// Exact selected inference provider for which the secret is intended.
    pub selected_provider_id: ProtocolId,
    /// Digest of the exact immutable-version secret reference.
    pub secret_reference_digest: Sha256Digest,
    /// The only supported use.
    pub purpose: SecretPurposeV1,
    /// Exact caller-owned high-risk operation context.
    pub operation_context_digest: Sha256Digest,
    /// Maximum bytes accepted from the injected resolver.
    pub maximum_secret_bytes: u32,
    /// Issuance time.
    pub issued_at: UtcTimestamp,
    /// Expiration time.
    pub expires_at: UtcTimestamp,
}

impl fmt::Debug for OperatorSecretAuthorizationClaimsV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperatorSecretAuthorizationClaimsV1")
            .field("schema_version", &self.schema_version)
            .field("sequence", &self.sequence)
            .field("operator_id", &self.operator_id)
            .field("runtime_configuration_digest", &"[REDACTED]")
            .field("consumer_id", &"[REDACTED]")
            .field("selected_provider_id", &"[REDACTED]")
            .field("secret_reference_digest", &"[REDACTED]")
            .field("purpose", &self.purpose)
            .field("operation_context_digest", &"[REDACTED]")
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// Detached Ed25519 signature over one exact secret authorization.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedOperatorSecretAuthorizationV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact signed claims.
    pub claims: OperatorSecretAuthorizationClaimsV1,
    /// Must equal the claims operator.
    pub signer_id: ProtocolId,
    /// Exact dedicated operator-secret verification key.
    pub key_id: ProtocolId,
    /// Fixed Ed25519 label.
    pub algorithm: String,
    /// Lowercase hexadecimal detached signature.
    pub signature_hex: String,
}

impl fmt::Debug for SignedOperatorSecretAuthorizationV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedOperatorSecretAuthorizationV1")
            .field("schema_version", &self.schema_version)
            .field("operator_id", &self.claims.operator_id)
            .field("consumer_id", &"[REDACTED]")
            .field("selected_provider_id", &"[REDACTED]")
            .field("secret_reference_digest", &"[REDACTED]")
            .field("operation_context_digest", &"[REDACTED]")
            .field("purpose", &self.claims.purpose)
            .field("key_id", &self.key_id)
            .field("signature_hex", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Serialize)]
struct AuthorizationSigningMessage<'a> {
    statement_domain: &'static str,
    wrapper_schema: &'a str,
    claims: &'a OperatorSecretAuthorizationClaimsV1,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl SignedOperatorSecretAuthorizationV1 {
    /// Return canonical domain-bound signing bytes, excluding only the signature.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, SecretProviderError> {
        Ok(canonical_json(&AuthorizationSigningMessage {
            statement_domain: AUTHORIZATION_STATEMENT_DOMAIN_V1,
            wrapper_schema: &self.schema_version,
            claims: &self.claims,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        })?)
    }

    /// Return the canonical identity of the complete signed authorization.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, SecretProviderError> {
        Ok(domain_separated_digest(
            AUTHORIZATION_DIGEST_DOMAIN_V1,
            self,
        )?)
    }
}

/// Inclusive revocation of one exact signed authorization.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSecretAuthorizationRevocationV1 {
    /// Operator-secret issuer.
    pub operator_id: ProtocolId,
    /// Complete signed-authorization digest.
    pub authorization_digest: Sha256Digest,
    /// Inclusive revocation instant.
    pub revoked_at: UtcTimestamp,
}

/// Inclusive revocation of one dedicated operator-secret key.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSecretKeyRevocationV1 {
    /// Operator-secret issuer.
    pub operator_id: ProtocolId,
    /// Exact key identity.
    pub key_id: ProtocolId,
    /// Exact dedicated key purpose.
    pub purpose: OperatorSecretKeyPurposeV1,
    /// Inclusive revocation instant.
    pub revoked_at: UtcTimestamp,
}

/// Caller-owned, freshness-bounded closed revocation state.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSecretRevocationStateV1 {
    /// Fixed revocation-state schema.
    pub schema_version: String,
    /// Operator-secret issuer.
    pub operator_id: ProtocolId,
    /// Exact runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Nonzero monotonic state sequence.
    pub sequence: u64,
    /// State issuance time.
    pub issued_at: UtcTimestamp,
    /// First represented evaluation instant.
    pub effective_at: UtcTimestamp,
    /// Last represented evaluation instant.
    pub expires_at: UtcTimestamp,
    /// Strictly sorted exact authorization revocations.
    pub authorization_revocations: Vec<OperatorSecretAuthorizationRevocationV1>,
    /// Strictly sorted exact key revocations.
    pub key_revocations: Vec<OperatorSecretKeyRevocationV1>,
}

impl fmt::Debug for OperatorSecretRevocationStateV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperatorSecretRevocationStateV1")
            .field("operator_id", &self.operator_id)
            .field("sequence", &self.sequence)
            .field("revocations", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Caller-pinned rollback and same-sequence-fork checkpoint.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct OperatorSecretRevocationCheckpointV1 {
    /// Exact operator-secret issuer.
    pub operator_id: ProtocolId,
    /// Exact runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Minimum accepted revocation sequence.
    pub minimum_sequence: u64,
    /// Exact state digest required at the minimum sequence.
    pub digest_at_minimum_sequence: Sha256Digest,
}

impl fmt::Debug for OperatorSecretRevocationCheckpointV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperatorSecretRevocationCheckpointV1")
            .field("operator_id", &self.operator_id)
            .field("runtime_configuration_digest", &"[REDACTED]")
            .field("minimum_sequence", &self.minimum_sequence)
            .field("digest_at_minimum_sequence", &"[REDACTED]")
            .finish()
    }
}

/// Exact caller-owned expectation for one resolution.
pub struct OperatorSecretAuthorizationExpectationV1<'a> {
    /// Expected operator-secret issuer.
    pub operator_id: &'a ProtocolId,
    /// Expected runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Expected material consumer.
    pub consumer_id: &'a ProtocolId,
    /// Expected selected inference provider.
    pub selected_provider_id: &'a ProtocolId,
    /// Expected immutable-version reference.
    pub secret_reference: &'a SecretReferenceV1,
    /// Expected closed purpose.
    pub purpose: SecretPurposeV1,
    /// Expected operation context.
    pub operation_context_digest: Sha256Digest,
    /// Expected maximum returned bytes.
    pub maximum_secret_bytes: u32,
}

/// Inputs atomically rechecked immediately before resolver invocation.
pub struct OperatorSecretAuthorizationVerificationContextV1<'a> {
    /// Dedicated caller-owned operator-secret trust registry.
    pub key_registry: &'a OperatorSecretKeyRegistryV1,
    /// Current caller-owned closed revocation state.
    pub revocation_state: &'a OperatorSecretRevocationStateV1,
    /// Current caller-pinned revocation checkpoint.
    pub revocation_checkpoint: &'a OperatorSecretRevocationCheckpointV1,
    /// Exact expected resolution context.
    pub expectation: &'a OperatorSecretAuthorizationExpectationV1<'a>,
    /// Explicit evaluation time.
    pub evaluated_at: &'a UtcTimestamp,
}

/// Opaque verified authorization consumed inside one atomic resolve call.
pub struct VerifiedOperatorSecretAuthorizationV1 {
    pub(crate) authorization_digest: Sha256Digest,
    pub(crate) runtime_configuration_digest: Sha256Digest,
    pub(crate) consumer_id: ProtocolId,
    pub(crate) selected_provider_id: ProtocolId,
    pub(crate) secret_reference: SecretReferenceV1,
    pub(crate) purpose: SecretPurposeV1,
    pub(crate) operation_context_digest: Sha256Digest,
    pub(crate) maximum_secret_bytes: u32,
    pub(crate) evaluated_at: UtcTimestamp,
    pub(crate) signing_key_record_digest: Sha256Digest,
    pub(crate) revocation_state_digest: Sha256Digest,
    pub(crate) revocation_checkpoint_digest: Sha256Digest,
}

impl fmt::Debug for VerifiedOperatorSecretAuthorizationV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedOperatorSecretAuthorizationV1")
            .field("authorization_digest", &"[REDACTED]")
            .field("runtime_configuration_digest", &"[REDACTED]")
            .field("consumer_id", &"[REDACTED]")
            .field("selected_provider_id", &"[REDACTED]")
            .field("secret_reference", &"[REDACTED]")
            .field("purpose", &self.purpose)
            .field("operation_context_digest", &"[REDACTED]")
            .field("evaluated_at", &self.evaluated_at)
            .field("signing_key_record_digest", &"[REDACTED]")
            .field("revocation_state_digest", &"[REDACTED]")
            .field("revocation_checkpoint_digest", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl VerifiedOperatorSecretAuthorizationV1 {
    /// This local verification never grants external authority.
    #[must_use]
    pub const fn external_authority_granted(&self) -> bool {
        false
    }
}

/// Return the domain-separated identity of one nonsecret reference.
///
/// This function never receives or hashes secret material.
///
/// # Errors
///
/// Returns when canonicalization fails.
pub fn secret_reference_digest_v1(
    reference: &SecretReferenceV1,
) -> Result<Sha256Digest, SecretProviderError> {
    Ok(domain_separated_digest(
        SECRET_REFERENCE_DIGEST_DOMAIN_V1,
        reference,
    )?)
}

/// Return the domain-separated identity of caller-owned revocation state.
///
/// # Errors
///
/// Returns when canonicalization fails.
pub fn operator_secret_revocation_state_digest_v1(
    state: &OperatorSecretRevocationStateV1,
) -> Result<Sha256Digest, SecretProviderError> {
    Ok(domain_separated_digest(
        OPERATOR_SECRET_REVOCATION_STATE_DIGEST_DOMAIN_V1,
        state,
    )?)
}

pub(crate) fn verify_operator_secret_authorization_v1(
    signed: &SignedOperatorSecretAuthorizationV1,
    context: &OperatorSecretAuthorizationVerificationContextV1<'_>,
) -> Result<VerifiedOperatorSecretAuthorizationV1, SecretProviderError> {
    validate_authorization_expectation(signed, context)?;
    let claims = &signed.claims;
    let revocation_state_digest = validate_revocation_context(context)?;
    let key = context
        .key_registry
        .get(
            &claims.operator_id,
            &signed.key_id,
            OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
        )
        .ok_or(SecretProviderError::AuthorizationDenied)?;
    verify_key_window(
        key,
        &claims.issued_at,
        &claims.expires_at,
        context.evaluated_at,
    )?;
    verify_signature(
        &key.public_key_hex,
        &signed.signature_hex,
        &signed.signing_message()?,
    )?;
    let authorization_digest = signed.digest()?;
    verify_not_revoked(signed, authorization_digest, context)?;
    Ok(VerifiedOperatorSecretAuthorizationV1 {
        authorization_digest,
        runtime_configuration_digest: claims.runtime_configuration_digest,
        consumer_id: claims.consumer_id.clone(),
        selected_provider_id: claims.selected_provider_id.clone(),
        secret_reference: context.expectation.secret_reference.clone(),
        purpose: claims.purpose,
        operation_context_digest: claims.operation_context_digest,
        maximum_secret_bytes: claims.maximum_secret_bytes,
        evaluated_at: context.evaluated_at.clone(),
        signing_key_record_digest: domain_separated_digest(
            OPERATOR_SECRET_KEY_RECORD_DIGEST_DOMAIN_V1,
            key,
        )?,
        revocation_state_digest,
        revocation_checkpoint_digest: domain_separated_digest(
            OPERATOR_SECRET_REVOCATION_CHECKPOINT_DIGEST_DOMAIN_V1,
            context.revocation_checkpoint,
        )?,
    })
}

fn validate_authorization_expectation(
    signed: &SignedOperatorSecretAuthorizationV1,
    context: &OperatorSecretAuthorizationVerificationContextV1<'_>,
) -> Result<(), SecretProviderError> {
    if signed.schema_version != SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1
        || signed.claims.schema_version != OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1
        || context.revocation_state.schema_version != OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1
    {
        return Err(SecretProviderError::UnsupportedSchema);
    }
    if signed.algorithm != ED25519_ALGORITHM_V1 {
        return Err(SecretProviderError::UnsupportedAlgorithm);
    }
    let claims = &signed.claims;
    let expectation = context.expectation;
    if claims.sequence == 0
        || claims.runtime_configuration_digest.is_zero()
        || claims.secret_reference_digest.is_zero()
        || claims.operation_context_digest.is_zero()
        || expectation.runtime_configuration_digest.is_zero()
        || expectation.operation_context_digest.is_zero()
        || context
            .revocation_state
            .runtime_configuration_digest
            .is_zero()
        || context
            .revocation_checkpoint
            .runtime_configuration_digest
            .is_zero()
        || context
            .revocation_checkpoint
            .digest_at_minimum_sequence
            .is_zero()
    {
        return Err(SecretProviderError::ZeroDigest);
    }
    let hard_maximum = u32::try_from(crate::MAX_SECRET_MATERIAL_BYTES)
        .map_err(|_| SecretProviderError::InvalidMaterial)?;
    if claims.maximum_secret_bytes == 0
        || claims.maximum_secret_bytes > hard_maximum
        || expectation.maximum_secret_bytes == 0
        || expectation.maximum_secret_bytes > hard_maximum
    {
        return Err(SecretProviderError::InvalidMaterial);
    }
    let expected_reference_digest = secret_reference_digest_v1(expectation.secret_reference)?;
    if signed.signer_id != claims.operator_id
        || claims.operator_id != *expectation.operator_id
        || claims.runtime_configuration_digest != expectation.runtime_configuration_digest
        || claims.consumer_id != *expectation.consumer_id
        || claims.selected_provider_id != *expectation.selected_provider_id
        || claims.secret_reference_digest != expected_reference_digest
        || claims.purpose != expectation.purpose
        || claims.operation_context_digest != expectation.operation_context_digest
        || claims.maximum_secret_bytes != expectation.maximum_secret_bytes
    {
        return Err(SecretProviderError::AuthorizationDenied);
    }
    validate_interval(
        &claims.issued_at,
        &claims.expires_at,
        Some(MAX_OPERATOR_SECRET_AUTHORIZATION_LIFETIME_SECONDS),
    )?;
    if context.evaluated_at < &claims.issued_at || context.evaluated_at > &claims.expires_at {
        return Err(SecretProviderError::InvalidValidity);
    }
    Ok(())
}

fn verify_not_revoked(
    signed: &SignedOperatorSecretAuthorizationV1,
    authorization_digest: Sha256Digest,
    context: &OperatorSecretAuthorizationVerificationContextV1<'_>,
) -> Result<(), SecretProviderError> {
    let claims = &signed.claims;
    if context
        .revocation_state
        .authorization_revocations
        .iter()
        .any(|item| {
            item.operator_id == claims.operator_id
                && item.authorization_digest == authorization_digest
                && context.evaluated_at >= &item.revoked_at
        })
        || context.revocation_state.key_revocations.iter().any(|item| {
            item.operator_id == claims.operator_id
                && item.key_id == signed.key_id
                && item.purpose == OperatorSecretKeyPurposeV1::SecretResolutionAuthorization
                && context.evaluated_at >= &item.revoked_at
        })
    {
        return Err(SecretProviderError::AuthorizationDenied);
    }
    Ok(())
}

fn validate_revocation_context(
    context: &OperatorSecretAuthorizationVerificationContextV1<'_>,
) -> Result<Sha256Digest, SecretProviderError> {
    let state = context.revocation_state;
    let checkpoint = context.revocation_checkpoint;
    let expectation = context.expectation;
    if state.operator_id != *expectation.operator_id
        || state.runtime_configuration_digest != expectation.runtime_configuration_digest
        || checkpoint.operator_id != state.operator_id
        || checkpoint.runtime_configuration_digest != state.runtime_configuration_digest
        || state.sequence == 0
        || checkpoint.minimum_sequence == 0
    {
        return Err(SecretProviderError::AuthorizationDenied);
    }
    validate_interval(
        &state.issued_at,
        &state.expires_at,
        Some(MAX_OPERATOR_SECRET_AUTHORIZATION_LIFETIME_SECONDS),
    )?;
    if state.effective_at < state.issued_at
        || state.effective_at > state.expires_at
        || context.evaluated_at < &state.effective_at
        || context.evaluated_at > &state.expires_at
    {
        return Err(SecretProviderError::InvalidValidity);
    }
    if !strictly_sorted(&state.authorization_revocations)
        || !strictly_sorted(&state.key_revocations)
        || state.authorization_revocations.windows(2).any(|pair| {
            pair[0].operator_id == pair[1].operator_id
                && pair[0].authorization_digest == pair[1].authorization_digest
        })
        || state.key_revocations.windows(2).any(|pair| {
            pair[0].operator_id == pair[1].operator_id
                && pair[0].key_id == pair[1].key_id
                && pair[0].purpose == pair[1].purpose
        })
        || state.authorization_revocations.iter().any(|item| {
            item.operator_id != state.operator_id || item.authorization_digest.is_zero()
        })
        || state
            .key_revocations
            .iter()
            .any(|item| item.operator_id != state.operator_id)
    {
        return Err(SecretProviderError::DuplicateIdentity);
    }
    let state_digest = operator_secret_revocation_state_digest_v1(state)?;
    if state.sequence < checkpoint.minimum_sequence
        || (state.sequence == checkpoint.minimum_sequence
            && state_digest != checkpoint.digest_at_minimum_sequence)
    {
        return Err(SecretProviderError::AuthorizationDenied);
    }
    Ok(state_digest)
}

fn strictly_sorted<T: Ord>(items: &[T]) -> bool {
    items.windows(2).all(|pair| pair[0] < pair[1])
}

fn validate_interval(
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    maximum_seconds: Option<u64>,
) -> Result<(), SecretProviderError> {
    if issued_at > expires_at {
        return Err(SecretProviderError::InvalidValidity);
    }
    if let Some(maximum) = maximum_seconds {
        let maximum_expiry = issued_at.checked_add_seconds(maximum)?;
        if expires_at > &maximum_expiry {
            return Err(SecretProviderError::InvalidValidity);
        }
    }
    Ok(())
}

fn verify_key_window(
    key: &RegisteredOperatorSecretKeyV1,
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    evaluated_at: &UtcTimestamp,
) -> Result<(), SecretProviderError> {
    if issued_at < &key.valid_from
        || expires_at > &key.valid_until
        || evaluated_at < &key.valid_from
        || evaluated_at > &key.valid_until
        || key
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| evaluated_at >= revoked_at)
    {
        return Err(SecretProviderError::AuthorizationDenied);
    }
    Ok(())
}

fn decode_verifying_key(public_key_hex: &str) -> Result<VerifyingKey, SecretProviderError> {
    let bytes = HEXLOWER
        .decode(public_key_hex.as_bytes())
        .map_err(|_| SecretProviderError::InvalidEncoding)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SecretProviderError::InvalidEncoding)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| SecretProviderError::InvalidEncoding)
}

fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), SecretProviderError> {
    let key = decode_verifying_key(public_key_hex)?;
    let bytes = HEXLOWER
        .decode(signature_hex.as_bytes())
        .map_err(|_| SecretProviderError::InvalidEncoding)?;
    let signature =
        Signature::try_from(bytes.as_slice()).map_err(|_| SecretProviderError::InvalidEncoding)?;
    key.verify_strict(message, &signature)
        .map_err(|_| SecretProviderError::AuthorizationDenied)
}
