//! Local ACP v1 process and session lifecycle.

#![cfg_attr(
    not(any(unix, all(windows, feature = "windows-acp"))),
    allow(dead_code, unused_imports, unused_mut)
)]

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    io,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};

use agent_client_protocol::{
    Agent, ByteStreams, Client, ConnectionTo, Error as AcpError,
    schema::{
        ProtocolVersion,
        v1::{
            CancelNotification, CloseSessionRequest, ContentBlock, InitializeRequest, McpServer,
            NewSessionRequest, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
            RequestPermissionResponse, SessionId, SessionNotification, StopReason, TextContent,
        },
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::{
    AgentMcpConfigV1, AgentProfileSummaryV1, AgentProfileV1, MCP_TOOL_NAMES_V6, ProfileError,
};

const MAX_PROMPT_BYTES: usize = 16 * 1024;
const MAX_CAPTURED_EVENTS: usize = 32;
const MAX_EVENT_TEXT_BYTES: usize = 4 * 1024;
const MAX_STDERR_BYTES: usize = 64 * 1024;
const MAX_STDOUT_BYTES: usize = 8 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const PROCESS_STOP_TIMEOUT: Duration = Duration::from_secs(2);
const CLOSE_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);

/// A bounded assistant text update retained only in volatile session memory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEventV1 {
    /// Monotonic local sequence within this session.
    pub sequence: u64,
    /// Assistant text only; tool payloads and thought updates are omitted.
    pub text: String,
}

/// Sanitized view of the active session for browser, CLI, and TUI clients.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSnapshotV1 {
    /// Versioned status response.
    pub schema_version: &'static str,
    /// Current lifecycle state.
    pub status: String,
    /// Random local identifier for this volatile session.
    pub local_session_id: Option<String>,
    /// ACP v1 session identifier, when a session was created.
    pub session_id: Option<String>,
    /// Digest of the exact validated profile bytes used at connect time.
    pub profile_digest: Option<String>,
    /// Version of the fixed `HyperAI` tool allowlist.
    pub tool_set_version: &'static str,
    /// Fixed local synthetic/read-only data scope for this session.
    pub data_scope: &'static str,
    /// Negotiated protocol version.
    pub protocol_version: Option<u8>,
    /// Non-secret identity of the user-selected profile, held in memory only.
    pub profile: Option<AgentProfileSummaryV1>,
    /// Advertised agent capability labels, without opaque metadata.
    pub capabilities: Vec<String>,
    /// Fixed tools provided by the `HyperAI` MCP server.
    pub tools: Vec<String>,
    /// Bounded assistant text events held only in memory.
    pub events: Vec<SessionEventV1>,
    /// Sanitized last error category, never process stderr or raw protocol data.
    pub error_code: Option<String>,
}

impl Default for SessionSnapshotV1 {
    fn default() -> Self {
        Self {
            schema_version: "hyperai.acp.session.v1",
            status: "disconnected".to_owned(),
            local_session_id: None,
            session_id: None,
            profile_digest: None,
            tool_set_version: "hyperai.agent-tools.readonly.v6",
            data_scope: "local_synthetic_buyer_supply_quote_workload_trace_data_readonly_fleet_eval_trade",
            protocol_version: None,
            profile: None,
            capabilities: Vec::new(),
            tools: MCP_TOOL_NAMES_V6
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            events: Vec::new(),
            error_code: None,
        }
    }
}

