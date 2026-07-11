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
    /// A reviewed challenge overlay violated its hermetic compilation contract.
    #[error("invalid reviewed overlay {overlay_id}: {reason}")]
    InvalidOverlay {
        /// Stable overlay identifier or fixture catalog boundary.
        overlay_id: String,
        /// Exact rejected invariant.
        reason: String,
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
    /// Conservative media type derived from bounded bytes and the normalized path.
    pub media_type: String,
    /// Whether an execute bit was observed on Unix; always false elsewhere.
    pub executable: bool,
}

/// Special checkout object that cannot be treated as ordinary runnable bytes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpecialObjectMetadata {
    /// One declaration parsed from a bounded UTF-8 `.gitmodules` file.
    GitSubmodule {
        /// Human-readable name from the section header.
        name: String,
        /// Normalized checkout-relative submodule path.
        path: Option<String>,
        /// Declared upstream URL, retained as inert text.
        url: Option<String>,
    },
    /// A Git LFS v1 pointer; referenced content was not read.
    GitLfsPointer {
        /// Lowercase SHA-256 object identifier from the pointer.
        oid_sha256: String,
        /// Declared external object length.
        declared_size: u64,
    },
    /// Archive bytes classified by filename without parsing or extraction.
    Archive {
        /// Recognized archive container/compression form.
        format: ArchiveFormat,
    },
    /// Dockerfile/Containerfile base-image declaration retained as inert text.
    OciImageReference {
        /// Exact `FROM` reference, without command execution or resolution.
        reference: String,
        /// Whether the reference is pinned by a lowercase SHA-256 digest.
        digest_pinned: bool,
    },
}

/// Recognized archive formats. Recognition never authorizes extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArchiveFormat {
    /// ZIP container.
    Zip,
    /// Uncompressed tar container.
    Tar,
    /// Gzip-compressed tar container.
    TarGzip,
    /// Bzip2-compressed tar container.
    TarBzip2,
    /// XZ-compressed tar container.
    TarXz,
}

/// Typed reason a special object remains unusable for settlement evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpecialObjectBlockerCode {
    /// Submodule path is absent, unsafe, or not normalized.
    UnsafeSubmodulePath,
    /// Submodule URL is not an inert credential-free HTTPS URL.
    UnsafeSubmoduleUrl,
    /// Submodule working-tree bytes are absent.
    SubmoduleObjectMissing,
    /// Submodule commit identity was not independently verified.
    SubmoduleCommitUnverified,
    /// LFS content is external to the pointer and was not materialized.
    LfsContentNotMaterialized,
    /// Declared LFS size exceeds the materializer's aggregate byte ceiling.
    LfsDeclaredSizeExceedsLimit,
    /// Archive contents were not parsed or extracted.
    ArchiveNotExtracted,
    /// Dockerfile or Containerfile was inspected as metadata only.
    DockerfileMetadataOnly,
    /// OCI image reference is mutable because it lacks a SHA-256 digest pin.
    OciReferenceNotDigestPinned,
}

/// One deterministic special-object inspection record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialObjectInspection {
    /// Checkout-relative file containing the declaration or special bytes.
    pub normalized_path: String,
    /// Typed inert metadata extracted without resolution or execution.
    pub metadata: SpecialObjectMetadata,
    /// Exact typed blockers; every record has at least one.
    pub blockers: Vec<SpecialObjectBlockerCode>,
}

/// Flattened blocker retained in the top-level manifest for admission checks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialObjectReadinessBlocker {
    /// File that caused the fail-closed readiness ceiling.
    pub normalized_path: String,
    /// Machine-readable blocker code.
    pub code: SpecialObjectBlockerCode,
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
    /// Deterministically ordered metadata-only special-object inspections.
    pub special_objects: Vec<SpecialObjectInspection>,
    /// Deterministically ordered blockers derived from special objects.
    pub readiness_blockers: Vec<SpecialObjectReadinessBlocker>,
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
    special_objects: &'a [SpecialObjectInspection],
    readiness_blockers: &'a [SpecialObjectReadinessBlocker],
    settlement_readiness: &'a SettlementReadiness,
    executed_files: bool,
}

const LOCAL_MATERIALIZATION_SCHEMA_VERSION: &str = "hsai.local-corpus-materialization.v3";
const MAX_GITMODULES_BYTES: usize = 64 * 1024;
const MAX_LFS_POINTER_BYTES: usize = 1024;

/// Complete role assigned to one materialized file by a reviewed overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewedArtifactRole {
    /// Bytes exposed to the solver.
    SolverPublic,
    /// Bytes available only to the isolated challenge runtime.
    RuntimePrivate,
    /// Bytes available only to the verifier.
    VerifierOnly,
    /// Organizer material excluded from all solver inputs.
    ContaminationExcluded,
    /// Inert build input; compilation does not execute it.
    BuildInput,
    /// Descriptive metadata that cannot affect resolution semantics.
    DescriptiveMetadata,
}

/// Fail-closed state of a reviewed runtime or verifier declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewedComponentStatus {
    /// Required bytes or configuration are absent.
    NotMaterialized,
    /// Declaration is retained as inert metadata only.
    MetadataOnly,
    /// Review identified a typed or explained blocker.
    Blocked,
}

/// One exact file assignment in a reviewed challenge overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedOverlayArtifact {
    /// Checkout-relative normalized materialized path.
    pub normalized_path: String,
    /// Complete isolation role for this file.
    pub role: ReviewedArtifactRole,
    /// Exact media type expected from the digest-bound local scan.
    pub expected_media_type: String,
    /// Exact execute-bit expectation from the digest-bound local scan.
    pub expected_executable: bool,
}

/// Item-level rights facts supplied by a reviewer without repository promotion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedItemRights {
    /// Attribution strings retained with the challenge package.
    pub upstream_attribution: Vec<String>,
    /// Repository-level license fact, if known.
    pub repository_license: Option<String>,
    /// Challenge/item-level license or permission evidence, if known.
    pub item_license: Option<String>,
    /// Conservative item-level rights status.
    pub rights_status: RightsStatus,
    /// Whether redistribution was explicitly established at item level.
    pub redistribution_allowed: bool,
    /// Whether local execution was explicitly established at item level.
    pub execution_allowed: bool,
    /// Whether commercial market use was explicitly established at item level.
    pub commercial_market_use_allowed: bool,
    /// Exact limitation or missing-evidence statement.
    pub exact_limitation: String,
}

