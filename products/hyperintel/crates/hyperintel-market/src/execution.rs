use hyperintel_assurance::{
    AssuranceRootClassV1, Ed25519MessageVerificationInputV1, SignatureError,
    VerificationKeyRegistry, verify_ed25519_message_v1,
};
use hyperintel_core::{
    CanonicalizationError, CapacityOfferV1, InferenceRequestPolicyV1, ProtocolId,
    ProviderIdentityV1, Sha256Digest, UtcTimestamp, canonical_json, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ExecutionStatusV1, MeteringError, MeteringReceiptDraftV1, MeteringReceiptV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayError, ReplayStateTransitionV1,
    RouteDecisionV1, RouteEvaluationOutcomeV1, RoutingError, build_metering_receipt_v1,
    evaluate_reliability_artifact_verified_route_v1, verify_replay_state_transition_v1,
};

/// State slice emitted by authenticated execution records.
pub const AUTHENTICATED_EXECUTION_STATE_SLICE_V1: &str =
    "hyperintel-authenticated-execution-receipt-v1";

/// Buyer signature metadata over one exact inference request policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuyerPolicyStatementV1 {
    /// Fixed statement schema.
    pub schema_version: String,
    /// Canonical request-policy identity.
    pub request_policy_digest: Sha256Digest,
    /// Replay snapshot committed by the policy.
    pub replay_snapshot_digest: Sha256Digest,
    /// Signature validity start, equal to policy validity start.
    pub issued_at: UtcTimestamp,
    /// Signature validity end, equal to policy validity end.
    pub expires_at: UtcTimestamp,
    /// Exact buyer nonce committed by the policy.
    pub sequence: u64,
    /// Buyer identity that owns the policy.
    pub signer_id: ProtocolId,
    /// Caller-trusted buyer-policy key.
    pub key_id: ProtocolId,
    /// Must be `ed25519`.
    pub algorithm: String,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

#[derive(Serialize)]
struct BuyerPolicySigningMessage<'a> {
    schema_version: &'a str,
    request_policy_digest: Sha256Digest,
    replay_snapshot_digest: Sha256Digest,
    issued_at: &'a UtcTimestamp,
    expires_at: &'a UtcTimestamp,
    sequence: u64,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl BuyerPolicyStatementV1 {
    /// Return the canonical buyer-policy signing message without the signature.
    ///
    /// # Errors
    ///
    /// Returns when canonical JSON construction fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, CanonicalizationError> {
        canonical_json(&BuyerPolicySigningMessage {
            schema_version: &self.schema_version,
            request_policy_digest: self.request_policy_digest,
            replay_snapshot_digest: self.replay_snapshot_digest,
            issued_at: &self.issued_at,
            expires_at: &self.expires_at,
            sequence: self.sequence,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        })
    }
}

/// One inference request policy and the buyer signature authenticating it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedInferenceRequestPolicyV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact buyer policy.
    pub policy: InferenceRequestPolicyV1,
    /// Buyer signature over the policy identity and replay commitment.
    pub statement: BuyerPolicyStatementV1,
}

/// Inputs required to reconstruct one authenticated execution admission.
pub struct AuthenticatedExecutionContextV1<'a> {
    /// V4-grade signed-offer, assurance, reliability, and routing inputs.
    pub routing: ReliabilityArtifactVerifiedRoutingContextV1<'a>,
    /// Exact pure-data replay transition for the reconstructed route evaluation.
    pub replay_transition: &'a ReplayStateTransitionV1,
    /// Buyer-authenticated request policy, equal to the routing policy.
    pub signed_policy: &'a SignedInferenceRequestPolicyV1,
}

