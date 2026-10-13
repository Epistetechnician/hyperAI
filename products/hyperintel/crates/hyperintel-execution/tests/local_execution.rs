//! Contract coverage for the hermetic local execution harness.

use std::{cell::Cell, collections::BTreeMap};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution::{
    CancellationSignalV1, ChatCompletionMessageV1, ChatCompletionRequestV1, ChatFinishReasonV1,
    ChatMessageRoleV1, IncompleteExecutionStatusV1, LocalExecutionContextV1, LocalExecutionError,
    LocalExecutionOutcomeV1, LocalProviderAgentV1, ProviderAgentErrorV1, ProviderExecutionPlanV1,
    ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1, ProviderStreamFrameV1,
    chat_completion_request_digest_v1, execute_local_chat_completion_v1,
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
use serde::Deserialize;

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

struct ScriptedProvider {
    provider_id: ProtocolId,
    plan: Option<Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>>,
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
        self.plan
            .take()
            .unwrap_or(Err(ProviderAgentErrorV1::ScriptExhausted))
    }
}

struct TestReceiptSigner {
    provider_id: ProtocolId,
    key_id: ProtocolId,
    signing_key: SigningKey,
    calls: Cell<u64>,
    unavailable: bool,
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
        if self.unavailable {
            return Err(ProviderReceiptSignerErrorV1::Unavailable);
        }
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
        let message = statement
            .signing_message()
            .map_err(|_| ProviderReceiptSignerErrorV1::Unavailable)?;
        statement.signature_hex = HEXLOWER.encode(&self.signing_key.sign(&message).to_bytes());
        Ok(statement)
    }
}

