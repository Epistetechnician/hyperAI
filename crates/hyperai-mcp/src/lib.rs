//! Read-only MCP stdio bridge to `HyperAI`'s authenticated local service.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use agent_client_protocol::{ConnectTo, role::mcp};
use agent_client_protocol_rmcp::McpServerExt;
use hyperai_acp::{MCP_TOOL_NAMES_V4, MCP_TOOL_NAMES_V5, MCP_TOOL_NAMES_V6};
use reqwest::{Client, Method, Response, Url};
use schemars::JsonSchema;
use serde::Deserialize;

const MAX_RESPONSE_BYTES: u64 = 1_048_576;
const MAX_INPUT_BYTES: usize = 16 * 1024;
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Environment-provided local route and per-session read-only capability.
#[derive(Clone)]
pub struct McpApi {
    client: Client,
    base: Url,
    capability: Arc<String>,
}

/// MCP bridge configuration was missing or outside the loopback boundary.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Required values were missing, oversized, or invalid.
    #[error("invalid local HyperAI MCP configuration")]
    InvalidConfiguration,
    /// The local service denied the request or exceeded the output limit.
    #[error("HyperAI service request was denied or exceeded its response limit")]
    ServiceRequest,
    /// The local service returned a bounded, versioned error response.
    #[error("HyperAI local service returned HTTP {status} ({code})")]
    ServiceResponse {
        /// HTTP response status code.
        status: u16,
        /// Validated stable service error identifier.
        code: String,
    },
}

#[derive(JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyInput {}

#[derive(JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataQueryInput {
    dataset_id: String,
    sql: String,
    format: String,
    max_rows: Option<usize>,
}

#[derive(JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
struct FleetPreviewInput {
    node_id: String,
    operation: String,
}

#[derive(JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
struct TradeSeriesInput {
    instrument_id: String,
}

#[derive(JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
struct TradeScenarioInput {
    expected_dataset_digest: String,
    primary_instrument_id: String,
    comparison_instrument_id: String,
    price_shock_bps: i32,
    availability_delta_units: i64,
}

#[derive(JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
struct SupplyQuotePreviewInput {
    comparison_digest: String,
}

impl McpApi {
    /// Construct a bridge that can only address the local IPv4 service.
    ///
    /// # Errors
    ///
    /// Returns an error when either required environment variable is missing
    /// or its value fails local endpoint or capability validation.
    pub fn from_environment() -> Result<Self, McpError> {
        let endpoint =
            std::env::var("HYPERAI_MCP_ENDPOINT").map_err(|_| McpError::InvalidConfiguration)?;
        let capability =
            std::env::var("HYPERAI_MCP_CAPABILITY").map_err(|_| McpError::InvalidConfiguration)?;
        Self::new(&endpoint, capability)
    }

    /// Construct a local bridge from explicit values, primarily for contract tests.
    ///
    /// # Errors
    ///
    /// Returns an error when the endpoint is not an unauthenticated IPv4
    /// loopback URL or the scoped capability has an invalid size.
    pub fn new(endpoint: &str, capability: String) -> Result<Self, McpError> {
        let base = Url::parse(endpoint).map_err(|_| McpError::InvalidConfiguration)?;
        if base.scheme() != "http"
            || base.host_str() != Some("127.0.0.1")
            || base.port().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
            || capability.len() < 32
            || capability.len() > 256
        {
            return Err(McpError::InvalidConfiguration);
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| McpError::InvalidConfiguration)?;
        Ok(Self {
            client,
            base,
            capability: Arc::new(capability),
        })
    }

