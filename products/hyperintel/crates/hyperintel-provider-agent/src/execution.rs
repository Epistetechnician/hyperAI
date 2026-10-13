use std::fmt;

use hyperintel_core::{InferenceCapability, ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution::{
    LocalExecutionContextV1, LocalExecutionOutcomeV1, LocalProviderAgentV1,
    ProviderReceiptSignerV1, execute_local_chat_completion_v1,
};
use hyperintel_market::{
    RouteEvaluationOutcomeV1, evaluate_reliability_artifact_verified_route_v1,
};

use crate::{
    ProviderAgentBindingV1, ProviderAgentError, ProviderAgentKeyRegistryV1,
    ProviderAgentRevocationCheckpointV1, ProviderTextStreamInterfaceV1,
    SignedProviderAgentReadinessV1, SignedProviderAgentRevocationSnapshotV1,
    SignedProviderAgentServiceManifestV1, control::verify_provider_agent_control_v1,
};

/// Borrowed exact control state and existing local execution harness inputs.
pub struct ProviderAgentExecutionContextV1<'a> {
    /// Existing authenticated, replay-protected local execution context.
    pub local_execution: LocalExecutionContextV1<'a>,
    /// Signed service manifest.
    pub manifest: &'a SignedProviderAgentServiceManifestV1,
    /// Signed short-lived readiness assertion.
    pub readiness: &'a SignedProviderAgentReadinessV1,
    /// Current signed revocation state.
    pub revocation: &'a SignedProviderAgentRevocationSnapshotV1,
    /// Caller-pinned rollback and fork checkpoint.
    pub checkpoint: &'a ProviderAgentRevocationCheckpointV1,
    /// Caller-owned purpose-separated provider-agent trust.
    pub key_registry: &'a ProviderAgentKeyRegistryV1,
    /// Exact deployment expectation supplied by the caller.
    pub expected_binding: &'a ProviderAgentBindingV1,
    /// Explicit current control-evaluation time.
    pub evaluated_at: &'a UtcTimestamp,
}

/// Non-authoritative terminal local outcome with complete provider-agent lineage.
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderAgentExecutionV1 {
    /// Exact provider selected by the authenticated admission.
    provider_id: ProtocolId,
    /// Exact asserted provider-agent instance.
    agent_instance_id: ProtocolId,
    /// Authenticated execution-admission identity.
    admission_digest: Sha256Digest,
    /// Reconstructed selected-route identity.
    route_decision_digest: Sha256Digest,
    /// Exact selected signed offer.
    selected_offer_digest: Sha256Digest,
    /// Complete signed service-manifest identity.
    manifest_digest: Sha256Digest,
    /// Complete signed readiness-assertion identity.
    readiness_digest: Sha256Digest,
    /// Complete signed revocation-state identity.
    revocation_snapshot_digest: Sha256Digest,
    /// Trusted manifest-key record identity.
    manifest_key_digest: Sha256Digest,
    /// Trusted readiness-key record identity.
    readiness_key_digest: Sha256Digest,
    /// Trusted revocation-key record identity.
    revocation_key_digest: Sha256Digest,
    /// Exact caller-pinned checkpoint identity.
    checkpoint_digest: Sha256Digest,
    /// Explicit control-evaluation time.
    evaluated_at: UtcTimestamp,
    /// Existing harness terminal outcome, whether completed or incomplete.
    outcome: LocalExecutionOutcomeV1,
}

impl ProviderAgentExecutionV1 {
    /// Exact provider selected by the authenticated admission.
    #[must_use]
    pub fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }
    /// Exact asserted provider-agent instance.
    #[must_use]
    pub fn agent_instance_id(&self) -> &ProtocolId {
        &self.agent_instance_id
    }
    /// Authenticated admission identity.
    #[must_use]
    pub fn admission_digest(&self) -> Sha256Digest {
        self.admission_digest
    }
    /// Reconstructed selected-route identity.
    #[must_use]
    pub fn route_decision_digest(&self) -> Sha256Digest {
        self.route_decision_digest
    }
    /// Selected signed-offer identity.
    #[must_use]
    pub fn selected_offer_digest(&self) -> Sha256Digest {
        self.selected_offer_digest
    }
    /// Complete signed manifest identity.
    #[must_use]
    pub fn manifest_digest(&self) -> Sha256Digest {
        self.manifest_digest
    }
    /// Complete signed readiness identity.
    #[must_use]
    pub fn readiness_digest(&self) -> Sha256Digest {
        self.readiness_digest
    }
    /// Complete signed revocation-state identity.
    #[must_use]
    pub fn revocation_snapshot_digest(&self) -> Sha256Digest {
        self.revocation_snapshot_digest
    }
    /// Trusted manifest-key record identity.
    #[must_use]
    pub fn manifest_key_digest(&self) -> Sha256Digest {
        self.manifest_key_digest
    }
    /// Trusted readiness-key record identity.
    #[must_use]
    pub fn readiness_key_digest(&self) -> Sha256Digest {
        self.readiness_key_digest
    }
    /// Trusted revocation-key record identity.
    #[must_use]
    pub fn revocation_key_digest(&self) -> Sha256Digest {
        self.revocation_key_digest
    }
    /// Caller-pinned checkpoint identity.
    #[must_use]
    pub fn checkpoint_digest(&self) -> Sha256Digest {
        self.checkpoint_digest
    }
    /// Explicit control-evaluation time.
    #[must_use]
    pub fn evaluated_at(&self) -> &UtcTimestamp {
        &self.evaluated_at
    }
    /// Existing harness terminal outcome.
    #[must_use]
    pub fn outcome(&self) -> &LocalExecutionOutcomeV1 {
        &self.outcome
    }
    /// Provider-agent evidence never grants execution or routing authority.
    #[must_use]
    pub const fn authority_granted(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProviderAgentExecutionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAgentExecutionV1")
            .field("provider_id", &self.provider_id)
            .field("agent_instance_id", &self.agent_instance_id)
            .field("lineage", &"[REDACTED]")
            .field("outcome", &"[REDACTED]")
            .field("authority_granted", &false)
            .finish_non_exhaustive()
    }
}

