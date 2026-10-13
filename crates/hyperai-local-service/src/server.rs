//! Authenticated loopback-only HTTP host for the `HyperAI` application facade.

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    auth::AuthState,
    lifecycle::{self, ServiceLock},
    market_history::{MarketHistoryError, MarketHistoryStore},
    route_policy::{AgentToolRoute, AgentToolRouteMethod},
    storage::{self, HomePaths},
};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{DefaultBodyLimit, FromRequest, Path as AxumPath, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use data_encoding::BASE32_NOPAD;
use hyperai_acp::{
    AgentMcpConfigV1, AgentProfileV1, AgentSessionManager, SessionError as AcpSessionError,
    SessionSnapshotV1, sibling_mcp_executable,
};
use hyperai_application::{
    API_ERROR_MESSAGE, API_ERROR_SCHEMA_VERSION, DataModelCatalogV1, DataModelV1,
    DataViewCatalogV1, DataViewV1, EvalCatalogV1, EvalError, EvalRunReportV1, EvalRunRequestV1,
    EvalScorerOutputV1, EvalScorerRequestV1, FleetDoctorV1, FleetError, FleetInventoryV1,
    FleetOperationPlanV1, FleetPlanRequestV1, FleetSimulationSnapshotV1, FleetSimulationStoreV1,
    FleetSimulationTransitionRequestV1, FleetSimulationTransitionV1, HyperIntelBundleError,
    HyperIntelBundleRequestV1, HyperIntelBundleVerificationV1, HyperIntelReconciliationError,
    HyperIntelReconciliationRequestV1, HyperIntelReconciliationV1, InterfaceError,
    ListingsCaptureError, LocalMarketDemoV1, LocalMarketQuoteApprovalRequestV1,
    LocalMarketQuoteApprovalV1, LocalMarketQuoteCompletionRequestV1, LocalMarketQuoteCompletionV1,
    LocalMarketQuotePreviewV1, LocalMarketQuoteScenarioV1, LocalMarketVerificationV1,
    QueryRequestV1, SimpleMarketCaptureRequestV1, SupplyQuotePreviewV1, TradeCatalogV1, TradeError,
    TradePaperReplayReportV1, TradePaperReplayRequestV1, TradeScenarioReportV1,
    TradeScenarioRequestV1, TradeSeriesReportV1, WorkloadTraceDataStatusPreviewV1, WorkloadTraceV1,
    approve_local_market_quote_v1, complete_local_market_quote_v1, data_catalog_v1, data_models_v1,
    data_views_v1, describe_data_v1, eval_catalog_v1, eval_run_report_v1,
    eval_run_request_digest_v1, fleet_doctor_v1, fleet_inventory_v1, fleet_simulation_snapshot_v1,
    import_gpudiff_data_snapshot_v1, interface_status, market_supply_comparison_v1,
    market_supply_quote_preview_v1, preview_data_model_v1, preview_data_synthetic_seed_v1,
    preview_data_view_v1, preview_fleet_operation_v1, preview_local_market_quote_scenario_v1,
    preview_local_market_quote_v1, query_data_v1, reconcile_hyperintel_execution_v1,
    run_local_market_demo, save_data_model_v1, save_data_view_v1, seed_data_synthetic_confirmed_v1,
    synthetic_data_release_v1, trade_catalog_v1, trade_paper_replay_v1, trade_scenario_v1,
    trade_series_v1, transition_fleet_simulation_v1, validate_eval_run_request_v1,
    validate_scorer_output_v1, verify_hyperintel_market_bundle_v1, verify_local_market_receipt_v1,
    verify_simple_market_capture_v1, workload_trace_data_status_preview_v1,
    workload_trace_duplicate_disposition_preview_v1, workload_trace_inconclusive_preview_v1,
    workload_trace_seller_failure_preview_v1, workload_trace_v1,
};
use hyperai_data::{
    DataError, DatasetManifestV1, DefinitionPreviewV1, GPUDIFF_SOURCE_URL_V1, GpudiffImportV1,
    QueryResultV1,
};
use hyperai_market::{
    LocalMarketHistoryEntryV1, LocalMarketReceiptV1, MarketError, SupplyComparisonV1,
    SupplyQuotePreviewRequestV1, local_market_history_page_v1, verify_local_market_history_page_v1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

const INDEX_HTML: &str = include_str!("../../../apps/hyperai-interface/index.html");
const APP_CSS: &str = include_str!("../../../apps/hyperai-interface/app.css");
const WORKLOAD_TRACE_JS: &str = include_str!("../../../apps/hyperai-interface/workload-trace.js");
const CONNECTION_INVENTORY_JS: &str =
    include_str!("../../../apps/hyperai-interface/connection-inventory.js");
const MARKET_SUPPLY_COMPARISON_JS: &str =
    include_str!("../../../apps/hyperai-interface/market-supply-comparison.js");
const APP_JS: &str = include_str!("../../../apps/hyperai-interface/app.js");
const ERROR_ENVELOPE_JS: &str = include_str!("../../../apps/hyperai-interface/error-envelope.js");
const MAX_JSON_BODY_BYTES: usize = 1_048_576;
const MAX_GPUDIFF_BODY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ACTIVE_DATA_QUERIES: usize = 2;
const MAX_EVAL_RUNS: usize = 32;
const MAX_EVAL_REQUEST_BYTES: usize = 4 * 1024;
const MAX_TRADE_REQUEST_BYTES: usize = 4 * 1024;
const EVAL_SCORER_TIMEOUT: Duration = Duration::from_secs(1);
const GPUDIFF_REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'self'; ",
    "img-src 'self' data:; ",
    "style-src 'self'; ",
    "script-src 'self'; ",
    "connect-src 'self'; ",
    "object-src 'none'; ",
    "base-uri 'none'; ",
    "frame-ancestors 'none'; ",
    "form-action 'none'"
);
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct ServiceState {
    auth: AuthState,
    shutdown: tokio::sync::watch::Sender<bool>,
    instance_id: String,
    port: u16,
    data_root: PathBuf,
    market_history: MarketHistoryStore,
    active_queries: Arc<Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
    query_slots: Arc<tokio::sync::Semaphore>,
    fleet_simulation: FleetSimulationStoreV1,
    eval_runs:
        tokio::sync::Mutex<HashMap<String, (hyperai_foundation::Sha256Digest, EvalRunReportV1)>>,
    agent_sessions: AgentSessionManager,
    agent_lifecycle_lock: tokio::sync::Mutex<()>,
    gpudiff_schedule_status: Arc<tokio::sync::Mutex<GpudiffScheduleStatusV1>>,
    gpudiff_schedule_task: tokio::sync::Mutex<Option<GpudiffScheduleTask>>,
    #[cfg(test)]
    test_query_gate: Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Semaphore>)>,
}

#[derive(Clone, Serialize)]
struct GpudiffScheduleStatusV1 {
    schema_version: &'static str,
    state_slice: &'static str,
    state: String,
    interval_seconds: u64,
    last_attempt_unix_seconds: Option<u64>,
    last_success_unix_seconds: Option<u64>,
    last_raw_source_digest: Option<String>,
    last_row_count: Option<u64>,
    last_observed_at_min: Option<String>,
    last_observed_at_max: Option<String>,
    last_error_code: Option<String>,
}

impl Default for GpudiffScheduleStatusV1 {
    fn default() -> Self {
        Self {
            schema_version: "hyperai.data.gpudiff_refresh_schedule.v1",
            state_slice: "hyperai.bazaar.gpudiff_owner_scheduled_refresh.v1",
            state: "stopped".to_owned(),
            interval_seconds: GPUDIFF_REFRESH_INTERVAL.as_secs(),
            last_attempt_unix_seconds: None,
            last_success_unix_seconds: None,
            last_raw_source_digest: None,
            last_row_count: None,
            last_observed_at_min: None,
            last_observed_at_max: None,
            last_error_code: None,
        }
    }
}

