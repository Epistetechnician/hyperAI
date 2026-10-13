//! End-to-end contracts for the provider-agent preflight wrapper.

use std::{cell::Cell, collections::BTreeMap};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution::{
    CancellationSignalV1, ChatCompletionMessageV1, ChatCompletionRequestV1, ChatMessageRoleV1,
    LocalExecutionContextV1, LocalExecutionOutcomeV1, LocalProviderAgentV1, ProviderAgentErrorV1,
    ProviderExecutionPlanV1, ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1,
    ProviderStreamFrameV1, chat_completion_request_digest_v1,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    ProviderExecutionReceiptV1, ProviderExecutionStatementV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayProtectedMarketEvidenceBundleV4,
    ReplayStateTransitionV1, SignedInferenceRequestPolicyV1, SignedReliabilityArtifactV1,
    build_authenticated_execution_admission_v1, derive_replay_state_transition_v1,
    evaluate_reliability_artifact_verified_route_v1,
    verify_replay_protected_market_evidence_bundle_v4,
};
use hyperintel_persistence::{InitializeReplayStateOutcomeV1, SqliteReplayStoreV1};
use hyperintel_provider_agent::{
    ED25519_ALGORITHM_V1, PROVIDER_AGENT_KEY_SCHEMA_V1, PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1,
    PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1, PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1,
    ProviderAgentBindingV1, ProviderAgentExecutionContextV1, ProviderAgentKeyPurposeV1,
    ProviderAgentKeyRegistryV1, ProviderAgentReadinessClaimsV1, ProviderAgentReadinessStatusV1,
    ProviderAgentRevocationCheckpointV1, ProviderAgentRevocationSnapshotV1,
    ProviderAgentServiceManifestClaimsV1, ProviderTextStreamInterfaceV1,
    RegisteredProviderAgentKeyV1, SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1,
    SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1,
    SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1, SignedProviderAgentReadinessV1,
    SignedProviderAgentRevocationSnapshotV1, SignedProviderAgentServiceManifestV1,
    execute_provider_agent_v1,
};
use serde::Deserialize;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

struct ExecutionCase {
    policy: InferenceRequestPolicyV1,
    signed_policy: SignedInferenceRequestPolicyV1,
    replay_transition: ReplayStateTransitionV1,
    admission: AuthenticatedExecutionAdmissionV1,
}

struct ControlFixture {
    registry: ProviderAgentKeyRegistryV1,
    binding: ProviderAgentBindingV1,
    manifest: SignedProviderAgentServiceManifestV1,
    readiness: SignedProviderAgentReadinessV1,
    revocation: SignedProviderAgentRevocationSnapshotV1,
    checkpoint: ProviderAgentRevocationCheckpointV1,
    evaluated_at: UtcTimestamp,
}

struct ScriptedProvider {
    provider_id: ProtocolId,
    result: Option<Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>>,
    calls: u64,
}

impl LocalProviderAgentV1 for ScriptedProvider {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    fn execute(
        &mut self,
        _request: &ChatCompletionRequestV1,
        _admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1> {
        self.calls += 1;
        self.result
            .take()
            .unwrap_or(Err(ProviderAgentErrorV1::ScriptExhausted))
    }
}

struct TestReceiptSigner {
    provider_id: ProtocolId,
    key_id: ProtocolId,
    key: SigningKey,
    calls: Cell<u64>,
}

impl ProviderReceiptSignerV1 for TestReceiptSigner {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }
    fn key_id(&self) -> &ProtocolId {
        &self.key_id
    }

