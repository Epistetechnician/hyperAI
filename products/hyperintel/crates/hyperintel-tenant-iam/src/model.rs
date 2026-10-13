use std::{collections::BTreeMap, fmt};

use hyperintel_core::{ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest};
use serde::{Deserialize, Serialize};

use crate::{TENANT_IAM_SNAPSHOT_SCHEMA_V1, TenantIamError};

/// Tenant lifecycle accepted by V1 policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantLifecycleV1 {
    /// Eligible for authorization within validity bounds.
    Active,
    /// Always denied.
    Suspended,
    /// Permanently denied and required to carry a revocation instant.
    Closed,
}

/// Principal kind accepted by V1 policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantPrincipalKindV1 {
    /// A human tenant member.
    Human,
    /// A tenant-controlled workload.
    Workload,
}

/// Fixed, non-wildcard tenant roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantRoleV1 {
    /// Submit and access only the principal's inference resources.
    InferenceCaller,
    /// Administer tenant principals and keys and read tenant audit resources.
    TenantAdministrator,
    /// Read tenant audit resources only.
    TenantAuditor,
}

/// One buyer tenant and its explicit IAM issuer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantV1 {
    /// Tenant identity.
    pub tenant_id: ProtocolId,
    /// Exact identity authorized to sign this tenant's IAM data.
    pub iam_issuer_id: ProtocolId,
    /// Current lifecycle.
    pub lifecycle: TenantLifecycleV1,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

/// One principal scoped to exactly one tenant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantPrincipalV1 {
    /// Tenant identity.
    pub tenant_id: ProtocolId,
    /// Principal identity within the tenant.
    pub principal_id: ProtocolId,
    /// Closed principal kind.
    pub kind: TenantPrincipalKindV1,
    /// Current lifecycle.
    pub lifecycle: TenantLifecycleV1,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

/// One fixed-role assignment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantRoleBindingV1 {
    /// Tenant identity.
    pub tenant_id: ProtocolId,
    /// Principal identity.
    pub principal_id: ProtocolId,
    /// Fixed role.
    pub role: TenantRoleV1,
    /// First valid instant.
    pub valid_from: UtcTimestamp,
    /// Last valid instant.
    pub valid_until: UtcTimestamp,
    /// Optional inclusive revocation instant.
    pub revoked_at: Option<UtcTimestamp>,
}

/// Caller-owned policy snapshot for exactly one tenant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantIamSnapshotV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Tenant and issuer boundary.
    pub tenant: TenantV1,
    /// Tenant principals.
    pub principals: Vec<TenantPrincipalV1>,
    /// Fixed-role bindings.
    pub role_bindings: Vec<TenantRoleBindingV1>,
}

/// Validated, indexed policy snapshot and canonical identity.
#[derive(Clone)]
pub struct ValidatedTenantIamSnapshotV1 {
    snapshot: TenantIamSnapshotV1,
    snapshot_digest: Sha256Digest,
    principals: BTreeMap<ProtocolId, TenantPrincipalV1>,
    role_bindings: BTreeMap<(ProtocolId, TenantRoleV1), TenantRoleBindingV1>,
}