struct GpudiffScheduleTask {
    cancel: tokio::sync::watch::Sender<bool>,
    join: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataSeedRequestV1 {
    preview_digest: hyperai_foundation::Sha256Digest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataQueryEnvelopeV1 {
    query_id: String,
    request: QueryRequestV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentConnectRequestV1 {
    profile: AgentProfileV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentPromptRequestV1 {
    prompt: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataModelSaveRequestV1 {
    model: DataModelV1,
    preview_digest: hyperai_foundation::Sha256Digest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataViewSaveRequestV1 {
    view: DataViewV1,
    preview_digest: hyperai_foundation::Sha256Digest,
}

struct ActiveQueryRegistration {
    queries: Arc<Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
    query_id: String,
}

impl Drop for ActiveQueryRegistration {
    fn drop(&mut self) {
        if let Ok(mut queries) = self.queries.lock() {
            queries.remove(&self.query_id);
        }
    }
}

struct RuntimeCleanup(HomePaths);

impl Drop for RuntimeCleanup {
    fn drop(&mut self) {
        let token = self.0.runtime.join("bearer.token");
        let _ = AuthState::remove_bearer_file(&token);
        let _ = lifecycle::remove_record(&self.0);
    }
}

/// Starts the local service on IPv4 loopback and holds exclusive home ownership.
///
/// # Errors
///
/// Returns an error if the local home, process lock, capability, socket, or
/// server lifecycle cannot be initialized.
pub async fn serve(
    home: Option<&Path>,
    requested_port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let lock_paths = storage::prepare_service_runtime(home)?;
    let _lock = ServiceLock::acquire(&lock_paths)?;
    let (paths, _) = storage::initialize_home(home).await?;
    let market_history = MarketHistoryStore::open(&paths).await?;
    let auth = AuthState::generate()?;
    let token_path = paths.runtime.join("bearer.token");
    auth.write_bearer_file(&token_path)?;
    let _cleanup = RuntimeCleanup(paths.clone());

    let address = SocketAddr::from(([127, 0, 0, 1], requested_port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    let actual_address = listener.local_addr()?;
    let record = lifecycle::new_record(actual_address.to_string())?;
    lifecycle::write_record(&paths, &record)?;
    let (shutdown, mut shutdown_signal) = tokio::sync::watch::channel(false);
    let state = Arc::new(ServiceState {
        auth,
        shutdown,
        instance_id: record.instance_id.clone(),
        port: actual_address.port(),
        data_root: paths.root.join("data"),
        market_history,
        active_queries: Arc::new(Mutex::new(HashMap::new())),
        query_slots: Arc::new(tokio::sync::Semaphore::new(MAX_ACTIVE_DATA_QUERIES)),
        fleet_simulation: FleetSimulationStoreV1::default(),
        eval_runs: tokio::sync::Mutex::new(HashMap::new()),
        agent_sessions: AgentSessionManager::default(),
        agent_lifecycle_lock: tokio::sync::Mutex::new(()),
        gpudiff_schedule_status: Arc::new(tokio::sync::Mutex::new(
            GpudiffScheduleStatusV1::default(),
        )),
        gpudiff_schedule_task: tokio::sync::Mutex::new(None),
        #[cfg(test)]
        test_query_gate: None,
    });
    eprintln!("HyperAI local interface: http://{actual_address}");
    eprintln!(
        "Mode: local; public GPUDiff refresh remains owner-triggered, including its optional six-hour schedule; no provider execution or funds movement."
    );
    eprintln!("Local client capability: {}", token_path.display());
    let shutdown_state = Arc::clone(&state);
    axum::serve(listener, app(state))
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = shutdown_signal.changed() => {},
                _ = tokio::signal::ctrl_c() => {},
            }
        })
        .await?;
    stop_gpudiff_schedule(&shutdown_state).await;
    let _ = shutdown_state.agent_sessions.disconnect().await;
    let _ = shutdown_state.auth.revoke_agent_tool_capability();
    Ok(())
}

fn workload_trace_routes() -> Router<Arc<ServiceState>> {
    Router::new()
        .route("/api/v1/market/workload-trace", get(market_workload_trace))
        .route(
            "/api/v1/market/supply-comparison",
            get(market_supply_comparison),
        )
        .route(
            "/api/v1/market/workload-trace/failure-preview",
            get(market_workload_trace_failure_preview),
        )
        .route(
            "/api/v1/market/workload-trace/inconclusive-preview",
            get(market_workload_trace_inconclusive_preview),
        )
        .route(
            "/api/v1/market/workload-trace/data-status-preview/{status}",
            get(market_workload_trace_data_status_preview),
        )
        .route(
            "/api/v1/market/workload-trace/duplicate-disposition-preview",
            get(market_workload_trace_duplicate_disposition_preview),
        )
}

macro_rules! register_agent_tool_route {
    ($router:expr, $route:expr, $handler:ident) => {{
        let route = $route;
        match route.method() {
            AgentToolRouteMethod::Get => $router.route(route.path(), get($handler)),
            AgentToolRouteMethod::Post => $router.route(route.path(), post($handler)),
        }
    }};
}

fn agent_tool_routes() -> Router<Arc<ServiceState>> {
    AgentToolRoute::ALL
        .into_iter()
        .fold(Router::new(), |router, route| match route {
            AgentToolRoute::DataCatalog => {
                register_agent_tool_route!(router, route, data_catalog)
            }
            AgentToolRoute::DataQuery => register_agent_tool_route!(router, route, data_query),
            AgentToolRoute::FleetInventory => {
                register_agent_tool_route!(router, route, fleet_inventory)
            }
            AgentToolRoute::FleetDoctor => register_agent_tool_route!(router, route, fleet_doctor),
            AgentToolRoute::FleetOperationPreview => {
                register_agent_tool_route!(router, route, fleet_operation_preview)
            }
            AgentToolRoute::EvalCatalog => register_agent_tool_route!(router, route, eval_catalog),
            AgentToolRoute::EvalReports => {
                register_agent_tool_route!(router, route, agent_eval_reports)
            }
            AgentToolRoute::WorkloadTrace => {
                register_agent_tool_route!(router, route, market_workload_trace)
            }
            AgentToolRoute::WorkloadTraceFailurePreview => {
                register_agent_tool_route!(router, route, market_workload_trace_failure_preview)
            }
            AgentToolRoute::WorkloadTraceInconclusivePreview => {
                register_agent_tool_route!(
                    router,
                    route,
                    market_workload_trace_inconclusive_preview
                )
            }
            AgentToolRoute::MarketSupplyComparison => {
                register_agent_tool_route!(router, route, market_supply_comparison)
            }
            AgentToolRoute::MarketSupplyQuotePreview => {
                register_agent_tool_route!(router, route, market_supply_quote_preview)
            }
            AgentToolRoute::TradeCatalog => {
                register_agent_tool_route!(router, route, trade_catalog)
            }
            AgentToolRoute::TradeSeries => register_agent_tool_route!(router, route, trade_series),
            AgentToolRoute::TradeScenario => {
                register_agent_tool_route!(router, route, trade_scenario)
            }
        })
}

#[allow(clippy::too_many_lines)]
fn app(state: Arc<ServiceState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.css", get(stylesheet))
        .route("/error-envelope.js", get(error_envelope_script))
        .route("/workload-trace.js", get(workload_trace_script))
        .route("/connection-inventory.js", get(connection_inventory_script))
        .route(
            "/market-supply-comparison.js",
            get(market_supply_comparison_script),
        )
        .route("/app.js", get(script))
        .route("/healthz", get(healthz))
        .route("/api/v1/session", get(session_bootstrap))
        .route("/api/v1/status", get(status))
        .route("/api/v1/connections", get(connections))
        .route("/api/v1/service/stop", post(stop_service))
        .route("/api/v1/agent/connect", post(agent_connect))
        .route("/api/v1/agent/status", get(agent_status))
        .route("/api/v1/agent/prompt", post(agent_prompt))
        .route("/api/v1/agent/cancel", post(agent_cancel))
        .route("/api/v1/agent/disconnect", post(agent_disconnect))
        .route("/api/v1/market/demo", post(market_demo))
        .merge(workload_trace_routes())
        .route("/api/v1/market/quote-preview", post(market_quote_preview))
        .route(
            "/api/v1/market/quote-preview/scenario/{scenario_id}",
            post(market_quote_preview_scenario),
        )
        .route(
            "/api/v1/market/supply-quote-preview",
            post(market_supply_quote_preview),
        )
        .route("/api/v1/market/quote-approve", post(market_quote_approve))
        .route("/api/v1/market/quote-complete", post(market_quote_complete))
        .route("/api/v1/market/deal-history", get(market_deal_history))
        .route("/api/v1/market/verify", post(market_verify))
        .route(
            "/api/v1/market/registry-capture/verify",
            post(simple_market_capture_verify),
        )
        .route(
            "/api/v1/inference/verify-bundle",
            post(hyperintel_verify_bundle),
        )
        .route(
            "/api/v1/inference/reconcile-execution",
            post(hyperintel_reconcile_execution),
        )
        .route("/api/v1/fleet/inventory", get(fleet_inventory))
        .route("/api/v1/fleet/doctor", get(fleet_doctor))
        .route(
            "/api/v1/fleet/operation-preview",
            post(fleet_operation_preview),
        )
        .route("/api/v1/fleet/simulation", get(fleet_simulation_snapshot))
        .route(
            "/api/v1/fleet/simulation/transition",
            post(fleet_simulation_transition),
        )
        .route("/api/v1/eval/catalog", get(eval_catalog))
        .route("/api/v1/eval/run", post(eval_run))
        .route("/api/v1/trade/catalog", get(trade_catalog))
        .route("/api/v1/trade/series/{instrument_id}", get(trade_series))
        .route("/api/v1/trade/scenario", post(trade_scenario))
        .route("/api/v1/trade/paper", post(trade_paper))
        .route("/api/v1/data/catalog", get(data_catalog))
        .route("/api/v1/data/models", get(data_models))
        .route("/api/v1/data/views", get(data_views))
        .route("/api/v1/data/describe/{dataset_id}", get(data_describe))
        .route(
            "/api/v1/data/synthetic-release",
            get(data_synthetic_release),
        )
        .route("/api/v1/data/synthetic/preview", get(data_seed_preview))
        .route("/api/v1/data/synthetic/seed", post(data_seed))
        .route("/api/v1/data/gpudiff/refresh", post(data_gpudiff_refresh))
        .route(
            "/api/v1/data/gpudiff/schedule",
            get(data_gpudiff_schedule_status).post(data_gpudiff_schedule_start),
        )
        .route(
            "/api/v1/data/gpudiff/schedule/stop",
            post(data_gpudiff_schedule_stop),
        )
        .route("/api/v1/data/query", post(data_query))
        .merge(agent_tool_routes())
        .route(
            "/api/v1/data/query/{query_id}/cancel",
            post(data_query_cancel),
        )
        .route("/api/v1/data/models/preview", post(data_model_preview))
        .route("/api/v1/data/models/save", post(data_model_save))
        .route("/api/v1/data/views/preview", post(data_view_preview))
        .route("/api/v1/data/views/save", post(data_view_save))
        .with_state(state.clone())
        .layer(DefaultBodyLimit::max(MAX_JSON_BODY_BYTES))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authentication,
        ))
        .layer(middleware::from_fn_with_state(state, request_security))
}

async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(INDEX_HTML),
    )
}

async fn stylesheet() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], APP_CSS)
}

async fn error_envelope_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        ERROR_ENVELOPE_JS,
    )
}

async fn workload_trace_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        WORKLOAD_TRACE_JS,
    )
}

async fn connection_inventory_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        CONNECTION_INVENTORY_JS,
    )
}

async fn market_supply_comparison_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        MARKET_SUPPLY_COMPARISON_JS,
    )
}

async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        APP_JS,
    )
}

async fn status() -> Json<hyperai_application::InterfaceStatusV1> {
    Json(interface_status())
}

async fn connections() -> Json<hyperai_application::ConnectionInventoryV1> {
    Json(hyperai_application::interface_connection_inventory_v1())
}

async fn fleet_inventory() -> Result<Json<FleetInventoryV1>, ApiError> {
    fleet_inventory_v1().map(Json).map_err(ApiError::from)
}

async fn fleet_simulation_snapshot(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<FleetSimulationSnapshotV1>, ApiError> {
    fleet_simulation_snapshot_v1(&state.fleet_simulation)
        .map(Json)
        .map_err(ApiError::from)
}

async fn fleet_simulation_transition(
    State(state): State<Arc<ServiceState>>,
    ApiJson(request): ApiJson<FleetSimulationTransitionRequestV1>,
) -> Result<Json<FleetSimulationTransitionV1>, ApiError> {
    transition_fleet_simulation_v1(&state.fleet_simulation, &request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn eval_catalog() -> Result<Json<EvalCatalogV1>, ApiError> {
    eval_catalog_v1().map(Json).map_err(ApiError::from)
}

async fn trade_catalog() -> Result<Json<TradeCatalogV1>, ApiError> {
    trade_catalog_v1().map(Json).map_err(ApiError::from)
}

async fn trade_series(
    AxumPath(instrument_id): AxumPath<String>,
) -> Result<Json<TradeSeriesReportV1>, ApiError> {
    trade_series_v1(&instrument_id)
        .map(Json)
        .map_err(ApiError::from)
}

async fn trade_scenario(
    LimitedTradeJson(request): LimitedTradeJson<TradeScenarioRequestV1>,
) -> Result<Json<TradeScenarioReportV1>, ApiError> {
    trade_scenario_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn trade_paper(
    LimitedTradeJson(request): LimitedTradeJson<TradePaperReplayRequestV1>,
) -> Result<Json<TradePaperReplayReportV1>, ApiError> {
    trade_paper_replay_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn eval_run(
    State(state): State<Arc<ServiceState>>,
    LimitedApiJson(request): LimitedApiJson<EvalRunRequestV1>,
) -> Result<Json<EvalRunReportV1>, ApiError> {
    let request_bytes = serde_json::to_vec(&request).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "EVAL_REQUEST_ENCODING_FAILED",
    })?;
    if request_bytes.len() > MAX_EVAL_REQUEST_BYTES {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "EVAL_REQUEST_TOO_LARGE",
        });
    }
    let request_digest = eval_run_request_digest_v1(&request)
        .map_err(|error| ApiError::from(InterfaceError::Eval(error)))?;
    let mut runs = state.eval_runs.lock().await;
    if let Some((existing_digest, report)) = runs.get(&request.idempotency_key) {
        return if *existing_digest == request_digest {
            Ok(Json(report.clone()))
        } else {
            Err(ApiError {
                status: StatusCode::CONFLICT,
                code: "EVAL_IDEMPOTENCY_CONFLICT",
            })
        };
    }
    validate_eval_run_request_v1(&request)
        .map_err(|error| ApiError::from(InterfaceError::Eval(error)))?;
    if runs.len() >= MAX_EVAL_RUNS {
        return Err(ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "EVAL_CACHE_FULL",
        });
    }

    let scorer_request = EvalScorerRequestV1 {
        expected_suite_digest: request.expected_suite_digest,
        pair_id: request.pair_id.clone(),
        seed_label: hyperai_application::EVAL_SEED_LABEL_V1.to_owned(),
    };
    let scorer_input = serde_json::to_vec(&scorer_request).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "EVAL_SCORER_INPUT_ENCODING_FAILED",
    })?;
    let mut expected_output: Option<EvalScorerOutputV1> = None;
    let mut child_wall_time_ms = Vec::with_capacity(hyperai_application::EVAL_REPEAT_COUNT_V1);
    let mut scorer_output_bytes = 0;
    for _ in 0..hyperai_application::EVAL_REPEAT_COUNT_V1 {
        let (output, elapsed_ms, output_bytes) = invoke_eval_scorer(
            &scorer_input,
            request.expected_suite_digest,
            &request.pair_id,
        )
        .await?;
        if expected_output
            .as_ref()
            .is_some_and(|expected| expected != &output)
        {
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "EVAL_SCORER_REPEAT_MISMATCH",
            });
        }
        expected_output = Some(output);
        child_wall_time_ms.push(elapsed_ms);
        scorer_output_bytes = output_bytes;
    }
    let report = eval_run_report_v1(
        expected_output.ok_or(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "EVAL_SCORER_NO_OUTPUT",
        })?,
        child_wall_time_ms,
        scorer_input.len(),
        scorer_output_bytes,
    )
    .map_err(|error| ApiError::from(InterfaceError::Eval(error)))?;
    runs.insert(request.idempotency_key, (request_digest, report.clone()));
    Ok(Json(report))
}

