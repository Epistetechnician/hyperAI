use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Named mutable state boundaries used by commands, repositories, and audits.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StateSlice {
    /// Frozen challenges, verifiers, claims, and bindings.
    ClaimRegistryV1,
    /// Corpus descriptors, snapshots, packages, and solver views.
    CorpusRegistryV1,
    /// Source snapshots, cursors, candidates, and completeness.
    ObservationLedgerV1,
    /// Candidate and execution admission decisions.
    HsaiAdmissionV1,
    /// Verification attempts, manifests, and replay agreement.
    VerificationLedgerV1,
    /// Content-addressed evidence and public/private projections.
    EvidenceStoreV1,
    /// Signer registry, attestations, and quorum records.
    AttestationLedgerV1,
    /// Truth aggregation and resolution records.
    ResolutionLedgerV1,
    /// Disputes, pauses, and finality.
    DisputeFinalityV1,
    /// External market metadata and exact binding.
    MarketBindingV1,
    /// Settlement preflight, intents, attempts, receipts, and reconciliation.
    SettlementLedgerV1,
    /// Jobs, leases, outbox, and idempotency.
    DeliveryV1,
    /// Roles and authorization decisions.
    AccessControlV1,
    /// Adapter capabilities, rights, provenance, and readiness.
    AdapterRegistryV1,
    /// Audit, metrics, dependency health, and operator controls.
    OperationsV1,
    /// Harness plans, runs, candidates, traces, and cost telemetry.
    AgentRunsV1,
}

/// A sorted, duplicate-free declaration of state slices a mutation may touch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MutationScope(BTreeSet<StateSlice>);

impl MutationScope {
    /// Create a declared scope. Empty scopes are permitted for read-only work.
    #[must_use]
    pub fn new(slices: impl IntoIterator<Item = StateSlice>) -> Self {
        Self(slices.into_iter().collect())
    }

    /// Return whether the slice was declared before the transaction began.
    #[must_use]
    pub fn contains(&self, slice: StateSlice) -> bool {
        self.0.contains(&slice)
    }

    /// Reject a repository mutation outside the declared scope.
    ///
    /// # Errors
    ///
    /// Returns [`UndeclaredMutation`] when `slice` was not declared before the
    /// transaction began.
    pub fn require(&self, slice: StateSlice) -> Result<(), UndeclaredMutation> {
        if self.contains(slice) {
            Ok(())
        } else {
            Err(UndeclaredMutation { slice })
        }
    }

    /// Iterate over slices in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = StateSlice> + '_ {
        self.0.iter().copied()
    }
}

/// Request context recorded with every material transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationContext {
    /// Stable actor identifier; never a raw credential.
    pub actor_id: String,
    /// Request or idempotency correlation identifier.
    pub request_id: String,
    /// Slices declared before opening the transaction.
    pub scope: MutationScope,
}

/// A repository was asked to mutate a state slice that was not declared.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("state slice {slice:?} was not declared in the mutation scope")]
pub struct UndeclaredMutation {
    /// State slice the repository refused to mutate.
    pub slice: StateSlice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_scope_is_sorted_and_unique() {
        let scope = MutationScope::new([
            StateSlice::OperationsV1,
            StateSlice::ClaimRegistryV1,
            StateSlice::OperationsV1,
        ]);
        assert_eq!(
            scope.iter().collect::<Vec<_>>(),
            vec![StateSlice::ClaimRegistryV1, StateSlice::OperationsV1]
        );
    }

    #[test]
    fn contract_undeclared_slice_fails_closed() {
        let scope = MutationScope::new([StateSlice::ClaimRegistryV1]);
        assert_eq!(
            scope.require(StateSlice::SettlementLedgerV1),
            Err(UndeclaredMutation {
                slice: StateSlice::SettlementLedgerV1
            })
        );
    }
}