    fn sign(
        &self,
        receipt: &ProviderExecutionReceiptV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionStatementV1, ProviderReceiptSignerErrorV1> {
        self.calls.set(self.calls.get() + 1);
        let mut statement = ProviderExecutionStatementV1 {
            schema_version: "hyperintel.provider-execution-statement.v1".to_owned(),
            receipt_digest: receipt.receipt_digest,
            admission_digest: admission.admission_digest,
            issued_at: receipt.completed_at.clone(),
            expires_at: admission.deadline_at.clone(),
            sequence: 1,
            signer_id: self.provider_id.clone(),
            key_id: self.key_id.clone(),
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        statement.signature_hex = HEXLOWER.encode(
            &self
                .key
                .sign(
                    &statement
                        .signing_message()
                        .map_err(|_| ProviderReceiptSignerErrorV1::Unavailable)?,
                )
                .to_bytes(),
        );
        Ok(statement)
    }
}

#[tokio::test]
async fn contract_valid_control_enters_once_binds_lineage_retries_never_and_debug_redacts()
-> TestResult {
    let (fixture, registry, buyer_key) = execution_environment()?;
    let request = request(&fixture);
    let case = execution_case(&fixture, &registry, &buyer_key, &request)?;
    let authenticated = authenticated_context(&fixture, &case, &registry);
    let store = initialized_store(&fixture).await?;
    let control = control_fixture(&fixture, &case)?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = successful_provider(&fixture)?;
    let signer = receipt_signer()?;

    let result = execute_provider_agent_v1(
        provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
    )
    .await?;
    assert!(matches!(
        result.outcome(),
        LocalExecutionOutcomeV1::Completed(_)
    ));
    assert_eq!(provider.calls, 1);
    assert_eq!(signer.calls.get(), 1);
    assert_eq!(store.consumption_count().await?, 1);
    assert_eq!(result.admission_digest(), case.admission.admission_digest);
    assert_eq!(
        result.route_decision_digest(),
        case.admission.route_decision_digest
    );
    assert_eq!(result.selected_offer_digest(), case.admission.offer_digest);
    assert_eq!(result.manifest_digest(), control.manifest.digest()?);
    assert_eq!(result.readiness_digest(), control.readiness.digest()?);
    assert_eq!(
        result.revocation_snapshot_digest(),
        control.revocation.digest()?
    );
    assert_eq!(result.checkpoint_digest(), control.checkpoint.digest()?);
    assert!(!result.authority_granted());
    let debug = format!("{result:?}");
    assert!(!debug.contains("response-secret-sentinel"));
    assert!(debug.contains("[REDACTED]"));

    let retry = execute_provider_agent_v1(
        provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
    )
    .await;
    assert!(retry.is_err());
    assert_eq!(provider.calls, 1);
    assert_eq!(signer.calls.get(), 1);
    Ok(())
}

#[tokio::test]
async fn contract_stale_and_cross_binding_readiness_leave_same_admission_executable() -> TestResult
{
    for cross_binding in [false, true] {
        let (fixture, registry, buyer_key) = execution_environment()?;
        let request = request(&fixture);
        let case = execution_case(&fixture, &registry, &buyer_key, &request)?;
        let authenticated = authenticated_context(&fixture, &case, &registry);
        let store = initialized_store(&fixture).await?;
        let mut control = control_fixture(&fixture, &case)?;
        if cross_binding {
            control.readiness.claims.binding.selected_offer_digest =
                Sha256Digest::from_bytes([99; 32]);
            resign_readiness(&mut control.readiness, &SigningKey::from_bytes(&[92; 32]))?;
        } else {
            control.evaluated_at = timestamp("2026-07-20T12:06:00Z")?;
        }
        let cancellation = CancellationSignalV1::default();
        let mut provider = successful_provider(&fixture)?;
        let signer = receipt_signer()?;
        let denied = execute_provider_agent_v1(
            provider_context(
                &store,
                &case,
                &authenticated,
                &registry,
                &request,
                &cancellation,
                &control,
            ),
            &mut provider,
            &signer,
        )
        .await;
        assert!(denied.is_err());
        assert_eq!(provider.calls, 0);
        assert_eq!(signer.calls.get(), 0);
        assert_eq!(store.consumption_count().await?, 0);

        control = control_fixture(&fixture, &case)?;
        let completed = execute_provider_agent_v1(
            provider_context(
                &store,
                &case,
                &authenticated,
                &registry,
                &request,
                &cancellation,
                &control,
            ),
            &mut provider,
            &signer,
        )
        .await?;
        assert!(matches!(
            completed.outcome(),
            LocalExecutionOutcomeV1::Completed(_)
        ));
        assert_eq!(provider.calls, 1);
        assert_eq!(signer.calls.get(), 1);
        assert_eq!(store.consumption_count().await?, 1);
    }
    Ok(())
}

#[tokio::test]
async fn contract_unavailable_provider_consumes_once_returns_incomplete_without_fallback()
-> TestResult {
    let (fixture, registry, buyer_key) = execution_environment()?;
    let request = request(&fixture);
    let case = execution_case(&fixture, &registry, &buyer_key, &request)?;
    let authenticated = authenticated_context(&fixture, &case, &registry);
    let store = initialized_store(&fixture).await?;
    let control = control_fixture(&fixture, &case)?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = ScriptedProvider {
        provider_id: "provider:alpha".parse()?,
        result: Some(Err(ProviderAgentErrorV1::Unavailable)),
        calls: 0,
    };
    let signer = receipt_signer()?;
    let result = execute_provider_agent_v1(
        provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
    )
    .await?;
    assert!(matches!(
        result.outcome(),
        LocalExecutionOutcomeV1::Incomplete(_)
    ));
    assert_eq!(provider.calls, 1);
    assert_eq!(signer.calls.get(), 0);
    assert_eq!(store.consumption_count().await?, 1);
    Ok(())
}

#[tokio::test]
async fn contract_evaluation_time_must_equal_consumption_time_before_replay_or_ports() -> TestResult
{
    let (fixture, registry, buyer_key) = execution_environment()?;
    let request = request(&fixture);
    let case = execution_case(&fixture, &registry, &buyer_key, &request)?;
    let authenticated = authenticated_context(&fixture, &case, &registry);
    let store = initialized_store(&fixture).await?;
    let mut control = control_fixture(&fixture, &case)?;
    control.evaluated_at = timestamp("2026-07-20T11:59:59Z")?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = successful_provider(&fixture)?;
    let signer = receipt_signer()?;

    let denied = execute_provider_agent_v1(
        provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
    )
    .await;
    assert!(denied.is_err());
    assert_eq!(provider.calls, 0);
    assert_eq!(signer.calls.get(), 0);
    assert_eq!(store.consumption_count().await?, 0);

    control.evaluated_at = case.admission.admitted_at.clone();
    let completed = execute_provider_agent_v1(
        provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
    )
    .await?;
    assert!(matches!(
        completed.outcome(),
        LocalExecutionOutcomeV1::Completed(_)
    ));
    assert_eq!(provider.calls, 1);
    assert_eq!(signer.calls.get(), 1);
    assert_eq!(store.consumption_count().await?, 1);
    Ok(())
}

fn provider_context<'a>(
    store: &'a SqliteReplayStoreV1,
    case: &'a ExecutionCase,
    authenticated: &'a AuthenticatedExecutionContextV1<'a>,
    registry: &'a VerificationKeyRegistry,
    request: &'a ChatCompletionRequestV1,
    cancellation: &'a CancellationSignalV1,
    control: &'a ControlFixture,
) -> ProviderAgentExecutionContextV1<'a> {
    ProviderAgentExecutionContextV1 {
        local_execution: LocalExecutionContextV1 {
            replay_store: store,
            admission: &case.admission,
            authenticated,
            verification_keys: registry,
            request,
            consumed_at: &case.admission.admitted_at,
            cancellation,
        },
        manifest: &control.manifest,
        readiness: &control.readiness,
        revocation: &control.revocation,
        checkpoint: &control.checkpoint,
        key_registry: &control.registry,
        expected_binding: &control.binding,
        evaluated_at: &control.evaluated_at,
    }
}

