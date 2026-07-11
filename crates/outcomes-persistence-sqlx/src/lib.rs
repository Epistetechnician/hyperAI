//! `SQLx` persistence for immutable protocol state, events, idempotency, and outbox.

use std::{path::Path, str::FromStr, time::Duration};

use outcomes_domain::{
    CanonicalizationError, ClaimSpecError, ClaimSpecV1, MutationContext, Sha256Digest, StateSlice,
    UndeclaredMutation, UtcTimestamp, canonical_json, domain_separated_digest,
};
use serde_json::json;
use sqlx::{
    Row, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

const SQLITE_MIGRATION_V1: &str = include_str!("../../../migrations/sqlite/0001_initial.sql");

/// Result of an idempotent immutable-claim freeze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeOutcome {
    /// This transaction inserted the frozen claim and its events.
    Inserted,
    /// The identical derived claim already existed; no duplicate event was emitted.
    ExistingIdentical,
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

        let checksum = domain_separated_digest(
            "hsai.sqlite-migration.v1",
            &json!({"version": 1_u64, "sql": SQLITE_MIGRATION_V1}),
        )?
        .to_string();
        let existing = sqlx::query("SELECT checksum FROM hsai_schema_migrations WHERE version = 1")
            .fetch_optional(&self.pool)
            .await?;
        if let Some(row) = existing {
            let stored: String = row.try_get("checksum")?;
            if stored == checksum {
                return Ok(());
            }
            return Err(PersistenceError::MigrationChecksumConflict { version: 1 });
        }

        let mut transaction = self.pool.begin().await?;
        sqlx::raw_sql(SQLITE_MIGRATION_V1)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "INSERT INTO hsai_schema_migrations(version, checksum, applied_at) VALUES(1, ?, ?)",
        )
        .bind(checksum)
        .bind("2026-07-10T00:00:00Z")
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdempotencyStart {
    Inserted,
    Existing,
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
    use outcomes_domain::{MutationScope, fixtures::local_demo_claim_draft};

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
}
