//! Contract coverage for fail-closed authenticated local reroute lineage.

use std::{collections::BTreeMap, error::Error};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution::{
    AuthenticatedRerouteContextV1, AuthenticatedRerouteError, CancellationSignalV1,
    ChatCompletionMessageV1, ChatCompletionRequestV1, ChatMessageRoleV1,
    IncompleteExecutionStatusV1, IncompleteLocalExecutionV1, LocalExecutionContextV1,
    LocalExecutionOutcomeV1, LocalProviderAgentV1, ProviderAgentErrorV1, ProviderExecutionPlanV1,
    ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1, ProviderStreamFrameV1,
    chat_completion_request_digest_v1, execute_authenticated_reroute_v1,
    execute_local_chat_completion_v1,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    BuyerReplayStateV1, ProviderExecutionReceiptV1, ProviderExecutionStatementV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayProtectedMarketEvidenceBundleV4,
    ReplayStateTransitionV1, SignedInferenceRequestPolicyV1, SignedReliabilityArtifactV1,
    build_authenticated_execution_admission_v1, derive_replay_state_transition_v1,
    evaluate_reliability_artifact_verified_route_v1,
    verify_replay_protected_market_evidence_bundle_v4,
};
use hyperintel_persistence::{InitializeReplayStateOutcomeV1, SqliteReplayStoreV1};
use serde::Deserialize;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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

struct CaseSpec<'a> {
    request: &'a ChatCompletionRequestV1,
    before_state: &'a BuyerReplayStateV1,
    provider: &'a str,
    request_id: &'a str,
    admitted_at: &'a str,
}

struct PriorFixture {
    fixture: V4Fixture,
    registry: VerificationKeyRegistry,
    buyer_key: SigningKey,
    request: ChatCompletionRequestV1,
    store: SqliteReplayStoreV1,
    prior_case: ExecutionCase,
    prior_attempt: Box<IncompleteLocalExecutionV1>,
}

