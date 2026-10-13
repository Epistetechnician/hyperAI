use std::{collections::BTreeMap, fmt};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signature, VerifyingKey};
use hyperintel_core::{
    ProtocolId, Sha256Digest, UtcTimestamp, canonical_json, domain_separated_digest,
};
use serde::{Deserialize, Serialize};

use crate::model::interval_contains;
use crate::{
    ED25519_ALGORITHM_V1, MAX_ACCESS_GRANT_LIFETIME_SECONDS,
    MAX_REVOCATION_SNAPSHOT_LIFETIME_SECONDS, SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1,
    SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1, TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1,
    TENANT_IAM_KEY_SCHEMA_V1, TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1, TenantIamError,
    TenantLifecycleV1, TenantResourceBindingV1, TenantRoleV1, ValidatedTenantIamSnapshotV1,
    validate_resource_binding_v1,
};

const ACCESS_GRANT_STATEMENT_DOMAIN_V1: &str = "hyperintel.tenant-access-grant-statement.v1";
const ACCESS_GRANT_DIGEST_DOMAIN_V1: &str = "hyperintel.tenant-access-grant-digest.v1";
const REVOCATION_STATEMENT_DOMAIN_V1: &str = "hyperintel.tenant-revocation-snapshot-statement.v1";
const REVOCATION_DIGEST_DOMAIN_V1: &str = "hyperintel.tenant-revocation-snapshot-digest.v1";
const IAM_KEY_RECORD_DIGEST_DOMAIN_V1: &str = "hyperintel.tenant-iam-key-record.v1";

/// Closed purpose for a tenant IAM verification key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantIamKeyPurposeV1 {
    /// Verify one-action access grants.
    AccessGrant,
    /// Verify revocation snapshots.
    RevocationSnapshot,
}

/// Closed one-action tenant IAM permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantIamActionV1 {
    /// Submit one inference request.
    SubmitInference,
    /// Cancel one principal-owned request.
    CancelOwnInference,
    /// Read one principal-owned request.
    ReadOwnInference,
    /// Read one principal-owned evidence bundle.
    ReadOwnEvidence,
    /// Read structural tenant audit metadata.
    ReadTenantAudit,
    /// Administer the tenant principal directory.
    ManageTenantPrincipals,
    /// Administer the tenant IAM key registry.
    ManageTenantTokenKeys,
}

/// One caller-trusted tenant IAM verification key.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredTenantIamKeyV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Tenant namespace.
    pub tenant_id: ProtocolId,
    /// Exact IAM issuer.
    pub issuer_id: ProtocolId,
    /// Key identity within tenant, issuer, and purpose.
    pub key_id: ProtocolId,
    /// Closed statement purpose.
    pub purpose: TenantIamKeyPurposeV1,
    /// Lowercase hexadecimal Ed25519 public key.
    pub public_key_hex: String,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive local revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

impl fmt::Debug for RegisteredTenantIamKeyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredTenantIamKeyV1")
            .field("schema_version", &self.schema_version)
            .field("tenant_id", &self.tenant_id)
            .field("issuer_id", &self.issuer_id)
            .field("key_id", &self.key_id)
            .field("purpose", &self.purpose)
            .field("public_key_hex", &"[REDACTED]")
            .field("valid_from", &self.valid_from)
            .field("valid_until", &self.valid_until)
            .field("revoked_at", &self.revoked_at)
            .finish()
    }
}

/// Caller-owned, tenant-namespaced IAM key registry.
#[derive(Clone, Debug, Default)]
pub struct TenantIamKeyRegistryV1 {
    keys: BTreeMap<
        (ProtocolId, ProtocolId, ProtocolId, TenantIamKeyPurposeV1),
        RegisteredTenantIamKeyV1,
    >,
}