async fn invoke_eval_scorer(
    input: &[u8],
    expected_suite_digest: hyperai_foundation::Sha256Digest,
    expected_pair_id: &str,
) -> Result<(EvalScorerOutputV1, u64, usize), ApiError> {
    let executable = std::env::current_exe().map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "EVAL_SCORER_UNAVAILABLE",
    })?;
    let mut command = Command::new(executable);
    command.arg("__internal_eval_scorer").env_clear();
    invoke_eval_scorer_with_command(
        command,
        input,
        expected_suite_digest,
        expected_pair_id,
        EVAL_SCORER_TIMEOUT,
    )
    .await
}

async fn invoke_eval_scorer_with_command(
    mut command: Command,
    input: &[u8],
    expected_suite_digest: hyperai_foundation::Sha256Digest,
    expected_pair_id: &str,
    timeout: Duration,
) -> Result<(EvalScorerOutputV1, u64, usize), ApiError> {
    if input.len() > hyperai_application::MAX_SCORER_INPUT_BYTES_V1 {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "EVAL_SCORER_INPUT_TOO_LARGE",
        });
    }
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "EVAL_SCORER_UNAVAILABLE",
        })?;
    let started = Instant::now();
    let run = tokio::time::timeout(timeout, async {
        read_eval_scorer_output(&mut child, input, expected_suite_digest, expected_pair_id).await
    })
    .await;
    let result = match run {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            terminate_eval_scorer(&mut child).await;
            return Err(error);
        }
        Err(_) => {
            terminate_eval_scorer(&mut child).await;
            return Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "EVAL_SCORER_TIMEOUT",
            });
        }
    };
    Ok((
        result.0,
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        result.1,
    ))
}

async fn read_eval_scorer_output(
    child: &mut tokio::process::Child,
    input: &[u8],
    expected_suite_digest: hyperai_foundation::Sha256Digest,
    expected_pair_id: &str,
) -> Result<(EvalScorerOutputV1, usize), ApiError> {
    let mut stdin = child.stdin.take().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "EVAL_SCORER_PROTOCOL_FAILED",
    })?;
    stdin.write_all(input).await.map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "EVAL_SCORER_PROTOCOL_FAILED",
    })?;
    drop(stdin);

    let stdout = child.stdout.take().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "EVAL_SCORER_PROTOCOL_FAILED",
    })?;
    let mut bounded_stdout = stdout.take(
        u64::try_from(hyperai_application::MAX_SCORER_OUTPUT_BYTES_V1 + 1).unwrap_or(u64::MAX),
    );
    let mut output_bytes = Vec::with_capacity(1024);
    bounded_stdout
        .read_to_end(&mut output_bytes)
        .await
        .map_err(|_| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "EVAL_SCORER_PROTOCOL_FAILED",
        })?;
    if output_bytes.len() > hyperai_application::MAX_SCORER_OUTPUT_BYTES_V1 {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "EVAL_SCORER_OUTPUT_TOO_LARGE",
        });
    }

    let status = child.wait().await.map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "EVAL_SCORER_PROCESS_FAILED",
    })?;
    if !status.success() {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "EVAL_SCORER_PROCESS_FAILED",
        });
    }
    let output: EvalScorerOutputV1 =
        serde_json::from_slice(&output_bytes).map_err(|_| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "EVAL_SCORER_INVALID_OUTPUT",
        })?;
    let encoded = serde_json::to_vec(&output).map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "EVAL_SCORER_INVALID_OUTPUT",
    })?;
    if encoded != output_bytes
        || validate_scorer_output_v1(&output, expected_suite_digest, expected_pair_id).is_err()
    {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "EVAL_SCORER_INVALID_OUTPUT",
        });
    }
    Ok((output, output_bytes.len()))
}

async fn terminate_eval_scorer(child: &mut tokio::process::Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[cfg(test)]
mod eval_fault_tests {
    use std::{
        error::Error,
        io::{Read, Write},
        process::Stdio,
        time::Duration,
    };

    use axum::http::StatusCode;
    use hyperai_application::{self, EvalScorerRequestV1};
    use tokio::process::Command;

    use super::{EVAL_SCORER_TIMEOUT, invoke_eval_scorer_with_command};

    const CHILD_MODE_ENV: &str = "HYPERAI_EVAL_FAULT_TEST_CHILD";

    fn fixed_scorer_input() -> Result<(Vec<u8>, hyperai_foundation::Sha256Digest), Box<dyn Error>> {
        let catalog = hyperai_application::eval_catalog_v1()?;
        let request = EvalScorerRequestV1 {
            expected_suite_digest: catalog.suite_digest,
            pair_id: hyperai_application::EVAL_PAIR_ID_V1.to_owned(),
            seed_label: hyperai_application::EVAL_SEED_LABEL_V1.to_owned(),
        };
        Ok((serde_json::to_vec(&request)?, catalog.suite_digest))
    }

    fn fault_child(mode: &str) -> Result<Command, Box<dyn Error>> {
        let executable = std::env::current_exe()?;
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "server::eval_fault_tests::child_fixture",
                "--nocapture",
            ])
            .env(CHILD_MODE_ENV, mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        Ok(command)
    }

    #[tokio::test]
    async fn scorer_timeout_kills_the_child_and_returns_a_fixed_error() -> Result<(), Box<dyn Error>>
    {
        let (input, digest) = fixed_scorer_input()?;
        let result = invoke_eval_scorer_with_command(
            fault_child("sleep")?,
            &input,
            digest,
            hyperai_application::EVAL_PAIR_ID_V1,
            Duration::from_millis(50),
        )
        .await;
        let Err(error) = result else {
            return Err("sleeping scorer child unexpectedly completed".into());
        };
        assert_eq!(error.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code, "EVAL_SCORER_TIMEOUT");
        assert!(EVAL_SCORER_TIMEOUT > Duration::from_millis(50));
        Ok(())
    }

    #[tokio::test]
    async fn scorer_output_limit_kills_the_child_and_returns_a_fixed_error()
    -> Result<(), Box<dyn Error>> {
        let (input, digest) = fixed_scorer_input()?;
        let result = invoke_eval_scorer_with_command(
            fault_child("overflow")?,
            &input,
            digest,
            hyperai_application::EVAL_PAIR_ID_V1,
            EVAL_SCORER_TIMEOUT,
        )
        .await;
        let Err(error) = result else {
            return Err("overflowing scorer output unexpectedly completed".into());
        };
        assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error.code, "EVAL_SCORER_OUTPUT_TOO_LARGE");
        Ok(())
    }

    #[tokio::test]
    async fn scorer_nonzero_exit_is_rejected() -> Result<(), Box<dyn Error>> {
        let (input, digest) = fixed_scorer_input()?;
        let result = invoke_eval_scorer_with_command(
            fault_child("exit")?,
            &input,
            digest,
            hyperai_application::EVAL_PAIR_ID_V1,
            EVAL_SCORER_TIMEOUT,
        )
        .await;
        let Err(error) = result else {
            return Err("failed scorer child unexpectedly completed".into());
        };
        assert_eq!(error.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code, "EVAL_SCORER_PROCESS_FAILED");
        Ok(())
    }

    #[test]
    fn child_fixture() -> Result<(), Box<dyn Error>> {
        let Ok(mode) = std::env::var(CHILD_MODE_ENV) else {
            return Ok(());
        };
        let mut request = Vec::new();
        std::io::stdin().read_to_end(&mut request)?;
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(30)),
            "overflow" => {
                let bytes = vec![b'x'; hyperai_application::MAX_SCORER_OUTPUT_BYTES_V1 + 2048];
                let _ = std::io::stdout().lock().write_all(&bytes);
            }
            "exit" => std::process::exit(23),
            _ => return Err("unknown test child mode".into()),
        }
        Ok(())
    }
}

async fn fleet_doctor() -> Json<FleetDoctorV1> {
    Json(fleet_doctor_v1())
}

async fn fleet_operation_preview(
    ApiJson(request): ApiJson<FleetPlanRequestV1>,
) -> Result<Json<FleetOperationPlanV1>, ApiError> {
    preview_fleet_operation_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn healthz(State(state): State<Arc<ServiceState>>) -> Json<Value> {
    Json(json!({
        "schema_version": "hyperai.local-service.health.v1",
        "status": "ready",
        "instance_id": state.instance_id,
    }))
}

async fn session_bootstrap(State(state): State<Arc<ServiceState>>) -> Result<Response, ApiError> {
    let (cookie, csrf) = state.auth.refresh_browser_session().map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "INTERNAL_ERROR",
    })?;
    let mut response = Json(json!({
        "schema_version": "hyperai.local-service.session.v1",
        "csrf_token": csrf.as_str(),
    }))
    .into_response();
    let value = HeaderValue::from_str(cookie.as_str()).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "INTERNAL_ERROR",
    })?;
    response.headers_mut().insert(header::SET_COOKIE, value);
    Ok(response)
}

async fn agent_connect(
    State(state): State<Arc<ServiceState>>,
    ApiJson(request): ApiJson<AgentConnectRequestV1>,
) -> Result<Json<SessionSnapshotV1>, ApiError> {
    let _lifecycle = state.agent_lifecycle_lock.lock().await;
    let current_snapshot = state.agent_sessions.snapshot().await;
    if !matches!(current_snapshot.status.as_str(), "disconnected" | "failed") {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "AGENT_SESSION_BUSY",
        });
    }
    request.profile.validate().map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "AGENT_PROFILE_INVALID",
    })?;
    let current_executable = std::env::current_exe().map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "AGENT_MCP_UNAVAILABLE",
    })?;
    let mcp_executable = sibling_mcp_executable(&current_executable).ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "AGENT_MCP_UNAVAILABLE",
    })?;
    let metadata = std::fs::symlink_metadata(&mcp_executable).map_err(|_| ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "AGENT_MCP_UNAVAILABLE",
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "AGENT_MCP_UNAVAILABLE",
        });
    }
    let capability = state
        .auth
        .issue_agent_tool_capability()
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
        })?;
    let mcp = AgentMcpConfigV1 {
        endpoint: format!("http://127.0.0.1:{}", state.port),
        capability,
        executable: mcp_executable,
    };
    match state.agent_sessions.connect(request.profile, mcp).await {
        Ok(snapshot) => Ok(Json(snapshot)),
        Err(error) => {
            let _ = state.auth.revoke_agent_tool_capability();
            Err(agent_api_error(error))
        }
    }
}