/// Non-authoritative admission of one authenticated buyer request to one selected route.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedExecutionAdmissionV1 {
    /// Fixed admission schema.
    pub schema_version: String,
    /// Named implementation state slice.
    pub state_slice: String,
    /// Buyer identity.
    pub buyer_id: ProtocolId,
    /// Buyer-policy verification key.
    pub buyer_policy_key_id: ProtocolId,
    /// Digest of the complete buyer policy statement, including its signature.
    pub buyer_policy_statement_digest: Sha256Digest,
    /// Exact buyer request-policy identity.
    pub request_policy_digest: Sha256Digest,
    /// Reconstructed full route-evaluation identity.
    pub route_evaluation_digest: Sha256Digest,
    /// Selected route identity.
    pub route_decision_digest: Sha256Digest,
    /// Pure-data replay-transition identity.
    pub replay_transition_digest: Sha256Digest,
    /// Selected service specification.
    pub service_spec_digest: Sha256Digest,
    /// Selected provider identity.
    pub provider_identity_digest: Sha256Digest,
    /// Selected provider identifier.
    pub provider_id: ProtocolId,
    /// Selected offer identity.
    pub offer_digest: Sha256Digest,
    /// Prompt commitment carried by the buyer policy.
    pub prompt_digest: Sha256Digest,
    /// Explicit admission time.
    pub admitted_at: UtcTimestamp,
    /// Latest completion time for this admission.
    pub deadline_at: UtcTimestamp,
    /// Pure-data admission records grant no request-release authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub admission_digest: Sha256Digest,
}

#[derive(Serialize)]
struct AdmissionIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    buyer_id: &'a ProtocolId,
    buyer_policy_key_id: &'a ProtocolId,
    buyer_policy_statement_digest: Sha256Digest,
    request_policy_digest: Sha256Digest,
    route_evaluation_digest: Sha256Digest,
    route_decision_digest: Sha256Digest,
    replay_transition_digest: Sha256Digest,
    service_spec_digest: Sha256Digest,
    provider_identity_digest: Sha256Digest,
    provider_id: &'a ProtocolId,
    offer_digest: Sha256Digest,
    prompt_digest: Sha256Digest,
    admitted_at: &'a UtcTimestamp,
    deadline_at: &'a UtcTimestamp,
    authority_granted: bool,
}

impl AuthenticatedExecutionAdmissionV1 {
    /// Recompute the admission identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        execution_admission_digest(self)
    }
}

/// Completed provider execution data before self-digest construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderExecutionReceiptDraftV1 {
    /// Provider execution start.
    pub started_at: UtcTimestamp,
    /// First streamed token observation.
    pub first_token_at: UtcTimestamp,
    /// Successful completion time.
    pub completed_at: UtcTimestamp,
    /// Actual input-token count.
    pub input_tokens: u64,
    /// Actual output-token count.
    pub output_tokens: u64,
    /// Commitment to the complete output stream.
    pub output_digest: Sha256Digest,
    /// Runtime image or service-runtime commitment observed for the execution.
    pub runtime_image_digest: Sha256Digest,
}

/// Self-digested provider execution and usage receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderExecutionReceiptV1 {
    /// Fixed receipt schema.
    pub schema_version: String,
    /// Named implementation state slice.
    pub state_slice: String,
    /// Exact authenticated admission.
    pub admission_digest: Sha256Digest,
    /// Buyer policy.
    pub request_policy_digest: Sha256Digest,
    /// Full route evaluation.
    pub route_evaluation_digest: Sha256Digest,
    /// Selected route.
    pub route_decision_digest: Sha256Digest,
    /// Pure-data replay transition.
    pub replay_transition_digest: Sha256Digest,
    /// Service specification.
    pub service_spec_digest: Sha256Digest,
    /// Provider identity.
    pub provider_identity_digest: Sha256Digest,
    /// Provider identifier.
    pub provider_id: ProtocolId,
    /// Selected offer.
    pub offer_digest: Sha256Digest,
    /// Prompt commitment.
    pub prompt_digest: Sha256Digest,
    /// Output-stream commitment.
    pub output_digest: Sha256Digest,
    /// Runtime image commitment.
    pub runtime_image_digest: Sha256Digest,
    /// Provider execution start.
    pub started_at: UtcTimestamp,
    /// First streamed token observation.
    pub first_token_at: UtcTimestamp,
    /// Successful completion time.
    pub completed_at: UtcTimestamp,
    /// Actual input-token count.
    pub input_tokens: u64,
    /// Actual output-token count.
    pub output_tokens: u64,
    /// V1 supports only one completed attempt.
    pub execution_status: ExecutionStatusV1,
    /// Pure-data receipts grant no execution or settlement authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub receipt_digest: Sha256Digest,
}

