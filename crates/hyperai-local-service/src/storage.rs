//! Versioned, per-user storage for the `HyperAI` local service.

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use directories::BaseDirs;
use fs2::FileExt;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Connection, Row, SqliteConnection};
use thiserror::Error;

use crate::lifecycle::{LifecycleError, ServiceLock};
use hyperai_foundation::Sha256Digest;

const CONFIG_SCHEMA: &str = "hyperai.local-config.v1";
const DATABASE_SCHEMA_VERSION: i64 = 1;
const MANUAL_BACKUP_SCHEMA: &str = "hyperai.local-storage-backup.v1";
const RESTORE_REPORT_SCHEMA: &str = "hyperai.local-storage-restore.v1";
static BACKUP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Resolved paths owned by one local `HyperAI` installation.
#[derive(Clone, Debug)]
pub struct HomePaths {
    /// The user's `HyperAI` home directory.
    pub root: PathBuf,
    /// Versioned local configuration file.
    pub config: PathBuf,
    /// `SQLite` metadata database.
    pub database: PathBuf,
    /// Runtime lock, service record, and session capability directory.
    pub runtime: PathBuf,
    /// Local diagnostic log directory.
    pub logs: PathBuf,
    /// Local cache directory.
    pub cache: PathBuf,
    /// Metadata database backup directory.
    pub backups: PathBuf,
    /// Market-owned local synthetic record directory.
    pub market_directory: PathBuf,
    /// Market-owned local synthetic history database.
    pub market_database: PathBuf,
}

/// Result of initializing or checking a local `HyperAI` home.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HomeReportV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Canonical local home path.
    pub home: String,
    /// Whether the config and metadata database are initialized.
    pub initialized: bool,
    /// Local service mode.
    pub effect_boundary: &'static str,
}

/// Receipt for one consistent local metadata snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StorageBackupReportV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Completed operation status.
    pub status: &'static str,
    /// Safe basename under the current home's `backups/` directory.
    pub backup_name: String,
    /// Database schema version included in the snapshot.
    pub database_schema_version: i64,
    /// Size of the completed snapshot in bytes.
    pub file_bytes: u64,
    /// Full-file lowercase SHA-256 digest.
    pub backup_sha256: String,
}

/// Receipt for a successful local metadata restore.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StorageRestoreReportV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Completed operation status.
    pub status: &'static str,
    /// Safe basename of the selected backup.
    pub selected_backup_name: String,
    /// Full-file SHA-256 digest of the selected backup.
    pub selected_backup_sha256: String,
    /// Safe basename of the automatic pre-restore snapshot.
    pub safety_backup_name: String,
    /// Full-file SHA-256 digest of the safety snapshot.
    pub safety_backup_sha256: String,
    /// Number of metadata rows restored.
    pub metadata_row_count: u64,
}

/// Storage resolution, permissions, configuration, or migration failure.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The requested path is not safe to use.
    #[error("unsafe HyperAI home path: {0}")]
    UnsafePath(PathBuf),
    /// The platform does not have a configured home mapping.
    #[error("cannot resolve the platform's per-user HyperAI home")]
    HomeUnavailable,
    /// Existing config cannot be interpreted as a supported version.
    #[error("local configuration is invalid or unsupported: {0}")]
    InvalidConfig(String),
    /// An unknown newer database schema must not be downgraded.
    #[error("metadata database schema {found} is newer than supported schema {supported}")]
    NewerDatabase {
        /// Schema version stored in the existing database.
        found: i64,
        /// Newest schema version supported by this binary.
        supported: i64,
    },
    /// Another process currently owns first-time home initialization.
    #[error("another process is initializing the HyperAI home")]
    InitializationBusy,
    /// The home is owned by the running service or another exclusive operation.
    #[error("the HyperAI home is busy")]
    HomeBusy,
    /// A storage command requires an initialized metadata home.
    #[error("the HyperAI home is not initialized")]
    HomeNotInitialized,
    /// A restore name is not a generated basename accepted by this command.
    #[error("invalid local metadata backup name")]
    InvalidBackupName,
    /// The selected backup is absent from the current home's backup directory.
    #[error("local metadata backup was not found")]
    BackupNotFound,
    /// The current database or backup does not use supported schema version 1.
    #[error("local metadata schema is unsupported")]
    UnsupportedSchema,
    /// The selected snapshot is corrupt or has an unsupported database layout.
    #[error("local metadata backup is invalid")]
    InvalidBackup,
    /// Restore failed after creating a recovery snapshot.
    #[error("local metadata restore failed")]
    RestoreFailed {
        /// Safe basename of the pre-restore snapshot.
        safety_backup_name: String,
        /// Whether the final transaction outcome is uncertain.
        outcome_unknown: bool,
    },
    /// The filesystem operation failed.
    #[error("local storage operation failed: {0}")]
    Io(#[from] io::Error),
    /// The `SQLite` operation failed.
    #[error("local metadata database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    /// Configuration serialization failed.
    #[error("local configuration serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalConfigV1 {
    schema_version: String,
    created_at_unix_seconds: u64,
}

