//! HyperAI-authored, deterministic ACP v1 peer for local contract tests.

use std::{
    io::{self, BufRead, Write},
    net::TcpStream,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use hyperai_acp::MCP_TOOL_NAMES_V6;
use serde_json::{Value, json};

const MCP_FRAME_LIMIT: usize = 1024 * 1024;
const MCP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

struct FixtureOptions {
    initialize: InitializeMode,
    prompt: PromptMode,
    close: CloseMode,
    probe_mcp_capability: bool,
    invoke_mcp_stdio: bool,
    workload_trace: WorkloadTraceMode,
}

enum WorkloadTraceMode {
    Normal,
    Conflict,
    Denied { expected_digest: Option<String> },
}

#[derive(Clone, Copy)]
enum InitializeMode {
    Normal,
    WrongVersion,
    BatchFrame,
    MalformedFrame,
    StdoutOverflow,
}

#[derive(Clone, Copy)]
enum PromptMode {
    Normal,
    WaitForCancel,
    StderrOverflow,
}

#[derive(Clone, Copy)]
enum CloseMode {
    Default,
    Supported,
    Stall,
}

impl FixtureOptions {
    fn from_args(args: &[String]) -> Self {
        let initialize = if has_arg(args, "--wrong-version") {
            InitializeMode::WrongVersion
        } else if has_arg(args, "--batch-on-initialize") {
            InitializeMode::BatchFrame
        } else if has_arg(args, "--malformed-on-initialize") {
            InitializeMode::MalformedFrame
        } else if has_arg(args, "--stdout-overflow-on-initialize") {
            InitializeMode::StdoutOverflow
        } else {
            InitializeMode::Normal
        };
        let prompt = if has_arg(args, "--wait-for-cancel") {
            PromptMode::WaitForCancel
        } else if has_arg(args, "--stderr-overflow-on-prompt") {
            PromptMode::StderrOverflow
        } else {
            PromptMode::Normal
        };
        let close = if has_arg(args, "--stall-close") {
            CloseMode::Stall
        } else if has_arg(args, "--close-support") {
            CloseMode::Supported
        } else {
            CloseMode::Default
        };
        let workload_trace = if has_arg(args, "--expect-workload-trace-conflict") {
            WorkloadTraceMode::Conflict
        } else if has_arg(args, "--expect-workload-trace-denied") {
            WorkloadTraceMode::Denied {
                expected_digest: argument_value(args, "--expected-workload-trace-digest"),
            }
        } else {
            WorkloadTraceMode::Normal
        };
        Self {
            initialize,
            prompt,
            close,
            probe_mcp_capability: has_arg(args, "--probe-mcp-capability"),
            invoke_mcp_stdio: has_arg(args, "--invoke-mcp-stdio"),
            workload_trace,
        }
    }
}

struct CapabilityProbe {
    port: u16,
    capability: String,
    supply_comparison_digest: Option<String>,
    fleet_preview_digest: Option<String>,
    workload_trace_status: Option<String>,
    mcp_sidecar: Option<McpSidecar>,
}

struct McpSidecar {
    child: Child,
    stdin: Option<ChildStdin>,
    responses: Receiver<Result<String, ()>>,
    reader: Option<JoinHandle<()>>,
    next_id: u64,
}

impl McpSidecar {
    fn start(server: &Value) -> Result<Self, Box<dyn std::error::Error>> {
        let command = server["command"]
            .as_str()
            .ok_or("MCP server command was missing")?;
        let args = server["args"]
            .as_array()
            .and_then(|values| values.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
            .ok_or("MCP server arguments were invalid")?;
        if args != ["stdio"] {
            return Err("MCP server arguments exceeded the fixed stdio contract".into());
        }
        let variables = server["env"]
            .as_array()
            .ok_or("MCP server environment was missing")?;
        if variables.len() != 2 {
            return Err("MCP server environment exceeded the fixed capability contract".into());
        }
        let endpoint = environment_value(variables, "HYPERAI_MCP_ENDPOINT")?;
        let capability = environment_value(variables, "HYPERAI_MCP_CAPABILITY")?;
        if !endpoint.starts_with("http://127.0.0.1:") || capability.len() < 32 {
            return Err("MCP server configuration was outside the synthetic local contract".into());
        }

        let mut child = Command::new(command)
            .args(args)
            .current_dir(std::env::current_dir()?)
            .env_clear()
            .env("HYPERAI_MCP_ENDPOINT", endpoint)
            .env("HYPERAI_MCP_CAPABILITY", capability)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or("MCP sidecar stdin was unavailable")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("MCP sidecar stdout was unavailable")?;
        let (response_tx, responses) = mpsc::sync_channel(1);
        let reader = thread::spawn(move || {
            let mut reader = io::BufReader::new(stdout);
            loop {
                if let Ok(Some(line)) = read_bounded_line(&mut reader, MCP_FRAME_LIMIT) {
                    if response_tx.send(Ok(line)).is_err() {
                        break;
                    }
                } else {
                    let _ = response_tx.send(Err(()));
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin: Some(stdin),
            responses,
            reader: Some(reader),
            next_id: 1,
        })
    }

    fn verify_readonly_tools(
        &mut self,
        capability: &str,
        workload_trace: &WorkloadTraceMode,
        expected_workload_trace: Option<&Value>,
    ) -> Result<(String, String, Option<String>), Box<dyn std::error::Error>> {
        let initialized = self.request(
            "initialize",
            &json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "hyperai-synthetic-acp-peer",
                    "version": "0.1.0"
                }
            }),
        )?;
        if initialized["error"].is_object()
            || initialized["result"]["capabilities"]["tools"].is_null()
        {
            return Err("MCP sidecar did not initialize its tool capability".into());
        }
        self.notify("notifications/initialized")?;

        let listed = self.request("tools/list", &json!({}))?;
        let tools = listed["result"]["tools"]
            .as_array()
            .ok_or("MCP sidecar tool list was missing")?;
        let names = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        if names != MCP_TOOL_NAMES_V6 {
            return Err("MCP sidecar advertised a tool set outside the fixed allowlist".into());
        }

        let supply = self.request(
            "tools/call",
            &json!({
                "name": MCP_TOOL_NAMES_V6[13],
                "arguments": {}
            }),
        )?;
        if supply["error"].is_object() || supply["result"]["isError"] == true {
            return Err("MCP buyer supply comparison tool returned an error".into());
        }
        let supply_text = supply["result"]["content"]
            .as_array()
            .and_then(|content| content.first())
            .and_then(|block| block["text"].as_str())
            .ok_or("MCP buyer supply comparison tool returned no text result")?;
        let supply_comparison_digest = validate_supply_comparison_text(supply_text)?;

        let supply_quote = self.request(
            "tools/call",
            &json!({
                "name": MCP_TOOL_NAMES_V6[14],
                "arguments": {"comparison_digest": supply_comparison_digest}
            }),
        )?;
        if supply_quote["error"].is_object() || supply_quote["result"]["isError"] == true {
            return Err("MCP supply-to-quote preview tool returned an error".into());
        }
        let supply_quote_text = supply_quote["result"]["content"]
            .as_array()
            .and_then(|content| content.first())
            .and_then(|block| block["text"].as_str())
            .ok_or("MCP supply-to-quote tool returned no text result")?;
        validate_supply_quote_preview_text(supply_quote_text, &supply_comparison_digest)?;

        if matches!(workload_trace, WorkloadTraceMode::Normal) {
            let response = self.request(
                "tools/call",
                &json!({
                    "name": MCP_TOOL_NAMES_V6[0],
                    "arguments": {}
                }),
            )?;
            if response["error"].is_object() {
                return Err("MCP Data catalog tool returned a protocol error".into());
            }
            if response["result"]["isError"] == true {
                return Err("MCP Data catalog tool returned an error result".into());
            }
            let text = response["result"]["content"]
                .as_array()
                .and_then(|content| content.first())
                .and_then(|block| block["text"].as_str())
                .ok_or("MCP Data catalog tool returned no text result")?;
            if !text.contains("hyperai.synthetic_market_observations_json.v1") {
                return Err("MCP Data catalog tool did not return the synthetic catalog".into());
            }
            self.verify_data_query_rejected(capability)?;
        }
        let workload_trace_status =
            self.verify_workload_trace(capability, workload_trace, expected_workload_trace)?;
        if matches!(workload_trace, WorkloadTraceMode::Normal) {
            self.verify_seller_failure_preview()?;
            self.verify_inconclusive_preview()?;
        }
        let fleet_preview_digest = self.verify_fleet_preview()?;
        Ok((
            supply_comparison_digest,
            fleet_preview_digest,
            workload_trace_status,
        ))
    }

    fn verify_workload_trace(
        &mut self,
        capability: &str,
        workload_trace: &WorkloadTraceMode,
        expected_trace: Option<&Value>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let response = self.request(
            "tools/call",
            &json!({
                    "name": MCP_TOOL_NAMES_V6[10],
                "arguments": {}
            }),
        )?;
        if response["error"].is_object() || response["result"]["isError"] == true {
            return Err("MCP workload trace returned a protocol error".into());
        }
        let text = response["result"]["content"]
            .as_array()
            .and_then(|content| content.first())
            .and_then(|block| block["text"].as_str())
            .ok_or("MCP workload trace returned no text result")?;
        let wrapped: Value = serde_json::from_str(text)?;
        let trace: Value = match wrapped {
            Value::String(json) => serde_json::from_str(&json)?,
            value => value,
        };
        if matches!(workload_trace, WorkloadTraceMode::Conflict) {
            let expected = expected_trace
                .ok_or("conflicting MCP Workload Trace had no direct route result")?;
            validate_conflicting_workload_trace(&trace)?;
            if &trace != expected || trace.to_string().contains(capability) {
                return Err("MCP Workload Trace did not match its scoped route result".into());
            }
            return Ok(Some("conflicting".to_owned()));
        }
        if matches!(workload_trace, WorkloadTraceMode::Denied { .. }) {
            let expected =
                expected_trace.ok_or("denied MCP Workload Trace had no direct route result")?;
            let expected_digest = expected["digest"]
                .as_str()
                .ok_or("direct denied Workload Trace omitted its digest")?;
            validate_denied_workload_trace(&trace, expected_digest)?;
            if &trace != expected || trace.to_string().contains(capability) {
                return Err("MCP Workload Trace did not match its scoped route result".into());
            }
            return Ok(Some("denied".to_owned()));
        }
        if trace["schema_version"] != "hyperai.interface.workload-trace.v1"
            || trace["state_slice"] != "hyperai.bazaar.workload_trace_synthetic.v1"
            || trace["intent"]["body"]["workload_id"] != "workload-hyperai-local-synthetic-v1"
            || trace["digest"].as_str().is_none()
            || !trace["references"].as_array().is_some_and(|references| {
                references.iter().any(|reference| {
                    reference["owner_id"] == "hyperai-market"
                        && reference["status"] == "mock_verdict_verified"
                }) && references.iter().any(|reference| {
                    reference["owner_id"] == "hyperai-market"
                        && reference["status"] == "mock_disposition_release_to_seller"
                }) && references.iter().any(|reference| {
                    reference["owner_id"] == "hyperai-trade"
                        && reference["status"] == "advisory_market_data_only"
                })
            })
        {
            return Err("MCP workload trace exceeded its fixed synthetic projection".into());
        }
        Ok(None)
    }

    fn verify_seller_failure_preview(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.request(
            "tools/call",
            &json!({
                    "name": MCP_TOOL_NAMES_V6[11],
                "arguments": {}
            }),
        )?;
        if response["error"].is_object() || response["result"]["isError"] == true {
            return Err("MCP seller-failure preview returned a protocol error".into());
        }
        let text = response["result"]["content"]
            .as_array()
            .and_then(|content| content.first())
            .and_then(|block| block["text"].as_str())
            .ok_or("MCP seller-failure preview returned no text result")?;
        let wrapped: Value = serde_json::from_str(text)?;
        let trace: Value = match wrapped {
            Value::String(json) => serde_json::from_str(&json)?,
            value => value,
        };
        let failure = trace["references"]
            .as_array()
            .and_then(|references| {
                references
                    .iter()
                    .find(|reference| reference["status"] == "fulfillment_failed")
            })
            .ok_or("MCP seller-failure preview omitted its failure reference")?;
        if trace["state_slice"] != "hyperai.bazaar.workload_trace_seller_failure.v1"
            || failure["owner_id"] != "hyperai-market"
            || failure["claim_ceiling"]
                != "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence"
            || failure["digest"].as_str().is_none()
            || trace["references"].as_array().is_some_and(|references| {
                references.iter().any(|reference| {
                    matches!(
                        reference["record_id"].as_str(),
                        Some(
                            "synthetic-fulfillment-v1"
                                | "synthetic-mock-verifier-decision-v1"
                                | "synthetic-mock-disposition-v1"
                        )
                    )
                })
            })
        {
            return Err("MCP seller-failure preview exceeded its claim boundary".into());
        }
        Ok(())
    }

    fn verify_inconclusive_preview(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.request(
            "tools/call",
            &json!({
                    "name": MCP_TOOL_NAMES_V6[12],
                "arguments": {}
            }),
        )?;
        if response["error"].is_object() || response["result"]["isError"] == true {
            return Err("MCP inconclusive preview returned a protocol error".into());
        }
        let text = response["result"]["content"]
            .as_array()
            .and_then(|content| content.first())
            .and_then(|block| block["text"].as_str())
            .ok_or("MCP inconclusive preview returned no text result")?;
        let wrapped: Value = serde_json::from_str(text)?;
        let trace: Value = match wrapped {
            Value::String(json) => serde_json::from_str(&json)?,
            value => value,
        };
        let references = trace["references"]
            .as_array()
            .ok_or("MCP inconclusive preview omitted its references")?;
        let verifier = references
            .iter()
            .find(|reference| reference["status"] == "mock_verdict_inconclusive")
            .ok_or("MCP inconclusive preview omitted the mock verdict")?;
        let disposition = references
            .iter()
            .find(|reference| reference["status"] == "mock_disposition_hold_for_dispute")
            .ok_or("MCP inconclusive preview omitted the mock disposition")?;
        if trace["state_slice"] != "hyperai.bazaar.workload_trace_inconclusive_served.v1"
            || trace["claim_ceiling"]
                != "digest-bound local synthetic inconclusive mock-outcome preview"
            || trace["digest"].as_str().is_none()
            || verifier["digest"].as_str().is_none()
            || verifier["claim_ceiling"]
                != "local_fixture_verdict_only_not_independent_correctness_evidence"
            || !disposition["digest"].is_null()
            || disposition["link_basis"] != "covered_by_the_enclosing_local_market_receipt_digest"
            || disposition["claim_ceiling"] != "no_payment_or_entitlement_claim"
        {
            return Err("MCP inconclusive preview exceeded its claim boundary".into());
        }
        Ok(())
    }

    fn verify_fleet_preview(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.request(
            "tools/call",
            &json!({
                    "name": MCP_TOOL_NAMES_V6[4],
                "arguments": {
                    "node_id": "fixture-node-atlas",
                    "operation": "launch"
                }
            }),
        )?;
        if response["error"].is_object() || response["result"]["isError"] == true {
            return Err("MCP Fleet preview returned a protocol error".into());
        }
        let text = response["result"]["content"]
            .as_array()
            .and_then(|content| content.first())
            .and_then(|block| block["text"].as_str())
            .ok_or("MCP Fleet preview returned no text result")?;
        let wrapped: Value = serde_json::from_str(text)?;
        let preview: Value = match wrapped {
            Value::String(json) => serde_json::from_str(&json)?,
            value => value,
        };
        let response_shape = match &preview {
            Value::Object(fields) => format!(
                "object fields [{}]",
                fields
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Array(_) => "array".to_owned(),
            Value::String(_) => "string".to_owned(),
            Value::Number(_) => "number".to_owned(),
            Value::Bool(_) => "boolean".to_owned(),
            Value::Null => "null".to_owned(),
        };
        let schema_matches = preview["schema_version"] == "hyperai.fleet.operation-preview.v1";
        let status_matches = preview["preview_status"] == "preview_ready";
        let authority_matches = preview["authority"] == "not_authorized";
        let boundary_matches =
            preview["effect_boundary"] == "advisory_only_does_not_invoke_or_reserve";
        let no_command = preview.get("command").is_none();
        let no_reservation = preview.get("reservation_id").is_none();
        if !schema_matches
            || !status_matches
            || !authority_matches
            || !boundary_matches
            || !no_command
            || !no_reservation
        {
            return Err(format!(
                "MCP Fleet preview contract mismatch ({response_shape}; schema={schema_matches}, status={status_matches}, authority={authority_matches}, boundary={boundary_matches}, no_command={no_command}, no_reservation={no_reservation})"
            )
            .into());
        }
        let digest = preview["preview_digest"]
            .as_str()
            .filter(|value| {
                value.len() == 64
                    && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && value.bytes().all(|byte| !byte.is_ascii_uppercase())
            })
            .ok_or("MCP Fleet preview omitted its canonical digest")?
            .to_owned();
        if preview["node_id"] != "fixture-node-atlas" || preview["operation"] != "launch" {
            return Err("MCP Fleet preview changed its requested target or intent".into());
        }
        Ok(digest)
    }

    fn verify_data_query_rejected(
        &mut self,
        capability: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sql = "SELECT read_file('/tmp/hyperai-parity-nonexistent-canary') FROM dataset";
        let response = self.request(
            "tools/call",
            &json!({
                    "name": MCP_TOOL_NAMES_V6[1],
                "arguments": {
                    "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                    "sql": sql,
                    "format": "json",
                    "max_rows": null
                }
            }),
        )?;
        let is_tool_error = response["error"].is_object() || response["result"]["isError"] == true;
        if !is_tool_error {
            return Err("MCP Data query did not preserve the local service denial".into());
        }
        let encoded = response.to_string();
        if !encoded.contains("HTTP 422 (DATA_QUERY_REJECTED)") {
            return Err("MCP Data query omitted the stable service status and code".into());
        }
        if encoded.contains(sql) || encoded.contains(capability) {
            return Err("MCP Data query error exposed SQL or the session capability".into());
        }
        Ok(())
    }

    fn verify_data_catalog_denied(
        &mut self,
        capability: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.request(
            "tools/call",
            &json!({
                    "name": MCP_TOOL_NAMES_V6[0],
                "arguments": {}
            }),
        )?;
        if !response["error"].is_object() && response["result"]["isError"] != true {
            return Err("MCP sidecar retained Data access after capability revocation".into());
        }
        if response.to_string().contains(capability) {
            return Err("MCP error response exposed the revoked capability".into());
        }
        Ok(())
    }

    fn request(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let message = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let stdin = self.stdin.as_mut().ok_or("MCP sidecar stdin was closed")?;
        serde_json::to_writer(&mut *stdin, &message)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;

        let response = match self.responses.recv_timeout(MCP_RESPONSE_TIMEOUT) {
            Ok(Ok(line)) => serde_json::from_str::<Value>(&line)?,
            Ok(Err(())) => return Err("MCP sidecar closed or exceeded its output limit".into()),
            Err(_) => return Err("MCP sidecar response timed out".into()),
        };
        if response["id"] != id {
            return Err("MCP sidecar response did not match its request".into());
        }
        Ok(response)
    }

    fn notify(&mut self, method: &str) -> Result<(), Box<dyn std::error::Error>> {
        let message = json!({ "jsonrpc": "2.0", "method": method });
        let stdin = self.stdin.as_mut().ok_or("MCP sidecar stdin was closed")?;
        serde_json::to_writer(&mut *stdin, &message)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stdin.take();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(status) = self.child.try_wait()? {
                if !status.success() {
                    return Err("MCP sidecar exited unsuccessfully after stdio close".into());
                }
                break;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return Err("MCP sidecar did not stop after stdio close".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        Ok(())
    }
}

impl Drop for McpSidecar {
    fn drop(&mut self) {
        self.stdin.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    maximum_bytes: usize,
) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete MCP stdio frame",
            ));
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(buffer.len(), |index| index + 1);
        if bytes.len().saturating_add(count) > maximum_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP stdio frame exceeded the fixture limit",
            ));
        }
        bytes.extend_from_slice(&buffer[..count]);
        reader.consume(count);
        if newline.is_some() {
            break;
        }
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "MCP stdio frame was not UTF-8"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = run();
    if let Err(error) = &result {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if has_arg(&args, "--probe-mcp-capability") {
            let _ = std::fs::write(
                std::env::current_dir()?.join("mcp-capability-probe.failure"),
                error.to_string(),
            );
        }
    }
    result
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if has_arg(&args, "--sleep-child") {
        loop {
            std::thread::park();
        }
    }
    spawn_descendant_if_requested(&args)?;
    let options = FixtureOptions::from_args(&args);
    serve(&options)
}

