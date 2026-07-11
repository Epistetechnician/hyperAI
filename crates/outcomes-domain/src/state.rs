use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Normalized outcome of one validator execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionOutcome {
    /// The pinned validator deterministically accepted the candidate.
    Pass,
    /// The validator ran correctly and rejected the candidate.
    Fail,
    /// The validator ran but cannot safely decide the predicate.
    Inconclusive,
    /// Infrastructure, runner, timeout, OOM, or malformed-output failure.
    Error,
}

/// Truth value of the frozen existential claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimTruth {
    /// At least one eligible candidate satisfied the pinned verifier policy.
    True,
    /// The complete negative-resolution prerequisites were satisfied.
    False,
    /// Available evidence cannot safely decide the claim.
    Unresolved,
}

/// Finality state of a resolution record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResolutionFinality {
    /// No resolution proposal exists.
    None,
    /// A provisional resolution exists inside its dispute policy.
    Provisional,
    /// A blocking dispute is active.
    Disputed,
    /// The immutable final-resolution event has been emitted.
    Final,
}

/// Error returned for an illegal finality transition.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("illegal resolution finality transition from {from:?} to {to:?}")]
pub struct FinalityTransitionError {
    /// Current state.
    pub from: ResolutionFinality,
    /// Requested state.
    pub to: ResolutionFinality,
}

impl ResolutionFinality {
    /// Apply one explicit legal finality transition.
    ///
    /// # Errors
    ///
    /// Returns [`FinalityTransitionError`] when the requested transition is not
    /// present in the frozen transition table.
    pub fn transition(self, next: Self) -> Result<Self, FinalityTransitionError> {
        let legal = matches!(
            (self, next),
            (Self::None | Self::Disputed, Self::Provisional)
                | (Self::Provisional, Self::Disputed | Self::Final)
        );
        if legal {
            Ok(next)
        } else {
            Err(FinalityTransitionError {
                from: self,
                to: next,
            })
        }
    }
}

/// Lifecycle of one external settlement binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementLifecycle {
    /// No external market is bound.
    Unbound,
    /// A digest-bound external market binding exists.
    Bound,
    /// One or more readiness prerequisites are absent.
    NotReady,
    /// All frozen preflight prerequisites are currently satisfied.
    Ready,
    /// A deterministic unsigned intent was persisted.
    IntentBuilt,
    /// A backend accepted or may have accepted the intent.
    Submitted,
    /// External authoritative status agrees with the final resolution.
    Confirmed,
    /// External status could not be reconciled safely.
    ReconciliationFailed,
    /// An operator or policy pause blocks further effects.
    Paused,
}

/// Error returned for an illegal settlement transition.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("illegal settlement transition from {from:?} to {to:?}")]
pub struct SettlementTransitionError {
    /// Current state.
    pub from: SettlementLifecycle,
    /// Requested state.
    pub to: SettlementLifecycle,
}

impl SettlementLifecycle {
    /// Apply one explicit legal settlement transition.
    ///
    /// # Errors
    ///
    /// Returns [`SettlementTransitionError`] when the requested transition is
    /// not present in the frozen transition table.
    pub fn transition(self, next: Self) -> Result<Self, SettlementTransitionError> {
        use SettlementLifecycle::{
            Bound, Confirmed, IntentBuilt, NotReady, Paused, Ready, ReconciliationFailed,
            Submitted, Unbound,
        };
        let legal = matches!(
            (self, next),
            (Unbound, Bound)
                | (Bound, NotReady | Ready | Paused)
                | (NotReady, Ready | Paused)
                | (Ready, IntentBuilt | NotReady | Paused)
                | (IntentBuilt, Submitted | NotReady | Paused)
                | (Submitted, Confirmed | ReconciliationFailed | Paused)
                | (ReconciliationFailed, Submitted | Confirmed | Paused)
                | (Paused, NotReady)
        );
        if legal {
            Ok(next)
        } else {
            Err(SettlementTransitionError {
                from: self,
                to: next,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn contract_final_is_terminal() {
        for next in [
            ResolutionFinality::None,
            ResolutionFinality::Provisional,
            ResolutionFinality::Disputed,
            ResolutionFinality::Final,
        ] {
            assert!(ResolutionFinality::Final.transition(next).is_err());
        }
    }

    #[test]
    fn contract_confirmed_is_terminal() {
        for next in [
            SettlementLifecycle::Unbound,
            SettlementLifecycle::Bound,
            SettlementLifecycle::NotReady,
            SettlementLifecycle::Ready,
            SettlementLifecycle::IntentBuilt,
            SettlementLifecycle::Submitted,
            SettlementLifecycle::Confirmed,
            SettlementLifecycle::ReconciliationFailed,
            SettlementLifecycle::Paused,
        ] {
            assert!(SettlementLifecycle::Confirmed.transition(next).is_err());
        }
    }

    proptest! {
        #[test]
        fn finality_never_transitions_out_of_final(index in 0_u8..4) {
            let next = [
                ResolutionFinality::None,
                ResolutionFinality::Provisional,
                ResolutionFinality::Disputed,
                ResolutionFinality::Final,
            ][usize::from(index)];
            prop_assert!(ResolutionFinality::Final.transition(next).is_err());
        }
    }
}