impl TenantIamKeyRegistryV1 {
    /// Register one exact key without overwrite or purpose fallback.
    ///
    /// # Errors
    ///
    /// Returns for schema, interval, encoding, or duplicate failures.
    pub fn register(&mut self, key: RegisteredTenantIamKeyV1) -> Result<(), TenantIamError> {
        if key.schema_version != TENANT_IAM_KEY_SCHEMA_V1 {
            return Err(TenantIamError::UnsupportedSchema);
        }
        validate_interval_and_lifetime(&key.valid_from, &key.valid_until, None)?;
        if key
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| revoked_at < &key.valid_from)
        {
            return Err(TenantIamError::InvalidValidity);
        }
        decode_verifying_key(&key.public_key_hex)?;
        let identity = (
            key.tenant_id.clone(),
            key.issuer_id.clone(),
            key.key_id.clone(),
            key.purpose,
        );
        if self.keys.contains_key(&identity) {
            return Err(TenantIamError::DuplicateIdentity);
        }
        self.keys.insert(identity, key);
        Ok(())
    }

    fn get(
        &self,
        tenant_id: &ProtocolId,
        issuer_id: &ProtocolId,
        key_id: &ProtocolId,
        purpose: TenantIamKeyPurposeV1,
    ) -> Option<&RegisteredTenantIamKeyV1> {
        self.keys.get(&(
            tenant_id.clone(),
            issuer_id.clone(),
            key_id.clone(),
            purpose,
        ))
    }
}

/// Exact claims for one short-lived tenant action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantAccessGrantClaimsV1 {
    /// Fixed claims schema.
    pub schema_version: String,
    /// Nonzero logical grant sequence.
    pub sequence: u64,
    /// Tenant namespace.
    pub tenant_id: ProtocolId,
    /// Principal receiving the grant.
    pub principal_id: ProtocolId,
    /// IAM issuer identity.
    pub issuer_id: ProtocolId,
    /// Intended local gateway audience.
    pub audience: ProtocolId,
    /// Exact validated runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Exact caller-owned IAM snapshot identity.
    pub iam_snapshot_digest: Sha256Digest,
    /// One closed action.
    pub action: TenantIamActionV1,
    /// Exact caller-owned resource binding.
    pub resource: TenantResourceBindingV1,
    /// Exact high-risk operation context.
    pub operation_context_digest: Sha256Digest,
    /// Issuance time.
    pub issued_at: UtcTimestamp,
    /// Expiration time.
    pub expires_at: UtcTimestamp,
}

/// Detached Ed25519 signature over exact access-grant claims.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedTenantAccessGrantV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact signed claims.
    pub claims: TenantAccessGrantClaimsV1,
    /// Must equal the claims issuer.
    pub signer_id: ProtocolId,
    /// Exact caller-trusted key identity.
    pub key_id: ProtocolId,
    /// Fixed Ed25519 label.
    pub algorithm: String,
    /// Lowercase hexadecimal detached signature.
    pub signature_hex: String,
}

impl fmt::Debug for SignedTenantAccessGrantV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedTenantAccessGrantV1")
            .field("schema_version", &self.schema_version)
            .field("tenant_id", &self.claims.tenant_id)
            .field("issuer_id", &self.claims.issuer_id)
            .field("principal_id", &"[REDACTED]")
            .field("action", &self.claims.action)
            .field("resource", &"[REDACTED]")
            .field("operation_context_digest", &"[REDACTED]")
            .field("key_id", &self.key_id)
            .field("signature_hex", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Serialize)]
struct AccessGrantSigningMessage<'a> {
    statement_domain: &'static str,
    wrapper_schema: &'a str,
    claims: &'a TenantAccessGrantClaimsV1,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl SignedTenantAccessGrantV1 {
    /// Return canonical domain-bound signing bytes, excluding only the signature.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, TenantIamError> {
        Ok(canonical_json(&AccessGrantSigningMessage {
            statement_domain: ACCESS_GRANT_STATEMENT_DOMAIN_V1,
            wrapper_schema: &self.schema_version,
            claims: &self.claims,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        })?)
    }

    /// Return the canonical identity of the complete signed grant.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, TenantIamError> {
        Ok(domain_separated_digest(
            ACCESS_GRANT_DIGEST_DOMAIN_V1,
            self,
        )?)
    }
}

/// One exact signed-grant revocation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantGrantRevocationV1 {
    /// Tenant namespace.
    pub tenant_id: ProtocolId,
    /// Exact complete signed-grant digest.
    pub grant_digest: Sha256Digest,
    /// Inclusive revocation instant.
    pub revoked_at: UtcTimestamp,
}

/// One exact tenant IAM key revocation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantKeyRevocationV1 {
    /// Tenant namespace.
    pub tenant_id: ProtocolId,
    /// IAM issuer identity.
    pub issuer_id: ProtocolId,
    /// Exact key identity.
    pub key_id: ProtocolId,
    /// Exact statement purpose.
    pub purpose: TenantIamKeyPurposeV1,
    /// Inclusive revocation instant.
    pub revoked_at: UtcTimestamp,
}