fn spawn_descendant_if_requested(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = args
        .windows(2)
        .find(|values| values[0] == "--spawn-child-pid-file")
        .map(|values| &values[1])
    {
        let child = Command::new(std::env::current_exe()?)
            .arg("--sleep-child")
            .spawn()?;
        std::fs::write(path, child.id().to_string())?;
        drop(child);
    }
    Ok(())
}

fn serve(options: &FixtureOptions) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut pending_prompt_id = None;
    let mut capability_probe = None;
    for line in stdin.lock().lines() {
        let line = line?;
        if line.len() > 64 * 1024 {
            return Err("synthetic ACP frame exceeded the fixture limit".into());
        }
        let request: Value = serde_json::from_str(&line)?;
        let method = request["method"].as_str().unwrap_or_default();
        let id = request.get("id").cloned();
        let result = match method {
            "initialize" => initialize(&request, options, &mut stdout)?,
            "session/new" => create_session(&request, options, &mut capability_probe)?,
            "session/prompt" => prompt(
                id.clone(),
                options,
                capability_probe.as_ref(),
                &mut pending_prompt_id,
                &mut stdout,
            )?,
            "session/cancel" => cancel_pending_prompt(&mut pending_prompt_id, &mut stdout)?,
            "session/close" if matches!(options.close, CloseMode::Stall) => continue,
            "session/close" => {
                if options.probe_mcp_capability {
                    let probe = capability_probe
                        .as_mut()
                        .ok_or("session/close arrived without an MCP capability probe")?;
                    verify_capability_revoked(probe)?;
                }
                Some(json!({}))
            }
            _ if id.is_none() => None,
            _ => return Err(format!("unexpected ACP request method: {method}").into()),
        };
        if let (Some(id), Some(result)) = (id, result) {
            write_response(&mut stdout, &id, &result)?;
        }
    }
    Ok(())
}

