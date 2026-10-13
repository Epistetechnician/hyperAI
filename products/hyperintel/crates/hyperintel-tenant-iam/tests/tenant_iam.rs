//! Contract coverage for pure-data tenant IAM verification.

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer, SigningKey};
use hyperintel_core::{ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_tenant_iam::{
    ED25519_ALGORITHM_V1, RegisteredTenantIamKeyV1, SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1,
    SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1, SignedTenantAccessGrantV1,
    SignedTenantRevocationSnapshotV1, TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1,
    TENANT_IAM_KEY_SCHEMA_V1, TENANT_IAM_SNAPSHOT_SCHEMA_V1, TENANT_RESOURCE_BINDING_SCHEMA_V1,
    TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1, TenantAccessExpectationV1, TenantAccessGrantClaimsV1,
    TenantAccessVerificationContextV1, TenantGrantRevocationV1, TenantIamActionV1, TenantIamError,
    TenantIamKeyPurposeV1, TenantIamKeyRegistryV1, TenantIamSnapshotV1, TenantKeyRevocationV1,
    TenantLifecycleV1, TenantPrincipalKindV1, TenantPrincipalV1, TenantResourceBindingV1,
    TenantResourceKindV1, TenantResourcePurposeV1, TenantRevocationCheckpointV1,
    TenantRevocationSnapshotV1, TenantRevocationVerificationContextV1, TenantRoleBindingV1,
    TenantRoleV1, TenantV1, ValidatedTenantIamSnapshotV1, ValidatedTenantRevocationSnapshotV1,
    verify_tenant_access_grant_v1, verify_tenant_revocation_snapshot_v1,
};
use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    access_signer: SigningKey,
    revocation_signer: SigningKey,
    iam: ValidatedTenantIamSnapshotV1,
    keys: TenantIamKeyRegistryV1,
    signed_revocations: SignedTenantRevocationSnapshotV1,
    revocations: ValidatedTenantRevocationSnapshotV1,
    revocation_checkpoint: TenantRevocationCheckpointV1,
    grant: SignedTenantAccessGrantV1,
    expectation: TenantAccessExpectationV1,
    evaluated_at: UtcTimestamp,
}

#[test]
fn contract_valid_grant_is_exact_repeatable_and_non_authoritative() -> TestResult {
    let fixture = fixture()?;
    let first = verify(&fixture)?;
    let second = verify(&fixture)?;
    assert_eq!(first.grant_digest(), second.grant_digest());
    assert_eq!(first.tenant_id().as_str(), "tenant:alpha");
    assert_eq!(first.action(), TenantIamActionV1::SubmitInference);
    assert!(!first.external_authority_granted());
    assert_eq!(
        fixture.iam.snapshot_digest().to_string(),
        "3c704f5b55ea1cc925310715c10e7e01a37bcd2197d4d394a3c76f5d657eaeeb"
    );
    Ok(())
}

#[test]
fn contract_original_signature_rejects_every_material_claim_tamper() -> TestResult {
    let fixture = fixture()?;
    let mut cases = Vec::new();
    let mut changed = fixture.grant.clone();
    changed.claims.sequence += 1;
    cases.push(changed);
    let mut changed = fixture.grant.clone();
    changed.claims.audience = id("gateway:other")?;
    cases.push(changed);
    let mut changed = fixture.grant.clone();
    changed.claims.resource.resource_digest = digest(90);
    cases.push(changed);
    let mut changed = fixture.grant.clone();
    changed.claims.operation_context_digest = digest(91);
    cases.push(changed);
    let mut changed = fixture.grant.clone();
    changed.key_id = id("key:other")?;
    cases.push(changed);
    for changed in cases {
        assert!(verify_tenant_access_grant_v1(&changed, &access_context(&fixture)).is_err());
    }
    Ok(())
}

