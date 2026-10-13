//! Hermetic contracts for signed local secret injection.

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer, SigningKey};
use hyperintel_core::{ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_runtime_config::SecretReferenceV1;
use hyperintel_secret_provider::{
    ED25519_ALGORITHM_V1, InjectedSecretResolverErrorV1, InjectedSecretResolverV1,
    MAX_OPERATOR_SECRET_AUTHORIZATION_LIFETIME_SECONDS, MAX_SECRET_MATERIAL_BYTES,
    OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1, OPERATOR_SECRET_KEY_SCHEMA_V1,
    OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1, OperatorSecretAuthorizationClaimsV1,
    OperatorSecretAuthorizationExpectationV1, OperatorSecretAuthorizationRevocationV1,
    OperatorSecretAuthorizationVerificationContextV1, OperatorSecretKeyPurposeV1,
    OperatorSecretKeyRegistryV1, OperatorSecretKeyRevocationV1,
    OperatorSecretRevocationCheckpointV1, OperatorSecretRevocationStateV1,
    RegisteredOperatorSecretKeyV1, ResolvedSecretCandidateV1, ResolvedSecretV1,
    SECRET_PROVIDER_STATE_SLICE_V1, SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1,
    SecretMaterialV1, SecretProviderError, SecretPurposeV1, SecretResolutionRequestV1,
    SignedOperatorSecretAuthorizationV1, operator_secret_revocation_state_digest_v1,
    resolve_authorized_secret_v1, secret_reference_digest_v1,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SECRET_BYTES: &[u8] = b"synthetic-provider-api-credential";

struct Fixture {
    signing_key: SigningKey,
    operator_id: ProtocolId,
    key_id: ProtocolId,
    consumer_id: ProtocolId,
    selected_provider_id: ProtocolId,
    runtime_digest: Sha256Digest,
    operation_digest: Sha256Digest,
    reference: SecretReferenceV1,
    signed: SignedOperatorSecretAuthorizationV1,
    registry: OperatorSecretKeyRegistryV1,
    revocation_state: OperatorSecretRevocationStateV1,
    checkpoint: OperatorSecretRevocationCheckpointV1,
    evaluated_at: UtcTimestamp,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let signing_key = SigningKey::from_bytes(&[0x41; 32]);
        let operator_id = id("operator:secrets")?;
        let key_id = id("operator-secret-key-1")?;
        let consumer_id = id("gateway:local")?;
        let selected_provider_id = id("inference-provider:alpha")?;
        let runtime_digest = digest(0x22);
        let operation_digest = digest(0x33);
        let reference = SecretReferenceV1::new(
            id("fixture-secret-backend")?,
            id("sensitive-provider-api-handle")?,
            digest(0x44),
        )?;
        let claims = OperatorSecretAuthorizationClaimsV1 {
            schema_version: OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1.to_owned(),
            sequence: 7,
            operator_id: operator_id.clone(),
            runtime_configuration_digest: runtime_digest,
            consumer_id: consumer_id.clone(),
            selected_provider_id: selected_provider_id.clone(),
            secret_reference_digest: secret_reference_digest_v1(&reference)?,
            purpose: SecretPurposeV1::ProviderApiAuthentication,
            operation_context_digest: operation_digest,
            maximum_secret_bytes: 64,
            issued_at: timestamp("2026-07-21T12:00:00Z")?,
            expires_at: timestamp("2026-07-21T12:05:00Z")?,
        };
        let signed = sign_authorization(
            SignedOperatorSecretAuthorizationV1 {
                schema_version: SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1.to_owned(),
                claims,
                signer_id: operator_id.clone(),
                key_id: key_id.clone(),
                algorithm: ED25519_ALGORITHM_V1.to_owned(),
                signature_hex: String::new(),
            },
            &signing_key,
        )?;
        let mut registry = OperatorSecretKeyRegistryV1::default();
        registry.register(registered_key(&operator_id, &key_id, &signing_key, None)?)?;
        let revocation_state = OperatorSecretRevocationStateV1 {
            schema_version: OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1.to_owned(),
            operator_id: operator_id.clone(),
            runtime_configuration_digest: runtime_digest,
            sequence: 11,
            issued_at: timestamp("2026-07-21T12:00:00Z")?,
            effective_at: timestamp("2026-07-21T12:00:00Z")?,
            expires_at: timestamp("2026-07-21T12:05:00Z")?,
            authorization_revocations: Vec::new(),
            key_revocations: Vec::new(),
        };
        let checkpoint = checkpoint_for(&revocation_state)?;
        Ok(Self {
            signing_key,
            operator_id,
            key_id,
            consumer_id,
            selected_provider_id,
            runtime_digest,
            operation_digest,
            reference,
            signed,
            registry,
            revocation_state,
            checkpoint,
            evaluated_at: timestamp("2026-07-21T12:02:00Z")?,
        })
    }

    fn refresh_checkpoint(&mut self) -> TestResult {
        self.checkpoint = checkpoint_for(&self.revocation_state)?;
        Ok(())
    }
}

#[derive(Clone)]
struct ObservedRequest {
    runtime_digest: Sha256Digest,
    consumer_id: ProtocolId,
    selected_provider_id: ProtocolId,
    operation_digest: Sha256Digest,
    maximum_secret_bytes: u32,
    authorization_digest: Sha256Digest,
    signing_key_record_digest: Sha256Digest,
    revocation_state_digest: Sha256Digest,
    revocation_checkpoint_digest: Sha256Digest,
}

struct FixtureResolver {
    provider_id: ProtocolId,
    returned_provider_id: ProtocolId,
    returned_handle_id: ProtocolId,
    returned_version_digest: Sha256Digest,
    returned_purpose: SecretPurposeV1,
    bytes: Vec<u8>,
    fail: bool,
    calls: usize,
    observed: Option<ObservedRequest>,
}

impl FixtureResolver {
    fn exact(fixture: &Fixture) -> Self {
        Self {
            provider_id: fixture.reference.provider_id().clone(),
            returned_provider_id: fixture.reference.provider_id().clone(),
            returned_handle_id: fixture.reference.handle_id().clone(),
            returned_version_digest: fixture.reference.version_digest(),
            returned_purpose: SecretPurposeV1::ProviderApiAuthentication,
            bytes: SECRET_BYTES.to_vec(),
            fail: false,
            calls: 0,
            observed: None,
        }
    }
}

impl InjectedSecretResolverV1 for FixtureResolver {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    fn resolve(
        &mut self,
        request: &SecretResolutionRequestV1,
    ) -> Result<ResolvedSecretCandidateV1, InjectedSecretResolverErrorV1> {
        self.calls += 1;
        self.observed = Some(ObservedRequest {
            runtime_digest: request.runtime_configuration_digest(),
            consumer_id: request.consumer_id().clone(),
            selected_provider_id: request.selected_provider_id().clone(),
            operation_digest: request.operation_context_digest(),
            maximum_secret_bytes: request.maximum_secret_bytes(),
            authorization_digest: request.authorization_digest(),
            signing_key_record_digest: request.signing_key_record_digest(),
            revocation_state_digest: request.revocation_state_digest(),
            revocation_checkpoint_digest: request.revocation_checkpoint_digest(),
        });
        if self.fail {
            return Err(InjectedSecretResolverErrorV1::Unavailable);
        }
        ResolvedSecretCandidateV1::new(
            self.returned_provider_id.clone(),
            self.returned_handle_id.clone(),
            self.returned_version_digest,
            self.returned_purpose,
            self.bytes.clone(),
        )
        .map_err(|_| InjectedSecretResolverErrorV1::Unavailable)
    }
}

#[test]
fn contract_exact_signed_authorization_invokes_once_and_returns_redacted_material() -> TestResult {
    let fixture = Fixture::new()?;
    let mut resolver = FixtureResolver::exact(&fixture);
    let resolution = resolve_fixture(&fixture, &fixture.signed, &mut resolver)?;

    assert_eq!(
        SECRET_PROVIDER_STATE_SLICE_V1,
        "hyperintel-local-secret-injection-v1"
    );
    assert_eq!(resolver.calls, 1);
    assert_eq!(resolution.expose_secret_bytes(), SECRET_BYTES);
    assert_eq!(resolution.secret_reference(), &fixture.reference);
    assert_eq!(
        resolution.purpose(),
        SecretPurposeV1::ProviderApiAuthentication
    );
    assert_eq!(resolution.consumer_id(), &fixture.consumer_id);
    assert_eq!(
        resolution.selected_provider_id(),
        &fixture.selected_provider_id
    );
    assert_eq!(resolution.authorization_digest(), fixture.signed.digest()?);
    assert_eq!(
        resolution.runtime_configuration_digest(),
        fixture.runtime_digest
    );
    assert_eq!(
        resolution.operation_context_digest(),
        fixture.operation_digest
    );
    assert_eq!(resolution.evaluated_at(), &fixture.evaluated_at);
    assert!(!resolution.signing_key_record_digest().is_zero());
    assert_eq!(
        resolution.revocation_state_digest(),
        operator_secret_revocation_state_digest_v1(&fixture.revocation_state)?
    );
    assert!(!resolution.revocation_checkpoint_digest().is_zero());

    let observed = resolver.observed.as_ref().ok_or("missing request")?;
    assert_eq!(observed.runtime_digest, fixture.runtime_digest);
    assert_eq!(observed.consumer_id, fixture.consumer_id);
    assert_eq!(observed.selected_provider_id, fixture.selected_provider_id);
    assert_eq!(observed.operation_digest, fixture.operation_digest);
    assert_eq!(observed.maximum_secret_bytes, 64);
    assert_eq!(observed.authorization_digest, fixture.signed.digest()?);
    assert!(!observed.signing_key_record_digest.is_zero());
    assert_eq!(
        observed.revocation_state_digest,
        operator_secret_revocation_state_digest_v1(&fixture.revocation_state)?
    );
    assert!(!observed.revocation_checkpoint_digest.is_zero());
    assert_redacted(&format!("{resolution:?}"));
    Ok(())
}

#[test]
fn contract_original_signature_tamper_matrix_fails_before_resolution() -> TestResult {
    let fixture = Fixture::new()?;
    let mut variants = Vec::new();

    let mut value = fixture.signed.clone();
    value.schema_version = "hyperintel.signed-operator-secret-authorization.v2".to_owned();
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.schema_version = "hyperintel.operator-secret-authorization-claims.v2".to_owned();
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.algorithm = "ed448".to_owned();
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.signer_id = id("operator:other")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.key_id = id("operator-secret-key-other")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.sequence += 1;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.operator_id = id("operator:other")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.runtime_configuration_digest = digest(0x70);
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.consumer_id = id("gateway:other")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.selected_provider_id = id("inference-provider:beta")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.secret_reference_digest = digest(0x71);
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.operation_context_digest = digest(0x72);
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.maximum_secret_bytes = 63;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.issued_at = timestamp("2026-07-21T12:00:01Z")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.claims.expires_at = timestamp("2026-07-21T12:04:59Z")?;
    variants.push(value);
    let mut value = fixture.signed.clone();
    value.signature_hex.replace_range(0..2, "00");
    variants.push(value);

    for signed in variants {
        assert_preflight_rejected(&fixture, &signed);
    }

    let mut wrong_signature = fixture.signed.clone();
    wrong_signature = sign_authorization(wrong_signature, &SigningKey::from_bytes(&[0x52; 32]))?;
    assert_preflight_rejected(&fixture, &wrong_signature);

    let mut malformed = fixture.signed.clone();
    malformed.signature_hex = "not-hex".to_owned();
    assert_preflight_rejected(&fixture, &malformed);
    Ok(())
}

#[test]
fn contract_expectation_reference_registry_and_checkpoint_substitution_fail_preflight() -> TestResult
{
    for mutation in 0..7 {
        let mut fixture = Fixture::new()?;
        match mutation {
            0 => fixture.runtime_digest = digest(0x81),
            1 => fixture.consumer_id = id("gateway:other")?,
            2 => fixture.selected_provider_id = id("inference-provider:beta")?,
            3 => fixture.operation_digest = digest(0x82),
            4 => {
                fixture.reference = SecretReferenceV1::new(
                    id("other-secret-backend")?,
                    fixture.reference.handle_id().clone(),
                    fixture.reference.version_digest(),
                )?;
            }
            5 => {
                fixture.reference = SecretReferenceV1::new(
                    fixture.reference.provider_id().clone(),
                    id("other-sensitive-handle")?,
                    fixture.reference.version_digest(),
                )?;
            }
            6 => {
                fixture.reference = SecretReferenceV1::new(
                    fixture.reference.provider_id().clone(),
                    fixture.reference.handle_id().clone(),
                    digest(0x83),
                )?;
            }
            _ => return Err("unreachable mutation".into()),
        }
        assert_preflight_rejected(&fixture, &fixture.signed);
    }

    let mut fixture = Fixture::new()?;
    let wrong_key = SigningKey::from_bytes(&[0x54; 32]);
    fixture.registry = OperatorSecretKeyRegistryV1::default();
    fixture.registry.register(registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &wrong_key,
        None,
    )?)?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.checkpoint.minimum_sequence += 1;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.checkpoint.digest_at_minimum_sequence = digest(0x84);
    assert_preflight_rejected(&fixture, &fixture.signed);
    Ok(())
}