struct SnapshotArtifact {
    name: String,
    file_bytes: u64,
    sha256: String,
}

#[derive(Clone, Copy)]
enum SnapshotKind {
    Manual,
    BeforeRestore,
}

impl SnapshotKind {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Manual => "desk-manual",
            Self::BeforeRestore => "desk-before-restore",
        }
    }
}

/// Resolves a CLI override, environment override, or supported OS default.
///
/// # Errors
///
/// Returns an error for an unavailable platform home, parent traversal,
/// filesystem inspection failure, or a symlink at the selected home path.
pub fn resolve_home(explicit: Option<&Path>) -> Result<PathBuf, StorageError> {
    let selected = explicit
        .map(Path::to_path_buf)
        .or_else(|| env::var_os("HYPERAI_HOME").map(PathBuf::from))
        .map_or_else(default_home, Ok)?;
    let absolute = if selected.is_absolute() {
        selected
    } else {
        env::current_dir()?.join(selected)
    };
    if absolute
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(StorageError::UnsafePath(absolute));
    }
    if absolute.parent().is_none() || absolute == Path::new("/") {
        return Err(StorageError::UnsafePath(absolute));
    }
    match fs::symlink_metadata(&absolute) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(StorageError::UnsafePath(absolute));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    Ok(absolute)
}

