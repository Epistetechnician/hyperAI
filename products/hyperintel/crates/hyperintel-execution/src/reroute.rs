//! Fail-closed lineage from a zero-output unavailable attempt to a new admission.

use hyperintel_core::{CanonicalizationError, ProtocolId, Sha256Digest, domain_separated_digest};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1,
    verify_authenticated_execution_admission_v1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    IncompleteExecutionStatusV1, IncompleteLocalExecutionV1, LOCAL_EXECUTION_STATE_SLICE_V1,
    LocalExecutionContextV1, LocalExecutionError, LocalExecutionOutcomeV1, LocalProviderAgentV1,
    ProviderAgentErrorV1, ProviderReceiptSignerV1, execute_local_chat_completion_v1, stream_digest,
};

/// State slice emitted by authenticated local reroute lineage.
pub const AUTHENTICATED_REROUTE_STATE_SLICE_V1: &str = "hyperintel-authenticated-reroute-v1";

/// Verified link from one unavailable attempt to one next-provider admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedRerouteRecordV1 {
    /// Fixed reroute schema.
    pub schema_version: String,
    /// Named reroute state slice.
    pub state_slice: String,
    /// Exact prior incomplete attempt.
    pub prior_attempt_digest: Sha256Digest,
    /// Exact prior authenticated admission.
    pub prior_admission_digest: Sha256Digest,
    /// Prior unavailable provider.
    pub prior_provider_id: ProtocolId,
    /// Exact prior durable replay consumption.
    pub prior_consumption_digest: Sha256Digest,
    /// Exact caller-supplied next authenticated admission.
    pub next_admission_digest: Sha256Digest,
    /// Different provider selected by the next admission.
    pub next_provider_id: ProtocolId,
    /// Buyer preserved across the reroute.
    pub buyer_id: ProtocolId,
    /// Service preserved across the reroute.
    pub service_spec_digest: Sha256Digest,
    /// Exact request commitment preserved across the reroute.
    pub prompt_digest: Sha256Digest,
    /// Nonce consumed by the unavailable attempt.
    pub prior_consumed_nonce: u64,
    /// Next sequential nonce consumed by the reroute admission.
    pub next_consumed_nonce: u64,
    /// A local lineage record grants no routing or execution authority.
    pub authority_granted: bool,
    /// Digest of every preceding semantic field.
    pub reroute_digest: Sha256Digest,
}

impl AuthenticatedRerouteRecordV1 {
    /// Recompute the authenticated reroute identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        reroute_digest(self)
    }
}

/// Completed local reroute invocation and its terminal execution result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedRerouteExecutionV1 {
    /// Fixed execution wrapper schema.
    pub schema_version: String,
    /// Named reroute state slice.
    pub state_slice: String,
    /// Verified unavailable-to-next-admission lineage.
    pub reroute: AuthenticatedRerouteRecordV1,
    /// Terminal result from the existing local execution harness.
    pub execution: LocalExecutionOutcomeV1,
    /// The wrapper grants no external execution or settlement authority.
    pub authority_granted: bool,
    /// Digest of every preceding semantic field.
    pub execution_digest: Sha256Digest,
}

impl AuthenticatedRerouteExecutionV1 {
    /// Recompute the reroute-execution wrapper identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        reroute_execution_digest(self)
    }
}

/// Borrowed prior attempt and exact next execution dependencies.
pub struct AuthenticatedRerouteContextV1<'a> {
    /// Prior provider-unavailable, zero-output attempt.
    pub prior_attempt: &'a IncompleteLocalExecutionV1,
    /// Exact prior authenticated admission committed by the attempt.
    pub prior_admission: &'a AuthenticatedExecutionAdmissionV1,
    /// Inputs required to reconstruct the prior admission.
    pub prior_authenticated: &'a AuthenticatedExecutionContextV1<'a>,
    /// Caller-supplied next admission, request, replay store, and trust roots.
    pub next_execution: LocalExecutionContextV1<'a>,
}