fn initialize(
    request: &Value,
    options: &FixtureOptions,
    stdout: &mut impl Write,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    if request["params"]["protocolVersion"] != 1 {
        return Err("fixture expected ACP v1".into());
    }
    match options.initialize {
        InitializeMode::BatchFrame => {
            stdout.write_all(b"[]\n")?;
            stdout.flush()?;
        }
        InitializeMode::MalformedFrame => {
            stdout.write_all(b"{malformed json}\n")?;
            stdout.flush()?;
        }
        InitializeMode::StdoutOverflow => {
            let oversized_stdout = vec![b'x'; 8 * 1024 * 1024 + 1];
            stdout.write_all(&oversized_stdout)?;
            stdout.flush()?;
        }
        InitializeMode::Normal | InitializeMode::WrongVersion => {}
    }
    let agent_capabilities = if matches!(options.close, CloseMode::Supported | CloseMode::Stall) {
        json!({ "sessionCapabilities": { "close": {} } })
    } else {
        json!({})
    };
    let protocol_version = if matches!(options.initialize, InitializeMode::WrongVersion) {
        2
    } else {
        1
    };
    Ok(Some(json!({
        "protocolVersion": protocol_version,
        "agentCapabilities": agent_capabilities
    })))
}

fn create_session(
    request: &Value,
    options: &FixtureOptions,
    capability_probe: &mut Option<CapabilityProbe>,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let servers = request["params"]["mcpServers"]
        .as_array()
        .ok_or("session/new omitted mcpServers")?;
    if servers.len() != 1
        || servers[0]["name"] != "hyperai-readonly"
        || servers[0]["args"] != json!(["stdio"])
        || servers[0]["env"].as_array().map(Vec::len) != Some(2)
        || servers[0]["command"].as_str().is_none()
    {
        return Err("session/new did not contain the fixed HyperAI MCP stdio server".into());
    }
    if options.probe_mcp_capability {
        *capability_probe = Some(probe_mcp_capability(
            &servers[0],
            options.invoke_mcp_stdio,
            &options.workload_trace,
        )?);
    }
    Ok(Some(json!({ "sessionId": "synthetic-session-v1" })))
}

