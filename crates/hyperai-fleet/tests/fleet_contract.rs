//! Public Fleet contract serialization and preview-boundary checks.

use hyperai_fleet::{
    FleetOperationV1, FleetPlanRequestV1, FleetSimulationFaultV1, FleetSimulationOperationV1,
    FleetSimulationPhaseV1, FleetSimulationStoreV1, FleetSimulationTransitionRequestV1, doctor_v1,
    inventory_v1, preview_operation_v1, simulation_snapshot_v1, transition_simulation_v1,
};

#[test]
fn public_inventory_and_doctor_are_versioned_and_fixture_labelled()
-> Result<(), Box<dyn std::error::Error>> {
    let inventory = inventory_v1()?;
    let inventory_json = serde_json::to_value(&inventory)?;
    let doctor_json = serde_json::to_value(doctor_v1())?;

    assert_eq!(
        inventory_json["schema_version"],
        "hyperai.fleet.inventory.v1"
    );
    assert_eq!(
        inventory_json["state_slice"],
        "hyperai.bazaar.fleet_readonly.v1"
    );
    assert_eq!(
        inventory_json["provenance"]["fixture_author"],
        "HyperAI synthetic fixture"
    );
    assert_eq!(
        inventory_json["provenance"]["observation_mode"],
        "no_live_probe_or_import"
    );
    assert_eq!(doctor_json["checks"][3]["status"], "unknown");
    assert_eq!(doctor_json["checks"][4]["status"], "blocked");
    assert_eq!(
        inventory_json["effect_boundary"],
        "local_synthetic_read_only_no_probe_no_external_effects"
    );
    Ok(())
}

#[test]
fn preview_binds_target_and_operation_without_exposing_an_action()
-> Result<(), Box<dyn std::error::Error>> {
    let first = preview_operation_v1(&FleetPlanRequestV1 {
        node_id: "fixture-node-atlas".to_owned(),
        operation: FleetOperationV1::Launch,
        expected_preview_digest: None,
    })?;
    let same = preview_operation_v1(&FleetPlanRequestV1 {
        node_id: "fixture-node-atlas".to_owned(),
        operation: FleetOperationV1::Launch,
        expected_preview_digest: Some(first.preview_digest),
    })?;
    let changed = preview_operation_v1(&FleetPlanRequestV1 {
        node_id: "fixture-node-atlas".to_owned(),
        operation: FleetOperationV1::Terminate,
        expected_preview_digest: Some(first.preview_digest),
    })?;
    let value = serde_json::to_value(&first)?;

    assert_eq!(first.preview_digest, same.preview_digest);
    assert_eq!(same.preview_status, "preview_ready");
    assert_eq!(changed.preview_status, "preview_changed");
    assert_ne!(first.preview_digest, changed.preview_digest);
    assert_eq!(value["authority"], "not_authorized");
    assert_eq!(
        value["effect_boundary"],
        "advisory_only_does_not_invoke_or_reserve"
    );
    assert!(value.get("command").is_none());
    assert!(value.get("reservation_id").is_none());
    Ok(())
}

#[test]
fn nonhealthy_states_and_nonfixture_targets_do_not_become_actionable() {
    for node_id in [
        "fixture-node-boreal",
        "fixture-node-cinder",
        "fixture-node-delta",
    ] {
        let plan = preview_operation_v1(&FleetPlanRequestV1 {
            node_id: node_id.to_owned(),
            operation: FleetOperationV1::WorkloadStart,
            expected_preview_digest: None,
        });
        assert!(plan.is_ok_and(|result| {
            result.authority == "not_authorized"
                && result.effect_boundary == "advisory_only_does_not_invoke_or_reserve"
        }));
    }
    assert!(
        preview_operation_v1(&FleetPlanRequestV1 {
            node_id: "actual-host".to_owned(),
            operation: FleetOperationV1::Attach,
            expected_preview_digest: None,
        })
        .is_err()
    );
}

fn simulate(
    store: &FleetSimulationStoreV1,
    operation: FleetSimulationOperationV1,
    key: &str,
    gpu_units: Option<u8>,
    fault: FleetSimulationFaultV1,
) -> Result<hyperai_fleet::FleetSimulationTransitionV1, hyperai_fleet::FleetError> {
    let snapshot = simulation_snapshot_v1(store)?;
    transition_simulation_v1(
        store,
        &FleetSimulationTransitionRequestV1 {
            expected_state_digest: snapshot.state_digest,
            idempotency_key: key.to_owned(),
            operation,
            gpu_units,
            fault,
        },
    )
}

