//! Metadata-only local execution journal and crash-state classification.
//!
//! This crate stores identities and timestamps, never request or response
//! bodies. Journal and replay databases are deliberately separate; no
//! cross-database atomicity is claimed.

use std::{path::Path, str::FromStr, time::Duration};

use hyperintel_core::{
    CanonicalizationError, ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use hyperintel_execution::{
    LocalExecutionContextV1, LocalExecutionError, LocalExecutionOutcomeV1, LocalProviderAgentV1,
    ProviderReceiptSignerV1, execute_local_chat_completion_v1,
};
use hyperintel_market::validate_buyer_replay_state_v1;
use hyperintel_persistence::{
    AuthenticatedReplayConsumptionV1, DURABLE_REPLAY_STATE_SLICE_V1, ReplayPersistenceError,
    SqliteReplayStoreV1,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

/// State slice emitted by the durable execution journal.
pub const EXECUTION_JOURNAL_STATE_SLICE_V1: &str = "hyperintel-durable-execution-journal-v1";

const JOURNAL_SCHEMA_V1: &str = r"
CREATE TABLE execution_starts (
  admission_digest TEXT PRIMARY KEY,
  start_digest TEXT NOT NULL UNIQUE,
  buyer_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  started_at TEXT NOT NULL
);

CREATE TABLE execution_terminals (
  admission_digest TEXT PRIMARY KEY REFERENCES execution_starts(admission_digest) ON DELETE RESTRICT,
  terminal_digest TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  finished_at TEXT NOT NULL
);
";

/// Metadata required to construct one journal start record.
pub struct ExecutionJournalStartDraftV1 {
    /// Exact authenticated admission.
    pub admission_digest: Sha256Digest,
    /// Authenticated buyer.
    pub buyer_id: ProtocolId,
    /// Selected provider.
    pub provider_id: ProtocolId,
    /// Exact prompt commitment, never the prompt body.
    pub prompt_digest: Sha256Digest,
    /// Explicit start time.
    pub started_at: UtcTimestamp,
}

/// Metadata-only start record written before local execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionJournalStartV1 {
    /// Fixed schema.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Stable local attempt identifier.
    pub attempt_id: String,
    /// Exact authenticated admission.
    pub admission_digest: Sha256Digest,
    /// Authenticated buyer.
    pub buyer_id: ProtocolId,
    /// Selected provider.
    pub provider_id: ProtocolId,
    /// Exact prompt commitment, never the prompt body.
    pub prompt_digest: Sha256Digest,
    /// Explicit start time.
    pub started_at: UtcTimestamp,
    /// Journal records grant no execution authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub start_digest: Sha256Digest,
}

impl ExecutionJournalStartV1 {
    /// Recompute the start identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        start_digest(self)
    }
}

/// Closed terminal classification retained without arbitrary error text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionJournalTerminalStatusV1 {
    /// The local harness returned completed evidence.
    Completed,
    /// The local harness returned typed incomplete evidence.
    Incomplete,
    /// The harness rejected before durable replay consumption.
    RejectedBeforeConsumption,
    /// The harness failed after durable replay consumption.
    FailedAfterConsumption,
}

/// Metadata required to construct one journal terminal record.
pub struct ExecutionJournalTerminalDraftV1 {
    /// Exact replay consumption when the nonce advanced.
    pub consumption_digest: Option<Sha256Digest>,
    /// Exact completed or incomplete local outcome when returned.
    pub outcome_digest: Option<Sha256Digest>,
    /// Closed terminal classification.
    pub status: ExecutionJournalTerminalStatusV1,
    /// Explicit terminal time.
    pub finished_at: UtcTimestamp,
}

/// Metadata-only terminal record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionJournalTerminalV1 {
    /// Fixed schema.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Exact start record.
    pub start_digest: Sha256Digest,
    /// Exact authenticated admission.
    pub admission_digest: Sha256Digest,
    /// Replay consumption when present.
    pub consumption_digest: Option<Sha256Digest>,
    /// Completed or incomplete outcome when present.
    pub outcome_digest: Option<Sha256Digest>,
    /// Closed terminal classification.
    pub status: ExecutionJournalTerminalStatusV1,
    /// Explicit terminal time.
    pub finished_at: UtcTimestamp,
    /// Journal records grant no settlement or execution authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub terminal_digest: Sha256Digest,
}

impl ExecutionJournalTerminalV1 {
    /// Recompute the terminal identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        terminal_digest(self)
    }
}

