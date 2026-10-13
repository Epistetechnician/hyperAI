//! Local, synthetic Fleet inventory and non-executable operation previews.
//!
//! This crate does not discover a host or provider. Every record is an
//! illustrative `HyperAI` fixture and remains separate from market and outcome
//! contracts.

use std::collections::HashMap;
use std::sync::Mutex;

use hyperai_foundation::{CanonicalizationError, Sha256Digest, domain_separated_digest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const INVENTORY_SCHEMA: &str = "hyperai.fleet.inventory.v1";
const DOCTOR_SCHEMA: &str = "hyperai.fleet.doctor.v1";
const PLAN_SCHEMA: &str = "hyperai.fleet.operation-preview.v1";
const SIMULATION_SCHEMA: &str = "hyperai.fleet.simulation.snapshot.v1";
const SIMULATION_TRANSITION_SCHEMA: &str = "hyperai.fleet.simulation.transition.v1";
const STATE_SLICE: &str = "hyperai.bazaar.fleet_readonly.v1";
const SIMULATION_STATE_SLICE: &str = "hyperai.bazaar.fleet_simulation.v1";
const FIXTURE_AUTHOR: &str = "HyperAI synthetic fixture";
const CLAIM_CEILING: &str =
    "Deterministic display of HyperAI-authored synthetic Fleet examples only.";
const NONCLAIMS: &str = "Does not establish actual hosts, owners, health, reachability, capacity, availability, authority, computation quality, outcomes, or payment entitlement.";
const SIMULATION_CLAIM_CEILING: &str = "Deterministic lifecycle behavior for one virtual Fleet scenario inside one local service process.";
const SIMULATION_NONCLAIMS: &str = "Does not establish real process management, resource ownership, remote operation safety, workload correctness, outcomes, or payment entitlement.";
const SIMULATION_CAPACITY_UNITS: u8 = 4;
const MAX_SIMULATION_IDEMPOTENCY_KEYS: usize = 128;

/// Errors returned while building or checking local Fleet contracts.
#[derive(Debug, Error)]
pub enum FleetError {
    /// A plan referred to a node outside the fixed synthetic inventory.
    #[error("the requested node is not present in the synthetic Fleet inventory")]
    UnknownNode,
    /// A versioned local fixture could not be represented canonically.
    #[error("the synthetic Fleet contract could not be encoded")]
    Canonicalization(#[from] CanonicalizationError),
    /// The requested state digest or idempotency key conflicts with prior state.
    #[error("the Fleet simulation state or idempotency key conflicts with prior state")]
    SimulationConflict,
    /// The request is outside the bounded simulation contract.
    #[error("the Fleet simulation request is invalid or exceeds its bounded contract")]
    InvalidSimulationRequest,
    /// The operation is not valid for the current simulated state.
    #[error("the Fleet simulation transition is not valid for the current state")]
    SimulationTransitionRejected,
    /// The service-owned simulation lock is unavailable.
    #[error("the Fleet simulation state is unavailable")]
    SimulationUnavailable,
    /// No new idempotency entries can be stored in this service process.
    #[error("the Fleet simulation idempotency cache is full")]
    SimulationCacheFull,
}

/// One illustrative status in the fixed Fleet scenario.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetHealthStateV1 {
    /// The scenario's latest illustrative sample is marked healthy.
    Healthy,
    /// The scenario marks the illustrative sample as stale.
    Stale,
    /// The scenario marks the illustrative endpoint unreachable.
    Unreachable,
    /// The scenario has no health observation.
    Unknown,
}

/// Provenance that distinguishes the local scenario from an imported fleet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetProvenanceV1 {
    /// Stable source kind.
    pub source_kind: &'static str,
    /// Author label for the synthetic fixture.
    pub fixture_author: &'static str,
    /// Scenario identifier, not a provider, machine, or owner identifier.
    pub scenario_id: &'static str,
    /// Whether a live connection or probe was used.
    pub observation_mode: &'static str,
}

/// One fictional node in the illustrative inventory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetNodeV1 {
    /// Stable synthetic fixture key; never a host or provider identifier.
    pub node_id: &'static str,
    /// Human-readable fictional display label.
    pub display_name: &'static str,
    /// Fixture owner label, not an account or legal owner.
    pub owner_label: &'static str,
    /// Fictional resource class without a capacity or availability claim.
    pub resource_profile: &'static str,
    /// Current illustrative scenario state, not a probe result.
    pub health_state: FleetHealthStateV1,
    /// Short scenario note for the current illustrative state.
    pub scenario_note: &'static str,
}

/// A fixed illustrative health event ordered by scenario step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetHealthEventV1 {
    /// One-based order inside this authored scenario; not a wall-clock time.
    pub scenario_step: u32,
    /// Synthetic node key from the same inventory response.
    pub node_id: &'static str,
    /// Illustrative health state for this sample.
    pub health_state: FleetHealthStateV1,
    /// Scenario-only explanation; not provider telemetry.
    pub summary: &'static str,
}