fn validate_supply_comparison_text(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let wrapped: Value = serde_json::from_str(text)?;
    let comparison: Value = match wrapped {
        Value::String(json) => serde_json::from_str(&json)?,
        value => value,
    };
    let schema = comparison["body"]["schema_version"]
        .as_str()
        .unwrap_or("<missing>");
    let selected_offer_id = comparison["body"]["selected_offer_id"]
        .as_str()
        .unwrap_or("<missing>");
    if schema != "hyperai.compute-market.supply-comparison.v1" || selected_offer_id != "offer-gpu-b"
    {
        return Err(format!(
            "MCP buyer supply comparison returned an unexpected contract (schema={schema:?}, selected_offer_id={selected_offer_id:?})"
        )
        .into());
    }
    comparison["digest"]
        .as_str()
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_owned)
        .ok_or_else(|| "MCP buyer supply comparison omitted a SHA-256 digest".into())
}

fn validate_supply_quote_preview_text(
    text: &str,
    comparison_digest: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapped: Value = serde_json::from_str(text)?;
    let preview: Value = match wrapped {
        Value::String(json) => serde_json::from_str(&json)?,
        value => value,
    };
    let body = &preview["body"];
    let quote_preview = &body["quote_preview"];
    let quote = &quote_preview["body"]["signed_quote"]["quote"];
    let selected_offer_id = body["selected_offer_id"].as_str().unwrap_or("<missing>");
    let quote_selected_offer_id = quote_preview["body"]["selected_offer_id"]
        .as_str()
        .unwrap_or("<missing>");
    let digest_is_sha256 = |value: &Value| {
        value.as_str().is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    };
    if body["schema_version"] != "hyperai.compute-market.supply-quote-preview.v1"
        || body["state_slice"] != "hyperai.bazaar.supply_to_quote.v1"
        || body["comparison_digest"] != comparison_digest
        || selected_offer_id != "offer-gpu-b"
        || quote_selected_offer_id != selected_offer_id
        || body["selected_offer_digest"] != quote["offer_digest"]
        || body["catalog_digest"] != quote_preview["body"]["catalog"]["digest"]
        || body["buyer_request_digest"] != quote["request_digest"]
        || quote["quoted_total_microusd"].as_u64().unwrap_or(u64::MAX)
            > quote["amount_cap_microusd"].as_u64().unwrap_or(0)
        || !digest_is_sha256(&preview["digest"])
        || !digest_is_sha256(&quote_preview["digest"])
    {
        return Err("MCP supply-to-quote preview returned an unexpected binding".into());
    }
    Ok(())
}