/// Atomically verify provider-agent control state, then enter the existing harness once.
///
/// Invalid control or execution binding fails before replay consumption and provider
/// invocation. Once delegated, the existing harness owns its documented replay and
/// terminal-outcome semantics. This wrapper performs no retry, fallback, or route selection.
///
/// # Errors
///
/// Returns on invalid signed control state, exact binding mismatch, route reconstruction,
/// or an error returned by the existing local harness.
pub async fn execute_provider_agent_v1<P, S>(
    context: ProviderAgentExecutionContextV1<'_>,
    provider: &mut P,
    signer: &S,
) -> Result<ProviderAgentExecutionV1, ProviderAgentError>
where
    P: LocalProviderAgentV1,
    S: ProviderReceiptSignerV1,
{
    if context.evaluated_at != context.local_execution.consumed_at {
        return Err(ProviderAgentError::ControlDenied);
    }
    let verified = verify_provider_agent_control_v1(
        context.manifest,
        context.readiness,
        context.revocation,
        context.checkpoint,
        context.key_registry,
        context.expected_binding,
        context.evaluated_at,
    )?;
    let route_decision_digest =
        preflight_execution_binding(&context.local_execution, context.expected_binding)?;
    let checkpoint_digest = context.checkpoint.digest()?;
    let admission_digest = context.local_execution.admission.admission_digest;
    let selected_offer_digest = context.local_execution.admission.offer_digest;
    let provider_id = context.expected_binding.provider_id.clone();
    let agent_instance_id = context.expected_binding.agent_instance_id.clone();
    let evaluated_at = context.evaluated_at.clone();

    let outcome =
        execute_local_chat_completion_v1(context.local_execution, provider, signer).await?;

    Ok(ProviderAgentExecutionV1 {
        provider_id,
        agent_instance_id,
        admission_digest,
        route_decision_digest,
        selected_offer_digest,
        manifest_digest: verified.manifest,
        readiness_digest: verified.readiness,
        revocation_snapshot_digest: verified.revocation,
        manifest_key_digest: verified.manifest_key,
        readiness_key_digest: verified.readiness_key,
        revocation_key_digest: verified.revocation_key,
        checkpoint_digest,
        evaluated_at,
        outcome,
    })
}

fn preflight_execution_binding(
    local: &LocalExecutionContextV1<'_>,
    expected: &ProviderAgentBindingV1,
) -> Result<Sha256Digest, ProviderAgentError> {
    let admission = local.admission;
    let routing = &local.authenticated.routing;
    let service = routing.service;
    let service_digest = service.digest()?;
    let provider_identity = routing
        .providers
        .get(&expected.provider_id)
        .ok_or(ProviderAgentError::ExecutionBindingMismatch)?;
    let provider_identity_digest = provider_identity.digest()?;
    let service_capabilities: Vec<InferenceCapability> =
        service.capabilities.iter().cloned().collect();

    if admission.digest()? != admission.admission_digest
        || admission.authority_granted
        || admission.provider_id != expected.provider_id
        || admission.offer_digest != expected.selected_offer_digest
        || admission.service_spec_digest != expected.service_spec_digest
        || admission.provider_identity_digest != provider_identity_digest
        || service_digest != expected.service_spec_digest
        || service.model_artifact_digest != expected.model_artifact_digest
        || service.tokenizer_digest != expected.tokenizer_digest
        || service.runtime_image_digest != expected.runtime_image_digest
        || service_capabilities != expected.capabilities
        || provider_identity.receipt_key_id != expected.receipt_key_id
        || local.request.model != service.model_version.as_str()
        || !local.request.stream
        || expected.interface != ProviderTextStreamInterfaceV1::OpenAiTextStreamingChatCompletionsV1
    {
        return Err(ProviderAgentError::ExecutionBindingMismatch);
    }

    let evaluation = evaluate_reliability_artifact_verified_route_v1(routing)?;
    if evaluation.authority_granted
        || evaluation.evaluation_digest != admission.route_evaluation_digest
    {
        return Err(ProviderAgentError::ExecutionBindingMismatch);
    }
    let RouteEvaluationOutcomeV1::Selected(route) = evaluation.outcome else {
        return Err(ProviderAgentError::ExecutionBindingMismatch);
    };
    if route.authority_granted
        || route.digest()? != route.route_digest
        || route.route_digest != admission.route_decision_digest
        || route.selected_provider_id != expected.provider_id
        || route.selected_offer_digest != expected.selected_offer_digest
        || route.selected_provider_identity_digest != provider_identity_digest
        || route.service_spec_digest != expected.service_spec_digest
    {
        return Err(ProviderAgentError::ExecutionBindingMismatch);
    }
    Ok(route.route_digest)
}
