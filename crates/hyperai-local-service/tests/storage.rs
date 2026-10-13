//! Contract checks for isolated local-home storage.

use std::{error::Error, fs};

use hyperai_local_service::lifecycle::ServiceLock;
use hyperai_local_service::storage::{
    StorageError, backup_metadata, connect_metadata, initialize_home, resolve_home,
    restore_metadata,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::test]
async fn initialize_is_idempotent_and_uses_private_unix_modes() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (_, first) = initialize_home(Some(&home)).await?;
    let config_before = fs::read(home.join("config.json"))?;
    let (_, second) = initialize_home(Some(&home)).await?;
    assert_eq!(first, second);
    assert_eq!(config_before, fs::read(home.join("config.json"))?);

    let pool = connect_metadata(&hyperai_local_service::storage::paths_for_home(Some(
        &home,
    ))?)
    .await?;
    let schema: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(schema, 1);
    pool.close().await;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&home)?.permissions().mode() & 0o777, 0o700);
        assert_eq!(
            fs::metadata(home.join("config.json"))?.permissions().mode() & 0o777,
            0o600
        );
    }
    Ok(())
}

#[test]
fn home_resolution_rejects_parent_traversal_and_root_symlinks() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let traversal = temporary.path().join("child").join("..").join("other");
    assert!(matches!(
        resolve_home(Some(&traversal)),
        Err(StorageError::UnsafePath(_))
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = temporary.path().join("outside");
        fs::create_dir(&outside)?;
        let link = temporary.path().join("linked-home");
        symlink(&outside, &link)?;
        assert!(matches!(
            resolve_home(Some(&link)),
            Err(StorageError::UnsafePath(_))
        ));
    }
    Ok(())
}

#[tokio::test]
async fn newer_database_schema_is_refused_without_downgrade() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let pool = connect_metadata(&paths).await?;
    sqlx::query("PRAGMA user_version = 77")
        .execute(&pool)
        .await?;
    pool.close().await;

    assert!(matches!(
        initialize_home(Some(&home)).await,
        Err(StorageError::NewerDatabase {
            found: 77,
            supported: 1
        })
    ));

    let options = SqliteConnectOptions::new()
        .filename(&paths.database)
        .create_if_missing(false);
    let check_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let schema: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&check_pool)
        .await?;
    assert_eq!(schema, 77);
    check_pool.close().await;
    Ok(())
}

#[tokio::test]
async fn migration_keeps_a_consistent_backup_of_existing_metadata() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let pool = connect_metadata(&paths).await?;
    sqlx::query("CREATE TABLE legacy_marker (value TEXT NOT NULL)")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO legacy_marker (value) VALUES ('preserved')")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA user_version = 0")
        .execute(&pool)
        .await?;
    pool.close().await;

    initialize_home(Some(&home)).await?;
    let backup = fs::read_dir(&paths.backups)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "sqlite3")
        })
        .ok_or("migration did not create a SQLite backup")?;
    let options = SqliteConnectOptions::new()
        .filename(backup)
        .create_if_missing(false);
    let backup_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let value: String = sqlx::query_scalar("SELECT value FROM legacy_marker")
        .fetch_one(&backup_pool)
        .await?;
    assert_eq!(value, "preserved");
    backup_pool.close().await;
    Ok(())
}