fn verify_normal_workload_trace_capability(
    port: u16,
    capability: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (catalog_status, catalog_body) = local_request(
        port,
        capability,
        "GET",
        "/api/v1/agent/tools/data/catalog",
        "",
    )?;
    if catalog_status != 200 || catalog_body.is_empty() {
        return Err("scoped MCP capability could not read the fixed Data catalog".into());
    }

    let (preview_status, preview_body) = local_request(
        port,
        capability,
        "GET",
        "/api/v1/agent/tools/market/workload-trace/inconclusive-preview",
        "",
    )?;
    if preview_status != 200 || preview_body.contains(capability) {
        return Err("scoped MCP capability could not read the fixed inconclusive preview".into());
    }
    let failure_path = "/api/v1/agent/tools/market/workload-trace/failure-preview";
    let (failure_status, failure_body) = local_request(port, capability, "GET", failure_path, "")?;
    if failure_status != 200 || failure_body.contains(capability) {
        return Err("scoped MCP capability could not read the fixed seller-failure preview".into());
    }
    let failure_trace: Value = serde_json::from_str(&failure_body)?;
    if failure_trace["state_slice"] != "hyperai.bazaar.workload_trace_seller_failure.v1" {
        return Err("scoped MCP capability received the wrong seller-failure preview".into());
    }

    for (method, path) in [
        ("GET", "/api/v1/market/workload-trace"),
        ("GET", "/api/v1/market/workload-trace/failure-preview"),
        ("GET", "/api/v1/market/workload-trace/inconclusive-preview"),
        ("POST", "/api/v1/market/quote-approve"),
        ("POST", "/api/v1/market/quote-complete"),
        (
            "POST",
            "/api/v1/agent/tools/market/workload-trace/failure-preview",
        ),
        (
            "POST",
            "/api/v1/agent/tools/market/workload-trace/inconclusive-preview",
        ),
        ("GET", "/api/v1/agent/tools/unknown"),
    ] {
        let (status, _) = local_request(port, capability, method, path, "{}")?;
        if status != 401 {
            return Err(format!(
                "scoped MCP capability reached denied Workload Trace route {method} {path}"
            )
            .into());
        }
    }
    Ok(())
}

