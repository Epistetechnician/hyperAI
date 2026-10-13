//! Strict local runtime-configuration parsing and validation.
//!
//! The crate reads at most one explicit local JSON file. It does not merge
//! defaults, inspect the environment, create directories, open databases,
//! resolve secrets, or perform network or chain operations.

mod secret_reference;

use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use hyperintel_core::{CanonicalizationError, ProtocolId, Sha256Digest, domain_separated_digest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use secret_reference::{SECRET_REFERENCE_SCHEMA_V1, SecretReferenceError, SecretReferenceV1};

/// Schema accepted by the V1 local runtime-configuration loader.
pub const RUNTIME_CONFIGURATION_SCHEMA_V1: &str = "hyperintel.runtime-configuration.v1";
/// Domain used to commit one validated runtime-configuration document.
pub const RUNTIME_CONFIGURATION_DIGEST_DOMAIN_V1: &str = "hyperintel.runtime-configuration.v1";
/// Largest accepted runtime-configuration document.
pub const MAX_RUNTIME_CONFIGURATION_BYTES: usize = 65_536;
/// Largest accepted local `SQLite` connection pool.
pub const MAX_LOCAL_SQLITE_CONNECTIONS: u32 = 16;

/// Complete V1 runtime-configuration document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfigurationV1 {
    /// Fixed runtime-configuration schema.
    pub schema_version: String,
    /// Canonical identity of this local runtime instance.
    pub runtime_id: ProtocolId,
    /// Explicitly bounded runtime profile.
    pub profile: RuntimeProfileV1,
    /// Local replay and execution-journal database settings.
    pub local_persistence: LocalPersistenceConfigurationV1,
}

/// Runtime profiles supported by the V1 loader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeProfileV1 {
    /// Hermetic runtime with operator-selected local files only.
    LocalHermetic,
}

/// Settings for the two deliberately separate local databases.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPersistenceConfigurationV1 {
    /// Durable authenticated replay database.
    pub replay_database: LocalSqliteConfigurationV1,
    /// Metadata-only execution journal database.
    pub execution_journal_database: LocalSqliteConfigurationV1,
}

/// Settings for one operator-selected local `SQLite` file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSqliteConfigurationV1 {
    /// Absolute lexical path to one database file.
    pub path: String,
    /// Bounded connection-pool size.
    pub max_connections: u32,
}

/// Database field named by a closed configuration failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalDatabaseFieldV1 {
    /// Replay database configuration.
    Replay,
    /// Execution-journal database configuration.
    ExecutionJournal,
}

/// Closed reasons that a configured database path is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabasePathRejectionV1 {
    /// Path string is empty.
    Empty,
    /// Path is not absolute.
    NotAbsolute,
    /// Path does not end in a file name.
    DoesNotNameFile,
    /// Path contains a NUL byte.
    ContainsNul,
    /// Path contains an explicit current-directory component.
    ContainsCurrentDirectory,
    /// Path contains a parent traversal.
    ContainsParentTraversal,
}

/// Strict runtime-configuration failures.
#[derive(Debug, Error)]
pub enum RuntimeConfigurationError {
    /// Explicit local configuration file could not be read.
    #[error("runtime configuration file read failed: {0}")]
    Io(#[from] std::io::Error),
    /// Empty input is never interpreted as defaults.
    #[error("runtime configuration document is empty")]
    EmptyDocument,
    /// Input exceeded the fixed document budget.
    #[error("runtime configuration exceeds the {maximum_bytes}-byte limit")]
    DocumentTooLarge {
        /// Fixed maximum accepted input size.
        maximum_bytes: usize,
    },
    /// JSON syntax or strict typed decoding failed.
    #[error("runtime configuration JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// Schema is not the exact V1 identifier.
    #[error("unsupported runtime-configuration schema")]
    WrongSchemaVersion,
    /// One database path violated the local lexical-path contract.
    #[error("runtime configuration database path is invalid for {field:?}: {reason:?}")]
    InvalidDatabasePath {
        /// Database setting that failed.
        field: LocalDatabaseFieldV1,
        /// Closed failure reason.
        reason: DatabasePathRejectionV1,
    },
    /// One local pool is outside the fixed inclusive range.
    #[error(
        "runtime configuration connection count is outside {minimum}..={maximum} for {field:?}"
    )]
    ConnectionCountOutOfRange {
        /// Database setting that failed.
        field: LocalDatabaseFieldV1,
        /// Fixed minimum.
        minimum: u32,
        /// Fixed maximum.
        maximum: u32,
    },
    /// Replay and journal resolve to the same lexical file path.
    #[error("runtime configuration database paths must be distinct")]
    DatabasePathCollision,
    /// Canonical configuration identity could not be produced.
    #[error("runtime configuration canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Validated configuration and its normalized local paths and identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedRuntimeConfigurationV1 {
    document: RuntimeConfigurationV1,
    configuration_digest: Sha256Digest,
    replay_database_path: PathBuf,
    execution_journal_database_path: PathBuf,
}

impl ValidatedRuntimeConfigurationV1 {
    /// Return the exact validated document.
    #[must_use]
    pub const fn document(&self) -> &RuntimeConfigurationV1 {
        &self.document
    }

    /// Return the canonical configuration identity.
    #[must_use]
    pub const fn configuration_digest(&self) -> Sha256Digest {
        self.configuration_digest
    }