#[test]
fn contract_time_key_and_closed_revocation_edges_are_inclusive() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.evaluated_at = timestamp("2026-07-21T11:59:59Z")?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.evaluated_at = timestamp("2026-07-21T12:05:01Z")?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.signed.claims.expires_at = fixture
        .signed
        .claims
        .issued_at
        .checked_add_seconds(MAX_OPERATOR_SECRET_AUTHORIZATION_LIFETIME_SECONDS + 1)?;
    fixture.signed = sign_authorization(fixture.signed.clone(), &fixture.signing_key)?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.registry = OperatorSecretKeyRegistryV1::default();
    fixture.registry.register(registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &fixture.signing_key,
        Some(timestamp("2026-07-21T12:02:01Z")?),
    )?)?;
    let mut resolver = FixtureResolver::exact(&fixture);
    assert!(resolve_fixture(&fixture, &fixture.signed, &mut resolver).is_ok());
    assert_eq!(resolver.calls, 1);

    fixture.registry = OperatorSecretKeyRegistryV1::default();
    fixture.registry.register(registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &fixture.signing_key,
        Some(fixture.evaluated_at.clone()),
    )?)?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.revocation_state.authorization_revocations =
        vec![OperatorSecretAuthorizationRevocationV1 {
            operator_id: fixture.operator_id.clone(),
            authorization_digest: fixture.signed.digest()?,
            revoked_at: fixture.evaluated_at.clone(),
        }];
    fixture.refresh_checkpoint()?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.revocation_state.key_revocations = vec![OperatorSecretKeyRevocationV1 {
        operator_id: fixture.operator_id.clone(),
        key_id: fixture.key_id.clone(),
        purpose: OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
        revoked_at: fixture.evaluated_at.clone(),
    }];
    fixture.refresh_checkpoint()?;
    assert_preflight_rejected(&fixture, &fixture.signed);
    Ok(())
}