async fn agent_status(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<SessionSnapshotV1>, ApiError> {
    Ok(Json(state.agent_sessions.snapshot().await))
}

async fn agent_prompt(
    State(state): State<Arc<ServiceState>>,
    ApiJson(request): ApiJson<AgentPromptRequestV1>,
) -> Result<Json<SessionSnapshotV1>, ApiError> {
    state
        .agent_sessions
        .prompt(request.prompt)
        .await
        .map(Json)
        .map_err(agent_api_error)
}

async fn agent_cancel(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<SessionSnapshotV1>, ApiError> {
    state
        .agent_sessions
        .cancel()
        .await
        .map(Json)
        .map_err(agent_api_error)
}

async fn agent_disconnect(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<SessionSnapshotV1>, ApiError> {
    let _lifecycle = state.agent_lifecycle_lock.lock().await;
    state
        .auth
        .revoke_agent_tool_capability()
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
        })?;
    let snapshot = state
        .agent_sessions
        .disconnect()
        .await
        .map_err(agent_api_error)?;
    Ok(Json(snapshot))
}

async fn agent_eval_reports(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<Vec<EvalRunReportV1>>, ApiError> {
    let mut reports = state
        .eval_runs
        .lock()
        .await
        .values()
        .map(|(_, report)| report.clone())
        .collect::<Vec<_>>();
    reports.sort_by_key(|report| format!("{:?}", report.repeated_result_digest));
    Ok(Json(reports))
}

fn agent_api_error(error: AcpSessionError) -> ApiError {
    let (status, code) = match error {
        AcpSessionError::InvalidProfile | AcpSessionError::LimitExceeded => {
            (StatusCode::BAD_REQUEST, "AGENT_REQUEST_REJECTED")
        }
        AcpSessionError::Busy => (StatusCode::CONFLICT, "AGENT_SESSION_BUSY"),
        AcpSessionError::UnsupportedPlatform => {
            (StatusCode::NOT_IMPLEMENTED, "AGENT_PLATFORM_UNSUPPORTED")
        }
        AcpSessionError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "AGENT_TIMEOUT"),
        AcpSessionError::VersionMismatch => {
            (StatusCode::UNPROCESSABLE_ENTITY, "AGENT_VERSION_MISMATCH")
        }
        AcpSessionError::Protocol
        | AcpSessionError::Disconnected
        | AcpSessionError::StderrLimit
        | AcpSessionError::Process
        | AcpSessionError::Random => (StatusCode::SERVICE_UNAVAILABLE, "AGENT_SESSION_FAILED"),
    };
    ApiError { status, code }
}

async fn stop_service(State(state): State<Arc<ServiceState>>) -> impl IntoResponse {
    let _lifecycle = state.agent_lifecycle_lock.lock().await;
    let _ = state.agent_sessions.disconnect().await;
    let _ = state.auth.revoke_agent_tool_capability();
    let _ = state.shutdown.send(true);
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "schema_version": "hyperai.local-service.stop.v1",
            "status": "shutdown_requested",
        })),
    )
}

async fn market_demo() -> Result<Json<LocalMarketDemoV1>, ApiError> {
    run_local_market_demo().map(Json).map_err(ApiError::from)
}

async fn market_supply_comparison() -> Result<Json<SupplyComparisonV1>, ApiError> {
    market_supply_comparison_v1()
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_workload_trace(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<WorkloadTraceV1>, ApiError> {
    workload_trace_v1(Some(&state.data_root))
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_workload_trace_failure_preview(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<WorkloadTraceV1>, ApiError> {
    workload_trace_seller_failure_preview_v1(Some(&state.data_root))
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_workload_trace_inconclusive_preview(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<WorkloadTraceV1>, ApiError> {
    workload_trace_inconclusive_preview_v1(Some(&state.data_root))
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_workload_trace_data_status_preview(
    State(_state): State<Arc<ServiceState>>,
    AxumPath(status): AxumPath<String>,
) -> Result<Json<WorkloadTraceV1>, ApiError> {
    let status = WorkloadTraceDataStatusPreviewV1::from_status(&status).ok_or(ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "WORKLOAD_TRACE_DATA_STATUS_INVALID",
    })?;
    workload_trace_data_status_preview_v1(status)
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_workload_trace_duplicate_disposition_preview(
    State(_state): State<Arc<ServiceState>>,
) -> Result<Json<WorkloadTraceV1>, ApiError> {
    workload_trace_duplicate_disposition_preview_v1()
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_quote_preview() -> Result<Json<LocalMarketQuotePreviewV1>, ApiError> {
    preview_local_market_quote_v1()
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_quote_preview_scenario(
    AxumPath(scenario_id): AxumPath<String>,
) -> Result<Json<LocalMarketQuotePreviewV1>, ApiError> {
    let scenario = LocalMarketQuoteScenarioV1::from_scenario_id(&scenario_id).ok_or(ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "MARKET_QUOTE_SCENARIO_INVALID",
    })?;
    preview_local_market_quote_scenario_v1(scenario)
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_supply_quote_preview(
    ApiJson(request): ApiJson<SupplyQuotePreviewRequestV1>,
) -> Result<Json<SupplyQuotePreviewV1>, ApiError> {
    market_supply_quote_preview_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn market_quote_approve(
    State(state): State<Arc<ServiceState>>,
    ApiJson(request): ApiJson<LocalMarketQuoteApprovalRequestV1>,
) -> Result<Json<LocalMarketQuoteApprovalV1>, ApiError> {
    let approval = approve_local_market_quote_v1(&request).map_err(ApiError::from)?;
    state
        .market_history
        .record_approval(&approval)
        .await
        .map_err(|error| market_history_api_error(&error))?;
    Ok(Json(approval))
}

async fn market_quote_complete(
    State(state): State<Arc<ServiceState>>,
    ApiJson(request): ApiJson<LocalMarketQuoteCompletionRequestV1>,
) -> Result<Json<LocalMarketQuoteCompletionV1>, ApiError> {
    let entry = state
        .market_history
        .find_deal(request.preview_digest, request.deal_digest)
        .await
        .map_err(|error| market_history_api_error(&error))?
        .ok_or(ApiError {
            status: StatusCode::CONFLICT,
            code: "MARKET_DEAL_NOT_APPROVED",
        })?;
    validate_market_history_entry(&entry)?;
    let completion = complete_local_market_quote_v1(&request).map_err(ApiError::from)?;
    state
        .market_history
        .record_completion(&entry.approval, &completion.demo.receipt)
        .await
        .map_err(|error| market_history_api_error(&error))?;
    Ok(Json(completion))
}

async fn market_deal_history(
    State(state): State<Arc<ServiceState>>,
    request: Request<Body>,
) -> Result<Json<hyperai_market::LocalMarketHistoryPageV1>, ApiError> {
    const DEFAULT_LIMIT: usize = 20;
    const MAX_LIMIT: usize = 50;
    let (requested_limit, cursor) = parse_market_history_query(request.uri().query())?;
    let limit = requested_limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "MARKET_HISTORY_INVALID_QUERY",
        });
    }
    let before_sequence = cursor
        .as_deref()
        .map(decode_market_history_cursor)
        .transpose()?;
    let (mut entries, has_more) = state
        .market_history
        .read_page(before_sequence, limit)
        .await
        .map_err(|error| market_history_api_error(&error))?;
    for entry in &entries {
        validate_market_history_entry(entry)?;
    }
    let next_cursor = if has_more {
        entries
            .last()
            .map(|entry| encode_market_history_cursor(entry.sequence))
    } else {
        None
    };
    let page =
        local_market_history_page_v1(std::mem::take(&mut entries), next_cursor).map_err(|_| {
            ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "MARKET_HISTORY_CORRUPT",
            }
        })?;
    verify_local_market_history_page_v1(&page).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "MARKET_HISTORY_CORRUPT",
    })?;
    let page_bytes = serde_json::to_vec(&page).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "MARKET_HISTORY_CORRUPT",
    })?;
    if page_bytes.len() > MAX_JSON_BODY_BYTES {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "MARKET_HISTORY_PAGE_TOO_LARGE",
        });
    }
    Ok(Json(page))
}

fn parse_market_history_query(
    query: Option<&str>,
) -> Result<(Option<usize>, Option<String>), ApiError> {
    let Some(query) = query else {
        return Ok((None, None));
    };
    if query.is_empty() || query.len() > 128 || !query.is_ascii() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "MARKET_HISTORY_INVALID_QUERY",
        });
    }
    let mut limit = None;
    let mut cursor = None;
    for field in query.split('&') {
        let (key, value) = field.split_once('=').ok_or(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "MARKET_HISTORY_INVALID_QUERY",
        })?;
        match key {
            "limit"
                if limit.is_none()
                    && !value.is_empty()
                    && value.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                limit = Some(value.parse().map_err(|_| ApiError {
                    status: StatusCode::BAD_REQUEST,
                    code: "MARKET_HISTORY_INVALID_QUERY",
                })?);
            }
            "cursor" if cursor.is_none() && !value.is_empty() => {
                cursor = Some(value.to_owned());
            }
            _ => {
                return Err(ApiError {
                    status: StatusCode::BAD_REQUEST,
                    code: "MARKET_HISTORY_INVALID_QUERY",
                });
            }
        }
    }
    Ok((limit, cursor))
}

fn encode_market_history_cursor(sequence: u64) -> String {
    format!("mh1_{}", BASE32_NOPAD.encode(&sequence.to_be_bytes()))
}

fn decode_market_history_cursor(cursor: &str) -> Result<u64, ApiError> {
    let encoded = cursor.strip_prefix("mh1_").ok_or(ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "MARKET_HISTORY_INVALID_QUERY",
    })?;
    let bytes = BASE32_NOPAD
        .decode(encoded.as_bytes())
        .map_err(|_| ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "MARKET_HISTORY_INVALID_QUERY",
        })?;
    let bytes: [u8; 8] = bytes.try_into().map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "MARKET_HISTORY_INVALID_QUERY",
    })?;
    let sequence = u64::from_be_bytes(bytes);
    if sequence == 0 || sequence > i64::MAX as u64 || cursor.len() > 32 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "MARKET_HISTORY_INVALID_QUERY",
        });
    }
    Ok(sequence)
}

fn validate_market_history_entry(entry: &LocalMarketHistoryEntryV1) -> Result<(), ApiError> {
    let approval_request = LocalMarketQuoteApprovalRequestV1 {
        preview_digest: entry.approval.preview_digest,
    };
    let expected_approval =
        approve_local_market_quote_v1(&approval_request).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "MARKET_HISTORY_CORRUPT",
        })?;
    if expected_approval != entry.approval {
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "MARKET_HISTORY_CORRUPT",
        });
    }
    if let Some(receipt) = &entry.completion_receipt {
        verify_local_market_receipt_v1(receipt).map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "MARKET_HISTORY_CORRUPT",
        })?;
        let completion_request = LocalMarketQuoteCompletionRequestV1 {
            preview_digest: entry.approval.preview_digest,
            deal_digest: entry.approval.deal_digest,
        };
        let expected_completion =
            complete_local_market_quote_v1(&completion_request).map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "MARKET_HISTORY_CORRUPT",
            })?;
        if expected_completion.demo.receipt != *receipt
            || receipt.body.approved_deal != entry.approval.approved_deal
        {
            return Err(ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "MARKET_HISTORY_CORRUPT",
            });
        }
    }
    Ok(())
}