#[test]
fn simulator_accounts_virtual_owner_lease_task_and_cleanup()
-> Result<(), Box<dyn std::error::Error>> {
    let store = FleetSimulationStoreV1::default();
    let initial = simulation_snapshot_v1(&store)?;
    assert_eq!(initial.phase, FleetSimulationPhaseV1::Detached);
    assert_eq!(initial.virtual_capacity_units, 4);
    assert_eq!(initial.owner, "local_service_simulation");

    let attached = simulate(
        &store,
        FleetSimulationOperationV1::Attach,
        "attach-1",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(attached.snapshot.phase, FleetSimulationPhaseV1::Attached);
    let launched = simulate(
        &store,
        FleetSimulationOperationV1::Launch,
        "launch-1",
        Some(3),
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(launched.snapshot.phase, FleetSimulationPhaseV1::Launched);
    assert_eq!(launched.snapshot.allocated_units, 3);
    assert!(
        launched
            .snapshot
            .virtual_lease_id
            .as_deref()
            .is_some_and(|value| value.starts_with("virtual-lease-"))
    );

    let started = simulate(
        &store,
        FleetSimulationOperationV1::WorkloadStart,
        "workload-1",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(
        started.snapshot.phase,
        FleetSimulationPhaseV1::WorkloadRunning
    );
    assert!(
        started
            .snapshot
            .virtual_task_id
            .as_deref()
            .is_some_and(|value| value.starts_with("virtual-task-"))
    );

    let cancelled = simulate(
        &store,
        FleetSimulationOperationV1::Cancel,
        "cancel-1",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(cancelled.snapshot.phase, FleetSimulationPhaseV1::Launched);
    assert_eq!(cancelled.snapshot.allocated_units, 3);
    assert_eq!(cancelled.snapshot.virtual_task_id, None);

    let terminated = simulate(
        &store,
        FleetSimulationOperationV1::Terminate,
        "terminate-1",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(
        terminated.snapshot.phase,
        FleetSimulationPhaseV1::Terminated
    );
    assert_eq!(terminated.snapshot.allocated_units, 0);
    assert_eq!(
        terminated.snapshot.lease_state,
        hyperai_fleet::FleetSimulationLeaseStateV1::Released
    );
    assert_eq!(
        terminated.effect_boundary,
        "simulation_only_no_process_no_network"
    );
    let json = serde_json::to_value(&terminated)?;
    assert!(json.get("command").is_none());
    assert!(json.get("pid").is_none());
    Ok(())
}

#[test]
fn simulator_replays_exact_retry_and_rejects_changed_or_stale_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let store = FleetSimulationStoreV1::default();
    let initial = simulation_snapshot_v1(&store)?;
    let request = FleetSimulationTransitionRequestV1 {
        expected_state_digest: initial.state_digest,
        idempotency_key: "attach-once".to_owned(),
        operation: FleetSimulationOperationV1::Attach,
        gpu_units: None,
        fault: FleetSimulationFaultV1::None,
    };
    let first = transition_simulation_v1(&store, &request)?;
    let replay = transition_simulation_v1(&store, &request)?;
    assert_eq!(first, replay);
    assert_eq!(simulation_snapshot_v1(&store)?.transition_count, 1);

    let changed = FleetSimulationTransitionRequestV1 {
        operation: FleetSimulationOperationV1::Launch,
        gpu_units: Some(1),
        ..request.clone()
    };
    assert!(matches!(
        transition_simulation_v1(&store, &changed),
        Err(hyperai_fleet::FleetError::SimulationConflict)
    ));
    let stale = FleetSimulationTransitionRequestV1 {
        idempotency_key: "stale-state".to_owned(),
        ..request
    };
    assert!(matches!(
        transition_simulation_v1(&store, &stale),
        Err(hyperai_fleet::FleetError::SimulationConflict)
    ));
    Ok(())
}

#[test]
fn simulator_timeout_reconciliation_and_partial_cleanup_are_explicit()
-> Result<(), Box<dyn std::error::Error>> {
    let store = FleetSimulationStoreV1::default();
    simulate(
        &store,
        FleetSimulationOperationV1::Attach,
        "attach-1",
        None,
        FleetSimulationFaultV1::None,
    )?;
    let unknown_before = simulate(
        &store,
        FleetSimulationOperationV1::Launch,
        "launch-timeout-before",
        Some(2),
        FleetSimulationFaultV1::TimeoutBeforeEffect,
    )?;
    assert_eq!(
        unknown_before.transition_status,
        "outcome_unknown_reconciliation_required"
    );
    assert_eq!(
        unknown_before.snapshot.phase,
        FleetSimulationPhaseV1::ReconciliationRequired
    );
    assert_eq!(unknown_before.snapshot.quarantined_units, 2);
    let reconciled_before = simulate(
        &store,
        FleetSimulationOperationV1::Reconcile,
        "reconcile-before",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(reconciled_before.transition_status, "reconciled_no_effect");
    assert_eq!(
        reconciled_before.snapshot.phase,
        FleetSimulationPhaseV1::Attached
    );
    assert_eq!(reconciled_before.snapshot.quarantined_units, 0);

    let unknown_after = simulate(
        &store,
        FleetSimulationOperationV1::Launch,
        "launch-timeout-after",
        Some(3),
        FleetSimulationFaultV1::TimeoutAfterEffect,
    )?;
    assert_eq!(
        unknown_after.snapshot.phase,
        FleetSimulationPhaseV1::ReconciliationRequired
    );
    assert_eq!(unknown_after.snapshot.quarantined_units, 3);
    let reconciled_after = simulate(
        &store,
        FleetSimulationOperationV1::Reconcile,
        "reconcile-after",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(
        reconciled_after.transition_status,
        "reconciled_effect_applied"
    );
    assert_eq!(
        reconciled_after.snapshot.phase,
        FleetSimulationPhaseV1::Launched
    );
    assert_eq!(reconciled_after.snapshot.allocated_units, 3);

    let partial = simulate(
        &store,
        FleetSimulationOperationV1::Terminate,
        "terminate-partial",
        None,
        FleetSimulationFaultV1::PartialEffect,
    )?;
    assert_eq!(
        partial.snapshot.phase,
        FleetSimulationPhaseV1::CleanupRequired
    );
    assert!(partial.snapshot.allocated_units > 0);
    let cleanup = simulate(
        &store,
        FleetSimulationOperationV1::Cleanup,
        "cleanup-partial",
        None,
        FleetSimulationFaultV1::None,
    )?;
    assert_eq!(cleanup.snapshot.phase, FleetSimulationPhaseV1::Terminated);
    assert_eq!(cleanup.snapshot.allocated_units, 0);
    assert_eq!(cleanup.snapshot.virtual_task_id, None);
    Ok(())
}

#[test]
fn simulator_rejects_unsafe_faults_and_bounds_retry_records()
-> Result<(), Box<dyn std::error::Error>> {
    let store = FleetSimulationStoreV1::default();
    let invalid = FleetSimulationTransitionRequestV1 {
        expected_state_digest: simulation_snapshot_v1(&store)?.state_digest,
        idempotency_key: "invalid".to_owned(),
        operation: FleetSimulationOperationV1::Launch,
        gpu_units: Some(5),
        fault: FleetSimulationFaultV1::None,
    };
    assert!(matches!(
        transition_simulation_v1(&store, &invalid),
        Err(hyperai_fleet::FleetError::InvalidSimulationRequest)
    ));

    simulate(
        &store,
        FleetSimulationOperationV1::Attach,
        "attach-1",
        None,
        FleetSimulationFaultV1::None,
    )?;
    simulate(
        &store,
        FleetSimulationOperationV1::Launch,
        "launch-1",
        Some(1),
        FleetSimulationFaultV1::None,
    )?;
    for index in 0..62 {
        simulate(
            &store,
            FleetSimulationOperationV1::WorkloadStart,
            &format!("timeout-{index}"),
            None,
            FleetSimulationFaultV1::TimeoutBeforeEffect,
        )?;
        simulate(
            &store,
            FleetSimulationOperationV1::Reconcile,
            &format!("reconcile-{index}"),
            None,
            FleetSimulationFaultV1::None,
        )?;
    }
    let final_unknown = simulate(
        &store,
        FleetSimulationOperationV1::WorkloadStart,
        "timeout-final",
        None,
        FleetSimulationFaultV1::TimeoutBeforeEffect,
    )?;
    assert_eq!(
        transition_simulation_v1(
            &store,
            &FleetSimulationTransitionRequestV1 {
                expected_state_digest: final_unknown.state_before_digest,
                idempotency_key: "timeout-final".to_owned(),
                operation: FleetSimulationOperationV1::WorkloadStart,
                gpu_units: None,
                fault: FleetSimulationFaultV1::TimeoutBeforeEffect,
            }
        )?,
        final_unknown
    );
    simulate(
        &store,
        FleetSimulationOperationV1::Reconcile,
        "reconcile-final",
        None,
        FleetSimulationFaultV1::None,
    )?;
    let full = FleetSimulationTransitionRequestV1 {
        expected_state_digest: simulation_snapshot_v1(&store)?.state_digest,
        idempotency_key: "cache-full".to_owned(),
        operation: FleetSimulationOperationV1::WorkloadStart,
        gpu_units: None,
        fault: FleetSimulationFaultV1::None,
    };
    assert!(matches!(
        transition_simulation_v1(&store, &full),
        Err(hyperai_fleet::FleetError::SimulationCacheFull)
    ));
    Ok(())
}