#[test]
fn contract_duplicate_key_and_logical_revocations_never_overwrite_or_pass() -> TestResult {
    let mut fixture = Fixture::new()?;
    let key = registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &fixture.signing_key,
        None,
    )?;
    let mut registry = OperatorSecretKeyRegistryV1::default();
    registry.register(key)?;
    let malicious = registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &SigningKey::from_bytes(&[0x61; 32]),
        None,
    )?;
    assert!(matches!(
        registry.register(malicious),
        Err(SecretProviderError::DuplicateIdentity)
    ));
    fixture.registry = registry;
    let mut resolver = FixtureResolver::exact(&fixture);
    assert!(resolve_fixture(&fixture, &fixture.signed, &mut resolver).is_ok());
    assert_eq!(resolver.calls, 1);

    let mut fixture = Fixture::new()?;
    let digest = fixture.signed.digest()?;
    fixture.revocation_state.authorization_revocations = vec![
        OperatorSecretAuthorizationRevocationV1 {
            operator_id: fixture.operator_id.clone(),
            authorization_digest: digest,
            revoked_at: timestamp("2026-07-21T12:01:00Z")?,
        },
        OperatorSecretAuthorizationRevocationV1 {
            operator_id: fixture.operator_id.clone(),
            authorization_digest: digest,
            revoked_at: timestamp("2026-07-21T12:02:00Z")?,
        },
    ];
    fixture.revocation_state.authorization_revocations.sort();
    fixture.refresh_checkpoint()?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.revocation_state.key_revocations = vec![
        OperatorSecretKeyRevocationV1 {
            operator_id: fixture.operator_id.clone(),
            key_id: fixture.key_id.clone(),
            purpose: OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
            revoked_at: timestamp("2026-07-21T12:01:00Z")?,
        },
        OperatorSecretKeyRevocationV1 {
            operator_id: fixture.operator_id.clone(),
            key_id: fixture.key_id.clone(),
            purpose: OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
            revoked_at: timestamp("2026-07-21T12:02:00Z")?,
        },
    ];
    fixture.revocation_state.key_revocations.sort();
    fixture.refresh_checkpoint()?;
    assert_preflight_rejected(&fixture, &fixture.signed);
    Ok(())
}

