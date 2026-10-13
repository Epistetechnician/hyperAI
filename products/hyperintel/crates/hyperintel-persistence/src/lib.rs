//! Local `SQLite` persistence for authenticated buyer replay consumption.
//!
//! This crate owns one narrow external effect: an operator-selected local
//! `SQLite` file. Successful replay consumption does not release a request,
//! contact a provider, submit settlement, or grant external authority.

use std::{path::Path, str::FromStr, time::Duration};

use hyperintel_assurance::VerificationKeyRegistry;
use hyperintel_core::{
    CanonicalizationError, ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerReplayStateV1,
    ExecutionEvidenceError, ReplayError, validate_buyer_replay_state_v1,
    verify_authenticated_execution_admission_v1,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

/// State slice persisted by this crate.
pub const DURABLE_REPLAY_STATE_SLICE_V1: &str = "hyperintel-durable-replay-store-v1";

const SQLITE_SCHEMA_V1: &str = r"
CREATE TABLE buyer_replay_states (
  buyer_id TEXT PRIMARY KEY,
  state_digest TEXT NOT NULL,
  next_nonce TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  version INTEGER NOT NULL CHECK(version >= 0),
  updated_at TEXT NOT NULL
);

CREATE TABLE authenticated_replay_consumptions (
  admission_digest TEXT PRIMARY KEY,
  transition_digest TEXT NOT NULL UNIQUE,
  buyer_id TEXT NOT NULL REFERENCES buyer_replay_states(buyer_id) ON DELETE RESTRICT,
  consumed_nonce TEXT NOT NULL,
  before_state_digest TEXT NOT NULL,
  after_state_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  consumed_at TEXT NOT NULL
);
";

/// Result of registering the initial durable state for one buyer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitializeReplayStateOutcomeV1 {
    /// The exact state was inserted.
    Inserted,
    /// The exact state already existed and was not rewritten.
    ExistingIdentical,
}

/// Result of atomically consuming one authenticated replay transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsumeReplayOutcomeV1 {
    /// The transition advanced durable state and its immutable record was inserted.
    Applied(AuthenticatedReplayConsumptionV1),
    /// The exact admission and transition were already consumed.
    ExistingIdentical(AuthenticatedReplayConsumptionV1),
}

/// Immutable record of one durable authenticated replay consumption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedReplayConsumptionV1 {
    /// Fixed record schema.
    pub schema_version: String,
    /// Named persistence state slice.
    pub state_slice: String,
    /// Exact authenticated admission accepted before the compare-and-swap.
    pub admission_digest: Sha256Digest,
    /// Exact pure transition accepted before the compare-and-swap.
    pub transition_digest: Sha256Digest,
    /// Buyer whose durable replay state advanced.
    pub buyer_id: ProtocolId,
    /// Exact nonce consumed.
    pub consumed_nonce: u64,
    /// Durable state required before the transaction.
    pub before_state: BuyerReplayStateV1,
    /// Durable state committed by the transaction.
    pub after_state: BuyerReplayStateV1,
    /// Explicit operator-supplied consumption time.
    pub consumed_at: UtcTimestamp,
    /// Replay consumption alone grants no prompt-release or execution authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub consumption_digest: Sha256Digest,
}

#[derive(Serialize)]
struct ReplayConsumptionIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    admission_digest: Sha256Digest,
    transition_digest: Sha256Digest,
    buyer_id: &'a ProtocolId,
    consumed_nonce: u64,
    before_state: &'a BuyerReplayStateV1,
    after_state: &'a BuyerReplayStateV1,
    consumed_at: &'a UtcTimestamp,
    authority_granted: bool,
}

impl AuthenticatedReplayConsumptionV1 {
    /// Recompute the durable consumption identity.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn digest(&self) -> Result<Sha256Digest, CanonicalizationError> {
        replay_consumption_digest(self)
    }
}