#[derive(Clone, Copy)]
enum PriorMode {
    Unavailable,
    Cancelled,
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
async fn contract_unavailable_provider_reroutes_through_new_signed_nonce_and_provider() -> TestResult
{
    let prior = prior_fixture(PriorMode::Unavailable).await?;
    let next_case = build_case(
        &prior,
        &prior.request,
        &prior.prior_attempt.replay_consumption.after_state,
        "provider:alpha",
        "request:reroute-alpha",
        "2026-07-20T12:00:05Z",
    )?;
    let prior_authenticated =
        authenticated_context(&prior.fixture, &prior.prior_case, &prior.registry);
    let next_authenticated = authenticated_context(&prior.fixture, &next_case, &prior.registry);
    let cancellation = CancellationSignalV1::default();
    let mut provider = scripted_provider(
        "provider:alpha",
        Ok(completion_plan("provider:alpha", &prior.fixture)?),
    )?;
    let signer = receipt_signer("provider:alpha", 81)?;
    let result = execute_authenticated_reroute_v1(
        AuthenticatedRerouteContextV1 {
            prior_attempt: &prior.prior_attempt,
            prior_admission: &prior.prior_case.admission,
            prior_authenticated: &prior_authenticated,
            next_execution: local_context(
                &prior.store,
                &next_case,
                &next_authenticated,
                &prior.registry,
                &prior.request,
                &cancellation,
            ),
        },
        &mut provider,
        &signer,
    )
    .await?;

    assert_eq!(result.reroute.prior_provider_id.as_str(), "provider:beta");
    assert_eq!(result.reroute.next_provider_id.as_str(), "provider:alpha");
    assert_eq!(result.reroute.prior_consumed_nonce, 1);
    assert_eq!(result.reroute.next_consumed_nonce, 2);
    assert_eq!(result.reroute.digest()?, result.reroute.reroute_digest);
    assert_eq!(result.digest()?, result.execution_digest);
    assert!(!result.authority_granted);
    assert!(matches!(
        result.execution,
        LocalExecutionOutcomeV1::Completed(_)
    ));
    assert_eq!(provider.calls, 1);
    assert_eq!(prior.store.consumption_count().await?, 2);
    assert_eq!(
        prior
            .store
            .current_state(&prior.fixture.bundle.policy.buyer_id)
            .await?,
        Some(next_case.replay_transition.after_state)
    );
    Ok(())
}

#[tokio::test]
async fn contract_reroute_rejects_cancelled_or_partial_prior_attempt() -> TestResult {
    let prior = prior_fixture(PriorMode::Cancelled).await?;
    assert_eq!(
        prior.prior_attempt.status,
        IncompleteExecutionStatusV1::Cancelled
    );
    assert_eq!(prior.prior_attempt.chunks.len(), 1);
    let next_case = build_case(
        &prior,
        &prior.request,
        &prior.prior_attempt.replay_consumption.after_state,
        "provider:alpha",
        "request:cancelled-reroute",
        "2026-07-20T12:00:05Z",
    )?;
    let error = rejected_reroute(&prior, &next_case, &prior.request).await?;
    assert!(matches!(
        error,
        AuthenticatedRerouteError::PriorAttemptNotReroutable
    ));
    assert_eq!(prior.store.consumption_count().await?, 1);
    Ok(())
}

#[tokio::test]
async fn contract_reroute_rejects_same_provider_prompt_change_and_stale_lineage() -> TestResult {
    let prior = prior_fixture(PriorMode::Unavailable).await?;
    let next_state = &prior.prior_attempt.replay_consumption.after_state;

    let same_provider = build_case(
        &prior,
        &prior.request,
        next_state,
        "provider:beta",
        "request:reroute-same-provider",
        "2026-07-20T12:00:05Z",
    )?;
    assert!(matches!(
        rejected_reroute(&prior, &same_provider, &prior.request).await?,
        AuthenticatedRerouteError::ProviderNotChanged
    ));

    let mut changed_request = prior.request.clone();
    changed_request.messages.push(ChatCompletionMessageV1 {
        role: ChatMessageRoleV1::User,
        content: "Change the committed request.".to_owned(),
    });
    let changed_prompt = build_case(
        &prior,
        &changed_request,
        next_state,
        "provider:alpha",
        "request:reroute-changed-prompt",
        "2026-07-20T12:00:05Z",
    )?;
    assert!(matches!(
        rejected_reroute(&prior, &changed_prompt, &changed_request).await?,
        AuthenticatedRerouteError::PromptBindingMismatch
    ));

    let stale = build_case(
        &prior,
        &prior.request,
        &prior.fixture.bundle.replay_transition.before_state,
        "provider:alpha",
        "request:reroute-stale",
        "2026-07-20T12:00:05Z",
    )?;
    assert!(matches!(
        rejected_reroute(&prior, &stale, &prior.request).await?,
        AuthenticatedRerouteError::ReplayLineageMismatch
    ));
    assert_eq!(prior.store.consumption_count().await?, 1);
    Ok(())
}

async fn rejected_reroute(
    prior: &PriorFixture,
    next_case: &ExecutionCase,
    request: &ChatCompletionRequestV1,
) -> TestResult<AuthenticatedRerouteError> {
    let prior_authenticated =
        authenticated_context(&prior.fixture, &prior.prior_case, &prior.registry);
    let next_authenticated = authenticated_context(&prior.fixture, next_case, &prior.registry);
    let cancellation = CancellationSignalV1::default();
    let mut provider = scripted_provider(
        next_case.admission.provider_id.as_str(),
        Ok(completion_plan(
            next_case.admission.provider_id.as_str(),
            &prior.fixture,
        )?),
    )?;
    let signer = signer_for_provider(next_case.admission.provider_id.as_str())?;
    let result = execute_authenticated_reroute_v1(
        AuthenticatedRerouteContextV1 {
            prior_attempt: &prior.prior_attempt,
            prior_admission: &prior.prior_case.admission,
            prior_authenticated: &prior_authenticated,
            next_execution: local_context(
                &prior.store,
                next_case,
                &next_authenticated,
                &prior.registry,
                request,
                &cancellation,
            ),
        },
        &mut provider,
        &signer,
    )
    .await;
    assert_eq!(provider.calls, 0);
    match result {
        Ok(_) => Err("invalid reroute unexpectedly executed".into()),
        Err(error) => Ok(error),
    }
}

async fn prior_fixture(mode: PriorMode) -> TestResult<PriorFixture> {
    let (fixture, registry, buyer_key) = environment()?;
    let request = request(&fixture);
    let prior_case = build_case_from_parts(
        &fixture,
        &registry,
        &buyer_key,
        &CaseSpec {
            request: &request,
            before_state: &fixture.bundle.replay_transition.before_state,
            provider: "provider:beta",
            request_id: "request:reroute-beta",
            admitted_at: "2026-07-20T12:00:00Z",
        },
    )?;
    let store = initialized_store(&fixture).await?;
    let authenticated = authenticated_context(&fixture, &prior_case, &registry);
    let cancellation = match mode {
        PriorMode::Unavailable => CancellationSignalV1::default(),
        PriorMode::Cancelled => CancellationSignalV1 {
            requested_at: Some(timestamp("2026-07-20T12:00:03Z")?),
        },
    };
    let plan = match mode {
        PriorMode::Unavailable => Err(ProviderAgentErrorV1::Unavailable),
        PriorMode::Cancelled => Ok(prior_plan("provider:beta", &fixture)?),
    };
    let mut provider = scripted_provider("provider:beta", plan)?;
    let signer = receipt_signer("provider:beta", 82)?;
    let outcome = execute_local_chat_completion_v1(
        local_context(
            &store,
            &prior_case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
        ),
        &mut provider,
        &signer,
    )
    .await?;
    let LocalExecutionOutcomeV1::Incomplete(prior_attempt) = outcome else {
        return Err("prior attempt unexpectedly completed".into());
    };
    Ok(PriorFixture {
        fixture,
        registry,
        buyer_key,
        request,
        store,
        prior_case,
        prior_attempt,
    })
}

fn build_case(
    prior: &PriorFixture,
    request: &ChatCompletionRequestV1,
    before_state: &BuyerReplayStateV1,
    provider: &str,
    request_id: &str,
    admitted_at: &str,
) -> TestResult<ExecutionCase> {
    build_case_from_parts(
        &prior.fixture,
        &prior.registry,
        &prior.buyer_key,
        &CaseSpec {
            request,
            before_state,
            provider,
            request_id,
            admitted_at,
        },
    )
}

fn build_case_from_parts(
    fixture: &V4Fixture,
    registry: &VerificationKeyRegistry,
    buyer_key: &SigningKey,
    spec: &CaseSpec<'_>,
) -> TestResult<ExecutionCase> {
    let mut policy = fixture.bundle.policy.clone();
    policy.request_id = spec.request_id.parse()?;
    policy.nonce = spec.before_state.next_nonce;
    policy.replay_snapshot_digest = spec.before_state.state_digest;
    policy.allowed_providers.clear();
    policy.allowed_providers.insert(spec.provider.parse()?);
    policy.prompt_digest = chat_completion_request_digest_v1(spec.request)?;
    let routing = routing_context(fixture, &policy, registry);
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&routing)?;
    let replay_transition = derive_replay_state_transition_v1(
        spec.before_state,
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
        timestamp(spec.admitted_at)?,
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
    execution_case: &'a ExecutionCase,
    registry: &'a VerificationKeyRegistry,
) -> AuthenticatedExecutionContextV1<'a> {
    AuthenticatedExecutionContextV1 {
        routing: routing_context(fixture, &execution_case.policy, registry),
        replay_transition: &execution_case.replay_transition,
        signed_policy: &execution_case.signed_policy,
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

fn environment() -> TestResult<(V4Fixture, VerificationKeyRegistry, SigningKey)> {
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

fn request(fixture: &V4Fixture) -> ChatCompletionRequestV1 {
    ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::Developer,
                content: "Return one deterministic local response.".to_owned(),
            },
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::User,
                content: "Exercise authenticated reroute.".to_owned(),
            },
        ],
        stream: true,
    }
}