/// Status for one deterministic local contract check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetDoctorCheckV1 {
    /// Stable check identifier.
    pub check_id: &'static str,
    /// pass, unknown, or blocked.
    pub status: &'static str,
    /// What this fixture-only check establishes.
    pub summary: &'static str,
}

/// Results from fixture validation, explicitly separate from a host probe.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetDoctorV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns these checks.
    pub state_slice: &'static str,
    /// Mode used to produce these results.
    pub mode: &'static str,
    /// Local deterministic checks and intentionally unknown live checks.
    pub checks: Vec<FleetDoctorCheckV1>,
    /// Effect boundary for this result.
    pub effect_boundary: &'static str,
}

/// The read-only local Fleet snapshot, built from fixed synthetic records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetInventoryV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this result.
    pub state_slice: &'static str,
    /// Synthetic-only provenance.
    pub provenance: FleetProvenanceV1,
    /// Digest of the fixed scenario payload.
    pub snapshot_digest: Sha256Digest,
    /// Fictional nodes and illustrative current state.
    pub nodes: Vec<FleetNodeV1>,
    /// Fixed health events ordered by scenario step.
    pub health_timeline: Vec<FleetHealthEventV1>,
    /// Deterministic local checks over the fixture only.
    pub doctor: FleetDoctorV1,
    /// Maximum claim supported by this response.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
    /// Effect boundary for this result.
    pub effect_boundary: &'static str,
}

/// Candidate operation labels supported by a preview, never an executor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetOperationV1 {
    /// Illustrative attach intent.
    Attach,
    /// Illustrative launch intent.
    Launch,
    /// Illustrative workload-start intent.
    WorkloadStart,
    /// Illustrative cancel intent.
    Cancel,
    /// Illustrative terminate intent.
    Terminate,
}

/// Request to calculate a deterministic preview for a synthetic node.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetPlanRequestV1 {
    /// Synthetic node key from the local inventory.
    pub node_id: String,
    /// Candidate intent whose digest is shown to the caller.
    pub operation: FleetOperationV1,
    /// Prior preview digest, when checking whether the intent changed.
    #[serde(default)]
    pub expected_preview_digest: Option<Sha256Digest>,
}

/// Stable fields bound by the operation-preview digest.
#[derive(Serialize)]
struct FleetPlanDigestBodyV1<'a> {
    schema_version: &'static str,
    state_slice: &'static str,
    snapshot_digest: Sha256Digest,
    node_id: &'a str,
    operation: FleetOperationV1,
    health_state: FleetHealthStateV1,
}

/// A digest-bound operation plan with no executable action or reservation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetOperationPlanV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this preview.
    pub state_slice: &'static str,
    /// `preview_ready` or `preview_changed` relative to the supplied digest.
    pub preview_status: &'static str,
    /// Digest of the exact preview fields.
    pub preview_digest: Sha256Digest,
    /// Caller digest if a previous preview was supplied.
    pub expected_preview_digest: Option<Sha256Digest>,
    /// Synthetic target key.
    pub node_id: String,
    /// Candidate operation; no corresponding action is exposed.
    pub operation: FleetOperationV1,
    /// Fixture health state bound by the preview.
    pub observed_health_state: FleetHealthStateV1,
    /// Health note for the illustrative state.
    pub health_gate: &'static str,
    /// Preview authority state.
    pub authority: &'static str,
    /// Whether the plan can invoke or reserve any resource.
    pub effect_boundary: &'static str,
    /// Gates a future separately authorized implementation would need.
    pub required_gates: Vec<&'static str>,
    /// Maximum claim supported by this response.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
}

