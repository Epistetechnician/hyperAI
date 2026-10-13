//! Contract coverage for local authenticated replay persistence.

use std::collections::BTreeMap;

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{InferenceRequestPolicyV1, Sha256Digest, UtcTimestamp};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayProtectedMarketEvidenceBundleV4,
    ReplayStateTransitionV1, SignedInferenceRequestPolicyV1, SignedReliabilityArtifactV1,
    build_authenticated_execution_admission_v1, build_buyer_replay_state_v1,
    derive_replay_state_transition_v1, evaluate_reliability_artifact_verified_route_v1,
    verify_replay_protected_market_evidence_bundle_v4,
};
use hyperintel_persistence::{
    ConsumeReplayOutcomeV1, InitializeReplayStateOutcomeV1, ReplayPersistenceError,
    SqliteReplayStoreV1,
};
use serde::Deserialize;
use tempfile::tempdir;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct V4Fixture {
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    bundle: ReplayProtectedMarketEvidenceBundleV4,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedKeysFixture {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

struct AdmissionFixture {
    fixture: V4Fixture,
    registry: VerificationKeyRegistry,
    signed_policy: SignedInferenceRequestPolicyV1,
    competing_policy: InferenceRequestPolicyV1,
    competing_signed_policy: SignedInferenceRequestPolicyV1,
    competing_transition: ReplayStateTransitionV1,
    original_admission: AuthenticatedExecutionAdmissionV1,
    competing_admission: AuthenticatedExecutionAdmissionV1,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contract_authenticated_replay_compare_and_swap_is_atomic_idempotent_and_restart_safe()
-> Result<(), Box<dyn std::error::Error>> {
    let inputs = admission_fixture()?;
    let original_context = authenticated_context(
        &inputs.fixture,
        &inputs.fixture.bundle.policy,
        &inputs.signed_policy,
        &inputs.fixture.bundle.replay_transition,
        &inputs.registry,
    );
    let competing_context = AuthenticatedExecutionContextV1 {
        routing: routing_context(&inputs.fixture, &inputs.competing_policy, &inputs.registry),
        replay_transition: &inputs.competing_transition,
        signed_policy: &inputs.competing_signed_policy,
    };

    let directory = tempdir()?;
    let database_path = directory.path().join("durable-replay.sqlite");
    let store = initialized_store(&database_path, &inputs).await?;

    assert_invalid_buyer_signature_does_not_consume(
        &store,
        &inputs.fixture,
        &inputs.registry,
        &inputs.signed_policy,
        &inputs.original_admission,
    )
    .await?;

    let consumed_at = timestamp("2026-07-20T12:00:00Z")?;
    let (original_result, competing_result) = tokio::join!(
        store.consume_authenticated_admission(
            &inputs.original_admission,
            &original_context,
            &inputs.registry,
            &consumed_at,
        ),
        store.consume_authenticated_admission(
            &inputs.competing_admission,
            &competing_context,
            &inputs.registry,
            &consumed_at,
        )
    );
    assert_eq!(
        [&original_result, &competing_result]
            .into_iter()
            .filter(|result| matches!(result, Ok(ConsumeReplayOutcomeV1::Applied(_))))
            .count(),
        1
    );
    assert_eq!(
        [&original_result, &competing_result]
            .into_iter()
            .filter(|result| matches!(result, Err(ReplayPersistenceError::StaleState)))
            .count(),
        1
    );

    let (winning_admission, winning_context) =
        if matches!(original_result, Ok(ConsumeReplayOutcomeV1::Applied(_))) {
            (&inputs.original_admission, &original_context)
        } else {
            (&inputs.competing_admission, &competing_context)
        };
    let exact_retry = store
        .consume_authenticated_admission(
            winning_admission,
            winning_context,
            &inputs.registry,
            &consumed_at,
        )
        .await?;
    let ConsumeReplayOutcomeV1::ExistingIdentical(consumption) = exact_retry else {
        return Err("exact authenticated replay was not idempotent".into());
    };
    assert!(!consumption.authority_granted);
    assert!(!consumption.consumption_digest.is_zero());
    assert_eq!(store.consumption_count().await?, 1);
    assert_eq!(
        store
            .current_state(&inputs.fixture.bundle.policy.buyer_id)
            .await?,
        Some(inputs.fixture.bundle.replay_transition.after_state.clone())
    );
    assert_eq!(
        store
            .consumption(winning_admission.admission_digest)
            .await?,
        Some(consumption)
    );

    store.checkpoint_and_close().await?;
    let reopened = SqliteReplayStoreV1::connect_path(&database_path, 2).await?;
    reopened.migrate().await?;
    assert_eq!(
        reopened
            .current_state(&inputs.fixture.bundle.policy.buyer_id)
            .await?,
        Some(inputs.fixture.bundle.replay_transition.after_state)
    );
    assert_eq!(reopened.consumption_count().await?, 1);
    reopened.checkpoint_and_close().await?;
    Ok(())
}

fn admission_fixture() -> Result<AdmissionFixture, Box<dyn std::error::Error>> {
    let fixture: V4Fixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/evidence-v4.json"
    ))?;
    let trusted: TrustedKeysFixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/trusted-keys-v4.json"
    ))?;
    let buyer_signing_key = SigningKey::from_bytes(&[80; 32]);
    let registry = execution_registry(&trusted.trusted_verification_keys, &buyer_signing_key)?;
    verify_replay_protected_market_evidence_bundle_v4(
        &fixture.bundle,
        &fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &registry,
    )?;

    let signed_policy = signed_buyer_policy(&fixture.bundle.policy, &buyer_signing_key)?;
    let original_context = authenticated_context(
        &fixture,
        &fixture.bundle.policy,
        &signed_policy,
        &fixture.bundle.replay_transition,
        &registry,
    );
    let original_admission = build_authenticated_execution_admission_v1(
        &original_context,
        &registry,
        timestamp("2026-07-20T12:00:00Z")?,
        timestamp("2026-07-20T12:05:00Z")?,
    )?;

    let mut competing_policy = fixture.bundle.policy.clone();
    competing_policy.request_id = "request:demo-2".parse()?;
    let competing_signed_policy = signed_buyer_policy(&competing_policy, &buyer_signing_key)?;
    let competing_routing = routing_context(&fixture, &competing_policy, &registry);
    let competing_evaluation = evaluate_reliability_artifact_verified_route_v1(&competing_routing)?;
    let competing_transition = derive_replay_state_transition_v1(
        &fixture.bundle.replay_transition.before_state,
        &competing_policy,
        competing_evaluation.evaluation_digest,
        &fixture.bundle.evaluated_at,
    )?;
    let competing_context = AuthenticatedExecutionContextV1 {
        routing: competing_routing,
        replay_transition: &competing_transition,
        signed_policy: &competing_signed_policy,
    };
    let competing_admission = build_authenticated_execution_admission_v1(
        &competing_context,
        &registry,
        timestamp("2026-07-20T12:00:00Z")?,
        timestamp("2026-07-20T12:05:00Z")?,
    )?;
    Ok(AdmissionFixture {
        fixture,
        registry,
        signed_policy,
        competing_policy,
        competing_signed_policy,
        competing_transition,
        original_admission,
        competing_admission,
    })
}