/// Signed, freshness-bounded revocation state for one tenant and runtime audience.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantRevocationSnapshotV1 {
    /// Fixed snapshot schema.
    pub schema_version: String,
    /// Tenant namespace.
    pub tenant_id: ProtocolId,
    /// IAM issuer identity.
    pub issuer_id: ProtocolId,
    /// Intended local gateway audience.
    pub audience: ProtocolId,
    /// Exact runtime-configuration identity.
    pub runtime_configuration_digest: Sha256Digest,
    /// Nonzero monotonic sequence.
    pub sequence: u64,
    /// Snapshot signing time.
    pub issued_at: UtcTimestamp,
    /// First evaluation instant represented by the snapshot.
    pub effective_at: UtcTimestamp,
    /// Last evaluation instant represented by the snapshot.
    pub expires_at: UtcTimestamp,
    /// Strictly sorted exact grant revocations.
    pub grant_revocations: Vec<TenantGrantRevocationV1>,
    /// Strictly sorted exact key revocations.
    pub key_revocations: Vec<TenantKeyRevocationV1>,
}

/// Detached Ed25519 signature over one exact revocation snapshot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedTenantRevocationSnapshotV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact signed snapshot.
    pub snapshot: TenantRevocationSnapshotV1,
    /// Must equal the snapshot issuer.
    pub signer_id: ProtocolId,
    /// Exact revocation-purpose key.
    pub key_id: ProtocolId,
    /// Fixed Ed25519 label.
    pub algorithm: String,
    /// Lowercase hexadecimal detached signature.
    pub signature_hex: String,
}

impl fmt::Debug for SignedTenantRevocationSnapshotV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedTenantRevocationSnapshotV1")
            .field("schema_version", &self.schema_version)
            .field("tenant_id", &self.snapshot.tenant_id)
            .field("issuer_id", &self.snapshot.issuer_id)
            .field("sequence", &self.snapshot.sequence)
            .field("key_id", &self.key_id)
            .field("revocations", &"[REDACTED]")
            .field("signature_hex", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Serialize)]
struct RevocationSigningMessage<'a> {
    statement_domain: &'static str,
    wrapper_schema: &'a str,
    snapshot: &'a TenantRevocationSnapshotV1,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl SignedTenantRevocationSnapshotV1 {
    /// Return canonical domain-bound signing bytes.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, TenantIamError> {
        Ok(canonical_json(&RevocationSigningMessage {
            statement_domain: REVOCATION_STATEMENT_DOMAIN_V1,
            wrapper_schema: &self.schema_version,
            snapshot: &self.snapshot,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        })?)
    }

    /// Return the canonical identity of the complete signed snapshot.
    ///
    /// # Errors
    ///
    /// Returns when canonicalization fails.
    pub fn digest(&self) -> Result<Sha256Digest, TenantIamError> {
        Ok(domain_separated_digest(REVOCATION_DIGEST_DOMAIN_V1, self)?)
    }
}

/// Caller-pinned lower bound against revocation rollback and same-sequence forks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenantRevocationCheckpointV1 {
    /// Exact tenant namespace.
    pub tenant_id: ProtocolId,
    /// Exact IAM issuer.
    pub issuer_id: ProtocolId,
    /// Exact audience.
    pub audience: ProtocolId,
    /// Exact runtime configuration.
    pub runtime_configuration_digest: Sha256Digest,
    /// Minimum acceptable monotonic sequence.
    pub minimum_sequence: u64,
    /// Digest required when the supplied snapshot has the minimum sequence.
    pub digest_at_minimum_sequence: Sha256Digest,
}

/// Inputs required to authenticate a revocation snapshot.
pub struct TenantRevocationVerificationContextV1<'a> {
    /// Dedicated caller-owned IAM key registry.
    pub key_registry: &'a TenantIamKeyRegistryV1,
    /// Caller-pinned rollback checkpoint.
    pub checkpoint: &'a TenantRevocationCheckpointV1,
    /// Explicit evaluation time.
    pub evaluated_at: &'a UtcTimestamp,
}

/// Authenticated, freshness-bounded revocation state.
#[derive(Clone)]
pub struct ValidatedTenantRevocationSnapshotV1 {
    snapshot: TenantRevocationSnapshotV1,
    snapshot_digest: Sha256Digest,
    evaluated_at: UtcTimestamp,
    signer_key_id: ProtocolId,
    signer_key_record_digest: Sha256Digest,
}