/// Return the deterministic read-only synthetic inventory and fixture doctor.
///
/// # Errors
///
/// Returns an error only if the fixed fixture cannot be canonically encoded.
pub fn inventory_v1() -> Result<FleetInventoryV1, FleetError> {
    let provenance = FleetProvenanceV1 {
        source_kind: "hyperai_authored_synthetic_scenario",
        fixture_author: FIXTURE_AUTHOR,
        scenario_id: "fleet-health-states-v1",
        observation_mode: "no_live_probe_or_import",
    };
    let nodes = vec![
        FleetNodeV1 {
            node_id: "fixture-node-atlas",
            display_name: "Atlas · illustrative GPU node",
            owner_label: FIXTURE_AUTHOR,
            resource_profile: "Illustrative accelerator class A",
            health_state: FleetHealthStateV1::Healthy,
            scenario_note: "Fixture sample marked healthy; no host was checked.",
        },
        FleetNodeV1 {
            node_id: "fixture-node-boreal",
            display_name: "Boreal · illustrative GPU node",
            owner_label: FIXTURE_AUTHOR,
            resource_profile: "Illustrative accelerator class B",
            health_state: FleetHealthStateV1::Stale,
            scenario_note: "Fixture sample is stale; freshness is not measured from a clock.",
        },
        FleetNodeV1 {
            node_id: "fixture-node-cinder",
            display_name: "Cinder · illustrative GPU node",
            owner_label: FIXTURE_AUTHOR,
            resource_profile: "Illustrative accelerator class C",
            health_state: FleetHealthStateV1::Unreachable,
            scenario_note: "Fixture records an unreachable example; no connection was attempted.",
        },
        FleetNodeV1 {
            node_id: "fixture-node-delta",
            display_name: "Delta · illustrative GPU node",
            owner_label: FIXTURE_AUTHOR,
            resource_profile: "Illustrative accelerator class D",
            health_state: FleetHealthStateV1::Unknown,
            scenario_note: "Fixture contains no health observation.",
        },
    ];
    let health_timeline = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| FleetHealthEventV1 {
            scenario_step: u32::try_from(index + 1).unwrap_or(u32::MAX),
            node_id: node.node_id,
            health_state: node.health_state,
            summary: node.scenario_note,
        })
        .collect::<Vec<_>>();
    let snapshot_digest = domain_separated_digest(
        "hyperai.fleet.synthetic-snapshot.v1",
        &FleetSnapshotDigestBodyV1 {
            schema_version: INVENTORY_SCHEMA,
            provenance: &provenance,
            nodes: &nodes,
            health_timeline: &health_timeline,
        },
    )?;
    let doctor = doctor_v1();
    Ok(FleetInventoryV1 {
        schema_version: INVENTORY_SCHEMA,
        state_slice: STATE_SLICE,
        provenance,
        snapshot_digest,
        nodes,
        health_timeline,
        doctor,
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
        effect_boundary: "local_synthetic_read_only_no_probe_no_external_effects",
    })
}

#[derive(Serialize)]
struct FleetSnapshotDigestBodyV1<'a> {
    schema_version: &'static str,
    provenance: &'a FleetProvenanceV1,
    nodes: &'a [FleetNodeV1],
    health_timeline: &'a [FleetHealthEventV1],
}

/// Return fixture-consistency checks and explicit unknown live checks.
#[must_use]
pub fn doctor_v1() -> FleetDoctorV1 {
    FleetDoctorV1 {
        schema_version: DOCTOR_SCHEMA,
        state_slice: STATE_SLICE,
        mode: "synthetic_fixture_only",
        checks: vec![
            FleetDoctorCheckV1 {
                check_id: "fixture_provenance",
                status: "pass",
                summary: "The sample is labeled as a HyperAI-authored synthetic scenario.",
            },
            FleetDoctorCheckV1 {
                check_id: "fixture_node_ids",
                status: "pass",
                summary: "Every displayed node key is a fixed fixture identifier.",
            },
            FleetDoctorCheckV1 {
                check_id: "fixture_health_vocabulary",
                status: "pass",
                summary: "Healthy, stale, unreachable, and unknown are preserved as scenario states.",
            },
            FleetDoctorCheckV1 {
                check_id: "live_health_probe",
                status: "unknown",
                summary: "No machine, provider, socket, or endpoint health probe was run.",
            },
            FleetDoctorCheckV1 {
                check_id: "operation_authority",
                status: "blocked",
                summary: "No live adapter, resource reservation, or operation authority is available.",
            },
        ],
        effect_boundary: "local_fixture_validation_only_no_host_or_provider_checks",
    }
}

/// Calculate a digest-bound preview without invoking or reserving an operation.
///
/// # Errors
///
/// Returns `FleetError::UnknownNode` for any target outside the fixed fixture.
/// Returns a canonicalization error if the preview cannot be encoded safely.
pub fn preview_operation_v1(
    request: &FleetPlanRequestV1,
) -> Result<FleetOperationPlanV1, FleetError> {
    let inventory = inventory_v1()?;
    let node = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == request.node_id)
        .ok_or(FleetError::UnknownNode)?;
    let preview_digest = domain_separated_digest(
        "hyperai.fleet.operation-preview.v1",
        &FleetPlanDigestBodyV1 {
            schema_version: PLAN_SCHEMA,
            state_slice: STATE_SLICE,
            snapshot_digest: inventory.snapshot_digest,
            node_id: &request.node_id,
            operation: request.operation,
            health_state: node.health_state,
        },
    )?;
    let preview_status = match request.expected_preview_digest {
        Some(expected) if expected != preview_digest => "preview_changed",
        Some(_) | None => "preview_ready",
    };
    let health_gate = if node.health_state == FleetHealthStateV1::Healthy {
        "fixture_marks_healthy_but_does_not_establish_live_health"
    } else {
        "blocked_by_stale_unreachable_or_unknown_fixture_state"
    };
    Ok(FleetOperationPlanV1 {
        schema_version: PLAN_SCHEMA,
        state_slice: STATE_SLICE,
        preview_status,
        preview_digest,
        expected_preview_digest: request.expected_preview_digest,
        node_id: request.node_id.clone(),
        operation: request.operation,
        observed_health_state: node.health_state,
        health_gate,
        authority: "not_authorized",
        effect_boundary: "advisory_only_does_not_invoke_or_reserve",
        required_gates: vec![
            "verify_real_owner_and_target_under_a_separate_slice",
            "authorize_an_adapter_and_exact_operation",
            "obtain_operator_approval_for_the_exact_digest",
            "establish_current_health_with_an_owner_authorized_source",
        ],
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
    })
}