#[tokio::test]
async fn contract_two_selected_providers_complete_without_network_credentials_or_funds()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = execution_environment()?;
    for (provider, receipt_seed) in [("provider:alpha", 81_u8), ("provider:beta", 82_u8)] {
        let request = request(&fixture, provider);
        let execution_case = execution_case(&fixture, &registry, &buyer_key, &request, provider)?;
        let store = initialized_store(&fixture).await?;
        let mut agent = scripted_provider(provider, receipt_plan(provider, &fixture)?)?;
        let signer = receipt_signer(provider, receipt_seed)?;
        let authenticated = authenticated_context(
            &fixture,
            &execution_case.policy,
            &execution_case.signed_policy,
            &execution_case.replay_transition,
            &registry,
        );
        let outcome = execute_local_chat_completion_v1(
            local_context(
                &store,
                &execution_case,
                &authenticated,
                &registry,
                &request,
                &CancellationSignalV1::default(),
            ),
            &mut agent,
            &signer,
        )
        .await?;
        let LocalExecutionOutcomeV1::Completed(completed) = outcome else {
            return Err("selected provider did not complete".into());
        };
        assert_eq!(agent.calls, 1);
        assert_eq!(signer.calls.get(), 1);
        assert_eq!(
            completed.signed_receipt.receipt.provider_id.as_str(),
            provider
        );
        assert_eq!(completed.chunks.len(), 2);
        assert_eq!(
            completed
                .chunks
                .first()
                .and_then(|chunk| chunk.choices.first())
                .and_then(|choice| choice.delta.role),
            Some(ChatMessageRoleV1::Assistant)
        );
        assert_eq!(
            completed
                .chunks
                .last()
                .and_then(|chunk| chunk.choices.first())
                .and_then(|choice| choice.finish_reason),
            Some(ChatFinishReasonV1::Stop)
        );
        assert!(!completed.authority_granted);
        assert!(!completed.signed_receipt.receipt.authority_granted);
        assert!(!completed.metering_receipt.authority_granted);
        assert!(!completed.settlement_intent.authority_granted);
        assert_eq!(completed.digest()?, completed.completion_digest);
        assert_eq!(store.consumption_count().await?, 1);

        let retry = execute_local_chat_completion_v1(
            local_context(
                &store,
                &execution_case,
                &authenticated,
                &registry,
                &request,
                &CancellationSignalV1::default(),
            ),
            &mut agent,
            &signer,
        )
        .await;
        assert!(matches!(
            retry,
            Err(LocalExecutionError::AdmissionAlreadyConsumed)
        ));
        assert_eq!(agent.calls, 1);
        assert_eq!(signer.calls.get(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn contract_provider_substitution_is_rejected_before_replay_consumption()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = execution_environment()?;
    let request = request(&fixture, "provider:alpha");
    let execution_case =
        execution_case(&fixture, &registry, &buyer_key, &request, "provider:alpha")?;
    let store = initialized_store(&fixture).await?;
    let mut wrong_provider =
        scripted_provider("provider:beta", receipt_plan("provider:beta", &fixture)?)?;
    let signer = receipt_signer("provider:alpha", 81)?;
    let authenticated = authenticated_context(
        &fixture,
        &execution_case.policy,
        &execution_case.signed_policy,
        &execution_case.replay_transition,
        &registry,
    );
    let outcome = execute_local_chat_completion_v1(
        local_context(
            &store,
            &execution_case,
            &authenticated,
            &registry,
            &request,
            &CancellationSignalV1::default(),
        ),
        &mut wrong_provider,
        &signer,
    )
    .await;
    assert!(matches!(
        outcome,
        Err(LocalExecutionError::ProviderBindingMismatch)
    ));
    assert_eq!(wrong_provider.calls, 0);
    assert_eq!(store.consumption_count().await?, 0);

    let mut selected_provider =
        scripted_provider("provider:alpha", receipt_plan("provider:alpha", &fixture)?)?;
    let cancelled_before_consumption = CancellationSignalV1 {
        requested_at: Some(execution_case.admission.admitted_at.clone()),
    };
    let outcome = execute_local_chat_completion_v1(
        local_context(
            &store,
            &execution_case,
            &authenticated,
            &registry,
            &request,
            &cancelled_before_consumption,
        ),
        &mut selected_provider,
        &signer,
    )
    .await;
    assert!(matches!(
        outcome,
        Err(LocalExecutionError::CancelledBeforeConsumption)
    ));
    assert_eq!(selected_provider.calls, 0);
    assert_eq!(store.consumption_count().await?, 0);
    Ok(())
}

#[tokio::test]
async fn contract_cancellation_and_deadline_never_create_completed_economics()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = execution_environment()?;
    let provider = "provider:beta";
    let request = request(&fixture, provider);
    let execution_case = execution_case(&fixture, &registry, &buyer_key, &request, provider)?;
    let authenticated = authenticated_context(
        &fixture,
        &execution_case.policy,
        &execution_case.signed_policy,
        &execution_case.replay_transition,
        &registry,
    );
    let signer = receipt_signer(provider, 82)?;

    let cancellation_store = initialized_store(&fixture).await?;
    let cancellation = CancellationSignalV1 {
        requested_at: Some(timestamp("2026-07-20T12:00:03Z")?),
    };
    let mut cancelled_provider = scripted_provider(provider, receipt_plan(provider, &fixture)?)?;
    let cancelled = execute_local_chat_completion_v1(
        local_context(
            &cancellation_store,
            &execution_case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
        ),
        &mut cancelled_provider,
        &signer,
    )
    .await?;
    let LocalExecutionOutcomeV1::Incomplete(cancelled) = cancelled else {
        return Err("cancelled attempt produced completed economics".into());
    };
    assert_eq!(cancelled.status, IncompleteExecutionStatusV1::Cancelled);
    assert_eq!(cancelled.chunks.len(), 1);
    assert_eq!(cancelled.digest()?, cancelled.attempt_digest);
    assert_eq!(signer.calls.get(), 0);

    let deadline_store = initialized_store(&fixture).await?;
    let mut late_plan = receipt_plan(provider, &fixture)?;
    late_plan.frames.push(ProviderStreamFrameV1 {
        emitted_at: timestamp("2026-07-20T12:05:01Z")?,
        content: " late".to_owned(),
        output_token_delta: 1,
    });
    late_plan.completed_at = timestamp("2026-07-20T12:06:00Z")?;
    let mut late_provider = scripted_provider(provider, late_plan)?;
    let deadline = execute_local_chat_completion_v1(
        local_context(
            &deadline_store,
            &execution_case,
            &authenticated,
            &registry,
            &request,
            &CancellationSignalV1::default(),
        ),
        &mut late_provider,
        &signer,
    )
    .await?;
    let LocalExecutionOutcomeV1::Incomplete(deadline) = deadline else {
        return Err("late attempt produced completed economics".into());
    };
    assert_eq!(
        deadline.status,
        IncompleteExecutionStatusV1::DeadlineExceeded
    );
    assert_eq!(deadline.chunks.len(), 2);
    assert_eq!(signer.calls.get(), 0);
    Ok(())
}

#[tokio::test]
async fn contract_provider_and_signer_failures_have_no_completed_economics()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = execution_environment()?;
    let provider = "provider:beta";
    let request = request(&fixture, provider);
    let execution_case = execution_case(&fixture, &registry, &buyer_key, &request, provider)?;
    let authenticated = authenticated_context(
        &fixture,
        &execution_case.policy,
        &execution_case.signed_policy,
        &execution_case.replay_transition,
        &registry,
    );

    let provider_failure_store = initialized_store(&fixture).await?;
    let mut failed_provider = ScriptedProvider {
        provider_id: provider.parse()?,
        plan: Some(Err(ProviderAgentErrorV1::Unavailable)),
        calls: 0,
    };
    let signer = receipt_signer(provider, 82)?;
    let provider_failure = execute_local_chat_completion_v1(
        local_context(
            &provider_failure_store,
            &execution_case,
            &authenticated,
            &registry,
            &request,
            &CancellationSignalV1::default(),
        ),
        &mut failed_provider,
        &signer,
    )
    .await?;
    let LocalExecutionOutcomeV1::Incomplete(provider_failure) = provider_failure else {
        return Err("provider failure produced completed economics".into());
    };
    assert_eq!(
        provider_failure.status,
        IncompleteExecutionStatusV1::ProviderFailed
    );
    assert_eq!(
        provider_failure.provider_error,
        Some(ProviderAgentErrorV1::Unavailable)
    );
    assert!(provider_failure.chunks.is_empty());
    assert_eq!(signer.calls.get(), 0);

    let signer_failure_store = initialized_store(&fixture).await?;
    let mut completed_provider = scripted_provider(provider, receipt_plan(provider, &fixture)?)?;
    let mut unavailable_signer = receipt_signer(provider, 82)?;
    unavailable_signer.unavailable = true;
    let signer_failure = execute_local_chat_completion_v1(
        local_context(
            &signer_failure_store,
            &execution_case,
            &authenticated,
            &registry,
            &request,
            &CancellationSignalV1::default(),
        ),
        &mut completed_provider,
        &unavailable_signer,
    )
    .await?;
    let LocalExecutionOutcomeV1::Incomplete(signer_failure) = signer_failure else {
        return Err("signer failure produced completed economics".into());
    };
    assert_eq!(
        signer_failure.status,
        IncompleteExecutionStatusV1::ReceiptSigningFailed
    );
    assert_eq!(signer_failure.chunks.len(), 2);
    assert_eq!(unavailable_signer.calls.get(), 1);
    Ok(())
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
    register_key(
        &mut registry,
        "buyer:demo",
        "key:buyer-policy-1",
        AssuranceRootClassV1::BuyerPolicy,
        &buyer_key,
    )?;
    register_key(
        &mut registry,
        "provider:alpha",
        "key:alpha-receipt",
        AssuranceRootClassV1::ProviderKey,
        &SigningKey::from_bytes(&[81; 32]),
    )?;
    register_key(
        &mut registry,
        "provider:beta",
        "key:beta-receipt",
        AssuranceRootClassV1::ProviderKey,
        &SigningKey::from_bytes(&[82; 32]),
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
    provider: &str,
) -> Result<ExecutionCase, Box<dyn std::error::Error>> {
    let mut policy = fixture.bundle.policy.clone();
    policy.request_id = format!("request:local-{}", provider.replace(':', "-")).parse()?;
    policy.allowed_providers.clear();
    policy.allowed_providers.insert(provider.parse()?);
    policy.prompt_digest = chat_completion_request_digest_v1(request)?;
    let routing = routing_context(fixture, &policy, registry);
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&routing)?;
    let replay_transition = derive_replay_state_transition_v1(
        &fixture.bundle.replay_transition.before_state,
        &policy,
        evaluation.evaluation_digest,
        &fixture.bundle.evaluated_at,
    )?;
    let signed_policy = signed_buyer_policy(&policy, buyer_key)?;
    let authenticated = authenticated_context(
        fixture,
        &policy,
        &signed_policy,
        &replay_transition,
        registry,
    );
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
    policy: &'a InferenceRequestPolicyV1,
    signed_policy: &'a SignedInferenceRequestPolicyV1,
    transition: &'a ReplayStateTransitionV1,
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

fn local_context<'a>(
    store: &'a SqliteReplayStoreV1,
    execution_case: &'a ExecutionCase,
    authenticated: &'a AuthenticatedExecutionContextV1<'a>,
    registry: &'a VerificationKeyRegistry,
    request: &'a ChatCompletionRequestV1,
    cancellation: &'a CancellationSignalV1,
) -> LocalExecutionContextV1<'a> {
    LocalExecutionContextV1 {
        replay_store: store,
        admission: &execution_case.admission,
        authenticated,
        verification_keys: registry,
        request,
        consumed_at: &execution_case.admission.admitted_at,
        cancellation,
    }
}

