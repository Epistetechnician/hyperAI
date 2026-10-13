use hyperintel_core::{
    CanonicalizationError, InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp,
    domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::MARKET_STATE_SLICE_V1;

/// Caller-supplied replay cursor for one buyer identity.
///
/// The object is pure data. Its self-digest and a matching request-policy commitment establish
/// internal consistency only; they do not establish persistence, currentness, atomic consumption,
/// rollback resistance, buyer authentication, or external enforcement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuyerReplayStateV1 {
    /// Fixed replay-state schema.
    pub schema_version: String,
    /// Named implementation state slice.
    pub state_slice: String,
    /// Buyer whose nonce sequence this cursor governs.
    pub buyer_id: ProtocolId,
    /// The only nonce accepted by the next transition.
    pub next_nonce: u64,
    /// Domain-separated digest of every preceding field.
    pub state_digest: Sha256Digest,
}

#[derive(Serialize)]
struct BuyerReplayStateIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    buyer_id: &'a ProtocolId,
    next_nonce: u64,
}

impl BuyerReplayStateV1 {
    /// Recompute the replay-state identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        buyer_replay_state_digest(self)
    }
}

/// Pure-data record of one exact buyer nonce advancing to its successor.
///
/// Full before and after states are retained so an offline verifier can reconstruct the transition.
/// `authority_granted` is always false: this record cannot persist or consume external state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStateTransitionV1 {
    /// Fixed replay-transition schema.
    pub schema_version: String,
    /// Named implementation state slice.
    pub state_slice: String,
    /// Buyer whose nonce was consumed by this pure transition.
    pub buyer_id: ProtocolId,
    /// Exact nonce accepted from the before state.
    pub consumed_nonce: u64,
    /// Canonical request-policy identity that committed the before state.
    pub request_policy_digest: Sha256Digest,
    /// Canonical route-evaluation identity produced for that exact request policy.
    ///
    /// Both selected and all-rejected evaluations can be committed here. Validation of the
    /// evaluation record is performed by its owning routing protocol before supplying this digest.
    pub route_evaluation_digest: Sha256Digest,
    /// Explicit route-evaluation time; no ambient clock is read.
    pub evaluated_at: UtcTimestamp,
    /// Full caller-supplied state before the transition.
    pub before_state: BuyerReplayStateV1,
    /// Full deterministic state after the transition.
    pub after_state: BuyerReplayStateV1,
    /// Pure-data replay transitions grant no execution or persistence authority.
    pub authority_granted: bool,
    /// Domain-separated digest of every preceding field.
    pub transition_digest: Sha256Digest,
}

#[derive(Serialize)]
struct ReplayStateTransitionIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    buyer_id: &'a ProtocolId,
    consumed_nonce: u64,
    request_policy_digest: Sha256Digest,
    route_evaluation_digest: Sha256Digest,
    evaluated_at: &'a UtcTimestamp,
    before_state: &'a BuyerReplayStateV1,
    after_state: &'a BuyerReplayStateV1,
    authority_granted: bool,
}

impl ReplayStateTransitionV1 {
    /// Recompute the replay-transition identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        replay_state_transition_digest(self)
    }
}

