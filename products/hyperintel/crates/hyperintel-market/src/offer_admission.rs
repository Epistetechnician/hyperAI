use hyperintel_assurance::{
    AssuranceRootClassV1, SignatureError, SignedStatementV1, VerificationKeyRegistry,
    verify_signed_statement_v1,
};
use hyperintel_core::{
    CanonicalizationError, CapacityOfferV1, ProviderIdentityV1, Sha256Digest, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A capacity offer and the provider statement authenticating its exact digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedCapacityOfferV1 {
    /// Fixed wrapper schema identifier.
    pub schema_version: String,
    /// Canonical inference-capacity declaration.
    pub offer: CapacityOfferV1,
    /// Provider signature over the exact canonical offer digest.
    pub statement: SignedStatementV1,
}

/// Fail-closed signed-offer admission failures.
#[derive(Debug, Error)]
pub enum OfferAdmissionError {
    /// Wrapper or offer schema is unsupported.
    #[error("unsupported signed capacity-offer schema")]
    UnsupportedSchema,
    /// The signature does not bind the canonical offer digest.
    #[error("capacity-offer signature subject does not match the offer")]
    SubjectMismatch,
    /// The provider or configured offer-signing key does not match.
    #[error("capacity-offer signature signer or key does not match the provider")]
    SignerMismatch,
    /// The caller did not classify the configured key as a provider key.
    #[error("capacity-offer signature key is not a caller-trusted provider key")]
    TrustRootMismatch,
    /// The signature is not bound to the verified provider-authorization envelope.
    #[error("capacity-offer signature assurance binding does not match")]
    AssuranceBindingMismatch,
    /// The logical sequence is not usable for signed-offer equivocation checks.
    #[error("capacity-offer signature sequence must be nonzero")]
    InvalidSequence,
    /// The statement validity is not contained by both offer and provider validity.
    #[error("capacity-offer signature validity exceeds offer or provider validity")]
    ValidityNotContained,
    /// Canonical offer identity could not be constructed.
    #[error("capacity-offer canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Ed25519, key-validity, or revocation verification failed.
    #[error("capacity-offer signature verification failed: {0}")]
    Signature(#[from] SignatureError),
}

/// Verify a signed offer against caller-owned trust and a verified provider envelope.
///
/// This proves only that the configured provider key signed the exact offer bytes under
/// the supplied validity and trust assumptions. It does not prove capacity, reliability,
/// service delivery, external authorization, or production authority.
///
/// # Errors
///
/// Returns closed failure for schema, subject, signer, key, trust class, assurance
/// binding, sequence, validity containment, canonicalization, or signature failure.
pub fn verify_signed_capacity_offer_v1(
    signed_offer: &SignedCapacityOfferV1,
    provider: &ProviderIdentityV1,
    provider_assurance_envelope_digest: Sha256Digest,
    registry: &VerificationKeyRegistry,
    evaluated_at: &UtcTimestamp,
) -> Result<(), OfferAdmissionError> {
    if signed_offer.schema_version != "hyperintel.signed-capacity-offer.v1"
        || signed_offer.offer.schema_version != "hyperintel.capacity-offer.v1"
    {
        return Err(OfferAdmissionError::UnsupportedSchema);
    }
    let offer_digest = signed_offer.offer.digest()?;
    let statement = &signed_offer.statement;
    if statement.subject_kind.as_str() != "capacity-offer"
        || statement.subject_digest != offer_digest
    {
        return Err(OfferAdmissionError::SubjectMismatch);
    }
    if statement.signer_id != signed_offer.offer.provider_id
        || statement.signer_id != provider.provider_id
        || statement.key_id != provider.offer_key_id
    {
        return Err(OfferAdmissionError::SignerMismatch);
    }
    if registry.trust_root_class(&statement.signer_id, &statement.key_id)
        != Some(AssuranceRootClassV1::ProviderKey)
    {
        return Err(OfferAdmissionError::TrustRootMismatch);
    }
    if provider_assurance_envelope_digest.is_zero()
        || statement.assurance_envelope_digest != provider_assurance_envelope_digest
    {
        return Err(OfferAdmissionError::AssuranceBindingMismatch);
    }
    if statement.sequence == 0 {
        return Err(OfferAdmissionError::InvalidSequence);
    }
    if statement.issued_at < signed_offer.offer.valid_from
        || statement.expires_at > signed_offer.offer.valid_until
        || statement.issued_at < provider.valid_from
        || statement.expires_at > provider.valid_until
    {
        return Err(OfferAdmissionError::ValidityNotContained);
    }
    verify_signed_statement_v1(statement, registry, evaluated_at)?;
    Ok(())
}
