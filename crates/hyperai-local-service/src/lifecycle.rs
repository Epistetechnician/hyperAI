//! Process ownership records for one local `HyperAI` service instance.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use data_encoding::HEXLOWER;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::HomePaths;

const RECORD_SCHEMA: &str = "hyperai.local-service-record.v1";

/// Exclusive ownership of one `HyperAI` service home.
pub struct ServiceLock {
    _file: File,
}

/// Public, non-secret locator for a running local service.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRecordV1 {
    /// Versioned record schema.
    pub schema_version: String,
    /// Process identifier, informational only.
    pub process_id: u32,
    /// IPv4 loopback socket address.
    pub address: String,
    /// Fresh instance identifier used to distinguish restarts.
    pub instance_id: String,
    /// Unix time when this process wrote the record.
    pub started_at_unix_seconds: u64,
}

/// Local service ownership or runtime-record failure.
#[derive(Debug, Error)]
pub enum LifecycleError {
    /// Another process currently owns this local home.
    #[error("another HyperAI service owns this home")]
    AlreadyRunning,
    /// An unsafe path or malformed record was found.
    #[error("unsafe or invalid local service record: {0}")]
    InvalidRecord(PathBuf),
    /// Secure random instance generation failed.
    #[error("secure random instance generation failed: {0}")]
    Random(#[from] getrandom::Error),
    /// A filesystem operation failed.
    #[error("local service lifecycle operation failed: {0}")]
    Io(#[from] io::Error),
    /// Record encoding or decoding failed.
    #[error("local service record encoding failed: {0}")]
    Json(#[from] serde_json::Error),
}

impl ServiceLock {
    /// Claims the service home until this value is dropped.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError::AlreadyRunning`] if another process holds
    /// the lock, or an I/O error if the lock file is unsafe or unavailable.
    pub fn acquire(paths: &HomePaths) -> Result<Self, LifecycleError> {
        let path = paths.runtime.join("service.lock");
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
            Ok(()) => Ok(Self { _file: file }),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(LifecycleError::AlreadyRunning)
            }
            Err(error) => Err(error.into()),
        }
    }
}

/// Builds a fresh service-instance record for its bound loopback address.
///
/// # Errors
///
/// Returns an error if secure random generation fails.
pub fn new_record(address: String) -> Result<ServiceRecordV1, LifecycleError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)?;
    let instance_id = HEXLOWER.encode(&bytes);
    let started_at_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(ServiceRecordV1 {
        schema_version: RECORD_SCHEMA.to_owned(),
        process_id: std::process::id(),
        address,
        instance_id,
        started_at_unix_seconds,
    })
}

/// Atomically persists a service locator while the caller holds the home lock.
///
/// # Errors
///
/// Returns an error if serialization or a secure filesystem operation fails.
pub fn write_record(paths: &HomePaths, record: &ServiceRecordV1) -> Result<(), LifecycleError> {
    let bytes = serde_json::to_vec_pretty(record)?;
    let temporary = paths.runtime.join(format!(
        "service-{}-{}.tmp",
        std::process::id(),
        record.instance_id
    ));
    reject_symlink(&temporary)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    restrict_file(&temporary)?;

    let destination = record_path(paths);
    reject_symlink(&destination)?;
    if destination.exists() {
        fs::remove_file(&destination)?;
    }
    fs::rename(&temporary, &destination)?;
    restrict_file(&destination)?;
    sync_parent(&destination)?;
    Ok(())
}

/// Reads a syntactically valid locator; callers must still probe the service.
///
/// # Errors
///
/// Returns an error if the record is missing, unsafe, or invalid.
pub fn read_record(paths: &HomePaths) -> Result<ServiceRecordV1, LifecycleError> {
    let path = record_path(paths);
    reject_symlink(&path)?;
    let bytes = fs::read(&path)?;
    let record: ServiceRecordV1 = serde_json::from_slice(&bytes)?;
    let address = record.address.parse::<std::net::SocketAddr>();
    if record.schema_version != RECORD_SCHEMA
        || !address.is_ok_and(|address| address.ip().is_loopback() && address.port() != 0)
        || record.instance_id.len() != 32
        || !record
            .instance_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(LifecycleError::InvalidRecord(path));
    }
    Ok(record)
}

/// Removes the current service record during clean shutdown.
///
/// # Errors
///
/// Returns an error if the record path is unsafe or removal fails.
pub fn remove_record(paths: &HomePaths) -> Result<(), LifecycleError> {
    let path = record_path(paths);
    reject_symlink(&path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Returns the local service record path.
#[must_use]
pub fn record_path(paths: &HomePaths) -> PathBuf {
    paths.runtime.join("service.json")
}

fn reject_symlink(path: &Path) -> Result<(), LifecycleError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LifecycleError::InvalidRecord(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn restrict_file(path: &Path) -> Result<(), LifecycleError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    crate::storage::restrict_windows_file(path)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}

fn sync_parent(path: &Path) -> Result<(), LifecycleError> {
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
