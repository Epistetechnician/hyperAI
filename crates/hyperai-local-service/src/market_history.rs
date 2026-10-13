//! Market-owned persistence for the local synthetic deal-history contract.

use std::{path::Path, str::FromStr, time::Duration};

use hyperai_foundation::Sha256Digest;
use hyperai_market::{LocalMarketHistoryEntryV1, LocalMarketQuoteApprovalV1, LocalMarketReceiptV1};
use serde_json::to_vec;
use sqlx::{
    Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqliteJournalMode,
    sqlite::SqlitePoolOptions, sqlite::SqliteSynchronous,
};
use thiserror::Error;

use crate::storage::{self, HomePaths, StorageError};

const DATABASE_SCHEMA_VERSION: i64 = 1;
const MAX_RECORD_BYTES: usize = 1024 * 1024;

/// Errors from opening or using the separate market-history database.
#[derive(Debug, Error)]
pub(crate) enum MarketHistoryError {
    /// Local-home path protection or preparation failed.
    #[error("market history path preparation failed: {0}")]
    Storage(#[from] StorageError),
    /// `SQLite` operation failed.
    #[error("market history database operation failed: {0}")]
    Sqlite(#[from] sqlx::Error),
    /// Stored or submitted record JSON is invalid.
    #[error("market history JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// The market-history database has a newer unsupported schema.
    #[error("market history database schema is newer than supported")]
    NewerSchema,
    /// The market-history database has an unknown or malformed schema/record.
    #[error("market history database is malformed")]
    Corrupt,
    /// A deal identity was reused with conflicting record bytes.
    #[error("market history record conflicts with an existing deal")]
    Conflict,
    /// No exact stored buyer approval exists for the requested completion.
    #[error("market history has no matching stored approval")]
    ApprovalMissing,
    /// A serialized history record exceeded the local size bound.
    #[error("market history record exceeds 1 MiB")]
    TooLarge,
}

/// One-writer local `SQLite` store for synthetic market approvals and receipts.
#[derive(Clone)]
pub(crate) struct MarketHistoryStore {
    pool: SqlitePool,
}

impl MarketHistoryStore {
    /// Open or create the Market-owned database at the selected local home.
    pub(crate) async fn open(paths: &HomePaths) -> Result<Self, MarketHistoryError> {
        storage::prepare_market_history_database(paths)?;
        let pool = connect(&paths.market_database).await?;
        migrate(&pool).await?;
        restrict_database_file(&paths.market_database)?;
        Ok(Self { pool })
    }

    /// Persist an approval idempotently, rejecting conflicting deal reuse.
    pub(crate) async fn record_approval(
        &self,
        approval: &LocalMarketQuoteApprovalV1,
    ) -> Result<(), MarketHistoryError> {
        let approval_bytes = bounded_json(approval)?;
        let approval_json =
            String::from_utf8(approval_bytes).map_err(|_| MarketHistoryError::Corrupt)?;
        let result = sqlx::query(
            "INSERT OR IGNORE INTO market_deal_history (deal_digest, preview_digest, approval_json, receipt_json) VALUES (?1, ?2, ?3, NULL)",
        )
        .bind(approval.deal_digest.to_string())
        .bind(approval.preview_digest.to_string())
        .bind(&approval_json)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 1 {
            return Ok(());
        }
        let Some((stored_preview, stored_approval, _)) =
            self.read_record_by_deal(approval.deal_digest).await?
        else {
            return Err(MarketHistoryError::Corrupt);
        };
        if stored_preview == approval.preview_digest && stored_approval == approval_json {
            Ok(())
        } else {
            Err(MarketHistoryError::Conflict)
        }
    }

    /// Persist a mock receipt only for the exact stored approval.
    pub(crate) async fn record_completion(
        &self,
        approval: &LocalMarketQuoteApprovalV1,
        receipt: &LocalMarketReceiptV1,
    ) -> Result<(), MarketHistoryError> {
        let receipt_bytes = bounded_json(receipt)?;
        let receipt_json =
            String::from_utf8(receipt_bytes).map_err(|_| MarketHistoryError::Corrupt)?;
        let Some((stored_preview, stored_approval, stored_receipt)) =
            self.read_record_by_deal(approval.deal_digest).await?
        else {
            return Err(MarketHistoryError::ApprovalMissing);
        };
        let approval_json =
            String::from_utf8(bounded_json(approval)?).map_err(|_| MarketHistoryError::Corrupt)?;
        if stored_preview != approval.preview_digest || stored_approval != approval_json {
            return Err(MarketHistoryError::ApprovalMissing);
        }
        match stored_receipt {
            Some(existing) if existing == receipt_json => return Ok(()),
            Some(_) => return Err(MarketHistoryError::Conflict),
            None => {}
        }
        let result = sqlx::query(
            "UPDATE market_deal_history SET receipt_json = ?1 WHERE deal_digest = ?2 AND preview_digest = ?3 AND receipt_json IS NULL",
        )
        .bind(&receipt_json)
        .bind(approval.deal_digest.to_string())
        .bind(approval.preview_digest.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 1 {
            return Ok(());
        }
        let Some((_, _, Some(existing))) = self.read_record_by_deal(approval.deal_digest).await?
        else {
            return Err(MarketHistoryError::ApprovalMissing);
        };
        if existing == receipt_json {
            Ok(())
        } else {
            Err(MarketHistoryError::Conflict)
        }
    }

    /// Return one exact stored deal, if its preview digest also matches.
    pub(crate) async fn find_deal(
        &self,
        preview_digest: Sha256Digest,
        deal_digest: Sha256Digest,
    ) -> Result<Option<LocalMarketHistoryEntryV1>, MarketHistoryError> {
        let Some((stored_preview, approval, receipt, sequence)) =
            self.read_record_by_deal_with_sequence(deal_digest).await?
        else {
            return Ok(None);
        };
        if stored_preview != preview_digest {
            return Ok(None);
        }
        Ok(Some(LocalMarketHistoryEntryV1 {
            sequence,
            approval,
            completion_receipt: receipt,
        }))
    }

    /// Read a bounded newest-first page. The bool indicates an additional row.
    pub(crate) async fn read_page(
        &self,
        before_sequence: Option<u64>,
        limit: usize,
    ) -> Result<(Vec<LocalMarketHistoryEntryV1>, bool), MarketHistoryError> {
        if !(1..=50).contains(&limit) {
            return Err(MarketHistoryError::Corrupt);
        }
        let fetch_limit = i64::try_from(limit + 1).map_err(|_| MarketHistoryError::Corrupt)?;
        let rows = if let Some(before) = before_sequence {
            let before = i64::try_from(before).map_err(|_| MarketHistoryError::Corrupt)?;
            sqlx::query(
                "SELECT sequence, deal_digest, preview_digest, approval_json, receipt_json FROM market_deal_history WHERE sequence < ?1 ORDER BY sequence DESC LIMIT ?2",
            )
            .bind(before)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT sequence, deal_digest, preview_digest, approval_json, receipt_json FROM market_deal_history ORDER BY sequence DESC LIMIT ?1",
            )
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await?
        };
        let mut entries = Vec::with_capacity(rows.len().min(limit));
        for row in rows.iter().take(limit) {
            entries.push(decode_row(row)?);
        }
        Ok((entries, rows.len() > limit))
    }

    async fn read_record_by_deal(
        &self,
        deal_digest: Sha256Digest,
    ) -> Result<Option<(Sha256Digest, String, Option<String>)>, MarketHistoryError> {
        let row = sqlx::query(
            "SELECT sequence, deal_digest, preview_digest, approval_json, receipt_json FROM market_deal_history WHERE deal_digest = ?1",
        )
        .bind(deal_digest.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let entry = decode_row(&row)?;
            let preview = entry.approval.preview_digest;
            let approval_json = bounded_string(row.try_get("approval_json")?)?;
            let receipt_json = row
                .try_get::<Option<String>, _>("receipt_json")?
                .map(bounded_string)
                .transpose()?;
            Ok((preview, approval_json, receipt_json))
        })
        .transpose()
    }

    async fn read_record_by_deal_with_sequence(
        &self,
        deal_digest: Sha256Digest,
    ) -> Result<
        Option<(
            Sha256Digest,
            LocalMarketQuoteApprovalV1,
            Option<LocalMarketReceiptV1>,
            u64,
        )>,
        MarketHistoryError,
    > {
        let row = sqlx::query(
            "SELECT sequence, deal_digest, preview_digest, approval_json, receipt_json FROM market_deal_history WHERE deal_digest = ?1",
        )
        .bind(deal_digest.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let entry = decode_row(&row)?;
            Ok((
                entry.approval.preview_digest,
                entry.approval,
                entry.completion_receipt,
                entry.sequence,
            ))
        })
        .transpose()
    }
}