#[test]
fn contract_revocation_state_malformed_zero_cross_operator_and_unsorted_entries_fail_preflight()
-> TestResult {
    for mutation in 0..8 {
        let mut fixture = Fixture::new()?;
        match mutation {
            0 => {
                fixture.revocation_state.schema_version =
                    "hyperintel.operator-secret-revocation-state.v2".to_owned();
            }
            1 => fixture.revocation_state.sequence = 0,
            2 => fixture.revocation_state.runtime_configuration_digest = digest(0),
            3 => fixture.checkpoint.digest_at_minimum_sequence = digest(0),
            4 => {
                fixture.revocation_state.authorization_revocations =
                    vec![OperatorSecretAuthorizationRevocationV1 {
                        operator_id: id("operator:other")?,
                        authorization_digest: digest(0xa1),
                        revoked_at: timestamp("2026-07-21T12:01:00Z")?,
                    }];
                fixture.refresh_checkpoint()?;
            }
            5 => {
                fixture.revocation_state.key_revocations = vec![OperatorSecretKeyRevocationV1 {
                    operator_id: id("operator:other")?,
                    key_id: fixture.key_id.clone(),
                    purpose: OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
                    revoked_at: timestamp("2026-07-21T12:01:00Z")?,
                }];
                fixture.refresh_checkpoint()?;
            }
            6 => {
                fixture.revocation_state.authorization_revocations = vec![
                    OperatorSecretAuthorizationRevocationV1 {
                        operator_id: fixture.operator_id.clone(),
                        authorization_digest: digest(0xa2),
                        revoked_at: timestamp("2026-07-21T12:01:00Z")?,
                    },
                    OperatorSecretAuthorizationRevocationV1 {
                        operator_id: fixture.operator_id.clone(),
                        authorization_digest: digest(0xa1),
                        revoked_at: timestamp("2026-07-21T12:01:00Z")?,
                    },
                ];
                fixture.refresh_checkpoint()?;
            }
            7 => {
                fixture.revocation_state.authorization_revocations =
                    vec![OperatorSecretAuthorizationRevocationV1 {
                        operator_id: fixture.operator_id.clone(),
                        authorization_digest: digest(0),
                        revoked_at: timestamp("2026-07-21T12:01:00Z")?,
                    }];
                fixture.refresh_checkpoint()?;
            }
            _ => return Err("unreachable mutation".into()),
        }
        assert_preflight_rejected(&fixture, &fixture.signed);
    }
    Ok(())
}