/// ACP connection or session operation failed closed.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum SessionError {
    /// A profile failed validation.
    #[error("invalid agent profile")]
    InvalidProfile,
    /// The host cannot provide the required owned process-group lifecycle.
    #[error("ACP process isolation lifecycle is unsupported on this platform")]
    UnsupportedPlatform,
    /// A session is already active or an operation is already running.
    #[error("agent session is busy")]
    Busy,
    /// Prompt or event limit was exceeded.
    #[error("ACP session limit exceeded")]
    LimitExceeded,
    /// ACP initialization or session setup failed.
    #[error("ACP protocol operation failed")]
    Protocol,
    /// The agent negotiated a protocol version other than ACP v1.
    #[error("agent did not negotiate ACP v1")]
    VersionMismatch,
    /// Session handshake did not complete before the local timeout.
    #[error("ACP session connection timed out")]
    Timeout,
    /// The active connection closed before the operation completed.
    #[error("ACP connection closed")]
    Disconnected,
    /// The process wrote more than the allowed stderr budget.
    #[error("agent exceeded the stderr budget")]
    StderrLimit,
    /// The process could not be launched or reaped.
    #[error("agent process operation failed")]
    Process,
    /// The operating system did not provide local session identity randomness.
    #[error("local session identity could not be generated")]
    Random,
}

/// One active local ACP session manager. It permits one session at a time.
#[derive(Default)]
pub struct AgentSessionManager {
    active: tokio::sync::Mutex<Option<ActiveSession>>,
}

struct ActiveSession {
    commands: mpsc::Sender<SessionCommand>,
    snapshot: Arc<Mutex<SessionSnapshotV1>>,
    task: JoinHandle<()>,
}

enum SessionCommand {
    Prompt(String, oneshot::Sender<Result<(), SessionError>>),
    Cancel(oneshot::Sender<Result<(), SessionError>>),
    Disconnect(Option<oneshot::Sender<Result<(), SessionError>>>),
}

impl AgentSessionManager {
    /// Launch a user-selected local executable and establish one ACP v1 session.
    ///
    /// # Errors
    ///
    /// Returns an error when the profile is invalid, the local process cannot
    /// start, the session cannot negotiate ACP v1, or setup exceeds its limit.
    pub async fn connect(
        &self,
        profile: AgentProfileV1,
        mcp: AgentMcpConfigV1,
    ) -> Result<SessionSnapshotV1, SessionError> {
        profile
            .validate()
            .map_err(|_: ProfileError| SessionError::InvalidProfile)?;
        let local_session_id = random_session_id()?;
        let profile_digest = digest_profile(&profile)?;
        let mcp_server = mcp
            .into_schema()
            .map_err(|_| SessionError::InvalidProfile)?;
        let mut active = self.active.lock().await;
        let inactive = active.as_ref().is_some_and(|session| {
            matches!(
                snapshot_value(&session.snapshot).status.as_str(),
                "failed" | "disconnected"
            )
        });
        if inactive && let Some(session) = active.take() {
            let _ = session.task.await;
        }
        if active.is_some() {
            return Err(SessionError::Busy);
        }

        let (command_tx, command_rx) = mpsc::channel(4);
        let (ready_tx, ready_rx) = oneshot::channel();
        let snapshot = Arc::new(Mutex::new(SessionSnapshotV1 {
            status: "connecting".to_owned(),
            local_session_id: Some(local_session_id),
            profile_digest: Some(profile_digest),
            profile: Some(AgentProfileSummaryV1::from(&profile)),
            ..SessionSnapshotV1::default()
        }));
        let task_snapshot = Arc::clone(&snapshot);
        let task = tokio::spawn(async move {
            let result = run_session(
                profile,
                mcp_server,
                command_rx,
                task_snapshot.clone(),
                ready_tx,
            )
            .await;
            if let Err(error) = result {
                update_snapshot(&task_snapshot, |value| {
                    if value.status != "inconclusive" {
                        "failed".clone_into(&mut value.status);
                        value.error_code = Some(error_code(error).to_owned());
                    }
                });
            }
        });

        match timeout(CONNECT_TIMEOUT, ready_rx).await {
            Ok(Ok(Ok(()))) => {
                *active = Some(ActiveSession {
                    commands: command_tx,
                    snapshot: Arc::clone(&snapshot),
                    task,
                });
                Ok(snapshot_value(&snapshot))
            }
            Ok(Ok(Err(error))) => {
                task.abort();
                Err(error)
            }
            Ok(Err(_)) => {
                task.abort();
                Err(SessionError::Disconnected)
            }
            Err(_) => {
                task.abort();
                Err(SessionError::Timeout)
            }
        }
    }

