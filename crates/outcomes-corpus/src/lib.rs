//! Challenge-corpus ports, normalized packages, and solver-view boundaries.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read as _,
    path::{Path, PathBuf},
};

use outcomes_domain::{
    AdapterHealthStatus, AdapterHealthV1, ChallengeDescriptorV1, ChallengePackageV1,
    CorpusDescriptorV1, CorpusSnapshotV1, PackageArtifactV1, PackageVisibility,
    RightsAndProvenanceAssessmentV1, RightsStatus, SettlementReadiness, Sha256Digest,
    SolverViewManifestV1, UtcTimestamp, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Failures returned by corpus adapters and solver-view validation.
#[derive(Debug, Error)]
pub enum CorpusError {
    /// Requested immutable snapshot does not exist in this adapter.
    #[error("unknown corpus snapshot {0}")]
    UnknownSnapshot(String),
    /// Requested challenge reference does not exist in the snapshot.
    #[error("unknown challenge reference {0}")]
    UnknownChallenge(String),
    /// A mutable discovery reference was used where a frozen commit was required.
    #[error("ref {0} is mutable or unresolved and cannot be frozen")]
    MutableRef(String),
    /// Solver view exposed a runtime, verifier, secret, or contamination artifact.
    #[error("solver view exposes prohibited artifact {0}")]
    SecretExposure(Sha256Digest),
    /// Solver view references an artifact absent from the package.
    #[error("solver view references unknown artifact {0}")]
    UnknownSolverArtifact(Sha256Digest),
    /// Domain payload could not be canonicalized.
    #[error("corpus payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Bundled profile JSON was invalid.
    #[error("corpus profile catalog JSON is invalid: {0}")]
    ProfileJson(#[from] serde_json::Error),
    /// Bundled profile violates immutable pin or uniqueness requirements.
    #[error("invalid corpus profile {profile_id}: {reason}")]
    InvalidProfile {
        /// Invalid profile identifier.
        profile_id: String,
        /// Exact invariant failure.
        reason: String,
    },
    /// The explicit local checkout boundary was missing or unsafe.
    #[error("local checkout root {path} is rejected: {reason}")]
    InvalidCheckoutRoot {
        /// Caller-supplied checkout root.
        path: String,
        /// Exact validation failure.
        reason: String,
    },
    /// The caller-supplied immutable pin did not match the catalog or checkout.
    #[error("commit mismatch for {profile_id}: expected {expected}, observed {observed}")]
    CommitMismatch {
        /// Catalog profile being materialized.
        profile_id: String,
        /// Catalog-pinned commit.
        expected: String,
        /// Conflicting caller-supplied commit.
        observed: String,
    },
    /// The requested inventory root was outside the profile allowlist.
    #[error("root {root} is not allowlisted for corpus profile {profile_id}")]
    UnallowlistedRoot {
        /// Catalog profile being materialized.
        profile_id: String,
        /// Rejected normalized root.
        root: String,
    },
    /// A path in the selected local tree violated the materialization policy.
    #[error("unsafe materialization path {path}: {reason}")]
    UnsafeMaterializationPath {
        /// Rejected normalized path.
        path: String,
        /// Exact policy violation.
        reason: String,
    },
    /// A configured file-count or byte limit was exceeded.
    #[error("materialization limit {limit} exceeded at {path}")]
    MaterializationLimitExceeded {
        /// Limit name and configured value.
        limit: String,
        /// Path at which scanning stopped.
        path: String,
    },
    /// A local filesystem operation failed.
    #[error("local corpus {operation} failed for {path}: {source}")]
    LocalIo {
        /// Filesystem operation.
        operation: &'static str,
        /// Explicit local path.
        path: String,
        /// Underlying operating-system error.
        #[source]
        source: std::io::Error,
    },
}

/// Hard resource ceilings for local corpus byte materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalMaterializationLimits {
    /// Maximum number of regular files accepted in one manifest.
    pub max_file_count: usize,
    /// Maximum accepted length of one file.
    pub max_file_bytes: u64,
    /// Maximum accepted aggregate length of all files.
    pub max_total_bytes: u64,
}

impl Default for LocalMaterializationLimits {
    fn default() -> Self {
        Self {
            max_file_count: 10_000,
            max_file_bytes: 16 * 1024 * 1024,
            max_total_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Caller-asserted boundary for a pre-existing, pinned local checkout.
///
/// The materializer never invokes Git, performs network I/O, or executes a
/// discovered file. Both commit values are supplied as data and checked
/// against the bundled immutable profile pin.
#[derive(Debug, Clone, Copy)]
pub struct LocalMaterializationRequest<'a> {
    /// Stable bundled profile identifier.
    pub profile_id: &'a str,
    /// Absolute path of the already checked-out repository.
    pub checkout_root: &'a Path,
    /// Normalized relative root selected below the checkout.
    pub materialization_root: &'a str,
    /// Commit the caller expects from policy/configuration.
    pub expected_commit: &'a str,
    /// Commit the caller observed for the local checkout.
    pub observed_checkout_commit: &'a str,
    /// Explicit scan resource ceilings.
    pub limits: LocalMaterializationLimits,
}

/// One deterministically ordered regular file in a local materialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMaterializedFile {
    /// Slash-normalized path relative to the checkout root.
    pub normalized_path: String,
    /// SHA-256 digest of the file bytes, without metadata or execution.
    pub content_digest: Sha256Digest,
    /// Exact byte length read under the configured ceiling.
    pub byte_length: u64,
}

/// Deterministic manifest of bytes read from one caller-provided local tree.
///
/// Materialization is an evaluation input only. It does not establish source
/// completeness, rights, verifier correctness, or settlement eligibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCorpusMaterialization {
    /// Manifest schema version.
    pub schema_version: String,
    /// Bundled profile identifier.
    pub profile_id: String,
    /// Immutable commit supplied and cross-checked as data.
    pub pinned_commit: String,
    /// Normalized allowlisted root that was scanned.
    pub materialization_root: String,
    /// Deterministically sorted file records.
    pub files: Vec<LocalMaterializedFile>,
    /// Aggregate accepted file bytes.
    pub total_bytes: u64,
    /// Digest over the canonical manifest body.
    pub manifest_digest: Sha256Digest,
    /// Catalog ceiling, which is structurally forbidden from settlement-ready.
    pub settlement_readiness: SettlementReadiness,
    /// Explicit execution boundary; always false for this materializer.
    pub executed_files: bool,
}

#[derive(Serialize)]
struct LocalCorpusMaterializationBody<'a> {
    schema_version: &'a str,
    profile_id: &'a str,
    pinned_commit: &'a str,
    materialization_root: &'a str,
    files: &'a [LocalMaterializedFile],
    total_bytes: u64,
    settlement_readiness: &'a SettlementReadiness,
    executed_files: bool,
}