fn market_history_api_error(error: &MarketHistoryError) -> ApiError {
    match error {
        MarketHistoryError::ApprovalMissing => ApiError {
            status: StatusCode::CONFLICT,
            code: "MARKET_DEAL_NOT_APPROVED",
        },
        MarketHistoryError::Conflict => ApiError {
            status: StatusCode::CONFLICT,
            code: "MARKET_HISTORY_CONFLICT",
        },
        MarketHistoryError::TooLarge => ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "MARKET_HISTORY_RECORD_TOO_LARGE",
        },
        MarketHistoryError::Corrupt | MarketHistoryError::Json(_) => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "MARKET_HISTORY_CORRUPT",
        },
        MarketHistoryError::NewerSchema
        | MarketHistoryError::Storage(_)
        | MarketHistoryError::Sqlite(_) => ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "MARKET_HISTORY_UNAVAILABLE",
        },
    }
}

async fn market_verify(
    ApiJson(receipt): ApiJson<LocalMarketReceiptV1>,
) -> Result<Json<LocalMarketVerificationV1>, ApiError> {
    verify_local_market_receipt_v1(&receipt)
        .map(Json)
        .map_err(ApiError::from)
}

async fn simple_market_capture_verify(
    ApiJson(request): ApiJson<SimpleMarketCaptureRequestV1>,
) -> Result<Json<hyperai_application::ListingsCaptureVerificationV1>, ApiError> {
    verify_simple_market_capture_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn hyperintel_verify_bundle(
    ApiJson(request): ApiJson<HyperIntelBundleRequestV1>,
) -> Result<Json<HyperIntelBundleVerificationV1>, ApiError> {
    verify_hyperintel_market_bundle_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn hyperintel_reconcile_execution(
    ApiJson(request): ApiJson<HyperIntelReconciliationRequestV1>,
) -> Result<Json<HyperIntelReconciliationV1>, ApiError> {
    reconcile_hyperintel_execution_v1(&request)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_catalog(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<hyperai_application::DataCatalogV1>, ApiError> {
    data_catalog_v1(&state.data_root)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_synthetic_release(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<hyperai_application::SyntheticDataReleaseV1>, ApiError> {
    let _query_slot = Arc::clone(&state.query_slots)
        .try_acquire_owned()
        .map_err(|_| ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "DATA_QUERY_BUSY",
        })?;
    synthetic_data_release_v1(&state.data_root)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_models(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<DataModelCatalogV1>, ApiError> {
    data_models_v1(&state.data_root)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_views(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<DataViewCatalogV1>, ApiError> {
    data_views_v1(&state.data_root)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_describe(
    State(state): State<Arc<ServiceState>>,
    AxumPath(dataset_id): AxumPath<String>,
) -> Result<Json<DatasetManifestV1>, ApiError> {
    describe_data_v1(&state.data_root, &dataset_id)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_seed_preview() -> Result<Json<DefinitionPreviewV1>, ApiError> {
    preview_data_synthetic_seed_v1()
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_seed(
    State(state): State<Arc<ServiceState>>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<DataSeedRequestV1>,
) -> Result<Json<DatasetManifestV1>, ApiError> {
    let idempotency_key = idempotency_key(&headers)?;
    seed_data_synthetic_confirmed_v1(&state.data_root, idempotency_key, request.preview_digest)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_gpudiff_refresh(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<GpudiffImportV1>, ApiError> {
    refresh_gpudiff_data_snapshot(state.data_root.clone())
        .await
        .map(Json)
}

async fn data_gpudiff_schedule_status(
    State(state): State<Arc<ServiceState>>,
) -> Json<GpudiffScheduleStatusV1> {
    Json(state.gpudiff_schedule_status.lock().await.clone())
}

async fn data_gpudiff_schedule_start(
    State(state): State<Arc<ServiceState>>,
) -> Json<GpudiffScheduleStatusV1> {
    let mut task_slot = state.gpudiff_schedule_task.lock().await;
    if task_slot
        .as_ref()
        .is_some_and(|task| !task.join.is_finished())
    {
        return Json(state.gpudiff_schedule_status.lock().await.clone());
    }

    let previous_status = state.gpudiff_schedule_status.lock().await.clone();
    if previous_status.state == "paused_capacity" {
        return Json(previous_status);
    }
    if let Some(finished_task) = task_slot.take() {
        let _ = finished_task.join.await;
    }

    let (cancel, mut cancel_signal) = tokio::sync::watch::channel(false);
    let (initial_result_sender, initial_result_receiver) = tokio::sync::oneshot::channel();
    let data_root = state.data_root.clone();
    let schedule_status = Arc::clone(&state.gpudiff_schedule_status);
    {
        let mut status = schedule_status.lock().await;
        "starting".clone_into(&mut status.state);
        status.last_error_code = None;
    }

    let task = tokio::spawn(async move {
        let mut initial_result_sender = Some(initial_result_sender);
        loop {
            {
                let mut status = schedule_status.lock().await;
                "refreshing".clone_into(&mut status.state);
                status.last_attempt_unix_seconds = unix_timestamp_seconds();
                status.last_error_code = None;
            }

            match refresh_gpudiff_data_snapshot(data_root.clone()).await {
                Ok(imported) => {
                    let mut status = schedule_status.lock().await;
                    "active".clone_into(&mut status.state);
                    status.last_success_unix_seconds = unix_timestamp_seconds();
                    status.last_raw_source_digest =
                        Some(imported.receipt.raw_source_digest.to_hex());
                    status.last_row_count = Some(imported.receipt.row_count);
                    status.last_observed_at_min = Some(imported.receipt.observed_at_min);
                    status.last_observed_at_max = Some(imported.receipt.observed_at_max);
                    status.last_error_code = None;
                }
                Err(error) if error.code == "DATA_CAPTURE_RETENTION_FULL" => {
                    let status_snapshot = {
                        let mut status = schedule_status.lock().await;
                        "paused_capacity".clone_into(&mut status.state);
                        status.last_error_code = Some(error.code.to_owned());
                        status.clone()
                    };
                    if let Some(sender) = initial_result_sender.take() {
                        let _ = sender.send(status_snapshot);
                    }
                    return;
                }
                Err(error) => {
                    let mut status = schedule_status.lock().await;
                    "active".clone_into(&mut status.state);
                    status.last_error_code = Some(error.code.to_owned());
                }
            }

            if let Some(sender) = initial_result_sender.take() {
                let _ = sender.send(schedule_status.lock().await.clone());
            }

            tokio::select! {
                () = tokio::time::sleep(GPUDIFF_REFRESH_INTERVAL) => {}
                _ = cancel_signal.changed() => break,
            }
        }

        let mut status = schedule_status.lock().await;
        if status.state != "paused_capacity" {
            "stopped".clone_into(&mut status.state);
        }
    });
    *task_slot = Some(GpudiffScheduleTask { cancel, join: task });
    drop(task_slot);

    let _ = initial_result_receiver.await;
    Json(state.gpudiff_schedule_status.lock().await.clone())
}

async fn data_gpudiff_schedule_stop(
    State(state): State<Arc<ServiceState>>,
) -> Json<GpudiffScheduleStatusV1> {
    stop_gpudiff_schedule(&state).await;
    Json(state.gpudiff_schedule_status.lock().await.clone())
}

async fn stop_gpudiff_schedule(state: &ServiceState) {
    let task = state.gpudiff_schedule_task.lock().await.take();
    if let Some(task) = task {
        let _ = task.cancel.send(true);
        let _ = task.join.await;
    }
    let mut status = state.gpudiff_schedule_status.lock().await;
    if status.state != "paused_capacity" {
        "stopped".clone_into(&mut status.state);
    }
}

fn unix_timestamp_seconds() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

async fn refresh_gpudiff_data_snapshot(data_root: PathBuf) -> Result<GpudiffImportV1, ApiError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(10))
        .user_agent("HyperAI local public-data snapshot/1.0")
        .build()
        .map_err(|_| ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "DATA_SOURCE_UNAVAILABLE",
        })?;
    let response = client
        .get(GPUDIFF_SOURCE_URL_V1)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "DATA_SOURCE_UNAVAILABLE",
        })?;
    if response.status() != StatusCode::OK
        || response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_none_or(|value| {
                !value.split(';').next().is_some_and(|media_type| {
                    media_type.trim().eq_ignore_ascii_case("application/json")
                })
            })
        || response
            .content_length()
            .is_none_or(|length| length == 0 || length > MAX_GPUDIFF_BODY_BYTES)
    {
        return Err(ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "DATA_SOURCE_RESPONSE_REJECTED",
        });
    }
    let expected_length = response.content_length().ok_or(ApiError {
        status: StatusCode::BAD_GATEWAY,
        code: "DATA_SOURCE_RESPONSE_REJECTED",
    })?;
    let etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let last_modified = response
        .headers()
        .get(header::LAST_MODIFIED)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.bytes().await.map_err(|_| ApiError {
        status: StatusCode::BAD_GATEWAY,
        code: "DATA_SOURCE_RESPONSE_REJECTED",
    })?;
    if bytes.len() as u64 != expected_length || bytes.len() as u64 > MAX_GPUDIFF_BODY_BYTES {
        return Err(ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "DATA_SOURCE_RESPONSE_REJECTED",
        });
    }

    let bytes = bytes.to_vec();
    let result = tokio::task::spawn_blocking(move || {
        import_gpudiff_data_snapshot_v1(
            &data_root,
            &bytes,
            etag.as_deref(),
            last_modified.as_deref(),
        )
    })
    .await
    .map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "DATA_IMPORT_FAILED",
    })?
    .map_err(ApiError::from)?;
    Ok(result)
}

async fn data_query(
    State(state): State<Arc<ServiceState>>,
    ApiJson(request): ApiJson<DataQueryEnvelopeV1>,
) -> Result<Json<QueryResultV1>, ApiError> {
    let _query_slot = Arc::clone(&state.query_slots)
        .try_acquire_owned()
        .map_err(|_| ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "DATA_QUERY_BUSY",
        })?;
    if !valid_request_id(&request.query_id) {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_REQUEST",
        });
    }
    let (sender, receiver) = tokio::sync::watch::channel(false);
    {
        let mut active = state.active_queries.lock().map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
        })?;
        if active.contains_key(&request.query_id) {
            return Err(ApiError {
                status: StatusCode::CONFLICT,
                code: "CONFLICT",
            });
        }
        active.insert(request.query_id.clone(), sender);
    }
    let _registration = ActiveQueryRegistration {
        queries: Arc::clone(&state.active_queries),
        query_id: request.query_id,
    };
    #[cfg(test)]
    if let Some((registered, execution_gate)) = &state.test_query_gate {
        registered.notify_one();
        let _permit = Arc::clone(execution_gate)
            .acquire_owned()
            .await
            .map_err(|_| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "INTERNAL_ERROR",
            })?;
    }
    query_data_v1(&state.data_root, &request.request, receiver)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_query_cancel(
    State(state): State<Arc<ServiceState>>,
    AxumPath(query_id): AxumPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let sender = state
        .active_queries
        .lock()
        .map_err(|_| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
        })?
        .get(&query_id)
        .cloned()
        .ok_or(ApiError {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
        })?;
    sender.send(true).map_err(|_| ApiError {
        status: StatusCode::CONFLICT,
        code: "CONFLICT",
    })?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "schema_version": "hyperai.data.query-cancel.v1",
            "query_id": query_id,
            "status": "cancellation_requested",
        })),
    ))
}