    /// Send one bounded prompt through the established ACP session.
    ///
    /// # Errors
    ///
    /// Returns an error when no session is connected, the prompt exceeds its
    /// limit, another prompt is active, or the ACP request fails or times out.
    pub async fn prompt(&self, prompt: String) -> Result<SessionSnapshotV1, SessionError> {
        if prompt.trim().is_empty() || prompt.len() > MAX_PROMPT_BYTES {
            return Err(SessionError::LimitExceeded);
        }
        let active = self.active.lock().await;
        let session = active.as_ref().ok_or(SessionError::Disconnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        session
            .commands
            .send(SessionCommand::Prompt(prompt, reply_tx))
            .await
            .map_err(|_| SessionError::Disconnected)?;
        drop(active);
        timeout(CONNECT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| SessionError::Timeout)?
            .map_err(|_| SessionError::Disconnected)??;
        Ok(self.snapshot().await)
    }

    /// Request cancellation of the active prompt.
    ///
    /// # Errors
    ///
    /// Returns an error when no session is connected, no prompt can be
    /// cancelled, or the cancellation request fails or times out.
    pub async fn cancel(&self) -> Result<SessionSnapshotV1, SessionError> {
        let active = self.active.lock().await;
        let session = active.as_ref().ok_or(SessionError::Disconnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        session
            .commands
            .send(SessionCommand::Cancel(reply_tx))
            .await
            .map_err(|_| SessionError::Disconnected)?;
        drop(active);
        timeout(Duration::from_secs(3), reply_rx)
            .await
            .map_err(|_| SessionError::Timeout)?
            .map_err(|_| SessionError::Disconnected)??;
        Ok(self.snapshot().await)
    }

    /// Close the session and terminate its owned local process tree.
    ///
    /// # Errors
    ///
    /// Returns an error when process shutdown cannot be confirmed within the
    /// bounded cleanup interval.
    pub async fn disconnect(&self) -> Result<SessionSnapshotV1, SessionError> {
        let mut active = self.active.lock().await;
        let Some(mut session) = active.take() else {
            return Ok(SessionSnapshotV1::default());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = session
            .commands
            .send(SessionCommand::Disconnect(Some(reply_tx)))
            .await;
        drop(active);
        let _ = timeout(CLOSE_REQUEST_TIMEOUT + Duration::from_secs(1), reply_rx).await;
        match timeout(
            PROCESS_STOP_TIMEOUT.saturating_mul(2) + Duration::from_secs(1),
            &mut session.task,
        )
        .await
        {
            Ok(Ok(())) => {
                let snapshot = snapshot_value(&session.snapshot);
                if matches!(snapshot.status.as_str(), "disconnected" | "failed") {
                    Ok(snapshot)
                } else {
                    mark_shutdown_inconclusive(&session.snapshot);
                    *self.active.lock().await = Some(session);
                    Err(SessionError::Process)
                }
            }
            Ok(Err(_)) => {
                mark_shutdown_inconclusive(&session.snapshot);
                *self.active.lock().await = Some(session);
                Err(SessionError::Process)
            }
            Err(_) => {
                mark_shutdown_inconclusive(&session.snapshot);
                *self.active.lock().await = Some(session);
                Err(SessionError::Timeout)
            }
        }
    }

    /// Return the current sanitized lifecycle snapshot.
    pub async fn snapshot(&self) -> SessionSnapshotV1 {
        let active = self.active.lock().await;
        active
            .as_ref()
            .map_or_else(SessionSnapshotV1::default, |session| {
                snapshot_value(&session.snapshot)
            })
    }
}

#[allow(clippy::too_many_lines)] // The ACP handshake, request loop, and owned-process cleanup form one lifecycle.
#[allow(clippy::unused_async)] // The spawned session task keeps one async signature on unsupported targets.
async fn run_session(
    profile: AgentProfileV1,
    mcp_server: McpServer,
    mut commands: mpsc::Receiver<SessionCommand>,
    snapshot: Arc<Mutex<SessionSnapshotV1>>,
    ready: oneshot::Sender<Result<(), SessionError>>,
) -> Result<(), SessionError> {
    #[cfg(not(any(unix, all(windows, feature = "windows-acp"))))]
    {
        let _ = (profile, mcp_server, commands, snapshot);
        let _ = ready.send(Err(SessionError::UnsupportedPlatform));
        Err(SessionError::UnsupportedPlatform)
    }

    #[cfg(any(unix, all(windows, feature = "windows-acp")))]
    {
        #[cfg(unix)]
        let mut owned_process = spawn_owned_process(&profile)?;
        #[cfg(all(windows, feature = "windows-acp"))]
        let mut owned_process = spawn_owned_process(&profile).await?;
        let stdin = owned_process
            .child
            .stdin
            .take()
            .ok_or(SessionError::Process)?;
        let stdout = owned_process
            .child
            .stdout
            .take()
            .ok_or(SessionError::Process)?;
        let stderr = owned_process
            .child
            .stderr
            .take()
            .ok_or(SessionError::Process)?;
        let stderr_monitor = tokio::spawn(read_bounded_stderr(stderr));
        let events = Arc::clone(&snapshot);
        let session_snapshot = Arc::clone(&snapshot);
        let profile_cwd = profile.working_directory;
        let connection = Client
            .builder()
            .on_receive_notification(
                async move |notification: SessionNotification, _connection| {
                    capture_agent_text(&events, notification);
                    Ok(())
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                async move |_: RequestPermissionRequest, responder, _connection: ConnectionTo<Agent>| {
                    responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(
                ByteStreams::new(
                    stdin.compat_write(),
                    stdout.take(MAX_STDOUT_BYTES as u64).compat(),
                ),
                async move |connection: ConnectionTo<Agent>| {
                    let initialize = connection
                        .send_request(InitializeRequest::new(ProtocolVersion::V1))
                        .block_task()
                        .await?;
                    if initialize.protocol_version != ProtocolVersion::V1 {
                        let _ = ready.send(Err(SessionError::VersionMismatch));
                        return Err(AcpError::invalid_request());
                    }
                    let capability_value = serde_json::to_value(&initialize.agent_capabilities)
                        .unwrap_or(serde_json::Value::Null);
                    let capabilities = capability_labels(&capability_value);
                    let new_session = connection
                        .send_request(
                            NewSessionRequest::new(profile_cwd)
                                .mcp_servers(vec![mcp_server]),
                        )
                        .block_task()
                        .await?;
                    let session_id = new_session.session_id;
                    update_snapshot(&session_snapshot, |value| {
                        "ready".clone_into(&mut value.status);
                        value.protocol_version = Some(1);
                        value.session_id = Some(session_id_string(&session_id));
                        value.capabilities = capabilities;
                    });
                    let _ = ready.send(Ok(()));

                    loop {
                        match commands.recv().await {
                            Some(SessionCommand::Prompt(prompt, reply)) => {
                                update_snapshot(&session_snapshot, |value| {
                                    "running".clone_into(&mut value.status);
                                    value.error_code = None;
                                });
                                let prompt_request = PromptRequest::new(
                                    session_id.clone(),
                                    vec![ContentBlock::Text(TextContent::new(prompt))],
                                );
                                let prompt_future = connection
                                    .send_request(prompt_request)
                                    .block_task();
                                tokio::pin!(prompt_future);
                                loop {
                                    tokio::select! {
                                        result = &mut prompt_future => {
                                            let response = result.map_err(|_| SessionError::Protocol);
                                            let next_status = match &response {
                                                Ok(response) if response.stop_reason == StopReason::Cancelled => "cancelled",
                                                Ok(_) => "ready",
                                                Err(_) => "failed",
                                            };
                                            update_snapshot(&session_snapshot, |value| {
                                                next_status.clone_into(&mut value.status);
                                                if response.is_err() {
                                                    value.error_code = Some("ACP_PROMPT_FAILED".to_owned());
                                                }
                                            });
                                            let _ = reply.send(response.map(|_| ()));
                                            break;
                                        }
                                        command = commands.recv() => {
                                            match command {
                                                Some(SessionCommand::Cancel(cancel_reply)) => {
                                                    connection
                                                        .send_notification(CancelNotification::new(session_id.clone()))
                                                        .map_err(|_| AcpError::internal_error())?;
                                                    update_snapshot(&session_snapshot, |value| {
                                                        "cancellation_requested".clone_into(&mut value.status);
                                                    });
                                                    let _ = cancel_reply.send(Ok(()));
                                                }
                                                Some(SessionCommand::Disconnect(disconnect_reply)) => {
                                                    let _ = connection
                                                        .send_notification(CancelNotification::new(session_id.clone()));
                                                    if initialize.agent_capabilities.session_capabilities.close.is_some() {
                                                        let close_request = connection
                                                            .send_request(CloseSessionRequest::new(session_id.clone()))
                                                            .block_task();
                                                        let _ = timeout(CLOSE_REQUEST_TIMEOUT, close_request).await;
                                                    }
                                                    let _ = reply.send(Err(SessionError::Disconnected));
                                                    update_snapshot(&session_snapshot, |value| {
                                                        "disconnecting".clone_into(&mut value.status);
                                                    });
                                                    if let Some(disconnect_reply) = disconnect_reply {
                                                        let _ = disconnect_reply.send(Ok(()));
                                                    }
                                                    return Ok(());
                                                }
                                                Some(SessionCommand::Prompt(_, queued_reply)) => {
                                                    let _ = queued_reply.send(Err(SessionError::Busy));
                                                }
                                                None => return Err(AcpError::internal_error()),
                                            }
                                        }
                                    }
                                }
                            }
                            Some(SessionCommand::Cancel(reply)) => {
                                let _ = reply.send(Err(SessionError::Busy));
                            }
                            Some(SessionCommand::Disconnect(reply)) => {
                                if initialize.agent_capabilities.session_capabilities.close.is_some() {
                                    let close_request = connection
                                        .send_request(CloseSessionRequest::new(session_id.clone()))
                                        .block_task();
                                    let _ = timeout(CLOSE_REQUEST_TIMEOUT, close_request).await;
                                }
                                update_snapshot(&session_snapshot, |value| {
                                    "disconnecting".clone_into(&mut value.status);
                                });
                                if let Some(reply) = reply {
                                    let _ = reply.send(Ok(()));
                                }
                                return Ok(());
                            }
                            None => return Err(AcpError::internal_error()),
                        }
                    }
                },
            );

        let result = tokio::select! {
            result = connection => result.map_err(|_| SessionError::Protocol),
            stderr_result = stderr_monitor => {
                match stderr_result {
                    Ok(Ok(())) => Err(SessionError::Disconnected),
                    Ok(Err(_)) => Err(SessionError::StderrLimit),
                    Err(_) => Err(SessionError::Process),
                }
            }
        };
        if let Err(error) = owned_process.stop().await {
            mark_shutdown_inconclusive(&snapshot);
            return Err(error);
        }
        result?;
        update_snapshot(&snapshot, |value| {
            "disconnected".clone_into(&mut value.status);
        });
        Ok(())
    }
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
struct OwnedAgentProcess {
    child: Child,
    #[cfg(unix)]
    process_group: ProcessGroupGuard,
    #[cfg(all(windows, feature = "windows-acp"))]
    job: subc_jobobject::JobObject,
}

#[cfg(unix)]
fn spawn_owned_process(profile: &AgentProfileV1) -> Result<OwnedAgentProcess, SessionError> {
    let mut command = Command::new(&profile.executable);
    command
        .args(&profile.args)
        .current_dir(&profile.working_directory)
        .env_clear()
        .envs(&profile.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command.as_std_mut().process_group(0);
    let child = command.spawn().map_err(|_| SessionError::Process)?;
    let pid = child.id().ok_or(SessionError::Process)?;
    Ok(OwnedAgentProcess {
        child,
        process_group: ProcessGroupGuard { pid, armed: true },
    })
}

#[cfg(all(windows, feature = "windows-acp"))]
async fn spawn_owned_process(profile: &AgentProfileV1) -> Result<OwnedAgentProcess, SessionError> {
    // Create the job first, then assign the suspended child before it can spawn
    // descendants. Do not use subc-jobobject's spawn_contained helper: it creates
    // the job after spawning and its error path can leave a suspended child.
    let job = subc_jobobject::JobObject::new().map_err(|_| SessionError::Process)?;
    let mut command = Command::new(&profile.executable);
    command
        .args(&profile.args)
        .current_dir(&profile.working_directory)
        .env_clear()
        .envs(&profile.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    subc_jobobject::suspend_on_create_async(&mut command);
    let mut child = command.spawn().map_err(|_| SessionError::Process)?;
    let pid = child.id().ok_or(SessionError::Process)?;
    if job.assign(&child).is_err() {
        let _ = child.kill().await;
        return Err(SessionError::Process);
    }
    if subc_jobobject::resume_main_thread(pid).is_err() {
        let _ = job.terminate();
        let _ = child.kill().await;
        return Err(SessionError::Process);
    }
    Ok(OwnedAgentProcess { child, job })
}

#[cfg(any(unix, all(windows, feature = "windows-acp")))]
impl OwnedAgentProcess {
    async fn stop(&mut self) -> Result<(), SessionError> {
        #[cfg(unix)]
        {
            stop_process_group(&mut self.child, self.process_group.pid).await?;
            self.process_group.disarm();
        }
        #[cfg(all(windows, feature = "windows-acp"))]
        stop_windows_job(&mut self.job, &mut self.child).await?;
        Ok(())
    }
}

#[cfg(unix)]
struct ProcessGroupGuard {
    pid: u32,
    armed: bool,
}

#[cfg(unix)]
impl ProcessGroupGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed
            && let Ok(raw) = i32::try_from(self.pid)
            && let Some(pid) = rustix::process::Pid::from_raw(raw)
        {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
    }
}

#[cfg(unix)]
async fn stop_process_group(child: &mut Child, pid: u32) -> Result<(), SessionError> {
    let raw = i32::try_from(pid).map_err(|_| SessionError::Process)?;
    let group = rustix::process::Pid::from_raw(raw).ok_or(SessionError::Process)?;
    let _ = rustix::process::kill_process_group(group, rustix::process::Signal::TERM);
    if timeout(PROCESS_STOP_TIMEOUT, child.wait()).await.is_err() {
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
    }
    timeout(PROCESS_STOP_TIMEOUT, child.wait())
        .await
        .map_err(|_| SessionError::Process)?
        .map_err(|_| SessionError::Process)?;
    Ok(())
}

#[cfg(all(windows, feature = "windows-acp"))]
async fn stop_windows_job(
    job: &mut subc_jobobject::JobObject,
    child: &mut Child,
) -> Result<(), SessionError> {
    if let Ok(Err(_)) = timeout(PROCESS_STOP_TIMEOUT, child.wait()).await {
        return Err(SessionError::Process);
    }

    if job.process_count().map_err(|_| SessionError::Process)? > 0 {
        job.terminate().map_err(|_| SessionError::Process)?;
        timeout(PROCESS_STOP_TIMEOUT, child.wait())
            .await
            .map_err(|_| SessionError::Process)?
            .map_err(|_| SessionError::Process)?;
    }

    let deadline = tokio::time::Instant::now() + PROCESS_STOP_TIMEOUT;
    loop {
        if job.process_count().map_err(|_| SessionError::Process)? == 0 {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(SessionError::Process);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn read_bounded_stderr<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> io::Result<()> {
    let mut total = 0_usize;
    let mut buffer = [0_u8; 4_096];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(());
        }
        total = total.saturating_add(count);
        if total > MAX_STDERR_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stderr limit exceeded",
            ));
        }
    }
}

fn capture_agent_text(snapshot: &Arc<Mutex<SessionSnapshotV1>>, notification: SessionNotification) {
    let Ok(value) = serde_json::to_value(notification) else {
        return;
    };
    let update = value.get("update");
    let is_agent_text = update
        .and_then(|update| update.get("sessionUpdate"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| kind == "agent_message_chunk");
    if !is_agent_text {
        return;
    }
    let Some(text) = update
        .and_then(|update| update.get("content"))
        .and_then(|content| content.get("text"))
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };
    let bounded = bounded_text(text, MAX_EVENT_TEXT_BYTES);
    update_snapshot(snapshot, |state| {
        if state.events.len() == MAX_CAPTURED_EVENTS {
            state.events.remove(0);
        }
        let sequence = state
            .events
            .last()
            .map_or(1, |event| event.sequence.saturating_add(1));
        state.events.push(SessionEventV1 {
            sequence,
            text: bounded,
        });
    });
}

fn capability_labels(value: &serde_json::Value) -> Vec<String> {
    let mut labels = Vec::new();
    if let Some(object) = value.as_object() {
        for (name, enabled) in object {
            if enabled.as_bool() == Some(true) {
                labels.push(name.clone());
            }
        }
        if let Some(sessions) = object
            .get("sessionCapabilities")
            .and_then(|item| item.as_object())
        {
            labels.extend(sessions.keys().map(|name| format!("session.{name}")));
        }
    }
    labels.sort();
    labels.dedup();
    labels
}

fn session_id_string(value: &SessionId) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_else(|| "session-id-unavailable".to_owned())
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    let mut bounded = String::new();
    for character in value.chars() {
        if bounded.len().saturating_add(character.len_utf8()) > max_bytes {
            break;
        }
        bounded.push(character);
    }
    bounded
}

fn snapshot_value(snapshot: &Arc<Mutex<SessionSnapshotV1>>) -> SessionSnapshotV1 {
    snapshot
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default()
}

fn update_snapshot(
    snapshot: &Arc<Mutex<SessionSnapshotV1>>,
    update: impl FnOnce(&mut SessionSnapshotV1),
) {
    if let Ok(mut value) = snapshot.lock() {
        update(&mut value);
    }
}

fn mark_shutdown_inconclusive(snapshot: &Arc<Mutex<SessionSnapshotV1>>) {
    update_snapshot(snapshot, |value| {
        "inconclusive".clone_into(&mut value.status);
        value.error_code = Some("ACP_SHUTDOWN_UNCONFIRMED".to_owned());
    });
}

fn error_code(error: SessionError) -> &'static str {
    match error {
        SessionError::InvalidProfile => "ACP_PROFILE_INVALID",
        SessionError::UnsupportedPlatform => "ACP_PLATFORM_UNSUPPORTED",
        SessionError::Busy => "ACP_SESSION_BUSY",
        SessionError::LimitExceeded => "ACP_LIMIT_EXCEEDED",
        SessionError::Protocol => "ACP_PROTOCOL_FAILED",
        SessionError::VersionMismatch => "ACP_VERSION_MISMATCH",
        SessionError::Timeout => "ACP_TIMEOUT",
        SessionError::Disconnected => "ACP_DISCONNECTED",
        SessionError::StderrLimit => "ACP_STDERR_LIMIT",
        SessionError::Process => "ACP_PROCESS_FAILED",
        SessionError::Random => "ACP_RANDOM_FAILED",
    }
}

fn random_session_id() -> Result<String, SessionError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| SessionError::Random)?;
    Ok(hex_string(&bytes))
}

fn hex_string(bytes: &[u8]) -> String {
    let alphabet = b"0123456789abcdef";
    let mut id = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes.iter().copied() {
        id.push(alphabet[usize::from(byte >> 4)] as char);
        id.push(alphabet[usize::from(byte & 0x0f)] as char);
    }
    id
}

fn digest_profile(profile: &AgentProfileV1) -> Result<String, SessionError> {
    let bytes = serde_json::to_vec(profile).map_err(|_| SessionError::InvalidProfile)?;
    Ok(hex_string(&Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{SessionUpdate, ToolCall};

    #[test]
    fn event_text_is_bounded_at_a_utf8_boundary() {
        assert_eq!(bounded_text("ééé", 3), "é");
    }

    #[test]
    fn event_history_keeps_only_the_latest_bounded_entries() -> Result<(), serde_json::Error> {
        let snapshot = Arc::new(Mutex::new(SessionSnapshotV1::default()));
        for sequence in 1..=40 {
            let notification: SessionNotification = serde_json::from_value(serde_json::json!({
                "sessionId": "synthetic-session-v1",
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": { "type": "text", "text": format!("event-{sequence}") }
                }
            }))?;
            capture_agent_text(&snapshot, notification);
        }

        let events = snapshot_value(&snapshot).events;
        assert_eq!(events.len(), MAX_CAPTURED_EVENTS);
        assert_eq!(events.first().map(|event| event.sequence), Some(9));
        assert_eq!(events.last().map(|event| event.sequence), Some(40));
        assert_eq!(
            events.first().map(|event| event.text.as_str()),
            Some("event-9")
        );
        Ok(())
    }

    #[test]
    fn tool_raw_input_and_output_are_not_captured() -> Result<(), serde_json::Error> {
        let snapshot = Arc::new(Mutex::new(SessionSnapshotV1::default()));
        let canary = "ACP_TOOL_CONTENT_CANARY_DO_NOT_RETAIN";
        let notification = SessionNotification::new(
            "synthetic-session-v1",
            SessionUpdate::ToolCall(
                ToolCall::new("synthetic-tool-call", "Synthetic read-only query")
                    .raw_input(serde_json::json!({"query": canary}))
                    .raw_output(serde_json::json!({"result": canary})),
            ),
        );

        capture_agent_text(&snapshot, notification);

        let value = snapshot_value(&snapshot);
        let serialized = serde_json::to_string(&value)?;
        assert!(value.events.is_empty());
        assert!(!serialized.contains(canary));
        Ok(())
    }

    #[test]
    fn snapshots_have_a_fixed_read_only_tool_set() {
        let snapshot = SessionSnapshotV1::default();
        assert_eq!(
            snapshot.tools,
            MCP_TOOL_NAMES_V6
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(snapshot.status, "disconnected");
    }

    #[cfg(not(any(unix, all(windows, feature = "windows-acp"))))]
    #[tokio::test]
    async fn unsupported_platform_refuses_to_launch_an_agent()
    -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let executable = std::env::current_exe()?;
        let profile = AgentProfileV1 {
            name: "HyperAI unsupported-platform fixture".to_owned(),
            executable: executable.clone(),
            args: Vec::new(),
            working_directory: temporary.path().to_path_buf(),
            environment: std::collections::BTreeMap::default(),
        };
        let mcp = AgentMcpConfigV1 {
            endpoint: "http://127.0.0.1:4317".to_owned(),
            capability: "synthetic-read-only-capability-0123456789".to_owned(),
            executable,
        };

        let manager = AgentSessionManager::default();
        assert!(matches!(
            manager.connect(profile, mcp).await,
            Err(SessionError::UnsupportedPlatform)
        ));
        Ok(())
    }
}
