use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperai_hyperintel_adapter::HyperIntelReconciliationInputV1;
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution::{
    CancellationSignalV1, ChatCompletionMessageV1, ChatCompletionRequestV1, ChatMessageRoleV1,
    LocalExecutionContextV1, LocalProviderAgentV1, ProviderAgentErrorV1, ProviderExecutionPlanV1,
    ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1, ProviderStreamFrameV1,
    chat_completion_request_digest_v1,
};
use hyperintel_execution_journal::{
    SqliteExecutionJournalV1, execute_journaled_local_chat_completion_v1,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    ProviderExecutionReceiptV1, ProviderExecutionStatementV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayStateTransitionV1,
    SignedInferenceRequestPolicyV1, build_authenticated_execution_admission_v1,
    derive_replay_state_transition_v1, evaluate_reliability_artifact_verified_route_v1,
    verify_replay_protected_market_evidence_bundle_v4,
};
use hyperintel_persistence::{InitializeReplayStateOutcomeV1, SqliteReplayStoreV1};
use serde::Deserialize;

pub const STREAM_CANARY: &str = "HYPERAI_SYNTHETIC_RECONCILIATION_STREAM_CANARY";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct V4Fixture {
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    bundle: hyperintel_market::ReplayProtectedMarketEvidenceBundleV4,
    reliability_artifacts: BTreeMap<Sha256Digest, hyperintel_market::SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedKeysFixture {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

struct Environment {
    fixture: V4Fixture,
    registry: VerificationKeyRegistry,
    buyer_key: SigningKey,
    trusted_keys: Vec<RegisteredVerificationKeyV1>,
}

struct ExecutionCase {
    policy: InferenceRequestPolicyV1,
    signed_policy: SignedInferenceRequestPolicyV1,
    replay_transition: ReplayStateTransitionV1,
    admission: AuthenticatedExecutionAdmissionV1,
}

struct ScriptedProvider {
    provider_id: ProtocolId,
    plan: Option<ProviderExecutionPlanV1>,
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
        self.plan
            .take()
            .ok_or(ProviderAgentErrorV1::ScriptExhausted)
    }
}

struct SyntheticReceiptSigner {
    provider_id: ProtocolId,
    key_id: ProtocolId,
    signing_key: SigningKey,
}

impl ProviderReceiptSignerV1 for SyntheticReceiptSigner {
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

pub async fn build_completed_input() -> Result<HyperIntelReconciliationInputV1, Box<dyn Error>> {
    let mut environment = load_environment()?;
    let request = request(&environment.fixture);
    let execution_case = execution_case(&mut environment, &request)?;
    let replay_store = initialized_store(&environment.fixture).await?;
    let journal = SqliteExecutionJournalV1::in_memory().await?;
    journal.migrate().await?;

    let authenticated = authenticated_context(&environment, &execution_case);
    let cancellation = CancellationSignalV1::default();
    let mut provider = ScriptedProvider {
        provider_id: "provider:alpha".parse()?,
        plan: Some(execution_plan(&environment.fixture)?),
    };
    let signer = SyntheticReceiptSigner {
        provider_id: "provider:alpha".parse()?,
        key_id: "key:alpha-receipt".parse()?,
        signing_key: SigningKey::from_bytes(&[81; 32]),
    };
    let consumed_at = execution_case.admission.admitted_at.clone();
    let journaled = execute_journaled_local_chat_completion_v1(
        &journal,
        LocalExecutionContextV1 {
            replay_store: &replay_store,
            admission: &execution_case.admission,
            authenticated: &authenticated,
            verification_keys: &environment.registry,
            request: &request,
            consumed_at: &consumed_at,
            cancellation: &cancellation,
        },
        &mut provider,
        &signer,
    )
    .await?;

    journal.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;

    let routing = &environment.fixture.bundle;
    let execution_input = serde_json::to_string(&serde_json::json!({
        "schema_version": "hyperintel.receipt-reconciliation-input.v1",
        "admission": execution_case.admission,
        "routing": {
            "evaluated_at": routing.evaluated_at,
            "service": routing.service,
            "providers": routing.providers,
            "offers": routing.offers,
            "policy": execution_case.policy,
            "assurance_sets": routing.assurance_sets,
            "assurance_blobs": environment.fixture.assurance_blobs,
            "reliability_artifacts": environment.fixture.reliability_artifacts,
        },
        "replay_transition": execution_case.replay_transition,
        "signed_policy": execution_case.signed_policy,
        "journal_start": journaled.start,
        "journal_terminal": journaled.terminal,
        "execution": journaled.execution,
    }))?;
    let trusted_keys_input = serde_json::to_string(&serde_json::json!({
        "trusted_verification_keys": environment.trusted_keys,
    }))?;

    Ok(HyperIntelReconciliationInputV1 {
        execution_input,
        trusted_keys_input,
    })
}

fn load_environment() -> Result<Environment, Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let fixture_path =
        root.join("products/hyperintel/crates/hyperintel-cli/tests/fixtures/evidence-v4.json");
    let trusted_path =
        root.join("products/hyperintel/crates/hyperintel-cli/tests/fixtures/trusted-keys-v4.json");
    let fixture: V4Fixture = serde_json::from_slice(&read_fixture(&fixture_path)?)?;
    let trusted: TrustedKeysFixture = serde_json::from_slice(&read_fixture(&trusted_path)?)?;
    let buyer_key = SigningKey::from_bytes(&[80; 32]);
    let alpha_key = SigningKey::from_bytes(&[81; 32]);
    let mut trusted_keys = trusted.trusted_verification_keys;
    let mut registry = VerificationKeyRegistry::default();
    for key in &trusted_keys {
        registry.register(key.clone())?;
    }
    let buyer_verification_key = registered_key(
        "buyer:demo",
        "key:buyer-policy-1",
        AssuranceRootClassV1::BuyerPolicy,
        &buyer_key,
    )?;
    registry.register(buyer_verification_key.clone())?;
    trusted_keys.push(buyer_verification_key);
    let alpha_verification_key = registered_key(
        "provider:alpha",
        "key:alpha-receipt",
        AssuranceRootClassV1::ProviderKey,
        &alpha_key,
    )?;
    registry.register(alpha_verification_key.clone())?;
    trusted_keys.push(alpha_verification_key);

    verify_replay_protected_market_evidence_bundle_v4(
        &fixture.bundle,
        &fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &registry,
    )?;

    Ok(Environment {
        fixture,
        registry,
        buyer_key,
        trusted_keys,
    })
}

fn read_fixture(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(fs::read(path)?)
}

fn execution_case(
    environment: &mut Environment,
    request: &ChatCompletionRequestV1,
) -> Result<ExecutionCase, Box<dyn Error>> {
    let mut policy = environment.fixture.bundle.policy.clone();
    policy.request_id = "request:hyperai-reconciliation-synthetic".parse()?;
    policy.allowed_providers.clear();
    policy.allowed_providers.insert("provider:alpha".parse()?);
    policy.prompt_digest = chat_completion_request_digest_v1(request)?;
    let evaluation =
        evaluate_reliability_artifact_verified_route_v1(&routing_context(environment, &policy))?;
    let replay_transition = derive_replay_state_transition_v1(
        &environment.fixture.bundle.replay_transition.before_state,
        &policy,
        evaluation.evaluation_digest,
        &environment.fixture.bundle.evaluated_at,
    )?;
    let signed_policy = signed_buyer_policy(&policy, &environment.buyer_key)?;
    let authenticated = AuthenticatedExecutionContextV1 {
        routing: routing_context(environment, &policy),
        replay_transition: &replay_transition,
        signed_policy: &signed_policy,
    };
    let admission = build_authenticated_execution_admission_v1(
        &authenticated,
        &environment.registry,
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

fn routing_context<'a>(
    environment: &'a Environment,
    policy: &'a InferenceRequestPolicyV1,
) -> ReliabilityArtifactVerifiedRoutingContextV1<'a> {
    let bundle = &environment.fixture.bundle;
    ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &bundle.evaluated_at,
        service: &bundle.service,
        providers: &bundle.providers,
        offers: &bundle.offers,
        policy,
        assurance_sets: &bundle.assurance_sets,
        assurance_blobs: &environment.fixture.assurance_blobs,
        reliability_artifacts: &environment.fixture.reliability_artifacts,
        verification_keys: &environment.registry,
    }
}