fn execution_environment()
-> Result<(V4Fixture, VerificationKeyRegistry, SigningKey), Box<dyn std::error::Error>> {
    let fixture: V4Fixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/evidence-v4.json"
    ))?;
    let trusted: TrustedKeysFixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/trusted-keys-v4.json"
    ))?;
    let buyer_key = SigningKey::from_bytes(&[80; 32]);
    let mut registry = VerificationKeyRegistry::default();
    for key in trusted.trusted_verification_keys {
        registry.register(key)?;
    }
    register_market_key(
        &mut registry,
        "buyer:demo",
        "key:buyer-policy-1",
        AssuranceRootClassV1::BuyerPolicy,
        &buyer_key,
    )?;
    register_market_key(
        &mut registry,
        "provider:alpha",
        "key:alpha-receipt",
        AssuranceRootClassV1::ProviderKey,
        &SigningKey::from_bytes(&[81; 32]),
    )?;
    verify_replay_protected_market_evidence_bundle_v4(
        &fixture.bundle,
        &fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &registry,
    )?;
    Ok((fixture, registry, buyer_key))
}

fn execution_case(
    fixture: &V4Fixture,
    registry: &VerificationKeyRegistry,
    buyer_key: &SigningKey,
    request: &ChatCompletionRequestV1,
) -> Result<ExecutionCase, Box<dyn std::error::Error>> {
    let mut policy = fixture.bundle.policy.clone();
    policy.request_id = "request:provider-agent-alpha".parse()?;
    policy.allowed_providers.clear();
    policy.allowed_providers.insert("provider:alpha".parse()?);
    policy.prompt_digest = chat_completion_request_digest_v1(request)?;
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&routing_context(
        fixture, &policy, registry,
    ))?;
    let replay_transition = derive_replay_state_transition_v1(
        &fixture.bundle.replay_transition.before_state,
        &policy,
        evaluation.evaluation_digest,
        &fixture.bundle.evaluated_at,
    )?;
    let signed_policy = signed_buyer_policy(&policy, buyer_key)?;
    let authenticated = AuthenticatedExecutionContextV1 {
        routing: routing_context(fixture, &policy, registry),
        replay_transition: &replay_transition,
        signed_policy: &signed_policy,
    };
    let admission = build_authenticated_execution_admission_v1(
        &authenticated,
        registry,
        timestamp("2026-07-20T12:00:00Z")?,
        timestamp("2026-07-20T12:05:00Z")?,
    )?;
    Ok(ExecutionCase {
        policy,
        signed_policy,
        replay_transition,
        admission,
    })
}