    /// Build the versioned fixed allowlist of `HyperAI` read-only tools.
    #[must_use]
    #[allow(clippy::too_many_lines)] // Keep the complete fixed tool allowlist reviewable in one place.
    pub fn server(
        self,
    ) -> agent_client_protocol::mcp_server::McpServer<
        mcp::Client,
        impl agent_client_protocol::RunWithConnectionTo<mcp::Client>,
    > {
        let server = agent_client_protocol::mcp_server::McpServer::<mcp::Client>::builder(
            "hyperai-readonly-v6",
        )
        .instructions(
            "Read-only local HyperAI buyer supply comparisons, workload traces, catalogs, bounded synthetic queries, Fleet previews, Eval reports, and Trade research. Market observations are advisory. No tools can approve deals, execute jobs, change Fleet state, run evaluations, move funds, or contact providers.",
        );

        let data_catalog = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[0],
            "List the local source-verified Data catalog and provenance.",
            async move |_: EmptyInput, _context| data_catalog.get("data/catalog").await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let data_query = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[1],
            "Run one bounded read-only query against a listed local synthetic dataset.",
            async move |input: DataQueryInput, _context| data_query.query(input).await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let fleet_inventory = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[2],
            "Read the fixed synthetic Fleet inventory.",
            async move |_: EmptyInput, _context| fleet_inventory.get("fleet/inventory").await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let fleet_doctor = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[3],
            "Read deterministic local Fleet fixture checks; no provider is contacted.",
            async move |_: EmptyInput, _context| fleet_doctor.get("fleet/doctor").await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let fleet_preview = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[4],
            "Build an advisory digest-bound preview for a fixed synthetic Fleet node.",
            async move |input: FleetPreviewInput, _context| {
                if input.node_id.len() > 128 {
                    return Err(agent_client_protocol::util::internal_error(
                        "invalid tool input",
                    ));
                }
                let operation = match input.operation.as_str() {
                    "attach" | "launch" | "workload_start" | "cancel" | "terminate" => {
                        input.operation
                    }
                    _ => {
                        return Err(agent_client_protocol::util::internal_error(
                            "invalid tool input",
                        ));
                    }
                };
                fleet_preview
                    .post(
                        "fleet/operation-preview",
                        serde_json::json!({ "node_id": input.node_id, "operation": operation }),
                    )
                    .await
            },
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let eval_catalog = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[5],
            "Read the fixed synthetic Eval catalog without executing an evaluation.",
            async move |_: EmptyInput, _context| eval_catalog.get("eval/catalog").await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let eval_reports = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[6],
            "Read bounded aggregate reports already present in the local session cache.",
            async move |_: EmptyInput, _context| eval_reports.get("eval/reports").await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let trade_catalog = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[7],
            "Read synthetic compute-market instruments and their provenance.",
            async move |_: EmptyInput, _context| trade_catalog.get("trade/catalog").await,
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let trade_series = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[8],
            "Analyze one listed synthetic Trade series.",
            async move |input: TradeSeriesInput, _context| {
                let id = safe_segment(&input.instrument_id)?;
                trade_series.get(&format!("trade/series/{id}")).await
            },
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let trade_scenario_api = self.clone();
        let server = server.tool_fn(
            MCP_TOOL_NAMES_V4[9],
            "Calculate a bounded synthetic price and availability research scenario.",
            async move |input: TradeScenarioInput, _context| {
                if input.primary_instrument_id.len() > 128
                    || input.comparison_instrument_id.len() > 128
                {
                    return Err(agent_client_protocol::util::internal_error(
                        "invalid tool input",
                    ));
                }
                trade_scenario_api
                    .post(
                        "trade/scenario",
                        serde_json::json!({
                            "expected_dataset_digest": input.expected_dataset_digest,
                            "primary_instrument_id": input.primary_instrument_id,
                            "comparison_instrument_id": input.comparison_instrument_id,
                            "price_shock_bps": input.price_shock_bps,
                            "availability_delta_units": input.availability_delta_units,
                        }),
                    )
                    .await
            },
            agent_client_protocol_rmcp::tool_fn!(),
        );

        let workload_trace = self.clone();
        let server = server
            .tool_fn(
                MCP_TOOL_NAMES_V4[10],
                "Read the digest-bound local synthetic workload-to-receipt trace. Context from Data, Eval, Fleet, and Trade stays source-labelled and non-authoritative.",
                async move |_: EmptyInput, _context| {
                    workload_trace.get("market/workload-trace").await
                },
                agent_client_protocol_rmcp::tool_fn!(),
            );

        let seller_failure_preview = self.clone();
        let server = server
            .tool_fn(
                MCP_TOOL_NAMES_V4[11],
                "Read the fixed synthetic seller-attributed failure report. It is not independent execution, correctness, refund, or payment evidence.",
                async move |_: EmptyInput, _context| {
                    seller_failure_preview
                        .get("market/workload-trace/failure-preview")
                        .await
                },
                agent_client_protocol_rmcp::tool_fn!(),
            );

        let inconclusive_preview = self.clone();
        let server = server
            .tool_fn(
                MCP_TOOL_NAMES_V4[12],
                "Read the fixed synthetic inconclusive-verifier and mock dispute-hold preview. It is not an actual verifier decision, dispute, funds hold, or payment authorization.",
                async move |_: EmptyInput, _context| {
                    inconclusive_preview
                        .get("market/workload-trace/inconclusive-preview")
                        .await
                },
                agent_client_protocol_rmcp::tool_fn!(),
            );

        let supply_comparison_api = self.clone();
        let supply_comparison = server
            .tool_fn(
                MCP_TOOL_NAMES_V5[13],
                "Read the digest-bound fixed synthetic buyer supply comparison. Advisory price and availability never affect offer eligibility; no registry or seller is contacted.",
                async move |_: EmptyInput, _context| {
                    supply_comparison_api.get("market/supply-comparison").await
                },
                agent_client_protocol_rmcp::tool_fn!(),
            );

        let supply_quote_preview = self.clone();
        supply_comparison
            .tool_fn(
                MCP_TOOL_NAMES_V6[14],
                "Preview the existing synthetic seller quote for one exact displayed supply-comparison digest. This is read-only and does not approve the quote, contact a seller, execute compute, or move funds.",
                async move |input: SupplyQuotePreviewInput, _context| {
                    let valid_digest = input.comparison_digest.len() == 64
                        && input
                            .comparison_digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
                    if !valid_digest {
                        return Err(agent_client_protocol::util::internal_error(
                            "invalid tool input",
                        ));
                    }
                    supply_quote_preview
                        .post(
                            "market/supply-quote-preview",
                            serde_json::json!({
                                "comparison_digest": input.comparison_digest
                            }),
                        )
                        .await
                },
                agent_client_protocol_rmcp::tool_fn!(),
            )
            .build()
    }

