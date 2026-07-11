//! Versioned public and protected operator API composition.

use std::{path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use outcomes_application::{LocalScenario, ScenarioReport, run_local_scenario};
use outcomes_domain::ClaimSpecV1;
use serde::Serialize;
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
    let state = AppState {
        config: Arc::new(config),
        capabilities: serde_json::from_str(CAPABILITIES)?,
    };
    Ok(Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/v1/adapters/capabilities", get(capabilities))
        .route("/v1/claims/validate", post(validate_claim))
        .route("/v1/demo/{scenario}", post(run_demo))
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

async fn ready() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ready-local",
        external_writes_enabled: false,
    })
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
        "false" => LocalScenario::False,
        "runner-error" => LocalScenario::RunnerError,
        _ => return Err(ApiError::UnknownScenario),
    };
    let report = run_local_scenario(scenario, &state.config.artifact_root)
        .map_err(|_| ApiError::ScenarioFailed)?;
    Ok(Json(report))
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            Self::InvalidClaim => (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_CLAIM"),
            Self::UnknownScenario => (StatusCode::NOT_FOUND, "UNKNOWN_SCENARIO"),
            Self::ScenarioFailed => (StatusCode::INTERNAL_SERVER_ERROR, "SCENARIO_FAILED"),
        };
        (status, Json(json!({"error_code": code}))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt as _;

    use super::*;

    fn test_router(root: PathBuf) -> Result<Router, ApiBuildError> {
        router(ApiConfig {
            artifact_root: root,
            operator_role: "operator".to_owned(),
        })
    }

    #[tokio::test]
    async fn contract_public_health_exposes_no_write_capability()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let response = test_router(directory.path().to_path_buf())?
            .oneshot(Request::builder().uri("/health/live").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 16_384).await?;
        let value: Value = serde_json::from_slice(&bytes)?;
        assert_eq!(value["external_writes_enabled"], false);
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

    #[test]
    fn contract_openapi_matches_implemented_routes() -> Result<(), Box<dyn std::error::Error>> {
        let document: Value = serde_json::from_str(OPENAPI_JSON)?;
        let paths = document["paths"]
            .as_object()
            .ok_or("OpenAPI paths must be an object")?;
        for path in [
            "/health/live",
            "/health/ready",
            "/v1/adapters/capabilities",
            "/v1/claims/validate",
            "/v1/demo/{scenario}",
        ] {
            assert!(paths.contains_key(path), "missing OpenAPI path {path}");
        }
        assert_eq!(paths.len(), 5);
        Ok(())
    }
}