#[derive(Serialize)]
struct ProviderExecutionReceiptIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    admission_digest: Sha256Digest,
    request_policy_digest: Sha256Digest,
    route_evaluation_digest: Sha256Digest,
    route_decision_digest: Sha256Digest,
    replay_transition_digest: Sha256Digest,
    service_spec_digest: Sha256Digest,
    provider_identity_digest: Sha256Digest,
    provider_id: &'a ProtocolId,
    offer_digest: Sha256Digest,
    prompt_digest: Sha256Digest,
    output_digest: Sha256Digest,
    runtime_image_digest: Sha256Digest,
    started_at: &'a UtcTimestamp,
    first_token_at: &'a UtcTimestamp,
    completed_at: &'a UtcTimestamp,
    input_tokens: u64,
    output_tokens: u64,
    execution_status: ExecutionStatusV1,
    authority_granted: bool,
}

impl ProviderExecutionReceiptV1 {
    /// Recompute the provider execution receipt identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        provider_execution_receipt_digest(self)
    }
}

/// Provider signature over one exact execution receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderExecutionStatementV1 {
    /// Fixed statement schema.
    pub schema_version: String,
    /// Exact execution receipt identity.
    pub receipt_digest: Sha256Digest,
    /// Exact authenticated admission identity.
    pub admission_digest: Sha256Digest,
    /// Signature issuance time, equal to receipt completion.
    pub issued_at: UtcTimestamp,
    /// Signature expiry, equal to admission deadline.
    pub expires_at: UtcTimestamp,
    /// Nonzero provider-controlled logical sequence.
    pub sequence: u64,
    /// Selected provider identity.
    pub signer_id: ProtocolId,
    /// Configured provider receipt key.
    pub key_id: ProtocolId,
    /// Must be `ed25519`.
    pub algorithm: String,
    /// Lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

#[derive(Serialize)]
struct ProviderExecutionSigningMessage<'a> {
    schema_version: &'a str,
    receipt_digest: Sha256Digest,
    admission_digest: Sha256Digest,
    issued_at: &'a UtcTimestamp,
    expires_at: &'a UtcTimestamp,
    sequence: u64,
    signer_id: &'a ProtocolId,
    key_id: &'a ProtocolId,
    algorithm: &'a str,
}

impl ProviderExecutionStatementV1 {
    /// Return the canonical provider-receipt signing message without the signature.
    ///
    /// # Errors
    ///
    /// Returns when canonical JSON construction fails.
    pub fn signing_message(&self) -> Result<Vec<u8>, CanonicalizationError> {
        canonical_json(&ProviderExecutionSigningMessage {
            schema_version: &self.schema_version,
            receipt_digest: self.receipt_digest,
            admission_digest: self.admission_digest,
            issued_at: &self.issued_at,
            expires_at: &self.expires_at,
            sequence: self.sequence,
            signer_id: &self.signer_id,
            key_id: &self.key_id,
            algorithm: &self.algorithm,
        })
    }
}

/// Provider receipt and signature over its exact identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProviderExecutionReceiptV1 {
    /// Fixed wrapper schema.
    pub schema_version: String,
    /// Exact completed-execution receipt.
    pub receipt: ProviderExecutionReceiptV1,
    /// Selected provider signature under its configured receipt key.
    pub statement: ProviderExecutionStatementV1,
}