#[tokio::test]
async fn sqlite_full_rejects_a_write_without_partial_metadata() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let pool = connect_metadata(&paths).await?;

    let page_count: i64 = sqlx::query_scalar("PRAGMA page_count")
        .fetch_one(&pool)
        .await?;
    let configured_max: i64 = sqlx::query_scalar(&format!("PRAGMA max_page_count = {page_count}"))
        .fetch_one(&pool)
        .await?;
    assert_eq!(configured_max, page_count);

    let oversized_value = "x".repeat(128 * 1024);
    let write = sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("disk-full-probe")
        .bind(oversized_value)
        .execute(&pool)
        .await;
    let error = match write {
        Err(sqlx::Error::Database(error)) => error,
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("SQLite accepted a write after reaching max_page_count".into()),
    };
    assert_eq!(error.code().as_deref(), Some("13"));

    let partial_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta WHERE key = 'disk-full-probe'")
            .fetch_one(&pool)
            .await?;
    let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(partial_rows, 0);
    assert_eq!(schema_version, 1);
    pool.close().await;
    Ok(())
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires HYPERAI_PHYSICAL_FULL_TEST_ROOT on a size-limited disposable tmpfs"]
async fn physical_tmpfs_exhaustion_rejects_metadata_write_without_partial_metadata()
-> Result<(), Box<dyn Error>> {
    use std::{env, fs::OpenOptions, io::Write, path::PathBuf};

    let root = env::var_os("HYPERAI_PHYSICAL_FULL_TEST_ROOT")
        .map(PathBuf::from)
        .ok_or("HYPERAI_PHYSICAL_FULL_TEST_ROOT is required")?;
    let root = fs::canonicalize(root)?;
    let mounts = fs::read_to_string("/proc/mounts")?;
    let mount = mounts
        .lines()
        .find_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (fields.get(1).copied() == Some(root.to_string_lossy().as_ref())).then_some(fields)
        })
        .ok_or("test root is not an exact mount point")?;
    assert_eq!(
        mount.get(2).copied(),
        Some("tmpfs"),
        "test mount must be tmpfs"
    );
    let mount_size = mount
        .get(3)
        .and_then(|options| {
            options
                .split(',')
                .find_map(|option| option.strip_prefix("size="))
        })
        .ok_or("tmpfs size option is missing")?;
    let digit_end = mount_size
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(mount_size.len());
    let size_value: u64 = mount_size[..digit_end].parse()?;
    let size_multiplier = match mount_size[digit_end..].to_ascii_lowercase().as_str() {
        "" | "b" => 1_u64,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024 * 1024,
        "g" | "gb" => 1024 * 1024 * 1024,
        unit => return Err(format!("unsupported tmpfs size unit: {unit}").into()),
    };
    let size_limit = size_value
        .checked_mul(size_multiplier)
        .ok_or("tmpfs size overflow")?;
    assert!(
        (8 * 1024 * 1024..=64 * 1024 * 1024).contains(&size_limit),
        "test tmpfs must be between 8 and 64 MiB, got {size_limit} bytes"
    );

    let temporary = tempfile::Builder::new()
        .prefix("hyperai-enospc-")
        .tempdir_in(&root)?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let pool = connect_metadata(&paths).await?;
    sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("preserved-before-enospc")
        .bind("sentinel")
        .execute(&pool)
        .await?;

    let filler_path = temporary.path().join("filesystem-filler.bin");
    let mut filler = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&filler_path)?;
    let block = [0x5a_u8; 64 * 1024];
    let mut filler_bytes = 0_u64;
    loop {
        match filler.write(&block) {
            Ok(0) => return Err("tmpfs filler stopped before ENOSPC".into()),
            Ok(written) => {
                filler_bytes = filler_bytes
                    .checked_add(written as u64)
                    .ok_or("tmpfs filler byte count overflow")?;
                assert!(
                    filler_bytes <= size_limit,
                    "filler exceeded the declared tmpfs size"
                );
            }
            Err(error) if error.raw_os_error() == Some(28) => break,
            Err(error) => return Err(error.into()),
        }
    }
    let oversized_value = "x".repeat(128 * 1024);
    let write = sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("physical-enospc-probe")
        .bind(oversized_value)
        .execute(&pool)
        .await;
    let error = match write {
        Err(sqlx::Error::Database(error)) => error,
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("SQLite accepted a metadata write after tmpfs ENOSPC".into()),
    };
    assert_eq!(error.code().as_deref(), Some("13"), "{error}");

    let partial_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta WHERE key = 'physical-enospc-probe'")
            .fetch_one(&pool)
            .await?;
    let sentinel: String =
        sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = 'preserved-before-enospc'")
            .fetch_one(&pool)
            .await?;
    let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(partial_rows, 0);
    assert_eq!(sentinel, "sentinel");
    assert_eq!(schema_version, 1);
    drop(filler);
    fs::remove_file(&filler_path)?;

    sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("after-enospc-recovery")
        .bind("write-succeeded")
        .execute(&pool)
        .await?;
    let recovered_value: String =
        sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = 'after-enospc-recovery'")
            .fetch_one(&pool)
            .await?;
    let failed_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta WHERE key = 'physical-enospc-probe'")
            .fetch_one(&pool)
            .await?;
    let sentinel_after_recovery: String =
        sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = 'preserved-before-enospc'")
            .fetch_one(&pool)
            .await?;
    let recovered_schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(recovered_value, "write-succeeded");
    assert_eq!(failed_rows, 0);
    assert_eq!(sentinel_after_recovery, "sentinel");
    assert_eq!(recovered_schema_version, 1);
    pool.close().await;

    println!(
        "tmpfs_limit_bytes={size_limit} filler_bytes={filler_bytes} sqlite_code=13 recovery_write=ok"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires HYPERAI_PHYSICAL_FULL_TEST_ROOT on a size-limited disposable tmpfs"]
async fn physical_tmpfs_full_restore_rolls_back_and_retains_safety_snapshot()
-> Result<(), Box<dyn Error>> {
    use std::{env, fs::OpenOptions, io::Write, path::PathBuf};

    const RESTORE_RESERVE_BYTES: u64 = 2 * 1024 * 1024;
    const SYNTHETIC_ROW_COUNT: u64 = 256;
    const SYNTHETIC_VALUE_BYTES: usize = 32 * 1024;

    let root = env::var_os("HYPERAI_PHYSICAL_FULL_TEST_ROOT")
        .map(PathBuf::from)
        .ok_or("HYPERAI_PHYSICAL_FULL_TEST_ROOT is required")?;
    let root = fs::canonicalize(root)?;
    let mounts = fs::read_to_string("/proc/mounts")?;
    let mount = mounts
        .lines()
        .find_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (fields.get(1).copied() == Some(root.to_string_lossy().as_ref())).then_some(fields)
        })
        .ok_or("test root is not an exact mount point")?;
    assert_eq!(mount.get(2).copied(), Some("tmpfs"));
    let mount_size = mount
        .get(3)
        .and_then(|options| {
            options
                .split(',')
                .find_map(|option| option.strip_prefix("size="))
        })
        .ok_or("tmpfs size option is missing")?;
    let digit_end = mount_size
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(mount_size.len());
    let size_value: u64 = mount_size[..digit_end].parse()?;
    let size_multiplier = match mount_size[digit_end..].to_ascii_lowercase().as_str() {
        "" | "b" => 1_u64,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024 * 1024,
        "g" | "gb" => 1024 * 1024 * 1024,
        unit => return Err(format!("unsupported tmpfs size unit: {unit}").into()),
    };
    let size_limit = size_value
        .checked_mul(size_multiplier)
        .ok_or("tmpfs size overflow")?;
    assert_eq!(size_limit, 32 * 1024 * 1024);

    let temporary = tempfile::Builder::new()
        .prefix("hyperai-enospc-restore-")
        .tempdir_in(&root)?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let large_value = "r".repeat(SYNTHETIC_VALUE_BYTES);
    let pool = connect_metadata(&paths).await?;
    let mut seed = pool.begin().await?;
    for index in 0..SYNTHETIC_ROW_COUNT {
        sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
            .bind(format!("restore-source-{index:04}"))
            .bind(large_value.as_str())
            .execute(&mut *seed)
            .await?;
    }
    seed.commit().await?;
    pool.close().await;

    let backup = backup_metadata(Some(&home)).await?;
    assert_eq!(backup.status, "created");
    assert!(backup.file_bytes > RESTORE_RESERVE_BYTES * 4);
    let backup_path = paths.backups.join(&backup.backup_name);
    let backup_bytes = fs::read(&backup_path)?;

    let pool = connect_metadata(&paths).await?;
    sqlx::query("DELETE FROM desk_meta").execute(&pool).await?;
    sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("setting")
        .bind("pre-restore")
        .execute(&pool)
        .await?;
    sqlx::query("VACUUM").execute(&pool).await?;
    pool.close().await;

    let filler_path = temporary.path().join("restore-filler.bin");
    let mut filler = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&filler_path)?;
    let block = [0x6b_u8; 64 * 1024];
    let mut filler_bytes = 0_u64;
    loop {
        match filler.write(&block) {
            Ok(0) => return Err("tmpfs filler stopped before ENOSPC".into()),
            Ok(written) => {
                filler_bytes = filler_bytes
                    .checked_add(written as u64)
                    .ok_or("tmpfs filler byte count overflow")?;
                assert!(filler_bytes <= size_limit);
            }
            Err(error) if error.raw_os_error() == Some(28) => break,
            Err(error) => return Err(error.into()),
        }
    }
    let filler_length = filler_bytes
        .checked_sub(RESTORE_RESERVE_BYTES)
        .ok_or("tmpfs had less space than the restore reserve")?;
    filler.set_len(filler_length)?;
    filler.sync_all()?;
    assert_eq!(fs::metadata(&filler_path)?.len(), filler_length);

    let (restore_error, outcome_unknown) = match restore_metadata(Some(&home), &backup.backup_name)
        .await
    {
        Err(StorageError::RestoreFailed {
            safety_backup_name,
            outcome_unknown,
        }) => (safety_backup_name, outcome_unknown),
        Err(error) => {
            return Err(format!("restore failed outside the rollback path: {error:?}").into());
        }
        Ok(_) => return Err("restore succeeded with less space than its source snapshot".into()),
    };
    assert!(restore_error.starts_with("desk-before-restore-"));
    let safety_path = paths.backups.join(&restore_error);
    assert!(fs::metadata(&safety_path)?.len() > 0);
    drop(filler);
    fs::remove_file(&filler_path)?;

    assert_eq!(fs::read(&backup_path)?, backup_bytes);
    let pool = connect_metadata(&paths).await?;
    let current_value: String =
        sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = 'setting'")
            .fetch_one(&pool)
            .await?;
    let current_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta")
        .fetch_one(&pool)
        .await?;
    let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(current_value, "pre-restore");
    assert_eq!(current_rows, 1);
    assert_eq!(schema_version, 1);
    pool.close().await;

    let safety_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&safety_path)
                .create_if_missing(false)
                .read_only(true),
        )
        .await?;
    let safety_value: String =
        sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = 'setting'")
            .fetch_one(&safety_pool)
            .await?;
    let safety_schema: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&safety_pool)
        .await?;
    assert_eq!(safety_value, "pre-restore");
    assert_eq!(safety_schema, 1);
    safety_pool.close().await;

    println!(
        "tmpfs_limit_bytes={size_limit} filler_bytes={filler_bytes} reserve_bytes={RESTORE_RESERVE_BYTES} backup_bytes={} outcome_unknown={outcome_unknown} live_rows={current_rows} live_value={current_value} safety_snapshot=ok",
        backup.file_bytes
    );

    Ok(())
}