#[test]
fn contract_validly_resigned_context_and_ownership_confusion_fail_closed() -> TestResult {
    let fixture = fixture()?;

    let mut wrong_action = fixture.grant.clone();
    wrong_action.claims.action = TenantIamActionV1::ReadTenantAudit;
    resign_grant(&mut wrong_action, &fixture.access_signer)?;
    assert!(matches!(
        verify_tenant_access_grant_v1(&wrong_action, &access_context(&fixture)),
        Err(TenantIamError::ExpectationMismatch)
    ));

    let mut wrong_owner = fixture.grant.clone();
    wrong_owner.claims.resource.owner_principal_id = Some(id("principal:other")?);
    resign_grant(&mut wrong_owner, &fixture.access_signer)?;
    let mut expectation = fixture.expectation.clone();
    expectation.resource = wrong_owner.claims.resource.clone();
    let context = TenantAccessVerificationContextV1 {
        expectation: &expectation,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&wrong_owner, &context),
        Err(TenantIamError::InvalidResourceBinding)
    ));

    let mut wrong_tenant = fixture.grant.clone();
    wrong_tenant.claims.tenant_id = id("tenant:beta")?;
    wrong_tenant.claims.resource.tenant_id = id("tenant:beta")?;
    resign_grant(&mut wrong_tenant, &fixture.access_signer)?;
    let mut expectation = fixture.expectation.clone();
    expectation.tenant_id = id("tenant:beta")?;
    expectation.resource = wrong_tenant.claims.resource.clone();
    let context = TenantAccessVerificationContextV1 {
        expectation: &expectation,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&wrong_tenant, &context),
        Err(TenantIamError::ExpectationMismatch)
    ));
    Ok(())
}

#[test]
fn contract_key_purpose_namespace_duplicate_and_validity_are_fail_closed() -> TestResult {
    let fixture = fixture()?;
    let access_key = registered_key(
        &fixture.access_signer,
        TenantIamKeyPurposeV1::AccessGrant,
        "key:access",
    )?;
    let mut registry = TenantIamKeyRegistryV1::default();
    registry.register(access_key.clone())?;
    registry.register(registered_key(
        &fixture.revocation_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:revocation",
    )?)?;
    let attacker = SigningKey::from_bytes(&[9; 32]);
    let mut replacement = access_key;
    replacement.public_key_hex = HEXLOWER.encode(attacker.verifying_key().as_bytes());
    assert!(matches!(
        registry.register(replacement),
        Err(TenantIamError::DuplicateIdentity)
    ));
    let context = TenantAccessVerificationContextV1 {
        key_registry: &registry,
        ..access_context(&fixture)
    };
    verify_tenant_access_grant_v1(&fixture.grant, &context)?;

    let mut purpose_only = TenantIamKeyRegistryV1::default();
    purpose_only.register(registered_key(
        &fixture.access_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:access",
    )?)?;
    purpose_only.register(registered_key(
        &fixture.revocation_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:revocation",
    )?)?;
    let context = TenantAccessVerificationContextV1 {
        key_registry: &purpose_only,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &context),
        Err(TenantIamError::UnknownKey)
    ));

    let mut other_tenant = TenantIamKeyRegistryV1::default();
    let mut key = registered_key(
        &fixture.access_signer,
        TenantIamKeyPurposeV1::AccessGrant,
        "key:access",
    )?;
    key.tenant_id = id("tenant:beta")?;
    other_tenant.register(key)?;
    other_tenant.register(registered_key(
        &fixture.revocation_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:revocation",
    )?)?;
    let context = TenantAccessVerificationContextV1 {
        key_registry: &other_tenant,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &context),
        Err(TenantIamError::UnknownKey)
    ));
    Ok(())
}

#[test]
fn contract_grant_time_boundaries_and_lifetime_are_exact() -> TestResult {
    let fixture = fixture()?;
    for instant in [
        fixture.grant.claims.issued_at.clone(),
        fixture.grant.claims.expires_at.clone(),
    ] {
        let revocations = validate_revocations(
            &fixture.signed_revocations,
            &fixture.keys,
            1,
            fixture.signed_revocations.digest()?,
            &instant,
        )?;
        let context = TenantAccessVerificationContextV1 {
            evaluated_at: &instant,
            revocation_snapshot: &revocations,
            ..access_context(&fixture)
        };
        verify_tenant_access_grant_v1(&fixture.grant, &context)?;
    }
    let before = time("2026-07-21T10:00:59Z")?;
    let after = time("2026-07-21T10:06:01Z")?;
    for instant in [&before, &after] {
        let context = TenantAccessVerificationContextV1 {
            evaluated_at: instant,
            ..access_context(&fixture)
        };
        assert!(matches!(
            verify_tenant_access_grant_v1(&fixture.grant, &context),
            Err(TenantIamError::InvalidValidity)
        ));
    }
    let mut too_long = fixture.grant.clone();
    too_long.claims.expires_at = time("2026-07-21T10:06:01Z")?;
    resign_grant(&mut too_long, &fixture.access_signer)?;
    assert!(matches!(
        verify_tenant_access_grant_v1(&too_long, &access_context(&fixture)),
        Err(TenantIamError::InvalidValidity)
    ));
    Ok(())
}

