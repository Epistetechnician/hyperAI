use std::fmt;

use hyperintel_core::{ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_runtime_config::SecretReferenceV1;

use crate::authorization::verify_operator_secret_authorization_v1;
use crate::{
    OperatorSecretAuthorizationVerificationContextV1, SecretMaterialV1, SecretProviderError,
    SecretPurposeV1, SignedOperatorSecretAuthorizationV1,
};

/// Closed failure returned by an injected fixture resolver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InjectedSecretResolverErrorV1 {
    /// The injected resolver did not return material.
    Unavailable,
}

/// Exact nonsecret request passed once to an injected in-process resolver.
pub struct SecretResolutionRequestV1 {
    runtime_configuration_digest: Sha256Digest,
    consumer_id: ProtocolId,
    selected_provider_id: ProtocolId,
    secret_reference: SecretReferenceV1,
    purpose: SecretPurposeV1,
    operation_context_digest: Sha256Digest,
    maximum_secret_bytes: u32,
    evaluated_at: UtcTimestamp,
    authorization_digest: Sha256Digest,
    signing_key_record_digest: Sha256Digest,
    revocation_state_digest: Sha256Digest,
    revocation_checkpoint_digest: Sha256Digest,
}

impl SecretResolutionRequestV1 {
    /// Return the exact runtime-configuration identity.
    #[must_use]
    pub const fn runtime_configuration_digest(&self) -> Sha256Digest {
        self.runtime_configuration_digest
    }

    /// Return the exact material consumer.
    #[must_use]
    pub const fn consumer_id(&self) -> &ProtocolId {
        &self.consumer_id
    }

    /// Return the selected inference provider.
    #[must_use]
    pub const fn selected_provider_id(&self) -> &ProtocolId {
        &self.selected_provider_id
    }

    /// Return the immutable-version reference.
    #[must_use]
    pub const fn secret_reference(&self) -> &SecretReferenceV1 {
        &self.secret_reference
    }

    /// Return the closed resolution purpose.
    #[must_use]
    pub const fn purpose(&self) -> SecretPurposeV1 {
        self.purpose
    }

    /// Return the exact operation-context identity.
    #[must_use]
    pub const fn operation_context_digest(&self) -> Sha256Digest {
        self.operation_context_digest
    }

    /// Return the signed maximum returned byte count.
    #[must_use]
    pub const fn maximum_secret_bytes(&self) -> u32 {
        self.maximum_secret_bytes
    }

    /// Return the explicit verification time.
    #[must_use]
    pub const fn evaluated_at(&self) -> &UtcTimestamp {
        &self.evaluated_at
    }

    /// Return the complete signed-authorization identity.
    #[must_use]
    pub const fn authorization_digest(&self) -> Sha256Digest {
        self.authorization_digest
    }

    /// Return the exact trusted signing-key record identity.
    #[must_use]
    pub const fn signing_key_record_digest(&self) -> Sha256Digest {
        self.signing_key_record_digest
    }

    /// Return the exact caller-owned revocation-state identity.
    #[must_use]
    pub const fn revocation_state_digest(&self) -> Sha256Digest {
        self.revocation_state_digest
    }

    /// Return the exact caller-pinned revocation-checkpoint identity.
    #[must_use]
    pub const fn revocation_checkpoint_digest(&self) -> Sha256Digest {
        self.revocation_checkpoint_digest
    }
}