fn probe_mcp_capability(
    server: &Value,
    invoke_mcp_stdio: bool,
    workload_trace: &WorkloadTraceMode,
) -> Result<CapabilityProbe, Box<dyn std::error::Error>> {
    let variables = server["env"]
        .as_array()
        .ok_or("MCP server env was missing")?;
    let endpoint = environment_value(variables, "HYPERAI_MCP_ENDPOINT")?;
    let capability = environment_value(variables, "HYPERAI_MCP_CAPABILITY")?;
    let port = endpoint
        .strip_prefix("http://127.0.0.1:")
        .ok_or("MCP endpoint was outside IPv4 loopback")?
        .trim_end_matches('/')
        .parse::<u16>()?;

    let direct_workload_trace = match workload_trace {
        WorkloadTraceMode::Conflict => {
            let (status, body) = local_request(
                port,
                capability,
                "GET",
                "/api/v1/agent/tools/market/workload-trace",
                "",
            )?;
            if status != 200 || body.contains(capability) {
                return Err(
                    "scoped capability Workload Trace did not return the conflict trace".into(),
                );
            }
            let trace: Value = serde_json::from_str(&body)?;
            validate_conflicting_workload_trace(&trace)?;
            Some(trace)
        }
        WorkloadTraceMode::Denied { expected_digest } => {
            let expected_digest = expected_digest
                .as_deref()
                .ok_or("denied Workload Trace probe omitted its owner digest")?;
            let (status, body) = local_request(
                port,
                capability,
                "GET",
                "/api/v1/agent/tools/market/workload-trace",
                "",
            )?;
            if status != 200 {
                return Err(
                    "scoped capability Workload Trace did not return the denied trace".into(),
                );
            }
            if body.contains(capability) {
                return Err("denied Workload Trace exposed the scoped capability".into());
            }
            let trace: Value = serde_json::from_str(&body)?;
            validate_denied_workload_trace(&trace, expected_digest)?;
            Some(trace)
        }
        WorkloadTraceMode::Normal => {
            verify_normal_workload_trace_capability(port, capability)?;
            None
        }
    };

    let (denied_status, _) = local_request(
        port,
        capability,
        "POST",
        "/api/v1/fleet/simulation/transition",
        "{}",
    )?;
    if denied_status != 401 {
        return Err("scoped MCP capability reached an owner-only Fleet route".into());
    }

    let mut mcp_sidecar = if invoke_mcp_stdio {
        Some(McpSidecar::start(server)?)
    } else {
        None
    };
    let (supply_comparison_digest, fleet_preview_digest, workload_trace_status) =
        if let Some(sidecar) = &mut mcp_sidecar {
            let (supply_digest, fleet_digest, trace_status) = sidecar.verify_readonly_tools(
                capability,
                workload_trace,
                direct_workload_trace.as_ref(),
            )?;
            (Some(supply_digest), Some(fleet_digest), trace_status)
        } else {
            (None, None, None)
        };
    Ok(CapabilityProbe {
        port,
        capability: capability.to_owned(),
        supply_comparison_digest,
        fleet_preview_digest,
        workload_trace_status,
        mcp_sidecar,
    })
}