impl fmt::Debug for ValidatedTenantRevocationSnapshotV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedTenantRevocationSnapshotV1")
            .field("snapshot", &"[REDACTED]")
            .field("snapshot_digest", &"[REDACTED]")
            .field("evaluated_at", &self.evaluated_at)
            .field("signer_key_id", &self.signer_key_id)
            .field("signer_key_record_digest", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl ValidatedTenantRevocationSnapshotV1 {
    /// Return the authenticated snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &TenantRevocationSnapshotV1 {
        &self.snapshot
    }

    /// Return the authenticated snapshot digest.
    #[must_use]
    pub const fn snapshot_digest(&self) -> Sha256Digest {
        self.snapshot_digest
    }
}

/// Authenticate and validate one signed revocation snapshot.
///
/// # Errors
///
/// Returns a closed failure for schema, binding, freshness, rollback, key, or signature errors.
pub fn verify_tenant_revocation_snapshot_v1(
    signed: &SignedTenantRevocationSnapshotV1,
    context: &TenantRevocationVerificationContextV1<'_>,
) -> Result<ValidatedTenantRevocationSnapshotV1, TenantIamError> {
    if signed.schema_version != SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1
        || signed.snapshot.schema_version != TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1
    {
        return Err(TenantIamError::UnsupportedSchema);
    }
    if signed.algorithm != ED25519_ALGORITHM_V1 {
        return Err(TenantIamError::UnsupportedAlgorithm);
    }
    let snapshot = &signed.snapshot;
    if snapshot.sequence == 0 || snapshot.runtime_configuration_digest.is_zero() {
        return Err(TenantIamError::ZeroDigest);
    }
    if context.checkpoint.minimum_sequence == 0
        || context.checkpoint.digest_at_minimum_sequence.is_zero()
    {
        return Err(TenantIamError::ZeroDigest);
    }
    if signed.signer_id != snapshot.issuer_id
        || snapshot.tenant_id != context.checkpoint.tenant_id
        || snapshot.issuer_id != context.checkpoint.issuer_id
        || snapshot.audience != context.checkpoint.audience
        || snapshot.runtime_configuration_digest != context.checkpoint.runtime_configuration_digest
    {
        return Err(TenantIamError::ExpectationMismatch);
    }
    validate_interval_and_lifetime(
        &snapshot.issued_at,
        &snapshot.expires_at,
        Some(MAX_REVOCATION_SNAPSHOT_LIFETIME_SECONDS),
    )?;
    if snapshot.effective_at < snapshot.issued_at
        || snapshot.effective_at > snapshot.expires_at
        || context.evaluated_at < &snapshot.effective_at
        || context.evaluated_at > &snapshot.expires_at
    {
        return Err(TenantIamError::InvalidValidity);
    }
    validate_sorted_revocations(snapshot)?;
    let snapshot_digest = signed.digest()?;
    if snapshot.sequence < context.checkpoint.minimum_sequence {
        return Err(TenantIamError::RevocationRollback);
    }
    if snapshot.sequence == context.checkpoint.minimum_sequence
        && snapshot_digest != context.checkpoint.digest_at_minimum_sequence
    {
        return Err(TenantIamError::RevocationFork);
    }
    let key = context
        .key_registry
        .get(
            &snapshot.tenant_id,
            &snapshot.issuer_id,
            &signed.key_id,
            TenantIamKeyPurposeV1::RevocationSnapshot,
        )
        .ok_or(TenantIamError::UnknownKey)?;
    let signer_key_record_digest = domain_separated_digest(IAM_KEY_RECORD_DIGEST_DOMAIN_V1, key)?;
    verify_key_window(
        key,
        &snapshot.issued_at,
        &snapshot.expires_at,
        context.evaluated_at,
    )?;
    verify_signature(
        &key.public_key_hex,
        &signed.signature_hex,
        &signed.signing_message()?,
    )?;
    if snapshot.key_revocations.iter().any(|revocation| {
        revocation.tenant_id == snapshot.tenant_id
            && revocation.issuer_id == snapshot.issuer_id
            && revocation.key_id == signed.key_id
            && revocation.purpose == TenantIamKeyPurposeV1::RevocationSnapshot
            && context.evaluated_at >= &revocation.revoked_at
    }) {
        return Err(TenantIamError::GrantOrKeyRevoked);
    }
    Ok(ValidatedTenantRevocationSnapshotV1 {
        snapshot: snapshot.clone(),
        snapshot_digest,
        evaluated_at: context.evaluated_at.clone(),
        signer_key_id: signed.key_id.clone(),
        signer_key_record_digest,
    })
}