    async fn query(&self, input: DataQueryInput) -> Result<String, agent_client_protocol::Error> {
        if input.sql.len() > MAX_INPUT_BYTES
            || input.dataset_id.len() > 128
            || !matches!(input.format.as_str(), "json" | "csv")
        {
            return Err(agent_client_protocol::util::internal_error(
                "invalid tool input",
            ));
        }
        let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        self.post(
            "data/query",
            serde_json::json!({
                "query_id": format!("agent-{}-{sequence}", std::process::id()),
                "request": {
                    "dataset_id": input.dataset_id,
                    "sql": input.sql,
                    "format": input.format,
                    "max_rows": input.max_rows,
                }
            }),
        )
        .await
    }

    async fn get(&self, path: &str) -> Result<String, agent_client_protocol::Error> {
        self.request(Method::GET, path, None).await
    }

    async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<String, agent_client_protocol::Error> {
        let bytes = serde_json::to_vec(&body)
            .map_err(|_| agent_client_protocol::util::internal_error("invalid tool input"))?;
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(agent_client_protocol::util::internal_error(
                "tool input limit exceeded",
            ));
        }
        self.request(Method::POST, path, Some(bytes)).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<String, agent_client_protocol::Error> {
        let url = self
            .base
            .join(&format!("api/v1/agent/tools/{path}"))
            .map_err(|_| {
                agent_client_protocol::util::internal_error("local service unavailable")
            })?;
        let mut request = self
            .client
            .request(method, url)
            .bearer_auth(self.capability.as_str());
        if let Some(bytes) = body {
            request = request
                .header("content-type", "application/json")
                .body(bytes);
        }
        let response = request.send().await.map_err(|_| {
            agent_client_protocol::util::internal_error("local service unavailable")
        })?;
        bounded_json_response(response)
            .await
            .map_err(|error| acp_request_error(&error))
    }
}

async fn bounded_json_response(response: Response) -> Result<String, McpError> {
    let status = response.status();
    if response
        .content_length()
        .is_none_or(|length| length > MAX_RESPONSE_BYTES)
    {
        return Err(McpError::ServiceRequest);
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| McpError::ServiceRequest)?;
    if u64::try_from(bytes.len()).map_err(|_| McpError::ServiceRequest)? > MAX_RESPONSE_BYTES {
        return Err(McpError::ServiceRequest);
    }
    if !status.is_success() {
        return Err(service_response_error(status.as_u16(), &bytes));
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| McpError::ServiceRequest)
}

fn service_response_error(status: u16, bytes: &[u8]) -> McpError {
    let body =
        serde_json::from_slice::<serde_json::Value>(bytes).unwrap_or(serde_json::Value::Null);
    let normalized = hyperai_application::normalize_interface_error_v1(status, &body);
    McpError::ServiceResponse {
        status: normalized.status,
        code: normalized.code,
    }
}