/// Versioned reviewed overlay bound to one local materialization identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedChallengeOverlay {
    /// Overlay schema version.
    pub schema_version: String,
    /// Stable reviewed overlay identifier.
    pub overlay_id: String,
    /// Matching corpus profile.
    pub profile_id: String,
    /// Matching immutable commit.
    pub pinned_commit: String,
    /// Matching allowlisted materialization root.
    pub materialization_root: String,
    /// Exact digest of the source local materialization.
    pub source_materialization_digest: Sha256Digest,
    /// Stable challenge reference within the profile.
    pub challenge_ref: String,
    /// Normalized challenge category.
    pub category: String,
    /// Original upstream category label retained as descriptive metadata.
    pub original_category: String,
    /// Complete one-to-one partition of every materialized file.
    pub artifacts: Vec<ReviewedOverlayArtifact>,
    /// Runtime declaration ceiling.
    pub runtime_status: ReviewedComponentStatus,
    /// Exact runtime blocker or metadata-only explanation.
    pub runtime_limitation: String,
    /// Verifier declaration ceiling.
    pub verifier_status: ReviewedComponentStatus,
    /// Exact verifier blocker or metadata-only explanation.
    pub verifier_limitation: String,
    /// Item-level rights facts.
    pub rights: ReviewedItemRights,
    /// Structural evaluation/catalog ceiling.
    pub readiness: SettlementReadiness,
}

/// Bundled sanitized reviewed-overlay fixture catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedOverlayCatalog {
    /// Catalog schema version.
    pub schema_version: String,
    /// Sanitized reviewed overlays.
    pub overlays: Vec<ReviewedChallengeOverlay>,
}

/// Digest-bound artifact emitted by reviewed-overlay compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledReviewedArtifact {
    /// Checkout-relative normalized path.
    pub normalized_path: String,
    /// Source byte digest.
    pub content_digest: Sha256Digest,
    /// Exact byte length.
    pub byte_length: u64,
    /// Conservative detected media type.
    pub media_type: String,
    /// Observed execute-bit state.
    pub executable: bool,
    /// Reviewed isolation role.
    pub role: ReviewedArtifactRole,
}

/// Solver-visible subset and excluded digests for a compiled overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledReviewedSolverView {
    /// Solver-view schema version.
    pub schema_version: String,
    /// Source materialization identity.
    pub source_materialization_digest: Sha256Digest,
    /// Solver-visible artifacts in deterministic path order.
    pub exposed_artifacts: Vec<CompiledReviewedArtifact>,
    /// All non-solver artifact digests in deterministic path order.
    pub excluded_artifact_digests: Vec<Sha256Digest>,
    /// Canonical solver-view digest.
    pub manifest_digest: Sha256Digest,
}

/// Hermetic evaluation package produced from a reviewed overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledReviewedChallengePackage {
    /// Package schema version.
    pub schema_version: String,
    /// Reviewed overlay identifier.
    pub overlay_id: String,
    /// Canonical digest of the reviewed overlay input.
    pub overlay_digest: Sha256Digest,
    /// Source local materialization identity.
    pub source_materialization_digest: Sha256Digest,
    /// Stable challenge reference.
    pub challenge_ref: String,
    /// Normalized category.
    pub category: String,
    /// Original upstream category label.
    pub original_category: String,
    /// Completely partitioned artifact list.
    pub artifacts: Vec<CompiledReviewedArtifact>,
    /// Digest-bound solver-view subset.
    pub solver_view: CompiledReviewedSolverView,
    /// Inherited unresolved special-object blockers.
    pub inherited_special_blockers: Vec<SpecialObjectReadinessBlocker>,
    /// Runtime status; compilation never runs it.
    pub runtime_status: ReviewedComponentStatus,
    /// Exact runtime limitation.
    pub runtime_limitation: String,
    /// Verifier status; compilation never runs it.
    pub verifier_status: ReviewedComponentStatus,
    /// Exact verifier limitation.
    pub verifier_limitation: String,
    /// Conservative item-level rights facts.
    pub rights: ReviewedItemRights,
    /// Canonical rights record digest.
    pub rights_digest: Sha256Digest,
    /// Structural evaluation/catalog ceiling.
    pub readiness: SettlementReadiness,
    /// Canonical package digest.
    pub package_digest: Sha256Digest,
}

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

    let mut scan = LocalMaterializationScan {
        checkout_root: request.checkout_root,
        profile,
        limits: request.limits,
        files: Vec::new(),
        total_bytes: 0,
        special_objects: Vec::new(),
    };
    scan_materialization_directory(&selected_root, &relative_root, &mut scan)?;
    let LocalMaterializationScan {
        mut files,
        total_bytes,
        mut special_objects,
        ..
    } = scan;
    files.sort_by(|left, right| left.normalized_path.cmp(&right.normalized_path));
    special_objects.sort();
    let mut readiness_blockers = special_objects
        .iter()
        .flat_map(|object| {
            object
                .blockers
                .iter()
                .map(|code| SpecialObjectReadinessBlocker {
                    normalized_path: object.normalized_path.clone(),
                    code: *code,
                })
        })
        .collect::<Vec<_>>();
    readiness_blockers.sort();
    readiness_blockers.dedup();
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
        special_objects: &special_objects,
        readiness_blockers: &readiness_blockers,
        settlement_readiness: &profile.readiness,
        executed_files: false,
    };
    let manifest_digest = domain_separated_digest("hsai.local-corpus-materialization.v3", &body)?;
    Ok(LocalCorpusMaterialization {
        schema_version: LOCAL_MATERIALIZATION_SCHEMA_VERSION.to_owned(),
        profile_id: profile.profile_id.clone(),
        pinned_commit: profile.commit.clone(),
        materialization_root: request.materialization_root.to_owned(),
        files,
        total_bytes,
        special_objects,
        readiness_blockers,
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
struct LocalMaterializationScan<'a> {
    checkout_root: &'a Path,
    profile: &'a CorpusProfile,
    limits: LocalMaterializationLimits,
    files: Vec<LocalMaterializedFile>,
    total_bytes: u64,
    special_objects: Vec<SpecialObjectInspection>,
}

fn scan_materialization_directory(
    directory: &Path,
    relative_directory: &Path,
    scan: &mut LocalMaterializationScan<'_>,
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
        if protected_materialization_path(scan.profile, &normalized_path) {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path,
                reason: "protected, secret, solution, or flag path is forbidden".to_owned(),
            });
        }
        if metadata.is_dir() {
            scan_materialization_directory(&absolute_path, &relative_path, scan)?;
        } else if metadata.is_file() {
            scan_regular_materialization_file(&absolute_path, normalized_path, &metadata, scan)?;
        } else {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path,
                reason: "only regular files and directories are allowed".to_owned(),
            });
        }
    }
    Ok(())
}