const LOCAL_MATERIALIZATION_SCHEMA_VERSION: &str = "hsai.local-corpus-materialization.v1";

/// Machine-readable catalog of reviewed generic-Git manifest profiles.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusProfileCatalog {
    /// Catalog schema version.
    pub schema_version: String,
    /// Dated verification snapshot.
    pub verified_at: UtcTimestamp,
    /// Seven required pinned corpus paths.
    pub profiles: Vec<CorpusProfile>,
}

/// One data-only profile over the shared generic-Git corpus engine.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusProfile {
    /// Stable profile identifier.
    pub profile_id: String,
    /// Shared adapter engine identifier.
    pub engine: String,
    /// Allowlisted public upstream repository.
    pub repository: String,
    /// Immutable 40-hex Git commit.
    pub commit: String,
    /// Reviewed inventory roots.
    pub roots: Vec<String>,
    /// Solver-view exclusion patterns.
    pub excluded_patterns: Vec<String>,
    /// Human-readable exact license boundary.
    pub license_status: String,
    /// Typed rights ceiling.
    pub rights_status: RightsStatus,
    /// Typed settlement readiness ceiling.
    pub readiness: SettlementReadiness,
}

/// Sanitized normalized fixture catalog for pinned external profiles.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedCorpusFixtureCatalog {
    /// Catalog schema version.
    pub schema_version: String,
    /// One sanitized path manifest per profile.
    pub fixtures: Vec<RecordedCorpusFixture>,
}

/// Normalized paths retained without copying upstream challenge bytes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedCorpusFixture {
    /// Matching corpus profile ID.
    pub profile_id: String,
    /// Stable recorded challenge reference.
    pub challenge_ref: String,
    /// Paths safe for a solver view.
    pub normalized_public_paths: Vec<String>,
    /// Paths excluded as verifier, secret, solution, or writeup material.
    pub protected_paths: Vec<String>,
    /// Exact runnable-verifier status.
    pub verifier_status: String,
}

/// Materialize regular bytes from one allowlisted root in an already-pinned
/// local checkout.
///
/// This operation is deliberately filesystem-only: it does not invoke Git,
/// resolve refs, fetch objects, follow symlinks, contact an upstream service,
/// or execute discovered content. The caller remains responsible for deriving
/// `observed_checkout_commit` through a separately trusted checkout process.
///
/// # Errors
///
/// Returns an error for an unknown profile, a non-absolute or symlink checkout,
/// commit disagreement, an unallowlisted or unsafe root, any unsafe/protected
/// tree entry, non-regular content, resource-limit overflow, filesystem races,
/// or canonical manifest failure. VCS control directories are skipped without
/// reading because they are checkout metadata rather than challenge bytes.
#[allow(clippy::too_many_lines)]
pub fn materialize_pinned_local_root(
    catalog: &CorpusProfileCatalog,
    request: &LocalMaterializationRequest<'_>,
) -> Result<LocalCorpusMaterialization, CorpusError> {
    let profile = catalog
        .profiles
        .iter()
        .find(|profile| profile.profile_id == request.profile_id)
        .ok_or_else(|| CorpusError::InvalidProfile {
            profile_id: request.profile_id.to_owned(),
            reason: "profile is absent from the supplied catalog".to_owned(),
        })?;

    if request.expected_commit != profile.commit {
        return Err(CorpusError::CommitMismatch {
            profile_id: profile.profile_id.clone(),
            expected: profile.commit.clone(),
            observed: request.expected_commit.to_owned(),
        });
    }
    if request.observed_checkout_commit != profile.commit {
        return Err(CorpusError::CommitMismatch {
            profile_id: profile.profile_id.clone(),
            expected: profile.commit.clone(),
            observed: request.observed_checkout_commit.to_owned(),
        });
    }
    if profile.readiness == SettlementReadiness::SettlementReady {
        return Err(CorpusError::InvalidProfile {
            profile_id: profile.profile_id.clone(),
            reason: "local byte materialization cannot promote settlement readiness".to_owned(),
        });
    }
    validate_checkout_root(request.checkout_root)?;
    validate_normalized_relative_path(request.materialization_root, true).map_err(|reason| {
        CorpusError::UnsafeMaterializationPath {
            path: request.materialization_root.to_owned(),
            reason,
        }
    })?;
    if !profile
        .roots
        .iter()
        .any(|allowed| root_is_allowlisted(allowed, request.materialization_root))
    {
        return Err(CorpusError::UnallowlistedRoot {
            profile_id: profile.profile_id.clone(),
            root: request.materialization_root.to_owned(),
        });
    }
    if request.limits.max_file_count == 0
        || request.limits.max_file_bytes == 0
        || request.limits.max_total_bytes == 0
    {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: "all limits must be non-zero".to_owned(),
            path: request.materialization_root.to_owned(),
        });
    }

    let relative_root = if request.materialization_root == "." {
        PathBuf::new()
    } else {
        request
            .materialization_root
            .split('/')
            .fold(PathBuf::new(), |mut path, component| {
                path.push(component);
                path
            })
    };
    let selected_root = request.checkout_root.join(&relative_root);
    let selected_metadata =
        fs::symlink_metadata(&selected_root).map_err(|source| CorpusError::LocalIo {
            operation: "inspect selected root",
            path: selected_root.display().to_string(),
            source,
        })?;
    if selected_metadata.file_type().is_symlink() || !selected_metadata.is_dir() {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: request.materialization_root.to_owned(),
            reason: "selected root must be a real directory, not a symlink".to_owned(),
        });
    }
    if protected_materialization_path(profile, request.materialization_root) {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: request.materialization_root.to_owned(),
            reason: "selected root is protected, secret, solution, or flag material".to_owned(),
        });
    }

    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    scan_materialization_directory(
        &selected_root,
        &relative_root,
        profile,
        request.limits,
        &mut files,
        &mut total_bytes,
    )?;
    files.sort_by(|left, right| left.normalized_path.cmp(&right.normalized_path));
    if files.is_empty() {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: request.materialization_root.to_owned(),
            reason: "selected root contains no accepted regular files".to_owned(),
        });
    }

    let body = LocalCorpusMaterializationBody {
        schema_version: LOCAL_MATERIALIZATION_SCHEMA_VERSION,
        profile_id: &profile.profile_id,
        pinned_commit: &profile.commit,
        materialization_root: request.materialization_root,
        files: &files,
        total_bytes,
        settlement_readiness: &profile.readiness,
        executed_files: false,
    };
    let manifest_digest = domain_separated_digest("hsai.local-corpus-materialization.v1", &body)?;
    Ok(LocalCorpusMaterialization {
        schema_version: LOCAL_MATERIALIZATION_SCHEMA_VERSION.to_owned(),
        profile_id: profile.profile_id.clone(),
        pinned_commit: profile.commit.clone(),
        materialization_root: request.materialization_root.to_owned(),
        files,
        total_bytes,
        manifest_digest,
        settlement_readiness: profile.readiness,
        executed_files: false,
    })
}