/// Exact caller-owned context required for an access decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenantAccessExpectationV1 {
    /// Expected tenant.
    pub tenant_id: ProtocolId,
    /// Expected principal.
    pub principal_id: ProtocolId,
    /// Expected IAM issuer.
    pub issuer_id: ProtocolId,
    /// Expected audience.
    pub audience: ProtocolId,
    /// Expected runtime configuration.
    pub runtime_configuration_digest: Sha256Digest,
    /// Expected IAM policy snapshot.
    pub iam_snapshot_digest: Sha256Digest,
    /// Expected one action.
    pub action: TenantIamActionV1,
    /// Expected caller-owned resource binding.
    pub resource: TenantResourceBindingV1,
    /// Expected operation context.
    pub operation_context_digest: Sha256Digest,
}

/// Inputs required for one access-grant decision.
pub struct TenantAccessVerificationContextV1<'a> {
    /// Validated caller-owned tenant policy.
    pub iam_snapshot: &'a ValidatedTenantIamSnapshotV1,
    /// Dedicated caller-owned IAM key registry.
    pub key_registry: &'a TenantIamKeyRegistryV1,
    /// Authenticated current revocation evidence.
    pub revocation_snapshot: &'a ValidatedTenantRevocationSnapshotV1,
    /// Current caller-pinned revocation rollback checkpoint.
    pub revocation_checkpoint: &'a TenantRevocationCheckpointV1,
    /// Exact expected operation context.
    pub expectation: &'a TenantAccessExpectationV1,
    /// Explicit caller-supplied evaluation time.
    pub evaluated_at: &'a UtcTimestamp,
}

/// Opaque non-authoritative result constructible only by successful verification.
#[derive(Clone)]
pub struct VerifiedTenantAccessV1 {
    grant_digest: Sha256Digest,
    iam_snapshot_digest: Sha256Digest,
    revocation_snapshot_digest: Sha256Digest,
    tenant_id: ProtocolId,
    principal_id: ProtocolId,
    audience: ProtocolId,
    runtime_configuration_digest: Sha256Digest,
    action: TenantIamActionV1,
    resource_digest: Sha256Digest,
    operation_context_digest: Sha256Digest,
    evaluated_at: UtcTimestamp,
}

impl fmt::Debug for VerifiedTenantAccessV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedTenantAccessV1")
            .field("tenant_id", &self.tenant_id)
            .field("principal_id", &"[REDACTED]")
            .field("action", &self.action)
            .field("grant_digest", &"[REDACTED]")
            .field("resource_digest", &"[REDACTED]")
            .field("operation_context_digest", &"[REDACTED]")
            .field("evaluated_at", &self.evaluated_at)
            .finish_non_exhaustive()
    }
}

impl VerifiedTenantAccessV1 {
    /// Return the exact tenant identity.
    #[must_use]
    pub const fn tenant_id(&self) -> &ProtocolId {
        &self.tenant_id
    }

    /// Return the exact principal identity.
    #[must_use]
    pub const fn principal_id(&self) -> &ProtocolId {
        &self.principal_id
    }

    /// Return the one verified action.
    #[must_use]
    pub const fn action(&self) -> TenantIamActionV1 {
        self.action
    }

    /// Return the complete signed-grant digest.
    #[must_use]
    pub const fn grant_digest(&self) -> Sha256Digest {
        self.grant_digest
    }

    /// Return the caller-owned IAM snapshot digest.
    #[must_use]
    pub const fn iam_snapshot_digest(&self) -> Sha256Digest {
        self.iam_snapshot_digest
    }

    /// Return the authenticated revocation snapshot digest.
    #[must_use]
    pub const fn revocation_snapshot_digest(&self) -> Sha256Digest {
        self.revocation_snapshot_digest
    }

    /// Return the exact audience.
    #[must_use]
    pub const fn audience(&self) -> &ProtocolId {
        &self.audience
    }