fn scan_regular_materialization_file(
    absolute_path: &Path,
    normalized_path: String,
    metadata: &fs::Metadata,
    scan: &mut LocalMaterializationScan<'_>,
) -> Result<(), CorpusError> {
    if scan.files.len() >= scan.limits.max_file_count {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: format!("max_file_count={}", scan.limits.max_file_count),
            path: normalized_path,
        });
    }
    if metadata.len() > scan.limits.max_file_bytes {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: format!("max_file_bytes={}", scan.limits.max_file_bytes),
            path: normalized_path,
        });
    }
    let next_total = scan
        .total_bytes
        .checked_add(metadata.len())
        .ok_or_else(|| CorpusError::MaterializationLimitExceeded {
            limit: format!("max_total_bytes={}", scan.limits.max_total_bytes),
            path: normalized_path.clone(),
        })?;
    if next_total > scan.limits.max_total_bytes {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: format!("max_total_bytes={}", scan.limits.max_total_bytes),
            path: normalized_path,
        });
    }
    let bytes = read_bounded_regular_file(
        absolute_path,
        metadata,
        scan.limits.max_file_bytes,
        &normalized_path,
    )?;
    inspect_special_file(
        scan.checkout_root,
        &normalized_path,
        &bytes,
        scan.limits,
        &mut scan.special_objects,
    )?;
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    let media_type = conservative_media_type(&normalized_path, &bytes);
    let executable = observed_executable(metadata);
    scan.files.push(LocalMaterializedFile {
        normalized_path,
        content_digest: Sha256Digest::from_bytes(digest),
        byte_length: metadata.len(),
        media_type,
        executable,
    });
    scan.total_bytes = next_total;
    Ok(())
}

fn conservative_media_type(normalized_path: &str, bytes: &[u8]) -> String {
    let filename = normalized_path
        .rsplit('/')
        .next()
        .unwrap_or(normalized_path);
    let lower = filename.to_ascii_lowercase();
    if bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        return "application/vnd.git-lfs.pointer".to_owned();
    }
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        return "application/zip".to_owned();
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return "application/gzip".to_owned();
    }
    if bytes.starts_with(b"BZh") {
        return "application/x-bzip2".to_owned();
    }
    if bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        return "application/x-xz".to_owned();
    }
    if bytes.len() > 262 && bytes.get(257..262) == Some(b"ustar") {
        return "application/x-tar".to_owned();
    }
    if std::str::from_utf8(bytes).is_err() {
        return "application/octet-stream".to_owned();
    }
    if dockerfile_name(filename) {
        "text/x-dockerfile".to_owned()
    } else {
        match lower.rsplit('.').next() {
            Some("md" | "markdown") => "text/markdown".to_owned(),
            Some("json") if serde_json::from_slice::<serde_json::Value>(bytes).is_ok() => {
                "application/json".to_owned()
            }
            Some("yaml" | "yml") => "application/yaml".to_owned(),
            Some("toml") => "application/toml".to_owned(),
            _ => "text/plain".to_owned(),
        }
    }
}

#[cfg(unix)]
fn observed_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn observed_executable(_metadata: &fs::Metadata) -> bool {
    false
}

fn inspect_special_file(
    checkout_root: &Path,
    normalized_path: &str,
    bytes: &[u8],
    limits: LocalMaterializationLimits,
    special_objects: &mut Vec<SpecialObjectInspection>,
) -> Result<(), CorpusError> {
    let filename = normalized_path
        .rsplit('/')
        .next()
        .unwrap_or(normalized_path);
    if filename == ".gitmodules" {
        special_objects.extend(inspect_gitmodules(checkout_root, normalized_path, bytes)?);
        return Ok(());
    }
    if bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        special_objects.push(inspect_lfs_pointer(normalized_path, bytes, limits)?);
        return Ok(());
    }
    if let Some(format) = archive_format(filename) {
        special_objects.push(SpecialObjectInspection {
            normalized_path: normalized_path.to_owned(),
            metadata: SpecialObjectMetadata::Archive { format },
            blockers: vec![SpecialObjectBlockerCode::ArchiveNotExtracted],
        });
        return Ok(());
    }
    if dockerfile_name(filename) {
        special_objects.extend(inspect_dockerfile(normalized_path, bytes)?);
    }
    Ok(())
}

#[derive(Default)]
struct ParsedSubmodule {
    name: String,
    path: Option<String>,
    url: Option<String>,
}

fn inspect_gitmodules(
    checkout_root: &Path,
    normalized_path: &str,
    bytes: &[u8],
) -> Result<Vec<SpecialObjectInspection>, CorpusError> {
    if bytes.len() > MAX_GITMODULES_BYTES {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: format!("max_gitmodules_bytes={MAX_GITMODULES_BYTES}"),
            path: normalized_path.to_owned(),
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| CorpusError::UnsafeMaterializationPath {
        path: normalized_path.to_owned(),
        reason: ".gitmodules must be valid UTF-8".to_owned(),
    })?;
    let mut parsed = Vec::new();
    let mut current: Option<ParsedSubmodule> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if let Some(section) = current.take() {
                parsed.push(section);
            }
            let Some(name) = line
                .strip_prefix("[submodule \"")
                .and_then(|value| value.strip_suffix("\"]"))
            else {
                return Err(CorpusError::UnsafeMaterializationPath {
                    path: normalized_path.to_owned(),
                    reason: "unsupported or malformed .gitmodules section".to_owned(),
                });
            };
            if name.is_empty() || name.chars().any(char::is_control) {
                return Err(CorpusError::UnsafeMaterializationPath {
                    path: normalized_path.to_owned(),
                    reason: "submodule name is empty or contains control characters".to_owned(),
                });
            }
            current = Some(ParsedSubmodule {
                name: name.to_owned(),
                ..ParsedSubmodule::default()
            });
            continue;
        }
        let Some(section) = current.as_mut() else {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path.to_owned(),
                reason: ".gitmodules key appears outside a submodule section".to_owned(),
            });
        };
        let Some((key, value)) = line.split_once('=') else {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path.to_owned(),
                reason: "malformed .gitmodules key/value line".to_owned(),
            });
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path.to_owned(),
                reason: ".gitmodules value is empty or contains control characters".to_owned(),
            });
        }
        match key {
            "path" if section.path.is_none() => section.path = Some(value.to_owned()),
            "url" if section.url.is_none() => section.url = Some(value.to_owned()),
            "branch" | "update" | "ignore" | "shallow" => {}
            "path" | "url" => {
                return Err(CorpusError::UnsafeMaterializationPath {
                    path: normalized_path.to_owned(),
                    reason: format!("duplicate submodule {key} key"),
                });
            }
            _ => {
                return Err(CorpusError::UnsafeMaterializationPath {
                    path: normalized_path.to_owned(),
                    reason: format!("unsupported .gitmodules key {key}"),
                });
            }
        }
    }
    if let Some(section) = current {
        parsed.push(section);
    }
    if parsed.is_empty() {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: ".gitmodules contains no submodule declarations".to_owned(),
        });
    }

    Ok(parsed
        .into_iter()
        .map(|section| submodule_inspection(checkout_root, normalized_path, section))
        .collect())
}