/// Result of an idempotent journal insert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalWriteOutcomeV1 {
    /// A new immutable record was inserted.
    Inserted,
    /// The exact immutable record already existed.
    ExistingIdentical,
}

/// Restart classification from journal and separately supplied replay state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionRecoveryStateV1 {
    /// Neither journal start nor replay consumption exists.
    NotStarted,
    /// Replay consumption exists without a journal start; never retry blindly.
    ConsumedWithoutStart(AuthenticatedReplayConsumptionV1),
    /// Start exists and replay was not consumed; caller may explicitly resume.
    StartedNotConsumed(ExecutionJournalStartV1),
    /// Replay was consumed but no terminal was retained; never retry blindly.
    ConsumedWithoutTerminal {
        /// Exact start record.
        start: ExecutionJournalStartV1,
        /// Exact durable replay consumption.
        consumption: AuthenticatedReplayConsumptionV1,
    },
    /// A terminal record exists.
    Terminal {
        /// Exact start record.
        start: ExecutionJournalStartV1,
        /// Exact terminal record.
        terminal: ExecutionJournalTerminalV1,
    },
}

/// Journaled successful return from the local execution harness.
pub struct JournaledLocalExecutionV1 {
    /// Durable start record.
    pub start: ExecutionJournalStartV1,
    /// Durable terminal record.
    pub terminal: ExecutionJournalTerminalV1,
    /// In-memory local execution result. Bodies are not journaled.
    pub execution: LocalExecutionOutcomeV1,
}