impl fmt::Debug for SecretResolutionRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretResolutionRequestV1")
            .field("runtime_configuration_digest", &"[REDACTED]")
            .field("consumer_id", &"[REDACTED]")
            .field("selected_provider_id", &"[REDACTED]")
            .field("secret_reference", &"[REDACTED]")
            .field("purpose", &self.purpose)
            .field("operation_context_digest", &"[REDACTED]")
            .field("authorization_digest", &"[REDACTED]")
            .field("signing_key_record_digest", &"[REDACTED]")
            .field("revocation_state_digest", &"[REDACTED]")
            .field("revocation_checkpoint_digest", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Resolver-produced candidate carrying exact metadata and owned material.
pub struct ResolvedSecretCandidateV1 {
    provider_id: ProtocolId,
    handle_id: ProtocolId,
    version_digest: Sha256Digest,
    purpose: SecretPurposeV1,
    material: SecretMaterialV1,
}

impl ResolvedSecretCandidateV1 {
    /// Construct a bounded candidate returned by an injected resolver.
    ///
    /// Secret bytes are wrapped before validation and zeroized on every
    /// rejected path.
    ///
    /// # Errors
    ///
    /// Returns when material is empty or exceeds the global bound.
    pub fn new(
        provider_id: ProtocolId,
        handle_id: ProtocolId,
        version_digest: Sha256Digest,
        purpose: SecretPurposeV1,
        bytes: Vec<u8>,
    ) -> Result<Self, SecretProviderError> {
        Ok(Self {
            provider_id,
            handle_id,
            version_digest,
            purpose,
            material: SecretMaterialV1::new(bytes)?,
        })
    }
}

impl fmt::Debug for ResolvedSecretCandidateV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedSecretCandidateV1")
            .field("provider_id", &self.provider_id)
            .field("handle_id", &"[REDACTED]")
            .field("version_digest", &"[REDACTED]")
            .field("purpose", &self.purpose)
            .field("material", &"[REDACTED]")
            .finish()
    }
}

/// Synchronous injected in-process resolver port.
pub trait InjectedSecretResolverV1 {
    /// Secret-backend identity implemented by this resolver.
    fn provider_id(&self) -> &ProtocolId;

    /// Resolve exactly one already verified request.
    ///
    /// # Errors
    ///
    /// Returns only a closed failure without arbitrary text.
    fn resolve(
        &mut self,
        request: &SecretResolutionRequestV1,
    ) -> Result<ResolvedSecretCandidateV1, InjectedSecretResolverErrorV1>;
}

/// Successfully resolved, authorization-bound secret material.
pub struct ResolvedSecretV1 {
    secret_reference: SecretReferenceV1,
    purpose: SecretPurposeV1,
    consumer_id: ProtocolId,
    selected_provider_id: ProtocolId,
    authorization_digest: Sha256Digest,
    runtime_configuration_digest: Sha256Digest,
    operation_context_digest: Sha256Digest,
    evaluated_at: UtcTimestamp,
    signing_key_record_digest: Sha256Digest,
    revocation_state_digest: Sha256Digest,
    revocation_checkpoint_digest: Sha256Digest,
    material: SecretMaterialV1,
}

impl ResolvedSecretV1 {
    /// Return the exact immutable-version reference.
    #[must_use]
    pub const fn secret_reference(&self) -> &SecretReferenceV1 {
        &self.secret_reference
    }

    /// Return the closed purpose.
    #[must_use]
    pub const fn purpose(&self) -> SecretPurposeV1 {
        self.purpose
    }

    /// Return the exact material consumer.
    #[must_use]
    pub const fn consumer_id(&self) -> &ProtocolId {
        &self.consumer_id
    }

    /// Return the selected inference provider.
    #[must_use]
    pub const fn selected_provider_id(&self) -> &ProtocolId {
        &self.selected_provider_id
    }

    /// Return the complete signed-authorization identity.
    #[must_use]
    pub const fn authorization_digest(&self) -> Sha256Digest {
        self.authorization_digest
    }

    /// Return the exact runtime-configuration identity.
    #[must_use]
    pub const fn runtime_configuration_digest(&self) -> Sha256Digest {
        self.runtime_configuration_digest
    }

    /// Return the exact operation-context identity.
    #[must_use]
    pub const fn operation_context_digest(&self) -> Sha256Digest {
        self.operation_context_digest
    }

    /// Return the explicit authorization evaluation time.
    #[must_use]
    pub const fn evaluated_at(&self) -> &UtcTimestamp {
        &self.evaluated_at
    }

    /// Return the exact trusted signing-key record identity.
    #[must_use]
    pub const fn signing_key_record_digest(&self) -> Sha256Digest {
        self.signing_key_record_digest
    }

    /// Return the exact caller-owned revocation-state identity.
    #[must_use]
    pub const fn revocation_state_digest(&self) -> Sha256Digest {
        self.revocation_state_digest
    }