#[test]
fn contract_signed_revocation_is_inclusive_and_rollback_protected() -> TestResult {
    let fixture = fixture()?;
    let grant_digest = fixture.grant.digest()?;
    let revoked_at = fixture.evaluated_at.clone();

    let mut signed = fixture.signed_revocations.clone();
    signed.snapshot.sequence = 2;
    signed.snapshot.grant_revocations = vec![TenantGrantRevocationV1 {
        tenant_id: id("tenant:alpha")?,
        grant_digest,
        revoked_at: revoked_at.clone(),
    }];
    resign_revocations(&mut signed, &fixture.revocation_signer)?;
    let validated = validate_revocations(&signed, &fixture.keys, 2, signed.digest()?, &revoked_at)?;
    let context = TenantAccessVerificationContextV1 {
        revocation_snapshot: &validated,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &context),
        Err(TenantIamError::GrantOrKeyRevoked)
    ));

    let rollback_checkpoint = TenantRevocationCheckpointV1 {
        minimum_sequence: 3,
        digest_at_minimum_sequence: digest(77),
        ..checkpoint(3, digest(77))?
    };
    let revocation_context = TenantRevocationVerificationContextV1 {
        key_registry: &fixture.keys,
        checkpoint: &rollback_checkpoint,
        evaluated_at: &revoked_at,
    };
    assert!(matches!(
        verify_tenant_revocation_snapshot_v1(&signed, &revocation_context),
        Err(TenantIamError::RevocationRollback)
    ));

    let fork_checkpoint = checkpoint(2, digest(78))?;
    let revocation_context = TenantRevocationVerificationContextV1 {
        key_registry: &fixture.keys,
        checkpoint: &fork_checkpoint,
        evaluated_at: &revoked_at,
    };
    assert!(matches!(
        verify_tenant_revocation_snapshot_v1(&signed, &revocation_context),
        Err(TenantIamError::RevocationFork)
    ));

    let zero_checkpoint = checkpoint(0, Sha256Digest::from_bytes([0; 32]))?;
    let revocation_context = TenantRevocationVerificationContextV1 {
        key_registry: &fixture.keys,
        checkpoint: &zero_checkpoint,
        evaluated_at: &revoked_at,
    };
    assert!(matches!(
        verify_tenant_revocation_snapshot_v1(&signed, &revocation_context),
        Err(TenantIamError::ZeroDigest)
    ));
    Ok(())
}

