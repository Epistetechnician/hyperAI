//! `SQLx` persistence for immutable protocol state, events, idempotency, and outbox.

use std::{path::Path, str::FromStr, time::Duration};

use outcomes_domain::{
    AuthorizationDenialError, AuthorizationDenialRecordV1, CanonicalizationError, ClaimSpecError,
    ClaimSpecV1, DisputeDecisionKind, DisputeDecisionRecordV1, DisputeRecordV1, MarketBindingV1,
    MutationContext, OutcomeEvidenceBundleV1, ReconciliationResultV1, ReconciliationStatus,
    ResolutionRecordV1, ResolutionSupersessionRecordV1, SettlementLifecycle, Sha256Digest,
    StateSlice, SubmissionReceiptV1, UndeclaredMutation, UnsignedSettlementIntentV1, UtcTimestamp,
    canonical_json, domain_separated_digest, submission_receipt_digest,
};
use serde::Serialize;
use serde_json::json;
use sqlx::{
    Row, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

const SQLITE_MIGRATION_V1: &str = include_str!("../../../migrations/sqlite/0001_initial.sql");
const SQLITE_MIGRATION_V2: &str =
    include_str!("../../../migrations/sqlite/0002_settlement_recovery.sql");
const SQLITE_MIGRATION_V3: &str =
    include_str!("../../../migrations/sqlite/0003_dispute_resolution_history.sql");

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

/// Exact durable record cardinality and identities for one settlement recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementRecoveryIntegrityV1 {
    /// Fully decoded restart state.
    pub recovery: SettlementRecoveryState,
    /// Matching durable intent rows; exactly one for a valid recovery.
    pub intent_count: u64,
    /// Matching durable receipt rows; zero before recovery and one afterwards.
    pub receipt_count: u64,
    /// Matching append-only reconciliation rows.
    pub reconciliation_count: u64,
    /// Canonical durable receipt identity, when present.
    pub receipt_digest: Option<Sha256Digest>,
    /// Canonical identity of the latest reconciliation, when present.
    pub latest_reconciliation_digest: Option<Sha256Digest>,
}

/// Read-only durable identity and cardinality for one authorization denial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationDenialIntegrityV1 {
    /// Decoded and identity-verified denial record.
    pub record: AuthorizationDenialRecordV1,
    /// Rows matching the deterministic denial identity.
    pub row_count: u64,
    /// Durable audit envelope digest.
    pub audit_digest: Sha256Digest,
}