#[test]
fn contract_authorization_revocation_and_key_window_endpoints_are_exact() -> TestResult {
    for evaluated_at in ["2026-07-21T12:00:00Z", "2026-07-21T12:05:00Z"] {
        let mut fixture = Fixture::new()?;
        fixture.evaluated_at = timestamp(evaluated_at)?;
        fixture.registry = OperatorSecretKeyRegistryV1::default();
        let mut exact_key = registered_key(
            &fixture.operator_id,
            &fixture.key_id,
            &fixture.signing_key,
            None,
        )?;
        exact_key.valid_from = fixture.signed.claims.issued_at.clone();
        exact_key.valid_until = fixture.signed.claims.expires_at.clone();
        fixture.registry.register(exact_key)?;
        let mut resolver = FixtureResolver::exact(&fixture);
        assert!(resolve_fixture(&fixture, &fixture.signed, &mut resolver).is_ok());
        assert_eq!(resolver.calls, 1);
    }

    let mut fixture = Fixture::new()?;
    fixture.revocation_state.effective_at = timestamp("2026-07-21T12:00:01Z")?;
    fixture.refresh_checkpoint()?;
    fixture.evaluated_at = timestamp("2026-07-21T12:00:00Z")?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.revocation_state.expires_at = timestamp("2026-07-21T12:04:59Z")?;
    fixture.refresh_checkpoint()?;
    fixture.evaluated_at = timestamp("2026-07-21T12:05:00Z")?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.registry = OperatorSecretKeyRegistryV1::default();
    let mut late_key = registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &fixture.signing_key,
        None,
    )?;
    late_key.valid_from = timestamp("2026-07-21T12:00:01Z")?;
    fixture.registry.register(late_key)?;
    assert_preflight_rejected(&fixture, &fixture.signed);

    let mut fixture = Fixture::new()?;
    fixture.registry = OperatorSecretKeyRegistryV1::default();
    let mut early_key = registered_key(
        &fixture.operator_id,
        &fixture.key_id,
        &fixture.signing_key,
        None,
    )?;
    early_key.valid_until = timestamp("2026-07-21T12:04:59Z")?;
    fixture.registry.register(early_key)?;
    assert_preflight_rejected(&fixture, &fixture.signed);
    Ok(())
}