#[tokio::test]
async fn manual_backup_restore_round_trip_keeps_a_safety_snapshot() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let pool = connect_metadata(&paths).await?;
    sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("setting")
        .bind("before-backup")
        .execute(&pool)
        .await?;
    pool.close().await;

    let backup = backup_metadata(Some(&home)).await?;
    assert_eq!(backup.schema_version, "hyperai.local-storage-backup.v1");
    assert_eq!(backup.status, "created");
    assert_eq!(backup.database_schema_version, 1);
    assert_eq!(backup.backup_sha256.len(), 64);
    let backup_path = paths.backups.join(&backup.backup_name);
    assert_eq!(fs::metadata(&backup_path)?.len(), backup.file_bytes);
    let original_backup_bytes = fs::read(&backup_path)?;
    let second_backup = backup_metadata(Some(&home)).await?;
    assert_ne!(second_backup.backup_name, backup.backup_name);
    assert_eq!(fs::read(&backup_path)?, original_backup_bytes);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&backup_path)?.permissions().mode() & 0o777,
            0o600
        );
    }

    let pool = connect_metadata(&paths).await?;
    sqlx::query("UPDATE desk_meta SET value = ?1 WHERE key = ?2")
        .bind("changed-after-backup")
        .bind("setting")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("later")
        .bind("kept-in-safety-snapshot")
        .execute(&pool)
        .await?;
    pool.close().await;

    let restored = restore_metadata(Some(&home), &backup.backup_name).await?;
    assert_eq!(restored.schema_version, "hyperai.local-storage-restore.v1");
    assert_eq!(restored.status, "restored");
    assert_eq!(restored.selected_backup_name, backup.backup_name);
    assert_eq!(restored.selected_backup_sha256, backup.backup_sha256);
    assert_eq!(restored.metadata_row_count, 1);
    assert_eq!(restored.safety_backup_sha256.len(), 64);
    assert!(paths.backups.join(&restored.safety_backup_name).is_file());

    let pool = connect_metadata(&paths).await?;
    let restored_value: String = sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = ?1")
        .bind("setting")
        .fetch_one(&pool)
        .await?;
    let later_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM desk_meta WHERE key = ?1")
        .bind("later")
        .fetch_one(&pool)
        .await?;
    assert_eq!(restored_value, "before-backup");
    assert_eq!(later_count, 0);
    pool.close().await;

    let safety_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(paths.backups.join(&restored.safety_backup_name))
                .create_if_missing(false),
        )
        .await?;
    let safety_value: String = sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = ?1")
        .bind("setting")
        .fetch_one(&safety_pool)
        .await?;
    assert_eq!(safety_value, "changed-after-backup");
    safety_pool.close().await;
    Ok(())
}