/// Fail-closed authenticated reroute errors.
#[derive(Debug, Error)]
pub enum AuthenticatedRerouteError {
    /// The prior attempt schema, state slice, digest, stream, or consumption is invalid.
    #[error("prior local execution attempt is invalid")]
    InvalidPriorAttempt,
    /// Only an unavailable provider with zero released chunks can enter this reroute lane.
    #[error("prior local execution status is not eligible for authenticated reroute")]
    PriorAttemptNotReroutable,
    /// Buyer identity changed between attempts.
    #[error("reroute buyer binding mismatch")]
    BuyerBindingMismatch,
    /// Service identity changed between attempts.
    #[error("reroute service binding mismatch")]
    ServiceBindingMismatch,
    /// Request commitment changed between attempts.
    #[error("reroute prompt commitment mismatch")]
    PromptBindingMismatch,
    /// The next admission selected the same provider.
    #[error("reroute must select a different provider")]
    ProviderNotChanged,
    /// The next transition is not the exact sequential successor of the prior consumption.
    #[error("reroute durable replay lineage mismatch")]
    ReplayLineageMismatch,
    /// The next admission predates the prior terminal record.
    #[error("reroute admission time regressed")]
    TimeRegression,
    /// The next authenticated admission cannot be reconstructed.
    #[error("reroute next admission is invalid: {0}")]
    NextAdmission(#[from] hyperintel_market::ExecutionEvidenceError),
    /// The existing local execution harness failed.
    #[error("reroute execution failed: {0}")]
    Execution(#[from] LocalExecutionError),
    /// Canonical identity construction failed.
    #[error("reroute canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Verify the unavailable-attempt lineage and invoke the supplied next provider.
///
/// This function does not select a route or sign a buyer policy. The caller
/// supplies a fully authenticated next-nonce admission whose route already
/// selected a different provider. Only the existing local harness may consume
/// that admission and invoke that provider.
///
/// # Errors
///
/// Returns before next-provider invocation on any prior-attempt, buyer,
/// service, prompt, provider, replay, time, authentication, or execution error.
pub async fn execute_authenticated_reroute_v1<P, S>(
    context: AuthenticatedRerouteContextV1<'_>,
    provider: &mut P,
    signer: &S,
) -> Result<AuthenticatedRerouteExecutionV1, AuthenticatedRerouteError>
where
    P: LocalProviderAgentV1,
    S: ProviderReceiptSignerV1,
{
    let reroute = validate_reroute(&context)?;
    let execution = execute_local_chat_completion_v1(context.next_execution, provider, signer)
        .await
        .map_err(AuthenticatedRerouteError::Execution)?;
    let mut result = AuthenticatedRerouteExecutionV1 {
        schema_version: "hyperintel.authenticated-reroute-execution.v1".to_owned(),
        state_slice: AUTHENTICATED_REROUTE_STATE_SLICE_V1.to_owned(),
        reroute,
        execution,
        authority_granted: false,
        execution_digest: Sha256Digest::from_bytes([0; 32]),
    };
    result.execution_digest = reroute_execution_digest(&result)?;
    Ok(result)
}

fn validate_reroute(
    context: &AuthenticatedRerouteContextV1<'_>,
) -> Result<AuthenticatedRerouteRecordV1, AuthenticatedRerouteError> {
    let prior = context.prior_attempt;
    let next = &context.next_execution;
    verify_authenticated_execution_admission_v1(
        context.prior_admission,
        context.prior_authenticated,
        next.verification_keys,
    )?;
    validate_prior_attempt(prior, context.prior_admission)?;
    verify_authenticated_execution_admission_v1(
        next.admission,
        next.authenticated,
        next.verification_keys,
    )?;
    validate_semantic_lineage(prior, context.prior_admission, next)?;
    build_reroute_record(prior, next)
}

fn validate_prior_attempt(
    prior: &IncompleteLocalExecutionV1,
    admission: &AuthenticatedExecutionAdmissionV1,
) -> Result<(), AuthenticatedRerouteError> {
    let consumption = &prior.replay_consumption;
    if prior.schema_version != "hyperintel.incomplete-local-execution.v1"
        || prior.state_slice != LOCAL_EXECUTION_STATE_SLICE_V1
        || prior.authority_granted
        || prior.attempt_digest.is_zero()
        || prior.digest()? != prior.attempt_digest
        || stream_digest(&prior.chunks)? != prior.stream_digest
        || consumption.authority_granted
        || consumption.consumption_digest.is_zero()
        || consumption.digest()? != consumption.consumption_digest
        || prior.admission_digest != consumption.admission_digest
        || prior.admission_digest != admission.admission_digest
        || prior.provider_id != admission.provider_id
    {
        return Err(AuthenticatedRerouteError::InvalidPriorAttempt);
    }
    if prior.status != IncompleteExecutionStatusV1::ProviderFailed
        || prior.provider_error != Some(ProviderAgentErrorV1::Unavailable)
        || !prior.chunks.is_empty()
    {
        return Err(AuthenticatedRerouteError::PriorAttemptNotReroutable);
    }
    Ok(())
}

fn validate_semantic_lineage(
    prior: &IncompleteLocalExecutionV1,
    prior_admission: &AuthenticatedExecutionAdmissionV1,
    next: &LocalExecutionContextV1<'_>,
) -> Result<(), AuthenticatedRerouteError> {
    let prior_consumption = &prior.replay_consumption;
    let transition = next.authenticated.replay_transition;
    if prior_admission.buyer_id != prior_consumption.buyer_id
        || next.admission.buyer_id != prior_admission.buyer_id
        || transition.buyer_id != prior_consumption.buyer_id
    {
        return Err(AuthenticatedRerouteError::BuyerBindingMismatch);
    }
    if next.admission.service_spec_digest != prior_admission.service_spec_digest
        || next.admission.service_spec_digest != next.authenticated.routing.service.digest()?
        || next.admission.service_spec_digest
            != next
                .authenticated
                .signed_policy
                .policy
                .required_service_spec_digest
    {
        return Err(AuthenticatedRerouteError::ServiceBindingMismatch);
    }
    if next.admission.prompt_digest != prior_admission.prompt_digest
        || next.admission.prompt_digest != next.authenticated.signed_policy.policy.prompt_digest
    {
        return Err(AuthenticatedRerouteError::PromptBindingMismatch);
    }
    if next.admission.provider_id == prior.provider_id {
        return Err(AuthenticatedRerouteError::ProviderNotChanged);
    }
    let expected_nonce = prior_consumption
        .consumed_nonce
        .checked_add(1)
        .ok_or(AuthenticatedRerouteError::ReplayLineageMismatch)?;
    if prior_consumption.after_state != transition.before_state
        || prior_consumption.after_state.next_nonce != expected_nonce
        || transition.consumed_nonce != expected_nonce
        || next.authenticated.signed_policy.policy.nonce != expected_nonce
        || next
            .authenticated
            .signed_policy
            .policy
            .replay_snapshot_digest
            != prior_consumption.after_state.state_digest
    {
        return Err(AuthenticatedRerouteError::ReplayLineageMismatch);
    }
    if next.authenticated.routing.evaluated_at < &prior.finished_at
        || next.admission.admitted_at < prior.finished_at
    {
        return Err(AuthenticatedRerouteError::TimeRegression);
    }
    Ok(())
}

fn build_reroute_record(
    prior: &IncompleteLocalExecutionV1,
    next: &LocalExecutionContextV1<'_>,
) -> Result<AuthenticatedRerouteRecordV1, AuthenticatedRerouteError> {
    let mut record = AuthenticatedRerouteRecordV1 {
        schema_version: "hyperintel.authenticated-reroute.v1".to_owned(),
        state_slice: AUTHENTICATED_REROUTE_STATE_SLICE_V1.to_owned(),
        prior_attempt_digest: prior.attempt_digest,
        prior_admission_digest: prior.admission_digest,
        prior_provider_id: prior.provider_id.clone(),
        prior_consumption_digest: prior.replay_consumption.consumption_digest,
        next_admission_digest: next.admission.admission_digest,
        next_provider_id: next.admission.provider_id.clone(),
        buyer_id: next.admission.buyer_id.clone(),
        service_spec_digest: next.admission.service_spec_digest,
        prompt_digest: next.admission.prompt_digest,
        prior_consumed_nonce: prior.replay_consumption.consumed_nonce,
        next_consumed_nonce: next.authenticated.replay_transition.consumed_nonce,
        authority_granted: false,
        reroute_digest: Sha256Digest::from_bytes([0; 32]),
    };
    record.reroute_digest = reroute_digest(&record)?;
    Ok(record)
}

fn reroute_digest(
    record: &AuthenticatedRerouteRecordV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.authenticated-reroute.v1",
        &(
            &record.schema_version,
            &record.state_slice,
            record.prior_attempt_digest,
            record.prior_admission_digest,
            &record.prior_provider_id,
            record.prior_consumption_digest,
            record.next_admission_digest,
            &record.next_provider_id,
            &record.buyer_id,
            record.service_spec_digest,
            record.prompt_digest,
            record.prior_consumed_nonce,
            record.next_consumed_nonce,
            record.authority_granted,
        ),
    )
}

fn reroute_execution_digest(
    result: &AuthenticatedRerouteExecutionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.authenticated-reroute-execution.v1",
        &(
            &result.schema_version,
            &result.state_slice,
            &result.reroute,
            &result.execution,
            result.authority_granted,
        ),
    )
}