async fn initialized_store(fixture: &V4Fixture) -> TestResult<SqliteReplayStoreV1> {
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

fn prior_plan(provider: &str, fixture: &V4Fixture) -> TestResult<ProviderExecutionPlanV1> {
    Ok(ProviderExecutionPlanV1 {
        provider_id: provider.parse()?,
        started_at: timestamp("2026-07-20T12:00:01Z")?,
        frames: vec![
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:02Z")?,
                content: "partial ".to_owned(),
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

fn completion_plan(provider: &str, fixture: &V4Fixture) -> TestResult<ProviderExecutionPlanV1> {
    Ok(ProviderExecutionPlanV1 {
        provider_id: provider.parse()?,
        started_at: timestamp("2026-07-20T12:00:06Z")?,
        frames: vec![
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:07Z")?,
                content: "rerouted ".to_owned(),
                output_token_delta: 1,
            },
            ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:08Z")?,
                content: provider.to_owned(),
                output_token_delta: 1,
            },
        ],
        completed_at: timestamp("2026-07-20T12:00:09Z")?,
        input_tokens: 10,
        runtime_image_digest: fixture.bundle.service.runtime_image_digest,
    })
}

fn scripted_provider(
    provider: &str,
    plan: Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>,
) -> TestResult<ScriptedProvider> {
    Ok(ScriptedProvider {
        provider_id: provider.parse()?,
        plan: Some(plan),
        calls: 0,
    })
}

fn signer_for_provider(provider: &str) -> TestResult<TestReceiptSigner> {
    match provider {
        "provider:alpha" => receipt_signer(provider, 81),
        "provider:beta" => receipt_signer(provider, 82),
        _ => Err("unsupported test provider".into()),
    }
}

fn receipt_signer(provider: &str, seed: u8) -> TestResult<TestReceiptSigner> {
    let key_id = match provider {
        "provider:alpha" => "key:alpha-receipt",
        "provider:beta" => "key:beta-receipt",
        _ => return Err("unsupported test provider".into()),
    };
    Ok(TestReceiptSigner {
        provider_id: provider.parse()?,
        key_id: key_id.parse()?,
        signing_key: SigningKey::from_bytes(&[seed; 32]),
    })
}

fn signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    signing_key: &SigningKey,
) -> TestResult<SignedInferenceRequestPolicyV1> {
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
) -> TestResult {
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

fn timestamp(value: &str) -> TestResult<UtcTimestamp> {
    Ok(value.parse()?)
}