#[test]
fn contract_key_revocation_and_future_revocation_use_inclusive_time() -> TestResult {
    let fixture = fixture()?;
    let mut signed = fixture.signed_revocations.clone();
    signed.snapshot.sequence = 2;
    signed.snapshot.key_revocations = vec![TenantKeyRevocationV1 {
        tenant_id: id("tenant:alpha")?,
        issuer_id: id("issuer:alpha")?,
        key_id: id("key:access")?,
        purpose: TenantIamKeyPurposeV1::AccessGrant,
        revoked_at: time("2026-07-21T10:04:00Z")?,
    }];
    resign_revocations(&mut signed, &fixture.revocation_signer)?;
    let digest = signed.digest()?;
    let before = time("2026-07-21T10:03:59Z")?;
    let at = time("2026-07-21T10:04:00Z")?;
    let before_snapshot = validate_revocations(&signed, &fixture.keys, 2, digest, &before)?;
    let before_context = TenantAccessVerificationContextV1 {
        revocation_snapshot: &before_snapshot,
        evaluated_at: &before,
        ..access_context(&fixture)
    };
    verify_tenant_access_grant_v1(&fixture.grant, &before_context)?;
    let at_snapshot = validate_revocations(&signed, &fixture.keys, 2, digest, &at)?;
    let at_context = TenantAccessVerificationContextV1 {
        revocation_snapshot: &at_snapshot,
        evaluated_at: &at,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &at_context),
        Err(TenantIamError::GrantOrKeyRevoked)
    ));

    let mut self_revoked = fixture.signed_revocations.clone();
    self_revoked.snapshot.sequence = 3;
    self_revoked.snapshot.key_revocations = vec![TenantKeyRevocationV1 {
        tenant_id: id("tenant:alpha")?,
        issuer_id: id("issuer:alpha")?,
        key_id: id("key:revocation")?,
        purpose: TenantIamKeyPurposeV1::RevocationSnapshot,
        revoked_at: at.clone(),
    }];
    resign_revocations(&mut self_revoked, &fixture.revocation_signer)?;
    let self_digest = self_revoked.digest()?;
    let checkpoint = checkpoint(3, self_digest)?;
    let context = TenantRevocationVerificationContextV1 {
        key_registry: &fixture.keys,
        checkpoint: &checkpoint,
        evaluated_at: &at,
    };
    assert!(matches!(
        verify_tenant_revocation_snapshot_v1(&self_revoked, &context),
        Err(TenantIamError::GrantOrKeyRevoked)
    ));

    let mut locally_revoked_keys = TenantIamKeyRegistryV1::default();
    locally_revoked_keys.register(registered_key(
        &fixture.access_signer,
        TenantIamKeyPurposeV1::AccessGrant,
        "key:access",
    )?)?;
    let mut revocation_key = registered_key(
        &fixture.revocation_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:revocation",
    )?;
    revocation_key.revoked_at = Some(at.clone());
    locally_revoked_keys.register(revocation_key)?;
    let validated_before = validate_revocations(
        &fixture.signed_revocations,
        &locally_revoked_keys,
        1,
        fixture.signed_revocations.digest()?,
        &fixture.evaluated_at,
    )?;
    let reused_context = TenantAccessVerificationContextV1 {
        key_registry: &locally_revoked_keys,
        revocation_snapshot: &validated_before,
        evaluated_at: &at,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &reused_context),
        Err(TenantIamError::ExpectationMismatch)
    ));
    Ok(())
}

#[test]
fn contract_validated_revocation_rejects_registry_substitution_and_checkpoint_advance() -> TestResult
{
    let fixture = fixture()?;
    let substitute_revocation_signer = SigningKey::from_bytes(&[10; 32]);
    let mut substitute_registry = TenantIamKeyRegistryV1::default();
    substitute_registry.register(registered_key(
        &fixture.access_signer,
        TenantIamKeyPurposeV1::AccessGrant,
        "key:access",
    )?)?;
    substitute_registry.register(registered_key(
        &substitute_revocation_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:revocation",
    )?)?;
    let substituted = TenantAccessVerificationContextV1 {
        key_registry: &substitute_registry,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &substituted),
        Err(TenantIamError::ExpectationMismatch)
    ));

    let advanced_checkpoint = checkpoint(2, digest(88))?;
    let advanced = TenantAccessVerificationContextV1 {
        revocation_checkpoint: &advanced_checkpoint,
        ..access_context(&fixture)
    };
    assert!(matches!(
        verify_tenant_access_grant_v1(&fixture.grant, &advanced),
        Err(TenantIamError::RevocationRollback)
    ));
    Ok(())
}

#[test]
fn contract_fixed_role_action_resource_matrix_has_no_implicit_grants() -> TestResult {
    let fixture = fixture()?;
    let cases = [
        (
            TenantRoleV1::InferenceCaller,
            TenantIamActionV1::SubmitInference,
        ),
        (
            TenantRoleV1::InferenceCaller,
            TenantIamActionV1::CancelOwnInference,
        ),
        (
            TenantRoleV1::InferenceCaller,
            TenantIamActionV1::ReadOwnInference,
        ),
        (
            TenantRoleV1::InferenceCaller,
            TenantIamActionV1::ReadOwnEvidence,
        ),
        (
            TenantRoleV1::TenantAdministrator,
            TenantIamActionV1::ReadTenantAudit,
        ),
        (
            TenantRoleV1::TenantAdministrator,
            TenantIamActionV1::ManageTenantPrincipals,
        ),
        (
            TenantRoleV1::TenantAdministrator,
            TenantIamActionV1::ManageTenantTokenKeys,
        ),
        (
            TenantRoleV1::TenantAuditor,
            TenantIamActionV1::ReadTenantAudit,
        ),
    ];
    for (role, action) in cases {
        let (iam, grant, expectation) = grant_for_role_action(&fixture, role, action)?;
        let context = TenantAccessVerificationContextV1 {
            iam_snapshot: &iam,
            expectation: &expectation,
            ..access_context(&fixture)
        };
        verify_tenant_access_grant_v1(&grant, &context)?;
    }

    for role in [
        TenantRoleV1::TenantAdministrator,
        TenantRoleV1::TenantAuditor,
    ] {
        let (iam, grant, expectation) =
            grant_for_role_action(&fixture, role, TenantIamActionV1::ReadOwnInference)?;
        let context = TenantAccessVerificationContextV1 {
            iam_snapshot: &iam,
            expectation: &expectation,
            ..access_context(&fixture)
        };
        assert!(matches!(
            verify_tenant_access_grant_v1(&grant, &context),
            Err(TenantIamError::ActionNotGranted)
        ));
    }
    Ok(())
}