fn decode_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<LocalMarketHistoryEntryV1, MarketHistoryError> {
    let sequence: i64 = row.try_get("sequence")?;
    let stored_deal: String = row.try_get("deal_digest")?;
    let stored_preview: String = row.try_get("preview_digest")?;
    let approval_json = bounded_string(row.try_get("approval_json")?)?;
    let receipt_json = row
        .try_get::<Option<String>, _>("receipt_json")?
        .map(bounded_string)
        .transpose()?;
    let approval: LocalMarketQuoteApprovalV1 = serde_json::from_str(&approval_json)?;
    let receipt: Option<LocalMarketReceiptV1> = receipt_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?;
    if sequence <= 0
        || Sha256Digest::from_str(&stored_deal).map_err(|_| MarketHistoryError::Corrupt)?
            != approval.deal_digest
        || Sha256Digest::from_str(&stored_preview).map_err(|_| MarketHistoryError::Corrupt)?
            != approval.preview_digest
    {
        return Err(MarketHistoryError::Corrupt);
    }
    Ok(LocalMarketHistoryEntryV1 {
        sequence: u64::try_from(sequence).map_err(|_| MarketHistoryError::Corrupt)?,
        approval,
        completion_receipt: receipt,
    })
}

fn bounded_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, MarketHistoryError> {
    let bytes = to_vec(value)?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(MarketHistoryError::TooLarge);
    }
    Ok(bytes)
}