/// Lifecycle stage in the one local, virtual Fleet scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSimulationPhaseV1 {
    /// No virtual attachment has been recorded.
    Detached,
    /// The fixture node is attached to the virtual owner session.
    Attached,
    /// A virtual lease is active and available for a simulated workload.
    Launched,
    /// A virtual workload task is running in the scenario.
    WorkloadRunning,
    /// The simulated outcome is ambiguous and requires reconciliation.
    ReconciliationRequired,
    /// A partial simulated effect retains resources until cleanup.
    CleanupRequired,
    /// Virtual leases and task references have been released.
    Terminated,
}

/// State-changing command in the virtual Fleet simulator.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSimulationOperationV1 {
    /// Attach the fixed fixture node to the local simulation owner.
    Attach,
    /// Acquire a virtual lease from the four-unit fixture pool.
    Launch,
    /// Create a virtual workload task under an active virtual lease.
    WorkloadStart,
    /// Cancel the virtual workload task and retain its lease.
    Cancel,
    /// Release all virtual task and lease state.
    Terminate,
    /// Resolve a previously ambiguous simulated outcome.
    Reconcile,
    /// Release all virtual state after a partial or ambiguous transition.
    Cleanup,
}

/// Deterministic failure injected into one simulator transition.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSimulationFaultV1 {
    /// Apply the requested transition normally.
    #[default]
    None,
    /// Model a timeout where the simulated effect did not occur.
    TimeoutBeforeEffect,
    /// Model a timeout after the simulated effect occurred.
    TimeoutAfterEffect,
    /// Model a partial launch or terminate effect requiring cleanup.
    PartialEffect,
}

/// Virtual resource lease state, unrelated to a real reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSimulationLeaseStateV1 {
    /// No virtual lease exists.
    None,
    /// All requested virtual units are held in the scenario.
    Held,
    /// A partial virtual lease remains and needs cleanup.
    Partial,
    /// The possible lease state is intentionally unresolved.
    Unknown,
    /// Virtual units were released by termination or cleanup.
    Released,
}

/// Read-only view of the current service-owned virtual Fleet scenario.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetSimulationSnapshotV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this simulator.
    pub state_slice: &'static str,
    /// Fixed scenario identifier, not a service or provider ID.
    pub simulation_id: &'static str,
    /// Fixed synthetic node key.
    pub node_id: &'static str,
    /// Local service simulation owner label, not a person or legal entity.
    pub owner: &'static str,
    /// Current lifecycle stage in the virtual scenario.
    pub phase: FleetSimulationPhaseV1,
    /// Fixed virtual pool size; not a hardware capacity claim.
    pub virtual_capacity_units: u8,
    /// Units attributed to a known virtual lease.
    pub allocated_units: u8,
    /// Units conservatively retained while an outcome is ambiguous.
    pub quarantined_units: u8,
    /// Deterministic virtual lease label, never a reservation identifier.
    pub virtual_lease_id: Option<String>,
    /// Virtual lease state.
    pub lease_state: FleetSimulationLeaseStateV1,
    /// Deterministic virtual task label, never an OS process ID.
    pub virtual_task_id: Option<String>,
    /// Most recent requested transition.
    pub last_operation: Option<FleetSimulationOperationV1>,
    /// Operation awaiting deterministic simulator reconciliation, if any.
    pub pending_operation: Option<FleetSimulationOperationV1>,
    /// Count of accepted distinct transitions, including recovery actions.
    pub transition_count: u16,
    /// Digest of the complete state, including recovery metadata.
    pub state_digest: Sha256Digest,
    /// Persistence boundary for this simulator instance.
    pub persistence: &'static str,
    /// Maximum claim supported by this response.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
    /// Effect boundary for this result.
    pub effect_boundary: &'static str,
}

/// Request for one idempotent state transition in the local simulator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSimulationTransitionRequestV1 {
    /// Digest of the snapshot most recently reviewed by the caller.
    pub expected_state_digest: Sha256Digest,
    /// Caller-generated retry key; 1–64 ASCII letters, digits, `-`, `_`, `:`, or `.`.
    pub idempotency_key: String,
    /// Requested simulator operation.
    pub operation: FleetSimulationOperationV1,
    /// Launch units; valid only for launch and bounded by the virtual pool.
    #[serde(default)]
    pub gpu_units: Option<u8>,
    /// Deterministic simulator-only fault scenario.
    #[serde(default)]
    pub fault: FleetSimulationFaultV1,
}