#[test]
fn contract_signed_revocation_tamper_and_wrong_signer_fail_closed() -> TestResult {
    let fixture = fixture()?;
    let mut cases = Vec::new();
    let mut changed = fixture.signed_revocations.clone();
    changed.schema_version = "hyperintel.signed-tenant-revocation-snapshot.v2".to_owned();
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.schema_version = "hyperintel.tenant-revocation-snapshot.v2".to_owned();
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.algorithm = "ed448".to_owned();
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.signer_id = id("issuer:other")?;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.key_id = id("key:other")?;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.tenant_id = id("tenant:beta")?;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.issuer_id = id("issuer:other")?;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.audience = id("gateway:other")?;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.runtime_configuration_digest = digest(99);
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.sequence = 0;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.expires_at = time("2026-07-21T10:06:01Z")?;
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.snapshot.grant_revocations = vec![TenantGrantRevocationV1 {
        tenant_id: id("tenant:alpha")?,
        grant_digest: fixture.grant.digest()?,
        revoked_at: fixture.evaluated_at.clone(),
    }];
    cases.push(changed);
    let mut changed = fixture.signed_revocations.clone();
    changed.signature_hex = "not-hex".to_owned();
    cases.push(changed);

    for changed in cases {
        let context = TenantRevocationVerificationContextV1 {
            key_registry: &fixture.keys,
            checkpoint: &fixture.revocation_checkpoint,
            evaluated_at: &fixture.evaluated_at,
        };
        assert!(verify_tenant_revocation_snapshot_v1(&changed, &context).is_err());
    }

    let wrong_signer = SigningKey::from_bytes(&[11; 32]);
    let mut wrong_signature = fixture.signed_revocations.clone();
    resign_revocations(&mut wrong_signature, &wrong_signer)?;
    let wrong_checkpoint = checkpoint(1, wrong_signature.digest()?)?;
    let context = TenantRevocationVerificationContextV1 {
        key_registry: &fixture.keys,
        checkpoint: &wrong_checkpoint,
        evaluated_at: &fixture.evaluated_at,
    };
    assert!(matches!(
        verify_tenant_revocation_snapshot_v1(&wrong_signature, &context),
        Err(TenantIamError::InvalidSignature)
    ));
    Ok(())
}