async fn data_model_preview(
    State(state): State<Arc<ServiceState>>,
    ApiJson(model): ApiJson<DataModelV1>,
) -> Result<Json<DefinitionPreviewV1>, ApiError> {
    preview_data_model_v1(&state.data_root, &model)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_model_save(
    State(state): State<Arc<ServiceState>>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<DataModelSaveRequestV1>,
) -> Result<Json<hyperai_application::DefinitionSavedV1>, ApiError> {
    save_data_model_v1(
        &state.data_root,
        &request.model,
        idempotency_key(&headers)?,
        request.preview_digest,
    )
    .map(Json)
    .map_err(ApiError::from)
}

async fn data_view_preview(
    State(state): State<Arc<ServiceState>>,
    ApiJson(view): ApiJson<DataViewV1>,
) -> Result<Json<DefinitionPreviewV1>, ApiError> {
    preview_data_view_v1(&state.data_root, &view)
        .map(Json)
        .map_err(ApiError::from)
}

async fn data_view_save(
    State(state): State<Arc<ServiceState>>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<DataViewSaveRequestV1>,
) -> Result<Json<hyperai_application::DefinitionSavedV1>, ApiError> {
    save_data_view_v1(
        &state.data_root,
        &request.view,
        idempotency_key(&headers)?,
        request.preview_digest,
    )
    .map(Json)
    .map_err(ApiError::from)
}

fn idempotency_key(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_request_id(value))
        .ok_or(ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "INVALID_REQUEST",
        })
}

async fn request_security(
    State(state): State<Arc<ServiceState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let supplied_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok());
    let invalid_id = request.headers().contains_key("x-request-id")
        && supplied_id.is_none_or(|value| !valid_request_id(value));
    let request_id = supplied_id
        .filter(|value| valid_request_id(value))
        .map_or_else(next_request_id, str::to_owned);

    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    let valid_host = host.is_some_and(|value| {
        value == format!("127.0.0.1:{}", state.port) || value == format!("localhost:{}", state.port)
    });
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let valid_origin =
        origin.is_none_or(|value| host.is_some_and(|host| value == format!("http://{host}")));
    if invalid_id || !valid_host || !valid_origin {
        let code = if invalid_id {
            "INVALID_REQUEST"
        } else {
            "FORBIDDEN"
        };
        return decorate_response(
            ApiError {
                status: if invalid_id {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::FORBIDDEN
                },
                code,
            }
            .into_response(),
            &request_id,
        );
    }

    let path = request.uri().path();
    if request.method() == Method::POST
        && path.starts_with("/api/v1/")
        && path != "/api/v1/service/stop"
    {
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !content_type
            .split(';')
            .next()
            .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
        {
            return decorate_response(
                ApiError {
                    status: StatusCode::BAD_REQUEST,
                    code: "INVALID_JSON",
                }
                .into_response(),
                &request_id,
            );
        }
    }

    let response = match tokio::time::timeout(Duration::from_secs(30), async move {
        let (parts, body) = request.into_parts();
        match to_bytes(body, MAX_JSON_BODY_BYTES).await {
            Ok(bytes) => {
                next.run(Request::from_parts(parts, Body::from(bytes)))
                    .await
            }
            Err(_) => ApiError {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "BODY_TOO_LARGE",
            }
            .into_response(),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => ApiError {
            status: StatusCode::GATEWAY_TIMEOUT,
            code: "REQUEST_TIMEOUT",
        }
        .into_response(),
    };
    decorate_response(response, &request_id)
}

async fn authentication(
    State(state): State<Arc<ServiceState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if !path.starts_with("/api/v1/") || matches!(path, "/api/v1/status" | "/api/v1/session") {
        return next.run(request).await;
    }

    let bearer = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if bearer
        .is_some_and(|candidate| bearer_authorized(&state.auth, request.method(), path, candidate))
    {
        return next.run(request).await;
    }

    let cookie = request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(';').find_map(|pair| {
                let (name, value) = pair.trim().split_once('=')?;
                (name == "hyperai_session").then_some(value)
            })
        });
    if cookie.is_some_and(|candidate| state.auth.session_matches(candidate)) {
        if !matches!(*request.method(), Method::GET | Method::HEAD)
            && !request
                .headers()
                .get("x-csrf-token")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|candidate| state.auth.csrf_matches(candidate))
        {
            return ApiError {
                status: StatusCode::FORBIDDEN,
                code: "FORBIDDEN",
            }
            .into_response();
        }
        if !matches!(*request.method(), Method::GET | Method::HEAD)
            && !request.headers().contains_key(header::ORIGIN)
        {
            return ApiError {
                status: StatusCode::FORBIDDEN,
                code: "FORBIDDEN",
            }
            .into_response();
        }
        return next.run(request).await;
    }

    ApiError {
        status: StatusCode::UNAUTHORIZED,
        code: "UNAUTHENTICATED",
    }
    .into_response()
}

fn bearer_authorized(auth: &AuthState, method: &Method, path: &str, candidate: &str) -> bool {
    auth.bearer_matches(candidate)
        || (crate::route_policy::agent_tool_route_allowed(method, path)
            && auth.agent_tool_capability_matches(candidate))
}

#[cfg(test)]
mod agent_tool_authorization_tests {
    use axum::http::{Method, StatusCode};

    use super::{
        AgentSessionManager, FleetSimulationStoreV1, GpudiffScheduleStatusV1,
        MAX_ACTIVE_DATA_QUERIES, MarketHistoryStore, ServiceState, app, bearer_authorized,
    };
    use crate::{
        auth::AuthState,
        route_policy::{AgentToolRoute, agent_tool_route_allowed},
    };

    const OWNER_ROUTES: [(&str, &str); 50] = [
        ("POST", "/api/v1/market/demo"),
        ("GET", "/api/v1/market/workload-trace"),
        ("GET", "/api/v1/market/supply-comparison"),
        ("POST", "/api/v1/market/supply-quote-preview"),
        ("GET", "/api/v1/market/workload-trace/failure-preview"),
        ("GET", "/api/v1/market/workload-trace/inconclusive-preview"),
        (
            "GET",
            "/api/v1/market/workload-trace/data-status-preview/missing",
        ),
        (
            "GET",
            "/api/v1/market/workload-trace/duplicate-disposition-preview",
        ),
        ("POST", "/api/v1/market/quote-preview"),
        ("POST", "/api/v1/market/quote-approve"),
        ("POST", "/api/v1/market/quote-complete"),
        ("GET", "/api/v1/market/deal-history"),
        ("POST", "/api/v1/market/verify"),
        ("POST", "/api/v1/market/registry-capture/verify"),
        ("POST", "/api/v1/inference/verify-bundle"),
        ("POST", "/api/v1/inference/reconcile-execution"),
        ("GET", "/api/v1/fleet/inventory"),
        ("GET", "/api/v1/fleet/doctor"),
        ("POST", "/api/v1/fleet/operation-preview"),
        ("GET", "/api/v1/fleet/simulation"),
        ("POST", "/api/v1/fleet/simulation/transition"),
        ("GET", "/api/v1/eval/catalog"),
        ("POST", "/api/v1/eval/run"),
        ("GET", "/api/v1/trade/catalog"),
        ("GET", "/api/v1/trade/series/gpu-price-index"),
        ("POST", "/api/v1/trade/scenario"),
        ("POST", "/api/v1/trade/paper"),
        ("GET", "/api/v1/data/catalog"),
        ("GET", "/api/v1/data/models"),
        ("GET", "/api/v1/data/views"),
        ("GET", "/api/v1/data/describe/synthetic-gpu-prices"),
        ("GET", "/api/v1/data/synthetic-release"),
        ("GET", "/api/v1/data/synthetic/preview"),
        ("POST", "/api/v1/data/synthetic/seed"),
        ("POST", "/api/v1/data/gpudiff/refresh"),
        ("GET", "/api/v1/data/gpudiff/schedule"),
        ("POST", "/api/v1/data/gpudiff/schedule"),
        ("POST", "/api/v1/data/gpudiff/schedule/stop"),
        ("POST", "/api/v1/data/query"),
        ("POST", "/api/v1/data/query/synthetic-query/cancel"),
        ("POST", "/api/v1/data/models/preview"),
        ("POST", "/api/v1/data/models/save"),
        ("POST", "/api/v1/data/views/preview"),
        ("POST", "/api/v1/data/views/save"),
        ("POST", "/api/v1/agent/connect"),
        ("GET", "/api/v1/agent/status"),
        ("POST", "/api/v1/agent/prompt"),
        ("POST", "/api/v1/agent/cancel"),
        ("POST", "/api/v1/agent/disconnect"),
        ("POST", "/api/v1/service/stop"),
    ];

    #[test]
    fn scoped_capability_authorizes_only_fixed_tool_methods_and_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        let auth = AuthState::generate()?;
        let capability = auth.issue_agent_tool_capability()?;
        for route in AgentToolRoute::ALL {
            let method = route.method().http_method();
            let path = route.path().replace("{instrument_id}", "gpu-price-index");
            assert!(agent_tool_route_allowed(&method, &path), "{method} {path}");
            assert!(
                bearer_authorized(&auth, &method, &path, &capability),
                "scoped capability denied {method} {path}"
            );
        }

        let denied = [
            (Method::GET, "/api/v1/agent/tools/data/query"),
            (Method::POST, "/api/v1/agent/tools/fleet/inventory"),
            (
                Method::POST,
                "/api/v1/agent/tools/fleet/simulation/transition",
            ),
            (Method::POST, "/api/v1/fleet/simulation/transition"),
            (Method::POST, "/api/v1/fleet/operation-preview"),
            (Method::GET, "/api/v1/data/gpudiff/schedule"),
            (Method::POST, "/api/v1/data/gpudiff/schedule"),
            (Method::POST, "/api/v1/data/gpudiff/schedule/stop"),
            (Method::GET, "/api/v1/agent/tools/trade/series/"),
            (Method::GET, "/api/v1/agent/tools/trade/series/a/extra"),
            (Method::POST, "/api/v1/agent/tools/market/workload-trace"),
            (
                Method::POST,
                "/api/v1/agent/tools/market/workload-trace/failure-preview",
            ),
            (
                Method::POST,
                "/api/v1/agent/tools/market/workload-trace/inconclusive-preview",
            ),
            (Method::POST, "/api/v1/agent/tools/market/supply-comparison"),
            (
                Method::GET,
                "/api/v1/market/workload-trace/inconclusive-preview",
            ),
            (Method::GET, "/api/v1/agent/tools/unknown"),
        ];

        for (method, path) in denied {
            assert!(!agent_tool_route_allowed(&method, path), "{method} {path}");
            assert!(
                !bearer_authorized(&auth, &method, path, &capability),
                "scoped capability authorized {method} {path}"
            );
        }