/// Fail-closed authenticated execution validation errors.
#[derive(Debug, Error)]
pub enum ExecutionEvidenceError {
    /// A wrapper, statement, admission, or receipt schema is unsupported.
    #[error("unsupported authenticated execution schema")]
    UnsupportedSchema,
    /// Buyer policy, signer, nonce, replay snapshot, or validity binding differs.
    #[error("buyer policy signature binding mismatch")]
    BuyerPolicyBindingMismatch,
    /// A required buyer or provider key has the wrong caller-assigned class.
    #[error("authenticated execution trust-root class mismatch")]
    TrustRootMismatch,
    /// The reconstructed route did not select an eligible provider.
    #[error("execution admission requires a selected route")]
    NoSelectedRoute,
    /// Admission fields do not exactly match reconstructed inputs.
    #[error("authenticated execution admission binding mismatch")]
    AdmissionBindingMismatch,
    /// Admission or completion times exceed the selected authorization window.
    #[error("authenticated execution time window is invalid")]
    InvalidTimeWindow,
    /// A pure-data object falsely claims external authority.
    #[error("authenticated execution record cannot grant authority")]
    AuthorityGranted,
    /// Admission or receipt self-digest is zero or inconsistent.
    #[error("authenticated execution self-digest mismatch")]
    DigestMismatch,
    /// Receipt fields do not match admission, route, service, provider, or offer.
    #[error("provider execution receipt binding mismatch")]
    ReceiptBindingMismatch,
    /// Provider receipt signing statement differs from the receipt or configured key.
    #[error("provider execution signature binding mismatch")]
    StatementBindingMismatch,
    /// Provider execution usage exceeds the selected authorization.
    #[error("provider execution usage exceeds an authorized bound")]
    UsageExceeded,
    /// Output or runtime evidence commitment is absent or inconsistent.
    #[error("provider execution evidence commitment is invalid")]
    InvalidEvidence,
    /// Routing reconstruction failed.
    #[error("authenticated execution routing failed: {0}")]
    Routing(#[from] RoutingError),
    /// Replay reconstruction failed.
    #[error("authenticated execution replay failed: {0}")]
    Replay(#[from] ReplayError),
    /// Signature, key-validity, revocation, or encoding verification failed.
    #[error("authenticated execution signature failed: {0}")]
    Signature(#[from] SignatureError),
    /// Metering reconstruction failed.
    #[error("authenticated execution metering failed: {0}")]
    Metering(#[from] MeteringError),
    /// Canonical identity construction failed.
    #[error("authenticated execution canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Verify a buyer signature over one exact request policy.
///
/// This authenticates only the supplied policy under caller-owned trust. It does not establish
/// durable replay consumption, request release, provider execution, or external authority.
///
/// # Errors
///
/// Returns on schema, digest, signer, key-class, nonce, replay, validity, or signature mismatch.
pub fn verify_signed_inference_request_policy_v1(
    signed: &SignedInferenceRequestPolicyV1,
    registry: &VerificationKeyRegistry,
    evaluated_at: &UtcTimestamp,
) -> Result<(), ExecutionEvidenceError> {
    if signed.schema_version != "hyperintel.signed-inference-request-policy.v1"
        || signed.policy.schema_version != "hyperintel.request-policy.v1"
        || signed.statement.schema_version != "hyperintel.buyer-policy-statement.v1"
    {
        return Err(ExecutionEvidenceError::UnsupportedSchema);
    }
    let policy_digest = signed.policy.digest()?;
    let statement = &signed.statement;
    if statement.request_policy_digest != policy_digest
        || statement.replay_snapshot_digest != signed.policy.replay_snapshot_digest
        || statement.signer_id != signed.policy.buyer_id
        || statement.sequence != signed.policy.nonce
        || statement.issued_at != signed.policy.valid_from
        || statement.expires_at != signed.policy.valid_until
    {
        return Err(ExecutionEvidenceError::BuyerPolicyBindingMismatch);
    }
    if registry.trust_root_class(&statement.signer_id, &statement.key_id)
        != Some(AssuranceRootClassV1::BuyerPolicy)
    {
        return Err(ExecutionEvidenceError::TrustRootMismatch);
    }
    let message = statement.signing_message()?;
    verify_ed25519_message_v1(
        &Ed25519MessageVerificationInputV1 {
            signer_id: &statement.signer_id,
            key_id: &statement.key_id,
            issued_at: &statement.issued_at,
            expires_at: &statement.expires_at,
            algorithm: &statement.algorithm,
            signature_hex: &statement.signature_hex,
            message: &message,
        },
        registry,
        evaluated_at,
    )?;
    Ok(())
}

/// Build one pure-data authenticated execution admission from reconstructed route inputs.
///
/// # Errors
///
/// Returns on buyer authentication, routing, replay, selected-subject, time-window, or identity
/// failure. The returned record always has `authority_granted = false`.
pub fn build_authenticated_execution_admission_v1(
    context: &AuthenticatedExecutionContextV1<'_>,
    registry: &VerificationKeyRegistry,
    admitted_at: UtcTimestamp,
    deadline_at: UtcTimestamp,
) -> Result<AuthenticatedExecutionAdmissionV1, ExecutionEvidenceError> {
    verify_signed_inference_request_policy_v1(context.signed_policy, registry, &admitted_at)?;
    if context.signed_policy.policy != *context.routing.policy {
        return Err(ExecutionEvidenceError::BuyerPolicyBindingMismatch);
    }
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&context.routing)?;
    verify_replay_state_transition_v1(
        context.replay_transition,
        context.routing.policy,
        evaluation.evaluation_digest,
        context.routing.evaluated_at,
    )?;
    let RouteEvaluationOutcomeV1::Selected(route) = &evaluation.outcome else {
        return Err(ExecutionEvidenceError::NoSelectedRoute);
    };
    let provider = context
        .routing
        .providers
        .get(&route.selected_provider_id)
        .ok_or(ExecutionEvidenceError::AdmissionBindingMismatch)?;
    let offer = context
        .routing
        .offers
        .iter()
        .map(|signed_offer| &signed_offer.offer)
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or(ExecutionEvidenceError::AdmissionBindingMismatch)?;
    if admitted_at < *context.routing.evaluated_at
        || admitted_at >= deadline_at
        || admitted_at < context.routing.policy.valid_from
        || deadline_at > context.routing.policy.valid_until
        || admitted_at < provider.valid_from
        || deadline_at > provider.valid_until
        || admitted_at < offer.valid_from
        || deadline_at > offer.valid_until
    {
        return Err(ExecutionEvidenceError::InvalidTimeWindow);
    }
    let mut admission = AuthenticatedExecutionAdmissionV1 {
        schema_version: "hyperintel.authenticated-execution-admission.v1".to_owned(),
        state_slice: AUTHENTICATED_EXECUTION_STATE_SLICE_V1.to_owned(),
        buyer_id: context.routing.policy.buyer_id.clone(),
        buyer_policy_key_id: context.signed_policy.statement.key_id.clone(),
        buyer_policy_statement_digest: buyer_policy_statement_digest(
            &context.signed_policy.statement,
        )?,
        request_policy_digest: context.routing.policy.digest()?,
        route_evaluation_digest: evaluation.evaluation_digest,
        route_decision_digest: route.route_digest,
        replay_transition_digest: context.replay_transition.transition_digest,
        service_spec_digest: context.routing.service.digest()?,
        provider_identity_digest: provider.digest()?,
        provider_id: provider.provider_id.clone(),
        offer_digest: offer.digest()?,
        prompt_digest: context.routing.policy.prompt_digest,
        admitted_at,
        deadline_at,
        authority_granted: false,
        admission_digest: Sha256Digest::from_bytes([0; 32]),
    };
    admission.admission_digest = execution_admission_digest(&admission)?;
    if admission.admission_digest.is_zero() {
        return Err(ExecutionEvidenceError::DigestMismatch);
    }
    Ok(admission)
}

/// Reconstruct and compare one authenticated execution admission.
///
/// # Errors
///
/// Returns on false authority, invalid identity, or any authentication/routing/replay mismatch.
pub fn verify_authenticated_execution_admission_v1(
    admission: &AuthenticatedExecutionAdmissionV1,
    context: &AuthenticatedExecutionContextV1<'_>,
    registry: &VerificationKeyRegistry,
) -> Result<(), ExecutionEvidenceError> {
    if admission.schema_version != "hyperintel.authenticated-execution-admission.v1"
        || admission.state_slice != AUTHENTICATED_EXECUTION_STATE_SLICE_V1
    {
        return Err(ExecutionEvidenceError::UnsupportedSchema);
    }
    if admission.authority_granted {
        return Err(ExecutionEvidenceError::AuthorityGranted);
    }
    if admission.admission_digest.is_zero()
        || execution_admission_digest(admission)? != admission.admission_digest
    {
        return Err(ExecutionEvidenceError::DigestMismatch);
    }
    let rebuilt = build_authenticated_execution_admission_v1(
        context,
        registry,
        admission.admitted_at.clone(),
        admission.deadline_at.clone(),
    )?;
    if rebuilt != *admission {
        return Err(ExecutionEvidenceError::AdmissionBindingMismatch);
    }
    Ok(())
}

/// Build one self-digested completed provider execution receipt.
///
/// The draft is data supplied by a future provider agent. It has no authenticity until wrapped
/// and accepted by [`verify_signed_provider_execution_receipt_v1`].
///
/// # Errors
///
/// Returns on admission identity, route binding, time, usage, evidence, or canonicalization
/// failure.
pub fn build_provider_execution_receipt_v1(
    admission: &AuthenticatedExecutionAdmissionV1,
    routing: &ReliabilityArtifactVerifiedRoutingContextV1<'_>,
    draft: ProviderExecutionReceiptDraftV1,
) -> Result<ProviderExecutionReceiptV1, ExecutionEvidenceError> {
    validate_admission_identity(admission)?;
    let evaluation = evaluate_reliability_artifact_verified_route_v1(routing)?;
    let RouteEvaluationOutcomeV1::Selected(route) = &evaluation.outcome else {
        return Err(ExecutionEvidenceError::NoSelectedRoute);
    };
    let provider = routing
        .providers
        .get(&route.selected_provider_id)
        .ok_or(ExecutionEvidenceError::ReceiptBindingMismatch)?;
    let offer = routing
        .offers
        .iter()
        .map(|signed_offer| &signed_offer.offer)
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or(ExecutionEvidenceError::ReceiptBindingMismatch)?;
    validate_admission_route_bindings(admission, routing, &evaluation, route, provider, offer)?;
    if draft.started_at < admission.admitted_at
        || draft.started_at > draft.first_token_at
        || draft.first_token_at > draft.completed_at
        || draft.completed_at > admission.deadline_at
    {
        return Err(ExecutionEvidenceError::InvalidTimeWindow);
    }
    if draft.input_tokens > routing.policy.max_input_tokens
        || draft.input_tokens > routing.service.max_input_tokens
        || draft.input_tokens > offer.max_input_tokens
        || draft.output_tokens > routing.policy.max_output_tokens
        || draft.output_tokens > routing.service.max_output_tokens
        || draft.output_tokens > offer.max_output_tokens
    {
        return Err(ExecutionEvidenceError::UsageExceeded);
    }
    if draft.output_digest.is_zero()
        || draft.runtime_image_digest.is_zero()
        || draft.runtime_image_digest != routing.service.runtime_image_digest
    {
        return Err(ExecutionEvidenceError::InvalidEvidence);
    }
    let mut receipt = ProviderExecutionReceiptV1 {
        schema_version: "hyperintel.provider-execution-receipt.v1".to_owned(),
        state_slice: AUTHENTICATED_EXECUTION_STATE_SLICE_V1.to_owned(),
        admission_digest: admission.admission_digest,
        request_policy_digest: admission.request_policy_digest,
        route_evaluation_digest: admission.route_evaluation_digest,
        route_decision_digest: admission.route_decision_digest,
        replay_transition_digest: admission.replay_transition_digest,
        service_spec_digest: admission.service_spec_digest,
        provider_identity_digest: admission.provider_identity_digest,
        provider_id: admission.provider_id.clone(),
        offer_digest: admission.offer_digest,
        prompt_digest: admission.prompt_digest,
        output_digest: draft.output_digest,
        runtime_image_digest: draft.runtime_image_digest,
        started_at: draft.started_at,
        first_token_at: draft.first_token_at,
        completed_at: draft.completed_at,
        input_tokens: draft.input_tokens,
        output_tokens: draft.output_tokens,
        execution_status: ExecutionStatusV1::Completed,
        authority_granted: false,
        receipt_digest: Sha256Digest::from_bytes([0; 32]),
    };
    receipt.receipt_digest = provider_execution_receipt_digest(&receipt)?;
    if receipt.receipt_digest.is_zero() {
        return Err(ExecutionEvidenceError::DigestMismatch);
    }
    Ok(receipt)
}

/// Verify one provider-signed completed execution receipt against reconstructed admission.
///
/// # Errors
///
/// Returns on admission, receipt, provider/key, trust, validity, or signature mismatch.
pub fn verify_signed_provider_execution_receipt_v1(
    signed: &SignedProviderExecutionReceiptV1,
    admission: &AuthenticatedExecutionAdmissionV1,
    context: &AuthenticatedExecutionContextV1<'_>,
    registry: &VerificationKeyRegistry,
) -> Result<(), ExecutionEvidenceError> {
    verify_authenticated_execution_admission_v1(admission, context, registry)?;
    if signed.schema_version != "hyperintel.signed-provider-execution-receipt.v1"
        || signed.receipt.schema_version != "hyperintel.provider-execution-receipt.v1"
        || signed.statement.schema_version != "hyperintel.provider-execution-statement.v1"
    {
        return Err(ExecutionEvidenceError::UnsupportedSchema);
    }
    if signed.receipt.authority_granted {
        return Err(ExecutionEvidenceError::AuthorityGranted);
    }
    let draft = ProviderExecutionReceiptDraftV1 {
        started_at: signed.receipt.started_at.clone(),
        first_token_at: signed.receipt.first_token_at.clone(),
        completed_at: signed.receipt.completed_at.clone(),
        input_tokens: signed.receipt.input_tokens,
        output_tokens: signed.receipt.output_tokens,
        output_digest: signed.receipt.output_digest,
        runtime_image_digest: signed.receipt.runtime_image_digest,
    };
    let rebuilt = build_provider_execution_receipt_v1(admission, &context.routing, draft)?;
    if rebuilt != signed.receipt {
        return Err(ExecutionEvidenceError::ReceiptBindingMismatch);
    }
    let provider = context
        .routing
        .providers
        .get(&admission.provider_id)
        .ok_or(ExecutionEvidenceError::ReceiptBindingMismatch)?;
    let statement = &signed.statement;
    if statement.receipt_digest != signed.receipt.receipt_digest
        || statement.admission_digest != admission.admission_digest
        || statement.signer_id != provider.provider_id
        || statement.key_id != provider.receipt_key_id
        || statement.issued_at != signed.receipt.completed_at
        || statement.expires_at != admission.deadline_at
        || statement.sequence == 0
    {
        return Err(ExecutionEvidenceError::StatementBindingMismatch);
    }
    if registry.trust_root_class(&statement.signer_id, &statement.key_id)
        != Some(AssuranceRootClassV1::ProviderKey)
    {
        return Err(ExecutionEvidenceError::TrustRootMismatch);
    }
    let message = statement.signing_message()?;
    verify_ed25519_message_v1(
        &Ed25519MessageVerificationInputV1 {
            signer_id: &statement.signer_id,
            key_id: &statement.key_id,
            issued_at: &statement.issued_at,
            expires_at: &statement.expires_at,
            algorithm: &statement.algorithm,
            signature_hex: &statement.signature_hex,
            message: &message,
        },
        registry,
        &signed.receipt.completed_at,
    )?;
    Ok(())
}

/// Reconstruct the existing metering receipt from one verified provider execution receipt.
///
/// The provider receipt digest becomes the metering evidence commitment. This remains a local
/// pure-data derivation and grants no settlement or external execution authority.
///
/// # Errors
///
/// Returns on admission, receipt, signature, selected-subject, or metering mismatch.
pub fn build_metering_from_verified_execution_receipt_v1(
    signed: &SignedProviderExecutionReceiptV1,
    admission: &AuthenticatedExecutionAdmissionV1,
    context: &AuthenticatedExecutionContextV1<'_>,
    registry: &VerificationKeyRegistry,
    buyer_observation_digest: Option<Sha256Digest>,
) -> Result<MeteringReceiptV1, ExecutionEvidenceError> {
    verify_signed_provider_execution_receipt_v1(signed, admission, context, registry)?;
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&context.routing)?;
    let RouteEvaluationOutcomeV1::Selected(route) = &evaluation.outcome else {
        return Err(ExecutionEvidenceError::NoSelectedRoute);
    };
    let provider = context
        .routing
        .providers
        .get(&route.selected_provider_id)
        .ok_or(ExecutionEvidenceError::ReceiptBindingMismatch)?;
    let offer = context
        .routing
        .offers
        .iter()
        .map(|signed_offer| &signed_offer.offer)
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or(ExecutionEvidenceError::ReceiptBindingMismatch)?;
    Ok(build_metering_receipt_v1(
        context.routing.service,
        provider,
        offer,
        context.routing.policy,
        route,
        MeteringReceiptDraftV1 {
            started_at: signed.receipt.started_at.clone(),
            first_token_at: signed.receipt.first_token_at.clone(),
            completed_at: signed.receipt.completed_at.clone(),
            input_tokens: signed.receipt.input_tokens,
            output_tokens: signed.receipt.output_tokens,
            execution_status: signed.receipt.execution_status,
            prompt_digest: signed.receipt.prompt_digest,
            output_digest: signed.receipt.output_digest,
            provider_metering_evidence_digest: signed.receipt.receipt_digest,
            buyer_observation_digest,
        },
    )?)
}

fn validate_admission_identity(
    admission: &AuthenticatedExecutionAdmissionV1,
) -> Result<(), ExecutionEvidenceError> {
    if admission.schema_version != "hyperintel.authenticated-execution-admission.v1"
        || admission.state_slice != AUTHENTICATED_EXECUTION_STATE_SLICE_V1
    {
        return Err(ExecutionEvidenceError::UnsupportedSchema);
    }
    if admission.authority_granted {
        return Err(ExecutionEvidenceError::AuthorityGranted);
    }
    if admission.admission_digest.is_zero()
        || execution_admission_digest(admission)? != admission.admission_digest
    {
        return Err(ExecutionEvidenceError::DigestMismatch);
    }
    Ok(())
}

fn validate_admission_route_bindings(
    admission: &AuthenticatedExecutionAdmissionV1,
    routing: &ReliabilityArtifactVerifiedRoutingContextV1<'_>,
    evaluation: &crate::RouteEvaluationRecordV1,
    route: &RouteDecisionV1,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
) -> Result<(), ExecutionEvidenceError> {
    if admission.request_policy_digest != routing.policy.digest()?
        || admission.route_evaluation_digest != evaluation.evaluation_digest
        || admission.route_decision_digest != route.route_digest
        || admission.service_spec_digest != routing.service.digest()?
        || admission.provider_identity_digest != provider.digest()?
        || admission.provider_id != provider.provider_id
        || admission.offer_digest != offer.digest()?
        || admission.prompt_digest != routing.policy.prompt_digest
    {
        return Err(ExecutionEvidenceError::ReceiptBindingMismatch);
    }
    Ok(())
}

fn buyer_policy_statement_digest(
    statement: &BuyerPolicyStatementV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest("hyperintel.buyer-policy-statement-record.v1", statement)
}

fn execution_admission_digest(
    admission: &AuthenticatedExecutionAdmissionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.authenticated-execution-admission.v1",
        &AdmissionIdentity {
            schema_version: &admission.schema_version,
            state_slice: &admission.state_slice,
            buyer_id: &admission.buyer_id,
            buyer_policy_key_id: &admission.buyer_policy_key_id,
            buyer_policy_statement_digest: admission.buyer_policy_statement_digest,
            request_policy_digest: admission.request_policy_digest,
            route_evaluation_digest: admission.route_evaluation_digest,
            route_decision_digest: admission.route_decision_digest,
            replay_transition_digest: admission.replay_transition_digest,
            service_spec_digest: admission.service_spec_digest,
            provider_identity_digest: admission.provider_identity_digest,
            provider_id: &admission.provider_id,
            offer_digest: admission.offer_digest,
            prompt_digest: admission.prompt_digest,
            admitted_at: &admission.admitted_at,
            deadline_at: &admission.deadline_at,
            authority_granted: admission.authority_granted,
        },
    )
}

fn provider_execution_receipt_digest(
    receipt: &ProviderExecutionReceiptV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.provider-execution-receipt.v1",
        &ProviderExecutionReceiptIdentity {
            schema_version: &receipt.schema_version,
            state_slice: &receipt.state_slice,
            admission_digest: receipt.admission_digest,
            request_policy_digest: receipt.request_policy_digest,
            route_evaluation_digest: receipt.route_evaluation_digest,
            route_decision_digest: receipt.route_decision_digest,
            replay_transition_digest: receipt.replay_transition_digest,
            service_spec_digest: receipt.service_spec_digest,
            provider_identity_digest: receipt.provider_identity_digest,
            provider_id: &receipt.provider_id,
            offer_digest: receipt.offer_digest,
            prompt_digest: receipt.prompt_digest,
            output_digest: receipt.output_digest,
            runtime_image_digest: receipt.runtime_image_digest,
            started_at: &receipt.started_at,
            first_token_at: &receipt.first_token_at,
            completed_at: &receipt.completed_at,
            input_tokens: receipt.input_tokens,
            output_tokens: receipt.output_tokens,
            execution_status: receipt.execution_status,
            authority_granted: receipt.authority_granted,
        },
    )
}