#[tokio::test]
async fn manual_backup_works_while_service_owns_home_but_restore_is_refused()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let _service_lock = ServiceLock::acquire(&paths)?;

    let backup = backup_metadata(Some(&home)).await?;
    assert!(paths.backups.join(&backup.backup_name).is_file());
    assert!(matches!(
        restore_metadata(Some(&home), &backup.backup_name).await,
        Err(StorageError::HomeBusy)
    ));
    Ok(())
}

#[tokio::test]
async fn restore_rejects_invalid_names_and_unsupported_schema_without_mutation()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let (paths, _) = initialize_home(Some(&home)).await?;
    let pool = connect_metadata(&paths).await?;
    sqlx::query("INSERT INTO desk_meta (key, value) VALUES (?1, ?2)")
        .bind("setting")
        .bind("preserved")
        .execute(&pool)
        .await?;
    pool.close().await;

    assert!(matches!(
        restore_metadata(Some(&home), "../outside.sqlite3").await,
        Err(StorageError::InvalidBackupName)
    ));

    let invalid_path = paths.backups.join("desk-manual-1-1-1.sqlite3");
    let invalid_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&invalid_path)
                .create_if_missing(true),
        )
        .await?;
    sqlx::query("CREATE TABLE desk_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&invalid_pool)
        .await?;
    sqlx::query("PRAGMA user_version = 77")
        .execute(&invalid_pool)
        .await?;
    invalid_pool.close().await;

    assert!(matches!(
        restore_metadata(Some(&home), "desk-manual-1-1-1.sqlite3").await,
        Err(StorageError::UnsupportedSchema)
    ));

    let old_schema_path = paths.backups.join("desk-manual-2-2-2.sqlite3");
    let old_schema_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&old_schema_path)
                .create_if_missing(true),
        )
        .await?;
    sqlx::query("CREATE TABLE desk_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&old_schema_pool)
        .await?;
    sqlx::query("PRAGMA user_version = 0")
        .execute(&old_schema_pool)
        .await?;
    old_schema_pool.close().await;
    assert!(matches!(
        restore_metadata(Some(&home), "desk-manual-2-2-2.sqlite3").await,
        Err(StorageError::UnsupportedSchema)
    ));

    let wrong_shape_path = paths.backups.join("desk-manual-3-3-3.sqlite3");
    let wrong_shape_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&wrong_shape_path)
                .create_if_missing(true),
        )
        .await?;
    sqlx::query("CREATE TABLE another_table (value TEXT NOT NULL)")
        .execute(&wrong_shape_pool)
        .await?;
    sqlx::query("PRAGMA user_version = 1")
        .execute(&wrong_shape_pool)
        .await?;
    wrong_shape_pool.close().await;
    assert!(matches!(
        restore_metadata(Some(&home), "desk-manual-3-3-3.sqlite3").await,
        Err(StorageError::InvalidBackup)
    ));

    let corrupt_path = paths.backups.join("desk-manual-4-4-4.sqlite3");
    fs::write(&corrupt_path, b"not a SQLite database")?;
    let corrupt_result = restore_metadata(Some(&home), "desk-manual-4-4-4.sqlite3").await;
    assert!(matches!(corrupt_result, Err(StorageError::InvalidBackup)));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let symlink_path = paths.backups.join("desk-manual-5-5-5.sqlite3");
        symlink(&invalid_path, &symlink_path)?;
        assert!(matches!(
            restore_metadata(Some(&home), "desk-manual-5-5-5.sqlite3").await,
            Err(StorageError::InvalidBackup)
        ));
    }

    let pool = connect_metadata(&paths).await?;
    let value: String = sqlx::query_scalar("SELECT value FROM desk_meta WHERE key = ?1")
        .bind("setting")
        .fetch_one(&pool)
        .await?;
    assert_eq!(value, "preserved");
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn migration_snapshots_are_not_accepted_as_manual_restore_inputs()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let _ = initialize_home(Some(&home)).await?;
    assert!(matches!(
        restore_metadata(Some(&home), "desk-before-migration-1-1.sqlite3").await,
        Err(StorageError::InvalidBackupName)
    ));
    Ok(())
}