    /// Return the exact runtime-configuration identity.
    #[must_use]
    pub const fn runtime_configuration_digest(&self) -> Sha256Digest {
        self.runtime_configuration_digest
    }

    /// Return the exact resource digest.
    #[must_use]
    pub const fn resource_digest(&self) -> Sha256Digest {
        self.resource_digest
    }

    /// Return the exact operation-context digest.
    #[must_use]
    pub const fn operation_context_digest(&self) -> Sha256Digest {
        self.operation_context_digest
    }

    /// Return the explicit evaluation time.
    #[must_use]
    pub const fn evaluated_at(&self) -> &UtcTimestamp {
        &self.evaluated_at
    }

    /// This local proof never grants external authority.
    #[must_use]
    pub const fn external_authority_granted(&self) -> bool {
        false
    }
}

/// Verify one exact, short-lived tenant access grant.
///
/// # Errors
///
/// Returns a closed failure for any schema, trust, context, policy, time, or revocation mismatch.
pub fn verify_tenant_access_grant_v1(
    signed: &SignedTenantAccessGrantV1,
    context: &TenantAccessVerificationContextV1<'_>,
) -> Result<VerifiedTenantAccessV1, TenantIamError> {
    let claims = &signed.claims;
    validate_access_claims_context(signed, context)?;
    validate_access_policy(claims, context)?;
    let revocations = validate_access_revocation_context(claims, context)?;

    let key = context
        .key_registry
        .get(
            &claims.tenant_id,
            &claims.issuer_id,
            &signed.key_id,
            TenantIamKeyPurposeV1::AccessGrant,
        )
        .ok_or(TenantIamError::UnknownKey)?;
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

    let grant_digest = signed.digest()?;
    if revocations.grant_revocations.iter().any(|revocation| {
        revocation.tenant_id == claims.tenant_id
            && revocation.grant_digest == grant_digest
            && context.evaluated_at >= &revocation.revoked_at
    }) || revocations.key_revocations.iter().any(|revocation| {
        revocation.tenant_id == claims.tenant_id
            && revocation.issuer_id == claims.issuer_id
            && revocation.key_id == signed.key_id
            && revocation.purpose == TenantIamKeyPurposeV1::AccessGrant
            && context.evaluated_at >= &revocation.revoked_at
    }) {
        return Err(TenantIamError::GrantOrKeyRevoked);
    }

    Ok(VerifiedTenantAccessV1 {
        grant_digest,
        iam_snapshot_digest: context.iam_snapshot.snapshot_digest(),
        revocation_snapshot_digest: context.revocation_snapshot.snapshot_digest(),
        tenant_id: claims.tenant_id.clone(),
        principal_id: claims.principal_id.clone(),
        audience: claims.audience.clone(),
        runtime_configuration_digest: claims.runtime_configuration_digest,
        action: claims.action,
        resource_digest: claims.resource.resource_digest,
        operation_context_digest: claims.operation_context_digest,
        evaluated_at: context.evaluated_at.clone(),
    })
}

fn validate_access_claims_context(
    signed: &SignedTenantAccessGrantV1,
    context: &TenantAccessVerificationContextV1<'_>,
) -> Result<(), TenantIamError> {
    let claims = &signed.claims;
    if signed.schema_version != SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1
        || claims.schema_version != TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1
    {
        return Err(TenantIamError::UnsupportedSchema);
    }
    if signed.algorithm != ED25519_ALGORITHM_V1 {
        return Err(TenantIamError::UnsupportedAlgorithm);
    }
    if claims.sequence == 0
        || claims.runtime_configuration_digest.is_zero()
        || claims.iam_snapshot_digest.is_zero()
        || claims.operation_context_digest.is_zero()
    {
        return Err(TenantIamError::ZeroDigest);
    }
    validate_interval_and_lifetime(
        &claims.issued_at,
        &claims.expires_at,
        Some(MAX_ACCESS_GRANT_LIFETIME_SECONDS),
    )?;
    if context.evaluated_at < &claims.issued_at || context.evaluated_at > &claims.expires_at {
        return Err(TenantIamError::InvalidValidity);
    }
    if signed.signer_id != claims.issuer_id
        || claims.tenant_id != context.expectation.tenant_id
        || claims.principal_id != context.expectation.principal_id
        || claims.issuer_id != context.expectation.issuer_id
        || claims.audience != context.expectation.audience
        || claims.runtime_configuration_digest != context.expectation.runtime_configuration_digest
        || claims.iam_snapshot_digest != context.expectation.iam_snapshot_digest
        || claims.action != context.expectation.action
        || claims.resource != context.expectation.resource
        || claims.operation_context_digest != context.expectation.operation_context_digest
    {
        return Err(TenantIamError::ExpectationMismatch);
    }
    Ok(())
}