async fn initialized_store(
    database_path: &std::path::Path,
    inputs: &AdmissionFixture,
) -> Result<SqliteReplayStoreV1, Box<dyn std::error::Error>> {
    assert!(matches!(
        SqliteReplayStoreV1::connect_path(database_path, 0).await,
        Err(ReplayPersistenceError::InvalidConnectionCount)
    ));
    let store = SqliteReplayStoreV1::connect_path(database_path, 4).await?;
    store.migrate().await?;
    let before_state = &inputs.fixture.bundle.replay_transition.before_state;
    let evaluated_at = &inputs.fixture.bundle.evaluated_at;
    assert_eq!(
        store
            .initialize_buyer_state(before_state, evaluated_at)
            .await?,
        InitializeReplayStateOutcomeV1::Inserted
    );
    assert_eq!(
        store
            .initialize_buyer_state(before_state, evaluated_at)
            .await?,
        InitializeReplayStateOutcomeV1::ExistingIdentical
    );
    let conflicting_initial = build_buyer_replay_state_v1(
        inputs.fixture.bundle.policy.buyer_id.clone(),
        inputs.fixture.bundle.policy.nonce + 4,
    )?;
    assert!(matches!(
        store
            .initialize_buyer_state(&conflicting_initial, evaluated_at)
            .await,
        Err(ReplayPersistenceError::InitialStateConflict)
    ));
    Ok(store)
}