fn authenticated_context<'a>(
    environment: &'a Environment,
    execution_case: &'a ExecutionCase,
) -> AuthenticatedExecutionContextV1<'a> {
    AuthenticatedExecutionContextV1 {
        routing: routing_context(environment, &execution_case.policy),
        replay_transition: &execution_case.replay_transition,
        signed_policy: &execution_case.signed_policy,
    }
}

fn request(fixture: &V4Fixture) -> ChatCompletionRequestV1 {
    ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::Developer,
                content: "Reconcile a synthetic local execution.".to_owned(),
            },
            ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::User,
                content: "Use the fixed local-only reconciliation fixture.".to_owned(),
            },
        ],
        stream: true,
    }
}

async fn initialized_store(fixture: &V4Fixture) -> Result<SqliteReplayStoreV1, Box<dyn Error>> {
    let store = SqliteReplayStoreV1::in_memory().await?;
    store.migrate().await?;
    let outcome = store
        .initialize_buyer_state(
            &fixture.bundle.replay_transition.before_state,
            &fixture.bundle.evaluated_at,
        )
        .await?;
    assert_eq!(outcome, InitializeReplayStateOutcomeV1::Inserted);
    Ok(store)
}

fn execution_plan(fixture: &V4Fixture) -> Result<ProviderExecutionPlanV1, Box<dyn Error>> {
    Ok(ProviderExecutionPlanV1 {
        provider_id: "provider:alpha".parse()?,
        started_at: timestamp("2026-07-20T12:00:01Z")?,
        frames: vec![ProviderStreamFrameV1 {
            emitted_at: timestamp("2026-07-20T12:00:02Z")?,
            content: STREAM_CANARY.to_owned(),
            output_token_delta: 1,
        }],
        completed_at: timestamp("2026-07-20T12:00:04Z")?,
        input_tokens: 10,
        runtime_image_digest: fixture.bundle.service.runtime_image_digest,
    })
}

fn signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    signing_key: &SigningKey,
) -> Result<SignedInferenceRequestPolicyV1, Box<dyn Error>> {
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

fn registered_key(
    signer_id: &str,
    key_id: &str,
    trust_root_class: AssuranceRootClassV1,
    signing_key: &SigningKey,
) -> Result<RegisteredVerificationKeyV1, Box<dyn Error>> {
    Ok(RegisteredVerificationKeyV1 {
        signer_id: signer_id.parse()?,
        key_id: key_id.parse()?,
        trust_root_class,
        public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn Error>> {
    Ok(value.parse()?)
}