fn authenticated_context<'a>(
    fixture: &'a V4Fixture,
    case: &'a ExecutionCase,
    registry: &'a VerificationKeyRegistry,
) -> AuthenticatedExecutionContextV1<'a> {
    AuthenticatedExecutionContextV1 {
        routing: routing_context(fixture, &case.policy, registry),
        replay_transition: &case.replay_transition,
        signed_policy: &case.signed_policy,
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

fn request(fixture: &V4Fixture) -> ChatCompletionRequestV1 {
    ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![ChatCompletionMessageV1 {
            role: ChatMessageRoleV1::User,
            content: "execute hermetic fixture".to_owned(),
        }],
        stream: true,
    }
}

async fn initialized_store(
    fixture: &V4Fixture,
) -> Result<SqliteReplayStoreV1, Box<dyn std::error::Error>> {
    let store = SqliteReplayStoreV1::in_memory().await?;
    store.migrate().await?;
    assert_eq!(
        store
            .initialize_buyer_state(
                &fixture.bundle.replay_transition.before_state,
                &fixture.bundle.evaluated_at
            )
            .await?,
        InitializeReplayStateOutcomeV1::Inserted
    );
    Ok(store)
}

fn successful_provider(
    fixture: &V4Fixture,
) -> Result<ScriptedProvider, Box<dyn std::error::Error>> {
    Ok(ScriptedProvider {
        provider_id: "provider:alpha".parse()?,
        result: Some(Ok(ProviderExecutionPlanV1 {
            provider_id: "provider:alpha".parse()?,
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            frames: vec![ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:02Z")?,
                content: "response-secret-sentinel".to_owned(),
                output_token_delta: 1,
            }],
            completed_at: timestamp("2026-07-20T12:00:03Z")?,
            input_tokens: 2,
            runtime_image_digest: fixture.bundle.service.runtime_image_digest,
        })),
        calls: 0,
    })
}