/// Resolves paths and creates the versioned local metadata home without
/// overwriting existing configuration.
///
/// # Errors
///
/// Returns an error for unsafe paths, inaccessible storage, invalid
/// configuration, an unsupported newer schema, or a failed migration.
pub async fn initialize_home(
    explicit: Option<&Path>,
) -> Result<(HomePaths, HomeReportV1), StorageError> {
    let paths = prepare_service_runtime(explicit)?;
    let _initialization_lock = acquire_initialization_lock(&paths)?;
    let root = &paths.root;
    ensure_directory(root)?;
    reject_symlink(root)?;
    for directory in [
        paths
            .database
            .parent()
            .ok_or_else(|| StorageError::UnsafePath(paths.database.clone()))?,
        paths.runtime.as_path(),
        paths.logs.as_path(),
        paths.cache.as_path(),
        paths.backups.as_path(),
    ] {
        ensure_directory(directory)?;
        reject_symlink(directory)?;
    }

    ensure_config(&paths.config)?;
    ensure_database_file(&paths.database)?;
    migrate_database(&paths).await?;

    let home = paths.root.to_string_lossy().into_owned();
    let report = HomeReportV1 {
        schema_version: "hyperai.local-home-report.v1",
        home,
        initialized: true,
        effect_boundary: "LOCAL_METADATA_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    };
    Ok((paths, report))
}

/// Creates only the protected root and runtime directory needed for service locking.
///
/// # Errors
///
/// Returns an error if the selected home is unsafe or its directories cannot
/// be created with restrictive permissions.
pub fn prepare_service_runtime(explicit: Option<&Path>) -> Result<HomePaths, StorageError> {
    let paths = paths_for_home(explicit)?;
    ensure_directory(&paths.root)?;
    reject_symlink(&paths.root)?;
    ensure_directory(&paths.runtime)?;
    reject_symlink(&paths.runtime)?;
    Ok(paths)
}

/// Resolves all known local-home paths without creating or changing them.
///
/// # Errors
///
/// Returns an error if the home cannot be resolved safely.
pub fn paths_for_home(explicit: Option<&Path>) -> Result<HomePaths, StorageError> {
    let root = resolve_home(explicit)?;
    Ok(HomePaths {
        config: root.join("config.json"),
        database: root.join("metadata").join("desk.sqlite3"),
        runtime: root.join("runtime"),
        logs: root.join("logs"),
        cache: root.join("cache"),
        backups: root.join("backups"),
        market_directory: root.join("market"),
        market_database: root.join("market").join("deal-history.sqlite3"),
        root,
    })
}

/// Creates or validates the separate owner-only market-history database path.
///
/// This storage boundary is intentionally separate from metadata migrations
/// and backups. The market module owns its `SQLite` schema.
///
/// # Errors
///
/// Returns an error if the market directory or database path is unsafe or
/// cannot be created with the local protection policy.
pub fn prepare_market_history_database(paths: &HomePaths) -> Result<(), StorageError> {
    ensure_directory(&paths.market_directory)?;
    reject_symlink(&paths.market_directory)?;
    ensure_database_file(&paths.market_database)?;
    Ok(())
}

/// Checks whether a home already contains a supported `HyperAI` config.
///
/// # Errors
///
/// Returns an error if the home path or existing config is unsafe or invalid.
pub fn inspect_home(explicit: Option<&Path>) -> Result<HomeReportV1, StorageError> {
    let root = resolve_home(explicit)?;
    let config = root.join("config.json");
    let initialized = config.exists();
    if initialized {
        let _ = read_config(&config)?;
    }
    Ok(HomeReportV1 {
        schema_version: "hyperai.local-home-report.v1",
        home: root.to_string_lossy().into_owned(),
        initialized,
        effect_boundary: "READ_ONLY_LOCAL_INSPECTION_NO_NETWORK",
    })
}

/// Opens the metadata database with one writer and a bounded lock wait.
///
/// # Errors
///
/// Returns an error if the metadata database cannot be opened with the
/// configured local `SQLite` policy.
pub async fn connect_metadata(paths: &HomePaths) -> Result<sqlx::SqlitePool, StorageError> {
    let options = SqliteConnectOptions::new()
        .filename(&paths.database)
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    Ok(pool)
}

/// Creates a consistent backup of the selected home's schema-v1 metadata.
///
/// The local service may remain running; the database engine provides the snapshot
/// boundary. No user input, market payload, or nested-product data is copied.
///
/// # Errors
///
/// Returns an error for an uninitialized/unsafe home, a busy initialization
/// lock, an unsupported database schema, or a failed backup operation.
pub async fn backup_metadata(
    explicit: Option<&Path>,
) -> Result<StorageBackupReportV1, StorageError> {
    let paths = paths_for_home(explicit)?;
    require_initialized_home(&paths)?;
    let _initialization_lock = acquire_initialization_lock(&paths).map_err(map_home_busy)?;
    let pool = connect_metadata(&paths).await?;
    {
        let mut connection = pool.acquire().await?;
        ensure_supported_metadata(&mut connection).await?;
    }
    let snapshot = create_snapshot(&pool, &paths, SnapshotKind::Manual).await?;
    pool.close().await;
    Ok(StorageBackupReportV1 {
        schema_version: MANUAL_BACKUP_SCHEMA,
        status: "created",
        backup_name: snapshot.name,
        database_schema_version: DATABASE_SCHEMA_VERSION,
        file_bytes: snapshot.file_bytes,
        backup_sha256: snapshot.sha256,
    })
}

/// Restores schema-v1 metadata from a generated backup in the same home.
///
/// The service lock prevents a concurrent service start. Restore replaces
/// only `desk_meta` rows and keeps an automatic pre-restore snapshot.
///
/// # Errors
///
/// Returns an error for an uninitialized/unsafe/busy home, an invalid or
/// unsupported backup, or a failed transactional restore.
pub async fn restore_metadata(
    explicit: Option<&Path>,
    backup_name: &str,
) -> Result<StorageRestoreReportV1, StorageError> {
    if !is_generated_restore_name(backup_name) {
        return Err(StorageError::InvalidBackupName);
    }

    let paths = paths_for_home(explicit)?;
    require_initialized_home(&paths)?;
    let _service_lock = acquire_restore_service_lock(&paths)?;
    let _initialization_lock = acquire_initialization_lock(&paths).map_err(map_home_busy)?;

    let backup_path = paths.backups.join(backup_name);
    require_backup_file(&backup_path)?;
    let backup_pool = connect_read_only_database(&backup_path)
        .await
        .map_err(|_| StorageError::InvalidBackup)?;
    let selected_backup_sha256 = hash_file(&backup_path)?;
    {
        let mut connection = backup_pool.acquire().await?;
        validate_restore_source(&mut connection)
            .await
            .map_err(map_invalid_backup)?;
    }

    let destination = connect_metadata(&paths).await?;
    {
        let mut connection = destination.acquire().await?;
        ensure_supported_metadata(&mut connection).await?;
    }
    let safety = create_snapshot(&destination, &paths, SnapshotKind::BeforeRestore).await?;
    let metadata_row_count = match restore_metadata_rows(
        &backup_pool,
        &destination,
        &safety.name,
        &backup_path,
        &selected_backup_sha256,
    )
    .await
    {
        Ok(count) => count,
        Err(error) => {
            destination.close().await;
            backup_pool.close().await;
            return Err(error);
        }
    };

    destination.close().await;
    backup_pool.close().await;
    Ok(StorageRestoreReportV1 {
        schema_version: RESTORE_REPORT_SCHEMA,
        status: "restored",
        selected_backup_name: backup_name.to_owned(),
        selected_backup_sha256,
        safety_backup_name: safety.name,
        safety_backup_sha256: safety.sha256,
        metadata_row_count,
    })
}

fn map_home_busy(error: StorageError) -> StorageError {
    match error {
        StorageError::InitializationBusy => StorageError::HomeBusy,
        other => other,
    }
}

fn map_invalid_backup(error: StorageError) -> StorageError {
    match error {
        supported @ (StorageError::UnsupportedSchema | StorageError::InvalidBackup) => supported,
        _ => StorageError::InvalidBackup,
    }
}

fn acquire_restore_service_lock(paths: &HomePaths) -> Result<ServiceLock, StorageError> {
    match ServiceLock::acquire(paths) {
        Ok(lock) => Ok(lock),
        Err(LifecycleError::AlreadyRunning) => Err(StorageError::HomeBusy),
        Err(error) => Err(StorageError::Io(io::Error::other(error))),
    }
}

fn require_initialized_home(paths: &HomePaths) -> Result<(), StorageError> {
    require_directory(&paths.root)?;
    require_directory(
        paths
            .database
            .parent()
            .ok_or_else(|| StorageError::UnsafePath(paths.database.clone()))?,
    )?;
    require_directory(&paths.runtime)?;
    require_directory(&paths.backups)?;
    require_regular_file(&paths.config)?;
    require_regular_file(&paths.database)?;
    let config = read_config(&paths.config)?;
    if config.schema_version != CONFIG_SCHEMA {
        return Err(StorageError::InvalidConfig(config.schema_version));
    }
    Ok(())
}

fn require_directory(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(StorageError::UnsafePath(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(StorageError::HomeNotInitialized)
        }
        Err(error) => Err(error.into()),
    }
}

fn require_regular_file(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(StorageError::UnsafePath(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(StorageError::HomeNotInitialized)
        }
        Err(error) => Err(error.into()),
    }
}

fn is_generated_restore_name(name: &str) -> bool {
    let Some(rest) = name
        .strip_prefix("desk-manual-")
        .or_else(|| name.strip_prefix("desk-before-restore-"))
    else {
        return false;
    };
    let Some(stem) = rest.strip_suffix(".sqlite3") else {
        return false;
    };
    let mut fields = stem.split('-');
    let numeric = |field: Option<&str>| {
        field.is_some_and(|value| {
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
        })
    };
    numeric(fields.next())
        && numeric(fields.next())
        && numeric(fields.next())
        && fields.next().is_none()
}

fn require_backup_file(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(StorageError::InvalidBackup)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(StorageError::BackupNotFound),
        Err(error) => Err(error.into()),
    }
}