#[test]
fn contract_snapshot_order_duplicates_schemas_and_unknown_fields_fail_closed() -> TestResult {
    let fixture = fixture()?;
    let mut duplicate = fixture.iam.snapshot().clone();
    duplicate.principals.push(duplicate.principals[0].clone());
    assert!(matches!(
        duplicate.validate(),
        Err(TenantIamError::DuplicateIdentity)
    ));
    let mut invalid_revocation = fixture.iam.snapshot().clone();
    invalid_revocation.role_bindings[0].revoked_at = Some(time("2026-07-21T09:59:59Z")?);
    assert!(matches!(
        invalid_revocation.validate(),
        Err(TenantIamError::InvalidValidity)
    ));

    let mut value = serde_json::to_value(&fixture.grant)?;
    value["claims"]
        .as_object_mut()
        .ok_or("claims must be object")?
        .insert("ambient_admin".to_owned(), Value::Bool(true));
    assert!(serde_json::from_value::<SignedTenantAccessGrantV1>(value).is_err());

    let mut wrong_schema = fixture.grant.clone();
    wrong_schema.claims.schema_version = "hyperintel.tenant-access-grant-claims.v2".to_owned();
    resign_grant(&mut wrong_schema, &fixture.access_signer)?;
    assert!(matches!(
        verify_tenant_access_grant_v1(&wrong_schema, &access_context(&fixture)),
        Err(TenantIamError::UnsupportedSchema)
    ));

    let mut duplicate_revocation = fixture.signed_revocations.clone();
    duplicate_revocation.snapshot.sequence = 2;
    duplicate_revocation.snapshot.grant_revocations = vec![
        TenantGrantRevocationV1 {
            tenant_id: id("tenant:alpha")?,
            grant_digest: fixture.grant.digest()?,
            revoked_at: time("2026-07-21T10:03:00Z")?,
        },
        TenantGrantRevocationV1 {
            tenant_id: id("tenant:alpha")?,
            grant_digest: fixture.grant.digest()?,
            revoked_at: time("2026-07-21T10:04:00Z")?,
        },
    ];
    resign_revocations(&mut duplicate_revocation, &fixture.revocation_signer)?;
    let checkpoint = checkpoint(2, duplicate_revocation.digest()?)?;
    let context = TenantRevocationVerificationContextV1 {
        key_registry: &fixture.keys,
        checkpoint: &checkpoint,
        evaluated_at: &fixture.evaluated_at,
    };
    assert!(matches!(
        verify_tenant_revocation_snapshot_v1(&duplicate_revocation, &context),
        Err(TenantIamError::DuplicateIdentity)
    ));
    Ok(())
}

#[test]
fn contract_debug_redacts_bearer_and_resource_metadata() -> TestResult {
    let fixture = fixture()?;
    let rendered = format!("{:?}", fixture.grant);
    for forbidden in [
        "principal:alice",
        &fixture.grant.signature_hex,
        &fixture.grant.claims.resource.resource_digest.to_string(),
        &fixture.grant.claims.operation_context_digest.to_string(),
    ] {
        assert!(!rendered.contains(forbidden));
    }
    let verified = verify(&fixture)?;
    let rendered = format!("{verified:?}");
    assert!(!rendered.contains("principal:alice"));
    assert!(!rendered.contains(&verified.grant_digest().to_string()));
    let policy_debug = format!("{:?}", fixture.iam);
    assert!(!policy_debug.contains("principal:alice"));
    assert!(!policy_debug.contains(&fixture.iam.snapshot_digest().to_string()));
    let revocation_debug = format!("{:?}", fixture.revocations);
    assert!(!revocation_debug.contains(&fixture.revocations.snapshot_digest().to_string()));
    assert!(!revocation_debug.contains(&fixture.grant.digest()?.to_string()));
    Ok(())
}

fn fixture() -> TestResult<Fixture> {
    let access_signer = SigningKey::from_bytes(&[7; 32]);
    let revocation_signer = SigningKey::from_bytes(&[8; 32]);
    let iam = iam_snapshot()?.validate()?;

    let mut keys = TenantIamKeyRegistryV1::default();
    keys.register(registered_key(
        &access_signer,
        TenantIamKeyPurposeV1::AccessGrant,
        "key:access",
    )?)?;
    keys.register(registered_key(
        &revocation_signer,
        TenantIamKeyPurposeV1::RevocationSnapshot,
        "key:revocation",
    )?)?;

    let evaluated_at = time("2026-07-21T10:03:00Z")?;
    let mut signed_revocations = unsigned_revocations()?;
    resign_revocations(&mut signed_revocations, &revocation_signer)?;
    let revocation_checkpoint = checkpoint(1, signed_revocations.digest()?)?;
    let revocations = validate_revocations(
        &signed_revocations,
        &keys,
        1,
        signed_revocations.digest()?,
        &evaluated_at,
    )?;

    let resource = inference_resource()?;
    let mut grant = unsigned_grant(&iam, resource.clone())?;
    resign_grant(&mut grant, &access_signer)?;
    let expectation = TenantAccessExpectationV1 {
        tenant_id: id("tenant:alpha")?,
        principal_id: id("principal:alice")?,
        issuer_id: id("issuer:alpha")?,
        audience: id("gateway:local")?,
        runtime_configuration_digest: digest(20),
        iam_snapshot_digest: iam.snapshot_digest(),
        action: TenantIamActionV1::SubmitInference,
        resource,
        operation_context_digest: digest(40),
    };
    Ok(Fixture {
        access_signer,
        revocation_signer,
        iam,
        keys,
        signed_revocations,
        revocations,
        revocation_checkpoint,
        grant,
        expectation,
        evaluated_at,
    })
}

