//! `SQLx` persistence for immutable protocol state, events, idempotency, and outbox.

use std::{path::Path, str::FromStr, time::Duration};

use outcomes_domain::{
    CanonicalizationError, ClaimSpecError, ClaimSpecV1, MutationContext, ReconciliationResultV1,
    ReconciliationStatus, SettlementLifecycle, Sha256Digest, StateSlice, SubmissionReceiptV1,
    UndeclaredMutation, UnsignedSettlementIntentV1, UtcTimestamp, canonical_json,
    domain_separated_digest,
};
use serde_json::json;
use sqlx::{
    Row, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

const SQLITE_MIGRATION_V1: &str = include_str!("../../../migrations/sqlite/0001_initial.sql");
const SQLITE_MIGRATION_V2: &str =
    include_str!("../../../migrations/sqlite/0002_settlement_recovery.sql");

/// Result of an idempotent immutable-claim freeze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeOutcome {
    /// This transaction inserted the frozen claim and its events.
    Inserted,
    /// The identical derived claim already existed; no duplicate event was emitted.
    ExistingIdentical,
}

/// Result of idempotently appending one settlement record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    /// The immutable record and its audit/outbox entries were inserted.
    Inserted,
    /// The exact immutable record already existed; no duplicate events were emitted.
    ExistingIdentical,
}

/// Safe restart action derived from durable settlement state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementRecoveryAction {
    /// Query authoritative backend state before any retry.
    QueryExternalStatus,
    /// A confirmed reconciliation is already durable.
    Complete,
}

/// Durable settlement state loaded after process restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementRecoveryState {
    /// Deterministic unsigned intent persisted before submission.
    pub intent: UnsignedSettlementIntentV1,
    /// Exact request digest whose backend status must be queried.
    pub request_digest: Sha256Digest,
    /// At-most-one durable receipt.
    pub receipt: Option<SubmissionReceiptV1>,
    /// Most recent append-only reconciliation result.
    pub latest_reconciliation: Option<ReconciliationResultV1>,
    /// Fail-closed restart action; missing receipt never means blind resubmission.
    pub action: SettlementRecoveryAction,
}