async fn connect_read_only_database(path: &Path) -> Result<sqlx::SqlitePool, StorageError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .read_only(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    Ok(pool)
}

async fn ensure_supported_metadata(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    validate_metadata_database(connection, false).await
}

async fn validate_restore_source(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    validate_metadata_database(connection, true).await
}

async fn validate_metadata_database(
    connection: &mut SqliteConnection,
    is_backup: bool,
) -> Result<(), StorageError> {
    let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *connection)
        .await?;
    if schema_version != DATABASE_SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchema);
    }

    let objects = sqlx::query(
        "SELECT type, name, sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type, name",
    )
    .fetch_all(&mut *connection)
    .await?;
    let expected_sql = "createtabledesk_meta(keytextprimarykey,valuetextnotnull)";
    let has_expected_table = objects.len() == 1
        && objects[0].try_get::<String, _>("type").ok().as_deref() == Some("table")
        && objects[0].try_get::<String, _>("name").ok().as_deref() == Some("desk_meta")
        && objects[0]
            .try_get::<Option<String>, _>("sql")
            .ok()
            .flatten()
            .is_some_and(|sql| normalize_ddl(&sql) == expected_sql);
    if !has_expected_table {
        return Err(if is_backup {
            StorageError::InvalidBackup
        } else {
            StorageError::UnsupportedSchema
        });
    }

    let integrity_rows = sqlx::query("PRAGMA integrity_check")
        .fetch_all(&mut *connection)
        .await?;
    let is_integral = integrity_rows.len() == 1
        && integrity_rows[0]
            .try_get::<String, _>(0)
            .is_ok_and(|result| result == "ok");
    if !is_integral {
        return Err(if is_backup {
            StorageError::InvalidBackup
        } else {
            StorageError::UnsupportedSchema
        });
    }

    let invalid_values: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM desk_meta WHERE typeof(key) != 'text' OR typeof(value) != 'text'",
    )
    .fetch_one(&mut *connection)
    .await?;
    if invalid_values != 0 {
        return Err(if is_backup {
            StorageError::InvalidBackup
        } else {
            StorageError::UnsupportedSchema
        });
    }
    Ok(())
}