/// Closed replay-state validation and transition failures.
#[derive(Debug, Error)]
pub enum ReplayError {
    /// Replay state, request policy, or transition schema is unsupported.
    #[error("unsupported replay-state input schema or state slice")]
    UnsupportedSchema,
    /// Replay-state self-digest is zero or differs from its canonical identity.
    #[error("buyer replay-state digest mismatch")]
    StateDigestMismatch,
    /// Replay-state buyer differs from the request-policy buyer.
    #[error("buyer replay-state binding mismatch")]
    BuyerBindingMismatch,
    /// Request policy does not commit the exact before-state digest.
    #[error("request policy replay snapshot binding mismatch")]
    SnapshotBindingMismatch,
    /// Request nonce precedes the exact next nonce and has already been consumed by this cursor.
    #[error("request nonce has already been consumed")]
    NonceAlreadyConsumed,
    /// Request nonce skips one or more required sequential nonces.
    #[error("request nonce is ahead of the replay cursor")]
    NonceGap,
    /// The exact nonce cannot be advanced without overflowing the protocol representation.
    #[error("buyer replay nonce is exhausted")]
    NonceExhausted,
    /// The caller supplied no usable route-evaluation identity.
    #[error("route evaluation digest must be nonzero")]
    InvalidRouteEvaluationDigest,
    /// Route evaluation time is outside the request-policy validity window.
    #[error("route evaluation is outside the request-policy validity window")]
    EvaluationOutsidePolicyValidity,
    /// A pure replay transition falsely claims external authority.
    #[error("replay-state transition cannot grant authority")]
    AuthorityGranted,
    /// Replay-transition self-digest is zero or differs from its canonical identity.
    #[error("replay-state transition digest mismatch")]
    TransitionDigestMismatch,
    /// Supplied replay transition differs from deterministic reconstruction.
    #[error("replay-state transition reconstruction mismatch")]
    TransitionMismatch,
    /// Canonical identity construction failed.
    #[error("replay-state canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Construct one self-digested buyer replay cursor.
///
/// This accepts a caller-selected starting nonce. It does not establish that the cursor is current
/// or durably stored.
///
/// # Errors
///
/// Returns when canonical identity construction fails.
pub fn build_buyer_replay_state_v1(
    buyer_id: ProtocolId,
    next_nonce: u64,
) -> Result<BuyerReplayStateV1, ReplayError> {
    let mut state = BuyerReplayStateV1 {
        schema_version: "hyperintel.buyer-replay-state.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        buyer_id,
        next_nonce,
        state_digest: Sha256Digest::from_bytes([0; 32]),
    };
    state.state_digest = buyer_replay_state_digest(&state)?;
    if state.state_digest.is_zero() {
        return Err(ReplayError::StateDigestMismatch);
    }
    Ok(state)
}

/// Validate the supported schema, state slice, and canonical self-digest.
///
/// # Errors
///
/// Returns when the schema, state slice, digest, or canonical identity is invalid.
pub fn validate_buyer_replay_state_v1(state: &BuyerReplayStateV1) -> Result<(), ReplayError> {
    if state.schema_version != "hyperintel.buyer-replay-state.v1"
        || state.state_slice != MARKET_STATE_SLICE_V1
    {
        return Err(ReplayError::UnsupportedSchema);
    }
    if state.state_digest.is_zero() || buyer_replay_state_digest(state)? != state.state_digest {
        return Err(ReplayError::StateDigestMismatch);
    }
    Ok(())
}

/// Derive one exact nonce transition bound to a route-evaluation identity.
///
/// This function checks only pure input consistency. Returning a transition does not persist the
/// after state or prevent another caller from reusing the before state. The supplied evaluation
/// digest may identify either a selected route or a deterministic all-rejected evaluation; replay
/// state does not inspect or reinterpret that routing outcome.
///
/// # Errors
///
/// Returns on unsupported schema, invalid state, buyer/snapshot/nonce mismatch, exhaustion,
/// invalid evaluation identity or time, or canonicalization failure.
pub fn derive_replay_state_transition_v1(
    before_state: &BuyerReplayStateV1,
    policy: &InferenceRequestPolicyV1,
    route_evaluation_digest: Sha256Digest,
    evaluated_at: &UtcTimestamp,
) -> Result<ReplayStateTransitionV1, ReplayError> {
    validate_replay_precondition_v1(before_state, policy, route_evaluation_digest, evaluated_at)?;
    let policy_digest = policy.digest()?;

    let next_nonce = before_state
        .next_nonce
        .checked_add(1)
        .ok_or(ReplayError::NonceExhausted)?;
    let after_state = build_buyer_replay_state_v1(before_state.buyer_id.clone(), next_nonce)?;
    let mut transition = ReplayStateTransitionV1 {
        schema_version: "hyperintel.replay-state-transition.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        buyer_id: before_state.buyer_id.clone(),
        consumed_nonce: policy.nonce,
        request_policy_digest: policy_digest,
        route_evaluation_digest,
        evaluated_at: evaluated_at.clone(),
        before_state: before_state.clone(),
        after_state,
        authority_granted: false,
        transition_digest: Sha256Digest::from_bytes([0; 32]),
    };
    transition.transition_digest = replay_state_transition_digest(&transition)?;
    if transition.transition_digest.is_zero() {
        return Err(ReplayError::TransitionDigestMismatch);
    }
    Ok(transition)
}

/// Verify a supplied transition by deterministic reconstruction.
///
/// # Errors
///
/// Returns when transition identity, authority boundary, input validation, or reconstruction fails.
pub fn verify_replay_state_transition_v1(
    transition: &ReplayStateTransitionV1,
    policy: &InferenceRequestPolicyV1,
    route_evaluation_digest: Sha256Digest,
    evaluated_at: &UtcTimestamp,
) -> Result<(), ReplayError> {
    if transition.schema_version != "hyperintel.replay-state-transition.v1"
        || transition.state_slice != MARKET_STATE_SLICE_V1
    {
        return Err(ReplayError::UnsupportedSchema);
    }
    if transition.authority_granted {
        return Err(ReplayError::AuthorityGranted);
    }
    if transition.transition_digest.is_zero()
        || replay_state_transition_digest(transition)? != transition.transition_digest
    {
        return Err(ReplayError::TransitionDigestMismatch);
    }
    let rebuilt = derive_replay_state_transition_v1(
        &transition.before_state,
        policy,
        route_evaluation_digest,
        evaluated_at,
    )?;
    if rebuilt != *transition {
        return Err(ReplayError::TransitionMismatch);
    }
    Ok(())
}

fn validate_replay_precondition_v1(
    before_state: &BuyerReplayStateV1,
    policy: &InferenceRequestPolicyV1,
    route_evaluation_digest: Sha256Digest,
    evaluated_at: &UtcTimestamp,
) -> Result<(), ReplayError> {
    validate_buyer_replay_state_v1(before_state)?;
    if policy.schema_version != "hyperintel.request-policy.v1" {
        return Err(ReplayError::UnsupportedSchema);
    }
    if policy.buyer_id != before_state.buyer_id {
        return Err(ReplayError::BuyerBindingMismatch);
    }
    if policy.replay_snapshot_digest != before_state.state_digest {
        return Err(ReplayError::SnapshotBindingMismatch);
    }
    if policy.nonce < before_state.next_nonce {
        return Err(ReplayError::NonceAlreadyConsumed);
    }
    if policy.nonce > before_state.next_nonce {
        return Err(ReplayError::NonceGap);
    }
    if before_state.next_nonce == u64::MAX {
        return Err(ReplayError::NonceExhausted);
    }
    if route_evaluation_digest.is_zero() {
        return Err(ReplayError::InvalidRouteEvaluationDigest);
    }
    if policy.valid_from > *evaluated_at || policy.valid_until < *evaluated_at {
        return Err(ReplayError::EvaluationOutsidePolicyValidity);
    }
    Ok(())
}

fn buyer_replay_state_digest(
    state: &BuyerReplayStateV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.buyer-replay-state.v1",
        &BuyerReplayStateIdentity {
            schema_version: &state.schema_version,
            state_slice: &state.state_slice,
            buyer_id: &state.buyer_id,
            next_nonce: state.next_nonce,
        },
    )
}

fn replay_state_transition_digest(
    transition: &ReplayStateTransitionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.replay-state-transition.v1",
        &ReplayStateTransitionIdentity {
            schema_version: &transition.schema_version,
            state_slice: &transition.state_slice,
            buyer_id: &transition.buyer_id,
            consumed_nonce: transition.consumed_nonce,
            request_policy_digest: transition.request_policy_digest,
            route_evaluation_digest: transition.route_evaluation_digest,
            evaluated_at: &transition.evaluated_at,
            before_state: &transition.before_state,
            after_state: &transition.after_state,
            authority_granted: transition.authority_granted,
        },
    )
}