fn iam_snapshot() -> TestResult<TenantIamSnapshotV1> {
    Ok(TenantIamSnapshotV1 {
        schema_version: TENANT_IAM_SNAPSHOT_SCHEMA_V1.to_owned(),
        tenant: TenantV1 {
            tenant_id: id("tenant:alpha")?,
            iam_issuer_id: id("issuer:alpha")?,
            lifecycle: TenantLifecycleV1::Active,
            valid_from: time("2026-07-21T10:00:00Z")?,
            valid_until: time("2026-07-21T12:00:00Z")?,
            revoked_at: None,
        },
        principals: vec![TenantPrincipalV1 {
            tenant_id: id("tenant:alpha")?,
            principal_id: id("principal:alice")?,
            kind: TenantPrincipalKindV1::Workload,
            lifecycle: TenantLifecycleV1::Active,
            valid_from: time("2026-07-21T10:00:00Z")?,
            valid_until: time("2026-07-21T12:00:00Z")?,
            revoked_at: None,
        }],
        role_bindings: vec![TenantRoleBindingV1 {
            tenant_id: id("tenant:alpha")?,
            principal_id: id("principal:alice")?,
            role: TenantRoleV1::InferenceCaller,
            valid_from: time("2026-07-21T10:00:00Z")?,
            valid_until: time("2026-07-21T12:00:00Z")?,
            revoked_at: None,
        }],
    })
}