/// Durable execution journal failures.
#[derive(Debug, Error)]
pub enum ExecutionJournalError {
    /// Database operation failed.
    #[error("execution journal database failed: {0}")]
    Database(#[from] sqlx::Error),
    /// JSON encoding or decoding failed.
    #[error("execution journal JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// Canonical identity construction failed.
    #[error("execution journal canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Migration checksum differs.
    #[error("execution journal migration checksum conflict")]
    MigrationChecksumConflict,
    /// Existing immutable identity has different semantics.
    #[error("execution journal immutable identity conflict")]
    IdentityConflict,
    /// Stored columns differ from the canonical payload.
    #[error("execution journal stored binding mismatch")]
    StoredBindingMismatch,
    /// Record schema, digest, authority, or terminal shape is invalid.
    #[error("execution journal record is invalid")]
    InvalidRecord,
    /// File-backed pool size must be positive.
    #[error("execution journal connection count must be positive")]
    InvalidConnectionCount,
    /// Admission already has a terminal record.
    #[error("execution journal admission is already terminal")]
    AlreadyTerminal,
    /// Admission already has a non-terminal start and cannot resume implicitly.
    #[error("execution journal admission already started and requires reconciliation")]
    AlreadyStarted,
    /// Admission was consumed without a terminal and cannot execute again.
    #[error("execution journal detected consumed admission without terminal")]
    ConsumedWithoutTerminal,
    /// Durable replay lookup failed.
    #[error("execution journal replay lookup failed: {0}")]
    Replay(#[from] ReplayPersistenceError),
    /// Local execution failed after its terminal classification was retained.
    #[error("journaled local execution failed: {0}")]
    Execution(#[from] LocalExecutionError),
}

/// Separate local `SQLite` metadata journal.
#[derive(Clone, Debug)]
pub struct SqliteExecutionJournalV1 {
    pool: SqlitePool,
}

impl SqliteExecutionJournalV1 {
    /// Open a shared one-connection in-memory journal.
    ///
    /// # Errors
    ///
    /// Returns when `SQLite` cannot initialize the pool.
    pub async fn in_memory() -> Result<Self, ExecutionJournalError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .foreign_keys(true)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Open an operator-selected local journal file.
    ///
    /// # Errors
    ///
    /// Returns when the connection count is zero or the file cannot be opened.
    pub async fn connect_path(
        path: &Path,
        max_connections: u32,
    ) -> Result<Self, ExecutionJournalError> {
        if max_connections == 0 {
            return Err(ExecutionJournalError::InvalidConnectionCount);
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Apply and checksum the fixed journal schema.
    ///
    /// # Errors
    ///
    /// Returns on database failure or migration checksum conflict.
    pub async fn migrate(&self) -> Result<(), ExecutionJournalError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS execution_journal_migrations (\
             version INTEGER PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL)",
        )
        .execute(&self.pool)
        .await?;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let checksum = domain_separated_digest(
            "hyperintel.sqlite-execution-journal-migration.v1",
            &(1_i64, JOURNAL_SCHEMA_V1),
        )?
        .to_hex();
        let row =
            sqlx::query("SELECT checksum FROM execution_journal_migrations WHERE version = 1")
                .fetch_optional(&mut *transaction)
                .await?;
        if let Some(row) = row {
            let stored: String = row.try_get("checksum")?;
            if stored != checksum {
                return Err(ExecutionJournalError::MigrationChecksumConflict);
            }
        } else {
            sqlx::raw_sql(JOURNAL_SCHEMA_V1)
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "INSERT INTO execution_journal_migrations(version, checksum, applied_at) \
                 VALUES(1, ?, '2026-07-21T00:00:00Z')",
            )
            .bind(checksum)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Insert one immutable start or accept an exact retry.
    ///
    /// # Errors
    ///
    /// Returns when the record is invalid, conflicts with an existing identity,
    /// or cannot be persisted.
    pub async fn begin(
        &self,
        start: &ExecutionJournalStartV1,
    ) -> Result<JournalWriteOutcomeV1, ExecutionJournalError> {
        validate_start(start)?;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let existing = sqlx::query(
            "SELECT admission_digest, start_digest, buyer_id, provider_id, payload_json, \
             started_at FROM execution_starts WHERE admission_digest = ?",
        )
        .bind(start.admission_digest.to_hex())
        .fetch_optional(&mut *transaction)
        .await?
        .map(|row| decode_start(&row))
        .transpose()?;
        if let Some(existing) = existing {
            transaction.commit().await?;
            return if existing == *start {
                Ok(JournalWriteOutcomeV1::ExistingIdentical)
            } else {
                Err(ExecutionJournalError::IdentityConflict)
            };
        }
        sqlx::query(
            "INSERT INTO execution_starts(admission_digest, start_digest, buyer_id, \
             provider_id, payload_json, started_at) VALUES(?, ?, ?, ?, ?, ?)",
        )
        .bind(start.admission_digest.to_hex())
        .bind(start.start_digest.to_hex())
        .bind(start.buyer_id.as_str())
        .bind(start.provider_id.as_str())
        .bind(serde_json::to_string(start)?)
        .bind(start.started_at.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(JournalWriteOutcomeV1::Inserted)
    }

    /// Insert one immutable terminal bound to the actual replay store, or
    /// accept an exact retry.
    ///
    /// # Errors
    ///
    /// Returns when the record is invalid, conflicts with an existing identity,
    /// differs from the durable replay record, lacks its exact start, or cannot
    /// be persisted.
    pub async fn finish(
        &self,
        terminal: &ExecutionJournalTerminalV1,
        replay_store: &SqliteReplayStoreV1,
    ) -> Result<JournalWriteOutcomeV1, ExecutionJournalError> {
        let consumption = replay_store.consumption(terminal.admission_digest).await?;
        self.finish_with_consumption(terminal, consumption.as_ref())
            .await
    }

    async fn finish_with_consumption(
        &self,
        terminal: &ExecutionJournalTerminalV1,
        consumption: Option<&AuthenticatedReplayConsumptionV1>,
    ) -> Result<JournalWriteOutcomeV1, ExecutionJournalError> {
        validate_terminal(terminal)?;
        validate_optional_consumption(terminal.admission_digest, consumption)?;
        if terminal.consumption_digest != consumption.map(|value| value.consumption_digest) {
            return Err(ExecutionJournalError::StoredBindingMismatch);
        }
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let start = sqlx::query(
            "SELECT admission_digest, start_digest, buyer_id, provider_id, payload_json, \
             started_at FROM execution_starts WHERE admission_digest = ?",
        )
        .bind(terminal.admission_digest.to_hex())
        .fetch_optional(&mut *transaction)
        .await?
        .map(|row| decode_start(&row))
        .transpose()?
        .ok_or(ExecutionJournalError::StoredBindingMismatch)?;
        if start.start_digest != terminal.start_digest
            || terminal.finished_at < start.started_at
            || consumption.is_some_and(|value| value.buyer_id != start.buyer_id)
        {
            return Err(ExecutionJournalError::StoredBindingMismatch);
        }
        let existing = sqlx::query(
            "SELECT admission_digest, terminal_digest, status, payload_json, finished_at \
             FROM execution_terminals WHERE admission_digest = ?",
        )
        .bind(terminal.admission_digest.to_hex())
        .fetch_optional(&mut *transaction)
        .await?
        .map(|row| decode_terminal(&row))
        .transpose()?;
        if let Some(existing) = existing {
            transaction.commit().await?;
            return if existing == *terminal {
                Ok(JournalWriteOutcomeV1::ExistingIdentical)
            } else {
                Err(ExecutionJournalError::IdentityConflict)
            };
        }
        sqlx::query(
            "INSERT INTO execution_terminals(admission_digest, terminal_digest, status, \
             payload_json, finished_at) VALUES(?, ?, ?, ?, ?)",
        )
        .bind(terminal.admission_digest.to_hex())
        .bind(terminal.terminal_digest.to_hex())
        .bind(status_name(terminal.status))
        .bind(serde_json::to_string(terminal)?)
        .bind(terminal.finished_at.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(JournalWriteOutcomeV1::Inserted)
    }

    /// Read and validate a start by admission.
    ///
    /// # Errors
    ///
    /// Returns on database, decoding, digest, or stored-column mismatch.
    pub async fn start(
        &self,
        admission: Sha256Digest,
    ) -> Result<Option<ExecutionJournalStartV1>, ExecutionJournalError> {
        let row = sqlx::query(
            "SELECT admission_digest, start_digest, buyer_id, provider_id, payload_json, \
             started_at FROM execution_starts WHERE admission_digest = ?",
        )
        .bind(admission.to_hex())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| decode_start(&row)).transpose()
    }

    /// Read and validate a terminal by admission.
    ///
    /// # Errors
    ///
    /// Returns on database, decoding, digest, or stored-column mismatch.
    pub async fn terminal(
        &self,
        admission: Sha256Digest,
    ) -> Result<Option<ExecutionJournalTerminalV1>, ExecutionJournalError> {
        let row = sqlx::query(
            "SELECT admission_digest, terminal_digest, status, payload_json, finished_at \
             FROM execution_terminals WHERE admission_digest = ?",
        )
        .bind(admission.to_hex())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| decode_terminal(&row)).transpose()
    }

    /// Classify restart state by querying the durable replay store.
    ///
    /// # Errors
    ///
    /// Returns when either durable store fails or their records do not bind
    /// exactly to each other.
    pub async fn recovery_state(
        &self,
        admission: Sha256Digest,
        replay_store: &SqliteReplayStoreV1,
    ) -> Result<ExecutionRecoveryStateV1, ExecutionJournalError> {
        let consumption = replay_store.consumption(admission).await?;
        self.classify_recovery_state(admission, consumption.as_ref())
            .await
    }

    async fn classify_recovery_state(
        &self,
        admission: Sha256Digest,
        consumption: Option<&AuthenticatedReplayConsumptionV1>,
    ) -> Result<ExecutionRecoveryStateV1, ExecutionJournalError> {
        validate_optional_consumption(admission, consumption)?;
        let start = self.start(admission).await?;
        let terminal = self.terminal(admission).await?;
        if let (Some(start), Some(consumption)) = (&start, consumption)
            && start.buyer_id != consumption.buyer_id
        {
            return Err(ExecutionJournalError::StoredBindingMismatch);
        }
        if let (Some(start), Some(terminal)) = (&start, &terminal)
            && (terminal.start_digest != start.start_digest
                || terminal.finished_at < start.started_at)
        {
            return Err(ExecutionJournalError::StoredBindingMismatch);
        }
        if terminal.as_ref().is_some_and(|value| {
            value.consumption_digest != consumption.map(|item| item.consumption_digest)
        }) {
            return Err(ExecutionJournalError::StoredBindingMismatch);
        }
        match (start, terminal, consumption) {
            (None, None, None) => Ok(ExecutionRecoveryStateV1::NotStarted),
            (None, None, Some(value)) => Ok(ExecutionRecoveryStateV1::ConsumedWithoutStart(
                value.clone(),
            )),
            (Some(start), None, None) => Ok(ExecutionRecoveryStateV1::StartedNotConsumed(start)),
            (Some(start), None, Some(value)) => {
                Ok(ExecutionRecoveryStateV1::ConsumedWithoutTerminal {
                    start,
                    consumption: value.clone(),
                })
            }
            (Some(start), Some(terminal), _) => {
                Ok(ExecutionRecoveryStateV1::Terminal { start, terminal })
            }
            _ => Err(ExecutionJournalError::StoredBindingMismatch),
        }
    }

    /// Checkpoint and close the journal.
    ///
    /// # Errors
    ///
    /// Returns when the local WAL checkpoint fails.
    pub async fn checkpoint_and_close(self) -> Result<(), ExecutionJournalError> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await?;
        self.pool.close().await;
        Ok(())
    }
}

/// Build one canonical metadata-only start.
///
/// # Errors
///
/// Returns when required commitments are zero or canonicalization fails.
pub fn build_execution_journal_start_v1(
    draft: ExecutionJournalStartDraftV1,
) -> Result<ExecutionJournalStartV1, ExecutionJournalError> {
    if draft.admission_digest.is_zero() || draft.prompt_digest.is_zero() {
        return Err(ExecutionJournalError::InvalidRecord);
    }
    let mut start = ExecutionJournalStartV1 {
        schema_version: "hyperintel.execution-journal-start.v1".to_owned(),
        state_slice: EXECUTION_JOURNAL_STATE_SLICE_V1.to_owned(),
        attempt_id: format!("attempt_{}", &draft.admission_digest.to_hex()[..32]),
        admission_digest: draft.admission_digest,
        buyer_id: draft.buyer_id,
        provider_id: draft.provider_id,
        prompt_digest: draft.prompt_digest,
        started_at: draft.started_at,
        authority_granted: false,
        start_digest: Sha256Digest::from_bytes([0; 32]),
    };
    start.start_digest = start_digest(&start)?;
    Ok(start)
}

/// Build one canonical metadata-only terminal.
///
/// # Errors
///
/// Returns when the start is invalid, the terminal shape is inconsistent, time
/// regresses, or canonicalization fails.
pub fn build_execution_journal_terminal_v1(
    start: &ExecutionJournalStartV1,
    draft: ExecutionJournalTerminalDraftV1,
) -> Result<ExecutionJournalTerminalV1, ExecutionJournalError> {
    validate_start(start)?;
    validate_terminal_shape(draft.status, draft.consumption_digest, draft.outcome_digest)?;
    if draft.finished_at < start.started_at {
        return Err(ExecutionJournalError::InvalidRecord);
    }
    let mut terminal = ExecutionJournalTerminalV1 {
        schema_version: "hyperintel.execution-journal-terminal.v1".to_owned(),
        state_slice: EXECUTION_JOURNAL_STATE_SLICE_V1.to_owned(),
        start_digest: start.start_digest,
        admission_digest: start.admission_digest,
        consumption_digest: draft.consumption_digest,
        outcome_digest: draft.outcome_digest,
        status: draft.status,
        finished_at: draft.finished_at,
        authority_granted: false,
        terminal_digest: Sha256Digest::from_bytes([0; 32]),
    };
    terminal.terminal_digest = terminal_digest(&terminal)?;
    Ok(terminal)
}

/// Journal one caller-invoked local execution attempt.
///
/// # Errors
///
/// Returns after a fail-closed journal, replay, or execution error. Local
/// execution errors are retained as closed terminal metadata before return.
pub async fn execute_journaled_local_chat_completion_v1<P, S>(
    journal: &SqliteExecutionJournalV1,
    context: LocalExecutionContextV1<'_>,
    provider: &mut P,
    signer: &S,
) -> Result<JournaledLocalExecutionV1, ExecutionJournalError>
where
    P: LocalProviderAgentV1,
    S: ProviderReceiptSignerV1,
{
    let start = build_execution_journal_start_v1(ExecutionJournalStartDraftV1 {
        admission_digest: context.admission.admission_digest,
        buyer_id: context.admission.buyer_id.clone(),
        provider_id: context.admission.provider_id.clone(),
        prompt_digest: context.admission.prompt_digest,
        started_at: context.consumed_at.clone(),
    })?;
    prepare_execution_journal_start(journal, context.replay_store, &start).await?;
    let replay_store = context.replay_store;
    let finished_fallback = context.consumed_at.clone();
    match execute_local_chat_completion_v1(context, provider, signer).await {
        Ok(execution) => {
            let (status, consumption, outcome, finished_at) = outcome_metadata(&execution);
            let terminal = build_execution_journal_terminal_v1(
                &start,
                ExecutionJournalTerminalDraftV1 {
                    consumption_digest: Some(consumption.consumption_digest),
                    outcome_digest: Some(outcome),
                    status,
                    finished_at,
                },
            )?;
            journal.finish(&terminal, replay_store).await?;
            Ok(JournaledLocalExecutionV1 {
                start,
                terminal,
                execution,
            })
        }
        Err(error) => {
            let consumption = replay_store.consumption(start.admission_digest).await?;
            if matches!(error, LocalExecutionError::AdmissionAlreadyConsumed)
                && consumption.is_some()
            {
                return Err(ExecutionJournalError::ConsumedWithoutTerminal);
            }
            let status = if consumption.is_some() {
                ExecutionJournalTerminalStatusV1::FailedAfterConsumption
            } else {
                ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption
            };
            let terminal = build_execution_journal_terminal_v1(
                &start,
                ExecutionJournalTerminalDraftV1 {
                    consumption_digest: consumption.as_ref().map(|value| value.consumption_digest),
                    outcome_digest: None,
                    status,
                    finished_at: finished_fallback,
                },
            )?;
            journal.finish(&terminal, replay_store).await?;
            Err(ExecutionJournalError::Execution(error))
        }
    }
}

async fn prepare_execution_journal_start(
    journal: &SqliteExecutionJournalV1,
    replay_store: &SqliteReplayStoreV1,
    start: &ExecutionJournalStartV1,
) -> Result<(), ExecutionJournalError> {
    let start_write = journal.begin(start).await?;
    match journal
        .recovery_state(start.admission_digest, replay_store)
        .await?
    {
        ExecutionRecoveryStateV1::StartedNotConsumed(_) => {
            if start_write == JournalWriteOutcomeV1::ExistingIdentical {
                return Err(ExecutionJournalError::AlreadyStarted);
            }
        }
        ExecutionRecoveryStateV1::Terminal { .. } => {
            return Err(ExecutionJournalError::AlreadyTerminal);
        }
        ExecutionRecoveryStateV1::ConsumedWithoutTerminal { .. }
        | ExecutionRecoveryStateV1::ConsumedWithoutStart(_) => {
            return Err(ExecutionJournalError::ConsumedWithoutTerminal);
        }
        ExecutionRecoveryStateV1::NotStarted => {
            return Err(ExecutionJournalError::StoredBindingMismatch);
        }
    }
    Ok(())
}

fn outcome_metadata(
    execution: &LocalExecutionOutcomeV1,
) -> (
    ExecutionJournalTerminalStatusV1,
    &AuthenticatedReplayConsumptionV1,
    Sha256Digest,
    UtcTimestamp,
) {
    match execution {
        LocalExecutionOutcomeV1::Completed(value) => (
            ExecutionJournalTerminalStatusV1::Completed,
            &value.replay_consumption,
            value.completion_digest,
            value.signed_receipt.receipt.completed_at.clone(),
        ),
        LocalExecutionOutcomeV1::Incomplete(value) => (
            ExecutionJournalTerminalStatusV1::Incomplete,
            &value.replay_consumption,
            value.attempt_digest,
            value.finished_at.clone(),
        ),
    }
}

fn validate_optional_consumption(
    admission: Sha256Digest,
    consumption: Option<&AuthenticatedReplayConsumptionV1>,
) -> Result<(), ExecutionJournalError> {
    if let Some(value) = consumption
        && (value.schema_version != "hyperintel.authenticated-replay-consumption.v1"
            || value.state_slice != DURABLE_REPLAY_STATE_SLICE_V1
            || value.admission_digest != admission
            || value.admission_digest.is_zero()
            || value.transition_digest.is_zero()
            || value.buyer_id != value.before_state.buyer_id
            || value.buyer_id != value.after_state.buyer_id
            || value.consumed_nonce != value.before_state.next_nonce
            || value.consumed_nonce.checked_add(1) != Some(value.after_state.next_nonce)
            || validate_buyer_replay_state_v1(&value.before_state).is_err()
            || validate_buyer_replay_state_v1(&value.after_state).is_err()
            || value.authority_granted
            || value.consumption_digest.is_zero()
            || value.digest()? != value.consumption_digest)
    {
        return Err(ExecutionJournalError::InvalidRecord);
    }
    Ok(())
}

fn validate_start(start: &ExecutionJournalStartV1) -> Result<(), ExecutionJournalError> {
    if start.schema_version != "hyperintel.execution-journal-start.v1"
        || start.state_slice != EXECUTION_JOURNAL_STATE_SLICE_V1
        || start.authority_granted
        || start.admission_digest.is_zero()
        || start.prompt_digest.is_zero()
        || start.start_digest.is_zero()
        || start.digest()? != start.start_digest
        || start.attempt_id != format!("attempt_{}", &start.admission_digest.to_hex()[..32])
    {
        return Err(ExecutionJournalError::InvalidRecord);
    }
    Ok(())
}

fn validate_terminal(terminal: &ExecutionJournalTerminalV1) -> Result<(), ExecutionJournalError> {
    if terminal.schema_version != "hyperintel.execution-journal-terminal.v1"
        || terminal.state_slice != EXECUTION_JOURNAL_STATE_SLICE_V1
        || terminal.authority_granted
        || terminal.start_digest.is_zero()
        || terminal.admission_digest.is_zero()
        || terminal.terminal_digest.is_zero()
        || terminal.digest()? != terminal.terminal_digest
    {
        return Err(ExecutionJournalError::InvalidRecord);
    }
    validate_terminal_shape(
        terminal.status,
        terminal.consumption_digest,
        terminal.outcome_digest,
    )
}

fn validate_terminal_shape(
    status: ExecutionJournalTerminalStatusV1,
    consumption: Option<Sha256Digest>,
    outcome: Option<Sha256Digest>,
) -> Result<(), ExecutionJournalError> {
    let valid = match status {
        ExecutionJournalTerminalStatusV1::Completed
        | ExecutionJournalTerminalStatusV1::Incomplete => {
            consumption.is_some() && outcome.is_some()
        }
        ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption => {
            consumption.is_none() && outcome.is_none()
        }
        ExecutionJournalTerminalStatusV1::FailedAfterConsumption => {
            consumption.is_some() && outcome.is_none()
        }
    };
    if valid
        && consumption.is_none_or(|value| !value.is_zero())
        && outcome.is_none_or(|value| !value.is_zero())
    {
        Ok(())
    } else {
        Err(ExecutionJournalError::InvalidRecord)
    }
}

fn decode_start(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<ExecutionJournalStartV1, ExecutionJournalError> {
    let payload: String = row.try_get("payload_json")?;
    let start: ExecutionJournalStartV1 = serde_json::from_str(&payload)?;
    validate_start(&start)?;
    let matches = start.admission_digest.to_hex()
        == row.try_get::<String, _>("admission_digest")?
        && start.start_digest.to_hex() == row.try_get::<String, _>("start_digest")?
        && start.buyer_id.as_str() == row.try_get::<String, _>("buyer_id")?
        && start.provider_id.as_str() == row.try_get::<String, _>("provider_id")?
        && start.started_at.as_str() == row.try_get::<String, _>("started_at")?;
    matches
        .then_some(start)
        .ok_or(ExecutionJournalError::StoredBindingMismatch)
}

fn decode_terminal(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<ExecutionJournalTerminalV1, ExecutionJournalError> {
    let payload: String = row.try_get("payload_json")?;
    let terminal: ExecutionJournalTerminalV1 = serde_json::from_str(&payload)?;
    validate_terminal(&terminal)?;
    let matches = terminal.admission_digest.to_hex()
        == row.try_get::<String, _>("admission_digest")?
        && terminal.terminal_digest.to_hex() == row.try_get::<String, _>("terminal_digest")?
        && status_name(terminal.status) == row.try_get::<String, _>("status")?
        && terminal.finished_at.as_str() == row.try_get::<String, _>("finished_at")?;
    matches
        .then_some(terminal)
        .ok_or(ExecutionJournalError::StoredBindingMismatch)
}

fn status_name(status: ExecutionJournalTerminalStatusV1) -> &'static str {
    match status {
        ExecutionJournalTerminalStatusV1::Completed => "COMPLETED",
        ExecutionJournalTerminalStatusV1::Incomplete => "INCOMPLETE",
        ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption => {
            "REJECTED_BEFORE_CONSUMPTION"
        }
        ExecutionJournalTerminalStatusV1::FailedAfterConsumption => "FAILED_AFTER_CONSUMPTION",
    }
}

fn start_digest(start: &ExecutionJournalStartV1) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.execution-journal-start.v1",
        &(
            &start.schema_version,
            &start.state_slice,
            &start.attempt_id,
            start.admission_digest,
            &start.buyer_id,
            &start.provider_id,
            start.prompt_digest,
            &start.started_at,
            start.authority_granted,
        ),
    )
}

fn terminal_digest(
    terminal: &ExecutionJournalTerminalV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.execution-journal-terminal.v1",
        &(
            &terminal.schema_version,
            &terminal.state_slice,
            terminal.start_digest,
            terminal.admission_digest,
            terminal.consumption_digest,
            terminal.outcome_digest,
            terminal.status,
            &terminal.finished_at,
            terminal.authority_granted,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperintel_market::build_buyer_replay_state_v1;
    use tempfile::tempdir;

    #[tokio::test]
    async fn contract_preexisting_consumption_and_existing_start_never_implicitly_execute()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let replay_path = directory.path().join("replay.sqlite");
        let journal_path = directory.path().join("journal.sqlite");
        let replay_store = SqliteReplayStoreV1::connect_path(&replay_path, 2).await?;
        replay_store.migrate().await?;
        let journal = SqliteExecutionJournalV1::connect_path(&journal_path, 2).await?;
        journal.migrate().await?;

        let orphan_admission = Sha256Digest::from_bytes([91; 32]);
        let orphan_consumption = replay_consumption(orphan_admission, 92)?;
        seed_replay_consumption(&replay_path, &orphan_consumption).await?;
        assert_eq!(
            replay_store.consumption(orphan_admission).await?,
            Some(orphan_consumption.clone())
        );
        let orphan_start = journal_start(orphan_admission, "2026-07-21T12:00:00Z")?;
        assert!(matches!(
            prepare_execution_journal_start(&journal, &replay_store, &orphan_start).await,
            Err(ExecutionJournalError::ConsumedWithoutTerminal)
        ));
        assert_eq!(journal.terminal(orphan_admission).await?, None);
        assert!(matches!(
            journal
                .recovery_state(orphan_admission, &replay_store)
                .await?,
            ExecutionRecoveryStateV1::ConsumedWithoutTerminal { .. }
        ));

        let fresh_start =
            journal_start(Sha256Digest::from_bytes([93; 32]), "2026-07-21T12:10:00Z")?;
        prepare_execution_journal_start(&journal, &replay_store, &fresh_start).await?;
        assert!(matches!(
            prepare_execution_journal_start(&journal, &replay_store, &fresh_start).await,
            Err(ExecutionJournalError::AlreadyStarted)
        ));
        assert_eq!(journal.terminal(fresh_start.admission_digest).await?, None);

        journal.checkpoint_and_close().await?;
        replay_store.checkpoint_and_close().await?;
        Ok(())
    }

    fn journal_start(
        admission_digest: Sha256Digest,
        started_at: &str,
    ) -> Result<ExecutionJournalStartV1, Box<dyn std::error::Error>> {
        Ok(build_execution_journal_start_v1(
            ExecutionJournalStartDraftV1 {
                admission_digest,
                buyer_id: "buyer:journal".parse()?,
                provider_id: "provider:journal".parse()?,
                prompt_digest: Sha256Digest::from_bytes([94; 32]),
                started_at: started_at.parse()?,
            },
        )?)
    }

    fn replay_consumption(
        admission_digest: Sha256Digest,
        marker: u8,
    ) -> Result<AuthenticatedReplayConsumptionV1, Box<dyn std::error::Error>> {
        let buyer_id: ProtocolId = "buyer:journal".parse()?;
        let before_state = build_buyer_replay_state_v1(buyer_id.clone(), 11)?;
        let after_state = build_buyer_replay_state_v1(buyer_id.clone(), 12)?;
        let mut value = AuthenticatedReplayConsumptionV1 {
            schema_version: "hyperintel.authenticated-replay-consumption.v1".to_owned(),
            state_slice: DURABLE_REPLAY_STATE_SLICE_V1.to_owned(),
            admission_digest,
            transition_digest: Sha256Digest::from_bytes([marker; 32]),
            buyer_id,
            consumed_nonce: 11,
            before_state,
            after_state,
            consumed_at: "2026-07-21T12:00:01Z".parse()?,
            authority_granted: false,
            consumption_digest: Sha256Digest::from_bytes([0; 32]),
        };
        value.consumption_digest = value.digest()?;
        Ok(value)
    }

    async fn seed_replay_consumption(
        path: &Path,
        consumption: &AuthenticatedReplayConsumptionV1,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::query(
            "INSERT INTO buyer_replay_states(\
             buyer_id, state_digest, next_nonce, payload_json, version, updated_at) \
             VALUES(?, ?, ?, ?, 0, ?)",
        )
        .bind(consumption.buyer_id.as_str())
        .bind(consumption.after_state.state_digest.to_hex())
        .bind(consumption.after_state.next_nonce.to_string())
        .bind(serde_json::to_string(&consumption.after_state)?)
        .bind(consumption.consumed_at.as_str())
        .execute(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO authenticated_replay_consumptions(\
             admission_digest, transition_digest, buyer_id, consumed_nonce, \
             before_state_digest, after_state_digest, payload_json, consumed_at) \
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(consumption.admission_digest.to_hex())
        .bind(consumption.transition_digest.to_hex())
        .bind(consumption.buyer_id.as_str())
        .bind(consumption.consumed_nonce.to_string())
        .bind(consumption.before_state.state_digest.to_hex())
        .bind(consumption.after_state.state_digest.to_hex())
        .bind(serde_json::to_string(consumption)?)
        .bind(consumption.consumed_at.as_str())
        .execute(&pool)
        .await?;
        pool.close().await;
        Ok(())
    }
}
