//! Versioned public and protected operator API composition.

mod readiness;

pub use readiness::{
    DependencyReadiness, DependencyState, MAX_READINESS_AGE_SECONDS, ReadinessClock,
    ReadinessClockError, ReadinessConfigError, ReadinessFreshness, ReadinessMonitor,
    ReadinessProviderError, ReadinessSnapshot, ReadinessSnapshotProvider, SystemReadinessClock,
};

use std::{path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use outcomes_application::{LocalScenario, ScenarioReport, run_local_scenario};
use outcomes_domain::{
    ClaimSpecV1, MarketBindingV1, ResolutionRecordV1, SettlementNetwork, Sha256Digest,
    domain_separated_digest,
};
use outcomes_settlement::{CapabilityMatrix, SettlementPreflight, preflight};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

const CAPABILITIES: &str = include_str!("../../../config/adapter-capabilities.json");
/// Checked-in `OpenAPI` 3.1 document validated against implemented routes.
pub const OPENAPI_JSON: &str = include_str!("../../../docs/openapi.json");

/// API configuration containing references, never raw production secrets.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Root used for local content-addressed demo artifacts.
    pub artifact_root: PathBuf,
    /// Expected development role header. Production deployments replace this
    /// boundary with an authenticated identity provider.
    pub operator_role: String,
}

#[derive(Debug, Clone)]
struct AppState {
    config: Arc<ApiConfig>,
    capabilities: Value,
    readiness: ReadinessMonitor,
}

/// API construction failure.
#[derive(Debug, Error)]
pub enum ApiBuildError {
    /// Checked-in capability JSON was invalid.
    #[error("checked-in adapter capability JSON is invalid: {0}")]
    CapabilityJson(#[from] serde_json::Error),
}

/// Build the versioned API router.
///
/// # Errors
///
/// Returns an error if the checked-in capability record cannot be parsed.
pub fn router(config: ApiConfig) -> Result<Router, ApiBuildError> {
    router_with_readiness(config, ReadinessMonitor::unconfigured())
}

/// Build the versioned API router with an injected cached-readiness monitor.
///
/// This constructor evaluates the injected provider and clock on each readiness
/// request. Neither this constructor nor the request handlers actively probe a
/// database, object store, signer, source adapter, or network service.
///
/// # Errors
///
/// Returns an error if the checked-in capability record cannot be parsed.
pub fn router_with_readiness(
    config: ApiConfig,
    readiness: ReadinessMonitor,
) -> Result<Router, ApiBuildError> {
    let state = AppState {
        config: Arc::new(config),
        capabilities: serde_json::from_str(CAPABILITIES)?,
        readiness,
    };
    Ok(Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/health/dependencies", get(dependencies))
        .route("/v1/adapters/capabilities", get(capabilities))
        .route("/v1/claims/validate", post(validate_claim))
        .route("/v1/demo/{scenario}", post(run_demo))
        .route(
            "/v1/settlement/mock/preflight",
            post(run_mock_settlement_preflight),
        )
        .route(
            "/v1/settlement/mock/workflow",
            post(run_mock_settlement_workflow),
        )
        .route(
            "/v1/settlement/mainnet/publish",
            post(refuse_mainnet_publish),
        )
        .layer(DefaultBodyLimit::max(1_048_576))
        .with_state(state))
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    external_writes_enabled: bool,
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "live",
        external_writes_enabled: false,
    })
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let is_ready = state.readiness.evaluate().is_ready();
    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = HealthResponse {
        status: if is_ready { "ready" } else { "not-ready" },
        external_writes_enabled: false,
    };
    (status, Json(body))
}

#[derive(Debug, Serialize)]
struct DependencyHealthResponse {
    status: &'static str,
    freshness: ReadinessFreshness,
    dependencies: DependencyReadiness,
    external_writes_enabled: bool,
}