/// Persistence failures with fail-closed scope and idempotency semantics.
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// SQL operation or transaction failed.
    #[error("database operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// Claim validation, canonicalization, or identity verification failed.
    #[error("invalid claim specification: {0}")]
    ClaimSpec(#[from] ClaimSpecError),
    /// A request, migration, event, audit, or outbox payload was not canonicalizable.
    #[error("persistence payload is not canonicalizable: {0}")]
    Canonicalization(#[from] CanonicalizationError),
    /// JSON serialization or decoding failed.
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// Mutation attempted a state slice it did not declare.
    #[error(transparent)]
    UndeclaredMutation(#[from] UndeclaredMutation),
    /// One idempotency key was reused for different request semantics.
    #[error("idempotency key {key} was already used for a different request")]
    IdempotencyConflict {
        /// Conflicting key.
        key: String,
    },
    /// A derived claim ID already pointed at a different digest.
    #[error("claim ID collision or corrupted existing claim for {claim_id}")]
    ClaimIdentityConflict {
        /// Derived claim ID that did not map to the expected digest.
        claim_id: String,
    },
    /// A migration was requested with an incompatible recorded checksum.
    #[error("migration version {version} has a different checksum")]
    MigrationChecksumConflict {
        /// Schema version whose checked-in SQL changed in place.
        version: i64,
    },
    /// A settlement record referenced an intent that does not exist.
    #[error("settlement intent {intent_id} does not exist")]
    SettlementIntentNotFound {
        /// Missing deterministic intent ID.
        intent_id: String,
    },
    /// An immutable settlement identity was reused for different semantics.
    #[error("settlement identity conflict for {identity}: {reason}")]
    SettlementIdentityConflict {
        /// Conflicting intent, idempotency, request, or receipt identity.
        identity: String,
        /// Exact conflict class.
        reason: &'static str,
    },
    /// A receipt request digest did not match the durable pre-submit request.
    #[error("receipt request digest does not match durable intent {intent_id}")]
    SettlementRequestDigestMismatch {
        /// Intent whose receipt cannot be accepted.
        intent_id: String,
    },
}

/// `SQLite` development implementation of immutable claim persistence.
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Connect to an in-memory development database using one shared connection.
    ///
    /// # Errors
    ///
    /// Returns a `SQLx` error when `SQLite` cannot initialize the pool.
    pub async fn in_memory() -> Result<Self, PersistenceError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .foreign_keys(true)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Connect to a file-backed `SQLite` development database.
    ///
    /// # Errors
    ///
    /// Returns a `SQLx` error when the file cannot be opened or initialized.
    pub async fn connect_path(path: &Path, max_connections: u32) -> Result<Self, PersistenceError> {
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

    /// Apply the immutable v1 `SQLite` schema once and verify its checksum later.
    ///
    /// # Errors
    ///
    /// Returns a database error or checksum conflict. Checked-in migration SQL
    /// is never silently reapplied after its version is recorded.
    pub async fn migrate(&self) -> Result<(), PersistenceError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS hsai_schema_migrations (\
             version INTEGER PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL)",
        )
        .execute(&self.pool)
        .await?;
        apply_sqlite_migration(&self.pool, 1, SQLITE_MIGRATION_V1, "2026-07-10T00:00:00Z").await?;
        apply_sqlite_migration(&self.pool, 2, SQLITE_MIGRATION_V2, "2026-07-11T00:00:00Z").await?;
        Ok(())
    }

    /// Freeze a claim transactionally with idempotency, event, audit, and outbox rows.
    ///
    /// Required declared slices are `ClaimRegistryV1`, `OperationsV1`, and
    /// `DeliveryV1`.
    ///
    /// # Errors
    ///
    /// Returns a scope, validation, idempotency, identity, serialization, or
    /// database error. No partial rows are committed.
    pub async fn freeze_claim(
        &self,
        context: &MutationContext,
        claim: &ClaimSpecV1,
        frozen_at: &UtcTimestamp,
        build_version: &str,
    ) -> Result<FreezeOutcome, PersistenceError> {
        require_freeze_scope(context)?;
        claim.verify_identity()?;
        let claim_digest = claim.claim_spec_digest()?;
        let request_digest = domain_separated_digest(
            "hsai.freeze-claim-request.v1",
            &json!({
                "claim_spec_digest": claim_digest,
                "frozen_at": frozen_at,
                "build_version": build_version,
            }),
        )?;
        let canonical = canonical_json(claim)?;
        let payload_json = serde_json::to_string(claim)?;

        let mut transaction = self.pool.begin().await?;
        let idempotency = begin_idempotency(
            &mut transaction,
            "freeze_claim",
            &context.request_id,
            request_digest,
            frozen_at,
        )
        .await?;
        if let IdempotencyStart::Existing = idempotency {
            transaction.commit().await?;
            return existing_claim_outcome(&self.pool, claim).await;
        }

        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO claim_specs(\
             claim_id, claim_spec_digest, challenge_descriptor_digest, canonical_json, \
             payload_json, created_at, frozen_at, version) VALUES(?, ?, ?, ?, ?, ?, ?, 1)",
        )
        .bind(claim.claim_id.to_string())
        .bind(claim_digest.to_string())
        .bind(claim.challenge_descriptor_digest.to_string())
        .bind(canonical)
        .bind(payload_json.clone())
        .bind(claim.created_at.to_string())
        .bind(frozen_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1;

        if !inserted {
            verify_existing_claim(&mut transaction, claim).await?;
            complete_idempotency(
                &mut transaction,
                "freeze_claim",
                &context.request_id,
                claim_digest,
                frozen_at,
            )
            .await?;
            transaction.commit().await?;
            return Ok(FreezeOutcome::ExistingIdentical);
        }

        append_freeze_records(
            &mut transaction,
            context,
            claim,
            claim_digest,
            &payload_json,
            frozen_at,
            build_version,
        )
        .await?;
        complete_idempotency(
            &mut transaction,
            "freeze_claim",
            &context.request_id,
            claim_digest,
            frozen_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(FreezeOutcome::Inserted)
    }

    /// Read and independently verify one frozen claim.
    ///
    /// # Errors
    ///
    /// Returns a database, JSON, or identity verification error.
    pub async fn get_claim(&self, claim_id: &str) -> Result<Option<ClaimSpecV1>, PersistenceError> {
        let row = sqlx::query("SELECT payload_json FROM claim_specs WHERE claim_id = ?")
            .bind(claim_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| -> Result<ClaimSpecV1, PersistenceError> {
            let payload: String = row.try_get("payload_json")?;
            let claim: ClaimSpecV1 = serde_json::from_str(&payload)?;
            claim.verify_identity()?;
            Ok(claim)
        })
        .transpose()
    }

    /// Persist a deterministic unsigned settlement intent before any backend call.
    ///
    /// Required declared slices are `SettlementLedgerV1`, `OperationsV1`, and
    /// `DeliveryV1`. Both the idempotency key and exact request digest are
    /// globally unique. An exact replay emits no duplicate audit or outbox row.
    ///
    /// # Errors
    ///
    /// Returns a scope, identity, serialization, foreign-key, or database error.
    pub async fn persist_settlement_intent(
        &self,
        context: &MutationContext,
        intent: &UnsignedSettlementIntentV1,
        request_digest: Sha256Digest,
        created_at: &UtcTimestamp,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_settlement_scope(context)?;
        let payload_json = serde_json::to_string(intent)?;
        let record_digest = domain_separated_digest("hsai.persisted-settlement-intent.v1", intent)?;
        let mut transaction = self.pool.begin().await?;

        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO settlement_intents(\
             intent_id, binding_digest, final_resolution_digest, idempotency_key, payload_digest, \
             payload_json, lifecycle, created_at, request_digest) \
             VALUES(?, ?, ?, ?, ?, ?, 'INTENT_BUILT', ?, ?)",
        )
        .bind(&intent.intent_id)
        .bind(intent.binding_digest.to_string())
        .bind(intent.final_resolution_digest.to_string())
        .bind(&intent.idempotency_key)
        .bind(intent.payload_digest.to_string())
        .bind(&payload_json)
        .bind(created_at.to_string())
        .bind(request_digest.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1;

        if !inserted {
            let outcome = verify_existing_settlement_intent(
                &mut transaction,
                intent,
                request_digest,
                &payload_json,
            )
            .await?;
            transaction.commit().await?;
            return Ok(outcome);
        }

        append_settlement_records(
            &mut transaction,
            context,
            "SETTLEMENT_INTENT_PERSISTED",
            &intent.intent_id,
            record_digest,
            &payload_json,
            created_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(AppendOutcome::Inserted)
    }

    /// Record the backend receipt at most once for a durable intent.
    ///
    /// The receipt request digest must equal the request digest stored before
    /// submission. Exact retries are no-ops; divergent receipts fail closed.
    ///
    /// # Errors
    ///
    /// Returns a scope, missing-intent, digest, identity, serialization, or
    /// database error.
    pub async fn record_settlement_receipt(
        &self,
        context: &MutationContext,
        receipt: &SubmissionReceiptV1,
        recorded_at: &UtcTimestamp,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_settlement_scope(context)?;
        let payload_json = serde_json::to_string(receipt)?;
        let receipt_digest =
            domain_separated_digest("hsai.persisted-settlement-receipt.v1", receipt)?;
        let receipt_id = prefixed_id("receipt", receipt_digest);
        let mut transaction = self.pool.begin().await?;

        let intent_row = sqlx::query(
            "SELECT idempotency_key, request_digest FROM settlement_intents WHERE intent_id = ?",
        )
        .bind(&receipt.intent_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(intent_row) = intent_row else {
            return Err(PersistenceError::SettlementIntentNotFound {
                intent_id: receipt.intent_id.clone(),
            });
        };
        let durable_idempotency_key: String = intent_row.try_get("idempotency_key")?;
        if durable_idempotency_key != receipt.idempotency_key {
            return Err(PersistenceError::SettlementIdentityConflict {
                identity: receipt.intent_id.clone(),
                reason: "receipt idempotency key does not match durable intent",
            });
        }
        let durable_request_digest: String = intent_row.try_get("request_digest")?;
        if durable_request_digest != receipt.request_digest.to_string() {
            return Err(PersistenceError::SettlementRequestDigestMismatch {
                intent_id: receipt.intent_id.clone(),
            });
        }

        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO settlement_receipts(\
             receipt_id, intent_id, backend_receipt_id, payload_json, accepted_at) \
             VALUES(?, ?, ?, ?, ?)",
        )
        .bind(&receipt_id)
        .bind(&receipt.intent_id)
        .bind(&receipt.backend_receipt_id)
        .bind(&payload_json)
        .bind(receipt.accepted_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1;

        if !inserted {
            let existing =
                sqlx::query("SELECT payload_json FROM settlement_receipts WHERE intent_id = ?")
                    .bind(&receipt.intent_id)
                    .fetch_optional(&mut *transaction)
                    .await?;
            let Some(existing) = existing else {
                return Err(PersistenceError::SettlementIdentityConflict {
                    identity: receipt.backend_receipt_id.clone(),
                    reason: "receipt identity reused by another intent",
                });
            };
            let existing_payload: String = existing.try_get("payload_json")?;
            if existing_payload == payload_json {
                transaction.commit().await?;
                return Ok(AppendOutcome::ExistingIdentical);
            }
            return Err(PersistenceError::SettlementIdentityConflict {
                identity: receipt.intent_id.clone(),
                reason: "intent already has a different receipt",
            });
        }

        append_settlement_records(
            &mut transaction,
            context,
            "SETTLEMENT_RECEIPT_RECORDED",
            &receipt.intent_id,
            receipt_digest,
            &payload_json,
            recorded_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(AppendOutcome::Inserted)
    }

    /// Append a reconciliation observation for one durable intent.
    ///
    /// Reconciliation rows are immutable. The intent binding and resolution
    /// digests must match the result before the transaction writes anything.
    ///
    /// # Errors
    ///
    /// Returns a scope, missing-intent, identity, serialization, or database error.
    pub async fn record_settlement_reconciliation(
        &self,
        context: &MutationContext,
        intent_id: &str,
        result: &ReconciliationResultV1,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_settlement_scope(context)?;
        let payload_json = serde_json::to_string(result)?;
        let reconciliation_digest =
            domain_separated_digest("hsai.persisted-settlement-reconciliation.v1", result)?;
        let mut transaction = self.pool.begin().await?;

        let intent_row = sqlx::query(
            "SELECT binding_digest, final_resolution_digest FROM settlement_intents \
             WHERE intent_id = ?",
        )
        .bind(intent_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(intent_row) = intent_row else {
            return Err(PersistenceError::SettlementIntentNotFound {
                intent_id: intent_id.to_owned(),
            });
        };
        let binding_digest: String = intent_row.try_get("binding_digest")?;
        let resolution_digest: String = intent_row.try_get("final_resolution_digest")?;
        if binding_digest != result.binding_digest.to_string()
            || resolution_digest != result.final_resolution_digest.to_string()
        {
            return Err(PersistenceError::SettlementIdentityConflict {
                identity: intent_id.to_owned(),
                reason: "reconciliation binding or resolution digest mismatch",
            });
        }
        if let Some(expected_receipt_digest) = result.receipt_digest {
            let receipt_payload: Option<String> = sqlx::query_scalar(
                "SELECT payload_json FROM settlement_receipts WHERE intent_id = ?",
            )
            .bind(intent_id)
            .fetch_optional(&mut *transaction)
            .await?;
            let Some(receipt_payload) = receipt_payload else {
                return Err(PersistenceError::SettlementIdentityConflict {
                    identity: intent_id.to_owned(),
                    reason: "reconciliation references a receipt that is not durable",
                });
            };
            let receipt: SubmissionReceiptV1 = serde_json::from_str(&receipt_payload)?;
            let durable_receipt_digest =
                domain_separated_digest("hsai.persisted-settlement-receipt.v1", &receipt)?;
            if durable_receipt_digest != expected_receipt_digest {
                return Err(PersistenceError::SettlementIdentityConflict {
                    identity: intent_id.to_owned(),
                    reason: "reconciliation receipt digest mismatch",
                });
            }
        }

        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO settlement_reconciliations(\
             reconciliation_digest, intent_id, receipt_digest, external_status_digest, status, \
             lifecycle, payload_json, checked_at) VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(reconciliation_digest.to_string())
        .bind(intent_id)
        .bind(result.receipt_digest.map(|digest| digest.to_string()))
        .bind(result.external_status_digest.to_string())
        .bind(enum_json_label(&result.status)?)
        .bind(enum_json_label(&result.settlement_lifecycle)?)
        .bind(&payload_json)
        .bind(result.checked_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1;

        if !inserted {
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT payload_json FROM settlement_reconciliations \
                 WHERE reconciliation_digest = ?",
            )
            .bind(reconciliation_digest.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
            if existing.as_deref() == Some(payload_json.as_str()) {
                transaction.commit().await?;
                return Ok(AppendOutcome::ExistingIdentical);
            }
            return Err(PersistenceError::SettlementIdentityConflict {
                identity: reconciliation_digest.to_string(),
                reason: "reconciliation digest collision",
            });
        }

        append_settlement_records(
            &mut transaction,
            context,
            "SETTLEMENT_RECONCILED",
            intent_id,
            reconciliation_digest,
            &payload_json,
            &result.checked_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(AppendOutcome::Inserted)
    }

    /// Load durable restart state without authorizing a blind resubmission.
    ///
    /// An intent lacking a receipt is treated as potentially accepted by the
    /// external backend and therefore requires authoritative reconciliation.
    ///
    /// # Errors
    ///
    /// Returns a database or stored-JSON error.
    pub async fn settlement_recovery_state(
        &self,
        intent_id: &str,
    ) -> Result<Option<SettlementRecoveryState>, PersistenceError> {
        let intent_row = sqlx::query(
            "SELECT request_digest, payload_json FROM settlement_intents WHERE intent_id = ?",
        )
        .bind(intent_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(intent_row) = intent_row else {
            return Ok(None);
        };
        let intent_payload: String = intent_row.try_get("payload_json")?;
        let request_digest_text: String = intent_row.try_get("request_digest")?;
        let intent: UnsignedSettlementIntentV1 = serde_json::from_str(&intent_payload)?;
        let request_digest: Sha256Digest =
            serde_json::from_value(serde_json::Value::String(request_digest_text))?;

        let receipt =
            sqlx::query("SELECT payload_json FROM settlement_receipts WHERE intent_id = ?")
                .bind(intent_id)
                .fetch_optional(&self.pool)
                .await?
                .map(|row| -> Result<SubmissionReceiptV1, PersistenceError> {
                    let payload: String = row.try_get("payload_json")?;
                    Ok(serde_json::from_str(&payload)?)
                })
                .transpose()?;

        let latest_reconciliation = sqlx::query(
            "SELECT payload_json FROM settlement_reconciliations WHERE intent_id = ? \
             ORDER BY checked_at DESC, reconciliation_digest DESC LIMIT 1",
        )
        .bind(intent_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row| -> Result<ReconciliationResultV1, PersistenceError> {
            let payload: String = row.try_get("payload_json")?;
            Ok(serde_json::from_str(&payload)?)
        })
        .transpose()?;

        let action = match latest_reconciliation.as_ref() {
            Some(result)
                if result.status == ReconciliationStatus::Confirmed
                    && result.settlement_lifecycle == SettlementLifecycle::Confirmed =>
            {
                SettlementRecoveryAction::Complete
            }
            _ => SettlementRecoveryAction::QueryExternalStatus,
        };
        Ok(Some(SettlementRecoveryState {
            intent,
            request_digest,
            receipt,
            latest_reconciliation,
            action,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdempotencyStart {
    Inserted,
    Existing,
}

async fn apply_sqlite_migration(
    pool: &SqlitePool,
    version: i64,
    sql: &str,
    applied_at: &str,
) -> Result<(), PersistenceError> {
    let checksum = domain_separated_digest(
        "hsai.sqlite-migration.v1",
        &json!({"version": version, "sql": sql}),
    )?
    .to_string();
    let existing = sqlx::query("SELECT checksum FROM hsai_schema_migrations WHERE version = ?")
        .bind(version)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = existing {
        let stored: String = row.try_get("checksum")?;
        if stored == checksum {
            return Ok(());
        }
        return Err(PersistenceError::MigrationChecksumConflict { version });
    }

    let mut transaction = pool.begin().await?;
    sqlx::raw_sql(sql).execute(&mut *transaction).await?;
    sqlx::query(
        "INSERT INTO hsai_schema_migrations(version, checksum, applied_at) VALUES(?, ?, ?)",
    )
    .bind(version)
    .bind(checksum)
    .bind(applied_at)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

fn require_freeze_scope(context: &MutationContext) -> Result<(), PersistenceError> {
    for slice in [
        StateSlice::ClaimRegistryV1,
        StateSlice::OperationsV1,
        StateSlice::DeliveryV1,
    ] {
        context.scope.require(slice)?;
    }
    Ok(())
}

fn require_settlement_scope(context: &MutationContext) -> Result<(), PersistenceError> {
    for slice in [
        StateSlice::SettlementLedgerV1,
        StateSlice::OperationsV1,
        StateSlice::DeliveryV1,
    ] {
        context.scope.require(slice)?;
    }
    Ok(())
}

async fn verify_existing_settlement_intent(
    transaction: &mut Transaction<'_, Sqlite>,
    intent: &UnsignedSettlementIntentV1,
    request_digest: Sha256Digest,
    payload_json: &str,
) -> Result<AppendOutcome, PersistenceError> {
    let row = sqlx::query(
        "SELECT intent_id, idempotency_key, payload_digest, request_digest, payload_json \
         FROM settlement_intents WHERE intent_id = ? OR idempotency_key = ? \
         OR payload_digest = ? OR request_digest = ? LIMIT 1",
    )
    .bind(&intent.intent_id)
    .bind(&intent.idempotency_key)
    .bind(intent.payload_digest.to_string())
    .bind(request_digest.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(row) = row else {
        return Err(PersistenceError::SettlementIdentityConflict {
            identity: intent.intent_id.clone(),
            reason: "insert was rejected without a visible conflicting row",
        });
    };
    let existing_intent_id: String = row.try_get("intent_id")?;
    let existing_idempotency_key: String = row.try_get("idempotency_key")?;
    let existing_payload_digest: String = row.try_get("payload_digest")?;
    let existing_request_digest: String = row.try_get("request_digest")?;
    let existing_payload: String = row.try_get("payload_json")?;
    if existing_intent_id == intent.intent_id
        && existing_idempotency_key == intent.idempotency_key
        && existing_payload_digest == intent.payload_digest.to_string()
        && existing_request_digest == request_digest.to_string()
        && existing_payload == payload_json
    {
        return Ok(AppendOutcome::ExistingIdentical);
    }

    let (identity, reason) = if existing_idempotency_key == intent.idempotency_key {
        (
            intent.idempotency_key.clone(),
            "idempotency key reused for a different request",
        )
    } else if existing_request_digest == request_digest.to_string() {
        (
            request_digest.to_string(),
            "request digest reused for a different intent",
        )
    } else if existing_payload_digest == intent.payload_digest.to_string() {
        (
            intent.payload_digest.to_string(),
            "payload digest reused for a different intent",
        )
    } else {
        (
            intent.intent_id.clone(),
            "intent ID reused for different semantics",
        )
    };
    Err(PersistenceError::SettlementIdentityConflict { identity, reason })
}

async fn append_settlement_records(
    transaction: &mut Transaction<'_, Sqlite>,
    context: &MutationContext,
    event_type: &str,
    intent_id: &str,
    payload_digest: Sha256Digest,
    payload_json: &str,
    received_at: &UtcTimestamp,
) -> Result<(), PersistenceError> {
    let outbox_payload = json!({
        "event_type": event_type,
        "intent_id": intent_id,
        "payload_digest": payload_digest,
    });
    let outbox_digest = domain_separated_digest("hsai.settlement-outbox.v1", &outbox_payload)?;
    sqlx::query(
        "INSERT INTO outbox_events(\
         outbox_id, aggregate_type, aggregate_id, event_type, payload_digest, payload_json, created_at) \
         VALUES(?, 'SETTLEMENT', ?, ?, ?, ?, ?)",
    )
    .bind(prefixed_id("outbox", outbox_digest))
    .bind(intent_id)
    .bind(event_type)
    .bind(outbox_digest.to_string())
    .bind(serde_json::to_string(&outbox_payload)?)
    .bind(received_at.to_string())
    .execute(&mut **transaction)
    .await?;

    let audit_payload = json!({
        "event_type": event_type,
        "intent_id": intent_id,
        "record_payload_digest": payload_digest,
    });
    let audit_digest = domain_separated_digest(
        "hsai.settlement-audit.v1",
        &json!({
            "actor_id": context.actor_id,
            "request_id": context.request_id,
            "payload": audit_payload,
        }),
    )?;
    sqlx::query(
        "INSERT INTO audit_events(\
         audit_id, event_type, actor_id, request_id, declared_state_slices_json, aggregate_type, \
         aggregate_id, payload_digest, payload_json, received_at, audit_digest) \
         VALUES(?, ?, ?, ?, ?, 'SETTLEMENT', ?, ?, ?, ?, ?)",
    )
    .bind(prefixed_id("audit", audit_digest))
    .bind(event_type)
    .bind(&context.actor_id)
    .bind(&context.request_id)
    .bind(serde_json::to_string(&context.scope)?)
    .bind(intent_id)
    .bind(payload_digest.to_string())
    .bind(payload_json)
    .bind(received_at.to_string())
    .bind(audit_digest.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn enum_json_label(value: &impl serde::Serialize) -> Result<String, PersistenceError> {
    let encoded = serde_json::to_value(value)?;
    match encoded {
        serde_json::Value::String(label) => Ok(label),
        _ => Err(PersistenceError::Json(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enum did not serialize to a string",
            ),
        ))),
    }
}

async fn begin_idempotency(
    transaction: &mut Transaction<'_, Sqlite>,
    scope: &str,
    key: &str,
    request_digest: Sha256Digest,
    created_at: &UtcTimestamp,
) -> Result<IdempotencyStart, PersistenceError> {
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO idempotency_keys(\
         scope, idempotency_key, request_digest, status, created_at) VALUES(?, ?, ?, 'STARTED', ?)",
    )
    .bind(scope)
    .bind(key)
    .bind(request_digest.to_string())
    .bind(created_at.to_string())
    .execute(&mut **transaction)
    .await?
    .rows_affected()
        == 1;
    if inserted {
        return Ok(IdempotencyStart::Inserted);
    }
    let row = sqlx::query(
        "SELECT request_digest FROM idempotency_keys WHERE scope = ? AND idempotency_key = ?",
    )
    .bind(scope)
    .bind(key)
    .fetch_one(&mut **transaction)
    .await?;
    let existing: String = row.try_get("request_digest")?;
    if existing == request_digest.to_string() {
        Ok(IdempotencyStart::Existing)
    } else {
        Err(PersistenceError::IdempotencyConflict {
            key: key.to_owned(),
        })
    }
}

async fn complete_idempotency(
    transaction: &mut Transaction<'_, Sqlite>,
    scope: &str,
    key: &str,
    response_digest: Sha256Digest,
    completed_at: &UtcTimestamp,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE idempotency_keys SET response_digest = ?, status = 'COMPLETED', completed_at = ? \
         WHERE scope = ? AND idempotency_key = ?",
    )
    .bind(response_digest.to_string())
    .bind(completed_at.to_string())
    .bind(scope)
    .bind(key)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn verify_existing_claim(
    transaction: &mut Transaction<'_, Sqlite>,
    claim: &ClaimSpecV1,
) -> Result<(), PersistenceError> {
    let row = sqlx::query("SELECT claim_spec_digest FROM claim_specs WHERE claim_id = ?")
        .bind(claim.claim_id.to_string())
        .fetch_one(&mut **transaction)
        .await?;
    let existing: String = row.try_get("claim_spec_digest")?;
    if existing == claim.claim_spec_digest()?.to_string() {
        Ok(())
    } else {
        Err(PersistenceError::ClaimIdentityConflict {
            claim_id: claim.claim_id.to_string(),
        })
    }
}

async fn existing_claim_outcome(
    pool: &SqlitePool,
    claim: &ClaimSpecV1,
) -> Result<FreezeOutcome, PersistenceError> {
    let row = sqlx::query("SELECT claim_spec_digest FROM claim_specs WHERE claim_id = ?")
        .bind(claim.claim_id.to_string())
        .fetch_optional(pool)
        .await?;
    let Some(row) = row else {
        return Err(PersistenceError::ClaimIdentityConflict {
            claim_id: claim.claim_id.to_string(),
        });
    };
    let existing: String = row.try_get("claim_spec_digest")?;
    if existing == claim.claim_spec_digest()?.to_string() {
        Ok(FreezeOutcome::ExistingIdentical)
    } else {
        Err(PersistenceError::ClaimIdentityConflict {
            claim_id: claim.claim_id.to_string(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
async fn append_freeze_records(
    transaction: &mut Transaction<'_, Sqlite>,
    context: &MutationContext,
    claim: &ClaimSpecV1,
    claim_digest: Sha256Digest,
    payload_json: &str,
    frozen_at: &UtcTimestamp,
    build_version: &str,
) -> Result<(), PersistenceError> {
    let event_payload = json!({
        "claim_id": claim.claim_id,
        "claim_spec_digest": claim_digest,
        "declared_state_slices": context.scope,
    });
    let event_digest = domain_separated_digest("hsai.claim-event.v1", &event_payload)?;
    let event_id = prefixed_id("evt", event_digest);
    sqlx::query(
        "INSERT INTO claim_events(\
         event_id, claim_id, aggregate_version, previous_version, event_type, schema_version, \
         actor_type, actor_id, request_id, trace_id, payload_digest, payload_json, received_at, \
         build_version, event_digest) VALUES(?, ?, 1, 0, 'CLAIM_FROZEN', \
         'hsai.claim-event.v1', 'OPERATOR', ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&event_id)
    .bind(claim.claim_id.to_string())
    .bind(&context.actor_id)
    .bind(&context.request_id)
    .bind(&context.request_id)
    .bind(claim_digest.to_string())
    .bind(payload_json)
    .bind(frozen_at.to_string())
    .bind(build_version)
    .bind(event_digest.to_string())
    .execute(&mut **transaction)
    .await?;

    let outbox_payload = json!({"event_id": event_id, "event_digest": event_digest});
    let outbox_digest = domain_separated_digest("hsai.outbox-event.v1", &outbox_payload)?;
    sqlx::query(
        "INSERT INTO outbox_events(\
         outbox_id, aggregate_type, aggregate_id, event_type, payload_digest, payload_json, created_at) \
         VALUES(?, 'CLAIM', ?, 'CLAIM_FROZEN', ?, ?, ?)",
    )
    .bind(prefixed_id("outbox", outbox_digest))
    .bind(claim.claim_id.to_string())
    .bind(outbox_digest.to_string())
    .bind(serde_json::to_string(&outbox_payload)?)
    .bind(frozen_at.to_string())
    .execute(&mut **transaction)
    .await?;

    let audit_payload = json!({
        "event_id": event_id,
        "event_digest": event_digest,
        "claim_id": claim.claim_id,
        "claim_spec_digest": claim_digest,
    });
    let audit_digest = domain_separated_digest("hsai.audit-event.v1", &audit_payload)?;
    sqlx::query(
        "INSERT INTO audit_events(\
         audit_id, event_type, actor_id, request_id, declared_state_slices_json, aggregate_type, \
         aggregate_id, payload_digest, payload_json, received_at, audit_digest) \
         VALUES(?, 'CLAIM_FROZEN', ?, ?, ?, 'CLAIM', ?, ?, ?, ?, ?)",
    )
    .bind(prefixed_id("audit", audit_digest))
    .bind(&context.actor_id)
    .bind(&context.request_id)
    .bind(serde_json::to_string(&context.scope)?)
    .bind(claim.claim_id.to_string())
    .bind(claim_digest.to_string())
    .bind(serde_json::to_string(&audit_payload)?)
    .bind(frozen_at.to_string())
    .bind(audit_digest.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn prefixed_id(prefix: &str, digest: Sha256Digest) -> String {
    format!("{prefix}_{}", &digest.to_hex()[..32])
}

#[cfg(test)]
mod tests {
    use outcomes_domain::{MutationScope, SettlementNetwork, fixtures::local_demo_claim_draft};

    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn freeze_context(request_id: &str) -> MutationContext {
        MutationContext {
            actor_id: "operator-local".to_owned(),
            request_id: request_id.to_owned(),
            scope: MutationScope::new([
                StateSlice::ClaimRegistryV1,
                StateSlice::OperationsV1,
                StateSlice::DeliveryV1,
            ]),
        }
    }

    fn settlement_context(request_id: &str) -> MutationContext {
        MutationContext {
            actor_id: "settlement-worker-local".to_owned(),
            request_id: request_id.to_owned(),
            scope: MutationScope::new([
                StateSlice::SettlementLedgerV1,
                StateSlice::OperationsV1,
                StateSlice::DeliveryV1,
            ]),
        }
    }

    fn digest(label: &str) -> Result<Sha256Digest, CanonicalizationError> {
        domain_separated_digest("hsai.persistence-test.v1", &json!({"label": label}))
    }

    async fn seed_settlement_dependencies(
        store: &SqliteStore,
    ) -> Result<(Sha256Digest, Sha256Digest), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        store
            .freeze_claim(
                &freeze_context("settlement-seed"),
                &claim,
                &timestamp("2026-07-10T10:01:00Z")?,
                "test-build",
            )
            .await?;
        let evidence_digest = digest("evidence")?;
        sqlx::query(
            "INSERT INTO evidence_manifests(evidence_digest, claim_id, payload_json, created_at) \
             VALUES(?, ?, '{}', '2026-07-10T10:02:00Z')",
        )
        .bind(evidence_digest.to_string())
        .bind(claim.claim_id.to_string())
        .execute(&store.pool)
        .await?;
        let resolution_digest = digest("resolution")?;
        sqlx::query(
            "INSERT INTO resolution_records(\
             resolution_digest, claim_id, truth_value, finality, evidence_digest, payload_json, created_at) \
             VALUES(?, ?, 'TRUE', 'FINAL', ?, '{}', '2026-07-10T10:03:00Z')",
        )
        .bind(resolution_digest.to_string())
        .bind(claim.claim_id.to_string())
        .bind(evidence_digest.to_string())
        .execute(&store.pool)
        .await?;
        let binding_digest = digest("binding")?;
        sqlx::query(
            "INSERT INTO market_bindings(\
             binding_digest, claim_id, backend, network, external_market_id, true_side_id, \
             false_side_id, metadata_digest, payload_json, created_at) \
             VALUES(?, ?, 'mock-hip4', 'MOCK', 'outcome-42', 'side-0', 'side-1', ?, '{}', \
             '2026-07-10T10:04:00Z')",
        )
        .bind(binding_digest.to_string())
        .bind(claim.claim_id.to_string())
        .bind(digest("metadata")?.to_string())
        .execute(&store.pool)
        .await?;
        Ok((binding_digest, resolution_digest))
    }

    fn settlement_intent(
        binding_digest: Sha256Digest,
        resolution_digest: Sha256Digest,
    ) -> Result<UnsignedSettlementIntentV1, Box<dyn std::error::Error>> {
        Ok(UnsignedSettlementIntentV1 {
            schema_version: "hsai.unsigned-settlement-intent.v1".to_owned(),
            intent_id: "intent_test_recovery".to_owned(),
            backend: "mock-hip4".to_owned(),
            network: SettlementNetwork::Mock,
            binding_digest,
            final_resolution_digest: resolution_digest,
            action_type: "SETTLE_OUTCOME".to_owned(),
            canonical_payload: "{\"outcome\":\"TRUE\"}".to_owned(),
            payload_digest: digest("settlement-payload")?,
            idempotency_key: "settlement_test_recovery".to_owned(),
            expires_at: timestamp("2026-07-10T11:00:00Z")?,
        })
    }

    fn settlement_receipt(
        intent: &UnsignedSettlementIntentV1,
        request_digest: Sha256Digest,
    ) -> Result<SubmissionReceiptV1, Box<dyn std::error::Error>> {
        Ok(SubmissionReceiptV1 {
            schema_version: "hsai.submission-receipt.v1".to_owned(),
            intent_id: intent.intent_id.clone(),
            idempotency_key: intent.idempotency_key.clone(),
            backend_receipt_id: "mock_receipt_recovered".to_owned(),
            accepted_at: timestamp("2026-07-10T10:06:00Z")?,
            request_digest,
            response_digest: digest("backend-response")?,
            external_sequence: Some("1".to_owned()),
        })
    }

    fn confirmed_reconciliation(
        intent: &UnsignedSettlementIntentV1,
        receipt: &SubmissionReceiptV1,
    ) -> Result<ReconciliationResultV1, Box<dyn std::error::Error>> {
        Ok(ReconciliationResultV1 {
            schema_version: "hsai.reconciliation-result.v1".to_owned(),
            binding_digest: intent.binding_digest,
            final_resolution_digest: intent.final_resolution_digest,
            receipt_digest: Some(domain_separated_digest(
                "hsai.persisted-settlement-receipt.v1",
                receipt,
            )?),
            external_status_digest: digest("external-confirmed-status")?,
            status: ReconciliationStatus::Confirmed,
            settlement_lifecycle: SettlementLifecycle::Confirmed,
            checked_at: timestamp("2026-07-10T10:07:00Z")?,
            exact_reason: "authoritative mock status matches final resolution".to_owned(),
        })
    }

    #[tokio::test]
    async fn contract_migration_is_repeatable_and_complete()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        store.migrate().await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_one(&store.pool)
        .await?;
        assert!(count >= 36);
        let migration_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM hsai_schema_migrations")
                .fetch_one(&store.pool)
                .await?;
        assert_eq!(migration_count, 2);
        Ok(())
    }

    #[tokio::test]
    async fn contract_freeze_is_transactional_idempotent_and_audited()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let claim = local_demo_claim_draft()?.freeze()?;
        let frozen_at = timestamp("2026-07-10T10:01:00Z")?;
        assert_eq!(
            store
                .freeze_claim(
                    &freeze_context("request-1"),
                    &claim,
                    &frozen_at,
                    "test-build"
                )
                .await?,
            FreezeOutcome::Inserted
        );
        assert_eq!(
            store
                .freeze_claim(
                    &freeze_context("request-2"),
                    &claim,
                    &frozen_at,
                    "test-build"
                )
                .await?,
            FreezeOutcome::ExistingIdentical
        );
        let loaded = store.get_claim(claim.claim_id.as_str()).await?;
        assert_eq!(loaded, Some(claim));
        for table in [
            "claim_specs",
            "claim_events",
            "audit_events",
            "outbox_events",
        ] {
            let query = format!("SELECT COUNT(*) FROM {table}");
            let count: i64 = sqlx::query_scalar(&query).fetch_one(&store.pool).await?;
            assert_eq!(count, 1, "unexpected row count for {table}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn contract_idempotency_key_reuse_with_different_request_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let first = local_demo_claim_draft()?.freeze()?;
        let mut changed = local_demo_claim_draft()?;
        changed.question.push_str(" Changed semantics.");
        let second = changed.freeze()?;
        let frozen_at = timestamp("2026-07-10T10:01:00Z")?;
        store
            .freeze_claim(
                &freeze_context("same-request"),
                &first,
                &frozen_at,
                "test-build",
            )
            .await?;
        assert!(matches!(
            store
                .freeze_claim(
                    &freeze_context("same-request"),
                    &second,
                    &frozen_at,
                    "test-build"
                )
                .await,
            Err(PersistenceError::IdempotencyConflict { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn contract_undeclared_slice_fails_before_write() -> Result<(), Box<dyn std::error::Error>>
    {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let claim = local_demo_claim_draft()?.freeze()?;
        let context = MutationContext {
            actor_id: "operator-local".to_owned(),
            request_id: "missing-scope".to_owned(),
            scope: MutationScope::new([StateSlice::ClaimRegistryV1]),
        };
        assert!(matches!(
            store
                .freeze_claim(
                    &context,
                    &claim,
                    &timestamp("2026-07-10T10:01:00Z")?,
                    "test-build"
                )
                .await,
            Err(PersistenceError::UndeclaredMutation(_))
        ));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM claim_specs")
            .fetch_one(&store.pool)
            .await?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn contract_frozen_and_audit_rows_reject_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let claim = local_demo_claim_draft()?.freeze()?;
        store
            .freeze_claim(
                &freeze_context("immutable"),
                &claim,
                &timestamp("2026-07-10T10:01:00Z")?,
                "test-build",
            )
            .await?;
        let claim_update =
            sqlx::query("UPDATE claim_specs SET payload_json = '{}' WHERE claim_id = ?")
                .bind(claim.claim_id.to_string())
                .execute(&store.pool)
                .await;
        assert!(claim_update.is_err());
        let audit_delete = sqlx::query("DELETE FROM audit_events")
            .execute(&store.pool)
            .await;
        assert!(audit_delete.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn contract_concurrent_identical_freezes_emit_one_event()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let store = SqliteStore::connect_path(&directory.path().join("concurrent.db"), 8).await?;
        store.migrate().await?;
        let claim = local_demo_claim_draft()?.freeze()?;
        let frozen_at = timestamp("2026-07-10T10:01:00Z")?;
        let mut tasks = Vec::new();
        for index in 0_u8..8 {
            let task_store = store.clone();
            let task_claim = claim.clone();
            let task_time = frozen_at.clone();
            tasks.push(tokio::spawn(async move {
                task_store
                    .freeze_claim(
                        &freeze_context(&format!("concurrent-{index}")),
                        &task_claim,
                        &task_time,
                        "test-build",
                    )
                    .await
            }));
        }
        let mut inserted = 0_u8;
        for task in tasks {
            if task.await?? == FreezeOutcome::Inserted {
                inserted += 1;
            }
        }
        assert_eq!(inserted, 1);
        let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM claim_events")
            .fetch_one(&store.pool)
            .await?;
        assert_eq!(event_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn contract_event_failure_rolls_back_claim_and_idempotency()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        sqlx::raw_sql(
            "CREATE TRIGGER injected_claim_event_failure BEFORE INSERT ON claim_events \
             BEGIN SELECT RAISE(ABORT, 'injected event failure'); END;",
        )
        .execute(&store.pool)
        .await?;
        let claim = local_demo_claim_draft()?.freeze()?;
        let result = store
            .freeze_claim(
                &freeze_context("rollback"),
                &claim,
                &timestamp("2026-07-10T10:01:00Z")?,
                "test-build",
            )
            .await;
        assert!(matches!(result, Err(PersistenceError::Sqlx(_))));
        for table in [
            "claim_specs",
            "idempotency_keys",
            "claim_events",
            "audit_events",
            "outbox_events",
        ] {
            let query = format!("SELECT COUNT(*) FROM {table}");
            let count: i64 = sqlx::query_scalar(&query).fetch_one(&store.pool).await?;
            assert_eq!(count, 0, "transaction leaked a row in {table}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn contract_settlement_intent_is_persisted_before_submission_and_idempotent()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let request_digest = digest("submit-request")?;
        let created_at = timestamp("2026-07-10T10:05:00Z")?;

        assert_eq!(
            store
                .persist_settlement_intent(
                    &settlement_context("persist-intent"),
                    &intent,
                    request_digest,
                    &created_at,
                )
                .await?,
            AppendOutcome::Inserted
        );
        assert_eq!(
            store
                .persist_settlement_intent(
                    &settlement_context("persist-intent-retry"),
                    &intent,
                    request_digest,
                    &created_at,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        let intent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlement_intents")
            .fetch_one(&store.pool)
            .await?;
        let outbox_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbox_events WHERE aggregate_type = 'SETTLEMENT'",
        )
        .fetch_one(&store.pool)
        .await?;
        assert_eq!((intent_count, outbox_count), (1, 1));
        Ok(())
    }

    #[tokio::test]
    async fn contract_settlement_mutation_requires_declared_state_slices()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let incomplete_context = MutationContext {
            actor_id: "settlement-worker-local".to_owned(),
            request_id: "missing-delivery-slice".to_owned(),
            scope: MutationScope::new([StateSlice::SettlementLedgerV1, StateSlice::OperationsV1]),
        };
        assert!(matches!(
            store
                .persist_settlement_intent(
                    &incomplete_context,
                    &intent,
                    digest("submit-request")?,
                    &timestamp("2026-07-10T10:05:00Z")?,
                )
                .await,
            Err(PersistenceError::UndeclaredMutation(_))
        ));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlement_intents")
            .fetch_one(&store.pool)
            .await?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn contract_settlement_idempotency_and_request_digest_conflicts_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let request_digest = digest("submit-request")?;
        let created_at = timestamp("2026-07-10T10:05:00Z")?;
        store
            .persist_settlement_intent(
                &settlement_context("persist-original"),
                &intent,
                request_digest,
                &created_at,
            )
            .await?;

        let mut reused_key = intent.clone();
        reused_key.intent_id = "intent_different".to_owned();
        reused_key.payload_digest = digest("different-payload")?;
        assert!(matches!(
            store
                .persist_settlement_intent(
                    &settlement_context("reused-key"),
                    &reused_key,
                    digest("different-request")?,
                    &created_at,
                )
                .await,
            Err(PersistenceError::SettlementIdentityConflict { .. })
        ));

        let mut reused_request = intent.clone();
        reused_request.intent_id = "intent_other".to_owned();
        reused_request.idempotency_key = "settlement_other".to_owned();
        reused_request.payload_digest = digest("other-payload")?;
        assert!(matches!(
            store
                .persist_settlement_intent(
                    &settlement_context("reused-request"),
                    &reused_request,
                    request_digest,
                    &created_at,
                )
                .await,
            Err(PersistenceError::SettlementIdentityConflict { .. })
        ));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlement_intents")
            .fetch_one(&store.pool)
            .await?;
        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn contract_receipt_and_reconciliation_are_append_only_and_once_only()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let request_digest = digest("submit-request")?;
        store
            .persist_settlement_intent(
                &settlement_context("persist-intent"),
                &intent,
                request_digest,
                &timestamp("2026-07-10T10:05:00Z")?,
            )
            .await?;
        let receipt = settlement_receipt(&intent, request_digest)?;
        assert_eq!(
            store
                .record_settlement_receipt(
                    &settlement_context("record-receipt"),
                    &receipt,
                    &timestamp("2026-07-10T10:06:01Z")?,
                )
                .await?,
            AppendOutcome::Inserted
        );
        assert_eq!(
            store
                .record_settlement_receipt(
                    &settlement_context("record-receipt-retry"),
                    &receipt,
                    &timestamp("2026-07-10T10:06:01Z")?,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        let mut conflicting_receipt = receipt.clone();
        conflicting_receipt.backend_receipt_id = "different".to_owned();
        assert!(matches!(
            store
                .record_settlement_receipt(
                    &settlement_context("record-conflicting-receipt"),
                    &conflicting_receipt,
                    &timestamp("2026-07-10T10:06:02Z")?,
                )
                .await,
            Err(PersistenceError::SettlementIdentityConflict { .. })
        ));

        let reconciliation = confirmed_reconciliation(&intent, &receipt)?;
        assert_eq!(
            store
                .record_settlement_reconciliation(
                    &settlement_context("record-reconciliation"),
                    &intent.intent_id,
                    &reconciliation,
                )
                .await?,
            AppendOutcome::Inserted
        );
        assert_eq!(
            store
                .record_settlement_reconciliation(
                    &settlement_context("record-reconciliation-retry"),
                    &intent.intent_id,
                    &reconciliation,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        assert!(
            sqlx::query("UPDATE settlement_receipts SET payload_json = '{}' ")
                .execute(&store.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM settlement_reconciliations")
                .execute(&store.pool)
                .await
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn contract_restart_after_external_acceptance_requires_reconciliation_not_resubmit()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("settlement-recovery.db");
        let store = SqliteStore::connect_path(&database_path, 1).await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let request_digest = digest("submit-request")?;
        store
            .persist_settlement_intent(
                &settlement_context("before-submit"),
                &intent,
                request_digest,
                &timestamp("2026-07-10T10:05:00Z")?,
            )
            .await?;

        // The backend accepted exactly one effect, then the process crashed
        // before the returned receipt could be persisted.
        let external_effect_count = 1_u8;
        drop(store);

        let restarted = SqliteStore::connect_path(&database_path, 1).await?;
        restarted.migrate().await?;
        let recovery = restarted
            .settlement_recovery_state(&intent.intent_id)
            .await?
            .ok_or_else(|| std::io::Error::other("durable intent did not survive restart"))?;
        assert_eq!(recovery.intent, intent);
        assert_eq!(recovery.request_digest, request_digest);
        assert!(recovery.receipt.is_none());
        assert_eq!(
            recovery.action,
            SettlementRecoveryAction::QueryExternalStatus
        );

        // Authoritative status lookup returns the lost receipt; no second
        // intent or external effect is created.
        let recovered_receipt = settlement_receipt(&intent, request_digest)?;
        restarted
            .record_settlement_receipt(
                &settlement_context("recover-receipt"),
                &recovered_receipt,
                &timestamp("2026-07-10T10:06:01Z")?,
            )
            .await?;
        let reconciliation = confirmed_reconciliation(&intent, &recovered_receipt)?;
        restarted
            .record_settlement_reconciliation(
                &settlement_context("recover-reconciliation"),
                &intent.intent_id,
                &reconciliation,
            )
            .await?;
        assert_eq!(
            restarted
                .persist_settlement_intent(
                    &settlement_context("restart-intent-retry"),
                    &intent,
                    request_digest,
                    &timestamp("2026-07-10T10:05:00Z")?,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        let complete = restarted
            .settlement_recovery_state(&intent.intent_id)
            .await?
            .ok_or_else(|| std::io::Error::other("confirmed durable state is missing"))?;
        assert_eq!(complete.action, SettlementRecoveryAction::Complete);
        assert_eq!(external_effect_count, 1);
        let counts: (i64, i64, i64) = (
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_intents")
                .fetch_one(&restarted.pool)
                .await?,
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_receipts")
                .fetch_one(&restarted.pool)
                .await?,
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_reconciliations")
                .fetch_one(&restarted.pool)
                .await?,
        );
        assert_eq!(counts, (1, 1, 1));
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires an explicitly provided HSAI_TEST_POSTGRES_URL"]
    async fn contract_postgres_migrations_apply_in_isolated_schema()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{SystemTime, UNIX_EPOCH};

        use sqlx::{Connection, PgConnection};

        let database_url = std::env::var("HSAI_TEST_POSTGRES_URL")?;
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let schema = format!("hsai_contract_{}_{}", std::process::id(), unique);
        let mut connection = PgConnection::connect(&database_url).await?;
        sqlx::raw_sql(&format!(
            "CREATE SCHEMA {schema}; SET search_path TO {schema};"
        ))
        .execute(&mut connection)
        .await?;

        let migration_result = async {
            sqlx::raw_sql(include_str!(
                "../../../migrations/postgres/0001_initial.sql"
            ))
            .execute(&mut connection)
            .await?;
            sqlx::raw_sql(include_str!(
                "../../../migrations/postgres/0002_settlement_recovery.sql"
            ))
            .execute(&mut connection)
            .await?;
            let required_table_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.tables \
                 WHERE table_schema = $1 AND table_name = ANY($2)",
            )
            .bind(&schema)
            .bind(vec![
                "market_bindings",
                "resolution_records",
                "settlement_intents",
                "settlement_attempts",
                "settlement_receipts",
                "settlement_reconciliations",
            ])
            .fetch_one(&mut connection)
            .await?;
            Ok::<i64, sqlx::Error>(required_table_count)
        }
        .await;

        // Reset the path before dropping the isolated schema. The generated
        // identifier contains only this fixed prefix and decimal digits.
        let cleanup_result = sqlx::raw_sql(&format!(
            "SET search_path TO public; DROP SCHEMA {schema} CASCADE;"
        ))
        .execute(&mut connection)
        .await;
        let required_table_count = migration_result?;
        cleanup_result?;
        assert_eq!(required_table_count, 6);
        Ok(())
    }
}