#[test]
fn contract_every_resolver_metadata_mismatch_invokes_once_and_fails_closed() -> TestResult {
    let fixture = Fixture::new()?;
    let mut resolver = FixtureResolver::exact(&fixture);
    resolver.provider_id = id("other-secret-backend")?;
    assert!(matches!(
        resolve_fixture(&fixture, &fixture.signed, &mut resolver),
        Err(SecretProviderError::ResolverBindingMismatch)
    ));
    assert_eq!(resolver.calls, 0);

    for mutation in 0..4 {
        let fixture = Fixture::new()?;
        let mut resolver = FixtureResolver::exact(&fixture);
        match mutation {
            0 => resolver.returned_provider_id = id("other-secret-backend")?,
            1 => resolver.returned_handle_id = id("other-sensitive-handle")?,
            2 => resolver.returned_version_digest = digest(0x91),
            3 => {
                let wrong_purpose_json = "\"UNSUPPORTED_PURPOSE\"";
                assert!(serde_json::from_str::<SecretPurposeV1>(wrong_purpose_json).is_err());
                // V1 has no second purpose that could be substituted.
                resolver.returned_purpose = SecretPurposeV1::ProviderApiAuthentication;
                resolver.returned_version_digest = digest(0x92);
            }
            _ => return Err("unreachable mutation".into()),
        }
        assert!(matches!(
            resolve_fixture(&fixture, &fixture.signed, &mut resolver),
            Err(SecretProviderError::ResolverBindingMismatch)
        ));
        assert_eq!(resolver.calls, 1);
    }
    Ok(())
}

