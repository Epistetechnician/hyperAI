use std::collections::BTreeMap;

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signature, VerifyingKey};
use hyperintel_core::{
    CanonicalizationError, ProtocolId, Sha256Digest, UtcTimestamp, canonical_json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::AssuranceRootClassV1;

/// Verification key with an explicit validity and revocation boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredVerificationKeyV1 {
    /// Signer identity.
    pub signer_id: ProtocolId,
    /// Key identity.
    pub key_id: ProtocolId,
    /// Assurance trust-root class assigned by the caller.
    pub trust_root_class: AssuranceRootClassV1,
    /// Lowercase hexadecimal Ed25519 public key.
    pub public_key_hex: String,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

/// Caller-owned registry of portable verification keys.
#[derive(Clone, Debug, Default)]
pub struct VerificationKeyRegistry {
    keys: BTreeMap<(ProtocolId, ProtocolId), RegisteredVerificationKeyV1>,
}

/// Domain-specific canonical message and detached Ed25519 verification metadata.
///
/// The caller owns the message schema and must include every protocol field whose mutation
/// should invalidate the signature. This type centralizes only key identity, validity,
/// revocation, encoding, and cryptographic verification.
pub struct Ed25519MessageVerificationInputV1<'a> {
    /// Claimed signer identity.
    pub signer_id: &'a ProtocolId,
    /// Claimed verification-key identity.
    pub key_id: &'a ProtocolId,
    /// First instant at which the signature is valid.
    pub issued_at: &'a UtcTimestamp,
    /// Last instant at which the signature is valid.
    pub expires_at: &'a UtcTimestamp,
    /// Must be `ed25519`.
    pub algorithm: &'a str,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: &'a str,
    /// Exact domain-specific canonical message bytes.
    pub message: &'a [u8],
}

impl VerificationKeyRegistry {
    /// Register a key, rejecting duplicate `(signer_id, key_id)` pairs.
    ///
    /// # Errors
    ///
    /// Returns on duplicate identity or invalid key bytes.
    pub fn register(&mut self, key: RegisteredVerificationKeyV1) -> Result<(), SignatureError> {
        decode_verifying_key(&key.public_key_hex)?;
        let identity = (key.signer_id.clone(), key.key_id.clone());
        if self.keys.contains_key(&identity) {
            return Err(SignatureError::DuplicateKey);
        }
        self.keys.insert(identity, key);
        Ok(())
    }

    /// Return whether the registry contains this exact key record.
    ///
    /// Bundle-disclosed keys must match a caller-controlled registry before
    /// they can participate in assurance verification.
    #[must_use]
    pub fn contains_exact(&self, key: &RegisteredVerificationKeyV1) -> bool {
        self.get(&key.signer_id, &key.key_id) == Some(key)
    }

    /// Return the caller-assigned trust-root class for a key.
    #[must_use]
    pub fn trust_root_class(
        &self,
        signer_id: &ProtocolId,
        key_id: &ProtocolId,
    ) -> Option<AssuranceRootClassV1> {
        self.get(signer_id, key_id).map(|key| key.trust_root_class)
    }

    fn get(
        &self,
        signer_id: &ProtocolId,
        key_id: &ProtocolId,
    ) -> Option<&RegisteredVerificationKeyV1> {
        self.keys.get(&(signer_id.clone(), key_id.clone()))
    }
}

/// Signature over one assurance-bound subject digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedStatementV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Typed subject label.
    pub subject_kind: ProtocolId,
    /// Exact subject digest.
    pub subject_digest: Sha256Digest,
    /// Assurance envelope bound by the statement.
    pub assurance_envelope_digest: Sha256Digest,
    /// Issuance time.
    pub issued_at: UtcTimestamp,
    /// Expiry time.
    pub expires_at: UtcTimestamp,
    /// Monotonic logical sequence chosen by the signer.
    pub sequence: u64,
    /// Signer identity.
    pub signer_id: ProtocolId,
    /// Verification key identity.
    pub key_id: ProtocolId,
    /// Fixed algorithm identifier.
    pub algorithm: String,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