/// Fail-closed local replay persistence failures.
#[derive(Debug, Error)]
pub enum ReplayPersistenceError {
    /// SQL operation or transaction failed.
    #[error("local replay database operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// Stored JSON is invalid.
    #[error("stored replay JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// Pure replay state or transition validation failed.
    #[error("replay validation failed: {0}")]
    Replay(#[from] ReplayError),
    /// Authenticated execution admission validation failed.
    #[error("authenticated admission validation failed: {0}")]
    Execution(#[from] ExecutionEvidenceError),
    /// Canonical identity construction failed.
    #[error("replay persistence canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// Checked-in migration content differs from the recorded version.
    #[error("local replay migration version {version} has a different checksum")]
    MigrationChecksumConflict {
        /// Conflicting schema version.
        version: i64,
    },
    /// Buyer state was not initialized before consumption.
    #[error("buyer replay state is not initialized")]
    StateNotInitialized,
    /// Initial buyer identity already points at different state.
    #[error("buyer replay state already exists with different semantics")]
    InitialStateConflict,
    /// Durable state no longer equals the admission's before-state.
    #[error("authenticated replay transition is stale or conflicts with durable state")]
    StaleState,
    /// Admission or transition identity was reused for different semantics.
    #[error("authenticated replay consumption identity conflict")]
    ConsumptionConflict,
    /// Stored columns disagree with their canonical payload.
    #[error("stored replay columns do not match the canonical payload")]
    StoredBindingMismatch,
    /// Consumption time is outside the authenticated admission window.
    #[error("replay consumption time is outside the authenticated admission window")]
    InvalidConsumptionTime,
    /// A file-backed connection pool must contain at least one connection.
    #[error("local replay database connection count must be positive")]
    InvalidConnectionCount,
    /// Stored consumption falsely claims external authority.
    #[error("durable replay consumption cannot grant external authority")]
    AuthorityGranted,
    /// Stored consumption digest is zero or inconsistent.
    #[error("durable replay consumption digest mismatch")]
    ConsumptionDigestMismatch,
}

/// Local `SQLite` implementation of authenticated replay compare-and-swap.
#[derive(Clone, Debug)]
pub struct SqliteReplayStoreV1 {
    pool: SqlitePool,
}

impl SqliteReplayStoreV1 {
    /// Connect to an in-memory test store with one shared connection.
    ///
    /// # Errors
    ///
    /// Returns when `SQLite` cannot initialize the pool.
    pub async fn in_memory() -> Result<Self, ReplayPersistenceError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .foreign_keys(true)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Connect to one operator-selected local `SQLite` path.
    ///
    /// # Errors
    ///
    /// Returns when the local file cannot be opened or initialized.
    pub async fn connect_path(
        path: &Path,
        max_connections: u32,
    ) -> Result<Self, ReplayPersistenceError> {
        if max_connections == 0 {
            return Err(ReplayPersistenceError::InvalidConnectionCount);
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

    /// Apply and checksum the fixed local replay schema.
    ///
    /// # Errors
    ///
    /// Returns on database failure or migration checksum conflict.
    pub async fn migrate(&self) -> Result<(), ReplayPersistenceError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS hyperintel_schema_migrations (\
             version INTEGER PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL)",
        )
        .execute(&self.pool)
        .await?;

        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let checksum = migration_checksum(1, SQLITE_SCHEMA_V1)?.to_hex();
        let existing =
            sqlx::query("SELECT checksum FROM hyperintel_schema_migrations WHERE version = 1")
                .fetch_optional(&mut *transaction)
                .await?;
        if let Some(row) = existing {
            let stored: String = row.try_get("checksum")?;
            if stored != checksum {
                return Err(ReplayPersistenceError::MigrationChecksumConflict { version: 1 });
            }
            transaction.commit().await?;
            return Ok(());
        }
        sqlx::raw_sql(SQLITE_SCHEMA_V1)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "INSERT INTO hyperintel_schema_migrations(version, checksum, applied_at) \
             VALUES(1, ?, '2026-07-21T00:00:00Z')",
        )
        .bind(checksum)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Register the exact initial state for one buyer without overwriting it.
    ///
    /// # Errors
    ///
    /// Returns on invalid state, conflicting initialization, or database failure.
    pub async fn initialize_buyer_state(
        &self,
        state: &BuyerReplayStateV1,
        initialized_at: &UtcTimestamp,
    ) -> Result<InitializeReplayStateOutcomeV1, ReplayPersistenceError> {
        validate_buyer_replay_state_v1(state)?;
        let payload_json = serde_json::to_string(state)?;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let existing = sqlx::query(
            "SELECT buyer_id, state_digest, next_nonce, payload_json FROM buyer_replay_states \
             WHERE buyer_id = ?",
        )
        .bind(state.buyer_id.as_str())
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(row) = existing {
            let decoded = decode_state_row(&row)?;
            transaction.commit().await?;
            return if decoded == *state {
                Ok(InitializeReplayStateOutcomeV1::ExistingIdentical)
            } else {
                Err(ReplayPersistenceError::InitialStateConflict)
            };
        }
        sqlx::query(
            "INSERT INTO buyer_replay_states(\
             buyer_id, state_digest, next_nonce, payload_json, version, updated_at) \
             VALUES(?, ?, ?, ?, 0, ?)",
        )
        .bind(state.buyer_id.as_str())
        .bind(state.state_digest.to_hex())
        .bind(state.next_nonce.to_string())
        .bind(payload_json)
        .bind(initialized_at.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(InitializeReplayStateOutcomeV1::Inserted)
    }

    /// Load and validate one buyer's current durable replay state.
    ///
    /// # Errors
    ///
    /// Returns on database, decoding, identity, or stored-column mismatch.
    pub async fn current_state(
        &self,
        buyer_id: &ProtocolId,
    ) -> Result<Option<BuyerReplayStateV1>, ReplayPersistenceError> {
        let row = sqlx::query(
            "SELECT buyer_id, state_digest, next_nonce, payload_json FROM buyer_replay_states \
             WHERE buyer_id = ?",
        )
        .bind(buyer_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|value| decode_state_row(&value)).transpose()
    }

    /// Verify and atomically consume one authenticated admission replay transition.
    ///
    /// An immediate transaction serializes writers before checking an immutable
    /// prior consumption or applying the exact state-digest compare-and-swap.
    /// Returning success grants no prompt-release or provider-execution authority.
    ///
    /// # Errors
    ///
    /// Returns on authentication, routing, replay, time, stale state, identity,
    /// stored-data, or database failure.
    pub async fn consume_authenticated_admission(
        &self,
        admission: &AuthenticatedExecutionAdmissionV1,
        context: &AuthenticatedExecutionContextV1<'_>,
        registry: &VerificationKeyRegistry,
        consumed_at: &UtcTimestamp,
    ) -> Result<ConsumeReplayOutcomeV1, ReplayPersistenceError> {
        verify_authenticated_execution_admission_v1(admission, context, registry)?;
        if *consumed_at < admission.admitted_at || *consumed_at > admission.deadline_at {
            return Err(ReplayPersistenceError::InvalidConsumptionTime);
        }
        let consumption = build_consumption(admission, context, consumed_at)?;
        let payload_json = serde_json::to_string(&consumption)?;
        let transition = context.replay_transition;

        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let existing = sqlx::query(
            "SELECT admission_digest, transition_digest, buyer_id, consumed_nonce, \
             before_state_digest, after_state_digest, payload_json, consumed_at \
             FROM authenticated_replay_consumptions \
             WHERE admission_digest = ? OR transition_digest = ?",
        )
        .bind(admission.admission_digest.to_hex())
        .bind(transition.transition_digest.to_hex())
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(row) = existing {
            let decoded = decode_consumption_row(&row)?;
            transaction.commit().await?;
            return if decoded == consumption {
                Ok(ConsumeReplayOutcomeV1::ExistingIdentical(decoded))
            } else {
                Err(ReplayPersistenceError::ConsumptionConflict)
            };
        }

        let updated = sqlx::query(
            "UPDATE buyer_replay_states SET state_digest = ?, next_nonce = ?, \
             payload_json = ?, version = version + 1, updated_at = ? \
             WHERE buyer_id = ? AND state_digest = ? AND next_nonce = ? AND payload_json = ?",
        )
        .bind(transition.after_state.state_digest.to_hex())
        .bind(transition.after_state.next_nonce.to_string())
        .bind(serde_json::to_string(&transition.after_state)?)
        .bind(consumed_at.as_str())
        .bind(transition.buyer_id.as_str())
        .bind(transition.before_state.state_digest.to_hex())
        .bind(transition.before_state.next_nonce.to_string())
        .bind(serde_json::to_string(&transition.before_state)?)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if updated != 1 {
            let initialized: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM buyer_replay_states WHERE buyer_id = ?")
                    .bind(transition.buyer_id.as_str())
                    .fetch_optional(&mut *transaction)
                    .await?;
            return if initialized.is_some() {
                Err(ReplayPersistenceError::StaleState)
            } else {
                Err(ReplayPersistenceError::StateNotInitialized)
            };
        }

        sqlx::query(
            "INSERT INTO authenticated_replay_consumptions(\
             admission_digest, transition_digest, buyer_id, consumed_nonce, \
             before_state_digest, after_state_digest, payload_json, consumed_at) \
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(admission.admission_digest.to_hex())
        .bind(transition.transition_digest.to_hex())
        .bind(transition.buyer_id.as_str())
        .bind(transition.consumed_nonce.to_string())
        .bind(transition.before_state.state_digest.to_hex())
        .bind(transition.after_state.state_digest.to_hex())
        .bind(payload_json)
        .bind(consumed_at.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(ConsumeReplayOutcomeV1::Applied(consumption))
    }

    /// Load and verify a durable consumption by admission identity.
    ///
    /// # Errors
    ///
    /// Returns on database, decoding, identity, or stored-column mismatch.
    pub async fn consumption(
        &self,
        admission_digest: Sha256Digest,
    ) -> Result<Option<AuthenticatedReplayConsumptionV1>, ReplayPersistenceError> {
        let row = sqlx::query(
            "SELECT admission_digest, transition_digest, buyer_id, consumed_nonce, \
             before_state_digest, after_state_digest, payload_json, consumed_at \
             FROM authenticated_replay_consumptions WHERE admission_digest = ?",
        )
        .bind(admission_digest.to_hex())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|value| decode_consumption_row(&value)).transpose()
    }

    /// Count immutable authenticated replay consumptions.
    ///
    /// # Errors
    ///
    /// Returns on database failure or an invalid negative count.
    pub async fn consumption_count(&self) -> Result<u64, ReplayPersistenceError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM authenticated_replay_consumptions")
                .fetch_one(&self.pool)
                .await?;
        u64::try_from(count).map_err(|_| ReplayPersistenceError::StoredBindingMismatch)
    }

    /// Checkpoint the WAL and close all local database connections.
    ///
    /// # Errors
    ///
    /// Returns when the local WAL checkpoint fails.
    pub async fn checkpoint_and_close(self) -> Result<(), ReplayPersistenceError> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await?;
        self.pool.close().await;
        Ok(())
    }
}

fn build_consumption(
    admission: &AuthenticatedExecutionAdmissionV1,
    context: &AuthenticatedExecutionContextV1<'_>,
    consumed_at: &UtcTimestamp,
) -> Result<AuthenticatedReplayConsumptionV1, ReplayPersistenceError> {
    let transition = context.replay_transition;
    let mut consumption = AuthenticatedReplayConsumptionV1 {
        schema_version: "hyperintel.authenticated-replay-consumption.v1".to_owned(),
        state_slice: DURABLE_REPLAY_STATE_SLICE_V1.to_owned(),
        admission_digest: admission.admission_digest,
        transition_digest: transition.transition_digest,
        buyer_id: transition.buyer_id.clone(),
        consumed_nonce: transition.consumed_nonce,
        before_state: transition.before_state.clone(),
        after_state: transition.after_state.clone(),
        consumed_at: consumed_at.clone(),
        authority_granted: false,
        consumption_digest: Sha256Digest::from_bytes([0; 32]),
    };
    consumption.consumption_digest = replay_consumption_digest(&consumption)?;
    if consumption.consumption_digest.is_zero() {
        return Err(ReplayPersistenceError::ConsumptionDigestMismatch);
    }
    Ok(consumption)
}

fn validate_consumption(
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<(), ReplayPersistenceError> {
    if consumption.schema_version != "hyperintel.authenticated-replay-consumption.v1"
        || consumption.state_slice != DURABLE_REPLAY_STATE_SLICE_V1
    {
        return Err(ReplayPersistenceError::StoredBindingMismatch);
    }
    if consumption.authority_granted {
        return Err(ReplayPersistenceError::AuthorityGranted);
    }
    validate_buyer_replay_state_v1(&consumption.before_state)?;
    validate_buyer_replay_state_v1(&consumption.after_state)?;
    if consumption.admission_digest.is_zero()
        || consumption.transition_digest.is_zero()
        || consumption.buyer_id != consumption.before_state.buyer_id
        || consumption.buyer_id != consumption.after_state.buyer_id
        || consumption.consumed_nonce != consumption.before_state.next_nonce
        || consumption.after_state.next_nonce
            != consumption
                .consumed_nonce
                .checked_add(1)
                .ok_or(ReplayPersistenceError::StoredBindingMismatch)?
    {
        return Err(ReplayPersistenceError::StoredBindingMismatch);
    }
    if consumption.consumption_digest.is_zero()
        || replay_consumption_digest(consumption)? != consumption.consumption_digest
    {
        return Err(ReplayPersistenceError::ConsumptionDigestMismatch);
    }
    Ok(())
}

fn decode_state_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<BuyerReplayStateV1, ReplayPersistenceError> {
    let payload_json: String = row.try_get("payload_json")?;
    let state: BuyerReplayStateV1 = serde_json::from_str(&payload_json)?;
    validate_buyer_replay_state_v1(&state)?;
    let buyer_id: String = row.try_get("buyer_id")?;
    let state_digest: String = row.try_get("state_digest")?;
    let next_nonce: String = row.try_get("next_nonce")?;
    if state.buyer_id.as_str() != buyer_id
        || state.state_digest.to_hex() != state_digest
        || state.next_nonce.to_string() != next_nonce
    {
        return Err(ReplayPersistenceError::StoredBindingMismatch);
    }
    Ok(state)
}

fn decode_consumption_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<AuthenticatedReplayConsumptionV1, ReplayPersistenceError> {
    let payload_json: String = row.try_get("payload_json")?;
    let consumption: AuthenticatedReplayConsumptionV1 = serde_json::from_str(&payload_json)?;
    validate_consumption(&consumption)?;
    let admission_digest: String = row.try_get("admission_digest")?;
    let transition_digest: String = row.try_get("transition_digest")?;
    let buyer_id: String = row.try_get("buyer_id")?;
    let consumed_nonce: String = row.try_get("consumed_nonce")?;
    let before_state_digest: String = row.try_get("before_state_digest")?;
    let after_state_digest: String = row.try_get("after_state_digest")?;
    let consumed_at: String = row.try_get("consumed_at")?;
    if consumption.admission_digest.to_hex() != admission_digest
        || consumption.transition_digest.to_hex() != transition_digest
        || consumption.buyer_id.as_str() != buyer_id
        || consumption.consumed_nonce.to_string() != consumed_nonce
        || consumption.before_state.state_digest.to_hex() != before_state_digest
        || consumption.after_state.state_digest.to_hex() != after_state_digest
        || consumption.consumed_at.as_str() != consumed_at
    {
        return Err(ReplayPersistenceError::StoredBindingMismatch);
    }
    Ok(consumption)
}

fn migration_checksum(version: i64, sql: &str) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest("hyperintel.sqlite-replay-migration.v1", &(version, sql))
}

fn replay_consumption_digest(
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.authenticated-replay-consumption.v1",
        &ReplayConsumptionIdentity {
            schema_version: &consumption.schema_version,
            state_slice: &consumption.state_slice,
            admission_digest: consumption.admission_digest,
            transition_digest: consumption.transition_digest,
            buyer_id: &consumption.buyer_id,
            consumed_nonce: consumption.consumed_nonce,
            before_state: &consumption.before_state,
            after_state: &consumption.after_state,
            consumed_at: &consumption.consumed_at,
            authority_granted: consumption.authority_granted,
        },
    )
}