        assert!(bearer_authorized(
            &auth,
            &Method::POST,
            "/api/v1/fleet/simulation/transition",
            auth.bearer()
        ));
        Ok(())
    }

    #[test]
    fn scoped_capability_denies_every_registered_owner_route()
    -> Result<(), Box<dyn std::error::Error>> {
        let auth = AuthState::generate()?;
        let capability = auth.issue_agent_tool_capability()?;
        for (method, path) in OWNER_ROUTES {
            let method = Method::from_bytes(method.as_bytes())?;
            assert!(!agent_tool_route_allowed(&method, path), "{method} {path}");
            assert!(
                !bearer_authorized(&auth, &method, path, &capability),
                "scoped capability authorized owner route {method} {path}"
            );
            assert!(
                bearer_authorized(&auth, &method, path, auth.bearer()),
                "owner bearer denied owner route {method} {path}"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn http_cancel_route_cancels_a_registered_query_without_partial_rows()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::{collections::HashMap, sync::Arc, time::Duration};

        use axum::http::StatusCode;
        use serde_json::{Value, json};

        let temporary = tempfile::tempdir()?;
        let data_root = temporary.path().join("data");
        std::fs::create_dir_all(&data_root)?;

        let auth = AuthState::generate()?;
        let bearer = auth.bearer().to_owned();
        let (shutdown, _) = tokio::sync::watch::channel(false);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let registered = Arc::new(tokio::sync::Notify::new());
        let execution_gate = Arc::new(tokio::sync::Semaphore::new(0));
        let state = Arc::new(ServiceState {
            auth,
            shutdown,
            instance_id: "synthetic-query-cancellation-test".to_owned(),
            port: address.port(),
            data_root,
            market_history: MarketHistoryStore::open(&crate::storage::paths_for_home(Some(
                temporary.path(),
            ))?)
            .await?,
            active_queries: Arc::new(std::sync::Mutex::new(HashMap::new())),
            query_slots: Arc::new(tokio::sync::Semaphore::new(MAX_ACTIVE_DATA_QUERIES)),
            fleet_simulation: FleetSimulationStoreV1::default(),
            eval_runs: tokio::sync::Mutex::new(HashMap::new()),
            agent_sessions: AgentSessionManager::default(),
            agent_lifecycle_lock: tokio::sync::Mutex::new(()),
            gpudiff_schedule_status: Arc::new(tokio::sync::Mutex::new(
                GpudiffScheduleStatusV1::default(),
            )),
            gpudiff_schedule_task: tokio::sync::Mutex::new(None),
            test_query_gate: Some((Arc::clone(&registered), Arc::clone(&execution_gate))),
        });
        let router = app(state);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let base = format!("http://{address}");
        let query_id = "cancel-active-query";
        let query_client = client.clone();
        let query_base = base.clone();
        let query_bearer = bearer.clone();
        let query_task = tokio::spawn(async move {
            query_client
                .post(format!("{query_base}/api/v1/data/query"))
                .bearer_auth(query_bearer)
                .json(&json!({
                    "query_id": query_id,
                    "request": {
                        "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                        "sql": "SELECT gpu_model FROM dataset",
                        "format": "json",
                        "max_rows": 10,
                    }
                }))
                .send()
                .await
        });

        registered.notified().await;
        let cancel_response = client
            .post(format!("{base}/api/v1/data/query/{query_id}/cancel"))
            .bearer_auth(&bearer)
            .json(&json!({}))
            .send()
            .await?;
        assert_eq!(cancel_response.status(), StatusCode::ACCEPTED);
        let cancel_body: Value = cancel_response.json().await?;
        assert_eq!(cancel_body["status"], "cancellation_requested");

        execution_gate.add_permits(1);
        let query_response = query_task.await??;
        assert_eq!(query_response.status(), StatusCode::CONFLICT);
        let query_body: Value = query_response.json().await?;
        assert_eq!(query_body["code"], "DATA_QUERY_CANCELLED");
        assert!(query_body.get("rows").is_none());

        server.abort();
        let _ = server.await;
        Ok(())
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn http_middleware_rejects_scoped_capability_on_every_owner_route()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::{collections::HashMap, sync::Arc, time::Duration};

        use axum::http::StatusCode;

        let temporary = tempfile::tempdir()?;
        let data_root = temporary.path().join("data");
        std::fs::create_dir_all(&data_root)?;

        let auth = AuthState::generate()?;
        let owner_bearer = auth.bearer().to_owned();
        let agent_capability = auth.issue_agent_tool_capability()?;
        let (shutdown, _) = tokio::sync::watch::channel(false);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let state = Arc::new(ServiceState {
            auth,
            shutdown,
            instance_id: "synthetic-middleware-test".to_owned(),
            port: address.port(),
            data_root,
            market_history: MarketHistoryStore::open(&crate::storage::paths_for_home(Some(
                temporary.path(),
            ))?)
            .await?,
            active_queries: Arc::new(std::sync::Mutex::new(HashMap::new())),
            query_slots: Arc::new(tokio::sync::Semaphore::new(MAX_ACTIVE_DATA_QUERIES)),
            fleet_simulation: FleetSimulationStoreV1::default(),
            eval_runs: tokio::sync::Mutex::new(HashMap::new()),
            agent_sessions: AgentSessionManager::default(),
            agent_lifecycle_lock: tokio::sync::Mutex::new(()),
            gpudiff_schedule_status: Arc::new(tokio::sync::Mutex::new(
                GpudiffScheduleStatusV1::default(),
            )),
            gpudiff_schedule_task: tokio::sync::Mutex::new(None),
            test_query_gate: None,
        });
        let router = app(state);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let base = format!("http://{address}");

        let owner_status = client
            .get(format!("{base}/api/v1/agent/status"))
            .bearer_auth(&owner_bearer)
            .send()
            .await?;
        assert_eq!(owner_status.status(), StatusCode::OK);

        for (method, path) in OWNER_ROUTES {
            let method = reqwest::Method::from_bytes(method.as_bytes())?;
            let is_post = method == reqwest::Method::POST;
            let mut request = client
                .request(method, format!("{base}{path}"))
                .bearer_auth(&agent_capability);
            if is_post {
                request = request
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body("{}");
            }
            let response = request.send().await?;
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "scoped capability reached owner route {path}"
            );
            let body: serde_json::Value = response.json().await?;
            assert_eq!(body["code"], "UNAUTHENTICATED", "{path}");
        }

        let trace = client
            .get(format!("{base}/api/v1/agent/tools/market/workload-trace"))
            .bearer_auth(&agent_capability)
            .send()
            .await?;
        assert_eq!(trace.status(), StatusCode::OK);
        let trace: serde_json::Value = trace.json().await?;
        assert_eq!(
            trace["schema_version"],
            "hyperai.interface.workload-trace.v1"
        );
        assert_eq!(
            trace["state_slice"],
            "hyperai.bazaar.workload_trace_synthetic.v1"
        );
        assert!(trace["references"].as_array().is_some_and(|references| {
            references
                .iter()
                .any(|reference| reference["owner_id"] == "hyperai-eval")
        }));

        let owner_trace = client
            .get(format!("{base}/api/v1/market/workload-trace"))
            .bearer_auth(&owner_bearer)
            .send()
            .await?;
        assert_eq!(owner_trace.status(), StatusCode::OK);
        let owner_trace: serde_json::Value = owner_trace.json().await?;
        assert_eq!(trace, owner_trace);

        assert_supply_comparison_parity(&client, &base, &agent_capability, &owner_bearer).await?;
        assert_supply_quote_preview_parity(&client, &base, &agent_capability, &owner_bearer)
            .await?;

        assert_scoped_failure_preview_parity(&client, &base, &agent_capability, &owner_bearer)
            .await?;

        server.abort();
        let _ = server.await;
        Ok(())
    }

    async fn assert_supply_comparison_parity(
        client: &reqwest::Client,
        base: &str,
        agent_capability: &str,
        owner_bearer: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let comparison = client
            .get(format!(
                "{base}/api/v1/agent/tools/market/supply-comparison"
            ))
            .bearer_auth(agent_capability)
            .send()
            .await?;
        assert_eq!(comparison.status(), StatusCode::OK);
        let comparison: serde_json::Value = comparison.json().await?;
        assert_eq!(
            comparison["body"]["schema_version"],
            "hyperai.compute-market.supply-comparison.v1"
        );
        assert_eq!(comparison["body"]["selected_offer_id"], "offer-gpu-b");

        let owner_comparison = client
            .get(format!("{base}/api/v1/market/supply-comparison"))
            .bearer_auth(owner_bearer)
            .send()
            .await?;
        assert_eq!(owner_comparison.status(), StatusCode::OK);
        let owner_comparison: serde_json::Value = owner_comparison.json().await?;
        assert_eq!(comparison, owner_comparison);
        Ok(())
    }

    async fn assert_supply_quote_preview_parity(
        client: &reqwest::Client,
        base: &str,
        agent_capability: &str,
        owner_bearer: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let comparison = client
            .get(format!("{base}/api/v1/market/supply-comparison"))
            .bearer_auth(owner_bearer)
            .send()
            .await?;
        assert_eq!(comparison.status(), StatusCode::OK);
        let comparison: serde_json::Value = comparison.json().await?;
        let comparison_digest = comparison["digest"]
            .as_str()
            .ok_or("supply comparison had no digest")?;
        let request = serde_json::json!({ "comparison_digest": comparison_digest });

        let scoped_preview = client
            .post(format!(
                "{base}/api/v1/agent/tools/market/supply-quote-preview"
            ))
            .bearer_auth(agent_capability)
            .json(&request)
            .send()
            .await?;
        assert_eq!(scoped_preview.status(), StatusCode::OK);
        let scoped_preview: serde_json::Value = scoped_preview.json().await?;

        let owner_preview = client
            .post(format!("{base}/api/v1/market/supply-quote-preview"))
            .bearer_auth(owner_bearer)
            .json(&request)
            .send()
            .await?;
        assert_eq!(owner_preview.status(), StatusCode::OK);
        let owner_preview: serde_json::Value = owner_preview.json().await?;
        assert_eq!(scoped_preview, owner_preview);
        assert_eq!(
            scoped_preview["body"]["schema_version"],
            "hyperai.compute-market.supply-quote-preview.v1"
        );
        assert_eq!(
            scoped_preview["body"]["comparison_digest"],
            comparison["digest"]
        );
        assert_eq!(scoped_preview["body"]["selected_offer_id"], "offer-gpu-b");
        assert_eq!(
            scoped_preview["body"]["selected_offer_digest"],
            scoped_preview["body"]["quote_preview"]["body"]["signed_quote"]["quote"]["offer_digest"]
        );

        for owner_action in ["quote-approve", "quote-complete"] {
            let denied = client
                .post(format!("{base}/api/v1/market/{owner_action}"))
                .bearer_auth(agent_capability)
                .json(&serde_json::json!({}))
                .send()
                .await?;
            assert_eq!(denied.status(), StatusCode::UNAUTHORIZED, "{owner_action}");
        }
        Ok(())
    }

    async fn assert_scoped_failure_preview_parity(
        client: &reqwest::Client,
        base: &str,
        agent_capability: &str,
        owner_bearer: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let failure_preview = client
            .get(format!(
                "{base}/api/v1/agent/tools/market/workload-trace/failure-preview"
            ))
            .bearer_auth(agent_capability)
            .send()
            .await?;
        assert_eq!(failure_preview.status(), reqwest::StatusCode::OK);
        let failure_preview: serde_json::Value = failure_preview.json().await?;
        assert_eq!(
            failure_preview["state_slice"],
            "hyperai.bazaar.workload_trace_seller_failure.v1"
        );
        assert_eq!(
            failure_preview["claim_ceiling"],
            "digest-bound local synthetic seller-attributed failure projection"
        );
        assert!(
            failure_preview["references"]
                .as_array()
                .is_some_and(|references| {
                    references.iter().any(|reference| {
                        reference["schema_version"]
                            == "hyperai.compute-market.fulfillment-failure.v1"
                            && reference["status"] == "fulfillment_failed"
                    })
                })
        );

        let owner_failure_preview = client
            .get(format!(
                "{base}/api/v1/market/workload-trace/failure-preview"
            ))
            .bearer_auth(owner_bearer)
            .send()
            .await?;
        assert_eq!(owner_failure_preview.status(), reqwest::StatusCode::OK);
        let owner_failure_preview: serde_json::Value = owner_failure_preview.json().await?;
        assert_eq!(failure_preview, owner_failure_preview);
        Ok(())
    }
}