async fn dependencies(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DependencyHealthResponse>, ApiError> {
    require_operator(&headers, &state.config.operator_role)?;
    let evaluation = state.readiness.evaluate();
    Ok(Json(DependencyHealthResponse {
        status: if evaluation.is_ready() {
            "READY"
        } else {
            "NOT_READY"
        },
        freshness: evaluation.freshness,
        dependencies: evaluation.dependencies,
        external_writes_enabled: false,
    }))
}

async fn capabilities(State(state): State<AppState>) -> Json<Value> {
    Json(state.capabilities)
}

#[derive(Debug, Serialize)]
struct ClaimValidationResponse {
    valid: bool,
    claim_id: String,
    claim_spec_digest: String,
}

async fn validate_claim(
    Json(claim): Json<ClaimSpecV1>,
) -> Result<Json<ClaimValidationResponse>, ApiError> {
    claim
        .verify_identity()
        .map_err(|_| ApiError::InvalidClaim)?;
    let digest = claim
        .claim_spec_digest()
        .map_err(|_| ApiError::InvalidClaim)?;
    Ok(Json(ClaimValidationResponse {
        valid: true,
        claim_id: claim.claim_id.to_string(),
        claim_spec_digest: digest.to_string(),
    }))
}

async fn run_demo(
    State(state): State<AppState>,
    Path(scenario): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ScenarioReport>, ApiError> {
    require_operator(&headers, &state.config.operator_role)?;
    let scenario = match scenario.as_str() {
        "true" => LocalScenario::True,
        "invalid-then-valid" => LocalScenario::InvalidThenValid,
        "disputed-false-counterexample-true" => LocalScenario::DisputedFalseCounterexampleTrue,
        "false" => LocalScenario::False,
        "runner-error" => LocalScenario::RunnerError,
        _ => return Err(ApiError::UnknownScenario),
    };
    let report = run_local_scenario(scenario, &state.config.artifact_root)
        .map_err(|_| ApiError::ScenarioFailed)?;
    Ok(Json(report))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MockSettlementPreflightRequest {
    binding: MarketBindingV1,
    final_resolution: ResolutionRecordV1,
    current_metadata_digest: Sha256Digest,
    capabilities: CapabilityMatrix,
}

#[derive(Debug, Serialize)]
struct MockSettlementPreflightResponse {
    backend: &'static str,
    network: SettlementNetwork,
    validation_scope: &'static str,
    external_writes_enabled: bool,
    preflight: SettlementPreflight,
}

#[derive(Debug, Serialize)]
struct MockSettlementWorkflowResponse {
    backend: &'static str,
    network: SettlementNetwork,
    operation: &'static str,
    report: ScenarioReport,
}

#[derive(Serialize)]
struct ResolutionIdentity<'a> {
    schema_version: &'static str,
    claim_id: &'a outcomes_domain::ClaimId,
    truth: outcomes_domain::ClaimTruth,
    finality: outcomes_domain::ResolutionFinality,
    evidence_bundle_digest: Sha256Digest,
    attestation_digests: &'a [Sha256Digest],
    dispute_window_start: &'a outcomes_domain::UtcTimestamp,
    dispute_window_end: &'a outcomes_domain::UtcTimestamp,
}

async fn run_mock_settlement_preflight(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MockSettlementPreflightResponse>, ApiError> {
    require_operator(&headers, &state.config.operator_role)?;
    let request: MockSettlementPreflightRequest =
        serde_json::from_slice(&body).map_err(|_| ApiError::InvalidSettlementPreflight)?;
    validate_mock_preflight_request(&request)?;
    let result = preflight(
        &request.binding,
        &request.final_resolution,
        request.current_metadata_digest,
        &request.capabilities,
    );
    Ok(Json(MockSettlementPreflightResponse {
        backend: "hyperliquid-hip4",
        network: SettlementNetwork::Mock,
        validation_scope: "DIGEST_AND_STATE_ONLY_NO_SIGNATURE_VERIFICATION",
        external_writes_enabled: false,
        preflight: result,
    }))
}

async fn run_mock_settlement_workflow(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MockSettlementWorkflowResponse>, ApiError> {
    require_operator(&headers, &state.config.operator_role)?;
    if !body.is_empty() {
        return Err(ApiError::UnexpectedRequestBody);
    }
    let artifact_root = state
        .config
        .artifact_root
        .join("operator-api")
        .join("mock-workflow");
    let report = run_local_scenario(LocalScenario::True, &artifact_root)
        .map_err(|_| ApiError::ScenarioFailed)?;
    Ok(Json(MockSettlementWorkflowResponse {
        backend: "hyperliquid-hip4",
        network: SettlementNetwork::Mock,
        operation: "EXECUTE_AND_RECONCILE_HERMETIC_MOCK",
        report,
    }))
}

async fn refuse_mainnet_publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    _body: Bytes,
) -> Result<Json<Value>, ApiError> {
    require_operator(&headers, &state.config.operator_role)?;
    Err(ApiError::MainnetMutationDisabled)
}

fn validate_mock_preflight_request(
    request: &MockSettlementPreflightRequest,
) -> Result<(), ApiError> {
    if request.binding.network != SettlementNetwork::Mock
        || request.capabilities.network != SettlementNetwork::Mock
        || request.binding.backend != "hyperliquid-hip4"
        || request.capabilities.backend != request.binding.backend
        || request.binding.schema_version != "hsai.market-binding.v1"
        || request.final_resolution.schema_version != "hsai.resolution-record.v1"
    {
        return Err(ApiError::InvalidSettlementPreflight);
    }

    let capability_digest =
        domain_separated_digest("hsai.backend-capability.v1", &request.capabilities)
            .map_err(|_| ApiError::InvalidSettlementPreflight)?;
    if capability_digest != request.binding.capability_snapshot_digest {
        return Err(ApiError::InvalidSettlementPreflight);
    }

    let binding_identity = (
        &request.binding.backend,
        request.binding.network,
        &request.binding.external_market_id,
        &request.binding.external_question_id,
        request.binding.external_metadata_digest,
        &request.binding.claim_id,
        request.binding.claim_spec_digest,
        request.binding.true_side_index,
        request.binding.false_side_index,
        request.binding.capability_snapshot_digest,
    );
    let binding_digest = domain_separated_digest("hsai.market-binding.v1", &binding_identity)
        .map_err(|_| ApiError::InvalidSettlementPreflight)?;
    if binding_digest != request.binding.binding_digest {
        return Err(ApiError::InvalidSettlementPreflight);
    }

    let resolution_identity = ResolutionIdentity {
        schema_version: "hsai.resolution-record.v1",
        claim_id: &request.final_resolution.claim_id,
        truth: request.final_resolution.truth,
        finality: request.final_resolution.finality,
        evidence_bundle_digest: request.final_resolution.evidence_bundle_digest,
        attestation_digests: &request.final_resolution.attestation_digests,
        dispute_window_start: &request.final_resolution.dispute_window_start,
        dispute_window_end: &request.final_resolution.dispute_window_end,
    };
    let resolution_digest =
        domain_separated_digest("hsai.resolution-record.v1", &resolution_identity)
            .map_err(|_| ApiError::InvalidSettlementPreflight)?;
    let expected_resolution_id = format!("resolution_{}", &resolution_digest.to_hex()[..32]);
    if resolution_digest != request.final_resolution.record_digest
        || expected_resolution_id != request.final_resolution.resolution_id
    {
        return Err(ApiError::InvalidSettlementPreflight);
    }
    Ok(())
}

fn require_operator(headers: &HeaderMap, expected_role: &str) -> Result<(), ApiError> {
    let role = headers
        .get("x-hsai-role")
        .and_then(|value| value.to_str().ok());
    if role == Some(expected_role) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("forbidden")]
    Forbidden,
    #[error("invalid claim")]
    InvalidClaim,
    #[error("unknown scenario")]
    UnknownScenario,
    #[error("scenario failed")]
    ScenarioFailed,
    #[error("invalid mock settlement preflight")]
    InvalidSettlementPreflight,
    #[error("request body is not accepted for this operation")]
    UnexpectedRequestBody,
    #[error("mainnet mutation is structurally disabled")]
    MainnetMutationDisabled,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "operator role required"),
            Self::InvalidClaim => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_CLAIM",
                "claim schema or identity is invalid",
            ),
            Self::UnknownScenario => (
                StatusCode::NOT_FOUND,
                "UNKNOWN_SCENARIO",
                "unknown local scenario",
            ),
            Self::ScenarioFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SCENARIO_FAILED",
                "hermetic scenario failed",
            ),
            Self::InvalidSettlementPreflight => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_MOCK_SETTLEMENT_PREFLIGHT",
                "mock preflight input failed schema, network, or digest validation",
            ),
            Self::UnexpectedRequestBody => (
                StatusCode::BAD_REQUEST,
                "UNEXPECTED_REQUEST_BODY",
                "this operation accepts no request body or signing material",
            ),
            Self::MainnetMutationDisabled => (
                StatusCode::CONFLICT,
                "MAINNET_MUTATION_DISABLED",
                "mainnet mutation is structurally disabled",
            ),
        };
        (
            status,
            Json(json!({
                "error_code": code,
                "message": message,
                "retryable": false
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt as _;

    use super::*;

    #[derive(Debug)]
    struct FixedSnapshotProvider(Result<ReadinessSnapshot, ReadinessProviderError>);

    impl ReadinessSnapshotProvider for FixedSnapshotProvider {
        fn snapshot(&self) -> Result<ReadinessSnapshot, ReadinessProviderError> {
            self.0
        }
    }

    #[derive(Debug)]
    struct FixedClock(Result<u64, ReadinessClockError>);

    impl ReadinessClock for FixedClock {
        fn now_unix_seconds(&self) -> Result<u64, ReadinessClockError> {
            self.0
        }
    }

    fn test_router(root: PathBuf) -> Result<Router, ApiBuildError> {
        router(ApiConfig {
            artifact_root: root,
            operator_role: "operator".to_owned(),
        })
    }

    fn test_router_with_readiness(
        root: PathBuf,
        snapshot: Result<ReadinessSnapshot, ReadinessProviderError>,
        now: Result<u64, ReadinessClockError>,
        max_age_seconds: u64,
    ) -> Result<Router, Box<dyn std::error::Error>> {
        let monitor = ReadinessMonitor::new(
            Arc::new(FixedSnapshotProvider(snapshot)),
            Arc::new(FixedClock(now)),
            max_age_seconds,
        )?;
        Ok(router_with_readiness(
            ApiConfig {
                artifact_root: root,
                operator_role: "operator".to_owned(),
            },
            monitor,
        )?)
    }

    #[tokio::test]
    async fn contract_public_health_exposes_no_write_capability()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let app = test_router(directory.path().to_path_buf())?;
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/health/live").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 16_384).await?;
        let value: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(value["external_writes_enabled"], false);
        assert_eq!(value["status"], "live");
        assert_eq!(value.as_object().map(serde_json::Map::len), Some(2));
        assert!(value.get("dependencies").is_none());
        let readiness = app
            .oneshot(
                Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(readiness.status(), StatusCode::SERVICE_UNAVAILABLE);
        Ok(())
    }

    #[tokio::test]
    async fn contract_readiness_is_200_only_for_fresh_all_ready_snapshot()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let snapshot = ReadinessSnapshot {
            observed_at_unix_seconds: 970,
            dependencies: DependencyReadiness::all(DependencyState::Ready),
        };
        let response = test_router_with_readiness(
            directory.path().to_path_buf(),
            Ok(snapshot),
            Ok(1_000),
            30,
        )?
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 16_384).await?;
        let body: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(body["status"], "ready");
        assert_eq!(body["external_writes_enabled"], false);
        assert_eq!(body.as_object().map(serde_json::Map::len), Some(2));
        Ok(())
    }

    #[tokio::test]
    async fn contract_readiness_fails_closed_for_every_non_ready_dependency()
    -> Result<(), Box<dyn std::error::Error>> {
        let states = [
            DependencyState::Degraded,
            DependencyState::Unavailable,
            DependencyState::Unknown,
        ];
        for state in states {
            let variants = [
                DependencyReadiness {
                    database_schema: state,
                    ..DependencyReadiness::all(DependencyState::Ready)
                },
                DependencyReadiness {
                    outbox: state,
                    ..DependencyReadiness::all(DependencyState::Ready)
                },
                DependencyReadiness {
                    evidence_store: state,
                    ..DependencyReadiness::all(DependencyState::Ready)
                },
                DependencyReadiness {
                    signer_registry: state,
                    ..DependencyReadiness::all(DependencyState::Ready)
                },
                DependencyReadiness {
                    required_sources: state,
                    ..DependencyReadiness::all(DependencyState::Ready)
                },
            ];
            for dependencies in variants {
                let directory = tempfile::tempdir()?;
                let app = test_router_with_readiness(
                    directory.path().to_path_buf(),
                    Ok(ReadinessSnapshot {
                        observed_at_unix_seconds: 1_000,
                        dependencies,
                    }),
                    Ok(1_000),
                    30,
                )?;
                let response = app
                    .oneshot(
                        Request::builder()
                            .uri("/health/ready")
                            .body(Body::empty())?,
                    )
                    .await?;
                assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn contract_readiness_freshness_and_provider_failures_are_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let all_ready = DependencyReadiness::all(DependencyState::Ready);
        for (snapshot, now) in [
            (
                Ok(ReadinessSnapshot {
                    observed_at_unix_seconds: 969,
                    dependencies: all_ready,
                }),
                Ok(1_000),
            ),
            (
                Ok(ReadinessSnapshot {
                    observed_at_unix_seconds: 1_001,
                    dependencies: all_ready,
                }),
                Ok(1_000),
            ),
            (Err(ReadinessProviderError), Ok(1_000)),
            (
                Ok(ReadinessSnapshot {
                    observed_at_unix_seconds: 1_000,
                    dependencies: all_ready,
                }),
                Err(ReadinessClockError),
            ),
        ] {
            let directory = tempfile::tempdir()?;
            let response =
                test_router_with_readiness(directory.path().to_path_buf(), snapshot, now, 30)?
                    .oneshot(
                        Request::builder()
                            .uri("/health/ready")
                            .body(Body::empty())?,
                    )
                    .await?;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            let bytes = to_bytes(response.into_body(), 16_384).await?;
            let text = String::from_utf8(bytes.to_vec())?;
            for prohibited in [
                "claim_SECRET",
                "candidate_SECRET",
                "wallet_SECRET",
                "signer_SECRET",
                "/private/path_SECRET",
                "raw_error_SECRET",
            ] {
                assert!(!text.contains(prohibited));
            }
        }
        assert_eq!(
            ReadinessMonitor::new(
                Arc::new(FixedSnapshotProvider(Err(ReadinessProviderError))),
                Arc::new(FixedClock(Ok(1_000))),
                0,
            )
            .err(),
            Some(ReadinessConfigError::InvalidMaxAge)
        );
        assert_eq!(
            ReadinessMonitor::new(
                Arc::new(FixedSnapshotProvider(Err(ReadinessProviderError))),
                Arc::new(FixedClock(Ok(1_000))),
                MAX_READINESS_AGE_SECONDS + 1,
            )
            .err(),
            Some(ReadinessConfigError::InvalidMaxAge)
        );
        Ok(())
    }

    #[tokio::test]
    async fn contract_dependency_detail_is_operator_gated_and_fixed_schema()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let app = test_router_with_readiness(
            directory.path().to_path_buf(),
            Ok(ReadinessSnapshot {
                observed_at_unix_seconds: 1_000,
                dependencies: DependencyReadiness::all(DependencyState::Ready),
            }),
            Ok(1_000),
            30,
        )?;
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health/dependencies")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let allowed = app
            .oneshot(
                Request::builder()
                    .uri("/health/dependencies")
                    .header("x-hsai-role", "operator")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(allowed.status(), StatusCode::OK);
        let bytes = to_bytes(allowed.into_body(), 16_384).await?;
        let body: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(body["status"], "READY");
        assert_eq!(body["freshness"], "FRESH");
        assert_eq!(body["external_writes_enabled"], false);
        let dependencies = body["dependencies"]
            .as_object()
            .ok_or("dependencies must be an object")?;
        assert_eq!(dependencies.len(), 5);
        for key in [
            "database_schema",
            "outbox",
            "evidence_store",
            "signer_registry",
            "required_sources",
        ] {
            assert_eq!(dependencies[key], "READY");
        }
        Ok(())
    }

    #[tokio::test]
    async fn contract_operator_endpoint_denies_public_reader_and_runs_for_operator()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let app = test_router(directory.path().to_path_buf())?;
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/demo/true")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        let allowed = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/demo/runner-error")
                    .header("x-hsai-role", "operator")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(allowed.status(), StatusCode::OK);
        let bytes = to_bytes(allowed.into_body(), 1_048_576).await?;
        let report: ScenarioReport = serde_json::from_slice(&bytes)?;
        assert_eq!(report.truth, outcomes_domain::ClaimTruth::Unresolved);
        Ok(())
    }

    #[tokio::test]
    async fn contract_invalid_then_valid_demo_reports_all_execution_outcomes()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let response = test_router(directory.path().to_path_buf())?
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/demo/invalid-then-valid")
                    .header("x-hsai-role", "operator")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 1_048_576).await?;
        let report: ScenarioReport = serde_json::from_slice(&bytes)?;
        assert_eq!(
            report.execution_outcomes,
            vec![
                outcomes_domain::ExecutionOutcome::Fail,
                outcomes_domain::ExecutionOutcome::Pass,
                outcomes_domain::ExecutionOutcome::Pass,
            ]
        );
        assert_eq!(report.truth, outcomes_domain::ClaimTruth::True);
        assert_eq!(report.mock_effect_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn contract_disputed_false_counterexample_demo_reports_superseding_true()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let response = test_router(directory.path().to_path_buf())?
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/demo/disputed-false-counterexample-true")
                    .header("x-hsai-role", "operator")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let report: ScenarioReport =
            serde_json::from_slice(&to_bytes(response.into_body(), 1_048_576).await?)?;
        assert_eq!(report.truth, outcomes_domain::ClaimTruth::True);
        assert_eq!(report.finality, outcomes_domain::ResolutionFinality::Final);
        assert_eq!(report.mock_effect_count, 1);
        assert_eq!(
            report.reconciliation_status,
            Some(outcomes_domain::ReconciliationStatus::Confirmed)
        );
        Ok(())
    }

    #[tokio::test]
    async fn contract_mock_preflight_is_operator_gated_digest_checked_and_effect_free()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let seed_root = directory.path().join("seed");
        let seed_report = run_local_scenario(LocalScenario::True, &seed_root)?;
        assert_eq!(seed_report.mock_effect_count, 1);
        let binding: MarketBindingV1 =
            serde_json::from_slice(&fs::read(seed_root.join("market-binding.json"))?)?;
        let final_resolution: ResolutionRecordV1 =
            serde_json::from_slice(&fs::read(seed_root.join("final-resolution.json"))?)?;
        let capabilities: CapabilityMatrix =
            serde_json::from_slice(&fs::read(seed_root.join("hip4-capabilities.json"))?)?;
        let body = serde_json::to_vec(&json!({
            "binding": binding,
            "final_resolution": final_resolution,
            "current_metadata_digest": binding.external_metadata_digest,
            "capabilities": capabilities,
        }))?;
        let app = test_router(directory.path().to_path_buf())?;

        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mock/preflight")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))?,
            )
            .await?;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let allowed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mock/preflight")
                    .header("x-hsai-role", "operator")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))?,
            )
            .await?;
        assert_eq!(allowed.status(), StatusCode::OK);
        let bytes = to_bytes(allowed.into_body(), 1_048_576).await?;
        let response: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(response["preflight"]["ready"], true);
        assert_eq!(response["external_writes_enabled"], false);
        assert_eq!(
            response["validation_scope"],
            "DIGEST_AND_STATE_ONLY_NO_SIGNATURE_VERIFICATION"
        );
        assert!(!directory.path().join("operator-api").exists());

        let mut signing_material: Value = serde_json::from_slice(&body)?;
        signing_material["private_key"] = json!("must-not-be-accepted");
        let signing_material_rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mock/preflight")
                    .header("x-hsai-role", "operator")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&signing_material)?))?,
            )
            .await?;
        assert_eq!(
            signing_material_rejected.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );

        let mut tampered: Value = serde_json::from_slice(&body)?;
        tampered["binding"]["external_market_id"] = json!("tampered");
        let rejected = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mock/preflight")
                    .header("x-hsai-role", "operator")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&tampered)?))?,
            )
            .await?;
        assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = to_bytes(rejected.into_body(), 16_384).await?;
        let error: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(error["error_code"], "INVALID_MOCK_SETTLEMENT_PREFLIGHT");
        assert_eq!(error["retryable"], false);
        Ok(())
    }

    #[tokio::test]
    async fn contract_mock_workflow_accepts_no_signing_input_and_remains_hermetic()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let app = test_router(directory.path().to_path_buf())?;
        let body_rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mock/workflow")
                    .header("x-hsai-role", "operator")
                    .body(Body::from(r#"{"private_key":"must-not-be-accepted"}"#))?,
            )
            .await?;
        assert_eq!(body_rejected.status(), StatusCode::BAD_REQUEST);

        let allowed = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mock/workflow")
                    .header("x-hsai-role", "operator")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(allowed.status(), StatusCode::OK);
        let bytes = to_bytes(allowed.into_body(), 1_048_576).await?;
        let response: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(response["network"], "MOCK");
        assert_eq!(response["report"]["mock_effect_count"], 1);
        assert_eq!(response["report"]["reconciliation_status"], "CONFIRMED");
        assert!(
            response["report"]["external_effect"]
                .as_str()
                .is_some_and(|value| value.contains("no testnet or mainnet write"))
        );
        Ok(())
    }

    #[tokio::test]
    async fn contract_mainnet_publish_refusal_is_operator_gated_and_unconditional()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let app = test_router(directory.path().to_path_buf())?;
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mainnet/publish")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let refused = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/settlement/mainnet/publish")
                    .header("x-hsai-role", "operator")
                    .body(Body::from(r#"{"private_key":"ignored-not-parsed"}"#))?,
            )
            .await?;
        assert_eq!(refused.status(), StatusCode::CONFLICT);
        let bytes = to_bytes(refused.into_body(), 16_384).await?;
        let error: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(error["error_code"], "MAINNET_MUTATION_DISABLED");
        assert_eq!(error["retryable"], false);
        assert!(!String::from_utf8(bytes.to_vec())?.contains("ignored-not-parsed"));
        Ok(())
    }

    #[test]
    fn contract_openapi_matches_implemented_routes() -> Result<(), Box<dyn std::error::Error>> {
        let document: Value = serde_json::from_str(OPENAPI_JSON)?;
        let paths = document["paths"]
            .as_object()
            .ok_or("OpenAPI paths must be an object")?;
        for path in [
            "/health/live",
            "/health/ready",
            "/health/dependencies",
            "/v1/adapters/capabilities",
            "/v1/claims/validate",
            "/v1/demo/{scenario}",
            "/v1/settlement/mock/preflight",
            "/v1/settlement/mock/workflow",
            "/v1/settlement/mainnet/publish",
        ] {
            assert!(paths.contains_key(path), "missing OpenAPI path {path}");
        }
        assert_eq!(paths.len(), 9);
        assert_eq!(
            paths["/health/ready"]["get"]["responses"]
                .as_object()
                .map(serde_json::Map::len),
            Some(2)
        );
        assert!(
            paths["/health/ready"]["get"]["responses"]
                .get("200")
                .is_some()
        );
        assert!(
            paths["/health/ready"]["get"]["responses"]
                .get("503")
                .is_some()
        );
        assert_eq!(
            paths["/health/dependencies"]["get"]["responses"]
                .as_object()
                .map(serde_json::Map::len),
            Some(2)
        );
        assert!(
            paths["/health/dependencies"]["get"]["responses"]
                .get("200")
                .is_some()
        );
        assert!(
            paths["/health/dependencies"]["get"]["responses"]
                .get("403")
                .is_some()
        );
        assert_eq!(
            paths["/health/dependencies"]["get"]["parameters"][0]["name"],
            "x-hsai-role"
        );
        for path in [
            "/v1/settlement/mock/preflight",
            "/v1/settlement/mock/workflow",
            "/v1/settlement/mainnet/publish",
        ] {
            assert!(paths[path].get("post").is_some(), "{path} must be POST");
            assert_eq!(
                paths[path]["post"]["parameters"][0]["name"], "x-hsai-role",
                "{path} must document the operator role boundary"
            );
        }
        Ok(())
    }
}