#[test]
fn contract_bounds_failure_no_retry_and_no_cache_are_explicit() -> TestResult {
    assert!(matches!(
        SecretMaterialV1::new(Vec::new()),
        Err(SecretProviderError::InvalidMaterial)
    ));
    assert!(matches!(
        SecretMaterialV1::new(vec![0x41; MAX_SECRET_MATERIAL_BYTES + 1]),
        Err(SecretProviderError::InvalidMaterial)
    ));

    let mut fixture = Fixture::new()?;
    fixture.signed.claims.maximum_secret_bytes = 8;
    fixture.signed = sign_authorization(fixture.signed.clone(), &fixture.signing_key)?;
    let mut resolver = FixtureResolver::exact(&fixture);
    assert!(matches!(
        resolve_fixture_with_maximum(&fixture, &fixture.signed, &mut resolver, 8),
        Err(SecretProviderError::InvalidMaterial)
    ));
    assert_eq!(resolver.calls, 1);

    let fixture = Fixture::new()?;
    let mut resolver = FixtureResolver::exact(&fixture);
    resolver.fail = true;
    assert!(matches!(
        resolve_fixture(&fixture, &fixture.signed, &mut resolver),
        Err(SecretProviderError::ResolverUnavailable)
    ));
    assert_eq!(resolver.calls, 1);

    let mut resolver = FixtureResolver::exact(&fixture);
    let first = resolve_fixture(&fixture, &fixture.signed, &mut resolver)?;
    drop(first);
    let second = resolve_fixture(&fixture, &fixture.signed, &mut resolver)?;
    drop(second);
    assert_eq!(resolver.calls, 2);
    Ok(())
}

#[test]
fn contract_zero_digests_limits_redaction_and_nonclone_nonserialization_hold() -> TestResult {
    for mutation in 0..4 {
        let mut fixture = Fixture::new()?;
        match mutation {
            0 => fixture.signed.claims.runtime_configuration_digest = digest(0),
            1 => fixture.signed.claims.secret_reference_digest = digest(0),
            2 => fixture.signed.claims.operation_context_digest = digest(0),
            3 => fixture.signed.claims.maximum_secret_bytes = 0,
            _ => return Err("unreachable mutation".into()),
        }
        fixture.signed = sign_authorization(fixture.signed.clone(), &fixture.signing_key)?;
        assert_preflight_rejected(&fixture, &fixture.signed);
    }

    let fixture = Fixture::new()?;
    let signed_debug = format!("{:?}", fixture.signed);
    assert_redacted(&signed_debug);
    let state_debug = format!("{:?}", fixture.revocation_state);
    assert!(!state_debug.contains("sensitive"));
    let mut material = SecretMaterialV1::new(SECRET_BYTES.to_vec())?;
    assert_redacted(&format!("{material:?}"));
    material.zeroize();
    assert!(material.is_empty());
    assert_eq!(material.len(), 0);

    assert_not_clone::<SecretMaterialV1>();
    assert_not_serialize::<SecretMaterialV1>();
    assert_not_clone::<ResolvedSecretV1>();
    assert_not_serialize::<ResolvedSecretV1>();
    Ok(())
}

fn resolve_fixture(
    fixture: &Fixture,
    signed: &SignedOperatorSecretAuthorizationV1,
    resolver: &mut FixtureResolver,
) -> Result<ResolvedSecretV1, SecretProviderError> {
    resolve_fixture_with_maximum(fixture, signed, resolver, 64)
}