impl fmt::Debug for ValidatedTenantIamSnapshotV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedTenantIamSnapshotV1")
            .field("snapshot", &"[REDACTED]")
            .field("snapshot_digest", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl TenantIamSnapshotV1 {
    /// Validate the single-tenant graph and compute its canonical digest.
    ///
    /// # Errors
    ///
    /// Returns a closed error for schema, identity, lifecycle, validity, or duplicate failures.
    pub fn validate(self) -> Result<ValidatedTenantIamSnapshotV1, TenantIamError> {
        if self.schema_version != TENANT_IAM_SNAPSHOT_SCHEMA_V1 {
            return Err(TenantIamError::UnsupportedSchema);
        }
        validate_lifecycle_interval(
            self.tenant.lifecycle,
            &self.tenant.valid_from,
            &self.tenant.valid_until,
            self.tenant.revoked_at.as_ref(),
        )?;

        if !strictly_sorted(&self.principals) || !strictly_sorted(&self.role_bindings) {
            return Err(TenantIamError::DuplicateIdentity);
        }

        let mut principals = BTreeMap::new();
        for principal in &self.principals {
            if principal.tenant_id != self.tenant.tenant_id {
                return Err(TenantIamError::InvalidIdentityRelationship);
            }
            validate_lifecycle_interval(
                principal.lifecycle,
                &principal.valid_from,
                &principal.valid_until,
                principal.revoked_at.as_ref(),
            )?;
            if principals
                .insert(principal.principal_id.clone(), principal.clone())
                .is_some()
            {
                return Err(TenantIamError::DuplicateIdentity);
            }
        }

        let mut role_bindings = BTreeMap::new();
        for binding in &self.role_bindings {
            if binding.tenant_id != self.tenant.tenant_id
                || !principals.contains_key(&binding.principal_id)
            {
                return Err(TenantIamError::InvalidIdentityRelationship);
            }
            validate_interval(&binding.valid_from, &binding.valid_until)?;
            if binding
                .revoked_at
                .as_ref()
                .is_some_and(|revoked_at| revoked_at < &binding.valid_from)
            {
                return Err(TenantIamError::InvalidValidity);
            }
            let identity = (binding.principal_id.clone(), binding.role);
            if role_bindings.insert(identity, binding.clone()).is_some() {
                return Err(TenantIamError::DuplicateIdentity);
            }
        }

        let snapshot_digest = domain_separated_digest(TENANT_IAM_SNAPSHOT_SCHEMA_V1, &self)?;
        Ok(ValidatedTenantIamSnapshotV1 {
            snapshot: self,
            snapshot_digest,
            principals,
            role_bindings,
        })
    }
}

fn strictly_sorted<T: Ord>(items: &[T]) -> bool {
    items.windows(2).all(|pair| pair[0] < pair[1])
}

impl ValidatedTenantIamSnapshotV1 {
    /// Return the exact caller-owned source snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &TenantIamSnapshotV1 {
        &self.snapshot
    }

    /// Return the canonical policy snapshot digest.
    #[must_use]
    pub const fn snapshot_digest(&self) -> Sha256Digest {
        self.snapshot_digest
    }

    pub(crate) fn principal(&self, principal_id: &ProtocolId) -> Option<&TenantPrincipalV1> {
        self.principals.get(principal_id)
    }

    pub(crate) fn role_bindings_for(
        &self,
        principal_id: &ProtocolId,
    ) -> impl Iterator<Item = &TenantRoleBindingV1> {
        self.role_bindings
            .range((principal_id.clone(), TenantRoleV1::InferenceCaller)..)
            .take_while(move |((candidate, _), _)| candidate == principal_id)
            .map(|(_, binding)| binding)
    }
}

pub(crate) fn interval_contains(
    valid_from: &UtcTimestamp,
    valid_until: &UtcTimestamp,
    revoked_at: Option<&UtcTimestamp>,
    issued_at: &UtcTimestamp,
    expires_at: &UtcTimestamp,
    evaluated_at: &UtcTimestamp,
) -> bool {
    valid_from <= issued_at
        && valid_until >= expires_at
        && revoked_at.is_none_or(|revoked_at| revoked_at > evaluated_at)
}

fn validate_lifecycle_interval(
    lifecycle: TenantLifecycleV1,
    valid_from: &UtcTimestamp,
    valid_until: &UtcTimestamp,
    revoked_at: Option<&UtcTimestamp>,
) -> Result<(), TenantIamError> {
    validate_interval(valid_from, valid_until)?;
    if lifecycle == TenantLifecycleV1::Closed && revoked_at.is_none() {
        return Err(TenantIamError::InactiveOrRevoked);
    }
    if revoked_at.is_some_and(|revoked_at| revoked_at < valid_from) {
        return Err(TenantIamError::InvalidValidity);
    }
    Ok(())
}

fn validate_interval(
    valid_from: &UtcTimestamp,
    valid_until: &UtcTimestamp,
) -> Result<(), TenantIamError> {
    if valid_from > valid_until {
        return Err(TenantIamError::InvalidValidity);
    }
    Ok(())
}