fn validate_checkout_root(checkout_root: &Path) -> Result<(), CorpusError> {
    let display = checkout_root.display().to_string();
    if !checkout_root.is_absolute() {
        return Err(CorpusError::InvalidCheckoutRoot {
            path: display,
            reason: "checkout root must be absolute".to_owned(),
        });
    }
    if checkout_root.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    }) {
        return Err(CorpusError::InvalidCheckoutRoot {
            path: display,
            reason: "checkout root must be lexically normalized".to_owned(),
        });
    }
    let metadata = fs::symlink_metadata(checkout_root).map_err(|source| CorpusError::LocalIo {
        operation: "inspect checkout root",
        path: display.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CorpusError::InvalidCheckoutRoot {
            path: display,
            reason: "checkout root must be a real directory, not a symlink".to_owned(),
        });
    }
    Ok(())
}

fn root_is_allowlisted(allowlisted_root: &str, requested_root: &str) -> bool {
    if validate_normalized_relative_path(allowlisted_root, true).is_err() {
        return false;
    }
    allowlisted_root == "."
        || requested_root == allowlisted_root
        || requested_root
            .strip_prefix(allowlisted_root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[allow(clippy::too_many_lines)]
fn scan_materialization_directory(
    directory: &Path,
    relative_directory: &Path,
    profile: &CorpusProfile,
    limits: LocalMaterializationLimits,
    files: &mut Vec<LocalMaterializedFile>,
    total_bytes: &mut u64,
) -> Result<(), CorpusError> {
    let read_dir = fs::read_dir(directory).map_err(|source| CorpusError::LocalIo {
        operation: "read directory",
        path: directory.display().to_string(),
        source,
    })?;
    let mut entries = read_dir
        .map(|entry| {
            entry.map_err(|source| CorpusError::LocalIo {
                operation: "read directory entry",
                path: directory.display().to_string(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let name = entry.file_name().into_string().map_err(|_| {
            CorpusError::UnsafeMaterializationPath {
                path: relative_directory.display().to_string(),
                reason: "path component is not valid UTF-8".to_owned(),
            }
        })?;
        validate_normalized_relative_path(&name, false).map_err(|reason| {
            CorpusError::UnsafeMaterializationPath {
                path: name.clone(),
                reason,
            }
        })?;
        let relative_path = relative_directory.join(&name);
        let normalized_path = slash_normalized_path(&relative_path)?;
        let absolute_path = entry.path();
        let metadata =
            fs::symlink_metadata(&absolute_path).map_err(|source| CorpusError::LocalIo {
                operation: "inspect tree entry",
                path: absolute_path.display().to_string(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path,
                reason: "symlinks are forbidden".to_owned(),
            });
        }
        if vcs_control_path(&normalized_path) {
            continue;
        }
        if protected_materialization_path(profile, &normalized_path) {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path,
                reason: "protected, secret, solution, or flag path is forbidden".to_owned(),
            });
        }
        if metadata.is_dir() {
            scan_materialization_directory(
                &absolute_path,
                &relative_path,
                profile,
                limits,
                files,
                total_bytes,
            )?;
        } else if metadata.is_file() {
            if files.len() >= limits.max_file_count {
                return Err(CorpusError::MaterializationLimitExceeded {
                    limit: format!("max_file_count={}", limits.max_file_count),
                    path: normalized_path,
                });
            }
            if metadata.len() > limits.max_file_bytes {
                return Err(CorpusError::MaterializationLimitExceeded {
                    limit: format!("max_file_bytes={}", limits.max_file_bytes),
                    path: normalized_path,
                });
            }
            let next_total = total_bytes.checked_add(metadata.len()).ok_or_else(|| {
                CorpusError::MaterializationLimitExceeded {
                    limit: format!("max_total_bytes={}", limits.max_total_bytes),
                    path: normalized_path.clone(),
                }
            })?;
            if next_total > limits.max_total_bytes {
                return Err(CorpusError::MaterializationLimitExceeded {
                    limit: format!("max_total_bytes={}", limits.max_total_bytes),
                    path: normalized_path,
                });
            }
            let bytes = read_bounded_regular_file(
                &absolute_path,
                &metadata,
                limits.max_file_bytes,
                &normalized_path,
            )?;
            let digest: [u8; 32] = Sha256::digest(&bytes).into();
            files.push(LocalMaterializedFile {
                normalized_path,
                content_digest: Sha256Digest::from_bytes(digest),
                byte_length: metadata.len(),
            });
            *total_bytes = next_total;
        } else {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path,
                reason: "only regular files and directories are allowed".to_owned(),
            });
        }
    }
    Ok(())
}

fn read_bounded_regular_file(
    path: &Path,
    inspected_metadata: &fs::Metadata,
    max_file_bytes: u64,
    normalized_path: &str,
) -> Result<Vec<u8>, CorpusError> {
    let file = File::open(path).map_err(|source| CorpusError::LocalIo {
        operation: "open regular file",
        path: path.display().to_string(),
        source,
    })?;
    let opened_metadata = file.metadata().map_err(|source| CorpusError::LocalIo {
        operation: "inspect opened file",
        path: path.display().to_string(),
        source,
    })?;
    if !opened_metadata.is_file() || !same_filesystem_object(inspected_metadata, &opened_metadata) {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "filesystem entry changed while it was opened".to_owned(),
        });
    }
    let mut bytes = Vec::new();
    file.take(max_file_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CorpusError::LocalIo {
            operation: "read regular file",
            path: path.display().to_string(),
            source,
        })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_file_bytes {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: format!("max_file_bytes={max_file_bytes}"),
            path: normalized_path.to_owned(),
        });
    }
    if u64::try_from(bytes.len()).ok() != Some(inspected_metadata.len()) {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "file length changed while bytes were read".to_owned(),
        });
    }
    Ok(bytes)
}