fn request(fixture: &V4Fixture, provider: &str) -> ChatCompletionRequestV1 {
    ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::Developer,
                content: "Answer with one local fixture response.".to_owned(),
            },
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::User,
                content: format!("Exercise {provider}."),
            },
        ],
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
                &fixture.bundle.evaluated_at,
            )
            .await?,
        InitializeReplayStateOutcomeV1::Inserted
    );
    Ok(store)
}

fn receipt_plan(
    provider: &str,
    fixture: &V4Fixture,
) -> Result<ProviderExecutionPlanV1, Box<dyn std::error::Error>> {
    Ok(ProviderExecutionPlanV1 {
        provider_id: provider.parse()?,
        started_at: timestamp("2026-07-20T12:00:01Z")?,
        frames: vec![
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:02Z")?,
                content: "local ".to_owned(),
                output_token_delta: 1,
            },
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:03Z")?,
                content: provider.to_owned(),
                output_token_delta: 1,
            },
        ],
        completed_at: timestamp("2026-07-20T12:00:04Z")?,
        input_tokens: 10,
        runtime_image_digest: fixture.bundle.service.runtime_image_digest,
    })
}

fn scripted_provider(
    provider: &str,
    plan: ProviderExecutionPlanV1,
) -> Result<ScriptedProvider, Box<dyn std::error::Error>> {
    Ok(ScriptedProvider {
        provider_id: provider.parse()?,
        plan: Some(Ok(plan)),
        calls: 0,
    })
}

fn receipt_signer(
    provider: &str,
    receipt_seed: u8,
) -> Result<TestReceiptSigner, Box<dyn std::error::Error>> {
    let key_id = match provider {
        "provider:alpha" => "key:alpha-receipt",
        "provider:beta" => "key:beta-receipt",
        _ => return Err("unsupported test provider".into()),
    };
    Ok(TestReceiptSigner {
        provider_id: provider.parse()?,
        key_id: key_id.parse()?,
        signing_key: SigningKey::from_bytes(&[receipt_seed; 32]),
        calls: Cell::new(0),
        unavailable: false,
    })
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

fn register_key(
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

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