    /// Return the normalized absolute replay-database path.
    #[must_use]
    pub fn replay_database_path(&self) -> &Path {
        &self.replay_database_path
    }

    /// Return the normalized absolute execution-journal path.
    #[must_use]
    pub fn execution_journal_database_path(&self) -> &Path {
        &self.execution_journal_database_path
    }
}

impl RuntimeConfigurationV1 {
    /// Validate every V1 invariant and compute the canonical configuration identity.
    ///
    /// # Errors
    ///
    /// Returns a closed failure for schema, path, pool-size, or canonicalization errors.
    pub fn validate(self) -> Result<ValidatedRuntimeConfigurationV1, RuntimeConfigurationError> {
        if self.schema_version != RUNTIME_CONFIGURATION_SCHEMA_V1 {
            return Err(RuntimeConfigurationError::WrongSchemaVersion);
        }

        let replay_database_path = validate_database_path(
            &self.local_persistence.replay_database.path,
            LocalDatabaseFieldV1::Replay,
        )?;
        let execution_journal_database_path = validate_database_path(
            &self.local_persistence.execution_journal_database.path,
            LocalDatabaseFieldV1::ExecutionJournal,
        )?;
        validate_connection_count(
            self.local_persistence.replay_database.max_connections,
            LocalDatabaseFieldV1::Replay,
        )?;
        validate_connection_count(
            self.local_persistence
                .execution_journal_database
                .max_connections,
            LocalDatabaseFieldV1::ExecutionJournal,
        )?;
        if replay_database_path == execution_journal_database_path {
            return Err(RuntimeConfigurationError::DatabasePathCollision);
        }

        let configuration_digest =
            domain_separated_digest(RUNTIME_CONFIGURATION_DIGEST_DOMAIN_V1, &self)?;
        Ok(ValidatedRuntimeConfigurationV1 {
            document: self,
            configuration_digest,
            replay_database_path,
            execution_journal_database_path,
        })
    }
}

/// Parse and validate one bounded JSON runtime-configuration document.
///
/// # Errors
///
/// Returns on empty, oversized, malformed, unsupported, or invalid input.
pub fn parse_runtime_configuration_v1(
    bytes: &[u8],
) -> Result<ValidatedRuntimeConfigurationV1, RuntimeConfigurationError> {
    if bytes.is_empty() {
        return Err(RuntimeConfigurationError::EmptyDocument);
    }
    if bytes.len() > MAX_RUNTIME_CONFIGURATION_BYTES {
        return Err(RuntimeConfigurationError::DocumentTooLarge {
            maximum_bytes: MAX_RUNTIME_CONFIGURATION_BYTES,
        });
    }
    serde_json::from_slice::<RuntimeConfigurationV1>(bytes)?.validate()
}

/// Load and validate one explicit local JSON runtime-configuration file.
///
/// The read is bounded before JSON decoding. This function creates nothing and
/// does not inspect the environment.
///
/// # Errors
///
/// Returns when the explicit file cannot be read or its document is rejected.
pub fn load_runtime_configuration_v1(
    path: &Path,
) -> Result<ValidatedRuntimeConfigurationV1, RuntimeConfigurationError> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(65_537).read_to_end(&mut bytes)?;
    parse_runtime_configuration_v1(&bytes)
}

fn validate_database_path(
    value: &str,
    field: LocalDatabaseFieldV1,
) -> Result<PathBuf, RuntimeConfigurationError> {
    if value.is_empty() {
        return Err(invalid_path(field, DatabasePathRejectionV1::Empty));
    }
    if value.as_bytes().contains(&0) {
        return Err(invalid_path(field, DatabasePathRejectionV1::ContainsNul));
    }
    for segment in value.split(['/', '\\']) {
        if segment == "." {
            return Err(invalid_path(
                field,
                DatabasePathRejectionV1::ContainsCurrentDirectory,
            ));
        }
        if segment == ".." {
            return Err(invalid_path(
                field,
                DatabasePathRejectionV1::ContainsParentTraversal,
            ));
        }
    }

    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(invalid_path(field, DatabasePathRejectionV1::NotAbsolute));
    }
    if value.ends_with('/') || value.ends_with('\\') || path.file_name().is_none() {
        return Err(invalid_path(
            field,
            DatabasePathRejectionV1::DoesNotNameFile,
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {
                return Err(invalid_path(
                    field,
                    DatabasePathRejectionV1::ContainsCurrentDirectory,
                ));
            }
            Component::ParentDir => {
                return Err(invalid_path(
                    field,
                    DatabasePathRejectionV1::ContainsParentTraversal,
                ));
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    if normalized.file_name().is_none() {
        return Err(invalid_path(
            field,
            DatabasePathRejectionV1::DoesNotNameFile,
        ));
    }
    Ok(normalized)
}

fn invalid_path(
    field: LocalDatabaseFieldV1,
    reason: DatabasePathRejectionV1,
) -> RuntimeConfigurationError {
    RuntimeConfigurationError::InvalidDatabasePath { field, reason }
}

fn validate_connection_count(
    value: u32,
    field: LocalDatabaseFieldV1,
) -> Result<(), RuntimeConfigurationError> {
    if (1..=MAX_LOCAL_SQLITE_CONNECTIONS).contains(&value) {
        Ok(())
    } else {
        Err(RuntimeConfigurationError::ConnectionCountOutOfRange {
            field,
            minimum: 1,
            maximum: MAX_LOCAL_SQLITE_CONNECTIONS,
        })
    }
}