fn verify_capability_revoked(
    probe: &mut CapabilityProbe,
) -> Result<(), Box<dyn std::error::Error>> {
    let (status, _) = local_request(
        probe.port,
        &probe.capability,
        "GET",
        "/api/v1/agent/tools/data/catalog",
        "",
    )?;
    if status != 401 {
        return Err("scoped MCP capability remained usable after disconnect".into());
    }
    let mcp_tool_revoked = if let Some(sidecar) = &mut probe.mcp_sidecar {
        sidecar.verify_data_catalog_denied(&probe.capability)?;
        sidecar.close()?;
        true
    } else {
        false
    };
    std::fs::write(
        std::env::current_dir()?.join("mcp-capability-probe.report"),
        if mcp_tool_revoked {
            let supply_digest = probe
                .supply_comparison_digest
                .as_deref()
                .ok_or("MCP supply comparison report had no validated digest")?;
            let digest = probe
                .fleet_preview_digest
                .as_deref()
                .ok_or("MCP preview report had no validated digest")?;
            let trace_status = probe.workload_trace_status.as_deref().unwrap_or("ok");
            let query_status = if probe.workload_trace_status.is_some() {
                "not_run"
            } else {
                "422:DATA_QUERY_REJECTED"
            };
            format!(
                "supply_comparison_digest={supply_digest}\nfleet_preview=preview_ready:not_authorized:advisory_only_does_not_invoke_or_reserve\nfleet_preview_digest={digest}\nquery_rejected={query_status}\nworkload_trace={trace_status}\nrevoked=401\nmcp_after_revoke=denied\n"
            )
        } else {
            "revoked=401\n".to_owned()
        },
    )?;
    Ok(())
}

fn environment_value<'a>(
    variables: &'a [Value],
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    variables
        .iter()
        .find(|variable| variable["name"] == name)
        .and_then(|variable| variable["value"].as_str())
        .ok_or_else(|| "MCP server environment was incomplete".into())
}