fn validate_access_policy(
    claims: &TenantAccessGrantClaimsV1,
    context: &TenantAccessVerificationContextV1<'_>,
) -> Result<(), TenantIamError> {
    let policy = context.iam_snapshot.snapshot();
    if claims.iam_snapshot_digest != context.iam_snapshot.snapshot_digest()
        || policy.tenant.tenant_id != claims.tenant_id
        || policy.tenant.iam_issuer_id != claims.issuer_id
    {
        return Err(TenantIamError::ExpectationMismatch);
    }
    if policy.tenant.lifecycle != TenantLifecycleV1::Active
        || !interval_contains(
            &policy.tenant.valid_from,
            &policy.tenant.valid_until,
            policy.tenant.revoked_at.as_ref(),
            &claims.issued_at,
            &claims.expires_at,
            context.evaluated_at,
        )
    {
        return Err(TenantIamError::InactiveOrRevoked);
    }
    let principal = context
        .iam_snapshot
        .principal(&claims.principal_id)
        .ok_or(TenantIamError::InvalidIdentityRelationship)?;
    if principal.lifecycle != TenantLifecycleV1::Active
        || !interval_contains(
            &principal.valid_from,
            &principal.valid_until,
            principal.revoked_at.as_ref(),
            &claims.issued_at,
            &claims.expires_at,
            context.evaluated_at,
        )
    {
        return Err(TenantIamError::InactiveOrRevoked);
    }
    if !context
        .iam_snapshot
        .role_bindings_for(&claims.principal_id)
        .any(|binding| {
            role_grants(binding.role, claims.action)
                && interval_contains(
                    &binding.valid_from,
                    &binding.valid_until,
                    binding.revoked_at.as_ref(),
                    &claims.issued_at,
                    &claims.expires_at,
                    context.evaluated_at,
                )
        })
    {
        return Err(TenantIamError::ActionNotGranted);
    }
    validate_resource_binding_v1(
        &claims.resource,
        claims.action,
        &claims.tenant_id,
        &claims.principal_id,
    )
}

fn validate_access_revocation_context<'a>(
    claims: &TenantAccessGrantClaimsV1,
    context: &'a TenantAccessVerificationContextV1<'_>,
) -> Result<&'a TenantRevocationSnapshotV1, TenantIamError> {
    let revocations = context.revocation_snapshot.snapshot();
    let checkpoint = context.revocation_checkpoint;
    if revocations.tenant_id != claims.tenant_id
        || revocations.issuer_id != claims.issuer_id
        || revocations.audience != claims.audience
        || revocations.runtime_configuration_digest != claims.runtime_configuration_digest
        || context.evaluated_at != &context.revocation_snapshot.evaluated_at
        || checkpoint.tenant_id != revocations.tenant_id
        || checkpoint.issuer_id != revocations.issuer_id
        || checkpoint.audience != revocations.audience
        || checkpoint.runtime_configuration_digest != revocations.runtime_configuration_digest
        || context.evaluated_at < &revocations.effective_at
        || context.evaluated_at > &revocations.expires_at
    {
        return Err(TenantIamError::ExpectationMismatch);
    }
    if checkpoint.minimum_sequence == 0 || checkpoint.digest_at_minimum_sequence.is_zero() {
        return Err(TenantIamError::ZeroDigest);
    }
    if revocations.sequence < checkpoint.minimum_sequence {
        return Err(TenantIamError::RevocationRollback);
    }
    if revocations.sequence == checkpoint.minimum_sequence
        && context.revocation_snapshot.snapshot_digest != checkpoint.digest_at_minimum_sequence
    {
        return Err(TenantIamError::RevocationFork);
    }
    let key = context
        .key_registry
        .get(
            &revocations.tenant_id,
            &revocations.issuer_id,
            &context.revocation_snapshot.signer_key_id,
            TenantIamKeyPurposeV1::RevocationSnapshot,
        )
        .ok_or(TenantIamError::UnknownKey)?;
    let key_digest = domain_separated_digest(IAM_KEY_RECORD_DIGEST_DOMAIN_V1, key)?;
    if key_digest != context.revocation_snapshot.signer_key_record_digest {
        return Err(TenantIamError::ExpectationMismatch);
    }
    verify_key_window(
        key,
        &revocations.issued_at,
        &revocations.expires_at,
        context.evaluated_at,
    )?;
    Ok(revocations)
}