fn submodule_inspection(
    checkout_root: &Path,
    normalized_path: &str,
    section: ParsedSubmodule,
) -> SpecialObjectInspection {
    let mut blockers = Vec::new();
    let safe_path = section
        .path
        .as_deref()
        .is_some_and(|path| validate_normalized_relative_path(path, false).is_ok());
    if !safe_path {
        blockers.push(SpecialObjectBlockerCode::UnsafeSubmodulePath);
    }
    if !section.url.as_deref().is_some_and(safe_submodule_url) {
        blockers.push(SpecialObjectBlockerCode::UnsafeSubmoduleUrl);
    }
    let object_present = section.path.as_deref().is_some_and(|path| {
        safe_path
            && fs::symlink_metadata(checkout_root.join(path))
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
    });
    if !object_present {
        blockers.push(SpecialObjectBlockerCode::SubmoduleObjectMissing);
    }
    blockers.push(SpecialObjectBlockerCode::SubmoduleCommitUnverified);
    blockers.sort();
    blockers.dedup();
    SpecialObjectInspection {
        normalized_path: normalized_path.to_owned(),
        metadata: SpecialObjectMetadata::GitSubmodule {
            name: section.name,
            path: section.path,
            url: section.url,
        },
        blockers,
    }
}

fn safe_submodule_url(url: &str) -> bool {
    let Some(authority_and_path) = url.strip_prefix("https://") else {
        return false;
    };
    let Some((authority, path)) = authority_and_path.split_once('/') else {
        return false;
    };
    !authority.is_empty()
        && !path.is_empty()
        && !authority.contains('@')
        && !url.contains(['?', '#', '\\'])
        && !url.chars().any(char::is_whitespace)
        && !url.split('/').any(|component| component == "..")
}

fn inspect_lfs_pointer(
    normalized_path: &str,
    bytes: &[u8],
    limits: LocalMaterializationLimits,
) -> Result<SpecialObjectInspection, CorpusError> {
    if bytes.len() > MAX_LFS_POINTER_BYTES {
        return Err(CorpusError::MaterializationLimitExceeded {
            limit: format!("max_lfs_pointer_bytes={MAX_LFS_POINTER_BYTES}"),
            path: normalized_path.to_owned(),
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| CorpusError::UnsafeMaterializationPath {
        path: normalized_path.to_owned(),
        reason: "Git LFS pointer must be valid UTF-8".to_owned(),
    })?;
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 3 || lines[0] != "version https://git-lfs.github.com/spec/v1" {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "unsupported or malformed Git LFS pointer".to_owned(),
        });
    }
    let oid_sha256 = lines[1]
        .strip_prefix("oid sha256:")
        .filter(|oid| {
            oid.len() == 64
                && oid
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
        .ok_or_else(|| CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "Git LFS oid must be lowercase SHA-256".to_owned(),
        })?
        .to_owned();
    let declared_size = lines[2]
        .strip_prefix("size ")
        .and_then(|size| size.parse::<u64>().ok())
        .ok_or_else(|| CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "Git LFS size must be an unsigned decimal integer".to_owned(),
        })?;
    let mut blockers = vec![SpecialObjectBlockerCode::LfsContentNotMaterialized];
    if declared_size > limits.max_total_bytes {
        blockers.push(SpecialObjectBlockerCode::LfsDeclaredSizeExceedsLimit);
    }
    Ok(SpecialObjectInspection {
        normalized_path: normalized_path.to_owned(),
        metadata: SpecialObjectMetadata::GitLfsPointer {
            oid_sha256,
            declared_size,
        },
        blockers,
    })
}

fn archive_format(filename: &str) -> Option<ArchiveFormat> {
    let lower = filename.to_ascii_lowercase();
    let reversed_components = lower.rsplit('.').collect::<Vec<_>>();
    match reversed_components.as_slice() {
        ["zip", ..] => Some(ArchiveFormat::Zip),
        ["tar", ..] => Some(ArchiveFormat::Tar),
        ["tgz", ..] | ["gz", "tar", ..] => Some(ArchiveFormat::TarGzip),
        ["tbz2", ..] | ["bz2", "tar", ..] => Some(ArchiveFormat::TarBzip2),
        ["txz", ..] | ["xz", "tar", ..] => Some(ArchiveFormat::TarXz),
        _ => None,
    }
}

fn dockerfile_name(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    matches!(lower.as_str(), "dockerfile" | "containerfile")
        || lower.starts_with("dockerfile.")
        || lower.starts_with("containerfile.")
}

fn inspect_dockerfile(
    normalized_path: &str,
    bytes: &[u8],
) -> Result<Vec<SpecialObjectInspection>, CorpusError> {
    let text = std::str::from_utf8(bytes).map_err(|_| CorpusError::UnsafeMaterializationPath {
        path: normalized_path.to_owned(),
        reason: "Dockerfile metadata must be valid UTF-8".to_owned(),
    })?;
    if text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "Dockerfile metadata contains unsupported control characters".to_owned(),
        });
    }
    let mut records = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        let Some((instruction, arguments)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !instruction.eq_ignore_ascii_case("FROM") {
            continue;
        }
        if arguments.contains('\\') {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path.to_owned(),
                reason: "continued Dockerfile FROM instructions are unsupported".to_owned(),
            });
        }
        let reference = arguments
            .split_whitespace()
            .find(|token| !token.starts_with("--"))
            .ok_or_else(|| CorpusError::UnsafeMaterializationPath {
                path: normalized_path.to_owned(),
                reason: "Dockerfile FROM instruction has no image reference".to_owned(),
            })?;
        if reference.contains(['$', '\0']) {
            return Err(CorpusError::UnsafeMaterializationPath {
                path: normalized_path.to_owned(),
                reason: "dynamic Dockerfile FROM references are unsupported".to_owned(),
            });
        }
        let digest_pinned = digest_pinned_oci_reference(reference);
        let mut blockers = vec![SpecialObjectBlockerCode::DockerfileMetadataOnly];
        if reference != "scratch" && !digest_pinned {
            blockers.push(SpecialObjectBlockerCode::OciReferenceNotDigestPinned);
        }
        records.push(SpecialObjectInspection {
            normalized_path: normalized_path.to_owned(),
            metadata: SpecialObjectMetadata::OciImageReference {
                reference: reference.to_owned(),
                digest_pinned,
            },
            blockers,
        });
    }
    if records.is_empty() {
        return Err(CorpusError::UnsafeMaterializationPath {
            path: normalized_path.to_owned(),
            reason: "Dockerfile contains no statically inspectable FROM instruction".to_owned(),
        });
    }
    Ok(records)
}