fn unsigned_revocations() -> TestResult<SignedTenantRevocationSnapshotV1> {
    Ok(SignedTenantRevocationSnapshotV1 {
        schema_version: SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
        snapshot: TenantRevocationSnapshotV1 {
            schema_version: TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
            tenant_id: id("tenant:alpha")?,
            issuer_id: id("issuer:alpha")?,
            audience: id("gateway:local")?,
            runtime_configuration_digest: digest(20),
            sequence: 1,
            issued_at: time("2026-07-21T10:01:00Z")?,
            effective_at: time("2026-07-21T10:01:00Z")?,
            expires_at: time("2026-07-21T10:06:00Z")?,
            grant_revocations: vec![],
            key_revocations: vec![],
        },
        signer_id: id("issuer:alpha")?,
        key_id: id("key:revocation")?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn inference_resource() -> TestResult<TenantResourceBindingV1> {
    Ok(TenantResourceBindingV1 {
        schema_version: TENANT_RESOURCE_BINDING_SCHEMA_V1.to_owned(),
        tenant_id: id("tenant:alpha")?,
        resource_kind: TenantResourceKindV1::InferenceRequest,
        purpose: TenantResourcePurposeV1::Submit,
        resource_digest: digest(30),
        owner_principal_id: Some(id("principal:alice")?),
    })
}

fn unsigned_grant(
    iam: &ValidatedTenantIamSnapshotV1,
    resource: TenantResourceBindingV1,
) -> TestResult<SignedTenantAccessGrantV1> {
    Ok(SignedTenantAccessGrantV1 {
        schema_version: SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1.to_owned(),
        claims: TenantAccessGrantClaimsV1 {
            schema_version: TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1.to_owned(),
            sequence: 1,
            tenant_id: id("tenant:alpha")?,
            principal_id: id("principal:alice")?,
            issuer_id: id("issuer:alpha")?,
            audience: id("gateway:local")?,
            runtime_configuration_digest: digest(20),
            iam_snapshot_digest: iam.snapshot_digest(),
            action: TenantIamActionV1::SubmitInference,
            resource,
            operation_context_digest: digest(40),
            issued_at: time("2026-07-21T10:01:00Z")?,
            expires_at: time("2026-07-21T10:06:00Z")?,
        },
        signer_id: id("issuer:alpha")?,
        key_id: id("key:access")?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn grant_for_role_action(
    fixture: &Fixture,
    role: TenantRoleV1,
    action: TenantIamActionV1,
) -> TestResult<(
    ValidatedTenantIamSnapshotV1,
    SignedTenantAccessGrantV1,
    TenantAccessExpectationV1,
)> {
    let mut snapshot = fixture.iam.snapshot().clone();
    snapshot.role_bindings[0].role = role;
    let iam = snapshot.validate()?;
    let resource = resource_for_action(action)?;
    let mut grant = fixture.grant.clone();
    grant.claims.iam_snapshot_digest = iam.snapshot_digest();
    grant.claims.action = action;
    grant.claims.resource = resource.clone();
    resign_grant(&mut grant, &fixture.access_signer)?;
    let mut expectation = fixture.expectation.clone();
    expectation.iam_snapshot_digest = iam.snapshot_digest();
    expectation.action = action;
    expectation.resource = resource;
    Ok((iam, grant, expectation))
}

fn resource_for_action(action: TenantIamActionV1) -> TestResult<TenantResourceBindingV1> {
    let (resource_kind, purpose, principal_owned) = match action {
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
    Ok(TenantResourceBindingV1 {
        schema_version: TENANT_RESOURCE_BINDING_SCHEMA_V1.to_owned(),
        tenant_id: id("tenant:alpha")?,
        resource_kind,
        purpose,
        resource_digest: digest(30),
        owner_principal_id: if principal_owned {
            Some(id("principal:alice")?)
        } else {
            None
        },
    })
}

fn verify(
    fixture: &Fixture,
) -> Result<hyperintel_tenant_iam::VerifiedTenantAccessV1, TenantIamError> {
    verify_tenant_access_grant_v1(&fixture.grant, &access_context(fixture))
}

fn access_context(fixture: &Fixture) -> TenantAccessVerificationContextV1<'_> {
    TenantAccessVerificationContextV1 {
        iam_snapshot: &fixture.iam,
        key_registry: &fixture.keys,
        revocation_snapshot: &fixture.revocations,
        revocation_checkpoint: &fixture.revocation_checkpoint,
        expectation: &fixture.expectation,
        evaluated_at: &fixture.evaluated_at,
    }
}

fn validate_revocations(
    signed: &SignedTenantRevocationSnapshotV1,
    keys: &TenantIamKeyRegistryV1,
    minimum_sequence: u64,
    digest_at_minimum_sequence: Sha256Digest,
    evaluated_at: &UtcTimestamp,
) -> Result<ValidatedTenantRevocationSnapshotV1, TenantIamError> {
    let checkpoint = checkpoint(minimum_sequence, digest_at_minimum_sequence)
        .map_err(|_| TenantIamError::InvalidIdentityRelationship)?;
    verify_tenant_revocation_snapshot_v1(
        signed,
        &TenantRevocationVerificationContextV1 {
            key_registry: keys,
            checkpoint: &checkpoint,
            evaluated_at,
        },
    )
}

fn checkpoint(
    minimum_sequence: u64,
    digest_at_minimum_sequence: Sha256Digest,
) -> TestResult<TenantRevocationCheckpointV1> {
    Ok(TenantRevocationCheckpointV1 {
        tenant_id: id("tenant:alpha")?,
        issuer_id: id("issuer:alpha")?,
        audience: id("gateway:local")?,
        runtime_configuration_digest: digest(20),
        minimum_sequence,
        digest_at_minimum_sequence,
    })
}

fn registered_key(
    signer: &SigningKey,
    purpose: TenantIamKeyPurposeV1,
    key_id: &str,
) -> TestResult<RegisteredTenantIamKeyV1> {
    Ok(RegisteredTenantIamKeyV1 {
        schema_version: TENANT_IAM_KEY_SCHEMA_V1.to_owned(),
        tenant_id: id("tenant:alpha")?,
        issuer_id: id("issuer:alpha")?,
        key_id: id(key_id)?,
        purpose,
        public_key_hex: HEXLOWER.encode(signer.verifying_key().as_bytes()),
        valid_from: time("2026-07-21T10:00:00Z")?,
        valid_until: time("2026-07-21T12:00:00Z")?,
        revoked_at: None,
    })
}

fn resign_grant(grant: &mut SignedTenantAccessGrantV1, signer: &SigningKey) -> TestResult {
    grant.signature_hex = HEXLOWER.encode(&signer.sign(&grant.signing_message()?).to_bytes());
    Ok(())
}

fn resign_revocations(
    snapshot: &mut SignedTenantRevocationSnapshotV1,
    signer: &SigningKey,
) -> TestResult {
    snapshot.signature_hex = HEXLOWER.encode(&signer.sign(&snapshot.signing_message()?).to_bytes());
    Ok(())
}

fn id(value: &str) -> Result<ProtocolId, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}

fn time(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}

const fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([byte; 32])
}