fn normalize_ddl(sql: &str) -> String {
    sql.chars()
        .filter(|character| !character.is_whitespace() && *character != ';')
        .flat_map(char::to_lowercase)
        .collect()
}

async fn create_snapshot(
    pool: &sqlx::SqlitePool,
    paths: &HomePaths,
    kind: SnapshotKind,
) -> Result<SnapshotArtifact, StorageError> {
    ensure_directory(&paths.backups)?;
    reject_symlink(&paths.backups)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for _ in 0..8 {
        let sequence = BACKUP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "{}-{timestamp}-{}-{sequence}.sqlite3",
            kind.prefix(),
            std::process::id()
        );
        let path = paths.backups.join(&name);
        match fs::symlink_metadata(&path) {
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let target = path
            .to_str()
            .ok_or_else(|| StorageError::UnsafePath(path.clone()))?;
        sqlx::query("VACUUM INTO ?1")
            .bind(target)
            .execute(pool)
            .await?;
        require_regular_file(&path)?;
        restrict_file(&path)?;
        File::open(&path)?.sync_all()?;
        sync_parent(&path)?;
        let file_bytes = fs::metadata(&path)?.len();
        let sha256 = hash_file(&path)?;
        return Ok(SnapshotArtifact {
            name,
            file_bytes,
            sha256,
        });
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique local backup name",
    )
    .into())
}