fn validate_sorted_revocations(
    snapshot: &TenantRevocationSnapshotV1,
) -> Result<(), TenantIamError> {
    if !strictly_sorted(&snapshot.grant_revocations)
        || !strictly_sorted(&snapshot.key_revocations)
        || snapshot.grant_revocations.windows(2).any(|pair| {
            pair[0].tenant_id == pair[1].tenant_id && pair[0].grant_digest == pair[1].grant_digest
        })
        || snapshot.key_revocations.windows(2).any(|pair| {
            pair[0].tenant_id == pair[1].tenant_id
                && pair[0].issuer_id == pair[1].issuer_id
                && pair[0].key_id == pair[1].key_id
                && pair[0].purpose == pair[1].purpose
        })
    {
        return Err(TenantIamError::DuplicateIdentity);
    }
    if snapshot
        .grant_revocations
        .iter()
        .any(|item| item.tenant_id != snapshot.tenant_id || item.grant_digest.is_zero())
        || snapshot.key_revocations.iter().any(|item| {
            item.tenant_id != snapshot.tenant_id || item.issuer_id != snapshot.issuer_id
        })
    {
        return Err(TenantIamError::InvalidIdentityRelationship);
    }
    Ok(())
}

fn strictly_sorted<T: Ord>(items: &[T]) -> bool {
    items.windows(2).all(|pair| pair[0] < pair[1])
}

fn role_grants(role: TenantRoleV1, action: TenantIamActionV1) -> bool {
    match role {
        TenantRoleV1::InferenceCaller => matches!(
            action,
            TenantIamActionV1::SubmitInference
                | TenantIamActionV1::CancelOwnInference
                | TenantIamActionV1::ReadOwnInference
                | TenantIamActionV1::ReadOwnEvidence
        ),
        TenantRoleV1::TenantAdministrator => matches!(
            action,
            TenantIamActionV1::ReadTenantAudit
                | TenantIamActionV1::ManageTenantPrincipals
                | TenantIamActionV1::ManageTenantTokenKeys
        ),
        TenantRoleV1::TenantAuditor => action == TenantIamActionV1::ReadTenantAudit,
    }
}

fn validate_interval_and_lifetime(
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    maximum_seconds: Option<u64>,
) -> Result<(), TenantIamError> {
    if issued_at > expires_at {
        return Err(TenantIamError::InvalidValidity);
    }
    if let Some(maximum_seconds) = maximum_seconds {
        let maximum_expiry = issued_at.checked_add_seconds(maximum_seconds)?;
        if expires_at > &maximum_expiry {
            return Err(TenantIamError::InvalidValidity);
        }
    }
    Ok(())
}

fn verify_key_window(
    key: &RegisteredTenantIamKeyV1,
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    evaluated_at: &UtcTimestamp,
) -> Result<(), TenantIamError> {
    if issued_at < &key.valid_from
        || expires_at > &key.valid_until
        || evaluated_at < &key.valid_from
        || evaluated_at > &key.valid_until
        || key
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| evaluated_at >= revoked_at)
    {
        return Err(TenantIamError::KeyOutsideValidityOrRevoked);
    }
    Ok(())
}

fn decode_verifying_key(public_key_hex: &str) -> Result<VerifyingKey, TenantIamError> {
    let bytes = HEXLOWER
        .decode(public_key_hex.as_bytes())
        .map_err(|_| TenantIamError::InvalidEncoding)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| TenantIamError::InvalidEncoding)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| TenantIamError::InvalidEncoding)
}

fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), TenantIamError> {
    let key = decode_verifying_key(public_key_hex)?;
    let bytes = HEXLOWER
        .decode(signature_hex.as_bytes())
        .map_err(|_| TenantIamError::InvalidEncoding)?;
    let signature =
        Signature::try_from(bytes.as_slice()).map_err(|_| TenantIamError::InvalidEncoding)?;
    key.verify_strict(message, &signature)
        .map_err(|_| TenantIamError::InvalidSignature)
}