fn resolve_fixture_with_maximum(
    fixture: &Fixture,
    signed: &SignedOperatorSecretAuthorizationV1,
    resolver: &mut FixtureResolver,
    maximum_secret_bytes: u32,
) -> Result<ResolvedSecretV1, SecretProviderError> {
    let expectation = OperatorSecretAuthorizationExpectationV1 {
        operator_id: &fixture.operator_id,
        runtime_configuration_digest: fixture.runtime_digest,
        consumer_id: &fixture.consumer_id,
        selected_provider_id: &fixture.selected_provider_id,
        secret_reference: &fixture.reference,
        purpose: SecretPurposeV1::ProviderApiAuthentication,
        operation_context_digest: fixture.operation_digest,
        maximum_secret_bytes,
    };
    let context = OperatorSecretAuthorizationVerificationContextV1 {
        key_registry: &fixture.registry,
        revocation_state: &fixture.revocation_state,
        revocation_checkpoint: &fixture.checkpoint,
        expectation: &expectation,
        evaluated_at: &fixture.evaluated_at,
    };
    resolve_authorized_secret_v1(resolver, signed, &context)
}

fn assert_preflight_rejected(fixture: &Fixture, signed: &SignedOperatorSecretAuthorizationV1) {
    let mut resolver = FixtureResolver::exact(fixture);
    assert!(resolve_fixture(fixture, signed, &mut resolver).is_err());
    assert_eq!(resolver.calls, 0);
}

fn checkpoint_for(
    state: &OperatorSecretRevocationStateV1,
) -> TestResult<OperatorSecretRevocationCheckpointV1> {
    Ok(OperatorSecretRevocationCheckpointV1 {
        operator_id: state.operator_id.clone(),
        runtime_configuration_digest: state.runtime_configuration_digest,
        minimum_sequence: state.sequence,
        digest_at_minimum_sequence: operator_secret_revocation_state_digest_v1(state)?,
    })
}

fn registered_key(
    operator_id: &ProtocolId,
    key_id: &ProtocolId,
    signing_key: &SigningKey,
    revoked_at: Option<UtcTimestamp>,
) -> TestResult<RegisteredOperatorSecretKeyV1> {
    Ok(RegisteredOperatorSecretKeyV1 {
        schema_version: OPERATOR_SECRET_KEY_SCHEMA_V1.to_owned(),
        operator_id: operator_id.clone(),
        key_id: key_id.clone(),
        purpose: OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
        public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-21T11:00:00Z")?,
        valid_until: timestamp("2026-07-21T13:00:00Z")?,
        revoked_at,
    })
}

fn sign_authorization(
    mut signed: SignedOperatorSecretAuthorizationV1,
    signing_key: &SigningKey,
) -> TestResult<SignedOperatorSecretAuthorizationV1> {
    signed.signature_hex.clear();
    signed.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&signed.signing_message()?).to_bytes());
    Ok(signed)
}

fn assert_redacted(debug: &str) {
    for forbidden in [
        "synthetic-provider-api-credential",
        "sensitive-provider-api-handle",
        &digest(0x44).to_string(),
        &digest(0x33).to_string(),
    ] {
        assert!(!debug.contains(forbidden));
    }
    assert!(debug.contains("REDACTED"));
}

fn assert_not_clone<T: ?Sized>() {
    trait AmbiguousIfClone<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}
    let _ = <T as AmbiguousIfClone<_>>::check;
}

fn assert_not_serialize<T: ?Sized>() {
    trait AmbiguousIfSerialize<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfSerialize<()> for T {}
    impl<T: ?Sized + serde::Serialize> AmbiguousIfSerialize<u8> for T {}
    let _ = <T as AmbiguousIfSerialize<_>>::check;
}

fn id(value: &str) -> TestResult<ProtocolId> {
    Ok(value.parse()?)
}

fn timestamp(value: &str) -> TestResult<UtcTimestamp> {
    Ok(value.parse()?)
}

const fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([byte; 32])
}
