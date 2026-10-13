use hyperintel_core::{ProtocolId, Sha256Digest};
use serde::{Deserialize, Serialize};

use crate::{TENANT_RESOURCE_BINDING_SCHEMA_V1, TenantIamActionV1, TenantIamError};

/// Closed resource kinds that V1 grants can bind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantResourceKindV1 {
    /// One inference request.
    InferenceRequest,
    /// One evidence bundle.
    EvidenceBundle,
    /// The tenant audit stream.
    TenantAudit,
    /// The tenant principal directory.
    PrincipalDirectory,
    /// The tenant IAM verification-key registry.
    VerificationKeyRegistry,
}

/// Closed resource purposes that V1 grants can bind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantResourcePurposeV1 {
    /// Submit one request.
    Submit,
    /// Cancel one request.
    Cancel,
    /// Read one resource.
    Read,
    /// Administer one tenant-wide resource.
    Administer,
}

/// Caller-owned identity and ownership statement for one exact resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantResourceBindingV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Tenant that owns the resource.
    pub tenant_id: ProtocolId,
    /// Closed resource kind.
    pub resource_kind: TenantResourceKindV1,
    /// Closed operation purpose.
    pub purpose: TenantResourcePurposeV1,
    /// Canonical caller-owned resource identity.
    pub resource_digest: Sha256Digest,
    /// Principal owner for principal-owned resources; absent for tenant-wide resources.
    pub owner_principal_id: Option<ProtocolId>,
}

/// Validate exact action/resource/tenant/principal compatibility.
///
/// # Errors
///
/// Returns when schema, digest, tenant, kind, purpose, or ownership is not exact.
pub fn validate_resource_binding_v1(
    resource: &TenantResourceBindingV1,
    action: TenantIamActionV1,
    tenant_id: &ProtocolId,
    principal_id: &ProtocolId,
) -> Result<(), TenantIamError> {
    if resource.schema_version != TENANT_RESOURCE_BINDING_SCHEMA_V1 {
        return Err(TenantIamError::UnsupportedSchema);
    }
    if resource.resource_digest.is_zero() {
        return Err(TenantIamError::ZeroDigest);
    }
    if &resource.tenant_id != tenant_id {
        return Err(TenantIamError::InvalidResourceBinding);
    }

    let (kind, purpose, principal_owned) = match action {
        TenantIamActionV1::SubmitInference => (
            TenantResourceKindV1::InferenceRequest,
            TenantResourcePurposeV1::Submit,
            true,
        ),
        TenantIamActionV1::CancelOwnInference => (
            TenantResourceKindV1::InferenceRequest,
            TenantResourcePurposeV1::Cancel,
            true,
        ),
        TenantIamActionV1::ReadOwnInference => (
            TenantResourceKindV1::InferenceRequest,
            TenantResourcePurposeV1::Read,
            true,
        ),
        TenantIamActionV1::ReadOwnEvidence => (
            TenantResourceKindV1::EvidenceBundle,
            TenantResourcePurposeV1::Read,
            true,
        ),
        TenantIamActionV1::ReadTenantAudit => (
            TenantResourceKindV1::TenantAudit,
            TenantResourcePurposeV1::Read,
            false,
        ),
        TenantIamActionV1::ManageTenantPrincipals => (
            TenantResourceKindV1::PrincipalDirectory,
            TenantResourcePurposeV1::Administer,
            false,
        ),
        TenantIamActionV1::ManageTenantTokenKeys => (
            TenantResourceKindV1::VerificationKeyRegistry,
            TenantResourcePurposeV1::Administer,
            false,
        ),
    };
    if resource.resource_kind != kind || resource.purpose != purpose {
        return Err(TenantIamError::InvalidResourceBinding);
    }
    match (&resource.owner_principal_id, principal_owned) {
        (Some(owner), true) if owner == principal_id => Ok(()),
        (None, false) => Ok(()),
        _ => Err(TenantIamError::InvalidResourceBinding),
    }
}