async fn assert_invalid_buyer_signature_does_not_consume(
    store: &SqliteReplayStoreV1,
    fixture: &V4Fixture,
    registry: &VerificationKeyRegistry,
    signed_policy: &SignedInferenceRequestPolicyV1,
    admission: &hyperintel_market::AuthenticatedExecutionAdmissionV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid_policy = signed_policy.clone();
    let replacement = if invalid_policy.statement.signature_hex.starts_with("00") {
        "01"
    } else {
        "00"
    };
    invalid_policy
        .statement
        .signature_hex
        .replace_range(0..2, replacement);
    let invalid_context = authenticated_context(
        fixture,
        &fixture.bundle.policy,
        &invalid_policy,
        &fixture.bundle.replay_transition,
        registry,
    );
    assert!(matches!(
        store
            .consume_authenticated_admission(
                admission,
                &invalid_context,
                registry,
                &fixture.bundle.evaluated_at,
            )
            .await,
        Err(ReplayPersistenceError::Execution(_))
    ));
    assert_eq!(store.consumption_count().await?, 0);
    assert_eq!(
        store.current_state(&fixture.bundle.policy.buyer_id).await?,
        Some(fixture.bundle.replay_transition.before_state.clone())
    );
    Ok(())
}

fn authenticated_context<'a>(
    fixture: &'a V4Fixture,
    policy: &'a InferenceRequestPolicyV1,
    signed_policy: &'a SignedInferenceRequestPolicyV1,
    transition: &'a hyperintel_market::ReplayStateTransitionV1,
    registry: &'a VerificationKeyRegistry,
) -> AuthenticatedExecutionContextV1<'a> {
    AuthenticatedExecutionContextV1 {
        routing: routing_context(fixture, policy, registry),
        replay_transition: transition,
        signed_policy,
    }
}

fn routing_context<'a>(
    fixture: &'a V4Fixture,
    policy: &'a InferenceRequestPolicyV1,
    registry: &'a VerificationKeyRegistry,
) -> ReliabilityArtifactVerifiedRoutingContextV1<'a> {
    ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &fixture.bundle.evaluated_at,
        service: &fixture.bundle.service,
        providers: &fixture.bundle.providers,
        offers: &fixture.bundle.offers,
        policy,
        assurance_sets: &fixture.bundle.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        reliability_artifacts: &fixture.reliability_artifacts,
        verification_keys: registry,
    }
}

fn signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    signing_key: &SigningKey,
) -> Result<SignedInferenceRequestPolicyV1, Box<dyn std::error::Error>> {
    let mut statement = BuyerPolicyStatementV1 {
        schema_version: "hyperintel.buyer-policy-statement.v1".to_owned(),
        request_policy_digest: policy.digest()?,
        replay_snapshot_digest: policy.replay_snapshot_digest,
        issued_at: policy.valid_from.clone(),
        expires_at: policy.valid_until.clone(),
        sequence: policy.nonce,
        signer_id: policy.buyer_id.clone(),
        key_id: "key:buyer-policy-1".parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedInferenceRequestPolicyV1 {
        schema_version: "hyperintel.signed-inference-request-policy.v1".to_owned(),
        policy: policy.clone(),
        statement,
    })
}

fn execution_registry(
    existing_keys: &[RegisteredVerificationKeyV1],
    buyer_signing_key: &SigningKey,
) -> Result<VerificationKeyRegistry, Box<dyn std::error::Error>> {
    let mut registry = VerificationKeyRegistry::default();
    for key in existing_keys {
        registry.register(key.clone())?;
    }
    registry.register(RegisteredVerificationKeyV1 {
        signer_id: "buyer:demo".parse()?,
        key_id: "key:buyer-policy-1".parse()?,
        trust_root_class: AssuranceRootClassV1::BuyerPolicy,
        public_key_hex: HEXLOWER.encode(buyer_signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })?;
    Ok(registry)
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