#[derive(Serialize)]
struct SigningMessage<'a> {
    schema_version: &'a str,
    subject_kind: &'a ProtocolId,
    subject_digest: Sha256Digest,
    assurance_envelope_digest: Sha256Digest,
    issued_at: &'a UtcTimestamp,
    expires_at: &'a UtcTimestamp,
    sequence: u64,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl SignedStatementV1 {
    /// Return the canonical signing message, excluding the signature field.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, SignatureError> {
        let message = SigningMessage {
            schema_version: &self.schema_version,
            subject_kind: &self.subject_kind,
            subject_digest: self.subject_digest,
            assurance_envelope_digest: self.assurance_envelope_digest,
            issued_at: &self.issued_at,
            expires_at: &self.expires_at,
            sequence: self.sequence,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        };
        canonical_json(&message).map_err(SignatureError::Canonicalization)
    }
}

/// Signature verification failures.
#[derive(Debug, Error)]
pub enum SignatureError {
    /// Schema is not supported.
    #[error("unsupported signed-statement schema")]
    UnsupportedSchema,
    /// Algorithm is not supported.
    #[error("unsupported signature algorithm")]
    UnsupportedAlgorithm,
    /// Key identity already exists.
    #[error("duplicate verification key identity")]
    DuplicateKey,
    /// Key does not exist.
    #[error("verification key not found")]
    UnknownKey,
    /// Key or signature bytes are malformed.
    #[error("invalid Ed25519 key or signature encoding")]
    InvalidEncoding,
    /// Issuance or evaluation falls outside validity.
    #[error("signature or key is outside its validity window")]
    OutsideValidity,
    /// Key was revoked at or before the explicit evaluation time.
    #[error("verification key was revoked")]
    Revoked,
    /// Signature does not verify.
    #[error("signature verification failed")]
    InvalidSignature,
    /// Canonical signing bytes could not be created.
    #[error("signature canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Verify one signed statement using a caller-provided registry and time.
///
/// # Errors
///
/// Returns a closed failure for schema, key, time, encoding, or signature.
pub fn verify_signed_statement_v1(
    statement: &SignedStatementV1,
    registry: &VerificationKeyRegistry,
    evaluated_at: &UtcTimestamp,
) -> Result<(), SignatureError> {
    if statement.schema_version != "hyperintel.signed-statement.v1" {
        return Err(SignatureError::UnsupportedSchema);
    }
    let message = statement.signing_message()?;
    verify_ed25519_message_v1(
        &Ed25519MessageVerificationInputV1 {
            signer_id: &statement.signer_id,
            key_id: &statement.key_id,
            issued_at: &statement.issued_at,
            expires_at: &statement.expires_at,
            algorithm: &statement.algorithm,
            signature_hex: &statement.signature_hex,
            message: &message,
        },
        registry,
        evaluated_at,
    )
}

/// Verify a domain-specific canonical message under caller-owned Ed25519 trust.
///
/// This function does not interpret the message schema. It verifies only the supplied
/// signature, key identity, caller-assigned key record, validity, and revocation state.
///
/// # Errors
///
/// Returns a closed failure for algorithm, key, time, revocation, encoding, or signature.
pub fn verify_ed25519_message_v1(
    input: &Ed25519MessageVerificationInputV1<'_>,
    registry: &VerificationKeyRegistry,
    evaluated_at: &UtcTimestamp,
) -> Result<(), SignatureError> {
    if input.algorithm != "ed25519" {
        return Err(SignatureError::UnsupportedAlgorithm);
    }
    if input.issued_at > input.expires_at
        || evaluated_at < input.issued_at
        || evaluated_at > input.expires_at
    {
        return Err(SignatureError::OutsideValidity);
    }
    let registered = registry
        .get(input.signer_id, input.key_id)
        .ok_or(SignatureError::UnknownKey)?;
    if input.issued_at < &registered.valid_from
        || input.issued_at > &registered.valid_until
        || input.expires_at > &registered.valid_until
        || evaluated_at < &registered.valid_from
        || evaluated_at > &registered.valid_until
    {
        return Err(SignatureError::OutsideValidity);
    }
    if registered
        .revoked_at
        .as_ref()
        .is_some_and(|revoked_at| evaluated_at >= revoked_at)
    {
        return Err(SignatureError::Revoked);
    }
    let key = decode_verifying_key(&registered.public_key_hex)?;
    let bytes = HEXLOWER
        .decode(input.signature_hex.as_bytes())
        .map_err(|_| SignatureError::InvalidEncoding)?;
    let signature =
        Signature::try_from(bytes.as_slice()).map_err(|_| SignatureError::InvalidEncoding)?;
    key.verify_strict(input.message, &signature)
        .map_err(|_| SignatureError::InvalidSignature)
}

fn decode_verifying_key(encoded: &str) -> Result<VerifyingKey, SignatureError> {
    let bytes = HEXLOWER
        .decode(encoded.as_bytes())
        .map_err(|_| SignatureError::InvalidEncoding)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SignatureError::InvalidEncoding)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| SignatureError::InvalidEncoding)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer as _, SigningKey};

    use super::*;

    #[test]
    fn contract_statement_signature_detects_tampering() -> Result<(), Box<dyn std::error::Error>> {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let signer_id: ProtocolId = "provider:alpha".parse()?;
        let key_id: ProtocolId = "key:alpha-1".parse()?;
        let mut registry = VerificationKeyRegistry::default();
        let registered_key = RegisteredVerificationKeyV1 {
            signer_id: signer_id.clone(),
            key_id: key_id.clone(),
            trust_root_class: AssuranceRootClassV1::ProviderKey,
            public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
            valid_from: "2026-07-20T00:00:00Z".parse()?,
            valid_until: "2026-07-22T00:00:00Z".parse()?,
            revoked_at: None,
        };
        registry.register(registered_key.clone())?;
        let mut statement = SignedStatementV1 {
            schema_version: "hyperintel.signed-statement.v1".to_owned(),
            subject_kind: "provider-identity".parse()?,
            subject_digest: Sha256Digest::from_bytes([1; 32]),
            assurance_envelope_digest: Sha256Digest::from_bytes([2; 32]),
            issued_at: "2026-07-20T10:00:00Z".parse()?,
            expires_at: "2026-07-21T10:00:00Z".parse()?,
            sequence: 1,
            signer_id,
            key_id,
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        statement.signature_hex =
            HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
        let evaluated_at = "2026-07-20T12:00:00Z".parse()?;
        let mut replacement_key = registered_key.clone();
        replacement_key.public_key_hex =
            HEXLOWER.encode(SigningKey::from_bytes(&[9; 32]).verifying_key().as_bytes());
        assert!(matches!(
            registry.register(replacement_key),
            Err(SignatureError::DuplicateKey)
        ));
        verify_signed_statement_v1(&statement, &registry, &evaluated_at)?;
        let mut revoked_registry = VerificationKeyRegistry::default();
        let mut revoked_key = registered_key;
        revoked_key.revoked_at = Some("2026-07-20T11:00:00Z".parse()?);
        revoked_registry.register(revoked_key)?;
        assert!(matches!(
            verify_signed_statement_v1(&statement, &revoked_registry, &evaluated_at),
            Err(SignatureError::Revoked)
        ));
        let mut expired_registry = VerificationKeyRegistry::default();
        let mut expired_key = revoked_registry
            .get(&statement.signer_id, &statement.key_id)
            .ok_or("revoked key missing")?
            .clone();
        expired_key.revoked_at = None;
        expired_key.valid_until = "2026-07-20T11:00:00Z".parse()?;
        expired_registry.register(expired_key)?;
        assert!(matches!(
            verify_signed_statement_v1(&statement, &expired_registry, &evaluated_at),
            Err(SignatureError::OutsideValidity)
        ));
        statement.subject_digest = Sha256Digest::from_bytes([3; 32]);
        assert!(matches!(
            verify_signed_statement_v1(&statement, &registry, &evaluated_at),
            Err(SignatureError::InvalidSignature)
        ));
        Ok(())
    }
}