fn receipt_signer() -> Result<TestReceiptSigner, Box<dyn std::error::Error>> {
    Ok(TestReceiptSigner {
        provider_id: "provider:alpha".parse()?,
        key_id: "key:alpha-receipt".parse()?,
        key: SigningKey::from_bytes(&[81; 32]),
        calls: Cell::new(0),
    })
}

fn signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    key: &SigningKey,
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
    statement.signature_hex = HEXLOWER.encode(&key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedInferenceRequestPolicyV1 {
        schema_version: "hyperintel.signed-inference-request-policy.v1".to_owned(),
        policy: policy.clone(),
        statement,
    })
}

fn register_market_key(
    registry: &mut VerificationKeyRegistry,
    signer_id: &str,
    key_id: &str,
    class: AssuranceRootClassV1,
    key: &SigningKey,
) -> Result<(), Box<dyn std::error::Error>> {
    registry.register(RegisteredVerificationKeyV1 {
        signer_id: signer_id.parse()?,
        key_id: key_id.parse()?,
        trust_root_class: class,
        public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })?;
    Ok(())
}

fn control_fixture(
    fixture: &V4Fixture,
    case: &ExecutionCase,
) -> Result<ControlFixture, Box<dyn std::error::Error>> {
    let manifest_key = SigningKey::from_bytes(&[91; 32]);
    let readiness_key = SigningKey::from_bytes(&[92; 32]);
    let revocation_key = SigningKey::from_bytes(&[93; 32]);
    let registry = provider_agent_registry(&manifest_key, &readiness_key, &revocation_key)?;
    let service = &fixture.bundle.service;
    let provider = fixture
        .bundle
        .providers
        .get(&"provider:alpha".parse::<ProtocolId>()?)
        .ok_or("provider missing")?;
    let binding = ProviderAgentBindingV1 {
        provider_id: "provider:alpha".parse()?,
        agent_instance_id: "agent:alpha-1".parse()?,
        service_spec_digest: service.digest()?,
        selected_offer_digest: case.admission.offer_digest,
        runtime_configuration_digest: Sha256Digest::from_bytes([71; 32]),
        model_artifact_digest: service.model_artifact_digest,
        tokenizer_digest: service.tokenizer_digest,
        runtime_image_digest: service.runtime_image_digest,
        capabilities: service.capabilities.iter().cloned().collect(),
        receipt_key_id: provider.receipt_key_id.clone(),
        interface: ProviderTextStreamInterfaceV1::OpenAiTextStreamingChatCompletionsV1,
        secret_reference_digest: Sha256Digest::from_bytes([72; 32]),
    };
    let mut manifest = unsigned_manifest(&binding)?;
    manifest.signature_hex =
        HEXLOWER.encode(&manifest_key.sign(&manifest.signing_message()?).to_bytes());
    let mut readiness = unsigned_readiness(&binding, manifest.digest()?)?;
    resign_readiness(&mut readiness, &readiness_key)?;
    let mut revocation = unsigned_revocation(&binding)?;
    revocation.signature_hex = HEXLOWER.encode(
        &revocation_key
            .sign(&revocation.signing_message()?)
            .to_bytes(),
    );
    let checkpoint = ProviderAgentRevocationCheckpointV1 {
        provider_id: binding.provider_id.clone(),
        runtime_configuration_digest: binding.runtime_configuration_digest,
        agent_instance_id: binding.agent_instance_id.clone(),
        binding_digest: binding.digest()?,
        manifest_minimum_sequence: 1,
        manifest_digest_at_minimum_sequence: manifest.digest()?,
        readiness_minimum_sequence: 1,
        readiness_digest_at_minimum_sequence: readiness.digest()?,
        revocation_minimum_sequence: 1,
        revocation_digest_at_minimum_sequence: revocation.digest()?,
    };
    Ok(ControlFixture {
        registry,
        binding,
        manifest,
        readiness,
        revocation,
        checkpoint,
        evaluated_at: timestamp("2026-07-20T12:00:00Z")?,
    })
}