fn digest_pinned_oci_reference(reference: &str) -> bool {
    reference
        .rsplit_once("@sha256:")
        .is_some_and(|(name, digest)| {
            !name.is_empty()
                && digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
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
    if path.chars().any(char::is_control) {
        return Err("control characters are forbidden".to_owned());
    }
    if path.split('/').any(|component| {
        component.is_empty() || matches!(component, "." | "..") || component.trim() != component
    }) {
        return Err(
            "path must be normalized without empty, dot, parent, or edge-whitespace components"
                .to_owned(),
        );
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

/// Load the sanitized reviewed-overlay fixture catalog.
///
/// # Errors
///
/// Returns an error for invalid JSON, an unsupported schema, duplicate or
/// empty overlay IDs, or an empty fixture catalog. Loading is compile-time
/// local and performs no network or filesystem discovery.
pub fn bundled_reviewed_overlay_fixtures() -> Result<ReviewedOverlayCatalog, CorpusError> {
    let catalog: ReviewedOverlayCatalog = serde_json::from_str(include_str!(
        "../../../fixtures/corpora/recorded-challenge-overlays.json"
    ))?;
    if catalog.schema_version != "hsai.reviewed-overlay-catalog.v1" || catalog.overlays.is_empty() {
        return Err(CorpusError::InvalidOverlay {
            overlay_id: "fixture-catalog".to_owned(),
            reason: "unsupported schema version or empty catalog".to_owned(),
        });
    }
    let mut ids = std::collections::BTreeSet::new();
    if catalog
        .overlays
        .iter()
        .any(|overlay| overlay.overlay_id.is_empty() || !ids.insert(overlay.overlay_id.as_str()))
    {
        return Err(CorpusError::InvalidOverlay {
            overlay_id: "fixture-catalog".to_owned(),
            reason: "overlay IDs must be non-empty and unique".to_owned(),
        });
    }
    Ok(catalog)
}

#[derive(Serialize)]
struct ReviewedSolverViewBody<'a> {
    schema_version: &'a str,
    source_materialization_digest: Sha256Digest,
    exposed_artifacts: &'a [CompiledReviewedArtifact],
    excluded_artifact_digests: &'a [Sha256Digest],
}

#[derive(Serialize)]
struct CompiledReviewedPackageBody<'a> {
    schema_version: &'a str,
    overlay_id: &'a str,
    overlay_digest: Sha256Digest,
    source_materialization_digest: Sha256Digest,
    challenge_ref: &'a str,
    category: &'a str,
    original_category: &'a str,
    artifacts: &'a [CompiledReviewedArtifact],
    solver_view_manifest_digest: Sha256Digest,
    inherited_special_blockers: &'a [SpecialObjectReadinessBlocker],
    runtime_status: ReviewedComponentStatus,
    runtime_limitation: &'a str,
    verifier_status: ReviewedComponentStatus,
    verifier_limitation: &'a str,
    rights_digest: Sha256Digest,
    readiness: SettlementReadiness,
}

/// Compile a complete reviewed overlay against one digest-bound local scan.
///
/// Compilation is pure validation and canonical hashing. It performs no
/// network access, object retrieval, extraction, execution, or readiness
/// promotion.
///
/// # Errors
///
/// Returns an error for identity disagreement, incomplete or overlapping path
/// assignment, solver-view exposure, media/executable mismatch, unsafe rights
/// promotion, invalid component limitations, or a readiness value outside the
/// evaluation/catalog ceiling.
pub fn compile_reviewed_overlay(
    materialization: &LocalCorpusMaterialization,
    overlay: &ReviewedChallengeOverlay,
) -> Result<CompiledReviewedChallengePackage, CorpusError> {
    validate_overlay_identity(materialization, overlay)?;
    validate_reviewed_rights(overlay)?;
    validate_overlay_status(materialization, overlay)?;
    let compiled = compile_reviewed_artifacts(materialization, overlay)?;
    let solver_view = compile_reviewed_solver_view(materialization, overlay, &compiled)?;
    let overlay_digest = domain_separated_digest("hsai.reviewed-challenge-overlay.v1", overlay)?;
    let rights_digest = domain_separated_digest("hsai.reviewed-item-rights.v1", &overlay.rights)?;
    let body = CompiledReviewedPackageBody {
        schema_version: "hsai.compiled-reviewed-challenge-package.v1",
        overlay_id: &overlay.overlay_id,
        overlay_digest,
        source_materialization_digest: materialization.manifest_digest,
        challenge_ref: &overlay.challenge_ref,
        category: &overlay.category,
        original_category: &overlay.original_category,
        artifacts: &compiled,
        solver_view_manifest_digest: solver_view.manifest_digest,
        inherited_special_blockers: &materialization.readiness_blockers,
        runtime_status: overlay.runtime_status,
        runtime_limitation: &overlay.runtime_limitation,
        verifier_status: overlay.verifier_status,
        verifier_limitation: &overlay.verifier_limitation,
        rights_digest,
        readiness: overlay.readiness,
    };
    let package_digest =
        domain_separated_digest("hsai.compiled-reviewed-challenge-package.v1", &body)?;
    Ok(CompiledReviewedChallengePackage {
        schema_version: "hsai.compiled-reviewed-challenge-package.v1".to_owned(),
        overlay_id: overlay.overlay_id.clone(),
        overlay_digest,
        source_materialization_digest: materialization.manifest_digest,
        challenge_ref: overlay.challenge_ref.clone(),
        category: overlay.category.clone(),
        original_category: overlay.original_category.clone(),
        artifacts: compiled,
        solver_view,
        inherited_special_blockers: materialization.readiness_blockers.clone(),
        runtime_status: overlay.runtime_status,
        runtime_limitation: overlay.runtime_limitation.clone(),
        verifier_status: overlay.verifier_status,
        verifier_limitation: overlay.verifier_limitation.clone(),
        rights: overlay.rights.clone(),
        rights_digest,
        readiness: overlay.readiness,
        package_digest,
    })
}

fn compile_reviewed_artifacts(
    materialization: &LocalCorpusMaterialization,
    overlay: &ReviewedChallengeOverlay,
) -> Result<Vec<CompiledReviewedArtifact>, CorpusError> {
    let mut assignments = BTreeMap::new();
    for assignment in &overlay.artifacts {
        validate_normalized_relative_path(&assignment.normalized_path, false).map_err(
            |reason| {
                overlay_error(
                    overlay,
                    format!(
                        "unsafe artifact path {}: {reason}",
                        assignment.normalized_path
                    ),
                )
            },
        )?;
        if assignments
            .insert(assignment.normalized_path.as_str(), assignment)
            .is_some()
        {
            return Err(overlay_error(
                overlay,
                format!(
                    "duplicate artifact assignment {}",
                    assignment.normalized_path
                ),
            ));
        }
    }
    if assignments.len() != materialization.files.len() {
        return Err(overlay_error(
            overlay,
            "overlay must classify every materialized file exactly once".to_owned(),
        ));
    }

    let mut compiled = Vec::with_capacity(materialization.files.len());
    for file in &materialization.files {
        let assignment = assignments
            .get(file.normalized_path.as_str())
            .ok_or_else(|| {
                overlay_error(
                    overlay,
                    format!(
                        "materialized path is unclassified: {}",
                        file.normalized_path
                    ),
                )
            })?;
        validate_overlay_artifact(file, assignment, overlay)?;
        compiled.push(CompiledReviewedArtifact {
            normalized_path: file.normalized_path.clone(),
            content_digest: file.content_digest,
            byte_length: file.byte_length,
            media_type: file.media_type.clone(),
            executable: file.executable,
            role: assignment.role,
        });
    }
    compiled.sort_by(|left, right| left.normalized_path.cmp(&right.normalized_path));
    if assignments.keys().any(|path| {
        materialization
            .files
            .iter()
            .all(|file| file.normalized_path != *path)
    }) {
        return Err(overlay_error(
            overlay,
            "overlay assigns a path absent from the materialization".to_owned(),
        ));
    }

    Ok(compiled)
}

fn validate_overlay_identity(
    materialization: &LocalCorpusMaterialization,
    overlay: &ReviewedChallengeOverlay,
) -> Result<(), CorpusError> {
    let body = LocalCorpusMaterializationBody {
        schema_version: &materialization.schema_version,
        profile_id: &materialization.profile_id,
        pinned_commit: &materialization.pinned_commit,
        materialization_root: &materialization.materialization_root,
        files: &materialization.files,
        total_bytes: materialization.total_bytes,
        special_objects: &materialization.special_objects,
        readiness_blockers: &materialization.readiness_blockers,
        settlement_readiness: &materialization.settlement_readiness,
        executed_files: materialization.executed_files,
    };
    let recomputed = domain_separated_digest("hsai.local-corpus-materialization.v3", &body)?;
    let reason = if overlay.schema_version != "hsai.reviewed-challenge-overlay.v1" {
        Some("unsupported overlay schema version")
    } else if materialization.schema_version != LOCAL_MATERIALIZATION_SCHEMA_VERSION {
        Some("unsupported source materialization schema version")
    } else if overlay.overlay_id.is_empty()
        || overlay.challenge_ref.is_empty()
        || overlay.category.is_empty()
        || overlay.original_category.is_empty()
    {
        Some("overlay identity and taxonomy fields must be non-empty")
    } else if overlay.profile_id != materialization.profile_id {
        Some("profile ID does not match source materialization")
    } else if overlay.pinned_commit != materialization.pinned_commit {
        Some("pinned commit does not match source materialization")
    } else if overlay.materialization_root != materialization.materialization_root {
        Some("materialization root does not match source materialization")
    } else if overlay.source_materialization_digest != materialization.manifest_digest {
        Some("source materialization digest does not match")
    } else if recomputed != materialization.manifest_digest {
        Some("source materialization body does not match its digest")
    } else if materialization.executed_files {
        Some("executed source materializations are forbidden")
    } else {
        None
    };
    reason.map_or(Ok(()), |reason| {
        Err(overlay_error(overlay, reason.to_owned()))
    })
}

fn validate_overlay_status(
    materialization: &LocalCorpusMaterialization,
    overlay: &ReviewedChallengeOverlay,
) -> Result<(), CorpusError> {
    if !matches!(
        overlay.readiness,
        SettlementReadiness::EvaluationOnly | SettlementReadiness::CatalogOnly
    ) {
        return Err(overlay_error(
            overlay,
            "reviewed overlays are structurally limited to EVALUATION_ONLY or CATALOG_ONLY"
                .to_owned(),
        ));
    }
    if !matches!(
        materialization.settlement_readiness,
        SettlementReadiness::EvaluationOnly | SettlementReadiness::CatalogOnly
    ) || (materialization.settlement_readiness == SettlementReadiness::CatalogOnly
        && overlay.readiness != SettlementReadiness::CatalogOnly)
    {
        return Err(overlay_error(
            overlay,
            "overlay cannot promote the source materialization readiness ceiling".to_owned(),
        ));
    }
    if overlay.runtime_limitation.trim().is_empty() || overlay.verifier_limitation.trim().is_empty()
    {
        return Err(overlay_error(
            overlay,
            "runtime and verifier limitations must be explicit".to_owned(),
        ));
    }
    Ok(())
}

fn validate_reviewed_rights(overlay: &ReviewedChallengeOverlay) -> Result<(), CorpusError> {
    let rights = &overlay.rights;
    if rights.upstream_attribution.is_empty()
        || rights
            .upstream_attribution
            .iter()
            .any(|attribution| attribution.trim().is_empty())
        || rights.exact_limitation.trim().is_empty()
        || rights.item_license.as_deref().is_some_and(str::is_empty)
    {
        return Err(overlay_error(
            overlay,
            "item rights require attribution and an exact non-empty limitation".to_owned(),
        ));
    }
    if rights.item_license.is_none()
        && (rights.rights_status == RightsStatus::Permitted
            || rights.redistribution_allowed
            || rights.execution_allowed
            || rights.commercial_market_use_allowed)
    {
        return Err(overlay_error(
            overlay,
            "repository license cannot promote missing item-level permission".to_owned(),
        ));
    }
    Ok(())
}

fn validate_overlay_artifact(
    file: &LocalMaterializedFile,
    assignment: &ReviewedOverlayArtifact,
    overlay: &ReviewedChallengeOverlay,
) -> Result<(), CorpusError> {
    if assignment.expected_media_type != file.media_type {
        return Err(overlay_error(
            overlay,
            format!(
                "media type mismatch for {}: expected {}, observed {}",
                file.normalized_path, assignment.expected_media_type, file.media_type
            ),
        ));
    }
    if assignment.expected_executable != file.executable {
        return Err(overlay_error(
            overlay,
            format!("executable-bit mismatch for {}", file.normalized_path),
        ));
    }
    if assignment.role == ReviewedArtifactRole::SolverPublic
        && solver_sensitive_path(&file.normalized_path)
    {
        return Err(overlay_error(
            overlay,
            format!(
                "solver-public role exposes sensitive path {}",
                file.normalized_path
            ),
        ));
    }
    Ok(())
}

fn compile_reviewed_solver_view(
    materialization: &LocalCorpusMaterialization,
    overlay: &ReviewedChallengeOverlay,
    artifacts: &[CompiledReviewedArtifact],
) -> Result<CompiledReviewedSolverView, CorpusError> {
    let exposed_artifacts = artifacts
        .iter()
        .filter(|artifact| artifact.role == ReviewedArtifactRole::SolverPublic)
        .cloned()
        .collect::<Vec<_>>();
    if exposed_artifacts.is_empty() {
        return Err(overlay_error(
            overlay,
            "solver view must expose at least one reviewed public artifact".to_owned(),
        ));
    }
    let excluded_artifact_digests = artifacts
        .iter()
        .filter(|artifact| artifact.role != ReviewedArtifactRole::SolverPublic)
        .map(|artifact| artifact.content_digest)
        .collect::<Vec<_>>();
    let exposed_digests = exposed_artifacts
        .iter()
        .map(|artifact| artifact.content_digest)
        .collect::<std::collections::HashSet<_>>();
    if excluded_artifact_digests
        .iter()
        .any(|digest| exposed_digests.contains(digest))
    {
        return Err(overlay_error(
            overlay,
            "solver-visible content digest overlaps an excluded artifact".to_owned(),
        ));
    }
    let body = ReviewedSolverViewBody {
        schema_version: "hsai.compiled-reviewed-solver-view.v1",
        source_materialization_digest: materialization.manifest_digest,
        exposed_artifacts: &exposed_artifacts,
        excluded_artifact_digests: &excluded_artifact_digests,
    };
    let manifest_digest = domain_separated_digest("hsai.compiled-reviewed-solver-view.v1", &body)?;
    Ok(CompiledReviewedSolverView {
        schema_version: "hsai.compiled-reviewed-solver-view.v1".to_owned(),
        source_materialization_digest: materialization.manifest_digest,
        exposed_artifacts,
        excluded_artifact_digests,
        manifest_digest,
    })
}

fn solver_sensitive_path(path: &str) -> bool {
    path.split('/').any(|component| {
        let lower = component.to_ascii_lowercase();
        let stem = lower.split('.').next().unwrap_or(&lower);
        matches!(lower.as_str(), "private" | "verifier" | "server_files")
            || matches!(
                stem,
                "flag" | "solution" | "solutions" | "solve" | "writeup" | "writeups"
            )
            || stem.starts_with("flag_")
            || stem.starts_with("solution_")
            || stem.starts_with("writeup_")
    })
}

fn overlay_error(overlay: &ReviewedChallengeOverlay, reason: String) -> CorpusError {
    CorpusError::InvalidOverlay {
        overlay_id: overlay.overlay_id.clone(),
        reason,
    }
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

    fn checked_in_reviewed_overlay_inputs()
    -> Result<(LocalCorpusMaterialization, ReviewedChallengeOverlay), Box<dyn std::error::Error>>
    {
        let profiles = bundled_corpus_profiles()?;
        let root = checked_in_materialization_root()?;
        let request = local_request(
            &root,
            "development/special-object-inspection",
            LocalMaterializationLimits::default(),
        );
        let materialization = materialize_pinned_local_root(&profiles, &request)?;
        let overlay = bundled_reviewed_overlay_fixtures()?
            .overlays
            .into_iter()
            .next()
            .ok_or("reviewed overlay fixture is absent")?;
        Ok((materialization, overlay))
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

    #[test]
    fn contract_special_objects_are_deterministic_metadata_only_blockers()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        let checkout_root = checked_in_materialization_root()?;
        let request = local_request(
            &checkout_root,
            "development/special-object-inspection",
            LocalMaterializationLimits::default(),
        );

        let first = materialize_pinned_local_root(&catalog, &request)?;
        let second = materialize_pinned_local_root(&catalog, &request)?;
        assert_eq!(first, second);
        assert_eq!(first.schema_version, "hsai.local-corpus-materialization.v3");
        assert_eq!(first.special_objects.len(), 4);
        assert_eq!(first.readiness_blockers.len(), 5);
        assert_eq!(
            first.settlement_readiness,
            SettlementReadiness::EvaluationOnly
        );
        assert!(!first.executed_files);
        assert_eq!(
            first
                .readiness_blockers
                .iter()
                .map(|blocker| blocker.code)
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from([
                SpecialObjectBlockerCode::ArchiveNotExtracted,
                SpecialObjectBlockerCode::DockerfileMetadataOnly,
                SpecialObjectBlockerCode::LfsContentNotMaterialized,
                SpecialObjectBlockerCode::SubmoduleCommitUnverified,
                SpecialObjectBlockerCode::SubmoduleObjectMissing,
            ])
        );
        assert!(first.special_objects.iter().any(|object| {
            matches!(
                &object.metadata,
                SpecialObjectMetadata::GitLfsPointer {
                    oid_sha256,
                    declared_size: 4096,
                } if oid_sha256 == "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            )
        }));
        assert!(first.special_objects.iter().any(|object| {
            matches!(
                &object.metadata,
                SpecialObjectMetadata::OciImageReference {
                    digest_pinned: true,
                    ..
                }
            ) && object.blockers == [SpecialObjectBlockerCode::DockerfileMetadataOnly]
        }));
        assert!(first.files.iter().all(|file| {
            !file
                .normalized_path
                .contains("vendor/recorded-safe-declaration/")
        }));
        Ok(())
    }

    #[test]
    fn contract_unsafe_submodule_declarations_are_blocked_without_resolution()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(
            checkout.path(),
            &[(
                ".gitmodules",
                b"[submodule \"unsafe\"]\npath = ../escape\nurl = file:///tmp/repo\n",
            )],
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
        assert_eq!(materialization.special_objects.len(), 1);
        assert_eq!(
            materialization.special_objects[0].blockers,
            [
                SpecialObjectBlockerCode::UnsafeSubmodulePath,
                SpecialObjectBlockerCode::UnsafeSubmoduleUrl,
                SpecialObjectBlockerCode::SubmoduleObjectMissing,
                SpecialObjectBlockerCode::SubmoduleCommitUnverified,
            ]
        );
        Ok(())
    }

    #[test]
    fn contract_malformed_or_unsupported_special_metadata_fails_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        for (name, bytes) in [
            (
                ".gitmodules",
                b"[submodule \"bad\"]\npath = valid\nurl = \xff\n".as_slice(),
            ),
            (
                "large.bin",
                b"version https://git-lfs.github.com/spec/v1\noid sha256:not-a-digest\nsize 7\n"
                    .as_slice(),
            ),
            ("Dockerfile", b"FROM ${UNRESOLVED_IMAGE}\n".as_slice()),
        ] {
            let checkout = tempfile::tempdir()?;
            write_test_tree(checkout.path(), &[(name, bytes)])?;
            let request = LocalMaterializationRequest {
                profile_id: "cryptohack-ctf-archive",
                checkout_root: checkout.path(),
                materialization_root: ".",
                expected_commit: CRYPTOHACK_PIN,
                observed_checkout_commit: CRYPTOHACK_PIN,
                limits: LocalMaterializationLimits::default(),
            };
            assert!(matches!(
                materialize_pinned_local_root(&catalog, &request),
                Err(CorpusError::UnsafeMaterializationPath { .. })
            ));
        }
        Ok(())
    }

    #[test]
    fn contract_lfs_declared_content_is_never_counted_as_materialized()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(
            checkout.path(),
            &[(
                "payload.bin",
                b"version https://git-lfs.github.com/spec/v1\noid sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\nsize 4096\n",
            )],
        )?;
        let catalog = bundled_corpus_profiles()?;
        let request = LocalMaterializationRequest {
            profile_id: "cryptohack-ctf-archive",
            checkout_root: checkout.path(),
            materialization_root: ".",
            expected_commit: CRYPTOHACK_PIN,
            observed_checkout_commit: CRYPTOHACK_PIN,
            limits: LocalMaterializationLimits {
                max_file_count: 10,
                max_file_bytes: 1024,
                max_total_bytes: 1024,
            },
        };
        let materialization = materialize_pinned_local_root(&catalog, &request)?;
        assert!(materialization.total_bytes < 1024);
        assert_eq!(
            materialization.special_objects[0].blockers,
            [
                SpecialObjectBlockerCode::LfsContentNotMaterialized,
                SpecialObjectBlockerCode::LfsDeclaredSizeExceedsLimit,
            ]
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn contract_hostile_control_character_filename_fails_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(checkout.path(), &[("hostile\nname", b"inert")])?;
        let catalog = bundled_corpus_profiles()?;
        let request = LocalMaterializationRequest {
            profile_id: "cryptohack-ctf-archive",
            checkout_root: checkout.path(),
            materialization_root: ".",
            expected_commit: CRYPTOHACK_PIN,
            observed_checkout_commit: CRYPTOHACK_PIN,
            limits: LocalMaterializationLimits::default(),
        };
        assert!(matches!(
            materialize_pinned_local_root(&catalog, &request),
            Err(CorpusError::UnsafeMaterializationPath { reason, .. })
                if reason.contains("control characters")
        ));
        Ok(())
    }

    #[test]
    fn contract_checked_in_reviewed_overlay_compiles_without_fixture_rewrite()
    -> Result<(), Box<dyn std::error::Error>> {
        let (materialization, overlay) = checked_in_reviewed_overlay_inputs()?;
        assert_eq!(
            overlay.source_materialization_digest, materialization.manifest_digest,
            "replace the checked-in digest with {}",
            materialization.manifest_digest
        );
        let first = compile_reviewed_overlay(&materialization, &overlay)?;
        let second = compile_reviewed_overlay(&materialization, &overlay)?;
        assert_eq!(first, second);
        assert_eq!(first.artifacts.len(), materialization.files.len());
        assert_eq!(first.solver_view.exposed_artifacts.len(), 1);
        assert_eq!(first.readiness, SettlementReadiness::EvaluationOnly);
        assert_eq!(
            first.inherited_special_blockers,
            materialization.readiness_blockers
        );
        assert!(!first.inherited_special_blockers.is_empty());
        assert_eq!(first.rights.rights_status, RightsStatus::Restricted);
        assert!(first.rights.item_license.is_none());
        assert!(!first.rights.redistribution_allowed);
        assert!(!first.rights.execution_allowed);
        assert!(!first.rights.commercial_market_use_allowed);
        Ok(())
    }

    #[test]
    fn contract_reviewed_overlay_schema_denies_unknown_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/corpora/recorded-challenge-overlays.json"
        ))?;
        value["overlays"][0]["unreviewed_extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReviewedOverlayCatalog>(value).is_err());
        Ok(())
    }

    #[test]
    fn contract_reviewed_overlay_rejects_identity_drift_and_incomplete_partition()
    -> Result<(), Box<dyn std::error::Error>> {
        let (materialization, overlay) = checked_in_reviewed_overlay_inputs()?;
        let mut wrong_digest = overlay.clone();
        wrong_digest.source_materialization_digest = Sha256Digest::from_bytes([9; 32]);
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &wrong_digest),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("materialization digest")
        ));

        let mut incomplete = overlay.clone();
        incomplete.artifacts.pop();
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &incomplete),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("classify every materialized file")
        ));

        let mut duplicate = overlay;
        duplicate.artifacts[1] = duplicate.artifacts[0].clone();
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &duplicate),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("duplicate artifact assignment")
        ));
        Ok(())
    }

    #[test]
    fn contract_reviewed_overlay_rejects_media_and_executable_mismatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let (materialization, overlay) = checked_in_reviewed_overlay_inputs()?;
        let mut wrong_media = overlay.clone();
        wrong_media.artifacts[0].expected_media_type = "application/zip".to_owned();
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &wrong_media),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("media type mismatch")
        ));

        let mut wrong_mode = overlay;
        wrong_mode.artifacts[0].expected_executable = true;
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &wrong_mode),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("executable-bit mismatch")
        ));
        Ok(())
    }

    #[test]
    fn contract_reviewed_overlay_prevents_repository_license_and_readiness_promotion()
    -> Result<(), Box<dyn std::error::Error>> {
        let (materialization, overlay) = checked_in_reviewed_overlay_inputs()?;
        let mut promoted_rights = overlay.clone();
        promoted_rights.rights.rights_status = RightsStatus::Permitted;
        promoted_rights.rights.redistribution_allowed = true;
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &promoted_rights),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("repository license cannot promote")
        ));

        let mut settlement_ready = overlay;
        settlement_ready.readiness = SettlementReadiness::SettlementReady;
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &settlement_ready),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("structurally limited")
        ));
        Ok(())
    }

    #[test]
    fn contract_reviewed_solver_view_rejects_digest_alias_with_excluded_content()
    -> Result<(), Box<dyn std::error::Error>> {
        let checkout = tempfile::tempdir()?;
        write_test_tree(
            checkout.path(),
            &[
                ("public.txt", b"same bytes"),
                ("runtime.txt", b"same bytes"),
            ],
        )?;
        let profiles = bundled_corpus_profiles()?;
        let request = LocalMaterializationRequest {
            profile_id: "cryptohack-ctf-archive",
            checkout_root: checkout.path(),
            materialization_root: ".",
            expected_commit: CRYPTOHACK_PIN,
            observed_checkout_commit: CRYPTOHACK_PIN,
            limits: LocalMaterializationLimits::default(),
        };
        let materialization = materialize_pinned_local_root(&profiles, &request)?;
        let (_, mut overlay) = checked_in_reviewed_overlay_inputs()?;
        overlay.overlay_id = "overlay_digest_alias_test".to_owned();
        overlay.profile_id = "cryptohack-ctf-archive".to_owned();
        overlay.pinned_commit = CRYPTOHACK_PIN.to_owned();
        overlay.materialization_root = ".".to_owned();
        overlay.source_materialization_digest = materialization.manifest_digest;
        overlay.artifacts = vec![
            ReviewedOverlayArtifact {
                normalized_path: "public.txt".to_owned(),
                role: ReviewedArtifactRole::SolverPublic,
                expected_media_type: "text/plain".to_owned(),
                expected_executable: false,
            },
            ReviewedOverlayArtifact {
                normalized_path: "runtime.txt".to_owned(),
                role: ReviewedArtifactRole::RuntimePrivate,
                expected_media_type: "text/plain".to_owned(),
                expected_executable: false,
            },
        ];
        assert!(matches!(
            compile_reviewed_overlay(&materialization, &overlay),
            Err(CorpusError::InvalidOverlay { reason, .. })
                if reason.contains("overlaps an excluded artifact")
        ));
        Ok(())
    }
}