fn decorate_response(mut response: Response, request_id: &str) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    if let Ok(value) = HeaderValue::from_str(request_id) {
        headers.insert(HeaderName::from_static("x-request-id"), value);
    }
    response
}

fn valid_request_id(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn next_request_id() -> String {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("r-{}-{sequence}", std::process::id())
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
}

struct ApiJson<T>(T);

struct LimitedApiJson<T>(T);
struct LimitedTradeJson<T>(T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request(request: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_| ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_JSON",
            })
    }
}

impl<S, T> FromRequest<S> for LimitedApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request(request: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let bytes = to_bytes(request.into_body(), MAX_EVAL_REQUEST_BYTES)
            .await
            .map_err(|_| ApiError {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "EVAL_REQUEST_TOO_LARGE",
            })?;
        serde_json::from_slice(&bytes)
            .map(Self)
            .map_err(|_| ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_JSON",
            })
    }
}

impl<S, T> FromRequest<S> for LimitedTradeJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request(request: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let bytes = to_bytes(request.into_body(), MAX_TRADE_REQUEST_BYTES)
            .await
            .map_err(|_| ApiError {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "TRADE_REQUEST_TOO_LARGE",
            })?;
        serde_json::from_slice(&bytes)
            .map(Self)
            .map_err(|_| ApiError {
                status: StatusCode::BAD_REQUEST,
                code: "INVALID_JSON",
            })
    }
}

impl From<InterfaceError> for ApiError {
    fn from(error: InterfaceError) -> Self {
        match error {
            InterfaceError::Eval(EvalError::InvalidScorerRequest) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "EVAL_REQUEST_REJECTED",
            },
            InterfaceError::Eval(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "EVAL_CONTRACT_FAILED",
            },
            InterfaceError::Data(DataError::DatasetNotFound) => Self {
                status: StatusCode::NOT_FOUND,
                code: "NOT_FOUND",
            },
            InterfaceError::Data(DataError::IntegrityMismatch | DataError::Conflict) => Self {
                status: StatusCode::CONFLICT,
                code: "CONFLICT",
            },
            InterfaceError::Data(DataError::CaptureRetentionFull) => Self {
                status: StatusCode::INSUFFICIENT_STORAGE,
                code: "DATA_CAPTURE_RETENTION_FULL",
            },
            InterfaceError::Data(DataError::LimitExceeded) => Self {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "DATA_LIMIT_EXCEEDED",
            },
            InterfaceError::Data(DataError::QueryRejected) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "DATA_QUERY_REJECTED",
            },
            InterfaceError::Data(DataError::Cancelled) => Self {
                status: StatusCode::CONFLICT,
                code: "DATA_QUERY_CANCELLED",
            },
            InterfaceError::Data(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "DATA_OPERATION_FAILED",
            },
            InterfaceError::Fleet(FleetError::SimulationConflict) => Self {
                status: StatusCode::CONFLICT,
                code: "FLEET_SIMULATION_CONFLICT",
            },
            InterfaceError::Fleet(FleetError::SimulationTransitionRejected) => Self {
                status: StatusCode::CONFLICT,
                code: "FLEET_SIMULATION_TRANSITION_REJECTED",
            },
            InterfaceError::Fleet(FleetError::SimulationCacheFull) => Self {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: "FLEET_SIMULATION_CACHE_FULL",
            },
            InterfaceError::Fleet(FleetError::SimulationUnavailable) => Self {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "FLEET_SIMULATION_UNAVAILABLE",
            },
            InterfaceError::Fleet(FleetError::InvalidSimulationRequest) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "FLEET_SIMULATION_REQUEST_REJECTED",
            },
            InterfaceError::Fleet(_) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "FLEET_REQUEST_REJECTED",
            },
            InterfaceError::Trade(error) => trade_error_to_api(&error),
            InterfaceError::Market(error) => market_error_to_api(&error),
            InterfaceError::HyperIntelBundle(HyperIntelBundleError::InputTooLarge) => Self {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "HYPERINTEL_INPUT_TOO_LARGE",
            },
            InterfaceError::HyperIntelBundle(_) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "HYPERINTEL_BUNDLE_REJECTED",
            },
            InterfaceError::HyperIntelReconciliation(
                HyperIntelReconciliationError::InputTooLarge,
            ) => Self {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "HYPERINTEL_RECONCILIATION_INPUT_TOO_LARGE",
            },
            InterfaceError::HyperIntelReconciliation(_) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "HYPERINTEL_RECONCILIATION_REJECTED",
            },
            InterfaceError::SimpleMarketCapture(ListingsCaptureError::InputTooLarge) => Self {
                status: StatusCode::PAYLOAD_TOO_LARGE,
                code: "SIMPLE_MARKET_CAPTURE_TOO_LARGE",
            },
            InterfaceError::SimpleMarketCapture(_) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "SIMPLE_MARKET_CAPTURE_REJECTED",
            },
            InterfaceError::Canonicalization(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "LOCAL_CONTRACT_OPERATION_FAILED",
            },
        }
    }
}

fn market_error_to_api(error: &MarketError) -> ApiError {
    match error {
        MarketError::QuotePreviewMismatch => ApiError {
            status: StatusCode::CONFLICT,
            code: "QUOTE_PREVIEW_CHANGED",
        },
        MarketError::QuoteApprovalMismatch => ApiError {
            status: StatusCode::CONFLICT,
            code: "QUOTE_APPROVAL_CHANGED",
        },
        MarketError::SupplyComparisonMismatch => ApiError {
            status: StatusCode::CONFLICT,
            code: "MARKET_SUPPLY_COMPARISON_CHANGED",
        },
        MarketError::SupplyQuoteNoEligibleOffer => ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "MARKET_NO_ELIGIBLE_OFFER",
        },
        _ => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "LOCAL_CONTRACT_OPERATION_FAILED",
        },
    }
}

fn trade_error_to_api(error: &TradeError) -> ApiError {
    match error {
        TradeError::DatasetDigestMismatch => ApiError {
            status: StatusCode::CONFLICT,
            code: "TRADE_DATASET_CHANGED",
        },
        TradeError::InstrumentNotFound => ApiError {
            status: StatusCode::NOT_FOUND,
            code: "TRADE_INSTRUMENT_NOT_FOUND",
        },
        TradeError::IncompatibleInstruments | TradeError::InvalidRequest => ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "TRADE_REQUEST_REJECTED",
        },
        TradeError::ArithmeticOverflow | TradeError::Data(_) | TradeError::Digest(_) => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "TRADE_CONTRACT_FAILED",
        },
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body: Value = json!({
            "schema_version": API_ERROR_SCHEMA_VERSION,
            "status": "error",
            "code": self.code,
            "message": API_ERROR_MESSAGE,
        });
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod data_query_capacity_tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use axum::http::StatusCode;
    use serde_json::{Value, json};

    use super::{
        AgentSessionManager, FleetSimulationStoreV1, GpudiffScheduleStatusV1,
        MAX_ACTIVE_DATA_QUERIES, MarketHistoryStore, ServiceState, app,
    };
    use crate::auth::AuthState;

    #[tokio::test]
    async fn full_shared_data_capacity_rejects_queries_and_synthetic_release()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let temporary = tempfile::tempdir()?;
        let data_root = temporary.path().join("empty-data");
        std::fs::create_dir_all(&data_root)?;

        let auth = AuthState::generate()?;
        let bearer = auth.bearer().to_owned();
        let (shutdown, _) = tokio::sync::watch::channel(false);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let registered = Arc::new(tokio::sync::Notify::new());
        let execution_gate = Arc::new(tokio::sync::Semaphore::new(0));
        let state = Arc::new(ServiceState {
            auth,
            shutdown,
            instance_id: "synthetic-data-capacity-test".to_owned(),
            port: address.port(),
            data_root,
            market_history: MarketHistoryStore::open(&crate::storage::paths_for_home(Some(
                temporary.path(),
            ))?)
            .await?,
            active_queries: Arc::new(std::sync::Mutex::new(HashMap::new())),
            query_slots: Arc::new(tokio::sync::Semaphore::new(MAX_ACTIVE_DATA_QUERIES)),
            fleet_simulation: FleetSimulationStoreV1::default(),
            eval_runs: tokio::sync::Mutex::new(HashMap::new()),
            agent_sessions: AgentSessionManager::default(),
            agent_lifecycle_lock: tokio::sync::Mutex::new(()),
            gpudiff_schedule_status: Arc::new(tokio::sync::Mutex::new(
                GpudiffScheduleStatusV1::default(),
            )),
            gpudiff_schedule_task: tokio::sync::Mutex::new(None),
            test_query_gate: Some((Arc::clone(&registered), Arc::clone(&execution_gate))),
        });
        let server = tokio::spawn(async move { axum::serve(listener, app(state)).await });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let base = format!("http://{address}");
        let mut active_queries = Vec::new();

        for query_id in ["active-query-one", "active-query-two"] {
            let query_client = client.clone();
            let query_base = base.clone();
            let query_bearer = bearer.clone();
            active_queries.push(tokio::spawn(async move {
                query_client
                    .post(format!("{query_base}/api/v1/data/query"))
                    .bearer_auth(query_bearer)
                    .json(&json!({
                        "query_id": query_id,
                        "request": {
                            "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                            "sql": "SELECT gpu_model FROM dataset",
                            "format": "json",
                            "max_rows": 10,
                        }
                    }))
                    .send()
                    .await
            }));
            tokio::time::timeout(Duration::from_secs(1), registered.notified()).await?;
        }

        let busy_query = client
            .post(format!("{base}/api/v1/data/query"))
            .bearer_auth(&bearer)
            .json(&json!({
                "query_id": "over-capacity-query",
                "request": {
                    "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                    "sql": "SELECT gpu_model FROM dataset",
                    "format": "json",
                    "max_rows": 10,
                }
            }))
            .send()
            .await?;
        assert_eq!(busy_query.status(), StatusCode::TOO_MANY_REQUESTS);
        let busy_query_body: Value = busy_query.json().await?;
        assert_eq!(busy_query_body["code"], "DATA_QUERY_BUSY");

        let busy_release = client
            .get(format!("{base}/api/v1/data/synthetic-release"))
            .bearer_auth(&bearer)
            .send()
            .await?;
        assert_eq!(busy_release.status(), StatusCode::TOO_MANY_REQUESTS);
        let busy_release_body: Value = busy_release.json().await?;
        assert_eq!(busy_release_body["code"], "DATA_QUERY_BUSY");

        server.abort();
        for query in active_queries {
            query.abort();
            let _ = query.await;
        }
        let _ = server.await;
        Ok(())
    }
}