fn bounded_string(value: String) -> Result<String, MarketHistoryError> {
    if value.len() > MAX_RECORD_BYTES {
        return Err(MarketHistoryError::TooLarge);
    }
    Ok(value)
}

async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5));
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
}

async fn migrate(pool: &SqlitePool) -> Result<(), MarketHistoryError> {
    let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;
    if schema_version > DATABASE_SCHEMA_VERSION {
        return Err(MarketHistoryError::NewerSchema);
    }
    if schema_version == 0 {
        let table_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_one(pool)
        .await?;
        if table_count != 0 {
            return Err(MarketHistoryError::Corrupt);
        }
        let mut transaction = pool.begin().await?;
        sqlx::query(
            "CREATE TABLE market_deal_history (sequence INTEGER PRIMARY KEY AUTOINCREMENT, deal_digest TEXT NOT NULL UNIQUE, preview_digest TEXT NOT NULL, approval_json TEXT NOT NULL, receipt_json TEXT NULL)",
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query("PRAGMA user_version = 1")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
    } else {
        let columns = sqlx::query("PRAGMA table_info(market_deal_history)")
            .fetch_all(pool)
            .await?;
        let names: Vec<String> = columns
            .iter()
            .map(|row| row.try_get("name"))
            .collect::<Result<_, _>>()?;
        let expected = [
            "sequence",
            "deal_digest",
            "preview_digest",
            "approval_json",
            "receipt_json",
        ];
        if names.len() != expected.len()
            || names
                .iter()
                .zip(expected)
                .any(|(actual, expected)| actual != expected)
        {
            return Err(MarketHistoryError::Corrupt);
        }
    }
    Ok(())
}

fn restrict_database_file(path: &Path) -> Result<(), StorageError> {
    // Reuse the path protection helper after SQLite has opened the file.
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(StorageError::UnsafePath(path.to_path_buf()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    storage::restrict_windows_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use hyperai_application::{
        LocalMarketQuoteApprovalRequestV1, LocalMarketQuoteCompletionRequestV1,
        approve_local_market_quote_v1, complete_local_market_quote_v1,
        preview_local_market_quote_v1,
    };
    use hyperai_foundation::Sha256Digest;
    use hyperai_market::{
        LocalMarketHistoryPageV1, local_market_history_page_v1, verify_local_market_history_page_v1,
    };

    use super::{MarketHistoryError, MarketHistoryStore};
    use crate::storage::{self, StorageError};

    #[tokio::test]
    async fn synthetic_history_is_idempotent_and_survives_reopen()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let temporary = tempfile::tempdir()?;
        let paths = storage::paths_for_home(Some(temporary.path()))?;
        let preview = preview_local_market_quote_v1()?;
        let approval = approve_local_market_quote_v1(&LocalMarketQuoteApprovalRequestV1 {
            preview_digest: preview.digest,
        })?;
        let completion = complete_local_market_quote_v1(&LocalMarketQuoteCompletionRequestV1 {
            preview_digest: preview.digest,
            deal_digest: approval.deal_digest,
        })?;

        let store = MarketHistoryStore::open(&paths).await?;
        assert!(matches!(
            store
                .record_completion(&approval, &completion.demo.receipt)
                .await,
            Err(MarketHistoryError::ApprovalMissing)
        ));
        store.record_approval(&approval).await?;
        store.record_approval(&approval).await?;
        store
            .record_completion(&approval, &completion.demo.receipt)
            .await?;
        store
            .record_completion(&approval, &completion.demo.receipt)
            .await?;

        let conflicting_approval = hyperai_market::LocalMarketQuoteApprovalV1 {
            effects: "conflicting synthetic retry".to_owned(),
            ..approval.clone()
        };
        assert!(matches!(
            store.record_approval(&conflicting_approval).await,
            Err(MarketHistoryError::Conflict)
        ));
        let conflicting_receipt = hyperai_market::LocalMarketReceiptV1 {
            digest: Sha256Digest::from_bytes([0x42; 32]),
            ..completion.demo.receipt.clone()
        };
        assert!(matches!(
            store
                .record_completion(&approval, &conflicting_receipt)
                .await,
            Err(MarketHistoryError::Conflict)
        ));
        drop(store);

        let reopened = MarketHistoryStore::open(&paths).await?;
        let (entries, has_more) = reopened.read_page(None, 20).await?;
        assert!(!has_more);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].approval, approval);
        assert_eq!(entries[0].completion_receipt, Some(completion.demo.receipt));
        Ok(())
    }

    #[tokio::test]
    async fn unsupported_market_database_schema_fails_closed()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let temporary = tempfile::tempdir()?;
        let paths = storage::paths_for_home(Some(temporary.path()))?;
        let store = MarketHistoryStore::open(&paths).await?;
        sqlx::query("PRAGMA user_version = 2")
            .execute(&store.pool)
            .await?;
        drop(store);
        assert!(matches!(
            MarketHistoryStore::open(&paths).await,
            Err(MarketHistoryError::NewerSchema)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn malformed_persisted_record_fails_closed()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let temporary = tempfile::tempdir()?;
        let paths = storage::paths_for_home(Some(temporary.path()))?;
        let store = MarketHistoryStore::open(&paths).await?;
        let preview = preview_local_market_quote_v1()?;
        let approval = approve_local_market_quote_v1(&LocalMarketQuoteApprovalRequestV1 {
            preview_digest: preview.digest,
        })?;
        store.record_approval(&approval).await?;
        sqlx::query("UPDATE market_deal_history SET approval_json = '{}' ")
            .execute(&store.pool)
            .await?;
        assert!(matches!(
            store.read_page(None, 20).await,
            Err(MarketHistoryError::Json(_))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn page_cursor_order_and_digest_are_stable()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let temporary = tempfile::tempdir()?;
        let paths = storage::paths_for_home(Some(temporary.path()))?;
        let store = MarketHistoryStore::open(&paths).await?;
        let preview = preview_local_market_quote_v1()?;
        let approval = approve_local_market_quote_v1(&LocalMarketQuoteApprovalRequestV1 {
            preview_digest: preview.digest,
        })?;
        let second_approval = hyperai_market::LocalMarketQuoteApprovalV1 {
            deal_digest: Sha256Digest::from_bytes([0x31; 32]),
            ..approval.clone()
        };
        store.record_approval(&approval).await?;
        store.record_approval(&second_approval).await?;

        let (entries, has_more) = store.read_page(None, 1).await?;
        assert!(has_more);
        let first_page: LocalMarketHistoryPageV1 =
            local_market_history_page_v1(entries, Some("mh1_test".to_owned()))?;
        verify_local_market_history_page_v1(&first_page)?;
        let (older_entries, has_more) = store
            .read_page(Some(first_page.body.entries[0].sequence), 1)
            .await?;
        assert!(!has_more);
        assert_eq!(older_entries.len(), 1);
        assert_eq!(older_entries[0].approval, approval);
        let second_page = local_market_history_page_v1(older_entries, None)?;
        verify_local_market_history_page_v1(&second_page)?;
        assert_ne!(first_page.digest, second_page.digest);
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn market_history_paths_are_owner_only_and_reject_symlinks()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::os::unix::{fs::PermissionsExt, fs::symlink};

        let temporary = tempfile::tempdir()?;
        let paths = storage::paths_for_home(Some(temporary.path()))?;
        let store = MarketHistoryStore::open(&paths).await?;
        drop(store);
        assert_eq!(
            std::fs::metadata(&paths.market_directory)?
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&paths.market_database)?
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        let other_file = temporary.path().join("outside.sqlite3");
        std::fs::write(&other_file, b"not market history")?;
        std::fs::remove_file(&paths.market_database)?;
        symlink(other_file, &paths.market_database)?;
        assert!(matches!(
            MarketHistoryStore::open(&paths).await,
            Err(MarketHistoryError::Storage(StorageError::UnsafePath(_)))
        ));
        Ok(())
    }
}