#[cfg(unix)]
fn same_filesystem_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_filesystem_object(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
        && left.file_type().is_file() == right.file_type().is_file()
        && left.modified().ok() == right.modified().ok()
}

fn slash_normalized_path(path: &Path) -> Result<String, CorpusError> {
    let components = path
        .iter()
        .map(|component| {
            component
                .to_str()
                .ok_or_else(|| CorpusError::UnsafeMaterializationPath {
                    path: path.display().to_string(),
                    reason: "path component is not valid UTF-8".to_owned(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let normalized = components.join("/");
    validate_normalized_relative_path(&normalized, false).map_err(|reason| {
        CorpusError::UnsafeMaterializationPath {
            path: normalized.clone(),
            reason,
        }
    })?;
    Ok(normalized)
}

fn validate_normalized_relative_path(path: &str, allow_dot: bool) -> Result<(), String> {
    if path.is_empty() {
        return Err("path is empty".to_owned());
    }
    if path == "." {
        return if allow_dot {
            Ok(())
        } else {
            Err("dot path is forbidden".to_owned())
        };
    }
    if Path::new(path).is_absolute() {
        return Err("absolute paths are forbidden".to_owned());
    }
    if path.contains('\0') {
        return Err("NUL bytes are forbidden".to_owned());
    }
    if path.contains('\\') {
        return Err("backslashes are forbidden".to_owned());
    }
    if path
        .split('/')
        .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err("path must be normalized without empty, dot, or parent components".to_owned());
    }
    Ok(())
}

fn protected_materialization_path(profile: &CorpusProfile, path: &str) -> bool {
    let components = path
        .split('/')
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    components.iter().any(|component| {
        let stem = component.split('.').next().unwrap_or(component);
        matches!(component.as_str(), "private" | "verifier" | "server_files")
            || matches!(
                stem,
                "flag" | "solution" | "solutions" | "solve" | "writeup" | "writeups"
            )
            || stem.starts_with("flag_")
            || stem.ends_with("_flag")
            || stem.starts_with("solution_")
            || stem.starts_with("solve_")
            || stem.starts_with("writeup_")
    }) || profile.excluded_patterns.iter().any(|pattern| {
        let lower = pattern.to_ascii_lowercase();
        (lower.contains("server_files") && components.iter().any(|part| part == "server_files"))
            || (lower.contains("solution")
                && components.iter().any(|part| part.contains("solution")))
            || (lower.contains("writeup") && components.iter().any(|part| part.contains("writeup")))
            || (lower.contains("flag") && components.iter().any(|part| part.contains("flag")))
            || (lower.contains("solve")
                && components.iter().any(|part| {
                    part == "solve" || part.starts_with("solve.") || part.starts_with("solve_")
                }))
    })
}

fn vcs_control_path(path: &str) -> bool {
    path.split('/')
        .any(|component| matches!(component, ".git" | ".hg" | ".svn"))
}

/// Load and validate the seven required pinned corpus profiles.
///
/// # Errors
///
/// Returns JSON, schema, duplicate, mutable-ref, engine, or empty-boundary
/// errors. Loading performs no network access.
pub fn bundled_corpus_profiles() -> Result<CorpusProfileCatalog, CorpusError> {
    let catalog: CorpusProfileCatalog =
        serde_json::from_str(include_str!("../../../config/corpus-profiles.json"))?;
    if catalog.schema_version != "hsai.corpus-profile-catalog.v1" {
        return Err(CorpusError::InvalidProfile {
            profile_id: "catalog".to_owned(),
            reason: "unsupported schema version".to_owned(),
        });
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut repositories = std::collections::BTreeSet::new();
    for profile in &catalog.profiles {
        let invalid_commit = profile.commit.len() != 40
            || profile
                .commit
                .bytes()
                .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        let reason = if !ids.insert(profile.profile_id.clone()) {
            Some("duplicate profile ID")
        } else if !repositories.insert(profile.repository.clone()) {
            Some("duplicate repository profile")
        } else if profile.engine != "generic-git-corpus.v1" {
            Some("profile does not use the shared generic-Git engine")
        } else if invalid_commit {
            Some("commit is not immutable lowercase 40-hex")
        } else if profile.roots.is_empty() || profile.excluded_patterns.is_empty() {
            Some("inventory roots and solver-view exclusions are required")
        } else if profile.readiness == SettlementReadiness::SettlementReady {
            Some("archive profile cannot be settlement-ready without live observation")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(CorpusError::InvalidProfile {
                profile_id: profile.profile_id.clone(),
                reason: reason.to_owned(),
            });
        }
    }
    Ok(catalog)
}

/// Load and cross-check sanitized normalized path fixtures for every profile.
///
/// These fixtures exercise catalog normalization and solver-view boundaries;
/// they do not assert that upstream bytes were downloaded or are executable.
///
/// # Errors
///
/// Returns JSON, schema, coverage, duplicate, unsafe-path, exposure, or empty
/// manifest errors. Loading performs no network access.
pub fn bundled_recorded_corpus_fixtures() -> Result<RecordedCorpusFixtureCatalog, CorpusError> {
    let profiles = bundled_corpus_profiles()?;
    let catalog: RecordedCorpusFixtureCatalog = serde_json::from_str(include_str!(
        "../../../fixtures/corpora/recorded-profile-manifests.json"
    ))?;
    if catalog.schema_version != "hsai.recorded-corpus-fixture-catalog.v1" {
        return Err(CorpusError::InvalidProfile {
            profile_id: "recorded-fixtures".to_owned(),
            reason: "unsupported fixture schema version".to_owned(),
        });
    }
    let expected = profiles
        .profiles
        .iter()
        .map(|profile| profile.profile_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut actual = std::collections::BTreeSet::new();
    for fixture in &catalog.fixtures {
        let paths_valid = fixture
            .normalized_public_paths
            .iter()
            .chain(&fixture.protected_paths)
            .all(|path| safe_normalized_path(path));
        let overlap = fixture
            .normalized_public_paths
            .iter()
            .any(|path| fixture.protected_paths.contains(path));
        let public_exposes_secret = fixture.normalized_public_paths.iter().any(|path| {
            let lower = path.to_ascii_lowercase();
            ["flag", "solution", "solve", "writeup", "server_files"]
                .iter()
                .any(|marker| lower.contains(marker))
        });
        if !actual.insert(fixture.profile_id.as_str())
            || fixture.challenge_ref.is_empty()
            || fixture.normalized_public_paths.is_empty()
            || fixture.protected_paths.is_empty()
            || fixture.verifier_status.is_empty()
            || !paths_valid
            || overlap
            || public_exposes_secret
        {
            return Err(CorpusError::InvalidProfile {
                profile_id: fixture.profile_id.clone(),
                reason: "unsafe or incomplete recorded normalized manifest".to_owned(),
            });
        }
    }
    if actual != expected {
        return Err(CorpusError::InvalidProfile {
            profile_id: "recorded-fixtures".to_owned(),
            reason: "recorded fixtures do not cover every corpus profile exactly once".to_owned(),
        });
    }
    Ok(catalog)
}

fn safe_normalized_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\0')
        && !path.contains('\\')
        && !path.split('/').any(|component| component == "..")
}

/// Versioned port for immutable challenge corpus materialization.
// Each fallible method returns the adapter-specific `CorpusError`; concrete
// implementations document their narrower failure conditions.
#[allow(clippy::missing_errors_doc)]
pub trait ChallengeCorpusAdapter {
    /// Return the corpus descriptor.
    fn describe_corpus(&self) -> &CorpusDescriptorV1;
    /// Resolve a discovery ref to an immutable snapshot.
    fn snapshot_corpus(&self, reference: &str) -> Result<&CorpusSnapshotV1, CorpusError>;
    /// List stable challenge references in the snapshot.
    fn list_challenges(&self, snapshot_digest: Sha256Digest) -> Result<Vec<String>, CorpusError>;
    /// Return one challenge descriptor.
    fn describe_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengeDescriptorV1, CorpusError>;
    /// Materialize one normalized challenge package.
    fn materialize_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengePackageV1, CorpusError>;
    /// Return the explicit solver-view manifest.
    fn solver_view(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&SolverViewManifestV1, CorpusError>;
    /// Return item-level rights and provenance evidence.
    fn assess_license_and_provenance(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&RightsAndProvenanceAssessmentV1, CorpusError>;
    /// Return health without changing corpus state.
    fn health(&self) -> &AdapterHealthV1;
}

/// Hermetic single-challenge adapter used by contract and end-to-end tests.
#[derive(Debug, Clone)]
pub struct LocalFixtureCorpus {
    descriptor: CorpusDescriptorV1,
    snapshot: CorpusSnapshotV1,
    challenge: ChallengeDescriptorV1,
    package: ChallengePackageV1,
    solver_view: SolverViewManifestV1,
    rights: RightsAndProvenanceAssessmentV1,
    health: AdapterHealthV1,
}

impl LocalFixtureCorpus {
    /// Construct the deterministic local challenge package.
    ///
    /// # Errors
    ///
    /// Returns an error only if a built-in fixture value is not canonicalizable.
    #[allow(clippy::too_many_lines)]
    pub fn new(now: UtcTimestamp) -> Result<Self, CorpusError> {
        let public_digest =
            domain_separated_digest("hsai.fixture-artifact.v1", &"public challenge instructions")?;
        let verifier_digest =
            domain_separated_digest("hsai.fixture-artifact.v1", &"hidden verifier")?;
        let snapshot_digest = domain_separated_digest(
            "hsai.corpus-snapshot.v1",
            &(
                "local-fixture",
                "commit:0000000000000000000000000000000000000001",
            ),
        )?;
        let runtime_manifest_digest =
            domain_separated_digest("hsai.runtime-manifest.v1", &verifier_digest)?;
        let verifier_manifest_digest =
            domain_separated_digest("hsai.verifier-manifest.v1", &verifier_digest)?;
        let solver_view_manifest_digest =
            domain_separated_digest("hsai.solver-view.v1", &(public_digest, verifier_digest))?;
        let rights_assessment_digest =
            domain_separated_digest("hsai.rights-assessment.v1", &"fixture-permitted")?;
        let package_digest = domain_separated_digest(
            "hsai.challenge-package.v1",
            &(
                snapshot_digest,
                public_digest,
                verifier_digest,
                solver_view_manifest_digest,
            ),
        )?;
        let descriptor = CorpusDescriptorV1 {
            schema_version: "hsai.corpus-descriptor.v1".to_owned(),
            corpus_id: "corpus_local_fixture_v1".to_owned(),
            adapter_id: "local-fixture-corpus".to_owned(),
            upstream_uri: "fixture://local/computational-market".to_owned(),
            discovery_ref: "commit:0000000000000000000000000000000000000001".to_owned(),
            rights_status: RightsStatus::Permitted,
            created_at: now.clone(),
        };
        let snapshot = CorpusSnapshotV1 {
            schema_version: "hsai.corpus-snapshot.v1".to_owned(),
            corpus_id: descriptor.corpus_id.clone(),
            resolved_commit: descriptor.discovery_ref.clone(),
            snapshot_digest,
            submodule_digests: BTreeMap::new(),
            lfs_object_digests: Vec::new(),
            image_digests: Vec::new(),
            external_artifact_digests: Vec::new(),
            adapter_version: "1.0.0".to_owned(),
            received_at: now.clone(),
        };
        let challenge_spec_digest = domain_separated_digest(
            "hsai.challenge-spec.v1",
            &"candidate bytes must equal hsai-pass-v1",
        )?;
        let challenge = ChallengeDescriptorV1 {
            schema_version: outcomes_domain::CHALLENGE_DESCRIPTOR_SCHEMA_V1.to_owned(),
            challenge_id: "challenge_local_exact_bytes_v1".to_owned(),
            source_platform: "local-fixture".to_owned(),
            source_namespace: "local.demo".to_owned(),
            source_challenge_ref: "exact-bytes-v1".to_owned(),
            display_name: "Local exact bytes validator".to_owned(),
            canonical_spec_uri: "fixture://local/exact-bytes-v1/spec".to_owned(),
            canonical_spec_digest: challenge_spec_digest,
            source_terms_uri: "fixture://local/terms".to_owned(),
            source_timezone: "UTC".to_owned(),
            source_adapter_kind: "local-fixture-corpus".to_owned(),
            source_adapter_version: "1.0.0".to_owned(),
            created_at: now.clone(),
            display_metadata: BTreeMap::new(),
        };
        let package = ChallengePackageV1 {
            schema_version: "hsai.challenge-package.v1".to_owned(),
            package_id: "package_local_exact_bytes_v1".to_owned(),
            corpus_snapshot_digest: snapshot_digest,
            challenge_ref: challenge.source_challenge_ref.clone(),
            artifacts: vec![
                PackageArtifactV1 {
                    logical_path: "public/README.txt".to_owned(),
                    digest: public_digest,
                    byte_length: 29,
                    media_type: "text/plain".to_owned(),
                    visibility: PackageVisibility::SolverPublic,
                    executable: false,
                },
                PackageArtifactV1 {
                    logical_path: "verifier/checker".to_owned(),
                    digest: verifier_digest,
                    byte_length: 15,
                    media_type: "application/octet-stream".to_owned(),
                    visibility: PackageVisibility::VerifierOnly,
                    executable: true,
                },
            ],
            runtime_manifest_digest,
            verifier_manifest_digest,
            solver_view_manifest_digest,
            rights_assessment_digest,
            package_digest,
        };
        let solver_view = SolverViewManifestV1 {
            schema_version: "hsai.solver-view-manifest.v1".to_owned(),
            challenge_package_digest: package_digest,
            exposed_artifact_digests: vec![public_digest],
            excluded_secret_digests: vec![verifier_digest],
            manifest_digest: solver_view_manifest_digest,
        };
        let rights = RightsAndProvenanceAssessmentV1 {
            schema_version: "hsai.rights-and-provenance.v1".to_owned(),
            package_digest,
            upstream_attribution: vec!["HSAI local deterministic fixture".to_owned()],
            repository_license: None,
            item_license: Some("test-fixture-only".to_owned()),
            rights_status: RightsStatus::Permitted,
            redistribution_allowed: true,
            execution_allowed: true,
            commercial_market_use_allowed: false,
            exact_limitation: Some("local test and demonstration only".to_owned()),
            assessed_at: now.clone(),
        };
        let health = AdapterHealthV1 {
            adapter_id: descriptor.adapter_id.clone(),
            status: AdapterHealthStatus::Healthy,
            checked_at: now,
            exact_reason: "hermetic fixture available".to_owned(),
        };
        let adapter = Self {
            descriptor,
            snapshot,
            challenge,
            package,
            solver_view,
            rights,
            health,
        };
        verify_solver_view(&adapter.package, &adapter.solver_view)?;
        Ok(adapter)
    }
}

impl ChallengeCorpusAdapter for LocalFixtureCorpus {
    fn describe_corpus(&self) -> &CorpusDescriptorV1 {
        &self.descriptor
    }

    fn snapshot_corpus(&self, reference: &str) -> Result<&CorpusSnapshotV1, CorpusError> {
        if reference == self.snapshot.resolved_commit {
            Ok(&self.snapshot)
        } else if matches!(reference, "main" | "master" | "development" | "latest") {
            Err(CorpusError::MutableRef(reference.to_owned()))
        } else {
            Err(CorpusError::UnknownSnapshot(reference.to_owned()))
        }
    }

    fn list_challenges(&self, snapshot_digest: Sha256Digest) -> Result<Vec<String>, CorpusError> {
        if snapshot_digest == self.snapshot.snapshot_digest {
            Ok(vec![self.challenge.source_challenge_ref.clone()])
        } else {
            Err(CorpusError::UnknownSnapshot(snapshot_digest.to_string()))
        }
    }

    fn describe_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengeDescriptorV1, CorpusError> {
        self.materialize_challenge(snapshot_digest, challenge_ref)?;
        Ok(&self.challenge)
    }

    fn materialize_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengePackageV1, CorpusError> {
        if snapshot_digest != self.snapshot.snapshot_digest {
            return Err(CorpusError::UnknownSnapshot(snapshot_digest.to_string()));
        }
        if challenge_ref != self.challenge.source_challenge_ref {
            return Err(CorpusError::UnknownChallenge(challenge_ref.to_owned()));
        }
        Ok(&self.package)
    }

    fn solver_view(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&SolverViewManifestV1, CorpusError> {
        if package_digest == self.package.package_digest {
            Ok(&self.solver_view)
        } else {
            Err(CorpusError::UnknownChallenge(package_digest.to_string()))
        }
    }

    fn assess_license_and_provenance(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&RightsAndProvenanceAssessmentV1, CorpusError> {
        if package_digest == self.package.package_digest {
            Ok(&self.rights)
        } else {
            Err(CorpusError::UnknownChallenge(package_digest.to_string()))
        }
    }

    fn health(&self) -> &AdapterHealthV1 {
        &self.health
    }
}

/// Verify that a solver view cannot reach secret or non-public package artifacts.
///
/// # Errors
///
/// Returns an exposure or unknown-artifact error.
pub fn verify_solver_view(
    package: &ChallengePackageV1,
    view: &SolverViewManifestV1,
) -> Result<(), CorpusError> {
    for digest in &view.exposed_artifact_digests {
        let artifact = package
            .artifacts
            .iter()
            .find(|artifact| artifact.digest == *digest)
            .ok_or(CorpusError::UnknownSolverArtifact(*digest))?;
        if artifact.visibility != PackageVisibility::SolverPublic
            || view.excluded_secret_digests.contains(digest)
        {
            return Err(CorpusError::SecretExposure(*digest));
        }
    }
    for artifact in &package.artifacts {
        if artifact.visibility != PackageVisibility::SolverPublic
            && !view.excluded_secret_digests.contains(&artifact.digest)
        {
            return Err(CorpusError::SecretExposure(artifact.digest));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        "2026-07-10T12:00:00Z".parse()
    }

    #[test]
    fn contract_local_fixture_has_stable_snapshot_and_secret_isolation()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = LocalFixtureCorpus::new(now()?)?;
        let snapshot = adapter.snapshot_corpus(&adapter.describe_corpus().discovery_ref)?;
        let package = adapter.materialize_challenge(snapshot.snapshot_digest, "exact-bytes-v1")?;
        verify_solver_view(package, adapter.solver_view(package.package_digest)?)?;
        assert_eq!(adapter.list_challenges(snapshot.snapshot_digest)?.len(), 1);
        Ok(())
    }

    #[test]
    fn contract_mutable_ref_fails_at_freeze() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = LocalFixtureCorpus::new(now()?)?;
        assert!(matches!(
            adapter.snapshot_corpus("main"),
            Err(CorpusError::MutableRef(_))
        ));
        Ok(())
    }

    #[test]
    fn contract_solver_view_exposure_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = LocalFixtureCorpus::new(now()?)?;
        let mut view = adapter.solver_view.clone();
        let secret = adapter.package.artifacts[1].digest;
        view.exposed_artifact_digests.push(secret);
        assert!(matches!(
            verify_solver_view(&adapter.package, &view),
            Err(CorpusError::SecretExposure(_))
        ));
        Ok(())
    }

    #[test]
    fn contract_seven_required_corpus_profiles_are_pinned_and_data_driven()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        assert_eq!(catalog.profiles.len(), 7);
        let author_profiles = catalog
            .profiles
            .iter()
            .filter(|profile| profile.profile_id.starts_with("public-author-"))
            .collect::<Vec<_>>();
        assert_eq!(author_profiles.len(), 3);
        assert!(
            author_profiles
                .iter()
                .all(|profile| profile.engine == "generic-git-corpus.v1")
        );
        assert!(catalog.profiles.iter().all(|profile| {
            profile.readiness != SettlementReadiness::SettlementReady && profile.commit.len() == 40
        }));
        Ok(())
    }

    #[test]
    fn contract_recorded_manifests_cover_profiles_and_exclude_secrets()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_recorded_corpus_fixtures()?;
        assert_eq!(catalog.fixtures.len(), 7);
        assert!(
            catalog
                .fixtures
                .iter()
                .all(|fixture| { fixture.verifier_status.starts_with("NOT_MATERIALIZED:") })
        );
        Ok(())
    }

    const NYU_PIN: &str = "4c5744f55a31a58e03d6118c9dbb1c18e5bbdfa9";
    const CRYPTOHACK_PIN: &str = "4697b69f51fa33acf9c6d9fd4276ca3fb117af0c";

    fn local_request<'a>(
        checkout_root: &'a Path,
        materialization_root: &'a str,
        limits: LocalMaterializationLimits,
    ) -> LocalMaterializationRequest<'a> {
        LocalMaterializationRequest {
            profile_id: "nyu-ctf-bench",
            checkout_root,
            materialization_root,
            expected_commit: NYU_PIN,
            observed_checkout_commit: NYU_PIN,
            limits,
        }
    }

    fn checked_in_materialization_root() -> Result<PathBuf, std::io::Error> {
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| {
                std::io::Error::other("crate is outside the workspace crates directory")
            })?
            .join("fixtures/corpora/local-materialization"))
    }

    fn write_test_tree(
        root: &Path,
        files: &[(&str, &[u8])],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (relative, bytes) in files {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)?;
        }
        Ok(())
    }

    #[test]
    fn contract_local_materialization_is_deterministic_and_non_settlement_ready()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        let checkout_root = checked_in_materialization_root()?;
        let request = local_request(
            &checkout_root,
            "development/crypto/demo",
            LocalMaterializationLimits::default(),
        );

        let first = materialize_pinned_local_root(&catalog, &request)?;
        let second = materialize_pinned_local_root(&catalog, &request)?;

        assert_eq!(first, second);
        assert_eq!(first.files.len(), 2);
        assert_eq!(
            first
                .files
                .iter()
                .map(|file| file.normalized_path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "development/crypto/demo/README.md",
                "development/crypto/demo/public/input.txt"
            ]
        );
        assert_eq!(
            first.files[0].content_digest.to_hex(),
            "f12290f64d3abecc1dbd5863e043ad17e59074ac56b6c2c624916e18372e280b"
        );
        assert_eq!(
            first.files[1].content_digest.to_hex(),
            "e41ad1ba98a95c59c967afd25a016ba41457fe56a57264597fa52ec4c8806df3"
        );
        assert_eq!(
            first.settlement_readiness,
            SettlementReadiness::EvaluationOnly
        );
        assert!(!first.executed_files);
        assert!(first.files.iter().all(|file| file.byte_length > 0));
        Ok(())
    }

    #[test]
    fn contract_local_materialization_requires_catalog_and_observed_pins()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        let checkout_root = checked_in_materialization_root()?;
        let mut request = local_request(
            &checkout_root,
            "development/crypto/demo",
            LocalMaterializationLimits::default(),
        );
        request.observed_checkout_commit = "0000000000000000000000000000000000000000";
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::CommitMismatch { .. })
        ));
        request.observed_checkout_commit = NYU_PIN;
        request.expected_commit = "0000000000000000000000000000000000000000";
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::CommitMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn contract_local_materialization_rejects_unsafe_and_unallowlisted_roots()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        let checkout_root = checked_in_materialization_root()?;
        for rejected in [
            "/development",
            "development/../test",
            "development\\crypto",
            "development\0crypto",
            "development//crypto",
        ] {
            let request = local_request(
                &checkout_root,
                rejected,
                LocalMaterializationLimits::default(),
            );
            assert!(matches!(
                materialize_pinned_local_root(&catalog, &request),
                Err(CorpusError::UnsafeMaterializationPath { .. })
            ));
        }
        let request = local_request(
            &checkout_root,
            "release",
            LocalMaterializationLimits::default(),
        );
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::UnallowlistedRoot { .. })
        ));
        let relative_checkout = Path::new("fixtures/corpora/local-materialization");
        let request = local_request(
            relative_checkout,
            "development/crypto/demo",
            LocalMaterializationLimits::default(),
        );
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::InvalidCheckoutRoot { .. })
        ));
        Ok(())
    }

    #[test]
    fn contract_local_materialization_never_reads_vcs_control_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(
            checkout.path(),
            &[
                ("public.txt", b"public"),
                (".git/config", b"checkout metadata"),
            ],
        )?;
        let catalog = bundled_corpus_profiles()?;
        let request = LocalMaterializationRequest {
            profile_id: "cryptohack-ctf-archive",
            checkout_root: checkout.path(),
            materialization_root: ".",
            expected_commit: CRYPTOHACK_PIN,
            observed_checkout_commit: CRYPTOHACK_PIN,
            limits: LocalMaterializationLimits::default(),
        };
        let materialization = materialize_pinned_local_root(&catalog, &request)?;
        assert_eq!(materialization.files.len(), 1);
        assert_eq!(materialization.files[0].normalized_path, "public.txt");
        assert_eq!(
            materialization.settlement_readiness,
            SettlementReadiness::EvaluationOnly
        );
        Ok(())
    }

    #[test]
    fn contract_local_materialization_rejects_protected_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(
            checkout.path(),
            &[
                ("development/challenge/public.txt", b"public"),
                ("development/challenge/solution.md", b"secret"),
            ],
        )?;
        let catalog = bundled_corpus_profiles()?;
        let request = local_request(
            checkout.path(),
            "development/challenge",
            LocalMaterializationLimits::default(),
        );
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::UnsafeMaterializationPath { path, .. })
                if path.ends_with("solution.md")
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn contract_local_materialization_rejects_symlinks() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let checkout = tempfile::tempdir()?;
        let outside = tempfile::NamedTempFile::new()?;
        write_test_tree(
            checkout.path(),
            &[("development/challenge/public.txt", b"public")],
        )?;
        symlink(
            outside.path(),
            checkout.path().join("development/challenge/linked.txt"),
        )?;
        let catalog = bundled_corpus_profiles()?;
        let request = local_request(
            checkout.path(),
            "development/challenge",
            LocalMaterializationLimits::default(),
        );
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::UnsafeMaterializationPath { path, reason })
                if path.ends_with("linked.txt") && reason.contains("symlink")
        ));
        Ok(())
    }

    #[test]
    fn contract_local_materialization_enforces_file_and_count_limits()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(
            checkout.path(),
            &[
                ("development/challenge/a.txt", b"four"),
                ("development/challenge/b.txt", b"five"),
            ],
        )?;
        let catalog = bundled_corpus_profiles()?;
        let oversized = local_request(
            checkout.path(),
            "development/challenge",
            LocalMaterializationLimits {
                max_file_count: 2,
                max_file_bytes: 3,
                max_total_bytes: 10,
            },
        );
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &oversized),
            Err(CorpusError::MaterializationLimitExceeded { limit, .. })
                if limit.starts_with("max_file_bytes")
        ));

        let excessive_count = local_request(
            checkout.path(),
            "development/challenge",
            LocalMaterializationLimits {
                max_file_count: 1,
                max_file_bytes: 10,
                max_total_bytes: 10,
            },
        );
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &excessive_count),
            Err(CorpusError::MaterializationLimitExceeded { limit, .. })
                if limit.starts_with("max_file_count")
        ));
        Ok(())
    }
}