    /// Return the exact caller-pinned revocation-checkpoint identity.
    #[must_use]
    pub const fn revocation_checkpoint_digest(&self) -> Sha256Digest {
        self.revocation_checkpoint_digest
    }

    /// Explicitly borrow the resolved secret bytes.
    #[must_use]
    pub fn expose_secret_bytes(&self) -> &[u8] {
        self.material.expose_secret_bytes()
    }
}

impl fmt::Debug for ResolvedSecretV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedSecretV1")
            .field("secret_reference", &"[REDACTED]")
            .field("purpose", &self.purpose)
            .field("consumer_id", &"[REDACTED]")
            .field("selected_provider_id", &"[REDACTED]")
            .field("authorization_digest", &"[REDACTED]")
            .field("runtime_configuration_digest", &"[REDACTED]")
            .field("operation_context_digest", &"[REDACTED]")
            .field("evaluated_at", &self.evaluated_at)
            .field("signing_key_record_digest", &"[REDACTED]")
            .field("revocation_state_digest", &"[REDACTED]")
            .field("revocation_checkpoint_digest", &"[REDACTED]")
            .field("material", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Atomically verify one signed authorization and invoke one injected resolver.
///
/// Every preflight check completes before resolver invocation. A valid call
/// invokes the resolver exactly once, performs no retry or fallback, and then
/// validates provider, handle, version, purpose, and byte-limit bindings.
/// Reusing the same signed authorization in a later call is not durably
/// prevented by this local slice.
///
/// # Errors
///
/// Returns before invocation for authorization or resolver-identity failures,
/// or after the single invocation for closed resolver and response failures.
pub fn resolve_authorized_secret_v1<R: InjectedSecretResolverV1>(
    resolver: &mut R,
    signed: &SignedOperatorSecretAuthorizationV1,
    context: &OperatorSecretAuthorizationVerificationContextV1<'_>,
) -> Result<ResolvedSecretV1, SecretProviderError> {
    let verified = verify_operator_secret_authorization_v1(signed, context)?;
    if resolver.provider_id() != verified.secret_reference.provider_id() {
        return Err(SecretProviderError::ResolverBindingMismatch);
    }
    let request = SecretResolutionRequestV1 {
        runtime_configuration_digest: verified.runtime_configuration_digest,
        consumer_id: verified.consumer_id.clone(),
        selected_provider_id: verified.selected_provider_id.clone(),
        secret_reference: verified.secret_reference.clone(),
        purpose: verified.purpose,
        operation_context_digest: verified.operation_context_digest,
        maximum_secret_bytes: verified.maximum_secret_bytes,
        evaluated_at: verified.evaluated_at,
        authorization_digest: verified.authorization_digest,
        signing_key_record_digest: verified.signing_key_record_digest,
        revocation_state_digest: verified.revocation_state_digest,
        revocation_checkpoint_digest: verified.revocation_checkpoint_digest,
    };
    let candidate = resolver
        .resolve(&request)
        .map_err(|_| SecretProviderError::ResolverUnavailable)?;
    if candidate.provider_id != *request.secret_reference.provider_id()
        || candidate.handle_id != *request.secret_reference.handle_id()
        || candidate.version_digest != request.secret_reference.version_digest()
        || candidate.purpose != request.purpose
    {
        return Err(SecretProviderError::ResolverBindingMismatch);
    }
    let maximum_secret_bytes = usize::try_from(request.maximum_secret_bytes)
        .map_err(|_| SecretProviderError::InvalidMaterial)?;
    if candidate.material.len() > maximum_secret_bytes {
        return Err(SecretProviderError::InvalidMaterial);
    }
    Ok(ResolvedSecretV1 {
        secret_reference: request.secret_reference,
        purpose: request.purpose,
        consumer_id: request.consumer_id,
        selected_provider_id: request.selected_provider_id,
        authorization_digest: request.authorization_digest,
        runtime_configuration_digest: request.runtime_configuration_digest,
        operation_context_digest: request.operation_context_digest,
        evaluated_at: request.evaluated_at,
        signing_key_record_digest: request.signing_key_record_digest,
        revocation_state_digest: request.revocation_state_digest,
        revocation_checkpoint_digest: request.revocation_checkpoint_digest,
        material: candidate.material,
    })
}