fn provider_agent_registry(
    manifest: &SigningKey,
    readiness: &SigningKey,
    revocation: &SigningKey,
) -> Result<ProviderAgentKeyRegistryV1, Box<dyn std::error::Error>> {
    let mut registry = ProviderAgentKeyRegistryV1::default();
    for (key, purpose, key_id) in [
        (
            manifest,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            "key:pa-manifest",
        ),
        (
            readiness,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:pa-readiness",
        ),
        (
            revocation,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
            "key:pa-revocation",
        ),
    ] {
        registry.register(RegisteredProviderAgentKeyV1 {
            schema_version: PROVIDER_AGENT_KEY_SCHEMA_V1.to_owned(),
            provider_id: "provider:alpha".parse()?,
            key_id: key_id.parse()?,
            purpose,
            public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
            valid_from: timestamp("2026-07-20T11:00:00Z")?,
            valid_until: timestamp("2026-07-20T13:00:00Z")?,
            revoked_at: None,
        })?;
    }
    Ok(registry)
}

fn unsigned_manifest(
    binding: &ProviderAgentBindingV1,
) -> Result<SignedProviderAgentServiceManifestV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentServiceManifestV1 {
        schema_version: SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1.to_owned(),
        claims: ProviderAgentServiceManifestClaimsV1 {
            schema_version: PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1.to_owned(),
            sequence: 1,
            binding: binding.clone(),
            issued_at: timestamp("2026-07-20T12:00:00Z")?,
            expires_at: timestamp("2026-07-20T12:30:00Z")?,
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:pa-manifest".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn unsigned_readiness(
    binding: &ProviderAgentBindingV1,
    manifest_digest: Sha256Digest,
) -> Result<SignedProviderAgentReadinessV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentReadinessV1 {
        schema_version: SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1.to_owned(),
        claims: ProviderAgentReadinessClaimsV1 {
            schema_version: PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1.to_owned(),
            sequence: 1,
            binding: binding.clone(),
            service_manifest_digest: manifest_digest,
            status: ProviderAgentReadinessStatusV1::Ready,
            issued_at: timestamp("2026-07-20T12:00:00Z")?,
            expires_at: timestamp("2026-07-20T12:05:00Z")?,
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:pa-readiness".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn unsigned_revocation(
    binding: &ProviderAgentBindingV1,
) -> Result<SignedProviderAgentRevocationSnapshotV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentRevocationSnapshotV1 {
        schema_version: SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
        snapshot: ProviderAgentRevocationSnapshotV1 {
            schema_version: PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
            provider_id: binding.provider_id.clone(),
            runtime_configuration_digest: binding.runtime_configuration_digest,
            sequence: 1,
            issued_at: timestamp("2026-07-20T12:00:00Z")?,
            effective_at: timestamp("2026-07-20T12:00:00Z")?,
            expires_at: timestamp("2026-07-20T12:05:00Z")?,
            assertion_revocations: vec![],
            key_revocations: vec![],
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:pa-revocation".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn resign_readiness(
    value: &mut SignedProviderAgentReadinessV1,
    key: &SigningKey,
) -> Result<(), Box<dyn std::error::Error>> {
    value.signature_hex = HEXLOWER.encode(&key.sign(&value.signing_message()?).to_bytes());
    Ok(())
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