/// Result of an idempotent simulator state transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetSimulationTransitionV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this transition.
    pub state_slice: &'static str,
    /// Operation applied or resolved by the simulator.
    pub operation: FleetSimulationOperationV1,
    /// Result status, including explicit failure and recovery cases.
    pub transition_status: &'static str,
    /// Digest of the exact transition request.
    pub request_digest: Sha256Digest,
    /// State digest before the transition.
    pub state_before_digest: Sha256Digest,
    /// Exact snapshot returned when this transition was first accepted.
    pub snapshot: FleetSimulationSnapshotV1,
    /// Effect boundary for this result.
    pub effect_boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct FleetSimulationCoreV1 {
    phase: FleetSimulationPhaseV1,
    allocated_units: u8,
    quarantined_units: u8,
    virtual_lease_id: Option<String>,
    lease_state: FleetSimulationLeaseStateV1,
    virtual_task_id: Option<String>,
    last_operation: Option<FleetSimulationOperationV1>,
    transition_count: u16,
}

impl Default for FleetSimulationCoreV1 {
    fn default() -> Self {
        Self {
            phase: FleetSimulationPhaseV1::Detached,
            allocated_units: 0,
            quarantined_units: 0,
            virtual_lease_id: None,
            lease_state: FleetSimulationLeaseStateV1::None,
            virtual_task_id: None,
            last_operation: None,
            transition_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct FleetSimulationPendingV1 {
    operation: FleetSimulationOperationV1,
    effect_applied: bool,
    before: FleetSimulationCoreV1,
    after: FleetSimulationCoreV1,
}

#[derive(Clone, Debug, Default)]
struct FleetSimulationStoreStateV1 {
    core: FleetSimulationCoreV1,
    pending: Option<FleetSimulationPendingV1>,
    idempotency: HashMap<String, (Sha256Digest, FleetSimulationTransitionV1)>,
}

/// Bounded, volatile simulator state owned by one local service process.
#[derive(Debug, Default)]
pub struct FleetSimulationStoreV1 {
    state: Mutex<FleetSimulationStoreStateV1>,
}

#[derive(Serialize)]
struct FleetSimulationRequestDigestBodyV1<'a> {
    schema_version: &'static str,
    state_slice: &'static str,
    request: &'a FleetSimulationTransitionRequestV1,
}

#[derive(Serialize)]
struct FleetSimulationStateDigestBodyV1<'a> {
    schema_version: &'static str,
    state_slice: &'static str,
    core: &'a FleetSimulationCoreV1,
    pending: &'a Option<FleetSimulationPendingV1>,
}

/// Return the initial or current simulator snapshot.
///
/// # Errors
///
/// Returns an error if the service-owned lock is poisoned or the state cannot
/// be canonically encoded.
pub fn simulation_snapshot_v1(
    store: &FleetSimulationStoreV1,
) -> Result<FleetSimulationSnapshotV1, FleetError> {
    let state = store
        .state
        .lock()
        .map_err(|_| FleetError::SimulationUnavailable)?;
    simulation_snapshot_from_state(&state)
}

/// Apply one digest-bound simulator transition with bounded idempotent replay.
///
/// # Errors
///
/// Returns an error for a stale state digest, conflicting retry key, invalid
/// request, rejected lifecycle transition, full idempotency cache, poisoned
/// service lock, or canonicalization failure.
pub fn transition_simulation_v1(
    store: &FleetSimulationStoreV1,
    request: &FleetSimulationTransitionRequestV1,
) -> Result<FleetSimulationTransitionV1, FleetError> {
    validate_simulation_request(request)?;
    let request_digest = domain_separated_digest(
        "hyperai.fleet.simulation.request.v1",
        &FleetSimulationRequestDigestBodyV1 {
            schema_version: SIMULATION_TRANSITION_SCHEMA,
            state_slice: SIMULATION_STATE_SLICE,
            request,
        },
    )?;
    let mut state = store
        .state
        .lock()
        .map_err(|_| FleetError::SimulationUnavailable)?;
    if let Some((cached_digest, cached_result)) = state.idempotency.get(&request.idempotency_key) {
        return if *cached_digest == request_digest {
            Ok(cached_result.clone())
        } else {
            Err(FleetError::SimulationConflict)
        };
    }
    if state.idempotency.len() >= MAX_SIMULATION_IDEMPOTENCY_KEYS {
        return Err(FleetError::SimulationCacheFull);
    }
    let before_snapshot = simulation_snapshot_from_state(&state)?;
    if request.expected_state_digest != before_snapshot.state_digest {
        return Err(FleetError::SimulationConflict);
    }

    let (next_core, next_pending, transition_status) =
        apply_simulation_request(&state, request, request_digest)?;
    state.core = next_core;
    state.pending = next_pending;
    let snapshot = simulation_snapshot_from_state(&state)?;
    let transition = FleetSimulationTransitionV1 {
        schema_version: SIMULATION_TRANSITION_SCHEMA,
        state_slice: SIMULATION_STATE_SLICE,
        operation: request.operation,
        transition_status,
        request_digest,
        state_before_digest: before_snapshot.state_digest,
        snapshot,
        effect_boundary: "simulation_only_no_process_no_network",
    };
    state.idempotency.insert(
        request.idempotency_key.clone(),
        (request_digest, transition.clone()),
    );
    Ok(transition)
}

fn validate_simulation_request(
    request: &FleetSimulationTransitionRequestV1,
) -> Result<(), FleetError> {
    let key = request.idempotency_key.as_str();
    if key.is_empty()
        || key.len() > 64
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
    {
        return Err(FleetError::InvalidSimulationRequest);
    }
    match request.operation {
        FleetSimulationOperationV1::Launch => {
            let units = request.gpu_units.unwrap_or(1);
            if units == 0 || units > SIMULATION_CAPACITY_UNITS {
                return Err(FleetError::InvalidSimulationRequest);
            }
            if request.fault == FleetSimulationFaultV1::PartialEffect && units < 2 {
                return Err(FleetError::InvalidSimulationRequest);
            }
        }
        FleetSimulationOperationV1::Reconcile | FleetSimulationOperationV1::Cleanup => {
            if request.gpu_units.is_some() || request.fault != FleetSimulationFaultV1::None {
                return Err(FleetError::InvalidSimulationRequest);
            }
        }
        FleetSimulationOperationV1::Terminate => {
            if request.gpu_units.is_some() {
                return Err(FleetError::InvalidSimulationRequest);
            }
        }
        FleetSimulationOperationV1::Attach
        | FleetSimulationOperationV1::WorkloadStart
        | FleetSimulationOperationV1::Cancel => {
            if request.gpu_units.is_some() || request.fault == FleetSimulationFaultV1::PartialEffect
            {
                return Err(FleetError::InvalidSimulationRequest);
            }
        }
    }
    Ok(())
}

fn apply_simulation_request(
    state: &FleetSimulationStoreStateV1,
    request: &FleetSimulationTransitionRequestV1,
    request_digest: Sha256Digest,
) -> Result<
    (
        FleetSimulationCoreV1,
        Option<FleetSimulationPendingV1>,
        &'static str,
    ),
    FleetError,
> {
    match request.operation {
        FleetSimulationOperationV1::Reconcile => {
            if request.fault != FleetSimulationFaultV1::None {
                return Err(FleetError::InvalidSimulationRequest);
            }
            let pending = state
                .pending
                .as_ref()
                .ok_or(FleetError::SimulationTransitionRejected)?;
            let mut resolved = if pending.effect_applied {
                pending.after.clone()
            } else {
                pending.before.clone()
            };
            resolved.transition_count = state.core.transition_count.saturating_add(1);
            resolved.last_operation = Some(FleetSimulationOperationV1::Reconcile);
            let status = if pending.effect_applied {
                "reconciled_effect_applied"
            } else {
                "reconciled_no_effect"
            };
            Ok((resolved, None, status))
        }
        FleetSimulationOperationV1::Cleanup => {
            if !matches!(
                state.core.phase,
                FleetSimulationPhaseV1::CleanupRequired
                    | FleetSimulationPhaseV1::ReconciliationRequired
            ) {
                return Err(FleetError::SimulationTransitionRejected);
            }
            let mut cleaned = cleanup_core(&state.core);
            cleaned.transition_count = state.core.transition_count.saturating_add(1);
            cleaned.last_operation = Some(FleetSimulationOperationV1::Cleanup);
            Ok((cleaned, None, "cleanup_released_virtual_state"))
        }
        _ => {
            if state.pending.is_some()
                || matches!(
                    state.core.phase,
                    FleetSimulationPhaseV1::CleanupRequired
                        | FleetSimulationPhaseV1::ReconciliationRequired
                        | FleetSimulationPhaseV1::Terminated
                )
            {
                return Err(FleetError::SimulationTransitionRejected);
            }
            let next = normal_simulation_transition(&state.core, request, request_digest)?;
            match request.fault {
                FleetSimulationFaultV1::None => Ok((next, None, "transition_applied")),
                FleetSimulationFaultV1::PartialEffect => {
                    let partial = partial_simulation_transition(&state.core, next, request)?;
                    Ok((partial, None, "partial_effect_cleanup_required"))
                }
                FleetSimulationFaultV1::TimeoutBeforeEffect
                | FleetSimulationFaultV1::TimeoutAfterEffect => {
                    let effect_applied =
                        request.fault == FleetSimulationFaultV1::TimeoutAfterEffect;
                    let prior_core = state.core.clone();
                    let mut next_with_count = next;
                    next_with_count.transition_count =
                        state.core.transition_count.saturating_add(1);
                    let quarantined_units = prior_core
                        .allocated_units
                        .saturating_add(prior_core.quarantined_units)
                        .max(
                            next_with_count
                                .allocated_units
                                .saturating_add(next_with_count.quarantined_units),
                        )
                        .max(request.gpu_units.unwrap_or(0));
                    let unknown_core = FleetSimulationCoreV1 {
                        phase: FleetSimulationPhaseV1::ReconciliationRequired,
                        allocated_units: 0,
                        quarantined_units,
                        virtual_lease_id: None,
                        lease_state: FleetSimulationLeaseStateV1::Unknown,
                        virtual_task_id: None,
                        last_operation: Some(request.operation),
                        transition_count: state.core.transition_count.saturating_add(1),
                    };
                    let pending = FleetSimulationPendingV1 {
                        operation: request.operation,
                        effect_applied,
                        before: prior_core,
                        after: next_with_count,
                    };
                    Ok((
                        unknown_core,
                        Some(pending),
                        "outcome_unknown_reconciliation_required",
                    ))
                }
            }
        }
    }
}

fn normal_simulation_transition(
    current: &FleetSimulationCoreV1,
    request: &FleetSimulationTransitionRequestV1,
    request_digest: Sha256Digest,
) -> Result<FleetSimulationCoreV1, FleetError> {
    let mut next = current.clone();
    match request.operation {
        FleetSimulationOperationV1::Attach if current.phase == FleetSimulationPhaseV1::Detached => {
            next.phase = FleetSimulationPhaseV1::Attached;
        }
        FleetSimulationOperationV1::Launch if current.phase == FleetSimulationPhaseV1::Attached => {
            let units = request.gpu_units.unwrap_or(1);
            next.phase = FleetSimulationPhaseV1::Launched;
            next.allocated_units = units;
            next.quarantined_units = 0;
            next.virtual_lease_id =
                Some(format!("virtual-lease-{}", digest_prefix(request_digest)));
            next.lease_state = FleetSimulationLeaseStateV1::Held;
        }
        FleetSimulationOperationV1::WorkloadStart
            if current.phase == FleetSimulationPhaseV1::Launched =>
        {
            next.phase = FleetSimulationPhaseV1::WorkloadRunning;
            next.virtual_task_id = Some(format!("virtual-task-{}", digest_prefix(request_digest)));
        }
        FleetSimulationOperationV1::Cancel
            if current.phase == FleetSimulationPhaseV1::WorkloadRunning =>
        {
            next.phase = FleetSimulationPhaseV1::Launched;
            next.virtual_task_id = None;
        }
        FleetSimulationOperationV1::Terminate
            if matches!(
                current.phase,
                FleetSimulationPhaseV1::Attached
                    | FleetSimulationPhaseV1::Launched
                    | FleetSimulationPhaseV1::WorkloadRunning
            ) =>
        {
            next = cleanup_core(current);
            next.phase = FleetSimulationPhaseV1::Terminated;
        }
        _ => return Err(FleetError::SimulationTransitionRejected),
    }
    next.last_operation = Some(request.operation);
    next.transition_count = current.transition_count.saturating_add(1);
    Ok(next)
}

fn partial_simulation_transition(
    current: &FleetSimulationCoreV1,
    mut next: FleetSimulationCoreV1,
    request: &FleetSimulationTransitionRequestV1,
) -> Result<FleetSimulationCoreV1, FleetError> {
    match request.operation {
        FleetSimulationOperationV1::Launch => {
            next.phase = FleetSimulationPhaseV1::CleanupRequired;
            next.allocated_units = request.gpu_units.unwrap_or(1).saturating_sub(1);
            next.lease_state = FleetSimulationLeaseStateV1::Partial;
        }
        FleetSimulationOperationV1::Terminate if current.allocated_units >= 2 => {
            next.phase = FleetSimulationPhaseV1::CleanupRequired;
            next.allocated_units = current.allocated_units / 2;
            next.quarantined_units = 0;
            next.virtual_lease_id.clone_from(&current.virtual_lease_id);
            next.lease_state = FleetSimulationLeaseStateV1::Partial;
            next.virtual_task_id = None;
        }
        _ => return Err(FleetError::InvalidSimulationRequest),
    }
    next.last_operation = Some(request.operation);
    next.transition_count = current.transition_count.saturating_add(1);
    Ok(next)
}

fn cleanup_core(current: &FleetSimulationCoreV1) -> FleetSimulationCoreV1 {
    FleetSimulationCoreV1 {
        phase: FleetSimulationPhaseV1::Terminated,
        allocated_units: 0,
        quarantined_units: 0,
        virtual_lease_id: None,
        lease_state: FleetSimulationLeaseStateV1::Released,
        virtual_task_id: None,
        last_operation: current.last_operation,
        transition_count: current.transition_count,
    }
}

fn digest_prefix(digest: Sha256Digest) -> String {
    digest.to_string().chars().take(16).collect()
}

fn simulation_snapshot_from_state(
    state: &FleetSimulationStoreStateV1,
) -> Result<FleetSimulationSnapshotV1, FleetError> {
    let state_digest = domain_separated_digest(
        "hyperai.fleet.simulation.state.v1",
        &FleetSimulationStateDigestBodyV1 {
            schema_version: SIMULATION_SCHEMA,
            state_slice: SIMULATION_STATE_SLICE,
            core: &state.core,
            pending: &state.pending,
        },
    )?;
    Ok(FleetSimulationSnapshotV1 {
        schema_version: SIMULATION_SCHEMA,
        state_slice: SIMULATION_STATE_SLICE,
        simulation_id: "fleet-scenario-atlas-v1",
        node_id: "fixture-node-atlas",
        owner: "local_service_simulation",
        phase: state.core.phase,
        virtual_capacity_units: SIMULATION_CAPACITY_UNITS,
        allocated_units: state.core.allocated_units,
        quarantined_units: state.core.quarantined_units,
        virtual_lease_id: state.core.virtual_lease_id.clone(),
        lease_state: state.core.lease_state,
        virtual_task_id: state.core.virtual_task_id.clone(),
        last_operation: state.core.last_operation,
        pending_operation: state.pending.as_ref().map(|pending| pending.operation),
        transition_count: state.core.transition_count,
        state_digest,
        persistence: "volatile_one_local_service_process",
        claim_ceiling: SIMULATION_CLAIM_CEILING,
        nonclaims: SIMULATION_NONCLAIMS,
        effect_boundary: "simulation_only_no_process_no_network",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_is_deterministic_and_keeps_each_health_state_explicit() -> Result<(), FleetError> {
        let first = inventory_v1()?;
        let second = inventory_v1()?;
        assert_eq!(first, second);
        assert_eq!(first.nodes.len(), 4);
        assert_eq!(first.health_timeline.len(), 4);
        assert_eq!(
            first
                .nodes
                .iter()
                .map(|node| node.health_state)
                .collect::<Vec<_>>(),
            vec![
                FleetHealthStateV1::Healthy,
                FleetHealthStateV1::Stale,
                FleetHealthStateV1::Unreachable,
                FleetHealthStateV1::Unknown,
            ]
        );
        assert_eq!(first.provenance.observation_mode, "no_live_probe_or_import");
        assert_eq!(
            first.effect_boundary,
            "local_synthetic_read_only_no_probe_no_external_effects"
        );
        Ok(())
    }

    #[test]
    fn doctor_keeps_live_health_unknown_and_operations_blocked() {
        let doctor = doctor_v1();
        assert_eq!(doctor.mode, "synthetic_fixture_only");
        assert_eq!(doctor.checks[3].status, "unknown");
        assert_eq!(doctor.checks[4].status, "blocked");
        assert_eq!(
            doctor.effect_boundary,
            "local_fixture_validation_only_no_host_or_provider_checks"
        );
    }

    #[test]
    fn plans_are_stable_non_executable_and_report_changed_intents() -> Result<(), FleetError> {
        let request = FleetPlanRequestV1 {
            node_id: "fixture-node-atlas".to_owned(),
            operation: FleetOperationV1::Launch,
            expected_preview_digest: None,
        };
        let first = preview_operation_v1(&request)?;
        let second = preview_operation_v1(&request)?;
        assert_eq!(first.preview_digest, second.preview_digest);
        assert_eq!(first.preview_status, "preview_ready");
        assert_eq!(first.authority, "not_authorized");
        assert_eq!(
            first.effect_boundary,
            "advisory_only_does_not_invoke_or_reserve"
        );

        let changed = preview_operation_v1(&FleetPlanRequestV1 {
            node_id: "fixture-node-atlas".to_owned(),
            operation: FleetOperationV1::Terminate,
            expected_preview_digest: Some(first.preview_digest),
        })?;
        assert_eq!(changed.preview_status, "preview_changed");
        assert_ne!(changed.preview_digest, first.preview_digest);
        Ok(())
    }

    #[test]
    fn stale_unreachable_unknown_and_invalid_targets_remain_blocked() -> Result<(), FleetError> {
        for (node_id, state) in [
            ("fixture-node-boreal", FleetHealthStateV1::Stale),
            ("fixture-node-cinder", FleetHealthStateV1::Unreachable),
            ("fixture-node-delta", FleetHealthStateV1::Unknown),
        ] {
            let plan = preview_operation_v1(&FleetPlanRequestV1 {
                node_id: node_id.to_owned(),
                operation: FleetOperationV1::WorkloadStart,
                expected_preview_digest: None,
            })?;
            assert_eq!(plan.observed_health_state, state);
            assert_eq!(
                plan.health_gate,
                "blocked_by_stale_unreachable_or_unknown_fixture_state"
            );
            assert_eq!(plan.authority, "not_authorized");
        }
        assert!(matches!(
            preview_operation_v1(&FleetPlanRequestV1 {
                node_id: "actual-hostname.example".to_owned(),
                operation: FleetOperationV1::Attach,
                expected_preview_digest: None,
            }),
            Err(FleetError::UnknownNode)
        ));
        Ok(())
    }

    #[test]
    fn request_contract_rejects_unknown_fields() {
        let parsed = serde_json::from_str::<FleetPlanRequestV1>(
            r#"{"node_id":"fixture-node-atlas","operation":"launch","ssh_host":"host"}"#,
        );
        assert!(parsed.is_err());
    }
}