/// Durable append-only dispute state loaded after process restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeRecoveryState {
    /// Immutable opening record.
    pub open_dispute: DisputeRecordV1,
    /// Terminal decision, when one has been appended.
    pub decision: Option<DisputeDecisionRecordV1>,
    /// Supersession edge for an accepted decision.
    pub supersession: Option<ResolutionSupersessionRecordV1>,
    /// Successor resolution referenced by the supersession edge.
    pub successor_resolution: Option<ResolutionRecordV1>,
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
    /// Authorization-denial record identity is invalid.
    #[error("invalid authorization denial: {0}")]
    AuthorizationDenial(#[from] AuthorizationDenialError),
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
    /// A dispute, decision, resolution, or supersession identity was reused.
    #[error("dispute-resolution identity conflict for {identity}: {reason}")]
    DisputeResolutionIdentityConflict {
        /// Conflicting immutable identity.
        identity: String,
        /// Exact conflict class.
        reason: &'static str,
    },
    /// Settlement prerequisites do not form one exact claim/evidence/resolution/binding chain.
    #[error("settlement prerequisite identity conflict: {0}")]
    SettlementPrerequisiteConflict(&'static str),
    /// Authorization-denial identity was reused for different semantics.
    #[error("authorization denial identity conflict for {0}")]
    AuthorizationDenialConflict(String),
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

    /// Open an existing `SQLite` artifact without migrations or filesystem writes.
    ///
    /// # Errors
    ///
    /// Returns a database error when the immutable read-only artifact cannot be opened.
    pub async fn connect_read_only(path: &Path) -> Result<Self, PersistenceError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .foreign_keys(true)
            .read_only(true)
            .immutable(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Close every pooled connection and wait for `SQLite` to release the file.
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Checkpoint and truncate the WAL before closing a retained `SQLite` artifact.
    ///
    /// # Errors
    ///
    /// Returns a database error when the WAL checkpoint cannot complete.
    pub async fn checkpoint_and_close(self) -> Result<(), PersistenceError> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await?;
        self.pool.close().await;
        Ok(())
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
        apply_sqlite_migration(&self.pool, 3, SQLITE_MIGRATION_V3, "2026-07-11T12:00:00Z").await?;
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

    /// Append one sanitized authorization denial without an outbox event.
    ///
    /// The mutation scope must contain exactly `AccessControlV1` and `OperationsV1`.
    /// Exact replay is idempotent; identity reuse with different semantics fails closed.
    ///
    /// # Errors
    ///
    /// Returns a scope, identity, serialization, or database error.
    pub async fn append_authorization_denial(
        &self,
        context: &MutationContext,
        denial: &AuthorizationDenialRecordV1,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_authorization_denial_scope(context)?;
        denial.verify_identity()?;
        if context.actor_id != denial.actor_id || context.request_id != denial.request_id {
            return Err(PersistenceError::AuthorizationDenialConflict(
                denial.denial_id.clone(),
            ));
        }
        let payload_json = serde_json::to_string(denial)?;
        let audit_digest = domain_separated_digest(
            "hsai.authorization-denial-audit.v1",
            &(
                &context.actor_id,
                &context.request_id,
                &context.scope,
                denial.record_digest,
            ),
        )?;
        let mut transaction = self.pool.begin().await?;
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO audit_events(audit_id, event_type, actor_id, request_id, declared_state_slices_json, aggregate_type, aggregate_id, payload_digest, payload_json, received_at, audit_digest) VALUES(?, 'AUTHORIZATION_DENIED', ?, ?, ?, 'AUTHORIZATION', ?, ?, ?, ?, ?)",
        )
        .bind(&denial.denial_id)
        .bind(&denial.actor_id)
        .bind(&denial.request_id)
        .bind(serde_json::to_string(&context.scope)?)
        .bind(&denial.denial_id)
        .bind(denial.record_digest.to_string())
        .bind(&payload_json)
        .bind(denial.denied_at.to_string())
        .bind(audit_digest.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected() == 1;
        if inserted {
            transaction.commit().await?;
            return Ok(AppendOutcome::Inserted);
        }
        let existing = sqlx::query(
            "SELECT actor_id, request_id, declared_state_slices_json, aggregate_id, payload_digest, payload_json, audit_digest FROM audit_events WHERE audit_id = ? OR audit_digest = ? LIMIT 1",
        )
        .bind(&denial.denial_id)
        .bind(audit_digest.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        let exact = existing.is_some_and(|row| {
            row.try_get::<String, _>("actor_id").ok().as_deref() == Some(&denial.actor_id)
                && row.try_get::<String, _>("request_id").ok().as_deref()
                    == Some(&denial.request_id)
                && row
                    .try_get::<String, _>("declared_state_slices_json")
                    .ok()
                    .as_deref()
                    == serde_json::to_string(&context.scope).ok().as_deref()
                && row.try_get::<String, _>("aggregate_id").ok().as_deref()
                    == Some(&denial.denial_id)
                && row.try_get::<String, _>("payload_digest").ok().as_deref()
                    == Some(&denial.record_digest.to_string())
                && row.try_get::<String, _>("payload_json").ok().as_deref() == Some(&payload_json)
                && row.try_get::<String, _>("audit_digest").ok().as_deref()
                    == Some(&audit_digest.to_string())
        });
        if exact {
            transaction.commit().await?;
            Ok(AppendOutcome::ExistingIdentical)
        } else {
            Err(PersistenceError::AuthorizationDenialConflict(
                denial.denial_id.clone(),
            ))
        }
    }

    /// Read and verify one durable authorization denial.
    ///
    /// # Errors
    ///
    /// Returns a stored-data identity, serialization, or database error.
    pub async fn authorization_denial_integrity(
        &self,
        denial_id: &str,
    ) -> Result<Option<AuthorizationDenialIntegrityV1>, PersistenceError> {
        let rows = sqlx::query("SELECT payload_json, audit_digest FROM audit_events WHERE aggregate_type = 'AUTHORIZATION' AND aggregate_id = ?")
            .bind(denial_id)
            .fetch_all(&self.pool)
            .await?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let payload: String = row.try_get("payload_json")?;
        let record: AuthorizationDenialRecordV1 = serde_json::from_str(&payload)?;
        record.verify_identity()?;
        let audit_digest_text: String = row.try_get("audit_digest")?;
        let audit_digest = serde_json::from_value(serde_json::Value::String(audit_digest_text))?;
        Ok(Some(AuthorizationDenialIntegrityV1 {
            record,
            row_count: u64::try_from(rows.len())
                .map_err(|_| PersistenceError::AuthorizationDenialConflict(denial_id.to_owned()))?,
            audit_digest,
        }))
    }

    /// Count all durable authorization-denial rows.
    ///
    /// # Errors
    ///
    /// Returns a database or invalid-count error.
    pub async fn authorization_denial_count(&self) -> Result<u64, PersistenceError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_events WHERE aggregate_type = 'AUTHORIZATION'",
        )
        .fetch_one(&self.pool)
        .await?;
        u64::try_from(count)
            .map_err(|_| PersistenceError::AuthorizationDenialConflict("negative-count".to_owned()))
    }

    /// Atomically persist the exact evidence, final resolution, and market binding needed by settlement.
    ///
    /// The claim must already be frozen. Required declared slices are `EvidenceStoreV1`,
    /// `ResolutionLedgerV1`, `MarketBindingV1`, `OperationsV1`, and `DeliveryV1`.
    ///
    /// # Errors
    ///
    /// Returns a scope, identity, binding, conflict, audit, outbox, or database error.
    #[allow(clippy::too_many_lines)]
    pub async fn persist_settlement_prerequisites(
        &self,
        context: &MutationContext,
        claim: &ClaimSpecV1,
        evidence_bundle: &OutcomeEvidenceBundleV1,
        final_resolution: &ResolutionRecordV1,
        binding: &MarketBindingV1,
        created_at: &UtcTimestamp,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_settlement_prerequisite_scope(context)?;
        validate_settlement_prerequisites(claim, evidence_bundle, final_resolution, binding)?;
        let evidence_json = serde_json::to_string(evidence_bundle)?;
        let resolution_json = serde_json::to_string(final_resolution)?;
        let binding_json = serde_json::to_string(binding)?;
        let mut transaction = self.pool.begin().await?;
        let frozen: Option<String> =
            sqlx::query_scalar("SELECT payload_json FROM claim_specs WHERE claim_id = ?")
                .bind(claim.claim_id.to_string())
                .fetch_optional(&mut *transaction)
                .await?;
        if frozen.as_deref() != Some(serde_json::to_string(claim)?.as_str()) {
            return Err(PersistenceError::SettlementPrerequisiteConflict(
                "frozen claim is missing or differs from supplied claim",
            ));
        }
        let evidence_inserted = sqlx::query(
            "INSERT OR IGNORE INTO evidence_manifests(evidence_digest, claim_id, payload_json, created_at) VALUES(?, ?, ?, ?)",
        )
        .bind(evidence_bundle.bundle_digest.to_string())
        .bind(claim.claim_id.to_string())
        .bind(&evidence_json)
        .bind(created_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected() == 1;
        let resolution_inserted = sqlx::query(
            "INSERT OR IGNORE INTO resolution_records(resolution_digest, claim_id, truth_value, finality, evidence_digest, payload_json, created_at) VALUES(?, ?, 'TRUE', 'FINAL', ?, ?, ?)",
        )
        .bind(final_resolution.record_digest.to_string())
        .bind(claim.claim_id.to_string())
        .bind(evidence_bundle.bundle_digest.to_string())
        .bind(&resolution_json)
        .bind(created_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected() == 1;
        let true_side = binding
            .sides
            .iter()
            .find(|side| side.side_index == binding.true_side_index)
            .ok_or(PersistenceError::SettlementPrerequisiteConflict(
                "true side is missing",
            ))?;
        let false_side = binding
            .sides
            .iter()
            .find(|side| side.side_index == binding.false_side_index)
            .ok_or(PersistenceError::SettlementPrerequisiteConflict(
                "false side is missing",
            ))?;
        let binding_inserted = sqlx::query(
            "INSERT OR IGNORE INTO market_bindings(binding_digest, claim_id, backend, network, external_market_id, true_side_id, false_side_id, metadata_digest, payload_json, created_at) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(binding.binding_digest.to_string())
        .bind(claim.claim_id.to_string())
        .bind(&binding.backend)
        .bind(enum_json_label(&binding.network)?)
        .bind(&binding.external_market_id)
        .bind(&true_side.side_id)
        .bind(&false_side.side_id)
        .bind(binding.external_metadata_digest.to_string())
        .bind(&binding_json)
        .bind(created_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected() == 1;
        if !(evidence_inserted && resolution_inserted && binding_inserted) {
            let existing_evidence: Option<String> = sqlx::query_scalar(
                "SELECT payload_json FROM evidence_manifests WHERE evidence_digest = ?",
            )
            .bind(evidence_bundle.bundle_digest.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
            let existing_resolution: Option<String> = sqlx::query_scalar(
                "SELECT payload_json FROM resolution_records WHERE resolution_digest = ?",
            )
            .bind(final_resolution.record_digest.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
            let existing_binding: Option<String> = sqlx::query_scalar(
                "SELECT payload_json FROM market_bindings WHERE binding_digest = ?",
            )
            .bind(binding.binding_digest.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
            if existing_evidence.as_deref() == Some(&evidence_json)
                && existing_resolution.as_deref() == Some(&resolution_json)
                && existing_binding.as_deref() == Some(&binding_json)
                && !evidence_inserted
                && !resolution_inserted
                && !binding_inserted
            {
                transaction.commit().await?;
                return Ok(AppendOutcome::ExistingIdentical);
            }
            return Err(PersistenceError::SettlementPrerequisiteConflict(
                "partial replay or immutable prerequisite identity conflict",
            ));
        }
        let payload = json!({
            "claim_id": claim.claim_id,
            "evidence_bundle_digest": evidence_bundle.bundle_digest,
            "final_resolution_digest": final_resolution.record_digest,
            "binding_digest": binding.binding_digest,
        });
        let payload_json = serde_json::to_string(&payload)?;
        let payload_digest = domain_separated_digest("hsai.settlement-prerequisites.v1", &payload)?;
        append_protocol_records(
            &mut transaction,
            context,
            "SETTLEMENT_PREREQUISITES_PERSISTED",
            "SETTLEMENT_PREREQUISITES",
            &claim.claim_id.to_string(),
            payload_digest,
            &payload_json,
            created_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(AppendOutcome::Inserted)
    }

    /// Append one immutable open dispute with audit and outbox records.
    ///
    /// Required declared slices are `DisputeFinalityV1`, `OperationsV1`, and
    /// `DeliveryV1`. Exact replay is idempotent and never rewrites the opening.
    ///
    /// # Errors
    ///
    /// Returns a scope, identity, serialization, foreign-key, or database error.
    pub async fn append_open_dispute(
        &self,
        context: &MutationContext,
        dispute: &DisputeRecordV1,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_open_dispute_scope(context)?;
        let request_digest = dispute.record_digest;
        let payload_json = serde_json::to_string(dispute)?;
        let mut transaction = self.pool.begin().await?;
        if begin_idempotency(
            &mut transaction,
            "append_open_dispute",
            &context.request_id,
            request_digest,
            &dispute.opened_at,
        )
        .await?
            == IdempotencyStart::Existing
        {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT payload_json FROM disputes WHERE dispute_id = ?")
                    .bind(&dispute.dispute_id)
                    .fetch_optional(&mut *transaction)
                    .await?;
            transaction.commit().await?;
            return if existing.as_deref() == Some(payload_json.as_str()) {
                Ok(AppendOutcome::ExistingIdentical)
            } else {
                Err(PersistenceError::DisputeResolutionIdentityConflict {
                    identity: dispute.dispute_id.clone(),
                    reason: "dispute ID reused for different semantics",
                })
            };
        }
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO disputes(dispute_id, claim_id, provisional_resolution_digest, \
             status, payload_json, opened_at, closes_at) VALUES(?, ?, ?, 'OPEN', ?, ?, ?)",
        )
        .bind(&dispute.dispute_id)
        .bind(dispute.claim_id.to_string())
        .bind(dispute.provisional_resolution_digest.to_string())
        .bind(&payload_json)
        .bind(dispute.opened_at.to_string())
        .bind(dispute.closes_at.to_string())
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1;
        if !inserted {
            return Err(PersistenceError::DisputeResolutionIdentityConflict {
                identity: dispute.dispute_id.clone(),
                reason: "dispute ID already exists",
            });
        }
        append_protocol_records(
            &mut transaction,
            context,
            "DISPUTE_OPENED",
            "DISPUTE",
            &dispute.dispute_id,
            dispute.record_digest,
            &payload_json,
            &dispute.opened_at,
        )
        .await?;
        complete_idempotency(
            &mut transaction,
            "append_open_dispute",
            &context.request_id,
            dispute.record_digest,
            &dispute.opened_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(AppendOutcome::Inserted)
    }

    /// Atomically append a terminal accepted decision, successor resolution,
    /// and immutable supersession edge.
    ///
    /// # Errors
    ///
    /// Returns a scope, binding, identity, serialization, foreign-key, audit,
    /// outbox, or database error. The transaction commits no partial chain.
    #[allow(clippy::too_many_lines)]
    pub async fn append_accepted_dispute_supersession(
        &self,
        context: &MutationContext,
        decision: &DisputeDecisionRecordV1,
        successor: &ResolutionRecordV1,
        supersession: &ResolutionSupersessionRecordV1,
    ) -> Result<AppendOutcome, PersistenceError> {
        require_dispute_supersession_scope(context)?;
        validate_supersession_identity(decision, successor, supersession)?;
        let payload = json!({
            "decision": decision,
            "successor_resolution": successor,
            "supersession": supersession,
        });
        let request_digest =
            domain_separated_digest("hsai.persist-dispute-supersession-request.v1", &payload)?;
        let payload_json = serde_json::to_string(&payload)?;
        let mut transaction = self.pool.begin().await?;
        if begin_idempotency(
            &mut transaction,
            "append_dispute_supersession",
            &context.request_id,
            request_digest,
            &decision.decided_at,
        )
        .await?
            == IdempotencyStart::Existing
        {
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT payload_json FROM resolution_supersessions WHERE supersession_digest = ?",
            )
            .bind(supersession.record_digest.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
            transaction.commit().await?;
            return if existing.as_deref() == Some(serde_json::to_string(supersession)?.as_str()) {
                Ok(AppendOutcome::ExistingIdentical)
            } else {
                Err(PersistenceError::DisputeResolutionIdentityConflict {
                    identity: supersession.supersession_id.clone(),
                    reason: "supersession replay differs from durable record",
                })
            };
        }

        let open_payload: Option<String> = sqlx::query_scalar(
            "SELECT payload_json FROM disputes WHERE dispute_id = ? AND claim_id = ? \
             AND provisional_resolution_digest = ?",
        )
        .bind(&decision.dispute_id)
        .bind(decision.claim_id.to_string())
        .bind(decision.provisional_resolution_digest.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        let open_payload =
            open_payload.ok_or_else(|| PersistenceError::DisputeResolutionIdentityConflict {
                identity: decision.dispute_id.clone(),
                reason: "matching open dispute does not exist",
            })?;
        let open: DisputeRecordV1 = serde_json::from_str(&open_payload)?;
        if open.record_digest != decision.open_dispute_digest {
            return Err(PersistenceError::DisputeResolutionIdentityConflict {
                identity: decision.dispute_id.clone(),
                reason: "decision does not bind the durable open dispute",
            });
        }

        sqlx::query(
            "INSERT INTO resolution_records(resolution_digest, claim_id, truth_value, finality, \
             evidence_digest, payload_json, created_at) VALUES(?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(successor.record_digest.to_string())
        .bind(successor.claim_id.to_string())
        .bind(enum_json_label(&successor.truth)?)
        .bind(enum_json_label(&successor.finality)?)
        .bind(successor.evidence_bundle_digest.to_string())
        .bind(serde_json::to_string(successor)?)
        .bind(supersession.linked_at.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(|_| PersistenceError::DisputeResolutionIdentityConflict {
            identity: successor.resolution_id.clone(),
            reason: "successor resolution identity already exists or is invalid",
        })?;
        sqlx::query(
            "INSERT INTO dispute_decisions(decision_digest, dispute_id, claim_id, terminal_status, \
             successor_resolution_digest, payload_json, decided_at) VALUES(?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(decision.record_digest.to_string())
        .bind(&decision.dispute_id)
        .bind(decision.claim_id.to_string())
        .bind(enum_json_label(&decision.decision)?)
        .bind(successor.record_digest.to_string())
        .bind(serde_json::to_string(decision)?)
        .bind(decision.decided_at.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(|_| PersistenceError::DisputeResolutionIdentityConflict {
            identity: decision.decision_id.clone(),
            reason: "dispute already has a terminal decision or identity conflicts",
        })?;
        sqlx::query(
            "INSERT INTO resolution_supersessions(supersession_digest, claim_id, \
             predecessor_resolution_digest, successor_resolution_digest, dispute_decision_digest, \
             payload_json, created_at) VALUES(?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(supersession.record_digest.to_string())
        .bind(supersession.claim_id.to_string())
        .bind(supersession.predecessor_resolution_digest.to_string())
        .bind(supersession.successor_resolution_digest.to_string())
        .bind(supersession.dispute_decision_digest.to_string())
        .bind(serde_json::to_string(supersession)?)
        .bind(supersession.linked_at.to_string())
        .execute(&mut *transaction)
        .await?;
        append_protocol_records(
            &mut transaction,
            context,
            "DISPUTE_ACCEPTED_SUPERSEDED",
            "DISPUTE",
            &decision.dispute_id,
            request_digest,
            &payload_json,
            &decision.decided_at,
        )
        .await?;
        complete_idempotency(
            &mut transaction,
            "append_dispute_supersession",
            &context.request_id,
            supersession.record_digest,
            &decision.decided_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(AppendOutcome::Inserted)
    }

    /// Load the immutable dispute chain after restart.
    ///
    /// # Errors
    ///
    /// Returns a database or stored-payload decoding error.
    pub async fn dispute_recovery_state(
        &self,
        dispute_id: &str,
    ) -> Result<Option<DisputeRecoveryState>, PersistenceError> {
        let open_payload: Option<String> =
            sqlx::query_scalar("SELECT payload_json FROM disputes WHERE dispute_id = ?")
                .bind(dispute_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(open_payload) = open_payload else {
            return Ok(None);
        };
        let open_dispute = serde_json::from_str(&open_payload)?;
        let decision_payload: Option<String> =
            sqlx::query_scalar("SELECT payload_json FROM dispute_decisions WHERE dispute_id = ?")
                .bind(dispute_id)
                .fetch_optional(&self.pool)
                .await?;
        let decision: Option<DisputeDecisionRecordV1> = decision_payload
            .map(|payload| serde_json::from_str(&payload))
            .transpose()?;
        let supersession_payload: Option<String> = match decision.as_ref() {
            Some(decision) => sqlx::query_scalar(
                "SELECT payload_json FROM resolution_supersessions WHERE dispute_decision_digest = ?",
            )
            .bind(decision.record_digest.to_string())
            .fetch_optional(&self.pool)
            .await?,
            None => None,
        };
        let supersession: Option<ResolutionSupersessionRecordV1> = supersession_payload
            .map(|payload| serde_json::from_str(&payload))
            .transpose()?;
        let successor_resolution = match supersession.as_ref() {
            Some(link) => {
                let payload: Option<String> = sqlx::query_scalar(
                    "SELECT payload_json FROM resolution_records WHERE resolution_digest = ?",
                )
                .bind(link.successor_resolution_digest.to_string())
                .fetch_optional(&self.pool)
                .await?;
                payload
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?
            }
            None => None,
        };
        Ok(Some(DisputeRecoveryState {
            open_dispute,
            decision,
            supersession,
            successor_resolution,
        }))
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
        let receipt_digest = submission_receipt_digest(receipt)?;
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
            let durable_receipt_digest = submission_receipt_digest(&receipt)?;
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

    /// Read exact durable recovery cardinalities and canonical record identities.
    ///
    /// # Errors
    ///
    /// Returns a database, JSON, or canonical identity error.
    pub async fn settlement_recovery_integrity(
        &self,
        intent_id: &str,
    ) -> Result<Option<SettlementRecoveryIntegrityV1>, PersistenceError> {
        let Some(recovery) = self.settlement_recovery_state(intent_id).await? else {
            return Ok(None);
        };
        let intent_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_intents WHERE intent_id = ?")
                .bind(intent_id)
                .fetch_one(&self.pool)
                .await?;
        let receipt_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_receipts WHERE intent_id = ?")
                .bind(intent_id)
                .fetch_one(&self.pool)
                .await?;
        let reconciliation_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM settlement_reconciliations WHERE intent_id = ?",
        )
        .bind(intent_id)
        .fetch_one(&self.pool)
        .await?;
        let receipt_digest = recovery
            .receipt
            .as_ref()
            .map(submission_receipt_digest)
            .transpose()?;
        let latest_reconciliation_digest = recovery
            .latest_reconciliation
            .as_ref()
            .map(|value| {
                domain_separated_digest("hsai.persisted-settlement-reconciliation.v1", value)
            })
            .transpose()?;
        Ok(Some(SettlementRecoveryIntegrityV1 {
            recovery,
            intent_count: u64::try_from(intent_count).map_err(|_| {
                PersistenceError::SettlementPrerequisiteConflict("negative intent count")
            })?,
            receipt_count: u64::try_from(receipt_count).map_err(|_| {
                PersistenceError::SettlementPrerequisiteConflict("negative receipt count")
            })?,
            reconciliation_count: u64::try_from(reconciliation_count).map_err(|_| {
                PersistenceError::SettlementPrerequisiteConflict("negative reconciliation count")
            })?,
            receipt_digest,
            latest_reconciliation_digest,
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

fn require_authorization_denial_scope(context: &MutationContext) -> Result<(), PersistenceError> {
    context.scope.require(StateSlice::AccessControlV1)?;
    context.scope.require(StateSlice::OperationsV1)?;
    if context.scope.iter().count() != 2 {
        return Err(PersistenceError::AuthorizationDenialConflict(
            "unexpected-state-slice".to_owned(),
        ));
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

fn require_settlement_prerequisite_scope(
    context: &MutationContext,
) -> Result<(), PersistenceError> {
    for slice in [
        StateSlice::EvidenceStoreV1,
        StateSlice::ResolutionLedgerV1,
        StateSlice::MarketBindingV1,
        StateSlice::OperationsV1,
        StateSlice::DeliveryV1,
    ] {
        context.scope.require(slice)?;
    }
    Ok(())
}

#[derive(Serialize)]
struct PersistedBundleIdentity<'a> {
    schema_version: &'static str,
    claim_id: &'a outcomes_domain::ClaimId,
    claim_spec_digest: Sha256Digest,
    challenge_descriptor_digest: Sha256Digest,
    observation_snapshot_digests: &'a [Sha256Digest],
    completeness_assessment_digest: Sha256Digest,
    qualifying_candidate_digests: &'a [Sha256Digest],
    admission_decision_digests: &'a [Sha256Digest],
    execution_manifest_digests: &'a [Sha256Digest],
    resolution_aggregation_digest: Sha256Digest,
    dispute_record_digests: &'a [Sha256Digest],
    code_build_provenance_digest: Sha256Digest,
    evidence_publication_policy_digest: Sha256Digest,
    blobs: &'a [outcomes_domain::ContentBlobRefV1],
}

#[derive(Serialize)]
struct PersistedResolutionIdentity<'a> {
    schema_version: &'static str,
    claim_id: &'a outcomes_domain::ClaimId,
    truth: outcomes_domain::ClaimTruth,
    finality: outcomes_domain::ResolutionFinality,
    evidence_bundle_digest: Sha256Digest,
    attestation_digests: &'a [Sha256Digest],
    dispute_window_start: &'a UtcTimestamp,
    dispute_window_end: &'a UtcTimestamp,
}

fn validate_settlement_prerequisites(
    claim: &ClaimSpecV1,
    evidence: &OutcomeEvidenceBundleV1,
    resolution: &ResolutionRecordV1,
    binding: &MarketBindingV1,
) -> Result<(), PersistenceError> {
    claim.verify_identity()?;
    let claim_digest = claim.claim_spec_digest()?;
    if evidence.schema_version != "hsai.outcome-evidence-bundle.v1"
        || evidence.claim_id != claim.claim_id
        || evidence.claim_spec_digest != claim_digest
        || evidence.challenge_descriptor_digest != claim.challenge_descriptor_digest
    {
        return Err(PersistenceError::SettlementPrerequisiteConflict(
            "evidence does not bind frozen claim",
        ));
    }
    let evidence_digest = domain_separated_digest(
        "hsai.outcome-evidence-bundle.v1",
        &PersistedBundleIdentity {
            schema_version: "hsai.outcome-evidence-bundle.v1",
            claim_id: &evidence.claim_id,
            claim_spec_digest: evidence.claim_spec_digest,
            challenge_descriptor_digest: evidence.challenge_descriptor_digest,
            observation_snapshot_digests: &evidence.observation_snapshot_digests,
            completeness_assessment_digest: evidence.completeness_assessment_digest,
            qualifying_candidate_digests: &evidence.qualifying_candidate_digests,
            admission_decision_digests: &evidence.admission_decision_digests,
            execution_manifest_digests: &evidence.execution_manifest_digests,
            resolution_aggregation_digest: evidence.resolution_aggregation_digest,
            dispute_record_digests: &evidence.dispute_record_digests,
            code_build_provenance_digest: evidence.code_build_provenance_digest,
            evidence_publication_policy_digest: evidence.evidence_publication_policy_digest,
            blobs: &evidence.blobs,
        },
    )?;
    if evidence_digest != evidence.bundle_digest {
        return Err(PersistenceError::SettlementPrerequisiteConflict(
            "evidence bundle digest mismatch",
        ));
    }
    if resolution.schema_version != "hsai.resolution-record.v1"
        || resolution.claim_id != claim.claim_id
        || resolution.truth != outcomes_domain::ClaimTruth::True
        || resolution.finality != outcomes_domain::ResolutionFinality::Final
        || resolution.evidence_bundle_digest != evidence.bundle_digest
    {
        return Err(PersistenceError::SettlementPrerequisiteConflict(
            "resolution is not exact FINAL TRUE evidence binding",
        ));
    }
    let resolution_digest = domain_separated_digest(
        "hsai.resolution-record.v1",
        &PersistedResolutionIdentity {
            schema_version: "hsai.resolution-record.v1",
            claim_id: &resolution.claim_id,
            truth: resolution.truth,
            finality: resolution.finality,
            evidence_bundle_digest: resolution.evidence_bundle_digest,
            attestation_digests: &resolution.attestation_digests,
            dispute_window_start: &resolution.dispute_window_start,
            dispute_window_end: &resolution.dispute_window_end,
        },
    )?;
    if resolution_digest != resolution.record_digest
        || resolution.resolution_id != format!("resolution_{}", &resolution_digest.to_hex()[..32])
    {
        return Err(PersistenceError::SettlementPrerequisiteConflict(
            "resolution digest mismatch",
        ));
    }
    let binding_digest = domain_separated_digest(
        "hsai.market-binding.v1",
        &(
            &binding.backend,
            binding.network,
            &binding.external_market_id,
            &binding.external_question_id,
            binding.external_metadata_digest,
            &binding.claim_id,
            binding.claim_spec_digest,
            binding.true_side_index,
            binding.false_side_index,
            binding.capability_snapshot_digest,
        ),
    )?;
    if binding.schema_version != "hsai.market-binding.v1"
        || binding.claim_id != claim.claim_id
        || binding.claim_spec_digest != claim_digest
        || binding.binding_digest != binding_digest
        || binding.true_side_index == binding.false_side_index
        || binding.invalidation_reason.is_some()
    {
        return Err(PersistenceError::SettlementPrerequisiteConflict(
            "market binding identity mismatch",
        ));
    }
    Ok(())
}

fn require_open_dispute_scope(context: &MutationContext) -> Result<(), PersistenceError> {
    for slice in [
        StateSlice::DisputeFinalityV1,
        StateSlice::OperationsV1,
        StateSlice::DeliveryV1,
    ] {
        context.scope.require(slice)?;
    }
    Ok(())
}

fn require_dispute_supersession_scope(context: &MutationContext) -> Result<(), PersistenceError> {
    require_open_dispute_scope(context)?;
    context.scope.require(StateSlice::ResolutionLedgerV1)?;
    Ok(())
}

fn validate_supersession_identity(
    decision: &DisputeDecisionRecordV1,
    successor: &ResolutionRecordV1,
    supersession: &ResolutionSupersessionRecordV1,
) -> Result<(), PersistenceError> {
    if decision.decision != DisputeDecisionKind::AcceptedSupersededBeforeFinality
        || decision.claim_id != successor.claim_id
        || decision.claim_id != supersession.claim_id
        || decision.provisional_resolution_digest != supersession.predecessor_resolution_digest
        || decision.record_digest != supersession.dispute_decision_digest
        || successor.record_digest != supersession.successor_resolution_digest
        || supersession.predecessor_resolution_digest == supersession.successor_resolution_digest
    {
        return Err(PersistenceError::DisputeResolutionIdentityConflict {
            identity: supersession.supersession_id.clone(),
            reason: "decision, successor, and supersession bindings do not agree",
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn append_protocol_records(
    transaction: &mut Transaction<'_, Sqlite>,
    context: &MutationContext,
    event_type: &str,
    aggregate_type: &str,
    aggregate_id: &str,
    payload_digest: Sha256Digest,
    payload_json: &str,
    received_at: &UtcTimestamp,
) -> Result<(), PersistenceError> {
    let outbox_payload = json!({
        "event_type": event_type,
        "aggregate_type": aggregate_type,
        "aggregate_id": aggregate_id,
        "payload_digest": payload_digest,
    });
    let outbox_digest = domain_separated_digest("hsai.protocol-outbox.v1", &outbox_payload)?;
    sqlx::query(
        "INSERT INTO outbox_events(outbox_id, aggregate_type, aggregate_id, event_type, \
         payload_digest, payload_json, created_at) VALUES(?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(prefixed_id("outbox", outbox_digest))
    .bind(aggregate_type)
    .bind(aggregate_id)
    .bind(event_type)
    .bind(outbox_digest.to_string())
    .bind(serde_json::to_string(&outbox_payload)?)
    .bind(received_at.to_string())
    .execute(&mut **transaction)
    .await?;

    let audit_digest = domain_separated_digest(
        "hsai.protocol-audit.v1",
        &json!({
            "actor_id": context.actor_id,
            "request_id": context.request_id,
            "event_type": event_type,
            "aggregate_type": aggregate_type,
            "aggregate_id": aggregate_id,
            "payload_digest": payload_digest,
        }),
    )?;
    sqlx::query(
        "INSERT INTO audit_events(audit_id, event_type, actor_id, request_id, \
         declared_state_slices_json, aggregate_type, aggregate_id, payload_digest, payload_json, \
         received_at, audit_digest) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(prefixed_id("audit", audit_digest))
    .bind(event_type)
    .bind(&context.actor_id)
    .bind(&context.request_id)
    .bind(serde_json::to_string(&context.scope)?)
    .bind(aggregate_type)
    .bind(aggregate_id)
    .bind(payload_digest.to_string())
    .bind(payload_json)
    .bind(received_at.to_string())
    .bind(audit_digest.to_string())
    .execute(&mut **transaction)
    .await?;
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
    use hyperliquid_hip4::Hip4MockBackend;
    use outcomes_domain::{
        AuthorizationActionV1, AuthorizationActorClassV1, AuthorizationDenialReasonV1, ClaimTruth,
        ExternalSideV1, MarketBindingV1, MutationScope, ResolutionFinality, SettlementNetwork,
        fixtures::local_demo_claim_draft,
    };

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

    fn prerequisite_context(request_id: &str) -> MutationContext {
        MutationContext {
            actor_id: "beta-local".to_owned(),
            request_id: request_id.to_owned(),
            scope: MutationScope::new([
                StateSlice::EvidenceStoreV1,
                StateSlice::ResolutionLedgerV1,
                StateSlice::MarketBindingV1,
                StateSlice::OperationsV1,
                StateSlice::DeliveryV1,
            ]),
        }
    }

    fn denial_context(request_id: &str) -> MutationContext {
        MutationContext {
            actor_id: "public-reader".to_owned(),
            request_id: request_id.to_owned(),
            scope: MutationScope::new([StateSlice::AccessControlV1, StateSlice::OperationsV1]),
        }
    }

    fn denial(request_id: &str) -> Result<AuthorizationDenialRecordV1, Box<dyn std::error::Error>> {
        Ok(AuthorizationDenialRecordV1::new(
            request_id.to_owned(),
            "public-reader".to_owned(),
            AuthorizationActorClassV1::Anonymous,
            AuthorizationActionV1::DependencyHealthRead,
            AuthorizationDenialReasonV1::MissingRole,
            timestamp("2026-07-12T12:00:00Z")?,
        )?)
    }

    #[allow(clippy::too_many_lines)]
    fn valid_prerequisites(
        claim: &ClaimSpecV1,
    ) -> Result<
        (OutcomeEvidenceBundleV1, ResolutionRecordV1, MarketBindingV1),
        Box<dyn std::error::Error>,
    > {
        let mut evidence = OutcomeEvidenceBundleV1 {
            schema_version: "hsai.outcome-evidence-bundle.v1".to_owned(),
            claim_id: claim.claim_id.clone(),
            claim_spec_digest: claim.claim_spec_digest()?,
            challenge_descriptor_digest: claim.challenge_descriptor_digest,
            observation_snapshot_digests: vec![digest("snapshot")?],
            completeness_assessment_digest: digest("complete")?,
            qualifying_candidate_digests: vec![digest("candidate")?],
            admission_decision_digests: vec![digest("admission")?],
            execution_manifest_digests: vec![digest("run-1")?, digest("run-2")?],
            resolution_aggregation_digest: digest("aggregation")?,
            dispute_record_digests: Vec::new(),
            code_build_provenance_digest: digest("build")?,
            evidence_publication_policy_digest: digest("publication")?,
            blobs: Vec::new(),
            bundle_digest: digest("placeholder")?,
        };
        evidence.bundle_digest = domain_separated_digest(
            "hsai.outcome-evidence-bundle.v1",
            &PersistedBundleIdentity {
                schema_version: "hsai.outcome-evidence-bundle.v1",
                claim_id: &evidence.claim_id,
                claim_spec_digest: evidence.claim_spec_digest,
                challenge_descriptor_digest: evidence.challenge_descriptor_digest,
                observation_snapshot_digests: &evidence.observation_snapshot_digests,
                completeness_assessment_digest: evidence.completeness_assessment_digest,
                qualifying_candidate_digests: &evidence.qualifying_candidate_digests,
                admission_decision_digests: &evidence.admission_decision_digests,
                execution_manifest_digests: &evidence.execution_manifest_digests,
                resolution_aggregation_digest: evidence.resolution_aggregation_digest,
                dispute_record_digests: &evidence.dispute_record_digests,
                code_build_provenance_digest: evidence.code_build_provenance_digest,
                evidence_publication_policy_digest: evidence.evidence_publication_policy_digest,
                blobs: &evidence.blobs,
            },
        )?;
        let start = timestamp("2026-07-10T10:00:00Z")?;
        let end = timestamp("2026-07-10T10:10:00Z")?;
        let attestations = vec![digest("attestation")?];
        let resolution_digest = domain_separated_digest(
            "hsai.resolution-record.v1",
            &PersistedResolutionIdentity {
                schema_version: "hsai.resolution-record.v1",
                claim_id: &claim.claim_id,
                truth: ClaimTruth::True,
                finality: ResolutionFinality::Final,
                evidence_bundle_digest: evidence.bundle_digest,
                attestation_digests: &attestations,
                dispute_window_start: &start,
                dispute_window_end: &end,
            },
        )?;
        let resolution = ResolutionRecordV1 {
            schema_version: "hsai.resolution-record.v1".to_owned(),
            resolution_id: format!("resolution_{}", &resolution_digest.to_hex()[..32]),
            claim_id: claim.claim_id.clone(),
            truth: ClaimTruth::True,
            finality: ResolutionFinality::Final,
            evidence_bundle_digest: evidence.bundle_digest,
            attestation_digests: attestations,
            dispute_window_start: start,
            dispute_window_end: end,
            record_digest: resolution_digest,
        };
        let metadata = digest("metadata")?;
        let capability = digest("capability")?;
        let binding_digest = domain_separated_digest(
            "hsai.market-binding.v1",
            &(
                "mock-hip4",
                SettlementNetwork::Mock,
                "outcome-42",
                &Option::<String>::None,
                metadata,
                &claim.claim_id,
                claim.claim_spec_digest()?,
                0_u8,
                1_u8,
                capability,
            ),
        )?;
        let binding = MarketBindingV1 {
            schema_version: "hsai.market-binding.v1".to_owned(),
            backend: "mock-hip4".to_owned(),
            network: SettlementNetwork::Mock,
            external_market_id: "outcome-42".to_owned(),
            external_question_id: None,
            sides: vec![
                ExternalSideV1 {
                    side_index: 0,
                    side_id: "side-0".to_owned(),
                    display_name: "TRUE".to_owned(),
                },
                ExternalSideV1 {
                    side_index: 1,
                    side_id: "side-1".to_owned(),
                    display_name: "FALSE".to_owned(),
                },
            ],
            true_side_index: 0,
            false_side_index: 1,
            true_settlement_fraction: "1".to_owned(),
            false_settlement_fraction: "0".to_owned(),
            quote_token: None,
            external_metadata_digest: metadata,
            raw_payload_digest: digest("raw")?,
            claim_id: claim.claim_id.clone(),
            claim_spec_digest: claim.claim_spec_digest()?,
            external_claim_commitment: None,
            expected_settlement_authority: Some("mock".to_owned()),
            capability_snapshot_digest: capability,
            capability_version: "hsai.backend-capability.v1".to_owned(),
            created_by: "test".to_owned(),
            created_at: timestamp("2026-07-10T10:00:00Z")?,
            binding_signature: "mock-signature".to_owned(),
            binding_digest,
            invalidation_reason: None,
        };
        Ok((evidence, resolution, binding))
    }

    fn dispute_context(request_id: &str, include_resolution: bool) -> MutationContext {
        let mut slices = vec![
            StateSlice::DisputeFinalityV1,
            StateSlice::OperationsV1,
            StateSlice::DeliveryV1,
        ];
        if include_resolution {
            slices.push(StateSlice::ResolutionLedgerV1);
        }
        MutationContext {
            actor_id: "dispute-authority-local".to_owned(),
            request_id: request_id.to_owned(),
            scope: MutationScope::new(slices),
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

    async fn seed_dispute_dependencies(
        store: &SqliteStore,
    ) -> Result<(ClaimSpecV1, ResolutionRecordV1, Sha256Digest), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        store
            .freeze_claim(
                &freeze_context("dispute-seed"),
                &claim,
                &timestamp("2026-07-10T10:01:00Z")?,
                "test-build",
            )
            .await?;
        let evidence_digest = digest("dispute-evidence")?;
        sqlx::query(
            "INSERT INTO evidence_manifests(evidence_digest, claim_id, payload_json, created_at) \
             VALUES(?, ?, '{}', '2026-07-10T10:02:00Z')",
        )
        .bind(evidence_digest.to_string())
        .bind(claim.claim_id.to_string())
        .execute(&store.pool)
        .await?;
        let predecessor = ResolutionRecordV1 {
            schema_version: "hsai.resolution-record.v1".to_owned(),
            resolution_id: "resolution_predecessor".to_owned(),
            claim_id: claim.claim_id.clone(),
            truth: outcomes_domain::ClaimTruth::False,
            finality: outcomes_domain::ResolutionFinality::Provisional,
            evidence_bundle_digest: evidence_digest,
            attestation_digests: vec![digest("predecessor-attestation")?],
            dispute_window_start: timestamp("2026-07-10T10:03:00Z")?,
            dispute_window_end: timestamp("2026-07-10T11:03:00Z")?,
            record_digest: digest("predecessor-resolution")?,
        };
        sqlx::query(
            "INSERT INTO resolution_records(resolution_digest, claim_id, truth_value, finality, \
             evidence_digest, payload_json, created_at) VALUES(?, ?, 'FALSE', 'PROVISIONAL', ?, ?, ?)",
        )
        .bind(predecessor.record_digest.to_string())
        .bind(claim.claim_id.to_string())
        .bind(evidence_digest.to_string())
        .bind(serde_json::to_string(&predecessor)?)
        .bind(predecessor.dispute_window_start.to_string())
        .execute(&store.pool)
        .await?;
        Ok((claim, predecessor, evidence_digest))
    }

    fn dispute_chain(
        claim: &ClaimSpecV1,
        predecessor: &ResolutionRecordV1,
        evidence_digest: Sha256Digest,
    ) -> Result<
        (
            DisputeRecordV1,
            DisputeDecisionRecordV1,
            ResolutionRecordV1,
            ResolutionSupersessionRecordV1,
        ),
        Box<dyn std::error::Error>,
    > {
        let open = DisputeRecordV1 {
            schema_version: "hsai.dispute-record.v1".to_owned(),
            dispute_id: "dispute_counterexample".to_owned(),
            claim_id: claim.claim_id.clone(),
            provisional_resolution_digest: predecessor.record_digest,
            opened_at: timestamp("2026-07-10T10:30:00Z")?,
            closes_at: predecessor.dispute_window_end.clone(),
            authenticated_actor_id: "challenger-local".to_owned(),
            reason_code: "VALID_COUNTEREXAMPLE".to_owned(),
            evidence_digests: vec![digest("counterexample")?],
            counterexample_candidate_digest: Some(digest("candidate")?),
            status: outcomes_domain::DisputeStatus::Open,
            decision_rationale: None,
            record_digest: digest("open-dispute")?,
        };
        let decision = DisputeDecisionRecordV1 {
            schema_version: "hsai.dispute-decision-record.v1".to_owned(),
            decision_id: "decision_counterexample".to_owned(),
            dispute_id: open.dispute_id.clone(),
            claim_id: claim.claim_id.clone(),
            open_dispute_digest: open.record_digest,
            provisional_resolution_digest: predecessor.record_digest,
            authority_id: "dispute-authority-local".to_owned(),
            decided_at: timestamp("2026-07-10T10:40:00Z")?,
            decision: DisputeDecisionKind::AcceptedSupersededBeforeFinality,
            rationale: "counterexample validated by bounded recomputation".to_owned(),
            decision_evidence_digests: vec![digest("decision-evidence")?],
            recomputation_evidence_bundle_digest: Some(evidence_digest),
            recomputation_aggregation_digest: Some(digest("recomputation")?),
            record_digest: digest("decision")?,
        };
        let successor = ResolutionRecordV1 {
            schema_version: "hsai.resolution-record.v1".to_owned(),
            resolution_id: "resolution_successor".to_owned(),
            claim_id: claim.claim_id.clone(),
            truth: outcomes_domain::ClaimTruth::True,
            finality: outcomes_domain::ResolutionFinality::Provisional,
            evidence_bundle_digest: evidence_digest,
            attestation_digests: vec![digest("successor-attestation")?],
            dispute_window_start: decision.decided_at.clone(),
            dispute_window_end: timestamp("2026-07-10T11:40:00Z")?,
            record_digest: digest("successor-resolution")?,
        };
        let supersession = ResolutionSupersessionRecordV1 {
            schema_version: "hsai.resolution-supersession-record.v1".to_owned(),
            supersession_id: "supersession_counterexample".to_owned(),
            claim_id: claim.claim_id.clone(),
            predecessor_resolution_digest: predecessor.record_digest,
            dispute_decision_digest: decision.record_digest,
            successor_resolution_digest: successor.record_digest,
            linked_at: decision.decided_at.clone(),
            record_digest: digest("supersession")?,
        };
        Ok((open, decision, successor, supersession))
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
            receipt_digest: Some(submission_receipt_digest(receipt)?),
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
        assert_eq!(migration_count, 3);
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

    fn joined_recovery_records(
        binding_digest: Sha256Digest,
        resolution_digest: Sha256Digest,
    ) -> Result<(MarketBindingV1, ResolutionRecordV1), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let resolution = ResolutionRecordV1 {
            schema_version: "hsai.resolution-record.v1".to_owned(),
            resolution_id: "resolution_joined_recovery".to_owned(),
            claim_id: claim.claim_id.clone(),
            truth: ClaimTruth::True,
            finality: ResolutionFinality::Final,
            evidence_bundle_digest: digest("evidence")?,
            attestation_digests: vec![digest("joined-attestation")?],
            dispute_window_start: timestamp("2026-07-10T10:03:00Z")?,
            dispute_window_end: timestamp("2026-07-10T10:04:00Z")?,
            record_digest: resolution_digest,
        };
        let binding = MarketBindingV1 {
            schema_version: "hsai.market-binding.v1".to_owned(),
            backend: "mock-hip4".to_owned(),
            network: SettlementNetwork::Mock,
            external_market_id: "outcome-42".to_owned(),
            external_question_id: None,
            sides: vec![
                ExternalSideV1 {
                    side_index: 0,
                    side_id: "side-0".to_owned(),
                    display_name: "TRUE".to_owned(),
                },
                ExternalSideV1 {
                    side_index: 1,
                    side_id: "side-1".to_owned(),
                    display_name: "FALSE".to_owned(),
                },
            ],
            true_side_index: 0,
            false_side_index: 1,
            true_settlement_fraction: "1".to_owned(),
            false_settlement_fraction: "0".to_owned(),
            quote_token: None,
            external_metadata_digest: digest("metadata")?,
            raw_payload_digest: digest("raw-market")?,
            claim_id: claim.claim_id.clone(),
            claim_spec_digest: domain_separated_digest("hsai.claim-spec.v1", &claim)?,
            external_claim_commitment: None,
            expected_settlement_authority: Some("mock-authority".to_owned()),
            capability_snapshot_digest: digest("capabilities")?,
            capability_version: "test".to_owned(),
            created_by: "operator-local".to_owned(),
            created_at: timestamp("2026-07-10T10:04:00Z")?,
            binding_signature: "development-binding-signature".to_owned(),
            binding_digest,
            invalidation_reason: None,
        };
        Ok((binding, resolution))
    }

    async fn settlement_record_counts(store: &SqliteStore) -> Result<(i64, i64, i64), sqlx::Error> {
        Ok((
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_intents")
                .fetch_one(&store.pool)
                .await?,
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_receipts")
                .fetch_one(&store.pool)
                .await?,
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_reconciliations")
                .fetch_one(&store.pool)
                .await?,
        ))
    }

    #[tokio::test]
    async fn contract_restart_after_external_acceptance_requires_reconciliation_not_resubmit()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("joined-settlement-recovery.db");
        let store = SqliteStore::connect_path(&database_path, 1).await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let (binding, resolution) = joined_recovery_records(binding_digest, resolution_digest)?;
        let request_digest = domain_separated_digest("hsai.mock-hip4-request.v1", &intent)?;
        store
            .persist_settlement_intent(
                &settlement_context("joined-before-submit"),
                &intent,
                request_digest,
                &timestamp("2026-07-10T10:05:00Z")?,
            )
            .await?;

        let mut backend = Hip4MockBackend::default();
        let lost_receipt = backend.submit(&intent, timestamp("2026-07-10T10:06:00Z")?)?;
        assert_eq!(lost_receipt.request_digest, request_digest);
        drop(lost_receipt);
        drop(store);

        let restarted = SqliteStore::connect_path(&database_path, 1).await?;
        restarted.migrate().await?;
        let recovery = restarted
            .settlement_recovery_state(&intent.intent_id)
            .await?
            .ok_or_else(|| std::io::Error::other("durable intent missing after restart"))?;
        assert_eq!(
            recovery.action,
            SettlementRecoveryAction::QueryExternalStatus
        );
        assert!(recovery.receipt.is_none());
        assert_eq!(backend.submission_call_count(), 1);

        let recovered_receipt = backend
            .recover_receipt(&recovery.intent)?
            .ok_or_else(|| std::io::Error::other("authoritative receipt missing"))?;
        assert_eq!(recovered_receipt.request_digest, recovery.request_digest);
        assert_eq!(backend.submission_call_count(), 1);
        assert_eq!(
            restarted
                .record_settlement_receipt(
                    &settlement_context("joined-recover-receipt"),
                    &recovered_receipt,
                    &timestamp("2026-07-10T10:06:01Z")?,
                )
                .await?,
            AppendOutcome::Inserted
        );
        let reconciliation =
            backend.reconcile(&binding, &resolution, timestamp("2026-07-10T10:07:00Z")?)?;
        assert_eq!(
            reconciliation.receipt_digest,
            Some(submission_receipt_digest(&recovered_receipt)?)
        );
        assert_eq!(
            restarted
                .record_settlement_reconciliation(
                    &settlement_context("joined-recover-reconciliation"),
                    &intent.intent_id,
                    &reconciliation,
                )
                .await?,
            AppendOutcome::Inserted
        );
        drop(restarted);

        let complete_store = SqliteStore::connect_path(&database_path, 1).await?;
        complete_store.migrate().await?;
        let complete = complete_store
            .settlement_recovery_state(&intent.intent_id)
            .await?
            .ok_or_else(|| std::io::Error::other("completed recovery state missing"))?;
        assert_eq!(complete.action, SettlementRecoveryAction::Complete);
        assert_eq!(complete.intent, intent);
        assert_eq!(complete.request_digest, request_digest);
        assert_eq!(complete.receipt, Some(recovered_receipt.clone()));
        assert_eq!(complete.latest_reconciliation, Some(reconciliation.clone()));
        assert_eq!(
            complete_store
                .record_settlement_receipt(
                    &settlement_context("joined-replay-receipt"),
                    &recovered_receipt,
                    &timestamp("2026-07-10T10:06:01Z")?,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        assert_eq!(
            complete_store
                .record_settlement_reconciliation(
                    &settlement_context("joined-replay-reconciliation"),
                    &intent.intent_id,
                    &reconciliation,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        assert_eq!(settlement_record_counts(&complete_store).await?, (1, 1, 1));
        assert_eq!(backend.submission_call_count(), 1);
        assert_eq!(backend.effect_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn settlement_prerequisites_are_atomic_scoped_and_idempotent()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let claim = local_demo_claim_draft()?.freeze()?;
        store
            .freeze_claim(
                &freeze_context("prereq-freeze"),
                &claim,
                &timestamp("2026-07-10T09:00:00Z")?,
                "test",
            )
            .await?;
        let (evidence, resolution, binding) = valid_prerequisites(&claim)?;
        let insufficient = MutationContext {
            actor_id: "beta-local".to_owned(),
            request_id: "prereq-bad-scope".to_owned(),
            scope: MutationScope::new([StateSlice::EvidenceStoreV1]),
        };
        assert!(matches!(
            store
                .persist_settlement_prerequisites(
                    &insufficient,
                    &claim,
                    &evidence,
                    &resolution,
                    &binding,
                    &timestamp("2026-07-10T10:00:00Z")?
                )
                .await,
            Err(PersistenceError::UndeclaredMutation(_))
        ));
        assert_eq!(
            store
                .persist_settlement_prerequisites(
                    &prerequisite_context("prereq-1"),
                    &claim,
                    &evidence,
                    &resolution,
                    &binding,
                    &timestamp("2026-07-10T10:00:00Z")?
                )
                .await?,
            AppendOutcome::Inserted
        );
        assert_eq!(
            store
                .persist_settlement_prerequisites(
                    &prerequisite_context("prereq-replay"),
                    &claim,
                    &evidence,
                    &resolution,
                    &binding,
                    &timestamp("2026-07-10T10:00:00Z")?
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        let mut conflicting = binding.clone();
        conflicting.binding_signature = "different".to_owned();
        assert!(matches!(
            store
                .persist_settlement_prerequisites(
                    &prerequisite_context("prereq-conflict"),
                    &claim,
                    &evidence,
                    &resolution,
                    &conflicting,
                    &timestamp("2026-07-10T10:00:00Z")?
                )
                .await,
            Err(PersistenceError::SettlementPrerequisiteConflict(_))
        ));
        let counts: (i64, i64, i64) = (
            sqlx::query_scalar("SELECT COUNT(*) FROM evidence_manifests")
                .fetch_one(&store.pool)
                .await?,
            sqlx::query_scalar("SELECT COUNT(*) FROM resolution_records")
                .fetch_one(&store.pool)
                .await?,
            sqlx::query_scalar("SELECT COUNT(*) FROM market_bindings")
                .fetch_one(&store.pool)
                .await?,
        );
        assert_eq!(counts, (1, 1, 1));
        Ok(())
    }

    #[tokio::test]
    async fn copied_database_opens_immutable_and_integrity_is_read_only()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("recovery.sqlite");
        let store = SqliteStore::connect_path(&path, 1).await?;
        store.migrate().await?;
        let (binding_digest, resolution_digest) = seed_settlement_dependencies(&store).await?;
        let intent = settlement_intent(binding_digest, resolution_digest)?;
        let request_digest = domain_separated_digest("hsai.mock-hip4-request.v1", &intent)?;
        store
            .persist_settlement_intent(
                &settlement_context("readonly-intent"),
                &intent,
                request_digest,
                &timestamp("2026-07-10T10:05:00Z")?,
            )
            .await?;
        store.checkpoint_and_close().await?;
        let readonly = SqliteStore::connect_read_only(&path).await?;
        let integrity = readonly
            .settlement_recovery_integrity(&intent.intent_id)
            .await?
            .ok_or("missing integrity")?;
        assert_eq!(
            (
                integrity.intent_count,
                integrity.receipt_count,
                integrity.reconciliation_count
            ),
            (1, 0, 0)
        );
        assert_eq!(
            integrity.recovery.action,
            SettlementRecoveryAction::QueryExternalStatus
        );
        assert!(
            sqlx::query("DELETE FROM settlement_intents")
                .execute(&readonly.pool)
                .await
                .is_err()
        );
        readonly.close().await;
        assert!(!path.with_extension("sqlite-wal").exists());
        assert!(!path.with_extension("sqlite-shm").exists());
        Ok(())
    }

    #[tokio::test]
    async fn authorization_denial_is_scoped_sanitized_idempotent_and_durable()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("denials.sqlite");
        let store = SqliteStore::connect_path(&path, 4).await?;
        store.migrate().await?;
        let record = denial("request-denial-1")?;
        let missing_scope = MutationContext {
            actor_id: "public-reader".to_owned(),
            request_id: record.request_id.clone(),
            scope: MutationScope::new([StateSlice::AccessControlV1]),
        };
        assert!(matches!(
            store
                .append_authorization_denial(&missing_scope, &record)
                .await,
            Err(PersistenceError::UndeclaredMutation(_))
        ));
        let extra_scope = MutationContext {
            actor_id: "public-reader".to_owned(),
            request_id: record.request_id.clone(),
            scope: MutationScope::new([
                StateSlice::AccessControlV1,
                StateSlice::OperationsV1,
                StateSlice::DeliveryV1,
            ]),
        };
        assert!(matches!(
            store
                .append_authorization_denial(&extra_scope, &record)
                .await,
            Err(PersistenceError::AuthorizationDenialConflict(_))
        ));
        assert_eq!(
            store
                .append_authorization_denial(&denial_context(&record.request_id), &record)
                .await?,
            AppendOutcome::Inserted
        );
        assert_eq!(
            store
                .append_authorization_denial(&denial_context(&record.request_id), &record)
                .await?,
            AppendOutcome::ExistingIdentical
        );
        assert_eq!(store.authorization_denial_count().await?, 1);
        let integrity = store
            .authorization_denial_integrity(&record.denial_id)
            .await?
            .ok_or("missing denial")?;
        assert_eq!(integrity.record, record);
        assert_eq!(integrity.row_count, 1);
        let payload = serde_json::to_string(&integrity.record)?;
        for forbidden in ["x-hsai-role", "credential", "password", "127.0.0.1"] {
            assert!(!payload.contains(forbidden));
        }
        let update = sqlx::query("UPDATE audit_events SET actor_id = 'changed' WHERE audit_id = ?")
            .bind(&record.denial_id)
            .execute(&store.pool)
            .await;
        assert!(update.is_err());
        store.checkpoint_and_close().await?;
        let reopened = SqliteStore::connect_read_only(&path).await?;
        assert_eq!(reopened.authorization_denial_count().await?, 1);
        assert_eq!(
            reopened
                .authorization_denial_integrity(&record.denial_id)
                .await?
                .ok_or("missing reopened denial")?
                .record,
            record
        );
        reopened.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn authorization_denial_conflict_and_concurrency_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let record = denial("request-denial-concurrent")?;
        let mut conflicting = record.clone();
        conflicting.reason = AuthorizationDenialReasonV1::RoleMismatch;
        conflicting.record_digest = AuthorizationDenialRecordV1::new(
            conflicting.request_id.clone(),
            conflicting.actor_id.clone(),
            conflicting.actor_class,
            conflicting.action,
            conflicting.reason,
            conflicting.denied_at.clone(),
        )?
        .record_digest;
        let left_store = store.clone();
        let left_record = record.clone();
        let left = tokio::spawn(async move {
            left_store
                .append_authorization_denial(&denial_context(&left_record.request_id), &left_record)
                .await
        });
        let right_store = store.clone();
        let right_record = record.clone();
        let right = tokio::spawn(async move {
            right_store
                .append_authorization_denial(
                    &denial_context(&right_record.request_id),
                    &right_record,
                )
                .await
        });
        let outcomes = [left.await??, right.await??];
        assert!(outcomes.contains(&AppendOutcome::Inserted));
        assert!(outcomes.contains(&AppendOutcome::ExistingIdentical));
        assert!(matches!(
            store
                .append_authorization_denial(&denial_context(&conflicting.request_id), &conflicting)
                .await,
            Err(PersistenceError::AuthorizationDenialConflict(_))
        ));
        assert_eq!(store.authorization_denial_count().await?, 1);
        Ok(())
    }

    #[tokio::test]
    async fn contract_dispute_supersession_is_append_only_idempotent_and_restart_safe()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database_path = directory.path().join("dispute-recovery.db");
        let store = SqliteStore::connect_path(&database_path, 1).await?;
        store.migrate().await?;
        let (claim, predecessor, evidence) = seed_dispute_dependencies(&store).await?;
        let (open, decision, successor, supersession) =
            dispute_chain(&claim, &predecessor, evidence)?;
        assert_eq!(
            store
                .append_open_dispute(&dispute_context("open", false), &open)
                .await?,
            AppendOutcome::Inserted
        );
        assert_eq!(
            store
                .append_accepted_dispute_supersession(
                    &dispute_context("decide", true),
                    &decision,
                    &successor,
                    &supersession,
                )
                .await?,
            AppendOutcome::Inserted
        );
        assert!(
            sqlx::query("UPDATE disputes SET status = 'REJECTED'")
                .execute(&store.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM dispute_decisions")
                .execute(&store.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE resolution_supersessions SET payload_json = '{}'")
                .execute(&store.pool)
                .await
                .is_err()
        );
        drop(store);

        let restarted = SqliteStore::connect_path(&database_path, 1).await?;
        restarted.migrate().await?;
        let recovered = restarted
            .dispute_recovery_state(&open.dispute_id)
            .await?
            .ok_or_else(|| std::io::Error::other("dispute chain missing after restart"))?;
        assert_eq!(recovered.open_dispute, open);
        assert_eq!(recovered.decision, Some(decision.clone()));
        assert_eq!(recovered.supersession, Some(supersession.clone()));
        assert_eq!(recovered.successor_resolution, Some(successor.clone()));
        assert_eq!(
            restarted
                .append_accepted_dispute_supersession(
                    &dispute_context("decide", true),
                    &decision,
                    &successor,
                    &supersession,
                )
                .await?,
            AppendOutcome::ExistingIdentical
        );
        for table in ["disputes", "dispute_decisions", "resolution_supersessions"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&restarted.pool)
                .await?;
            assert_eq!(count, 1, "unexpected rows in {table}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn contract_dispute_supersession_scope_and_conflicts_fail_without_partial_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let (claim, predecessor, evidence) = seed_dispute_dependencies(&store).await?;
        let (open, decision, successor, supersession) =
            dispute_chain(&claim, &predecessor, evidence)?;
        store
            .append_open_dispute(&dispute_context("open-scope", false), &open)
            .await?;
        let incomplete = dispute_context("missing-resolution-slice", false);
        assert!(matches!(
            store
                .append_accepted_dispute_supersession(
                    &incomplete,
                    &decision,
                    &successor,
                    &supersession,
                )
                .await,
            Err(PersistenceError::UndeclaredMutation(_))
        ));
        for table in ["dispute_decisions", "resolution_supersessions"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&store.pool)
                .await?;
            assert_eq!(count, 0);
        }

        let mut conflicting = decision.clone();
        conflicting.rationale = "different semantics".to_owned();
        conflicting.record_digest = digest("conflicting-decision")?;
        assert!(matches!(
            store
                .append_accepted_dispute_supersession(
                    &dispute_context("conflict", true),
                    &conflicting,
                    &successor,
                    &supersession,
                )
                .await,
            Err(PersistenceError::DisputeResolutionIdentityConflict { .. })
        ));
        let successor_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resolution_records WHERE resolution_digest = ?",
        )
        .bind(successor.record_digest.to_string())
        .fetch_one(&store.pool)
        .await?;
        assert_eq!(successor_count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn contract_dispute_supersession_audit_failure_rolls_back_atomically()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = SqliteStore::in_memory().await?;
        store.migrate().await?;
        let (claim, predecessor, evidence) = seed_dispute_dependencies(&store).await?;
        let (open, decision, successor, supersession) =
            dispute_chain(&claim, &predecessor, evidence)?;
        store
            .append_open_dispute(&dispute_context("open-rollback", false), &open)
            .await?;
        sqlx::raw_sql(
            "CREATE TRIGGER injected_dispute_audit_failure BEFORE INSERT ON audit_events \
             WHEN NEW.event_type = 'DISPUTE_ACCEPTED_SUPERSEDED' \
             BEGIN SELECT RAISE(ABORT, 'injected dispute audit failure'); END;",
        )
        .execute(&store.pool)
        .await?;
        assert!(matches!(
            store
                .append_accepted_dispute_supersession(
                    &dispute_context("decide-rollback", true),
                    &decision,
                    &successor,
                    &supersession,
                )
                .await,
            Err(PersistenceError::Sqlx(_))
        ));
        for table in ["dispute_decisions", "resolution_supersessions"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&store.pool)
                .await?;
            assert_eq!(count, 0, "transaction leaked a row in {table}");
        }
        let successor_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resolution_records WHERE resolution_digest = ?",
        )
        .bind(successor.record_digest.to_string())
        .fetch_one(&store.pool)
        .await?;
        let idempotency_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM idempotency_keys WHERE scope = 'append_dispute_supersession'",
        )
        .fetch_one(&store.pool)
        .await?;
        assert_eq!((successor_count, idempotency_count), (0, 0));
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
            sqlx::raw_sql(include_str!(
                "../../../migrations/postgres/0003_dispute_resolution_history.sql"
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
                "dispute_decisions",
                "resolution_supersessions",
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
        assert_eq!(required_table_count, 8);
        Ok(())
    }
}