fn hash_file(path: &Path) -> Result<String, StorageError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest: [u8; 32] = hasher.finalize().into();
    Ok(Sha256Digest::from_bytes(digest).to_hex())
}

async fn restore_metadata_rows(
    backup_pool: &sqlx::SqlitePool,
    destination_pool: &sqlx::SqlitePool,
    safety_backup_name: &str,
    backup_path: &Path,
    expected_backup_digest: &str,
) -> Result<u64, StorageError> {
    let mut source = backup_pool
        .acquire()
        .await
        .map_err(|_| restore_failed(safety_backup_name, false))?;
    let mut source_transaction = source
        .begin()
        .await
        .map_err(|_| restore_failed(safety_backup_name, false))?;
    validate_restore_source(&mut source_transaction)
        .await
        .map_err(|_| restore_failed(safety_backup_name, false))?;

    let mut destination = destination_pool
        .acquire()
        .await
        .map_err(|_| restore_failed(safety_backup_name, false))?;
    let mut transaction = destination
        .begin()
        .await
        .map_err(|_| restore_failed(safety_backup_name, false))?;
    if sqlx::query("DELETE FROM desk_meta")
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return Err(restore_failed(
            safety_backup_name,
            transaction.rollback().await.is_err(),
        ));
    }

    let Ok(row_count) = copy_metadata_rows(&mut source_transaction, &mut transaction).await else {
        return Err(restore_failed(
            safety_backup_name,
            transaction.rollback().await.is_err(),
        ));
    };

    if source_transaction.commit().await.is_err() {
        return Err(restore_failed(
            safety_backup_name,
            transaction.rollback().await.is_err(),
        ));
    }
    let Ok(current_backup_digest) = hash_file(backup_path) else {
        return Err(restore_failed(
            safety_backup_name,
            transaction.rollback().await.is_err(),
        ));
    };
    if current_backup_digest != expected_backup_digest {
        return Err(restore_failed(
            safety_backup_name,
            transaction.rollback().await.is_err(),
        ));
    }
    transaction
        .commit()
        .await
        .map_err(|_| restore_failed(safety_backup_name, true))?;
    Ok(row_count)
}

async fn copy_metadata_rows(
    source: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    destination: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<u64, sqlx::Error> {
    let mut row_count = 0_u64;
    let mut rows =
        sqlx::query("SELECT key, value FROM desk_meta ORDER BY key").fetch(&mut **source);
    while let Some(row) = rows.try_next().await? {
        let key: String = row.try_get(0)?;
        let value: String = row.try_get(1)?;
        sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
            .bind(key)
            .bind(value)
            .execute(&mut **destination)
            .await?;
        row_count = row_count.saturating_add(1);
    }
    Ok(row_count)
}

fn restore_failed(safety_backup_name: &str, outcome_unknown: bool) -> StorageError {
    StorageError::RestoreFailed {
        safety_backup_name: safety_backup_name.to_owned(),
        outcome_unknown,
    }
}

fn default_home() -> Result<PathBuf, StorageError> {
    let base = BaseDirs::new().ok_or(StorageError::HomeUnavailable)?;
    #[cfg(target_os = "macos")]
    {
        return Ok(base
            .home_dir()
            .join("Library")
            .join("Application Support")
            .join("HyperAI"));
    }
    #[cfg(target_os = "windows")]
    {
        return Ok(base.data_local_dir().join("HyperAI"));
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(base.data_dir().join("hyperai"));
    }
    #[allow(unreachable_code)]
    Err(StorageError::HomeUnavailable)
}

fn ensure_directory(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(StorageError::UnsafePath(path.to_path_buf()));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(StorageError::UnsafePath(path.to_path_buf()));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(path)?;
        }
        Err(error) => return Err(error.into()),
    }
    restrict_directory(path)?;
    Ok(())
}

