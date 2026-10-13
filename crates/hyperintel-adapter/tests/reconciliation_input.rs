//! Serialized `HyperIntel` lineage integration checks for the `HyperAI` adapter.

#[path = "support/reconciliation.rs"]
mod support;

use std::error::Error;

use hyperai_hyperintel_adapter::{HyperIntelReconciliationError, reconcile_execution_input_v1};
use hyperintel_reconciliation::ReceiptReconciliationStatusV1;
use serde_json::Value;

#[tokio::test]
async fn serialized_synthetic_lineage_returns_only_identity_report() -> Result<(), Box<dyn Error>> {
    let input = support::build_completed_input().await?;
    let result = reconcile_execution_input_v1(&input)?;

    assert_eq!(result.status, "local_record_reconciliation_pass");
    assert_eq!(
        result.report.status,
        ReceiptReconciliationStatusV1::Completed
    );
    assert!(!result.authority_granted);
    assert!(!result.inputs_persisted);
    assert!(result.report.report_digest == result.report.digest()?);

    let report_json = serde_json::to_string(&result)?;
    assert!(!report_json.contains(support::STREAM_CANARY));
    for omitted in [
        "\"chunks\"",
        "\"messages\"",
        "\"signature_hex\"",
        "\"public_key_hex\"",
        "\"credential\"",
    ] {
        assert!(!report_json.contains(omitted));
    }
    Ok(())
}

#[tokio::test]
async fn changed_digest_bound_routing_time_is_rejected() -> Result<(), Box<dyn Error>> {
    let mut input = support::build_completed_input().await?;
    let mut document: Value = serde_json::from_str(&input.execution_input)?;
    document["routing"]["evaluated_at"] = Value::String("2026-07-20T12:01:00Z".to_owned());
    input.execution_input = serde_json::to_string(&document)?;

    assert!(matches!(
        reconcile_execution_input_v1(&input),
        Err(HyperIntelReconciliationError::ReconciliationFailed)
    ));
    Ok(())
}