fn acp_request_error(error: &McpError) -> agent_client_protocol::Error {
    agent_client_protocol::util::internal_error(error.to_string())
}

fn safe_segment(value: &str) -> Result<&str, agent_client_protocol::Error> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(agent_client_protocol::util::internal_error(
            "invalid tool input",
        ));
    }
    Ok(value)
}

/// Serve the read-only tool allowlist over MCP stdio.
///
/// # Errors
///
/// Returns an error when the local capability configuration is invalid or
/// the MCP stdio connection fails.
pub async fn serve_stdio() -> Result<(), agent_client_protocol::Error> {
    let api = McpApi::from_environment()
        .map_err(|_| agent_client_protocol::util::internal_error("invalid MCP configuration"))?;
    api.server()
        .connect_to(agent_client_protocol::Stdio::new())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperai_acp::{MCP_TOOL_NAMES_V2, MCP_TOOL_NAMES_V3};

    async fn assert_bound_trace_owner_mcp_parity(
        client: &reqwest::Client,
        base: &str,
        bearer: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let preview: serde_json::Value = client
            .get(format!("{base}/api/v1/data/synthetic/preview"))
            .bearer_auth(bearer)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        client
            .post(format!("{base}/api/v1/data/synthetic/seed"))
            .bearer_auth(bearer)
            .header("Idempotency-Key", "trace-error-parity-seed-v1")
            .json(&serde_json::json!({
                "preview_digest": preview["preview_digest"],
            }))
            .send()
            .await?
            .error_for_status()?;

        let response = client
            .get(format!("{base}/api/v1/market/workload-trace"))
            .bearer_auth(bearer)
            .send()
            .await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let owner_trace: serde_json::Value = response.json().await?;
        let data_reference = owner_trace["references"]
            .as_array()
            .and_then(|references| {
                references
                    .iter()
                    .find(|reference| reference["owner_id"] == "hyperai-data")
            })
            .ok_or("bound owner trace omitted the Data context reference")?;
        assert_eq!(data_reference["status"], "bound");
        let bound_digest = data_reference["digest"]
            .as_str()
            .ok_or("bound owner trace omitted the verified Data digest")?;
        assert_eq!(
            owner_trace["intent"]["body"]["data_references"][0]["digest"],
            bound_digest
        );

        let api = McpApi::new(&format!("{base}/"), bearer.to_owned())?;
        let mcp_trace: serde_json::Value =
            serde_json::from_str(&api.get("market/workload-trace").await?)?;
        assert_eq!(mcp_trace, owner_trace);

        let failure_response = client
            .get(format!(
                "{base}/api/v1/market/workload-trace/failure-preview"
            ))
            .bearer_auth(bearer)
            .send()
            .await?;
        assert_eq!(failure_response.status(), reqwest::StatusCode::OK);
        let owner_failure: serde_json::Value = failure_response.json().await?;
        assert_eq!(
            owner_failure["state_slice"],
            "hyperai.bazaar.workload_trace_seller_failure.v1"
        );
        assert_eq!(
            owner_failure["claim_ceiling"],
            "digest-bound local synthetic seller-attributed failure projection"
        );
        let mcp_failure: serde_json::Value =
            serde_json::from_str(&api.get("market/workload-trace/failure-preview").await?)?;
        assert_eq!(mcp_failure, owner_failure);

        let inconclusive_response = client
            .get(format!(
                "{base}/api/v1/market/workload-trace/inconclusive-preview"
            ))
            .bearer_auth(bearer)
            .send()
            .await?;
        assert_eq!(inconclusive_response.status(), reqwest::StatusCode::OK);
        let owner_inconclusive: serde_json::Value = inconclusive_response.json().await?;
        assert_eq!(
            owner_inconclusive["state_slice"],
            "hyperai.bazaar.workload_trace_inconclusive_served.v1"
        );
        let mcp_inconclusive: serde_json::Value = serde_json::from_str(
            &api.get("market/workload-trace/inconclusive-preview")
                .await?,
        )?;
        assert_eq!(mcp_inconclusive, owner_inconclusive);
        Ok(())
    }

    async fn assert_unbound_trace_owner_mcp_parity(
        client: &reqwest::Client,
        base: &str,
        bearer: &str,
        expected_status: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = client
            .get(format!("{base}/api/v1/market/workload-trace"))
            .bearer_auth(bearer)
            .send()
            .await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let owner_trace: serde_json::Value = response.json().await?;
        let data_reference = owner_trace["references"]
            .as_array()
            .and_then(|references| {
                references
                    .iter()
                    .find(|reference| reference["owner_id"] == "hyperai-data")
            })
            .ok_or("unbound owner trace omitted the Data context reference")?;
        assert_eq!(data_reference["status"], expected_status);
        assert!(data_reference["digest"].is_null());
        assert_eq!(
            owner_trace["intent"]["body"]["data_references"],
            serde_json::json!([])
        );
        assert!(!owner_trace.to_string().contains("synthetic-a100-01"));
        assert!(!owner_trace.to_string().contains("preview_digest"));

        let api = McpApi::new(&format!("{base}/"), bearer.to_owned())?;
        let mcp_trace: serde_json::Value =
            serde_json::from_str(&api.get("market/workload-trace").await?)?;
        assert_eq!(mcp_trace, owner_trace);
        if expected_status == "unavailable" {
            assert!(
                !owner_trace
                    .to_string()
                    .contains("unavailable-test-manifest.json")
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    async fn assert_permission_limited_trace_mcp_parity(
        client: &reqwest::Client,
        base: &str,
        bearer: &str,
        sample: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(sample)?.permissions();
        let original_mode = permissions.mode();
        permissions.set_mode(original_mode & !0o444);
        std::fs::set_permissions(sample, permissions)?;

        let direct_read = std::fs::read(sample);
        let denied = direct_read
            .as_ref()
            .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
        eprintln!(
            "Data permission probe: {}",
            if denied {
                "PermissionDenied"
            } else {
                "readable; denied not exercised"
            }
        );
        let parity = if denied {
            assert_unbound_trace_owner_mcp_parity(client, base, bearer, "denied").await
        } else {
            assert_bound_trace_owner_mcp_parity(client, base, bearer).await
        };

        let mut restored = std::fs::metadata(sample)?.permissions();
        restored.set_mode(original_mode);
        std::fs::set_permissions(sample, restored)?;
        parity
    }

    #[tokio::test]
    async fn data_query_preserves_versioned_error_from_local_service()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::Duration;

        use hyperai_local_service::{AuthState, lifecycle, server, storage};

        let temporary = tempfile::tempdir()?;
        let home = temporary.path().join("home");
        let service_home = home.clone();
        let service = tokio::spawn(async move { server::serve(Some(&service_home), 0).await });
        let paths = storage::paths_for_home(Some(&home))?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()?;
        let mut ready = None;
        for _ in 0..100 {
            if let Ok(record) = lifecycle::read_record(&paths)
                && client
                    .get(format!("http://{}/healthz", record.address))
                    .send()
                    .await
                    .is_ok_and(|response| response.status().is_success())
            {
                let bearer = AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))?;
                ready = Some((record, bearer));
                break;
            }
            if service.is_finished() {
                service.await??;
                return Err("local service exited before readiness".into());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let (record, bearer) = ready.ok_or("local service did not become ready")?;
        let api = McpApi::new(&format!("http://{}/", record.address), bearer.clone())?;
        let result = api
            .query(DataQueryInput {
                dataset_id: "hyperai.synthetic_market_observations_json.v1".to_owned(),
                sql: "SELECT read_file('/tmp/hyperai-parity-nonexistent-canary') FROM dataset"
                    .to_owned(),
                format: "json".to_owned(),
                max_rows: Some(100),
            })
            .await;
        let Err(error) = result else {
            return Err("expected the service error response".into());
        };

        assert_eq!(
            error.data,
            Some(serde_json::json!(
                "HyperAI local service returned HTTP 422 (DATA_QUERY_REJECTED)"
            ))
        );
        assert!(!format!("{error:?}").contains("read_file"));
        client
            .post(format!("http://{}/api/v1/service/stop", record.address))
            .bearer_auth(&bearer)
            .json(&serde_json::json!({}))
            .send()
            .await?
            .error_for_status()?;
        service.await??;
        Ok(())
    }

    #[tokio::test]
    async fn workload_trace_mcp_matches_changed_data_owner_route()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::Duration;

        use hyperai_local_service::{AuthState, lifecycle, server, storage};

        let temporary = tempfile::tempdir()?;
        let home = temporary.path().join("home");
        let service_home = home.clone();
        let service = tokio::spawn(async move { server::serve(Some(&service_home), 0).await });
        let paths = storage::paths_for_home(Some(&home))?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()?;
        let mut ready = None;
        for _ in 0..100 {
            if let Ok(record) = lifecycle::read_record(&paths)
                && client
                    .get(format!("http://{}/healthz", record.address))
                    .send()
                    .await
                    .is_ok_and(|response| response.status().is_success())
            {
                let bearer = AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))?;
                ready = Some((record, bearer));
                break;
            }
            if service.is_finished() {
                service.await??;
                return Err("local service exited before readiness".into());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let (record, bearer) = ready.ok_or("local service did not become ready")?;
        let base = format!("http://{}", record.address);
        assert_bound_trace_owner_mcp_parity(&client, &base, &bearer).await?;

        let sample = home.join("data/synthetic-market-observations.json");
        #[cfg(unix)]
        assert_permission_limited_trace_mcp_parity(&client, &base, &bearer, &sample).await?;

        let mut sample_bytes = std::fs::read(&sample)?;
        sample_bytes.push(b' ');
        std::fs::write(&sample, &sample_bytes)?;
        assert_unbound_trace_owner_mcp_parity(&client, &base, &bearer, "conflicting").await?;

        std::fs::remove_file(&sample)?;
        assert_unbound_trace_owner_mcp_parity(&client, &base, &bearer, "missing").await?;

        let oversized_manifest = home
            .join("data/manifests")
            .join("unavailable-test-manifest.json");
        std::fs::write(&oversized_manifest, vec![b'x'; 16 * 1024 + 1])?;
        assert_unbound_trace_owner_mcp_parity(&client, &base, &bearer, "unavailable").await?;

        client
            .post(format!("{base}/api/v1/service/stop"))
            .bearer_auth(&bearer)
            .json(&serde_json::json!({}))
            .send()
            .await?
            .error_for_status()?;
        service.await??;
        Ok(())
    }

    #[test]
    fn service_error_forwards_only_status_and_validated_code() {
        let error = service_response_error(
            422,
            br#"{"schema_version":"hyperai.interface.error.v1","status":"error","code":"DATA_QUERY_REJECTED","message":"request details must stay local"}"#,
        );

        assert_eq!(
            error.to_string(),
            "HyperAI local service returned HTTP 422 (DATA_QUERY_REJECTED)"
        );
        assert!(!error.to_string().contains("request details"));
    }

    #[test]
    fn acp_error_data_contains_status_and_code_without_service_message() {
        let error = acp_request_error(&service_response_error(
            422,
            br#"{"schema_version":"hyperai.interface.error.v1","status":"error","code":"DATA_QUERY_REJECTED","message":"private request detail"}"#,
        ));

        assert_eq!(
            error.data,
            Some(serde_json::json!(
                "HyperAI local service returned HTTP 422 (DATA_QUERY_REJECTED)"
            ))
        );
    }

    #[test]
    fn unknown_or_malformed_service_errors_use_generic_code() {
        for body in [
            br#"{"schema_version":"other.v1","status":"error","code":"DATA_QUERY_REJECTED"}"#.as_slice(),
            br#"{"schema_version":"hyperai.interface.error.v1","status":"error","code":"BAD\nCODE"}"#,
            b"not-json",
        ] {
            let error = service_response_error(500, body);
            assert_eq!(
                error.to_string(),
                "HyperAI local service returned HTTP 500 (UNKNOWN_ERROR)"
            );
        }
    }

    #[test]
    fn rejects_remote_and_credentialed_endpoints() {
        assert!(McpApi::new("https://example.com", "x".repeat(48)).is_err());
        assert!(McpApi::new("http://u:p@127.0.0.1:4444", "x".repeat(48)).is_err());
    }

    #[test]
    fn safe_segments_cannot_escape_route_prefix() -> Result<(), agent_client_protocol::Error> {
        assert_eq!(safe_segment("node_01")?, "node_01");
        assert!(safe_segment("../../status").is_err());
        Ok(())
    }

    #[test]
    fn rejects_extra_tool_arguments_and_uses_the_shared_fixed_tool_names() {
        assert_eq!(
            MCP_TOOL_NAMES_V2,
            [
                "hyperai_data_catalog",
                "hyperai_data_query",
                "hyperai_fleet_inventory",
                "hyperai_fleet_doctor",
                "hyperai_fleet_operation_preview",
                "hyperai_eval_catalog",
                "hyperai_eval_reports",
                "hyperai_trade_catalog",
                "hyperai_trade_series",
                "hyperai_trade_scenario",
                "hyperai_market_workload_trace",
            ]
        );
        assert_eq!(
            MCP_TOOL_NAMES_V3,
            [
                "hyperai_data_catalog",
                "hyperai_data_query",
                "hyperai_fleet_inventory",
                "hyperai_fleet_doctor",
                "hyperai_fleet_operation_preview",
                "hyperai_eval_catalog",
                "hyperai_eval_reports",
                "hyperai_trade_catalog",
                "hyperai_trade_series",
                "hyperai_trade_scenario",
                "hyperai_market_workload_trace",
                "hyperai_market_workload_trace_seller_failure_preview",
            ]
        );
        assert_eq!(
            MCP_TOOL_NAMES_V4,
            [
                "hyperai_data_catalog",
                "hyperai_data_query",
                "hyperai_fleet_inventory",
                "hyperai_fleet_doctor",
                "hyperai_fleet_operation_preview",
                "hyperai_eval_catalog",
                "hyperai_eval_reports",
                "hyperai_trade_catalog",
                "hyperai_trade_series",
                "hyperai_trade_scenario",
                "hyperai_market_workload_trace",
                "hyperai_market_workload_trace_seller_failure_preview",
                "hyperai_market_workload_trace_inconclusive_preview",
            ]
        );
        assert_eq!(
            MCP_TOOL_NAMES_V5,
            [
                "hyperai_data_catalog",
                "hyperai_data_query",
                "hyperai_fleet_inventory",
                "hyperai_fleet_doctor",
                "hyperai_fleet_operation_preview",
                "hyperai_eval_catalog",
                "hyperai_eval_reports",
                "hyperai_trade_catalog",
                "hyperai_trade_series",
                "hyperai_trade_scenario",
                "hyperai_market_workload_trace",
                "hyperai_market_workload_trace_seller_failure_preview",
                "hyperai_market_workload_trace_inconclusive_preview",
                "hyperai_market_supply_comparison",
            ]
        );
        assert_eq!(
            &MCP_TOOL_NAMES_V3[..MCP_TOOL_NAMES_V2.len()],
            MCP_TOOL_NAMES_V2
        );
        assert_eq!(
            &MCP_TOOL_NAMES_V4[..MCP_TOOL_NAMES_V3.len()],
            MCP_TOOL_NAMES_V3
        );
        assert_eq!(
            &MCP_TOOL_NAMES_V5[..MCP_TOOL_NAMES_V4.len()],
            MCP_TOOL_NAMES_V4
        );
        assert_eq!(hyperai_acp::MCP_TOOL_NAMES_V1.len(), 10);
        assert!(serde_json::from_value::<EmptyInput>(serde_json::json!({ "path": "/" })).is_err());
        assert!(
            serde_json::from_value::<DataQueryInput>(serde_json::json!({
                "dataset_id": "fixture",
                "sql": "SELECT 1",
                "format": "json",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn v6_appends_quote_preview_and_rejects_approval_arguments() {
        assert_eq!(
            MCP_TOOL_NAMES_V6,
            [
                "hyperai_data_catalog",
                "hyperai_data_query",
                "hyperai_fleet_inventory",
                "hyperai_fleet_doctor",
                "hyperai_fleet_operation_preview",
                "hyperai_eval_catalog",
                "hyperai_eval_reports",
                "hyperai_trade_catalog",
                "hyperai_trade_series",
                "hyperai_trade_scenario",
                "hyperai_market_workload_trace",
                "hyperai_market_workload_trace_seller_failure_preview",
                "hyperai_market_workload_trace_inconclusive_preview",
                "hyperai_market_supply_comparison",
                "hyperai_market_supply_quote_preview",
            ]
        );
        assert_eq!(
            &MCP_TOOL_NAMES_V6[..MCP_TOOL_NAMES_V5.len()],
            MCP_TOOL_NAMES_V5
        );
        assert!(
            serde_json::from_value::<SupplyQuotePreviewInput>(serde_json::json!({
                "comparison_digest": "a".repeat(64),
                "approve": true
            }))
            .is_err()
        );
    }
}