fn acquire_initialization_lock(paths: &HomePaths) -> Result<File, StorageError> {
    let path = paths.runtime.join("initialize.lock");
    reject_symlink(&path)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path)?;
    restrict_file(&path)?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(file),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
            Err(StorageError::InitializationBusy)
        }
        Err(error) => Err(error.into()),
    }
}

fn reject_symlink(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(StorageError::UnsafePath(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn ensure_config(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(StorageError::UnsafePath(path.to_path_buf()));
        }
        Ok(_) => {
            let config = read_config(path)?;
            if config.schema_version != CONFIG_SCHEMA {
                return Err(StorageError::InvalidConfig(config.schema_version));
            }
            restrict_file(path)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let created_at_unix_seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let config = LocalConfigV1 {
                schema_version: CONFIG_SCHEMA.to_owned(),
                created_at_unix_seconds,
            };
            let bytes = serde_json::to_vec_pretty(&config)?;
            let mut file = secure_open_new(path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            sync_parent(path)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn read_config(path: &Path) -> Result<LocalConfigV1, StorageError> {
    let bytes = fs::read(path)?;
    let config: LocalConfigV1 = serde_json::from_slice(&bytes)?;
    if config.schema_version != CONFIG_SCHEMA {
        return Err(StorageError::InvalidConfig(config.schema_version));
    }
    Ok(config)
}

fn ensure_database_file(path: &Path) -> Result<(), StorageError> {
    if path.exists() {
        reject_symlink(path)?;
        restrict_file(path)?;
        return Ok(());
    }
    let file = secure_open_new(path)?;
    file.sync_all()?;
    sync_parent(path)?;
    Ok(())
}

async fn migrate_database(paths: &HomePaths) -> Result<(), StorageError> {
    let options = SqliteConnectOptions::new()
        .filename(&paths.database)
        .create_if_missing(false)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    if schema_version > DATABASE_SCHEMA_VERSION {
        return Err(StorageError::NewerDatabase {
            found: schema_version,
            supported: DATABASE_SCHEMA_VERSION,
        });
    }
    if schema_version == DATABASE_SCHEMA_VERSION {
        pool.close().await;
        let operational_pool = connect_metadata(paths).await?;
        operational_pool.close().await;
        return Ok(());
    }
    let metadata = fs::metadata(&paths.database)?;
    if metadata.len() > 0 {
        backup_database(&pool, paths).await?;
    }
    sqlx::query("CREATE TABLE IF NOT EXISTS desk_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA user_version = 1")
        .execute(&pool)
        .await?;
    pool.close().await;
    restrict_file(&paths.database)?;
    let operational_pool = connect_metadata(paths).await?;
    operational_pool.close().await;
    Ok(())
}

async fn backup_database(pool: &sqlx::SqlitePool, paths: &HomePaths) -> Result<(), StorageError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let backup = paths.backups.join(format!(
        "desk-before-migration-{timestamp}-{}.sqlite3",
        std::process::id()
    ));
    let backup_text = backup.to_string_lossy().into_owned();
    sqlx::query("VACUUM INTO ?1")
        .bind(backup_text)
        .execute(pool)
        .await?;
    restrict_file(&backup)?;
    sync_parent(&backup)?;
    Ok(())
}

fn secure_open_new(path: &Path) -> Result<File, StorageError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    restrict_file(path)?;
    Ok(file)
}

fn restrict_directory(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(windows)]
    {
        apply_windows_acl(path, true)?;
    }
    Ok(())
}

fn restrict_file(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    {
        apply_windows_acl(path, false)?;
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn restrict_windows_file(path: &Path) -> Result<(), StorageError> {
    apply_windows_acl(path, false)
}

#[cfg(windows)]
fn apply_windows_acl(path: &Path, directory: bool) -> Result<(), StorageError> {
    use std::process::Command;

    let identity = Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()?;
    if !identity.status.success() {
        return Err(io::Error::other("could not resolve current Windows user SID").into());
    }
    let identity_text = String::from_utf8_lossy(&identity.stdout);
    let sid = identity_text
        .split(',')
        .find_map(|part| {
            let candidate = part.trim().trim_matches('"');
            (candidate.starts_with("S-1-")
                && candidate
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'S' || byte == b'-'))
            .then_some(candidate)
        })
        .ok_or_else(|| io::Error::other("could not parse current Windows user SID"))?;

    let owner_grant = if directory {
        format!("*{sid}:(OI)(CI)F")
    } else {
        format!("*{sid}:F")
    };
    let system_grant = if directory {
        "*S-1-5-18:(OI)(CI)F"
    } else {
        "*S-1-5-18:F"
    };
    let administrators_grant = if directory {
        "*S-1-5-32-544:(OI)(CI)F"
    } else {
        "*S-1-5-32-544:F"
    };
    let result = Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(owner_grant)
        .arg(system_grant)
        .arg(administrators_grant)
        .output()?;
    if !result.status.success() {
        return Err(io::Error::other("could not restrict HyperAI home ACL").into());
    }
    Ok(())
}

fn sync_parent(path: &Path) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        let file = File::open(parent)?;
        file.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod restore_failure_tests {
    use super::*;

    #[tokio::test]
    async fn failed_row_copy_rolls_back_and_reports_the_safety_backup()
    -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let home = temporary.path().join("hyperai-home");
        let (paths, _) = initialize_home(Some(&home))
            .await
            .map_err(|error| std::io::Error::other(format!("initialize_home: {error:?}")))?;
        let pool = connect_metadata(&paths).await?;
        sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
            .bind("restore-failure")
            .bind("before-backup")
            .execute(&pool)
            .await?;
        pool.close().await;
        let backup = backup_metadata(Some(&home))
            .await
            .map_err(|error| std::io::Error::other(format!("backup_metadata: {error:?}")))?;

        let pool = connect_metadata(&paths).await?;
        sqlx::query("UPDATE desk_meta SET value = ?1 WHERE key = ?2")
            .bind("current-value")
            .bind("restore-failure")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
            .bind("current-only")
            .bind("preserve-on-failure")
            .execute(&pool)
            .await?;
        sqlx::query(
            r"CREATE TRIGGER reject_restore BEFORE INSERT ON desk_meta
               WHEN NEW.key = 'restore-failure'
               BEGIN SELECT RAISE(ABORT, 'forced test failure'); END",
        )
        .execute(&pool)
        .await?;
        pool.close().await;

        let destination_pool = connect_metadata(&paths).await?;
        let safety =
            create_snapshot(&destination_pool, &paths, SnapshotKind::BeforeRestore).await?;
        let backup_path = paths.backups.join(&backup.backup_name);
        let backup_pool = connect_read_only_database(&backup_path).await?;
        let result = restore_metadata_rows(
            &backup_pool,
            &destination_pool,
            &safety.name,
            &backup_path,
            &backup.backup_sha256,
        )
        .await;
        let Err(error) = result else {
            return Err("forced restore failure unexpectedly succeeded".into());
        };
        assert!(matches!(
            error,
            StorageError::RestoreFailed {
                ref safety_backup_name,
                outcome_unknown: false
            } if safety_backup_name == &safety.name
        ));
        destination_pool.close().await;
        backup_pool.close().await;

        let pool = connect_metadata(&paths).await?;
        let restored_value: String =
            sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = ?1")
                .bind("restore-failure")
                .fetch_one(&pool)
                .await?;
        let current_only_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta WHERE key = ?1")
                .bind("current-only")
                .fetch_one(&pool)
                .await?;
        assert_eq!(restored_value, "current-value");
        assert_eq!(current_only_count, 1);
        pool.close().await;
        Ok(())
    }
}
