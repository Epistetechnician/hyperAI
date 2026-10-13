//! Keyboard-driven terminal client for the local `HyperAI` service.

use std::{
    io,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use hyperai_market::{LocalMarketHistoryEntryV1, LocalMarketHistoryPageV1};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use thiserror::Error;

const WORKSPACES: [&str; 13] = [
    "Overview",
    "Market",
    "Deals",
    "Data",
    "Inference",
    "Fleet",
    "Eval",
    "Evidence",
    "Activity",
    "Trade",
    "Agents & Access",
    "Workload Trace",
    "Connections",
];
const FLEET_PREVIEW_OPERATIONS: [&str; 5] =
    ["attach", "launch", "workload_start", "cancel", "terminate"];
const DEFAULT_DATA_QUERY: &str =
    "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
const MAX_DATA_QUERY_BYTES: usize = 4 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DataQueryAction {
    Changed,
    Cancelled,
    Submit(String),
    Empty,
    LimitReached,
    Exit,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarketQuoteAction {
    Preview,
    Approve,
    Complete,
    PreviewRequired,
    ApprovalAlreadyRecorded,
    CompletionAlreadyRecorded,
    ApprovalRequired,
}

#[derive(Debug, Clone, Default)]
struct MarketQuoteLifecycleState {
    preview: Option<Value>,
    approval: Option<Value>,
    completion: Option<Value>,
    message: String,
}

#[derive(Debug, Default)]
struct MarketHistoryState {
    entries: Vec<LocalMarketHistoryEntryV1>,
    next_cursor: Option<String>,
    page_digests: Vec<String>,
    error: Option<String>,
    message: String,
}

impl MarketHistoryState {
    fn accept_page(&mut self, page: LocalMarketHistoryPageV1) {
        self.entries.extend(page.body.entries);
        self.next_cursor = page.body.next_cursor;
        self.page_digests.push(page.digest.to_string());
        self.error = None;
        self.message = if self.next_cursor.is_some() {
            "More local records are available. Press n to load the next page.".to_owned()
        } else {
            "End of the available local history pages.".to_owned()
        };
    }

    fn render_value(&self) -> Value {
        json!({
            "entries": self.entries,
            "next_cursor": self.next_cursor,
            "page_digests": self.page_digests,
            "error": self.error,
            "message": self.message,
        })
    }
}

impl MarketQuoteLifecycleState {
    fn action_for_key(&self, key: KeyEvent) -> Option<MarketQuoteAction> {
        match key.code {
            KeyCode::Enter => Some(MarketQuoteAction::Preview),
            KeyCode::Char('a' | 'c') if self.completion.is_some() => {
                Some(MarketQuoteAction::CompletionAlreadyRecorded)
            }
            KeyCode::Char('a') if self.approval.is_some() => {
                Some(MarketQuoteAction::ApprovalAlreadyRecorded)
            }
            KeyCode::Char('a') if self.preview.is_some() => Some(MarketQuoteAction::Approve),
            KeyCode::Char('a') => Some(MarketQuoteAction::PreviewRequired),
            KeyCode::Char('c') if self.preview.is_some() && self.approval.is_some() => {
                Some(MarketQuoteAction::Complete)
            }
            KeyCode::Char('c') => Some(MarketQuoteAction::ApprovalRequired),
            _ => None,
        }
    }

    fn handle_action(
        &mut self,
        action: MarketQuoteAction,
        client: &Client,
        root: &str,
        bearer_token: &str,
        comparison: &Value,
    ) {
        match action {
            MarketQuoteAction::Preview => {
                self.preview = None;
                self.approval = None;
                self.completion = None;
                self.message.clear();
                match load_market_quote_preview(client, root, bearer_token, comparison) {
                    Ok(preview) => {
                        self.preview = Some(preview);
                        "Synthetic quote preview ready. No approval has been created."
                            .clone_into(&mut self.message);
                    }
                    Err(error) => self.message = format!("Quote preview failed: {error}"),
                }
            }
            MarketQuoteAction::Approve => {
                let Some(preview) = self.preview.as_ref() else {
                    "Preview a quote before approving it.".clone_into(&mut self.message);
                    return;
                };
                match approve_market_quote(client, root, bearer_token, preview) {
                    Ok(approval) => {
                        self.approval = Some(approval);
                        "Synthetic buyer approval recorded. No seller was contacted and no funds moved."
                            .clone_into(&mut self.message);
                    }
                    Err(error) => self.message = format!("Synthetic approval failed: {error}"),
                }
            }
            MarketQuoteAction::Complete => {
                let (Some(preview), Some(approval)) =
                    (self.preview.as_ref(), self.approval.as_ref())
                else {
                    "Record a synthetic approval before mock completion."
                        .clone_into(&mut self.message);
                    return;
                };
                match complete_market_quote(client, root, bearer_token, preview, approval) {
                    Ok(completion) => {
                        self.completion = Some(completion);
                        "Mock fulfillment and receipt completed. No workload ran and no funds moved."
                            .clone_into(&mut self.message);
                    }
                    Err(error) => self.message = format!("Mock completion failed: {error}"),
                }
            }
            MarketQuoteAction::PreviewRequired => {
                "Press Enter to load the current synthetic quote before approval."
                    .clone_into(&mut self.message);
            }
            MarketQuoteAction::ApprovalAlreadyRecorded => {
                "Approval is already recorded for this preview; no second request was sent."
                    .clone_into(&mut self.message);
            }
            MarketQuoteAction::CompletionAlreadyRecorded => {
                "Mock completion is already recorded for this preview; no second request was sent."
                    .clone_into(&mut self.message);
            }
            MarketQuoteAction::ApprovalRequired => {
                "Review and approve the displayed quote before mock completion."
                    .clone_into(&mut self.message);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct DataQueryInput {
    sql: String,
    editing: bool,
}

impl Default for DataQueryInput {
    fn default() -> Self {
        Self {
            sql: DEFAULT_DATA_QUERY.to_owned(),
            editing: false,
        }
    }
}

impl DataQueryInput {
    fn begin_editing(&mut self) {
        self.editing = true;
    }

    fn handle_key(&mut self, key: KeyEvent) -> DataQueryAction {
        if !self.editing {
            return DataQueryAction::Ignored;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.editing = false;
            return DataQueryAction::Exit;
        }
        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                DataQueryAction::Cancelled
            }
            KeyCode::Enter => {
                self.editing = false;
                if self.sql.trim().is_empty() {
                    DataQueryAction::Empty
                } else {
                    DataQueryAction::Submit(self.sql.clone())
                }
            }
            KeyCode::Backspace => {
                if self.sql.pop().is_some() {
                    DataQueryAction::Changed
                } else {
                    DataQueryAction::Ignored
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.sql.clear();
                DataQueryAction::Changed
            }
            KeyCode::Char(character)
                if !character.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if self.sql.len().saturating_add(character.len_utf8()) > MAX_DATA_QUERY_BYTES {
                    DataQueryAction::LimitReached
                } else {
                    self.sql.push(character);
                    DataQueryAction::Changed
                }
            }
            _ => DataQueryAction::Ignored,
        }
    }
}

/// Errors returned while connecting to or rendering the local desk.
#[derive(Debug, Error)]
pub enum DeskError {
    /// An HTTP request to the local service failed.
    #[error("local service request failed: {0}")]
    Request(#[from] reqwest::Error),
    /// The local service returned its versioned JSON error envelope.
    #[error("local service returned HTTP {status} ({code}): {message}")]
    ServiceResponse {
        /// HTTP response status code.
        status: u16,
        /// Stable service error identifier.
        code: String,
        /// Bounded, control-character-free service message.
        message: String,
    },
    /// The local service returned JSON that violated the market contract.
    #[error("local service returned an invalid market response: {0}")]
    InvalidMarketResponse(&'static str),
    /// The terminal could not be initialized or read.
    #[error("terminal error: {0}")]
    Terminal(#[from] io::Error),
}

/// Connects to the local service and runs the terminal workspace until quit.
///
/// # Errors
///
/// Returns an error when the service request fails or terminal setup, input,
/// rendering, or restoration cannot complete.
#[allow(clippy::too_many_lines)] // The terminal event loop keeps key dispatch and terminal restoration together.
pub fn run(base_url: &str, bearer_token: &str) -> Result<(), DeskError> {
    let client = Client::builder().timeout(Duration::from_secs(25)).build()?;
    let root = base_url.trim_end_matches('/');
    let mut status = fetch_service_json(&client, root, bearer_token, "status")?;
    status["connection_inventory"] =
        fetch_service_json(&client, root, bearer_token, "connections")?;
    status["workload_trace"] =
        fetch_service_json(&client, root, bearer_token, "market/workload-trace")?;
    status["workload_trace_error"] = Value::Null;
    match fetch_service_json(&client, root, bearer_token, "market/supply-comparison") {
        Ok(comparison) => {
            status["supply_comparison"] = comparison;
            status["supply_comparison_error"] = Value::Null;
        }
        Err(error) => {
            status["supply_comparison"] = Value::Null;
            status["supply_comparison_error"] = Value::String(error.to_string());
        }
    }
    status["supply_quote_preview"] = Value::Null;
    status["supply_quote_approval"] = Value::Null;
    status["supply_quote_completion"] = Value::Null;
    status["supply_quote_message"] = Value::Null;
    let mut market_history = MarketHistoryState::default();
    match fetch_market_history_page(&client, root, bearer_token, None) {
        Ok(page) => market_history.accept_page(page),
        Err(error) => market_history.error = Some(error.to_string()),
    }
    let catalog = fetch_service_json(&client, root, bearer_token, "data/catalog")?;
    let models = fetch_service_json(&client, root, bearer_token, "data/models")?;
    let views = fetch_service_json(&client, root, bearer_token, "data/views")?;
    let fleet_inventory = fetch_service_json(&client, root, bearer_token, "fleet/inventory")?;
    let fleet_doctor = fetch_service_json(&client, root, bearer_token, "fleet/doctor")?;
    let mut fleet_simulation = fetch_fleet_simulation(&client, root, bearer_token)?;
    let eval_catalog = fetch_service_json(&client, root, bearer_token, "eval/catalog")?;
    let trade_catalog = fetch_service_json(&client, root, bearer_token, "trade/catalog")?;
    let mut agent_snapshot = fetch_service_json(&client, root, bearer_token, "agent/status")?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let _restore = TerminalRestore::new(|| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    });
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut selected = 0usize;
    let mut list_state = ListState::default().with_selected(Some(selected));
    let mut data_result = None;
    let mut data_query = DataQueryInput::default();
    let mut data_query_message = String::new();
    let mut eval_report = None;
    let mut trade_series = None;
    let mut trade_scenario = None;
    let mut trade_paper = None;
    let mut fleet_node_index = 0usize;
    let mut fleet_operation_index = 0usize;
    let mut fleet_preview: Option<Value> = None;
    let mut fleet_message = String::new();
    let mut market_quote = MarketQuoteLifecycleState::default();
    let mut query_sequence = 0_u64;
    let mut agent_prompt_draft: Option<String> = None;
    let mut agent_profile_lines = Vec::<String>::new();
    let mut agent_profile_line = String::new();
    let mut agent_profile_capture = false;
    let mut agent_message = String::new();
    let mut agent_prompt_rx: Option<Receiver<Result<Value, String>>> = None;

    loop {
        if let Some(receiver) = &agent_prompt_rx {
            match receiver.try_recv() {
                Ok(Ok(snapshot)) => {
                    agent_snapshot = snapshot;
                    "ACP prompt completed. Output remains in volatile session memory."
                        .clone_into(&mut agent_message);
                    agent_prompt_rx = None;
                }
                Ok(Err(error)) => {
                    agent_message = format!("ACP prompt failed: {error}");
                    agent_snapshot["status"] = Value::String("failed".to_owned());
                    agent_prompt_rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    "ACP prompt worker ended without a result.".clone_into(&mut agent_message);
                    agent_snapshot["status"] = Value::String("inconclusive".to_owned());
                    agent_prompt_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let mut render_status = status.clone();
        render_status["data_query_draft"] = Value::String(data_query.sql.clone());
        render_status["data_query_editing"] = Value::Bool(data_query.editing);
        render_status["data_query_message"] = Value::String(data_query_message.clone());
        render_status["agent"] = agent_snapshot.clone();
        render_status["agent_prompt_draft"] = agent_prompt_draft
            .as_ref()
            .map_or(Value::Null, |draft| Value::String(draft.clone()));
        render_status["agent_profile_capture"] = Value::Bool(agent_profile_capture);
        render_status["agent_profile_line"] = Value::String(agent_profile_line.clone());
        render_status["agent_profile_count"] = json!(agent_profile_lines.len());
        render_status["agent_message"] = Value::String(agent_message.clone());
        render_status["fleet_inventory"] = fleet_inventory.clone();
        render_status["fleet_doctor"] = fleet_doctor.clone();
        render_status["fleet_preview"] = fleet_preview.clone().unwrap_or(Value::Null);
        render_status["fleet_selected_node_index"] = json!(fleet_node_index);
        render_status["fleet_selected_operation"] =
            Value::String(FLEET_PREVIEW_OPERATIONS[fleet_operation_index].to_owned());
        render_status["fleet_message"] = Value::String(fleet_message.clone());
        render_status["supply_quote_preview"] = market_quote.preview.clone().unwrap_or(Value::Null);
        render_status["supply_quote_approval"] =
            market_quote.approval.clone().unwrap_or(Value::Null);
        render_status["supply_quote_completion"] =
            market_quote.completion.clone().unwrap_or(Value::Null);
        render_status["supply_quote_message"] = Value::String(market_quote.message.clone());
        render_status["market_history"] = market_history.render_value();
        terminal.draw(|frame| {
            render_workspace_with_data(
                frame,
                &render_status,
                &catalog,
                &models,
                &views,
                data_result.as_ref(),
                Some(&fleet_simulation),
                Some(&eval_catalog),
                eval_report.as_ref(),
                TradeWorkspaceData {
                    catalog: Some(&trade_catalog),
                    series: trade_series.as_ref(),
                    scenario: trade_scenario.as_ref(),
                    paper: trade_paper.as_ref(),
                },
                base_url,
                selected,
                &mut list_state,
            );
        })?;

        #[cfg(all(feature = "test-support", debug_assertions))]
        if std::env::var_os("HYPERAI_TUI_TEST_PANIC_AFTER_DRAW").is_some() {
            wait_for_test_panic_key()?;
            trigger_test_panic_after_first_draw();
        }

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if agent_profile_capture {
                match key.code {
                    KeyCode::Esc => {
                        agent_profile_capture = false;
                        agent_profile_lines.clear();
                        agent_profile_line.clear();
                        "Agent profile entry cancelled.".clone_into(&mut agent_message);
                    }
                    KeyCode::Enter => {
                        agent_profile_lines.push(std::mem::take(&mut agent_profile_line));
                        if agent_profile_lines.len() == 3 {
                            let name = agent_profile_lines[0].trim().to_owned();
                            let executable = agent_profile_lines[1].trim().to_owned();
                            let working_directory = agent_profile_lines[2].trim().to_owned();
                            agent_profile_capture = false;
                            agent_profile_lines.clear();
                            if name.is_empty()
                                || !std::path::Path::new(&executable).is_absolute()
                                || !std::path::Path::new(&working_directory).is_absolute()
                            {
                                "Profile rejected: provide a name and absolute executable and working-directory paths."
                                    .clone_into(&mut agent_message);
                            } else {
                                agent_snapshot["status"] = Value::String("connecting".to_owned());
                                let profile = json!({
                                    "name": name,
                                    "executable": executable,
                                    "working_directory": working_directory,
                                    "args": [],
                                    "environment": {},
                                });
                                agent_snapshot = post_service_json(
                                    &client,
                                    root,
                                    bearer_token,
                                    "agent/connect",
                                    &json!({ "profile": profile }),
                                )?;
                                "Connected to the selected local executable using ACP v1."
                                    .clone_into(&mut agent_message);
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        agent_profile_line.pop();
                    }
                    KeyCode::Char(character)
                        if agent_profile_line.len() + character.len_utf8() <= 4096 =>
                    {
                        agent_profile_line.push(character);
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(prompt) = &mut agent_prompt_draft {
                match key.code {
                    KeyCode::Esc => {
                        agent_prompt_draft = None;
                        "Prompt entry cancelled.".clone_into(&mut agent_message);
                    }
                    KeyCode::Backspace => {
                        prompt.pop();
                    }
                    KeyCode::Enter => {
                        let prompt = std::mem::take(prompt);
                        agent_prompt_draft = None;
                        if prompt.trim().is_empty() {
                            "Prompt must not be empty.".clone_into(&mut agent_message);
                        } else if agent_prompt_rx.is_none() {
                            let worker_client = client.clone();
                            let worker_root = root.to_owned();
                            let worker_bearer = bearer_token.to_owned();
                            let (sender, receiver) = mpsc::channel();
                            thread::spawn(move || {
                                let response = post_service_json(
                                    &worker_client,
                                    &worker_root,
                                    &worker_bearer,
                                    "agent/prompt",
                                    &json!({ "prompt": prompt }),
                                )
                                .map_err(|error| error.to_string());
                                let _ = sender.send(response);
                            });
                            agent_prompt_rx = Some(receiver);
                            agent_snapshot["status"] = Value::String("running".to_owned());
                            "Prompt sent. Press c to request cancellation."
                                .clone_into(&mut agent_message);
                        }
                    }
                    KeyCode::Char(character) if prompt.len() + character.len_utf8() <= 16_384 => {
                        prompt.push(character);
                    }
                    _ => {}
                }
                continue;
            }
            if WORKSPACES[selected] == "Data" && data_query.editing {
                match data_query.handle_key(key) {
                    DataQueryAction::Changed => data_query_message.clear(),
                    DataQueryAction::Cancelled => {
                        "Query edit cancelled; draft retained. Press Enter to run or e to edit."
                            .clone_into(&mut data_query_message);
                    }
                    DataQueryAction::Submit(sql) => {
                        data_result = None;
                        if catalog["datasets"]
                            .as_array()
                            .is_some_and(|datasets| !datasets.is_empty())
                        {
                            query_sequence = query_sequence.saturating_add(1);
                            match run_data_query(
                                &client,
                                base_url,
                                bearer_token,
                                &catalog,
                                query_sequence,
                                &sql,
                            ) {
                                Ok(result) => {
                                    data_result = Some(result);
                                    "Query completed. Results are kept in this TUI session only."
                                        .clone_into(&mut data_query_message);
                                }
                                Err(error) => {
                                    data_query_message = format!("Query failed: {error}");
                                }
                            }
                        } else {
                            "No local dataset is available. Seed the synthetic snapshot first."
                                .clone_into(&mut data_query_message);
                        }
                    }
                    DataQueryAction::Empty => {
                        data_result = None;
                        "Query must not be empty.".clone_into(&mut data_query_message);
                    }
                    DataQueryAction::LimitReached => {
                        data_query_message = format!(
                            "Query input is limited to {MAX_DATA_QUERY_BYTES} UTF-8 bytes."
                        );
                    }
                    DataQueryAction::Exit => break,
                    DataQueryAction::Ignored => {}
                }
                continue;
            }
            if key.code == KeyCode::Char('q')
                || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                break;
            } else if key.code == KeyCode::Esc {
                continue;
            }
            if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
                selected = (selected + 1) % WORKSPACES.len();
            } else if matches!(key.code, KeyCode::Up | KeyCode::Char('k')) {
                selected = selected.checked_sub(1).unwrap_or(WORKSPACES.len() - 1);
            } else if WORKSPACES[selected] == "Workload Trace" && key.code == KeyCode::Char('f') {
                load_workload_trace_preview(
                    &client,
                    root,
                    bearer_token,
                    &mut status,
                    WorkloadTracePreviewMode::SellerFailure,
                );
            } else if WORKSPACES[selected] == "Workload Trace" && key.code == KeyCode::Char('i') {
                load_workload_trace_preview(
                    &client,
                    root,
                    bearer_token,
                    &mut status,
                    WorkloadTracePreviewMode::Inconclusive,
                );
            } else if WORKSPACES[selected] == "Workload Trace" && key.code == KeyCode::Char('r') {
                load_workload_trace_preview(
                    &client,
                    root,
                    bearer_token,
                    &mut status,
                    WorkloadTracePreviewMode::Default,
                );
            } else if WORKSPACES[selected] == "Market"
                && let Some(action) = market_quote.action_for_key(key)
            {
                market_quote.handle_action(
                    action,
                    &client,
                    root,
                    bearer_token,
                    &status["supply_comparison"],
                );
            } else if WORKSPACES[selected] == "Deals" && key.code == KeyCode::Char('n') {
                load_next_market_history_page(&mut market_history, &client, root, bearer_token);
            } else if WORKSPACES[selected] == "Data" && key.code == KeyCode::Char('e') {
                if catalog["datasets"]
                    .as_array()
                    .is_some_and(|datasets| !datasets.is_empty())
                {
                    data_query.begin_editing();
                    data_result = None;
                    data_query_message.clear();
                } else {
                    "No local dataset is available. Seed the synthetic snapshot first."
                        .clone_into(&mut data_query_message);
                }
            } else if key.code == KeyCode::Enter && WORKSPACES[selected] == "Data" {
                data_result = None;
                if catalog["datasets"]
                    .as_array()
                    .is_some_and(|datasets| !datasets.is_empty())
                {
                    query_sequence = query_sequence.saturating_add(1);
                    match run_data_query(
                        &client,
                        base_url,
                        bearer_token,
                        &catalog,
                        query_sequence,
                        &data_query.sql,
                    ) {
                        Ok(result) => {
                            data_result = Some(result);
                            "Query completed. Results are kept in this TUI session only."
                                .clone_into(&mut data_query_message);
                        }
                        Err(error) => {
                            data_query_message = format!("Query failed: {error}");
                        }
                    }
                } else {
                    "No local dataset is available. Seed the synthetic snapshot first."
                        .clone_into(&mut data_query_message);
                }
            } else if key.code == KeyCode::Enter
                && WORKSPACES[selected] == "Fleet"
                && let Some((operation, gpu_units)) = next_fleet_step(&fleet_simulation)
            {
                fleet_simulation = advance_fleet_simulation(
                    &client,
                    root,
                    bearer_token,
                    &fleet_simulation,
                    operation,
                    gpu_units,
                )?;
            } else if WORKSPACES[selected] == "Fleet" && key.code == KeyCode::Char('n') {
                let node_count = fleet_inventory["nodes"].as_array().map_or(0, Vec::len);
                if node_count > 0 {
                    fleet_node_index = (fleet_node_index + 1) % node_count;
                    fleet_preview = None;
                    "Target changed; previous preview cleared.".clone_into(&mut fleet_message);
                } else {
                    "No synthetic Fleet target is available.".clone_into(&mut fleet_message);
                }
            } else if WORKSPACES[selected] == "Fleet" && key.code == KeyCode::Char('o') {
                fleet_operation_index =
                    (fleet_operation_index + 1) % FLEET_PREVIEW_OPERATIONS.len();
                fleet_preview = None;
                "Intent changed; previous preview cleared.".clone_into(&mut fleet_message);
            } else if WORKSPACES[selected] == "Fleet" && key.code == KeyCode::Char('v') {
                let selected_node = fleet_inventory["nodes"]
                    .as_array()
                    .and_then(|nodes| nodes.get(fleet_node_index));
                if let Some(node_id) = selected_node.and_then(|node| node["node_id"].as_str()) {
                    let operation = FLEET_PREVIEW_OPERATIONS[fleet_operation_index];
                    let expected_preview_digest = fleet_preview
                        .as_ref()
                        .filter(|preview| {
                            preview["node_id"] == node_id && preview["operation"] == operation
                        })
                        .and_then(|preview| preview["preview_digest"].as_str())
                        .map(str::to_owned);
                    let request = json!({
                        "node_id": node_id,
                        "operation": operation,
                        "expected_preview_digest": expected_preview_digest,
                    });
                    match post_service_json(
                        &client,
                        root,
                        bearer_token,
                        "fleet/operation-preview",
                        &request,
                    ) {
                        Ok(preview) => {
                            fleet_preview = Some(preview);
                            "Advisory preview ready. No resource was contacted or reserved."
                                .clone_into(&mut fleet_message);
                        }
                        Err(error) => fleet_message = format!("Preview failed: {error}"),
                    }
                } else {
                    "No synthetic Fleet target is available.".clone_into(&mut fleet_message);
                }
            } else if key.code == KeyCode::Enter && WORKSPACES[selected] == "Eval" {
                eval_report = Some(run_eval_qualification(
                    &client,
                    root,
                    bearer_token,
                    &eval_catalog,
                )?);
            } else if key.code == KeyCode::Enter && WORKSPACES[selected] == "Trade" {
                let (series, scenario, paper) = run_trade_research(
                    &client,
                    root,
                    bearer_token,
                    &trade_catalog,
                    TradeRunParameters {
                        primary_instrument_id: "a100-us-east",
                        comparison_instrument_id: "a100-us-west",
                        price_shock_bps: 0,
                        availability_delta_units: 0,
                        baseline_instrument_id: "a100-us-west",
                        requested_units: 1,
                    },
                )?;
                trade_series = Some(series);
                trade_scenario = Some(scenario);
                trade_paper = Some(paper);
            } else if WORKSPACES[selected] == "Agents & Access"
                && key.code == KeyCode::Char('p')
                && matches!(
                    agent_snapshot["status"].as_str(),
                    Some("disconnected" | "failed")
                )
            {
                agent_profile_capture = true;
                agent_profile_lines.clear();
                agent_profile_line.clear();
                "Enter profile name, absolute executable path, and working directory; press Enter after each."
                    .clone_into(&mut agent_message);
            } else if WORKSPACES[selected] == "Agents & Access"
                && key.code == KeyCode::Enter
                && !matches!(
                    agent_snapshot["status"].as_str(),
                    Some("disconnected" | "failed")
                )
                && agent_prompt_rx.is_none()
            {
                agent_prompt_draft = Some(String::new());
                "Type a prompt; Enter sends it once, Esc clears it.".clone_into(&mut agent_message);
            } else if WORKSPACES[selected] == "Agents & Access"
                && key.code == KeyCode::Char('c')
                && agent_prompt_rx.is_some()
            {
                agent_snapshot =
                    post_service_json(&client, root, bearer_token, "agent/cancel", &Value::Null)?;
                "Cancellation requested; local completion remains pending."
                    .clone_into(&mut agent_message);
            } else if WORKSPACES[selected] == "Agents & Access"
                && key.code == KeyCode::Char('x')
                && !matches!(
                    agent_snapshot["status"].as_str(),
                    Some("disconnected" | "failed")
                )
            {
                agent_snapshot = post_service_json(
                    &client,
                    root,
                    bearer_token,
                    "agent/disconnect",
                    &Value::Null,
                )?;
                agent_prompt_rx = None;
                "Local session disconnected.".clone_into(&mut agent_message);
            }
            list_state.select(Some(selected));
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkloadTracePreviewMode {
    Default,
    SellerFailure,
    Inconclusive,
}

fn load_workload_trace_preview(
    client: &Client,
    root: &str,
    bearer_token: &str,
    status: &mut Value,
    preview: WorkloadTracePreviewMode,
) {
    status["workload_trace"] = Value::Null;
    status["workload_trace_error"] = Value::Null;
    let route = workload_trace_preview_route(preview);
    match fetch_service_json(client, root, bearer_token, route) {
        Ok(trace) => status["workload_trace"] = trace,
        Err(error) => status["workload_trace_error"] = Value::String(error.to_string()),
    }
}

fn workload_trace_preview_route(preview: WorkloadTracePreviewMode) -> &'static str {
    match preview {
        WorkloadTracePreviewMode::Default => "market/workload-trace",
        WorkloadTracePreviewMode::SellerFailure => "market/workload-trace/failure-preview",
        WorkloadTracePreviewMode::Inconclusive => "market/workload-trace/inconclusive-preview",
    }
}

/// Renders the shared desk navigation and a read-only module summary.
///
/// This compatibility entrypoint preserves the original no-data renderer for
/// embedders and existing terminal contract checks.
pub fn render_workspace(
    frame: &mut Frame<'_>,
    status: &Value,
    base_url: &str,
    selected: usize,
    list_state: &mut ListState,
) {
    let empty = Value::Null;
    render_workspace_with_data(
        frame,
        status,
        &empty,
        &empty,
        &empty,
        None,
        None,
        None,
        None,
        TradeWorkspaceData::default(),
        base_url,
        selected,
        list_state,
    );
}

/// Renders the Fleet workspace with its current service-owned simulator view.
pub fn render_workspace_with_fleet(
    frame: &mut Frame<'_>,
    status: &Value,
    simulation: &Value,
    base_url: &str,
    selected: usize,
    list_state: &mut ListState,
) {
    let empty = Value::Null;
    render_workspace_with_data(
        frame,
        status,
        &empty,
        &empty,
        &empty,
        None,
        Some(simulation),
        None,
        None,
        TradeWorkspaceData::default(),
        base_url,
        selected,
        list_state,
    );
}

/// Renders the synthetic Eval workspace with its fixed catalog and report.
#[allow(clippy::too_many_arguments)]
pub fn render_workspace_with_eval(
    frame: &mut Frame<'_>,
    status: &Value,
    evaluation_catalog: &Value,
    evaluation_report: Option<&Value>,
    base_url: &str,
    selected: usize,
    list_state: &mut ListState,
) {
    let empty = Value::Null;
    render_workspace_with_data(
        frame,
        status,
        &empty,
        &empty,
        &empty,
        None,
        None,
        Some(evaluation_catalog),
        evaluation_report,
        TradeWorkspaceData::default(),
        base_url,
        selected,
        list_state,
    );
}

/// Renders navigation, module inventory, and the local data workspace.
#[allow(clippy::too_many_arguments)]
fn render_workspace_with_data(
    frame: &mut Frame<'_>,
    status: &Value,
    catalog: &Value,
    models: &Value,
    views: &Value,
    data_result: Option<&Value>,
    fleet_simulation: Option<&Value>,
    evaluation_catalog: Option<&Value>,
    evaluation_report: Option<&Value>,
    trade: TradeWorkspaceData<'_>,
    base_url: &str,
    selected: usize,
    list_state: &mut ListState,
) {
    let selected = selected.min(WORKSPACES.len() - 1);
    let sidebar_width = frame
        .area()
        .width
        .min(if frame.area().width < 72 { 16 } else { 24 });
    let areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Min(1)])
        .split(frame.area());

    let items = WORKSPACES
        .iter()
        .map(|name| ListItem::new(Line::from(*name)));
    let navigation = List::new(items)
        .block(Block::default().title("HyperAI").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(navigation, areas[0], list_state);

    let modules = status
        .get("connections")
        .and_then(Value::as_array)
        .map_or_else(
            || "No module inventory was returned.".to_owned(),
            |items| {
                items
                    .iter()
                    .map(|item| {
                        let name = item
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        let state = item
                            .get("state")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        format!("{name}: {state}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        );
    let detail_text = if WORKSPACES[selected] == "Data" {
        data_workspace_text(
            base_url,
            catalog,
            models,
            views,
            data_result,
            DataQueryPresentation {
                draft: status["data_query_draft"]
                    .as_str()
                    .unwrap_or(DEFAULT_DATA_QUERY),
                editing: status["data_query_editing"].as_bool().unwrap_or(false),
                message: status["data_query_message"].as_str().unwrap_or_default(),
                preview_width: areas[1].width.saturating_sub(2) as usize,
            },
        )
    } else if WORKSPACES[selected] == "Workload Trace" {
        workload_trace_workspace_text(
            base_url,
            &status["workload_trace"],
            status["workload_trace_error"].as_str(),
        )
    } else if WORKSPACES[selected] == "Market" {
        market_workspace_text_from_status(base_url, status)
    } else if WORKSPACES[selected] == "Deals" {
        market_history_workspace_text(base_url, &status["market_history"])
    } else if WORKSPACES[selected] == "Fleet" {
        fleet_workspace_text(base_url, status, fleet_simulation)
    } else if WORKSPACES[selected] == "Eval" {
        eval_workspace_text(base_url, evaluation_catalog, evaluation_report)
    } else if WORKSPACES[selected] == "Trade" {
        trade_workspace_text(base_url, trade)
    } else if WORKSPACES[selected] == "Agents & Access" {
        agent_workspace_text(base_url, status)
    } else if WORKSPACES[selected] == "Connections" {
        connection_inventory_workspace_text(&status["connection_inventory"])
    } else {
        format!(
            "Connected to {base_url}\n\nWorkspace: {}\n\n{modules}\n\nMarket observations are advisory. Fleet operations and paid evaluations are not enabled.",
            WORKSPACES[selected]
        )
    };
    let detail = Paragraph::new(detail_text)
        .block(
            Block::default()
                .title("Local workspace")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, areas[1]);
}

fn market_workspace_text_from_status(base_url: &str, status: &Value) -> String {
    market_supply_comparison_workspace_text(
        base_url,
        &status["supply_comparison"],
        status["supply_comparison_error"].as_str(),
        status["supply_quote_preview"]
            .as_object()
            .map(|_| &status["supply_quote_preview"]),
        status["supply_quote_approval"]
            .as_object()
            .map(|_| &status["supply_quote_approval"]),
        status["supply_quote_completion"]
            .as_object()
            .map(|_| &status["supply_quote_completion"]),
        status["supply_quote_message"].as_str(),
    )
}

fn market_history_workspace_text(base_url: &str, history: &Value) -> String {
    let mut lines = vec![
        format!("Connected to {base_url}"),
        String::new(),
        "Workspace: Deals / Market-owned local history".to_owned(),
        "Durable synthetic approvals and mock receipts · no execution or payment claim".to_owned(),
        format!(
            "Verified page digests: {}",
            history["page_digests"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" · ")
        ),
    ];
    if let Some(error) = history["error"].as_str() {
        lines.push(format!("History unavailable: {error}"));
        lines.push("No records are presented as a complete history.".to_owned());
        return lines.join("\n");
    }
    let Some(entries) = history["entries"].as_array() else {
        lines.push("History response is unavailable or malformed.".to_owned());
        return lines.join("\n");
    };
    if entries.is_empty() {
        lines.push("No locally approved synthetic deals are recorded yet.".to_owned());
    }
    for entry in entries {
        let approval = &entry["approval"];
        let deal_digest = approval["deal_digest"].as_str().unwrap_or("unavailable");
        let preview_digest = approval["preview_digest"].as_str().unwrap_or("unavailable");
        let quote = &approval["approved_deal"]["signed_quote"]["quote"];
        let seller = quote["seller_id"].as_str().unwrap_or("unknown seller");
        let completion = entry["completion_receipt"].as_object().is_some();
        lines.push(String::new());
        lines.push(format!(
            "Record {} · {}",
            entry["sequence"].as_u64().unwrap_or(0),
            if completion {
                "MOCK COMPLETED"
            } else {
                "APPROVED LOCALLY"
            }
        ));
        lines.push(format!(
            "Seller: {seller} · {} units · {} hours",
            quote["units"].as_u64().unwrap_or(0),
            quote["duration_hours"].as_u64().unwrap_or(0)
        ));
        lines.push(format!("Deal digest: {deal_digest}"));
        lines.push(format!("Quote preview digest: {preview_digest}"));
        if let Some(receipt_digest) = entry["completion_receipt"]["digest"].as_str() {
            lines.push(format!("Mock receipt digest: {receipt_digest}"));
        }
    }
    if let Some(message) = history["message"].as_str() {
        lines.push(String::new());
        lines.push(message.to_owned());
    }
    lines.push("Keys: n load next page · q quit".to_owned());
    lines.join("\n")
}

fn market_supply_comparison_workspace_text(
    base_url: &str,
    comparison: &Value,
    request_error: Option<&str>,
    quote_preview: Option<&Value>,
    quote_approval: Option<&Value>,
    quote_completion: Option<&Value>,
    quote_message: Option<&str>,
) -> String {
    if let Some(error) = request_error {
        return format!(
            "Connected to {base_url}\n\nWorkspace: Market / buyer supply\n\nThe read-only synthetic comparison is unavailable: {error}\nNo seller, registry, quote, or deal action was attempted."
        );
    }
    let body = &comparison["body"];
    if body["schema_version"] != "hyperai.compute-market.supply-comparison.v1" {
        return format!(
            "Connected to {base_url}\n\nWorkspace: Market / buyer supply\n\nThe service returned an unsupported comparison contract. No market action was attempted."
        );
    }
    let request = &body["buyer_request"];
    let selected = body["selected_offer_id"]
        .as_str()
        .unwrap_or("none eligible");
    let mut lines = vec![
        format!("Connected to {base_url}"),
        String::new(),
        "Workspace: Market / buyer supply".to_owned(),
        format!(
            "Buyer request: {} · {} · {} units · {} hours",
            request["resource_model"]
                .as_str()
                .unwrap_or("unknown resource"),
            request["region"].as_str().unwrap_or("unknown region"),
            request["units"].as_u64().unwrap_or(0),
            request["duration_hours"].as_u64().unwrap_or(0),
        ),
        format!(
            "Rate cap: {} micro-USD/unit-hour · total cap: {} micro-USD",
            request["max_rate_microusd_per_unit_hour"]
                .as_u64()
                .unwrap_or(0),
            request["max_total_microusd"].as_u64().unwrap_or(0),
        ),
        format!("Selected eligible offer: {selected}"),
        "Keys: Enter preview · a approve synthetic quote · c mock-complete".to_owned(),
    ];
    if let Some(message) = quote_message {
        lines.extend([String::new(), message.to_owned()]);
    }
    lines.extend(market_quote_lifecycle_lines(
        quote_preview,
        quote_approval,
        quote_completion,
    ));
    lines.extend([
        String::new(),
        "SIGNED OFFERS · buyer constraints only".to_owned(),
    ]);
    lines.extend(market_supply_offer_lines(body));
    lines.extend([
        String::new(),
        "ADVISORY OBSERVATIONS · do not set eligibility or seller capacity".to_owned(),
    ]);
    lines.extend(market_advisory_observation_lines(body));
    lines.extend([
        String::new(),
        format!(
            "Catalog digest: {} · compared at {}",
            body["catalog"]["digest"].as_str().unwrap_or("unknown"),
            body["compared_at"].as_str().unwrap_or("unknown"),
        ),
        "Freshness: not evaluated. Synthetic fixture only; no live registry or seller was contacted."
            .to_owned(),
    ]);
    lines.join("\n")
}

fn market_quote_lifecycle_lines(
    quote_preview: Option<&Value>,
    quote_approval: Option<&Value>,
    quote_completion: Option<&Value>,
) -> Vec<String> {
    let Some(preview) = quote_preview else {
        return Vec::new();
    };
    let body = &preview["body"];
    let nested = &body["quote_preview"];
    let quote = &nested["body"]["signed_quote"]["quote"];
    let mut lines = vec![
        String::new(),
        "SYNTHETIC QUOTE PREVIEW · no seller contact or funds movement".to_owned(),
        format!(
            "Selected offer: {} · signed offer digest: {}",
            body["selected_offer_id"].as_str().unwrap_or("unknown"),
            body["selected_offer_digest"].as_str().unwrap_or("unknown"),
        ),
        format!(
            "Quote: {} micro-USD · cap: {} micro-USD · preview digest: {}",
            quote["quoted_total_microusd"].as_u64().unwrap_or(0),
            quote["amount_cap_microusd"].as_u64().unwrap_or(0),
            nested["digest"].as_str().unwrap_or("unknown"),
        ),
        format!(
            "Supply-to-quote digest: {} · comparison digest: {}",
            preview["digest"].as_str().unwrap_or("unknown"),
            body["comparison_digest"].as_str().unwrap_or("unknown"),
        ),
        body["claim_ceiling"]
            .as_str()
            .unwrap_or("claim ceiling unavailable")
            .to_owned(),
    ];
    if let Some(approval) = quote_approval {
        lines.extend([
            "SYNTHETIC BUYER APPROVAL · not a purchase authorization".to_owned(),
            format!(
                "Preview: {} · deal: {}",
                approval["preview_digest"].as_str().unwrap_or("unknown"),
                approval["deal_digest"].as_str().unwrap_or("unknown"),
            ),
            "a will not send a second approval request.".to_owned(),
        ]);
        if let Some(completion) = quote_completion {
            lines.extend([
                "MOCK FULFILLMENT AND RECEIPT · no workload ran or funds moved".to_owned(),
                format!(
                    "Deal digest: {}",
                    completion["deal_digest"].as_str().unwrap_or("unknown"),
                ),
                format!(
                    "Receipt digest: {}",
                    completion["demo"]["receipt"]["digest"]
                        .as_str()
                        .unwrap_or("unknown"),
                ),
                format!(
                    "Boundary: {}",
                    completion["effect_boundary"].as_str().unwrap_or("unknown"),
                ),
            ]);
        } else {
            lines.push(
                "Press c to request the deterministic mock receipt; no compute will run."
                    .to_owned(),
            );
        }
    } else {
        lines.push(
            "Press a to record the local synthetic approval; no purchase is authorized.".to_owned(),
        );
    }
    lines
}

fn market_supply_offer_lines(body: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let entries = body["catalog"]["body"]["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(assessments) = body["offer_assessments"].as_array() {
        for assessment in assessments {
            let offer_id = assessment["offer_id"].as_str().unwrap_or("unknown offer");
            let offer = entries
                .iter()
                .find(|entry| entry["signed_offer"]["offer"]["offer_id"] == offer_id)
                .map(|entry| &entry["signed_offer"]["offer"]);
            let status = if assessment["eligible"] == true {
                "eligible"
            } else {
                "not eligible"
            };
            let reasons = assessment["ineligibility_reasons"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|value| value.replace('_', " "))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let selected_mark = if assessment["selected"] == true {
                " · SELECTED"
            } else {
                ""
            };
            lines.push(format!(
                "{}{} · {} · {} micro-USD/unit-hour · {} · source {}",
                offer_id,
                selected_mark,
                offer
                    .and_then(|value| value["resource_model"].as_str())
                    .unwrap_or("unknown resource"),
                assessment["asking_rate_microusd_per_unit_hour"]
                    .as_u64()
                    .unwrap_or(0),
                if reasons.is_empty() {
                    status.to_owned()
                } else {
                    format!("{status}: {reasons}")
                },
                assessment["source_ids"].as_array().map_or_else(
                    || "unknown".to_owned(),
                    |values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                ),
            ));
        }
    }
    lines
}

fn market_advisory_observation_lines(body: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(observations) = body["advisory_observations"].as_array() {
        for observation in observations {
            lines.push(format!(
                "{} · estimated rate {:?} micro-USD/unit-hour · availability {:?} units · source {} · observed {}",
                observation["body"]["offer_id"].as_str().unwrap_or("unknown offer"),
                observation["body"]["estimated_rate_microusd_per_unit_hour"].as_u64(),
                observation["body"]["estimated_available_units"].as_u64(),
                observation["body"]["source_id"].as_str().unwrap_or("unknown"),
                observation["body"]["observed_at"].as_str().unwrap_or("unknown time"),
            ));
        }
    }
    lines
}

fn connection_inventory_workspace_text(inventory: &Value) -> String {
    let Some(connections) = inventory["connections"].as_array() else {
        return "Connection inventory is unavailable.\n\nThis workspace performs no health or reachability probe.".to_owned();
    };
    if connections.is_empty() {
        return "The static connection inventory contains no entries.".to_owned();
    }

    let mut lines = vec![format!(
        "{} · {}\n{}",
        inventory["schema_version"]
            .as_str()
            .unwrap_or("Unknown inventory schema"),
        inventory["mode"].as_str().unwrap_or("Unknown mode"),
        inventory["authority_note"]
            .as_str()
            .unwrap_or("Inventory authority is unspecified."),
    )];
    for item in connections {
        let name = item["name"].as_str().unwrap_or("Unknown connection");
        let mode = item["connection_mode"].as_str().unwrap_or("unknown");
        let owner = item["owner"].as_str().unwrap_or("Unknown owner");
        let state = item["state"].as_str().unwrap_or("unknown");
        let boundary = item["boundary"].as_str().unwrap_or("Boundary unavailable.");
        let revision = item["source_revision"].as_str().unwrap_or("Not pinned");
        let capabilities = item["exposed_capability_ids"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|values| !values.is_empty())
            .unwrap_or_else(|| "None exposed".to_owned());
        let gate_id = item["next_blocked_gate"]["id"]
            .as_str()
            .unwrap_or("unspecified");
        let gate_summary = item["next_blocked_gate"]["summary"]
            .as_str()
            .unwrap_or("No gate description was returned.");
        lines.push(format!(
            "{name}\nMode: {} · state: {}\nOwner: {owner}\nSource revision: {revision}\nExposed surfaces: {capabilities}\nBoundary: {boundary}\nNext gate ({gate_id}): {gate_summary}",
            mode.replace('_', " "),
            state.replace('_', " "),
        ));
    }
    lines.join("\n\n")
}

fn workload_trace_workspace_text(
    base_url: &str,
    trace: &Value,
    request_error: Option<&str>,
) -> String {
    let reference_items = trace["references"].as_array();
    let seller_failure_reported = reference_items.is_some_and(|items| {
        items
            .iter()
            .any(|reference| reference["status"] == "fulfillment_failed")
    });
    let inconclusive_reported =
        trace["state_slice"] == "hyperai.bazaar.workload_trace_inconclusive_served.v1";
    let limitations = if seller_failure_reported {
        "Seller-attributed failure report only.\nExecution, correctness, and payment remain unverified."
    } else if inconclusive_reported {
        "Mock verdict: inconclusive.\nMock disposition: hold for dispute.\nNo actual verifier decision, dispute, funds hold, or payment authorization."
    } else {
        "Synthetic fixture only.\nNo workload ran or funds moved.\nNo correctness or payment claim."
    };
    let references = reference_items.map_or_else(
        || "No workload trace was returned.".to_owned(),
        |items| {
            items
                .iter()
                .map(|reference| {
                    let owner = reference["owner_id"].as_str().unwrap_or("unknown owner");
                    let record = reference["record_id"].as_str().unwrap_or("unknown record");
                    let status = reference["status"].as_str().map_or_else(
                        || "unknown status".to_owned(),
                        |status| status.replace('_', " "),
                    );
                    let digest = reference["digest"]
                        .as_str()
                        .map_or_else(|| "no source digest".to_owned(), short_digest);
                    let reference_claim = if status == "fulfillment failed" {
                        match reference["claim_ceiling"].as_str() {
                            Some("seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence") => {
                                "\n  Claim: seller report only\n  Not independent execution or payment evidence".to_owned()
                            }
                            Some(claim) => format!("\n  Claim ceiling: {}", claim.replace('_', " ")),
                            None => "\n  Claim ceiling unavailable".to_owned(),
                        }
                    } else if matches!(
                        status.as_str(),
                        "mock verdict inconclusive" | "mock disposition hold for dispute"
                    ) {
                        format!(
                            "\n  Claim ceiling: {}",
                            reference["claim_ceiling"]
                                .as_str()
                                .unwrap_or("unavailable")
                                .replace('_', " ")
                        )
                    } else {
                        String::new()
                    };
                    format!("{owner} · {record}\n  {status}\n  {digest}{reference_claim}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        },
    );
    let claim_ceiling = if seller_failure_reported {
        "seller report only"
    } else {
        trace["claim_ceiling"].as_str().unwrap_or("unavailable")
    };
    let preview_mode = if seller_failure_reported {
        "fixed synthetic failure"
    } else if inconclusive_reported {
        "fixed inconclusive mock outcome"
    } else {
        "default trace"
    };
    let request_status = request_error.map_or_else(String::new, |error| {
        format!("\nLast preview request failed; prior trace cleared: {error}")
    });
    format!(
        "Connected to {base_url}\nPreview: {preview_mode}{request_status}\nKeys: i=hold f=seller r=default\n\nWorkload: {}\nTrace digest: {}\nClaim ceiling: {claim_ceiling}\n\n{limitations}\n\nSource records\n{references}",
        trace["intent"]["body"]["workload_id"]
            .as_str()
            .unwrap_or("unavailable"),
        trace["digest"]
            .as_str()
            .map_or_else(|| "unavailable".to_owned(), short_digest),
    )
}

fn short_digest(digest: &str) -> String {
    if digest.len() <= 28 {
        digest.to_owned()
    } else {
        format!("{}…{}", &digest[..12], &digest[digest.len() - 8..])
    }
}

#[allow(clippy::too_many_lines)] // Keeps the complete local session contract visible together.
fn agent_workspace_text(base_url: &str, status: &Value) -> String {
    let agent = &status["agent"];
    let lifecycle = agent["status"].as_str().unwrap_or("unknown");
    let profile = &agent["profile"];
    let capabilities = agent["capabilities"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let tools = agent["tools"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n  ")
        })
        .unwrap_or_default();
    let events = agent["events"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .rev()
                .take(6)
                .rev()
                .filter_map(|item| {
                    let sequence = item["sequence"].as_u64()?;
                    let text = item["text"].as_str()?;
                    let safe_text = text
                        .chars()
                        .map(|character| {
                            if character.is_control() && !matches!(character, '\n' | '\t') {
                                '�'
                            } else {
                                character
                            }
                        })
                        .collect::<String>();
                    Some(format!("{sequence:02}: {safe_text}"))
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let prompt = status["agent_prompt_draft"].as_str();
    let profile_stage = status["agent_profile_count"].as_u64().unwrap_or_default() + 1;
    let mut lines = vec![
        format!("Connected to {base_url}"),
        String::new(),
        format!("ACP session: {}", lifecycle.replace('_', " ")),
        format!(
            "Protocol: {}",
            agent["protocol_version"].as_u64().map_or_else(
                || "negotiating / unavailable".to_owned(),
                |version| format!("v{version}")
            )
        ),
    ];
    if let Some(name) = profile["name"].as_str() {
        lines.push(format!("Profile: {name}"));
        lines.push(format!(
            "Executable: {}",
            profile["executable"].as_str().unwrap_or("unavailable")
        ));
        lines.push(format!(
            "Working directory: {}",
            profile["working_directory"]
                .as_str()
                .unwrap_or("unavailable")
        ));
    }
    lines.push(format!(
        "Local session: {}",
        agent["local_session_id"].as_str().unwrap_or("unavailable")
    ));
    lines.push(format!(
        "Profile digest: {}",
        agent["profile_digest"].as_str().unwrap_or("unavailable")
    ));
    lines.push(format!(
        "Tool scope: {}",
        agent["tool_set_version"].as_str().unwrap_or("unavailable")
    ));
    lines.push(format!(
        "Capabilities: {}",
        if capabilities.is_empty() {
            "none reported"
        } else {
            &capabilities
        }
    ));
    lines.push("Read-only HyperAI tools:".to_owned());
    if tools.is_empty() {
        lines.push("  unavailable".to_owned());
    } else {
        lines.push(format!("  {tools}"));
    }
    if let Some(code) = agent["error_code"].as_str() {
        lines.push(format!("Status code: {code}"));
    }
    if !events.is_empty() {
        lines.extend([
            String::new(),
            "Recent assistant text (volatile):".to_owned(),
            events,
        ]);
    }
    lines.extend([
        String::new(),
        "Process boundary: ACP does not sandbox the selected executable or its own tools, files, network, or credentials.".to_owned(),
        "Keys: p connect · Enter prompt · c cancel · x disconnect · q quit".to_owned(),
    ]);
    if status["agent_profile_capture"] == true {
        lines.push(format!(
            "Profile field {profile_stage}/3: {}",
            status["agent_profile_line"].as_str().unwrap_or_default()
        ));
        lines.push(
            "Enter name, absolute executable, absolute working directory; Enter advances."
                .to_owned(),
        );
    }
    if let Some(prompt) = prompt {
        lines.push(format!("Prompt input: {prompt}"));
        lines.push("Enter sends · Backspace edits · Esc clears".to_owned());
    }
    if let Some(message) = status["agent_message"]
        .as_str()
        .filter(|message| !message.is_empty())
    {
        lines.extend([String::new(), message.to_owned()]);
    }
    lines.join("\n")
}

#[derive(Clone, Copy, Default)]
struct TradeWorkspaceData<'a> {
    catalog: Option<&'a Value>,
    series: Option<&'a Value>,
    scenario: Option<&'a Value>,
    paper: Option<&'a Value>,
}

fn trade_workspace_text(base_url: &str, trade: TradeWorkspaceData<'_>) -> String {
    let Some(catalog) = trade.catalog else {
        return format!(
            "Connected to {base_url}\n\nWorkspace: Trade / local synthetic research\n\nThe fixed research catalog is unavailable. No scenario or replay ran."
        );
    };
    let mut lines = trade_catalog_summary(base_url, catalog);
    if let Some(series) = trade.series {
        lines.extend(trade_series_summary(series));
    } else {
        lines.extend([
            String::new(),
            "Press Enter to load local research results.".to_owned(),
        ]);
    }
    if let Some(scenario) = trade.scenario {
        lines.extend(trade_scenario_summary(scenario));
    }
    if let Some(paper) = trade.paper {
        lines.extend(trade_paper_summary(paper));
    }
    lines.extend(trade_claim_summary(catalog));
    lines.join("\n")
}

fn trade_catalog_summary(base_url: &str, catalog: &Value) -> Vec<String> {
    vec![
        format!("Connected to {base_url}"),
        String::new(),
        "Workspace: Trade / local synthetic research".to_owned(),
        format!(
            "Source: {} · {}",
            catalog["dataset_revision"].as_str().unwrap_or("unknown"),
            catalog["freshness"].as_str().unwrap_or("unknown"),
        ),
        format!(
            "Coverage: {}/{} rows · {} bps",
            catalog["observed_row_count"].as_u64().unwrap_or(0),
            catalog["expected_row_count"].as_u64().unwrap_or(0),
            catalog["fixture_coverage_bps"].as_u64().unwrap_or(0),
        ),
        format!(
            "Dataset digest: {}",
            catalog["dataset_digest"].as_str().unwrap_or("unknown"),
        ),
        "Enter loads the East series, an unshocked regional comparison, and a one-unit paper replay.".to_owned(),
        String::new(),
        format!(
            "Methods: {}",
            catalog["method_versions"]
                .as_array()
                .map_or_else(String::new, |items| items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                .join(", ")),
        ),
    ]
}

fn trade_series_summary(series: &Value) -> Vec<String> {
    let missing = series["missing_periods"].as_array().map_or_else(
        || "none".to_owned(),
        |items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    vec![
        String::new(),
        format!(
            "SERIES / {} · {} observed of {}",
            series["instrument"]["instrument_id"]
                .as_str()
                .unwrap_or("unknown"),
            series["observed_period_count"].as_u64().unwrap_or(0),
            series["expected_period_count"].as_u64().unwrap_or(0),
        ),
        format!(
            "Missing slots: {missing} · availability unknown: {} · known zero: {}",
            series["unknown_availability_count"].as_u64().unwrap_or(0),
            series["zero_availability_count"].as_u64().unwrap_or(0),
        ),
        format!(
            "Mean absolute contiguous hourly change: {} bps across {} pairs (not forecast volatility)",
            series["mean_absolute_contiguous_change_bps"]
                .as_u64()
                .map_or_else(|| "insufficient data".to_owned(), |value| value.to_string()),
            series["contiguous_change_sample_count"]
                .as_u64()
                .unwrap_or(0),
        ),
    ]
}

fn trade_scenario_summary(scenario: &Value) -> Vec<String> {
    vec![
        String::new(),
        "REGIONAL ASKING-PRICE SCENARIO".to_owned(),
        format!(
            "{} vs {} at {} · basis {} → {} micros/GPU-hour",
            scenario["primary_instrument"]["instrument_id"]
                .as_str()
                .unwrap_or("unknown"),
            scenario["comparison_instrument"]["instrument_id"]
                .as_str()
                .unwrap_or("unknown"),
            scenario["observed_at"].as_str().unwrap_or("unknown"),
            scenario["regional_basis_usd_micros_per_hour"]
                .as_i64()
                .unwrap_or(0),
            scenario["scenario_regional_basis_usd_micros_per_hour"]
                .as_i64()
                .unwrap_or(0),
        ),
        format!(
            "Availability: {} → {} units · unknown remains unknown",
            format_optional_units(&scenario["primary_available_units"]),
            format_optional_units(&scenario["scenario_available_units"]),
        ),
        format!(
            "Outlier median sensitivity: {} micros · one-period lag: {} micros",
            scenario["sensitivity"]["outlier_median_change_usd_micros_per_hour"]
                .as_i64()
                .unwrap_or(0),
            scenario["sensitivity"]["one_period_lag_change_usd_micros_per_hour"]
                .as_i64()
                .unwrap_or(0),
        ),
    ]
}

fn trade_paper_summary(paper: &Value) -> Vec<String> {
    vec![
        String::new(),
        "PAPER PROCUREMENT ASKING-COST REPLAY".to_owned(),
        format!(
            "Comparable periods: {} · skipped unknown capacity: {} · no eligible capacity: {}",
            paper["comparable_period_count"].as_u64().unwrap_or(0),
            paper["unknown_capacity_observation_count"]
                .as_u64()
                .unwrap_or(0),
            paper["no_eligible_capacity_count"].as_u64().unwrap_or(0),
        ),
        format!(
            "Baseline minus selected asking cost: {} micros · not a fill or realized saving",
            paper["baseline_minus_strategy_asking_cost_microusd"]
                .as_i64()
                .unwrap_or(0),
        ),
    ]
}

fn trade_claim_summary(catalog: &Value) -> Vec<String> {
    vec![
        String::new(),
        format!(
            "Claim ceiling: {}",
            catalog["claim_ceiling"].as_str().unwrap_or("unavailable")
        ),
        format!(
            "Nonclaims: {}",
            catalog["nonclaims"].as_str().unwrap_or("unavailable")
        ),
        "No orders, forecasts, funds, providers, or external effects.".to_owned(),
    ]
}

fn format_optional_units(value: &Value) -> String {
    value
        .as_u64()
        .map_or_else(|| "unknown".to_owned(), |units| units.to_string())
}

fn run_trade_research(
    client: &Client,
    root: &str,
    bearer_token: &str,
    catalog: &Value,
    parameters: TradeRunParameters<'_>,
) -> Result<(Value, Value, Value), DeskError> {
    let TradeRunParameters {
        primary_instrument_id,
        comparison_instrument_id,
        price_shock_bps,
        availability_delta_units,
        baseline_instrument_id,
        requested_units,
    } = parameters;
    let digest = catalog["dataset_digest"].as_str().unwrap_or_default();
    let series = fetch_service_json(
        client,
        root,
        bearer_token,
        &format!("trade/series/{primary_instrument_id}"),
    )?;
    let scenario_request = json!({
        "expected_dataset_digest": digest,
        "primary_instrument_id": primary_instrument_id,
        "comparison_instrument_id": comparison_instrument_id,
        "price_shock_bps": price_shock_bps,
        "availability_delta_units": availability_delta_units,
    });
    let scenario = client
        .post(format!("{root}/api/v1/trade/scenario"))
        .bearer_auth(bearer_token)
        .json(&scenario_request)
        .send()?;
    let scenario = checked_response(scenario)?.json::<Value>()?;
    let paper_request = json!({
        "expected_dataset_digest": digest,
        "baseline_instrument_id": baseline_instrument_id,
        "requested_units": requested_units,
    });
    let paper = client
        .post(format!("{root}/api/v1/trade/paper"))
        .bearer_auth(bearer_token)
        .json(&paper_request)
        .send()?;
    let paper = checked_response(paper)?.json::<Value>()?;
    Ok((series, scenario, paper))
}

#[derive(Clone, Copy)]
struct TradeRunParameters<'a> {
    primary_instrument_id: &'a str,
    comparison_instrument_id: &'a str,
    price_shock_bps: i32,
    availability_delta_units: i64,
    baseline_instrument_id: &'a str,
    requested_units: u64,
}

fn fleet_workspace_text(base_url: &str, status: &Value, simulation: Option<&Value>) -> String {
    let mut lines = vec![
        format!("Connected to {base_url}"),
        String::new(),
        "Workspace: Fleet / inventory, preview, simulator".to_owned(),
    ];

    append_fleet_inventory(&mut lines, status);
    append_fleet_doctor(&mut lines, status);
    append_fleet_simulation(&mut lines, simulation);
    append_fleet_preview(&mut lines, status);
    lines.extend([
        String::new(),
        "Synthetic fixtures only · no live health/capacity or host calls.".to_owned(),
    ]);
    lines.join("\n")
}

fn append_fleet_inventory(lines: &mut Vec<String>, status: &Value) {
    if let Some(nodes) = status["fleet_inventory"]["nodes"].as_array() {
        lines.push(format!("Inventory: {} synthetic fixtures", nodes.len()));
        for node in nodes {
            let display_name = node["display_name"].as_str().unwrap_or("unnamed fixture");
            let short_name = display_name.split(" · ").next().unwrap_or(display_name);
            lines.push(format!(
                "{} [{}] {}",
                short_name,
                node["node_id"].as_str().unwrap_or("unknown"),
                node["health_state"].as_str().unwrap_or("unknown"),
            ));
        }
    } else {
        lines.push("Synthetic inventory unavailable.".to_owned());
    }
}

fn append_fleet_doctor(lines: &mut Vec<String>, status: &Value) {
    let doctor_checks = status["fleet_doctor"]["checks"].as_array();
    let live_health = doctor_checks
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["check_id"] == "live_health_probe")
        })
        .and_then(|check| check["status"].as_str())
        .unwrap_or("unknown");
    let operation_authority = doctor_checks
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["check_id"] == "operation_authority")
        })
        .and_then(|check| check["status"].as_str())
        .unwrap_or("unknown");
    lines.push(format!(
        "Doctor · live probe {live_health} · operation authority {operation_authority}"
    ));
}

fn append_fleet_simulation(lines: &mut Vec<String>, simulation: Option<&Value>) {
    if let Some(simulation) = simulation {
        let phase = simulation["phase"].as_str().unwrap_or("unknown");
        let operation = simulation["last_operation"].as_str().unwrap_or("none");
        let next = next_fleet_step(simulation).map_or_else(
            || "none".to_owned(),
            |(next_operation, _)| next_operation.to_owned(),
        );
        lines.extend([
            String::new(),
            format!(
                "Simulator: {} · phase {phase} · next {next}",
                simulation["simulation_id"].as_str().unwrap_or("unknown")
            ),
            format!(
                "Virtual units: {}/{} allocated · {} quarantined",
                simulation["allocated_units"].as_u64().unwrap_or(0),
                simulation["virtual_capacity_units"].as_u64().unwrap_or(0),
                simulation["quarantined_units"].as_u64().unwrap_or(0),
            ),
            format!(
                "Lease: {} · task: {} · last: {operation}",
                simulation["lease_state"].as_str().unwrap_or("unknown"),
                simulation["virtual_task_id"].as_str().unwrap_or("none"),
            ),
            format!(
                "Simulator state: {}",
                simulation["persistence"].as_str().unwrap_or("unknown")
            ),
        ]);
    } else {
        lines.extend([String::new(), "Simulator snapshot unavailable.".to_owned()]);
    }
}

fn append_fleet_preview(lines: &mut Vec<String>, status: &Value) {
    let selected_node = status["fleet_inventory"]["nodes"]
        .as_array()
        .and_then(|nodes| {
            status["fleet_selected_node_index"]
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .and_then(|index| nodes.get(index))
        });
    lines.extend([
        String::new(),
        format!(
            "Target: {} · intent {}",
            selected_node
                .and_then(|node| node["node_id"].as_str())
                .unwrap_or("unavailable"),
            status["fleet_selected_operation"]
                .as_str()
                .unwrap_or("attach"),
        ),
        "Keys: n target · o intent · v preview · Enter simulate".to_owned(),
    ]);
    let preview = &status["fleet_preview"];
    if let Some(digest) = preview["preview_digest"].as_str() {
        lines.extend([
            format!(
                "Preview: {}",
                preview["preview_status"].as_str().unwrap_or("unknown")
            ),
            format!("Digest: {digest}"),
            format!(
                "Authority: {}",
                preview["authority"].as_str().unwrap_or("unknown"),
            ),
            format!(
                "Effect boundary: {}",
                preview["effect_boundary"].as_str().unwrap_or("unknown")
            ),
            format!(
                "Health gate: {}",
                preview["health_gate"].as_str().unwrap_or("unknown")
            ),
        ]);
    }
    if let Some(message) = status["fleet_message"]
        .as_str()
        .filter(|message| !message.is_empty())
    {
        lines.push(message.to_owned());
    }
}

fn eval_workspace_text(base_url: &str, catalog: Option<&Value>, report: Option<&Value>) -> String {
    let Some(catalog) = catalog else {
        return format!(
            "Connected to {base_url}\n\nWorkspace: Eval\n\nThe fixed synthetic catalog is unavailable. No evaluation ran."
        );
    };
    let mut lines = vec![
        format!("Connected to {base_url}"),
        String::new(),
        "Workspace: Eval / harness qualification".to_owned(),
        format!(
            "Suite: {} · {} cases",
            catalog["suite_id"].as_str().unwrap_or("unknown"),
            catalog["cases"].as_array().map_or(0, Vec::len),
        ),
        format!(
            "Suite digest: {}",
            catalog["suite_digest"].as_str().unwrap_or("unknown"),
        ),
        "Fixed fixtures: control-fixture-v1 and alternate-fixture-v1".to_owned(),
        "Enter runs three local scorer subprocesses; no agent or model runs.".to_owned(),
        String::new(),
        "SYNTHETIC CASES".to_owned(),
    ];
    if let Some(cases) = catalog["cases"].as_array() {
        lines.extend(cases.iter().map(|case| {
            format!(
                "{} / {}",
                case["case_id"].as_str().unwrap_or("unknown"),
                case["category"].as_str().unwrap_or("unknown"),
            )
        }));
    }
    if let Some(report) = report {
        lines.extend([
            String::new(),
            "SYNTHETIC FIXTURE REPORT".to_owned(),
            format!(
                "Control fixture: {}/{} rubric matches",
                report["control"]["matching_cases"].as_u64().unwrap_or(0),
                report["control"]["case_count"].as_u64().unwrap_or(0),
            ),
            format!(
                "Alternate fixture: {}/{} rubric matches",
                report["alternate"]["matching_cases"].as_u64().unwrap_or(0),
                report["alternate"]["case_count"].as_u64().unwrap_or(0),
            ),
            format!(
                "Repeat digest: {}",
                report["repeated_result_digest"]
                    .as_str()
                    .unwrap_or("unknown"),
            ),
            format!(
                "Child times ms: {}",
                report["child_wall_time_ms"]
                    .as_array()
                    .map_or_else(String::new, |items| items
                        .iter()
                        .filter_map(Value::as_u64)
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")),
            ),
            format!(
                "Serialized bytes per child: {} in / {} out",
                report["scorer_input_bytes"].as_u64().unwrap_or(0),
                report["scorer_output_bytes"].as_u64().unwrap_or(0),
            ),
            "Fixture comparison only; no candidate-quality claim.".to_owned(),
        ]);
    } else {
        lines.extend([
            String::new(),
            "Press Enter to run the fixed harness qualification.".to_owned(),
        ]);
    }
    lines.push(
        "No prompts, user tasks, model calls, provider access, or promotion authority.".to_owned(),
    );
    lines.join("\n")
}

fn run_eval_qualification(
    client: &Client,
    root: &str,
    bearer_token: &str,
    catalog: &Value,
) -> Result<Value, DeskError> {
    let digest = catalog["suite_digest"].as_str().unwrap_or_default();
    let key_prefix = digest.chars().take(16).collect::<String>();
    let request = json!({
        "suite_id": catalog["suite_id"].as_str().unwrap_or_default(),
        "expected_suite_digest": digest,
        "pair_id": catalog["pair_id"].as_str().unwrap_or_default(),
        "idempotency_key": format!("tui-eval-{key_prefix}"),
    });
    let response = checked_response(
        client
            .post(format!("{root}/api/v1/eval/run"))
            .bearer_auth(bearer_token)
            .json(&request)
            .send()?,
    )?;
    Ok(response.json::<Value>()?)
}

fn next_fleet_step(simulation: &Value) -> Option<(&'static str, Option<u8>)> {
    match simulation["phase"].as_str()? {
        "detached" => Some(("attach", None)),
        "attached" => Some(("launch", Some(1))),
        "launched" if simulation["last_operation"] == "cancel" => Some(("terminate", None)),
        "launched" => Some(("workload_start", None)),
        "workload_running" => Some(("cancel", None)),
        "reconciliation_required" => Some(("reconcile", None)),
        "cleanup_required" => Some(("cleanup", None)),
        _ => None,
    }
}

fn fetch_fleet_simulation(
    client: &Client,
    root: &str,
    bearer_token: &str,
) -> Result<Value, DeskError> {
    let response = checked_response(
        client
            .get(format!("{root}/api/v1/fleet/simulation"))
            .bearer_auth(bearer_token)
            .send()?,
    )?;
    Ok(response.json::<Value>()?)
}

fn fetch_market_history_page(
    client: &Client,
    root: &str,
    bearer_token: &str,
    cursor: Option<&str>,
) -> Result<LocalMarketHistoryPageV1, DeskError> {
    let mut url = reqwest::Url::parse(&format!("{root}/api/v1/market/deal-history"))
        .map_err(|_| DeskError::InvalidMarketResponse("history route URL was invalid"))?;
    url.query_pairs_mut().append_pair("limit", "20");
    if let Some(cursor) = cursor {
        url.query_pairs_mut().append_pair("cursor", cursor);
    }
    let bytes = checked_response(client.get(url).bearer_auth(bearer_token).send()?)?.bytes()?;
    decode_market_history_page(&bytes)
}

fn load_next_market_history_page(
    history: &mut MarketHistoryState,
    client: &Client,
    root: &str,
    bearer_token: &str,
) {
    let Some(cursor) = history.next_cursor.clone() else {
        "No additional local history page is available.".clone_into(&mut history.message);
        return;
    };
    match fetch_market_history_page(client, root, bearer_token, Some(&cursor)) {
        Ok(page) => history.accept_page(page),
        Err(error) => history.error = Some(error.to_string()),
    }
}

fn decode_market_history_page(bytes: &[u8]) -> Result<LocalMarketHistoryPageV1, DeskError> {
    let page = serde_json::from_slice::<LocalMarketHistoryPageV1>(bytes)
        .map_err(|_| DeskError::InvalidMarketResponse("history page could not be decoded"))?;
    hyperai_market::verify_local_market_history_page_v1(&page)
        .map_err(|_| DeskError::InvalidMarketResponse("history page digest did not verify"))?;
    Ok(page)
}

#[cfg(all(feature = "test-support", debug_assertions))]
fn wait_for_test_panic_key() -> Result<(), DeskError> {
    loop {
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('p')
        {
            return Ok(());
        }
    }
}

#[cfg(all(feature = "test-support", debug_assertions))]
#[allow(clippy::panic)] // Deliberately unwind only in the opt-in PTY panic fixture.
fn trigger_test_panic_after_first_draw() -> ! {
    panic!("test-support panic after first TUI draw");
}

fn fetch_service_json(
    client: &Client,
    root: &str,
    bearer_token: &str,
    path: &str,
) -> Result<Value, DeskError> {
    let response = checked_response(
        client
            .get(format!("{root}/api/v1/{path}"))
            .bearer_auth(bearer_token)
            .send()?,
    )?;
    Ok(response.json::<Value>()?)
}

fn checked_response(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response, DeskError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.json::<Value>().unwrap_or(Value::Null);
    Err(service_response_error(status.as_u16(), &body))
}

fn service_response_error(status: u16, body: &Value) -> DeskError {
    let normalized = hyperai_application::normalize_interface_error_v1(status, body);
    DeskError::ServiceResponse {
        status: normalized.status,
        code: normalized.code,
        message: normalized.message.to_owned(),
    }
}

fn post_service_json(
    client: &Client,
    root: &str,
    bearer_token: &str,
    path: &str,
    body: &Value,
) -> Result<Value, DeskError> {
    let response = checked_response(
        client
            .post(format!("{root}/api/v1/{path}"))
            .bearer_auth(bearer_token)
            .json(&body)
            .send()?,
    )?;
    Ok(response.json::<Value>()?)
}

fn valid_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn load_market_quote_preview(
    client: &Client,
    root: &str,
    bearer_token: &str,
    comparison: &Value,
) -> Result<Value, DeskError> {
    let comparison_digest = comparison["digest"]
        .as_str()
        .filter(|digest| valid_lower_sha256(digest))
        .ok_or(DeskError::InvalidMarketResponse(
            "the supply comparison has no valid digest",
        ))?;
    let preview = post_service_json(
        client,
        root,
        bearer_token,
        "market/supply-quote-preview",
        &json!({ "comparison_digest": comparison_digest }),
    )?;
    let quote = &preview["body"]["quote_preview"];
    let nested_digest = quote["digest"].as_str();
    if preview["body"]["schema_version"] != "hyperai.compute-market.supply-quote-preview.v1"
        || preview["body"]["comparison_digest"] != comparison_digest
        || preview["body"]["selected_offer_id"] != comparison["body"]["selected_offer_id"]
        || !preview["digest"].as_str().is_some_and(valid_lower_sha256)
        || quote["body"]["schema_version"] != "hyperai.compute-market.local-quote-preview.v1"
        || !nested_digest.is_some_and(valid_lower_sha256)
        || quote["body"]["selected_offer_id"] != preview["body"]["selected_offer_id"]
        || quote["body"]["signed_quote"]["quote"]["offer_digest"]
            != preview["body"]["selected_offer_digest"]
    {
        return Err(DeskError::InvalidMarketResponse(
            "the supply quote does not bind the displayed comparison and selected offer",
        ));
    }
    Ok(preview)
}

fn approve_market_quote(
    client: &Client,
    root: &str,
    bearer_token: &str,
    supply_quote: &Value,
) -> Result<Value, DeskError> {
    let quote = &supply_quote["body"]["quote_preview"];
    let preview_digest = quote["digest"]
        .as_str()
        .filter(|digest| valid_lower_sha256(digest))
        .ok_or(DeskError::InvalidMarketResponse(
            "approval requires the displayed quote-preview digest",
        ))?;
    let approval = post_service_json(
        client,
        root,
        bearer_token,
        "market/quote-approve",
        &json!({ "preview_digest": preview_digest }),
    )?;
    if approval["schema_version"] != "hyperai.compute-market.local-quote-approval.v1"
        || approval["state_slice"] != "hyperai.bazaar_interface.synthetic_quote_review.v1"
    {
        return Err(DeskError::InvalidMarketResponse(
            "the approval has an unsupported schema or owner slice",
        ));
    }
    if approval["preview_digest"] != preview_digest {
        return Err(DeskError::InvalidMarketResponse(
            "the approval refers to a different quote preview",
        ));
    }
    if !approval["deal_digest"]
        .as_str()
        .is_some_and(valid_lower_sha256)
    {
        return Err(DeskError::InvalidMarketResponse(
            "the approval has no valid deal digest",
        ));
    }
    if approval["approved_deal"]["signed_quote"] != quote["body"]["signed_quote"]
        || approval["approved_deal"]["buyer_request"] != quote["body"]["buyer_request"]
    {
        return Err(DeskError::InvalidMarketResponse(
            "the approved terms differ from the displayed seller quote or buyer request",
        ));
    }
    if !approval["effects"].as_str().is_some_and(|effects| {
        effects.contains("synthetic in-memory approval only")
            && effects.contains("no fulfillment")
            && effects.contains("funds effect")
    }) {
        return Err(DeskError::InvalidMarketResponse(
            "the approval did not declare its synthetic no-effect boundary",
        ));
    }
    Ok(approval)
}

fn complete_market_quote(
    client: &Client,
    root: &str,
    bearer_token: &str,
    supply_quote: &Value,
    approval: &Value,
) -> Result<Value, DeskError> {
    let quote = &supply_quote["body"]["quote_preview"];
    let preview_digest = quote["digest"]
        .as_str()
        .filter(|digest| valid_lower_sha256(digest))
        .ok_or(DeskError::InvalidMarketResponse(
            "completion requires the displayed quote-preview digest",
        ))?;
    let deal_digest = approval["deal_digest"]
        .as_str()
        .filter(|digest| valid_lower_sha256(digest))
        .ok_or(DeskError::InvalidMarketResponse(
            "completion requires the recorded approval digest",
        ))?;
    if approval["preview_digest"] != preview_digest {
        return Err(DeskError::InvalidMarketResponse(
            "the approval is for a different quote preview",
        ));
    }
    let completion = post_service_json(
        client,
        root,
        bearer_token,
        "market/quote-complete",
        &json!({
            "preview_digest": preview_digest,
            "deal_digest": deal_digest,
        }),
    )?;
    let receipt = &completion["demo"]["receipt"];
    if completion["schema_version"] != "hyperai.interface.market-quote-completion.v1"
        || completion["state_slice"] != "hyperai.bazaar_interface.synthetic_quote_completion.v1"
        || completion["preview_digest"] != preview_digest
        || completion["deal_digest"] != deal_digest
        || completion["demo"]["report"]["deal_digest"] != deal_digest
        || !receipt["digest"].as_str().is_some_and(valid_lower_sha256)
        || receipt["body"]["approved_deal"] != approval["approved_deal"]
        || completion["effect_boundary"]
            != "local_synthetic_mock_fulfillment_verifier_and_settlement_only"
        || !receipt["body"]["effects"].as_str().is_some_and(|effects| {
            effects.contains("no network") && effects.contains("or funds moved")
        })
    {
        return Err(DeskError::InvalidMarketResponse(
            "the mock receipt does not bind the reviewed quote and approval",
        ));
    }
    Ok(completion)
}

fn advance_fleet_simulation(
    client: &Client,
    root: &str,
    bearer_token: &str,
    simulation: &Value,
    operation: &str,
    gpu_units: Option<u8>,
) -> Result<Value, DeskError> {
    let state_digest = simulation["state_digest"].as_str().unwrap_or_default();
    let digest_prefix = state_digest.chars().take(16).collect::<String>();
    let request = json!({
        "expected_state_digest": state_digest,
        "idempotency_key": format!("tui-{digest_prefix}-{operation}"),
        "operation": operation,
        "gpu_units": gpu_units,
        "fault": "none",
    });
    checked_response(
        client
            .post(format!("{root}/api/v1/fleet/simulation/transition"))
            .bearer_auth(bearer_token)
            .json(&request)
            .send()?,
    )?;
    fetch_fleet_simulation(client, root, bearer_token)
}

#[derive(Clone, Copy)]
struct DataQueryPresentation<'a> {
    draft: &'a str,
    editing: bool,
    message: &'a str,
    preview_width: usize,
}

fn data_workspace_text(
    base_url: &str,
    catalog: &Value,
    models: &Value,
    views: &Value,
    result: Option<&Value>,
    query: DataQueryPresentation<'_>,
) -> String {
    let DataQueryPresentation {
        draft,
        editing,
        message,
        preview_width,
    } = query;
    let datasets = format_datasets(catalog);
    let result_text = result.map_or_else(
        || "No query has run in this session.".to_owned(),
        format_data_result,
    );
    let saved_models = format_saved_items(
        &models["models"],
        |item| {
            format!(
                "{} · {} input",
                item["model"]["model_id"].as_str().unwrap_or("unknown"),
                item["input_state"].as_str().unwrap_or("unknown")
            )
        },
        "No saved query models.",
    );
    let saved_views = format_saved_items(
        &views["views"],
        |item| {
            format!(
                "{} · {}",
                item["view"]["view_id"].as_str().unwrap_or("unknown"),
                item["model_state"].as_str().unwrap_or("unknown")
            )
        },
        "No saved views.",
    );
    let query_help = data_query_help(catalog, editing);
    let query_mode = if editing { "Editing" } else { "Ready" };
    let query_preview = preview_data_query(draft, preview_width);
    format!(
        "Connected to {base_url}\n\nWorkspace: Data\nCatalog: {}\n\n{datasets}\n\nSQL query · {query_mode}\n{query_preview}\n{query_help}\n{message}\n\n{result_text}\n\nSaved models\n{saved_models}\n\nSaved views\n{saved_views}\n\nLocal snapshot only. Asking prices and availability are not live market data or execution proof.",
        catalog["status"].as_str().unwrap_or("unknown"),
    )
}

fn preview_data_query(query: &str, max_width: usize) -> String {
    let query_chars = query.chars().count();
    if query_chars <= max_width {
        return query.to_owned();
    }
    if max_width < 16 {
        return query
            .chars()
            .rev()
            .take(max_width)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
    }
    let mut suffix_width = max_width.min(80);
    loop {
        let omitted = query_chars.saturating_sub(suffix_width);
        let prefix = format!("…(+{omitted} chars) ");
        if prefix.chars().count() + suffix_width <= max_width {
            let suffix = query.chars().skip(omitted).collect::<String>();
            return format!("{prefix}{suffix}");
        }
        suffix_width = suffix_width.saturating_sub(1);
    }
}

fn format_datasets(catalog: &Value) -> String {
    catalog
        .get("datasets")
        .and_then(Value::as_array)
        .map_or_else(
            || "No local datasets are available.".to_owned(),
            |items| {
                items
                    .iter()
                    .map(|item| {
                        let dataset_id = item["dataset_id"].as_str().unwrap_or("unknown");
                        let format = item["format"].as_str().unwrap_or("unknown");
                        let rows = item["row_count"].as_u64().unwrap_or_default();
                        let freshness = item["freshness"].as_str().unwrap_or("unknown");
                        format!("{dataset_id} · {format} · {rows} rows · {freshness}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        )
}

fn format_data_result(value: &Value) -> String {
    let columns = value["columns"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .unwrap_or_default();
    let rows = value["rows"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|row| {
                    row.as_array().map_or_else(String::new, |cells| {
                        cells
                            .iter()
                            .map(format_data_cell)
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    format!(
        "{columns}\n{rows}\n\n{} rows · {}",
        value["row_count"], value["snapshot_digest"]
    )
}

fn format_data_cell(value: &Value) -> String {
    value.as_str().map_or_else(
        || {
            if value.is_null() {
                "NULL".to_owned()
            } else {
                value.to_string()
            }
        },
        str::to_owned,
    )
}

fn format_saved_items(
    items: &Value,
    format_item: impl Fn(&Value) -> String,
    empty: &str,
) -> String {
    items
        .as_array()
        .map(|items| items.iter().map(format_item).collect::<Vec<_>>().join("\n"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| empty.to_owned())
}

fn data_query_help(catalog: &Value, editing: bool) -> &'static str {
    if catalog["datasets"]
        .as_array()
        .is_some_and(|datasets| !datasets.is_empty())
    {
        if editing {
            "Type SQL · Enter run · Ctrl+U clear · Esc cancel edit"
        } else {
            "e edit · Enter run saved query"
        }
    } else {
        "Seed the local synthetic snapshot with `hyperai data seed-synthetic`, then return here."
    }
}

fn run_data_query(
    client: &Client,
    base_url: &str,
    bearer_token: &str,
    catalog: &Value,
    sequence: u64,
    sql: &str,
) -> Result<Value, DeskError> {
    let dataset_id = catalog["datasets"]
        .as_array()
        .and_then(|datasets| datasets.first())
        .and_then(|dataset| dataset["dataset_id"].as_str())
        .unwrap_or("hyperai.synthetic_market_observations_json.v1");
    let response = checked_response(
        client
            .post(format!(
                "{}/api/v1/data/query",
                base_url.trim_end_matches('/')
            ))
            .bearer_auth(bearer_token)
            .json(&json!({
                "query_id": format!("tui-{}-{sequence}", std::process::id()),
                "request": {
                    "dataset_id": dataset_id,
                    "sql": sql,
                    "format": "json",
                    "max_rows": 100,
                }
            }))
            .send()?,
    )?
    .json::<Value>()?;
    Ok(response)
}

/// Runs cleanup when a terminal-owning scope exits, including startup errors.
#[must_use]
pub struct TerminalRestore(Option<Box<dyn FnOnce()>>);

impl TerminalRestore {
    /// Creates a guard whose callback restores terminal state on drop.
    pub fn new(cleanup: impl FnOnce() + 'static) -> Self {
        Self(Some(Box::new(cleanup)))
    }
}

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        if let Some(cleanup) = self.0.take() {
            cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
    use reqwest::blocking::Client;
    use serde_json::{Value, json};

    use super::{
        DataQueryAction, DataQueryInput, DeskError, MAX_DATA_QUERY_BYTES, MarketHistoryState,
        MarketQuoteAction, MarketQuoteLifecycleState, TradeWorkspaceData, WorkloadTracePreviewMode,
        approve_market_quote, checked_response, complete_market_quote,
        connection_inventory_workspace_text, decode_market_history_page, fetch_market_history_page,
        load_next_market_history_page, load_workload_trace_preview, market_history_workspace_text,
        market_supply_comparison_workspace_text, preview_data_query, render_workspace,
        render_workspace_with_data, run_data_query, service_response_error, trade_workspace_text,
        workload_trace_preview_route,
    };

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn overview_workspace_renders_default_boundaries_in_wide_frame()
    -> Result<(), Box<dyn std::error::Error>> {
        let status = json!({});
        let mut terminal = Terminal::new(TestBackend::new(120, 40))?;
        let mut list_state = ListState::default();
        terminal.draw(|frame| {
            render_workspace(frame, &status, "http://127.0.0.1:4317", 0, &mut list_state);
        })?;
        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(rendered.contains("Workspace: Overview"));
        assert!(rendered.contains("Market observations are advisory."));
        assert!(rendered.contains("Fleet operations and paid evaluations are not enabled."));
        Ok(())
    }

    #[test]
    fn deals_workspace_shows_market_owned_empty_and_error_states() {
        let empty = market_history_workspace_text(
            "http://127.0.0.1:4317",
            &json!({"entries": [], "page_digests": [], "message": "End of the available local history pages."}),
        );
        assert!(empty.contains("Market-owned local history"));
        assert!(empty.contains("No locally approved synthetic deals"));
        assert!(empty.contains("no execution or payment claim"));

        let failed = market_history_workspace_text(
            "http://127.0.0.1:4317",
            &json!({"error": "local service unavailable", "entries": []}),
        );
        assert!(failed.contains("History unavailable: local service unavailable"));
        assert!(failed.contains("No records are presented as a complete history"));
    }

    #[test]
    fn market_history_page_decoder_recomputes_the_typed_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        let page = hyperai_market::local_market_history_page_v1(Vec::new(), None)?;
        let bytes = serde_json::to_vec(&page)?;
        assert_eq!(decode_market_history_page(&bytes)?, page);

        let mut changed: Value = serde_json::from_slice(&bytes)?;
        changed["body"]["claim_ceiling"] = Value::String("incorrect".to_owned());
        let changed = serde_json::to_vec(&changed)?;
        assert!(matches!(
            decode_market_history_page(&changed),
            Err(DeskError::InvalidMarketResponse(
                "history page digest did not verify"
            ))
        ));
        Ok(())
    }

    #[test]
    fn market_history_fetch_forwards_the_opaque_cursor() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let page =
            hyperai_market::local_market_history_page_v1(Vec::new(), Some("opaque+/=".to_owned()))?;
        let body = serde_json::to_vec(&page)?;
        let expected_page = page.clone();
        let worker = thread::spawn(move || -> Result<String, std::io::Error> {
            let (mut stream, _) = listener.accept()?;
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = stream.read(&mut chunk)?;
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request_line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(&body)?;
            Ok(request_line)
        });
        let client = Client::builder().no_proxy().build()?;
        let page = fetch_market_history_page(
            &client,
            &format!("http://{address}"),
            "synthetic-owner-token",
            Some("opaque+/="),
        )?;
        let request_line = worker
            .join()
            .map_err(|_| "mock history server panicked")??;

        assert_eq!(page, expected_page);
        assert!(request_line.starts_with(
            "GET /api/v1/market/deal-history?limit=20&cursor=opaque%2B%2F%3D HTTP/1.1"
        ));
        Ok(())
    }

    #[test]
    fn deals_next_page_appends_verified_page_and_stops_at_terminal_cursor()
    -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let alternate_preview = hyperai_market::local_market_quote_preview_for_scenario(
            hyperai_market::LocalMarketQuoteScenarioV1::Alternate,
        )?;
        let baseline_preview = hyperai_market::local_market_quote_preview()?;
        let alternate_approval = hyperai_market::approve_local_market_quote_preview_for_scenario(
            alternate_preview.digest,
            hyperai_market::LocalMarketQuoteScenarioV1::Alternate,
        )?;
        let baseline_approval =
            hyperai_market::approve_local_market_quote_preview(baseline_preview.digest)?;
        let alternate_entry = hyperai_market::LocalMarketHistoryEntryV1 {
            sequence: 2,
            approval: alternate_approval,
            completion_receipt: None,
        };
        let baseline_entry = hyperai_market::LocalMarketHistoryEntryV1 {
            sequence: 1,
            approval: baseline_approval,
            completion_receipt: None,
        };
        let first = hyperai_market::local_market_history_page_v1(
            vec![alternate_entry],
            Some("opaque+/=".to_owned()),
        )?;
        let second = hyperai_market::local_market_history_page_v1(vec![baseline_entry], None)?;
        let first_digest = first.digest.to_string();
        let second_digest = second.digest.to_string();
        let alternate_deal_digest = first.body.entries[0].approval.deal_digest.to_string();
        let baseline_deal_digest = second.body.entries[0].approval.deal_digest.to_string();
        let body = serde_json::to_vec(&second)?;
        let worker = thread::spawn(move || -> Result<String, std::io::Error> {
            let (mut stream, _) = listener.accept()?;
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = stream.read(&mut chunk)?;
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(&body)?;
            Ok(String::from_utf8_lossy(&request).into_owned())
        });
        let client = Client::builder().no_proxy().build()?;
        let mut history = MarketHistoryState::default();
        history.accept_page(first);

        load_next_market_history_page(
            &mut history,
            &client,
            &format!("http://{address}"),
            "synthetic-owner-token",
        );
        let request = worker
            .join()
            .map_err(|_| "mock history server panicked")??;

        assert!(request.starts_with(
            "GET /api/v1/market/deal-history?limit=20&cursor=opaque%2B%2F%3D HTTP/1.1"
        ));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer synthetic-owner-token")
        );
        assert_eq!(history.page_digests, vec![first_digest, second_digest]);
        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.entries[0].sequence, 2);
        assert_eq!(history.entries[1].sequence, 1);
        assert_eq!(
            history.entries[0].approval.deal_digest.to_string(),
            alternate_deal_digest
        );
        assert_eq!(
            history.entries[1].approval.deal_digest.to_string(),
            baseline_deal_digest
        );
        assert!(history.next_cursor.is_none());
        assert_eq!(history.message, "End of the available local history pages.");

        load_next_market_history_page(
            &mut history,
            &client,
            &format!("http://{address}"),
            "synthetic-owner-token",
        );
        assert_eq!(history.page_digests.len(), 2);
        assert_eq!(
            history.message,
            "No additional local history page is available."
        );
        Ok(())
    }

    #[test]
    fn market_workspace_keeps_eligibility_and_advisory_observations_separate() {
        let comparison = json!({
            "body": {
                "schema_version": "hyperai.compute-market.supply-comparison.v1",
                "buyer_request": {
                    "resource_model": "GPU-H100",
                    "region": "us-east",
                    "units": 2,
                    "duration_hours": 3,
                    "max_rate_microusd_per_unit_hour": 2_500_000,
                    "max_total_microusd": 15_000_000
                },
                "selected_offer_id": "offer-gpu-b",
                "catalog": {"digest": "catalog-digest", "body": {"entries": []}},
                "offer_assessments": [{
                    "offer_id": "offer-gpu-b",
                    "eligible": true,
                    "selected": true,
                    "asking_rate_microusd_per_unit_hour": 1_800_000,
                    "ineligibility_reasons": [],
                    "source_ids": ["synthetic-registry-beta"]
                }],
                "advisory_observations": [{"body": {
                    "offer_id": "offer-gpu-b",
                    "estimated_rate_microusd_per_unit_hour": 1_600_000,
                    "estimated_available_units": 4,
                    "source_id": "synthetic-market-observation-feed",
                    "observed_at": "2026-09-24T12:00:00Z"
                }}],
                "compared_at": "2026-09-24T12:00:00Z",
                "freshness_status": "not_evaluated"
            }
        });
        let text = market_supply_comparison_workspace_text(
            "http://127.0.0.1:4317",
            &comparison,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(text.contains("SIGNED OFFERS · buyer constraints only"));
        assert!(text.contains("offer-gpu-b · SELECTED"));
        assert!(text.contains("ADVISORY OBSERVATIONS · do not set eligibility"));
        assert!(text.contains("Some(1600000)"));
        assert!(text.contains("Freshness: not evaluated"));
        assert!(text.contains("Keys: Enter preview · a approve synthetic quote · c mock-complete"));
    }

    #[test]
    fn market_workspace_renders_digest_bound_quote_without_approval() {
        let comparison = json!({
            "body": {
                "schema_version": "hyperai.compute-market.supply-comparison.v1",
                "buyer_request": {"resource_model": "GPU-H100", "region": "us-east"},
                "selected_offer_id": "offer-gpu-b",
                "catalog": {"digest": "catalog-digest", "body": {"entries": []}},
                "offer_assessments": [],
                "advisory_observations": [],
                "compared_at": "2026-09-24T12:00:00Z"
            }
        });
        let preview = json!({
            "digest": "wrapper-digest",
            "body": {
                "comparison_digest": "comparison-digest",
                "selected_offer_id": "offer-gpu-b",
                "selected_offer_digest": "signed-offer-digest",
                "claim_ceiling": "fixed local synthetic supply-to-quote contract consistency only",
                "quote_preview": {
                    "digest": "quote-preview-digest",
                    "body": {
                        "signed_quote": {"quote": {
                            "quoted_total_microusd": 10_800_000,
                            "amount_cap_microusd": 15_000_000
                        }}
                    }
                }
            }
        });
        let text = market_supply_comparison_workspace_text(
            "http://127.0.0.1:4317",
            &comparison,
            None,
            Some(&preview),
            None,
            None,
            Some("Read-only quote preview ready."),
        );
        assert!(text.contains("SYNTHETIC QUOTE PREVIEW"));
        assert!(text.contains("signed-offer-digest"));
        assert!(text.contains("10800000 micro-USD · cap: 15000000 micro-USD"));
        assert!(text.contains("quote-preview-digest"));
        assert!(text.contains("wrapper-digest"));
        assert!(text.contains("Press a to record the local synthetic approval"));
    }

    #[test]
    fn market_action_keys_follow_the_quote_approval_and_completion_order() {
        let mut state = MarketQuoteLifecycleState::default();
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::PreviewRequired)
        );
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::ApprovalRequired)
        );

        state.preview = Some(json!({ "digest": "preview" }));
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::Approve)
        );
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::ApprovalRequired)
        );

        state.approval = Some(json!({ "deal_digest": "deal" }));
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::ApprovalAlreadyRecorded)
        );
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::Complete)
        );

        state.completion = Some(json!({ "receipt": "receipt" }));
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::CompletionAlreadyRecorded)
        );
        assert_eq!(
            state.action_for_key(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(MarketQuoteAction::CompletionAlreadyRecorded)
        );
    }

    #[test]
    fn market_workspace_renders_approval_and_mock_completion_as_distinct_stages() {
        let comparison = json!({
            "body": {
                "schema_version": "hyperai.compute-market.supply-comparison.v1",
                "buyer_request": {"resource_model": "GPU-H100", "region": "us-east"},
                "selected_offer_id": "offer-gpu-b",
                "catalog": {"digest": "catalog-digest", "body": {"entries": []}},
                "offer_assessments": [],
                "advisory_observations": [],
                "compared_at": "2026-09-24T12:00:00Z"
            }
        });
        let preview = json!({
            "digest": "wrapper-digest",
            "body": {
                "selected_offer_id": "offer-gpu-b",
                "quote_preview": {
                    "digest": "quote-preview-digest",
                    "body": {"signed_quote": {"quote": {
                        "quoted_total_microusd": 10_800_000,
                        "amount_cap_microusd": 15_000_000
                    }}}
                }
            }
        });
        let approval = json!({
            "preview_digest": "quote-preview-digest",
            "deal_digest": "deal-digest"
        });
        let completion = json!({
            "deal_digest": "deal-digest",
            "effect_boundary": "local_synthetic_mock_fulfillment_verifier_and_settlement_only",
            "demo": {"receipt": {"digest": "receipt-digest"}}
        });
        let text = market_supply_comparison_workspace_text(
            "http://127.0.0.1:4317",
            &comparison,
            None,
            Some(&preview),
            Some(&approval),
            Some(&completion),
            Some("Mock fulfillment and receipt completed. No workload ran and no funds moved."),
        );
        assert!(text.contains("SYNTHETIC BUYER APPROVAL"));
        assert!(text.contains("Deal digest: deal-digest"));
        assert!(text.contains("MOCK FULFILLMENT AND RECEIPT"));
        assert!(text.contains("Receipt digest: receipt-digest"));
        assert!(text.contains("no workload ran or funds moved"));
    }

    fn market_quote_visibility_comparison() -> Value {
        json!({
            "body": {
                "schema_version": "hyperai.compute-market.supply-comparison.v1",
                "buyer_request": {
                    "resource_model": "GPU-H100",
                    "region": "us-east",
                    "units": 2,
                    "duration_hours": 4,
                    "max_rate_microusd_per_unit_hour": 2_500_000,
                    "max_total_microusd": 20_000_000
                },
                "selected_offer_id": "offer-gpu-b",
                "catalog": {
                    "digest": "catalog-digest",
                    "body": {"entries": [
                        {"signed_offer": {"offer": {
                            "offer_id": "offer-gpu-b",
                            "resource_model": "GPU-H100"
                        }}},
                        {"signed_offer": {"offer": {
                            "offer_id": "offer-gpu-a",
                            "resource_model": "GPU-H100"
                        }}}
                    ]}
                },
                "offer_assessments": [
                    {
                        "offer_id": "offer-gpu-b",
                        "eligible": true,
                        "selected": true,
                        "asking_rate_microusd_per_unit_hour": 1_800_000,
                        "ineligibility_reasons": [],
                        "source_ids": ["synthetic-registry-alpha"]
                    },
                    {
                        "offer_id": "offer-gpu-a",
                        "eligible": false,
                        "selected": false,
                        "asking_rate_microusd_per_unit_hour": 3_200_000,
                        "ineligibility_reasons": [
                            "buyer_rate_cap", "buyer_total_cap", "region_mismatch",
                            "resource_mismatch", "availability_unverified"
                        ],
                        "source_ids": ["synthetic-registry-beta"]
                    }
                ],
                "advisory_observations": [
                    {"body": {
                        "offer_id": "offer-gpu-b",
                        "estimated_rate_microusd_per_unit_hour": 1_700_000,
                        "estimated_available_units": 4,
                        "source_id": "synthetic-observation-feed-west-long-name",
                        "observed_at": "2026-09-24T12:00:00Z"
                    }},
                    {"body": {
                        "offer_id": "offer-gpu-a",
                        "estimated_rate_microusd_per_unit_hour": 2_900_000,
                        "estimated_available_units": 1,
                        "source_id": "synthetic-observation-feed-east-long-name",
                        "observed_at": "2026-09-24T12:00:00Z"
                    }}
                ],
                "compared_at": "2026-09-24T12:00:00Z"
            }
        })
    }

    fn market_quote_visibility_status() -> Value {
        let receipt_digest = "b".repeat(64);
        let preview = json!({
            "digest": "wrapper-digest",
            "body": {
                "selected_offer_id": "offer-gpu-b",
                "selected_offer_digest": "signed-offer-digest",
                "comparison_digest": "comparison-digest",
                "claim_ceiling": "fixed local synthetic supply-to-quote consistency only; this record does not describe external availability, completed computation, or payment entitlement",
                "quote_preview": {
                    "digest": "quote-preview-digest",
                    "body": {"signed_quote": {"quote": {
                        "quoted_total_microusd": 14_400_000,
                        "amount_cap_microusd": 20_000_000
                    }}}
                }
            }
        });
        let approval = json!({
            "preview_digest": "quote-preview-digest",
            "deal_digest": "deal-digest"
        });
        let completion = json!({
            "deal_digest": "deal-digest",
            "effect_boundary": "local_synthetic_mock_fulfillment_verifier_and_settlement_only",
            "demo": {"receipt": {"digest": receipt_digest}}
        });
        let status = json!({
            "supply_comparison": market_quote_visibility_comparison(),
            "supply_quote_preview": preview,
            "supply_quote_approval": approval,
            "supply_quote_completion": completion,
            "supply_quote_message": "Mock fulfillment and receipt completed. No workload ran and no funds moved."
        });
        status
    }

    #[test]
    fn completed_market_quote_receipt_is_visible_with_comparison_details()
    -> Result<(), Box<dyn std::error::Error>> {
        let status = market_quote_visibility_status();
        let mut terminal = Terminal::new(TestBackend::new(120, 40))?;
        let mut list_state = ListState::default();
        terminal.draw(|frame| {
            render_workspace(frame, &status, "http://127.0.0.1:4317", 1, &mut list_state);
        })?;
        let buffer = terminal.backend().buffer();
        let rendered = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        let receipt_digest = "b".repeat(64);
        let receipt_line = format!("Receipt digest: {receipt_digest}");
        let receipt_line_visible = buffer.content().chunks(120).any(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
                .contains(&receipt_line)
        });

        assert!(rendered.contains("MOCK FULFILLMENT AND RECEIPT"));
        assert!(receipt_line_visible);
        assert!(rendered.contains(&receipt_digest));
        assert!(rendered.contains("local_synthetic_mock_fulfillment_verifier_and_settlement_only"));
        assert!(
            rendered
                .find("MOCK FULFILLMENT AND RECEIPT")
                .zip(rendered.find("SIGNED OFFERS"))
                .is_some_and(|(receipt, offers)| receipt < offers)
        );
        Ok(())
    }

    #[test]
    fn connections_workspace_renders_inventory_in_wide_and_narrow_panes()
    -> Result<(), Box<dyn std::error::Error>> {
        let inventory = json!({
            "schema_version": "hyperai.interface.connection-inventory.v1",
            "mode": "static_local_inventory",
            "authority_note": "Descriptive inventory only; not health or authorization.",
            "connections": [{
                "module_id": "compute_bazaar",
                "name": "The Compute Bazaar",
                "owner": "The Compute Bazaar",
                "source_revision": "d1809e5cfa843c087ebc4de9f00561bf0be67d55",
                "connection_mode": "reference_only",
                "state": "reference_only_rights_unresolved",
                "boundary": "Architecture reference only; no source or data is loaded.",
                "exposed_capability_ids": [],
                "next_blocked_gate": {
                    "id": "written_rights_for_exact_materials",
                    "summary": "Written rights for exact materials are required before reuse."
                }
            }]
        });
        let status = json!({
            "connections": [],
            "connection_inventory": inventory,
        });
        let summary = connection_inventory_workspace_text(&status["connection_inventory"]);
        assert!(summary.contains("The Compute Bazaar"));
        assert!(summary.contains("written_rights_for_exact_materials"));
        assert!(summary.contains("None exposed"));

        for (width, height) in [(120, 40), (52, 30)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height))?;
            let mut list_state = ListState::default();
            terminal.draw(|frame| {
                render_workspace(frame, &status, "http://127.0.0.1:4317", 12, &mut list_state);
            })?;
            let rendered = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>();
            assert!(rendered.contains("Connections"));
            assert!(rendered.contains("The Compute Bazaar"));
        }
        Ok(())
    }

    #[test]
    fn data_query_input_edits_clears_cancels_and_submits_utf8() {
        let mut input = DataQueryInput::default();
        input.begin_editing();
        assert_eq!(
            input.handle_key(key(KeyCode::Char('u'), KeyModifiers::CONTROL,)),
            DataQueryAction::Changed
        );
        for character in "SELECT 'λ'".chars() {
            assert_eq!(
                input.handle_key(key(KeyCode::Char(character), KeyModifiers::NONE)),
                DataQueryAction::Changed
            );
        }
        assert_eq!(
            input.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            DataQueryAction::Submit("SELECT 'λ'".to_owned())
        );
        assert!(!input.editing);

        input.begin_editing();
        input.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE));
        let cancelled_draft = input.sql.clone();
        assert_eq!(
            input.handle_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            DataQueryAction::Cancelled
        );
        assert_eq!(input.sql, cancelled_draft);
        assert!(!input.editing);

        input.begin_editing();
        assert_eq!(
            input.handle_key(key(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            DataQueryAction::Changed
        );
        assert_eq!(
            input.handle_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            DataQueryAction::Empty
        );
        input.begin_editing();
        assert_eq!(
            input.handle_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            DataQueryAction::Exit
        );
    }

    #[test]
    fn data_query_input_enforces_utf8_byte_limit_without_partial_character() {
        let mut input = DataQueryInput::default();
        input.begin_editing();
        input.handle_key(key(KeyCode::Char('u'), KeyModifiers::CONTROL));
        for _ in 0..MAX_DATA_QUERY_BYTES - 1 {
            assert_eq!(
                input.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE)),
                DataQueryAction::Changed
            );
        }
        assert_eq!(input.sql.len(), MAX_DATA_QUERY_BYTES - 1);
        assert_eq!(
            input.handle_key(key(KeyCode::Char('λ'), KeyModifiers::NONE)),
            DataQueryAction::LimitReached
        );
        assert_eq!(input.sql.len(), MAX_DATA_QUERY_BYTES - 1);
        assert_eq!(
            input.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE)),
            DataQueryAction::Changed
        );
        assert_eq!(input.sql.len(), MAX_DATA_QUERY_BYTES);
        assert_eq!(
            input.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE)),
            DataQueryAction::LimitReached
        );
    }

    #[test]
    fn data_query_preview_keeps_the_query_suffix_visible_at_narrow_width() {
        let sql = format!("{}SELECT 'λ'", "x".repeat(256));
        let preview = preview_data_query(&sql, 36);
        assert!(preview.chars().count() <= 36);
        assert!(preview.starts_with("…(+"));
        assert!(preview.ends_with("SELECT 'λ'"));
    }

    #[test]
    fn data_workspace_renders_the_editing_draft_and_key_controls()
    -> Result<(), Box<dyn std::error::Error>> {
        let status = json!({
            "connections": [],
            "data_query_draft": "SELECT gpu_model FROM dataset",
            "data_query_editing": true,
            "data_query_message": ""
        });
        let catalog = json!({
            "status": "ready",
            "datasets": [{
                "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                "format": "json",
                "row_count": 3,
                "freshness": "synthetic"
            }]
        });
        let empty = serde_json::Value::Null;
        let mut terminal = Terminal::new(TestBackend::new(120, 40))?;
        let mut list_state = ListState::default();
        terminal.draw(|frame| {
            render_workspace_with_data(
                frame,
                &status,
                &catalog,
                &empty,
                &empty,
                None,
                None,
                None,
                None,
                TradeWorkspaceData::default(),
                "http://127.0.0.1:4317",
                3,
                &mut list_state,
            );
        })?;
        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(rendered.contains("SQL query · Editing"));
        assert!(rendered.contains("SELECT gpu_model FROM dataset"));
        assert!(rendered.contains("Type SQL · Enter run · Ctrl+U clear · Esc cancel edit"));
        Ok(())
    }

    #[test]
    fn data_result_keeps_numeric_values_instead_of_rendering_them_as_null() {
        let result = json!({
            "columns": ["gpu_model", "asking_usd_micros_per_hour", "available_count"],
            "rows": [["A100", 1_200_000, 2], ["H100", null, 0]],
            "row_count": 2,
            "snapshot_digest": "synthetic-snapshot-digest"
        });
        let rendered = super::format_data_result(&result);
        assert!(rendered.contains("A100 | 1200000 | 2"));
        assert!(rendered.contains("H100 | NULL | 0"));
    }

    #[test]
    fn workload_trace_preview_selector_uses_only_the_three_fixed_routes() {
        assert_eq!(
            workload_trace_preview_route(WorkloadTracePreviewMode::SellerFailure),
            "market/workload-trace/failure-preview"
        );
        assert_eq!(
            workload_trace_preview_route(WorkloadTracePreviewMode::Inconclusive),
            "market/workload-trace/inconclusive-preview"
        );
        assert_eq!(
            workload_trace_preview_route(WorkloadTracePreviewMode::Default),
            "market/workload-trace"
        );
    }

    #[test]
    fn failed_workload_trace_preview_clears_the_stale_projection() -> Result<(), DeskError> {
        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(1))
            .build()?;
        let mut status = json!({
            "workload_trace": {"digest": "stale-digest"},
            "workload_trace_error": null
        });
        load_workload_trace_preview(
            &client,
            "http://127.0.0.1:0",
            "synthetic-local-test-capability",
            &mut status,
            WorkloadTracePreviewMode::Inconclusive,
        );
        assert!(status["workload_trace"].is_null());
        assert!(status["workload_trace_error"].as_str().is_some());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn checked_response_preserves_versioned_error_from_local_service()
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
        let query_base = base.clone();
        let query_bearer = bearer.clone();
        let (error_text, is_expected_error) = tokio::task::spawn_blocking(move || {
            let client = Client::builder().no_proxy().build()?;
            let response = client
                .post(format!("{query_base}/api/v1/data/query"))
                .bearer_auth(query_bearer)
                .json(&json!({
                    "query_id": "tui-local-error-parity",
                    "request": {
                        "dataset_id": "hyperai.synthetic_market_observations_json.v1",
                        "sql": "SELECT read_file('/tmp/hyperai-parity-nonexistent-canary') FROM dataset",
                        "format": "json",
                        "max_rows": null
                    }
                }))
                .send()?;
            let error = checked_response(response)
                .err()
                .ok_or_else(|| std::io::Error::other("expected the service error response"))?;
            let is_expected_error = matches!(
                &error,
                DeskError::ServiceResponse { status: 422, code, .. }
                    if code == "DATA_QUERY_REJECTED"
            );
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((
                error.to_string(),
                is_expected_error,
            ))
        })
        .await??;

        assert!(
            is_expected_error,
            "unexpected TUI local-service error: {error_text}"
        );
        assert_eq!(
            error_text,
            "local service returned HTTP 422 (DATA_QUERY_REJECTED): The local HyperAI contract operation could not complete."
        );
        assert!(!error_text.contains("read_file"));
        client
            .post(format!("{base}/api/v1/service/stop"))
            .bearer_auth(&bearer)
            .json(&json!({}))
            .send()
            .await?
            .error_for_status()?;
        service.await??;
        Ok(())
    }

    fn exercise_market_quote_lifecycle(
        root: &str,
        bearer: &str,
        comparison: &Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(3))
            .build()?;
        let mut state = MarketQuoteLifecycleState::default();
        let action = state
            .action_for_key(key(KeyCode::Enter, KeyModifiers::NONE))
            .ok_or("Enter did not select quote preview")?;
        assert_eq!(action, MarketQuoteAction::Preview);
        state.handle_action(action, &client, root, bearer, comparison);
        assert!(state.message.contains("quote preview ready"));
        let supply_quote = state
            .preview
            .clone()
            .ok_or("TUI did not retain the quote preview")?;

        let mut stale_quote = supply_quote.clone();
        stale_quote["body"]["quote_preview"]["digest"] = json!("0".repeat(64));
        assert!(matches!(
            approve_market_quote(&client, root, bearer, &stale_quote),
            Err(DeskError::ServiceResponse { status: 409, ref code, .. })
                if code == "QUOTE_PREVIEW_CHANGED"
        ));
        let action = state
            .action_for_key(key(KeyCode::Char('a'), KeyModifiers::NONE))
            .ok_or("a did not select quote approval")?;
        assert_eq!(action, MarketQuoteAction::Approve);
        state.handle_action(action, &client, root, bearer, comparison);
        assert!(state.message.contains("Synthetic buyer approval recorded"));
        let approval = state
            .approval
            .clone()
            .ok_or("TUI did not retain the approval")?;

        let mut stale_approval = approval.clone();
        stale_approval["deal_digest"] = json!("0".repeat(64));
        assert!(matches!(
            complete_market_quote(&client, root, bearer, &supply_quote, &stale_approval),
            Err(DeskError::ServiceResponse { status: 409, ref code, .. })
                if code == "MARKET_DEAL_NOT_APPROVED"
        ));
        let action = state
            .action_for_key(key(KeyCode::Char('c'), KeyModifiers::NONE))
            .ok_or("c did not select mock completion")?;
        assert_eq!(action, MarketQuoteAction::Complete);
        state.handle_action(action, &client, root, bearer, comparison);
        assert!(state.message.contains("No workload ran and no funds moved"));
        let completion = state
            .completion
            .as_ref()
            .ok_or("TUI did not retain the mock completion")?;
        assert_eq!(
            completion["preview_digest"],
            supply_quote["body"]["quote_preview"]["digest"]
        );
        assert_eq!(completion["deal_digest"], approval["deal_digest"]);
        assert_eq!(
            completion["demo"]["report"]["deal_digest"],
            approval["deal_digest"]
        );
        assert!(completion["demo"]["receipt"]["digest"].as_str().is_some());

        let action = state
            .action_for_key(key(KeyCode::Char('c'), KeyModifiers::NONE))
            .ok_or("repeated c did not report the completed state")?;
        assert_eq!(action, MarketQuoteAction::CompletionAlreadyRecorded);
        state.handle_action(action, &client, root, bearer, comparison);
        assert!(state.message.contains("no second request was sent"));
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn market_tui_quote_actions_drive_the_digest_bound_synthetic_lifecycle()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::Duration;

        use hyperai_local_service::{AuthState, lifecycle, server, storage};

        let temporary = tempfile::tempdir()?;
        let home = temporary.path().join("home");
        let service_home = home.clone();
        let service = tokio::spawn(async move { server::serve(Some(&service_home), 0).await });
        let paths = storage::paths_for_home(Some(&home))?;
        let async_client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()?;
        let mut ready = None;
        for _ in 0..100 {
            if let Ok(record) = lifecycle::read_record(&paths)
                && async_client
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
        let root = format!("http://{}", record.address);
        let comparison: Value = async_client
            .get(format!("{root}/api/v1/market/supply-comparison"))
            .bearer_auth(&bearer)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let blocking_root = root.clone();
        let blocking_bearer = bearer.clone();
        tokio::task::spawn_blocking(move || {
            exercise_market_quote_lifecycle(&blocking_root, &blocking_bearer, &comparison)
        })
        .await??;

        async_client
            .post(format!("{root}/api/v1/service/stop"))
            .bearer_auth(&bearer)
            .json(&json!({}))
            .send()
            .await?
            .error_for_status()?;
        service.await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn data_tui_query_submits_edited_sql_through_authenticated_service()
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
            .timeout(Duration::from_secs(3))
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
        let preview: Value = client
            .get(format!("{base}/api/v1/data/synthetic/preview"))
            .bearer_auth(&bearer)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let seeded: Value = client
            .post(format!("{base}/api/v1/data/synthetic/seed"))
            .bearer_auth(&bearer)
            .header("idempotency-key", "tui-query-input-seed-v1")
            .json(&json!({ "preview_digest": preview["preview_digest"] }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let query_base = base.clone();
        let query_bearer = bearer.clone();
        let catalog = json!({
            "datasets": [{
                "dataset_id": "hyperai.synthetic_market_observations_json.v1"
            }]
        });
        let sql = "SELECT gpu_model FROM dataset ORDER BY gpu_model".to_owned();
        let result = tokio::task::spawn_blocking(move || {
            let client = Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(3))
                .build()?;
            run_data_query(&client, &query_base, &query_bearer, &catalog, 1, &sql)
        })
        .await??;

        assert_eq!(result["columns"], json!(["gpu_model"]));
        assert!(result["row_count"].as_u64().is_some_and(|rows| rows > 0));
        assert_eq!(result["snapshot_digest"], seeded["content_digest"]);

        client
            .post(format!("{base}/api/v1/service/stop"))
            .bearer_auth(&bearer)
            .json(&json!({}))
            .send()
            .await?
            .error_for_status()?;
        service.await??;
        Ok(())
    }

    #[test]
    fn service_error_preserves_status_code_and_message() {
        let error = service_response_error(
            409,
            &json!({
                "schema_version": "hyperai.interface.error.v1",
                "status": "error",
                "code": "FLEET_SIMULATION_CONFLICT",
                "message": "The local HyperAI contract operation could not complete."
            }),
        );

        assert_eq!(
            error.to_string(),
            "local service returned HTTP 409 (FLEET_SIMULATION_CONFLICT): The local HyperAI contract operation could not complete."
        );
    }

    #[test]
    fn service_error_rejects_invalid_envelopes_and_never_displays_body_message() {
        let error = service_response_error(
            400,
            &json!({
                "schema_version": "hyperai.interface.error.v1",
                "status": "error",
                "code": format!("{}\n", "C".repeat(90)),
                "message": format!("{}\u{1b}", "private".repeat(260))
            }),
        );

        assert_eq!(
            error.to_string(),
            "local service returned HTTP 400 (UNKNOWN_ERROR): The local HyperAI contract operation could not complete."
        );

        let valid = service_response_error(
            409,
            &json!({
                "schema_version": "hyperai.interface.error.v1",
                "status": "error",
                "code": "FLEET_SIMULATION_CONFLICT",
                "message": "private request detail"
            }),
        );
        assert_eq!(
            valid.to_string(),
            "local service returned HTTP 409 (FLEET_SIMULATION_CONFLICT): The local HyperAI contract operation could not complete."
        );
        assert!(!valid.to_string().contains("private"));
    }

    #[test]
    fn trade_summary_preserves_synthetic_research_claim_limits() {
        let catalog = json!({
            "dataset_revision": "hyperai-synthetic-a100-r1",
            "freshness": "synthetic_not_market_data",
            "observed_row_count": 23,
            "expected_row_count": 24,
            "fixture_coverage_bps": 9583,
            "dataset_digest": "fixture-digest",
            "method_versions": ["series-v1", "paper-v1"],
            "claim_ceiling": "synthetic only",
            "nonclaims": "No forecast or payment entitlement."
        });
        let series = json!({
            "instrument": {"instrument_id": "a100-us-east"},
            "observed_period_count": 7,
            "expected_period_count": 8,
            "missing_periods": ["2026-01-01T04:00:00Z"],
            "unknown_availability_count": 0,
            "zero_availability_count": 0,
            "mean_absolute_contiguous_change_bps": 100,
            "contiguous_change_sample_count": 5
        });
        let scenario = json!({
            "primary_instrument": {"instrument_id": "a100-us-east"},
            "comparison_instrument": {"instrument_id": "a100-us-west"},
            "observed_at": "2026-01-01T07:00:00Z",
            "regional_basis_usd_micros_per_hour": 10,
            "scenario_regional_basis_usd_micros_per_hour": 10,
            "primary_available_units": null,
            "scenario_available_units": null,
            "sensitivity": {
                "outlier_median_change_usd_micros_per_hour": 2,
                "one_period_lag_change_usd_micros_per_hour": -1
            }
        });
        let paper = json!({
            "comparable_period_count": 6,
            "unknown_capacity_observation_count": 1,
            "no_eligible_capacity_count": 1,
            "baseline_minus_strategy_asking_cost_microusd": 20
        });
        let summary = trade_workspace_text(
            "http://127.0.0.1:4317",
            TradeWorkspaceData {
                catalog: Some(&catalog),
                series: Some(&series),
                scenario: Some(&scenario),
                paper: Some(&paper),
            },
        );
        assert!(summary.contains("local synthetic research"));
        assert!(summary.contains("not forecast volatility"));
        assert!(summary.contains("unknown remains unknown"));
        assert!(summary.contains("not a fill or realized saving"));
        assert!(summary.contains("No forecast or payment entitlement."));
    }
}