fn validate_denied_workload_trace(
    trace: &Value,
    expected_digest: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let data_reference = trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("denied Workload Trace omitted the Data reference")?;
    if trace["schema_version"] != "hyperai.interface.workload-trace.v1"
        || trace["state_slice"] != "hyperai.bazaar.workload_trace_synthetic.v1"
        || trace["digest"] != expected_digest
        || data_reference["status"] != "denied"
        || data_reference["link_basis"] != "local_filesystem_permission_denied"
        || data_reference["claim_ceiling"]
            != "local_read_permission_denied_only_no_policy_entitlement_claim"
        || !data_reference["digest"].is_null()
        || trace["intent"]["body"]["data_references"] != json!([])
        || trace
            .to_string()
            .contains("synthetic-market-observations.json")
    {
        return Err("Workload Trace exceeded the denied-state claim ceiling".into());
    }
    Ok(())
}

fn validate_conflicting_workload_trace(trace: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let data_reference = trace["references"]
        .as_array()
        .and_then(|references| {
            references
                .iter()
                .find(|reference| reference["owner_id"] == "hyperai-data")
        })
        .ok_or("conflicting Workload Trace omitted the Data reference")?;
    if trace["schema_version"] != "hyperai.interface.workload-trace.v1"
        || trace["state_slice"] != "hyperai.bazaar.workload_trace_synthetic.v1"
        || trace["intent"]["body"]["workload_id"] != "workload-hyperai-local-synthetic-v1"
        || trace["digest"].as_str().is_none()
        || data_reference["status"] != "conflicting"
        || data_reference["link_basis"] != "local_catalog_manifest_or_content_integrity_rejected"
        || data_reference["claim_ceiling"] != "local_integrity_conflict_only_no_computation_claim"
        || !data_reference["digest"].is_null()
        || trace["intent"]["body"]["data_references"] != json!([])
        || trace
            .to_string()
            .contains("synthetic-market-observations.json")
    {
        return Err("Workload Trace exceeded the conflicting-state claim ceiling".into());
    }
    Ok(())
}

fn argument_value(args: &[String], expected: &str) -> Option<String> {
    let index = args.iter().position(|argument| argument == expected)?;
    args.get(index + 1).cloned()
}

fn local_request(
    port: u16,
    capability: &str,
    method: &str,
    path: &str,
    body: &str,
) -> Result<(u16, String), Box<dyn std::error::Error>> {
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {capability}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let status = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or("local HyperAI response status was invalid")?
        .parse::<u16>()?;
    let body = response
        .split_once("\r\n\r\n")
        .map_or("", |(_, body)| body)
        .to_owned();
    Ok((status, body))
}

fn prompt(
    id: Option<Value>,
    options: &FixtureOptions,
    capability_probe: Option<&CapabilityProbe>,
    pending_prompt_id: &mut Option<Value>,
    stdout: &mut impl Write,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    match options.prompt {
        PromptMode::StderrOverflow => {
            io::stderr().write_all(b"ACP_STDERR_CANARY_DO_NOT_RETAIN")?;
            let oversized_stderr = vec![b'x'; 64 * 1024 + 1];
            io::stderr().write_all(&oversized_stderr)?;
            io::stderr().flush()?;
            Ok(None)
        }
        PromptMode::WaitForCancel => {
            *pending_prompt_id = id;
            send_agent_text(stdout, "Synthetic prompt is pending.")?;
            Ok(None)
        }
        PromptMode::Normal => {
            let text = if matches!(&options.workload_trace, WorkloadTraceMode::Conflict) {
                let observed = capability_probe
                    .and_then(|probe| probe.workload_trace_status.as_deref())
                    .ok_or("ACP fixture had no validated conflicting Workload Trace")?;
                if observed != "conflicting" {
                    return Err("ACP fixture received an unexpected Workload Trace status".into());
                }
                "Synthetic ACP fixture observed Workload Trace Data status conflicting."
            } else if matches!(&options.workload_trace, WorkloadTraceMode::Denied { .. }) {
                let observed = capability_probe
                    .and_then(|probe| probe.workload_trace_status.as_deref())
                    .ok_or("ACP fixture had no validated denied Workload Trace")?;
                if observed != "denied" {
                    return Err("ACP fixture received an unexpected denied trace status".into());
                }
                "Synthetic ACP fixture observed Workload Trace Data status denied."
            } else {
                "Synthetic ACP response."
            };
            send_agent_text(stdout, text)?;
            Ok(Some(json!({ "stopReason": "end_turn" })))
        }
    }
}

fn cancel_pending_prompt(
    pending_prompt_id: &mut Option<Value>,
    stdout: &mut impl Write,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    if let Some(prompt_id) = pending_prompt_id.take() {
        write_response(stdout, &prompt_id, &json!({ "stopReason": "cancelled" }))?;
    }
    Ok(None)
}

fn write_response(
    writer: &mut impl Write,
    id: &Value,
    result: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(
        &mut *writer,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn has_arg(args: &[String], expected: &str) -> bool {
    args.iter().any(|arg| arg == expected)
}

fn send_agent_text(writer: &mut impl Write, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(
        &mut *writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "synthetic-session-v1",
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": { "type": "text", "text": text }
                }
            }
        }),
    )?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
