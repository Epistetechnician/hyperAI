//! Admission-aware validator execution, normalization, and replay ports.

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use outcomes_domain::{
    AdmissionDecisionKind, AdmissionDecisionV1, CandidateEnvelopeV1, ClaimSpecV1, DeterminismTier,
    ExecutionManifestV1, ExecutionOutcome, NetworkPolicy, ResourceUsageV1, Sha256Digest,
    UtcTimestamp, VerificationPolicyV1, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

static OCI_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Runner failures are infrastructure states, never a false claim result.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// Admission decision was deny or manual review.
    #[error("execution blocked by admission decision {0:?}")]
    AdmissionBlocked(AdmissionDecisionKind),
    /// Candidate bytes do not match their immutable envelope commitment.
    #[error("candidate artifact length does not match the envelope")]
    ArtifactLengthMismatch,
    /// A built-in runner record could not be canonicalized.
    #[error("runner record is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Scripted test runner had no configured outcome remaining.
    #[error("scripted runner exhausted configured outcomes")]
    ScriptExhausted,
    /// OCI policy is insufficient for untrusted execution.
    #[error("unsafe OCI runner policy: {0}")]
    UnsafeOciPolicy(String),
    /// Local OCI execution is unavailable and was not falsely simulated.
    #[error("OCI runner unavailable: {0}")]
    OciUnavailable(String),
    /// Docker CLI process could not be spawned or polled.
    #[error("OCI process I/O failed: {0}")]
    OciIo(#[from] std::io::Error),
    /// Docker returned state that does not satisfy the requested control contract.
    #[error("OCI daemon contract violation: {0}")]
    OciContract(String),
    /// An attached output reader could not be joined safely.
    #[error("OCI output reader terminated unexpectedly")]
    OciOutputReader,
    /// The runtime OCI identity differs from the environment frozen into the claim.
    #[error("Docker environment identity does not match the frozen claim")]
    DockerEnvironmentIdentityMismatch,
}

/// Immutable input to one verification attempt.
#[derive(Debug, Clone)]
pub struct RunnerRequest<'a> {
    /// Frozen claim semantics.
    pub claim: &'a ClaimSpecV1,
    /// Candidate envelope.
    pub candidate: &'a CandidateEnvelopeV1,
    /// Untrusted candidate bytes.
    pub candidate_bytes: &'a [u8],
    /// Admission decision authorizing this exact execution.
    pub admission: &'a AdmissionDecisionV1,
    /// Unique run identifier.
    pub run_id: &'a str,
    /// Receipt start time.
    pub started_at: &'a UtcTimestamp,
    /// Receipt end time.
    pub ended_at: &'a UtcTimestamp,
    /// Frozen logical time injected into the validator.
    pub logical_time: &'a UtcTimestamp,
}

/// Versioned runner port. Implementations return typed manifests, not log guesses.
pub trait VerificationRunner {
    /// Execute one admitted attempt.
    ///
    /// # Errors
    ///
    /// Returns a runner error for blocked admission, exhausted test scripts, or
    /// unavailable/unsafe execution infrastructure.
    fn execute(&mut self, request: &RunnerRequest<'_>) -> Result<ExecutionManifestV1, RunnerError>;
}

/// Deterministic runner used by hermetic contract and end-to-end tests.
#[derive(Debug, Clone)]
pub struct ScriptedRunner {
    outcomes: Vec<ExecutionOutcome>,
    next: usize,
}

impl ScriptedRunner {
    /// Construct a runner that emits the supplied normalized outcomes in order.
    #[must_use]
    pub fn new(outcomes: Vec<ExecutionOutcome>) -> Self {
        Self { outcomes, next: 0 }
    }

    /// Construct the deterministic local exact-bytes validator.
    #[must_use]
    pub fn exact_bytes_fixture() -> Self {
        Self::new(Vec::new())
    }
}

impl VerificationRunner for ScriptedRunner {
    fn execute(&mut self, request: &RunnerRequest<'_>) -> Result<ExecutionManifestV1, RunnerError> {
        if request.admission.decision != AdmissionDecisionKind::Allow {
            return Err(RunnerError::AdmissionBlocked(request.admission.decision));
        }
        if request.candidate.artifact_byte_length != request.candidate_bytes.len() as u64 {
            return Err(RunnerError::ArtifactLengthMismatch);
        }
        if plain_digest(request.candidate_bytes) != request.candidate.artifact_digest {
            return Err(RunnerError::ArtifactLengthMismatch);
        }
        let outcome = if self.outcomes.is_empty() {
            match request.candidate_bytes {
                b"hsai-pass-v1" => ExecutionOutcome::Pass,
                b"hsai-inconclusive-v1" => ExecutionOutcome::Inconclusive,
                b"hsai-error-v1" => ExecutionOutcome::Error,
                _ => ExecutionOutcome::Fail,
            }
        } else {
            let outcome = self
                .outcomes
                .get(self.next)
                .copied()
                .ok_or(RunnerError::ScriptExhausted)?;
            self.next += 1;
            outcome
        };
        build_manifest(request, outcome)
    }
}

/// Result of comparing the configured replay attempts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayAssessment {
    /// Whether all normalized outcomes agree.
    pub agreed: bool,
    /// Determinism tier actually established.
    pub established_tier: Option<DeterminismTier>,
    /// Agreed outcome, absent when attempts diverge or are empty.
    pub normalized_outcome: Option<ExecutionOutcome>,
}

/// Compare normalized attempts under the frozen determinism tier.
#[must_use]
pub fn assess_replays(
    manifests: &[ExecutionManifestV1],
    required_tier: DeterminismTier,
) -> ReplayAssessment {
    let first = manifests
        .first()
        .map(|manifest| manifest.normalized_outcome);
    let agreed = first.is_some()
        && manifests
            .iter()
            .all(|manifest| Some(manifest.normalized_outcome) == first);
    ReplayAssessment {
        agreed,
        established_tier: agreed.then_some(required_tier),
        normalized_outcome: agreed.then_some(first).flatten(),
    }
}

/// Create an admission record for the exact claim, candidate, and runner policy.
///
/// Deny and manual-review decisions remain evidence; they never imply false.
///
/// # Errors
///
/// Returns an error if the decision record cannot be canonicalized.
pub fn evaluate_admission(
    claim: &ClaimSpecV1,
    candidate: &CandidateEnvelopeV1,
) -> Result<AdmissionDecisionV1, RunnerError> {
    let claim_digest = claim.claim_spec_digest().map_err(|error| {
        RunnerError::UnsafeOciPolicy(format!(
            "invalid frozen claim supplied to admission: {error}"
        ))
    })?;
    let media_allowed = claim
        .eligible_submission_policy
        .allowed_artifact_media_types
        .contains(&candidate.artifact_media_type);
    let size_allowed =
        candidate.artifact_byte_length <= claim.eligible_submission_policy.max_artifact_bytes;
    let claim_matches = candidate.claim_id == claim.claim_id;
    let decision = if media_allowed && size_allowed && claim_matches {
        AdmissionDecisionKind::Allow
    } else {
        AdmissionDecisionKind::Deny
    };
    let reason_codes = [
        (!media_allowed).then_some("ARTIFACT_MEDIA_TYPE_DENIED"),
        (!size_allowed).then_some("ARTIFACT_TOO_LARGE"),
        (!claim_matches).then_some("CLAIM_ID_MISMATCH"),
    ]
    .into_iter()
    .flatten()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let artifact_policy_digest =
        domain_separated_digest("hsai.artifact-policy.v1", &claim.eligible_submission_policy)?;
    let sandbox_policy_digest =
        domain_separated_digest("hsai.sandbox-policy.v1", &claim.verification_policy)?;
    let resource_budget_digest = domain_separated_digest(
        "hsai.resource-budget.v1",
        &claim.verification_policy.resource_limits,
    )?;
    let secrets_policy_digest =
        domain_separated_digest("hsai.secrets-policy.v1", &"no-ambient-secrets")?;
    let decision_payload = (
        claim_digest,
        candidate.artifact_digest,
        claim.verification_policy.validator_artifact_digest,
        claim.verification_policy.environment_digest,
        decision,
        &reason_codes,
    );
    let decision_digest =
        domain_separated_digest("hsai.execution-admission.v1", &decision_payload)?;
    Ok(AdmissionDecisionV1 {
        schema_version: "hsai.execution-admission.v1".to_owned(),
        decision_id: format!("admission_{}", &decision_digest.to_hex()[..32]),
        claim_spec_digest: claim_digest,
        candidate_digest: candidate.artifact_digest,
        validator_digest: claim.verification_policy.validator_artifact_digest,
        environment_digest: claim.verification_policy.environment_digest,
        source_provenance_status: "CONTENT_DIGEST_BOUND".to_owned(),
        artifact_policy_digest,
        sandbox_policy_digest,
        resource_budget_digest,
        secrets_policy_digest,
        risk_classification: "UNTRUSTED_CODE_OR_DATA".to_owned(),
        decision,
        reason_codes,
        policy_version: "hsai.execution-admission.v1".to_owned(),
        decision_digest,
    })
}

/// Minimum OCI controls required by the production-capable runner boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
// These booleans are explicit independently auditable security controls.
#[allow(clippy::struct_excessive_bools)]
pub struct OciSandboxPolicy {
    /// Local repository name combined with the immutable digest at execution.
    pub image_repository: String,
    /// Immutable `sha256:` image reference.
    pub image_digest: String,
    /// Explicit Docker platform in canonical `linux/architecture` form.
    pub platform: String,
    /// Container UID; zero is forbidden.
    pub user_id: u32,
    /// Whether the root filesystem is read-only.
    pub read_only_root: bool,
    /// Whether all Linux capabilities are dropped.
    pub drop_all_capabilities: bool,
    /// Whether networking is disabled.
    pub network_disabled: bool,
    /// Whether the host or Docker socket is mounted.
    pub host_socket_mounted: bool,
    /// Structured argv passed without a shell.
    pub argv: Vec<String>,
    /// Explicit environment allowlist.
    pub environment_allowlist: Vec<String>,
    /// Maximum process count.
    pub pids_limit: u32,
    /// Maximum container memory.
    pub memory_bytes: u64,
    /// CPU quota in thousandths of one CPU.
    pub cpu_millis: u32,
    /// Size of the no-exec scratch tmpfs.
    pub scratch_bytes: u64,
    /// Hard wall-clock timeout for the probe process.
    pub timeout_millis: u64,
}

/// Canonical claim-bound identity of one Docker verification environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct DockerEnvironmentBindingV1 {
    /// Schema and domain-separation version.
    pub schema_version: String,
    /// Repository portion of the immutable image reference.
    pub image_repository: String,
    /// Canonical immutable image digest.
    pub image_digest: String,
    /// Explicit OS and CPU architecture selected from a multi-platform image.
    pub platform: String,
    /// Non-root container UID.
    pub user_id: u32,
    /// Structured verifier argv, excluding the fixed environment wrapper.
    pub argv: Vec<String>,
    /// Exact environment values installed after clearing the image environment.
    pub environment: BTreeMap<String, String>,
    /// Exact sorted environment-key allowlist.
    pub environment_allowlist: Vec<String>,
    /// Whether the root filesystem is read-only.
    pub read_only_root: bool,
    /// Whether all Linux capabilities are dropped.
    pub drop_all_capabilities: bool,
    /// Whether container networking is disabled.
    pub network_disabled: bool,
    /// Whether any host or Docker socket is mounted.
    pub host_socket_mounted: bool,
    /// Maximum process count.
    pub pids_limit: u32,
    /// Maximum container memory in bytes.
    pub memory_bytes: u64,
    /// CPU quota in thousandths of one CPU.
    pub cpu_millis: u32,
    /// Isolated scratch tmpfs size in bytes.
    pub scratch_bytes: u64,
    /// Hard wall-clock timeout in milliseconds.
    pub timeout_millis: u64,
    /// Fixed read-only candidate mount destination.
    pub candidate_mount_destination: String,
    /// Fixed read-only verifier-data mount destination.
    pub verifier_mount_destination: String,
    /// Versioned structured environment-clearing wrapper identity.
    pub wrapper_version: String,
}

/// Build the canonical Docker environment identity for explicit locale/timezone values.
///
/// # Errors
///
/// Returns an error when OCI controls or exact environment values are unsafe.
pub fn docker_environment_binding(
    policy: &OciSandboxPolicy,
    locale: &str,
    timezone: &str,
) -> Result<DockerEnvironmentBindingV1, RunnerError> {
    validate_oci_policy(policy)?;
    let environment = exact_policy_environment(locale, timezone, policy)?;
    let mut environment_allowlist = policy.environment_allowlist.clone();
    environment_allowlist.sort();
    environment_allowlist.dedup();
    Ok(DockerEnvironmentBindingV1 {
        schema_version: "hsai.docker-environment-binding.v1".to_owned(),
        image_repository: policy.image_repository.clone(),
        image_digest: policy.image_digest.clone(),
        platform: policy.platform.clone(),
        user_id: policy.user_id,
        argv: policy.argv.clone(),
        environment,
        environment_allowlist,
        read_only_root: policy.read_only_root,
        drop_all_capabilities: policy.drop_all_capabilities,
        network_disabled: policy.network_disabled,
        host_socket_mounted: policy.host_socket_mounted,
        pids_limit: policy.pids_limit,
        memory_bytes: policy.memory_bytes,
        cpu_millis: policy.cpu_millis,
        scratch_bytes: policy.scratch_bytes,
        timeout_millis: policy.timeout_millis,
        candidate_mount_destination: CONTAINER_CANDIDATE_PATH.to_owned(),
        verifier_mount_destination: CONTAINER_VERIFIER_PATH.to_owned(),
        wrapper_version: "hsai.docker-env-i-wrapper.v1:/usr/bin/env".to_owned(),
    })
}

/// Compute the domain-separated canonical Docker environment identity digest.
///
/// # Errors
///
/// Returns an error when the environment binding is unsafe or cannot be
/// canonicalized.
pub fn docker_environment_digest(
    policy: &OciSandboxPolicy,
    locale: &str,
    timezone: &str,
) -> Result<Sha256Digest, RunnerError> {
    Ok(domain_separated_digest(
        "hsai.docker-environment-binding.v1",
        &docker_environment_binding(policy, locale, timezone)?,
    )?)
}

/// Result of enforcing the OCI control-plane contract against a local daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciProbeResult {
    /// Immutable repository-at-digest reference executed.
    pub image_reference: String,
    /// Whether the container command exited successfully.
    pub success: bool,
    /// Measured host wall-clock time.
    pub wall_clock_millis: u64,
}

/// Candidate path visible inside the isolated verifier container.
pub const CONTAINER_CANDIDATE_PATH: &str = "/input/candidate";
/// Expected/verifier input path visible inside the isolated verifier container.
pub const CONTAINER_VERIFIER_PATH: &str = "/verifier/expected";

/// Public, bounded diagnostic view of one fully digested output stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedOutputStream {
    /// SHA-256 over every byte drained before the isolated process terminated.
    pub digest: Sha256Digest,
    /// Total drained byte count, including bytes omitted from the preview.
    pub byte_count: u64,
    /// Prefix bounded by the runner's configured public diagnostic limit.
    pub public_preview: Vec<u8>,
}

/// Bounded diagnostics retained from the most recent Docker verification attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerPublicOutput {
    /// Captured standard output metadata and bounded prefix.
    pub stdout: BoundedOutputStream,
    /// Captured standard error metadata and bounded prefix.
    pub stderr: BoundedOutputStream,
}

/// Docker CLI verifier implementing [`VerificationRunner`].
///
/// Candidate and expected verifier bytes are copied into separate read-only
/// regular files and mounted at fixed paths. The verifier executable itself is
/// part of the immutable container image.
#[derive(Debug, Clone)]
pub struct DockerVerificationRunner {
    policy: OciSandboxPolicy,
    verifier_expected_bytes: Vec<u8>,
    public_preview_bytes_per_stream: usize,
    docker_binary: PathBuf,
    last_public_output: Option<DockerPublicOutput>,
}

impl DockerVerificationRunner {
    /// Construct a Docker verifier using expected bytes committed by the claim's
    /// `validator_artifact_digest`.
    ///
    /// # Errors
    ///
    /// Returns an error when the OCI policy is unsafe, the verifier bytes are
    /// empty, or the fixed input paths are absent from the structured argv.
    pub fn new(
        policy: OciSandboxPolicy,
        verifier_expected_bytes: Vec<u8>,
    ) -> Result<Self, RunnerError> {
        validate_oci_policy(&policy)?;
        if verifier_expected_bytes.is_empty() {
            return Err(RunnerError::UnsafeOciPolicy(
                "verifier expected bytes must be non-empty".to_owned(),
            ));
        }
        if !policy
            .argv
            .iter()
            .any(|value| value == CONTAINER_CANDIDATE_PATH)
            || !policy
                .argv
                .iter()
                .any(|value| value == CONTAINER_VERIFIER_PATH)
        {
            return Err(RunnerError::UnsafeOciPolicy(
                "verifier argv must reference both fixed read-only input paths".to_owned(),
            ));
        }
        Ok(Self {
            policy,
            verifier_expected_bytes,
            public_preview_bytes_per_stream: 4_096,
            docker_binary: PathBuf::from("docker"),
            last_public_output: None,
        })
    }

    /// Set the maximum public diagnostic prefix retained per stream.
    #[must_use]
    pub fn with_public_preview_limit(mut self, bytes_per_stream: usize) -> Self {
        self.public_preview_bytes_per_stream = bytes_per_stream;
        self
    }

    /// Return bounded diagnostics from the most recent completed attempt.
    #[must_use]
    pub fn last_public_output(&self) -> Option<&DockerPublicOutput> {
        self.last_public_output.as_ref()
    }
}

/// Validate the fail-closed OCI policy before any process is spawned.
///
/// # Errors
///
/// Returns an exact unsafe-policy reason when a required control is absent.
pub fn validate_oci_policy(policy: &OciSandboxPolicy) -> Result<(), RunnerError> {
    let digest_hex = policy.image_digest.strip_prefix("sha256:");
    if !digest_hex.is_some_and(|value| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    }) {
        return Err(RunnerError::UnsafeOciPolicy(
            "image must be pinned by a 64-hex SHA-256 digest".to_owned(),
        ));
    }
    if policy.user_id == 0 {
        return Err(RunnerError::UnsafeOciPolicy(
            "container user must be non-root".to_owned(),
        ));
    }
    split_docker_platform(&policy.platform)?;
    if !policy.read_only_root || !policy.drop_all_capabilities || !policy.network_disabled {
        return Err(RunnerError::UnsafeOciPolicy(
            "read-only root, dropped capabilities, and disabled network are required".to_owned(),
        ));
    }
    if policy.host_socket_mounted {
        return Err(RunnerError::UnsafeOciPolicy(
            "host and Docker socket mounts are forbidden".to_owned(),
        ));
    }
    if policy.argv.is_empty() || policy.argv.iter().any(String::is_empty) {
        return Err(RunnerError::UnsafeOciPolicy(
            "structured non-empty argv is required".to_owned(),
        ));
    }
    validate_structured_argv(&policy.argv)?;
    if policy.image_repository.is_empty()
        || policy.image_repository.starts_with('-')
        || policy.image_repository.contains('@')
        || policy.image_repository.chars().any(char::is_whitespace)
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "image repository is empty or contains unsafe syntax".to_owned(),
        ));
    }
    if policy.pids_limit == 0
        || policy.memory_bytes < 16 * 1024 * 1024
        || policy.cpu_millis == 0
        || policy.cpu_millis > 1_000
        || policy.scratch_bytes == 0
        || policy.timeout_millis == 0
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "bounded pids, memory, CPU, scratch, and timeout are required".to_owned(),
        ));
    }
    Ok(())
}

fn split_docker_platform(platform: &str) -> Result<(&str, &str), RunnerError> {
    let Some((operating_system, architecture)) = platform.split_once('/') else {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker platform must be explicit canonical os/architecture".to_owned(),
        ));
    };
    if operating_system != "linux"
        || !matches!(architecture, "amd64" | "arm64")
        || architecture.contains('/')
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker platform must be linux/amd64 or linux/arm64".to_owned(),
        ));
    }
    Ok((operating_system, architecture))
}

fn canonical_cpu_architecture(architecture: &str) -> Option<&'static str> {
    match architecture {
        "amd64" | "x86_64" => Some("amd64"),
        "arm64" | "aarch64" => Some("arm64"),
        _ => None,
    }
}

fn validate_structured_argv(argv: &[String]) -> Result<(), RunnerError> {
    if argv.iter().any(|argument| argument.contains('\0')) {
        return Err(RunnerError::UnsafeOciPolicy(
            "argv contains a NUL byte".to_owned(),
        ));
    }
    let executable = Path::new(&argv[0])
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if matches!(
        executable,
        "sh" | "bash" | "dash" | "ash" | "zsh" | "csh" | "ksh" | "fish"
    ) {
        return Err(RunnerError::UnsafeOciPolicy(
            "shell executables are forbidden; use structured verifier argv".to_owned(),
        ));
    }
    Ok(())
}

/// Build the exact Docker `create` arguments for the validated policy.
///
/// The result contains no shell string and mounts no host path or socket.
///
/// # Errors
///
/// Returns a typed unsafe-policy error before returning arguments.
pub fn docker_create_args(
    policy: &OciSandboxPolicy,
    container_name: &str,
) -> Result<Vec<String>, RunnerError> {
    validate_oci_policy(policy)?;
    if container_name.is_empty()
        || container_name.starts_with('-')
        || !container_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "container name contains unsafe syntax".to_owned(),
        ));
    }
    let mut arguments = vec![
        "create".to_owned(),
        "--name".to_owned(),
        container_name.to_owned(),
        "--platform".to_owned(),
        policy.platform.clone(),
        "--network".to_owned(),
        "none".to_owned(),
        "--read-only".to_owned(),
        "--cap-drop".to_owned(),
        "ALL".to_owned(),
        "--security-opt".to_owned(),
        "no-new-privileges".to_owned(),
        "--user".to_owned(),
        format!("{}:{}", policy.user_id, policy.user_id),
        "--pids-limit".to_owned(),
        policy.pids_limit.to_string(),
        "--memory".to_owned(),
        policy.memory_bytes.to_string(),
        "--cpus".to_owned(),
        format_cpu_quota(policy.cpu_millis),
        "--tmpfs".to_owned(),
        format!("/tmp:rw,noexec,nosuid,nodev,size={}", policy.scratch_bytes),
    ];
    for name in &policy.environment_allowlist {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(RunnerError::UnsafeOciPolicy(
                "environment allowlist contains an unsafe name".to_owned(),
            ));
        }
        arguments.push("--env".to_owned());
        // An explicit empty value is intentional. Passing only the name would
        // inherit the Docker CLI process's ambient host value.
        arguments.push(format!("{name}="));
    }
    arguments.push("--entrypoint".to_owned());
    arguments.push(policy.argv[0].clone());
    arguments.push(format!(
        "{}@{}",
        policy.image_repository, policy.image_digest
    ));
    arguments.extend(policy.argv.iter().skip(1).cloned());
    Ok(arguments)
}

/// Build exact Docker `create` arguments for a candidate verification run.
///
/// The container process starts via `env -i`, which discards both host-provided
/// values and image-baked environment entries before invoking the structured
/// verifier argv. Both inputs are read-only regular-file bind mounts.
///
/// # Errors
///
/// Returns an error for unsafe policy, input paths, environment values, or
/// container names.
pub fn docker_verifier_create_args(
    policy: &OciSandboxPolicy,
    container_name: &str,
    candidate_path: &Path,
    verifier_path: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<Vec<String>, RunnerError> {
    let mut arguments = docker_create_args(policy, container_name)?;
    validate_mount_source(candidate_path)?;
    validate_mount_source(verifier_path)?;
    validate_exact_environment(policy, environment)?;

    // Replace the probe image/command tail with verifier-specific mounts and
    // the explicit environment-clearing wrapper.
    let image_index = arguments
        .iter()
        .position(|argument| {
            argument == &format!("{}@{}", policy.image_repository, policy.image_digest)
        })
        .ok_or_else(|| RunnerError::OciContract("validated image argument is absent".to_owned()))?;
    let entrypoint_index = arguments[..image_index]
        .iter()
        .rposition(|argument| argument == "--entrypoint")
        .ok_or_else(|| {
            RunnerError::OciContract("validated entrypoint argument is absent".to_owned())
        })?;
    arguments.truncate(entrypoint_index);
    arguments.extend([
        "--pull".to_owned(),
        "never".to_owned(),
        "--mount".to_owned(),
        format!(
            "type=bind,src={},dst={CONTAINER_CANDIDATE_PATH},readonly",
            candidate_path.display()
        ),
        "--mount".to_owned(),
        format!(
            "type=bind,src={},dst={CONTAINER_VERIFIER_PATH},readonly",
            verifier_path.display()
        ),
        "--entrypoint".to_owned(),
        "/usr/bin/env".to_owned(),
        format!("{}@{}", policy.image_repository, policy.image_digest),
        "-i".to_owned(),
    ]);
    arguments.extend(
        environment
            .iter()
            .map(|(name, value)| format!("{name}={value}")),
    );
    arguments.extend(policy.argv.iter().cloned());
    Ok(arguments)
}

fn validate_mount_source(path: &Path) -> Result<(), RunnerError> {
    if !path.is_absolute()
        || path
            .as_os_str()
            .to_string_lossy()
            .chars()
            .any(|character| matches!(character, ',' | '\n' | '\r' | '\0'))
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "mount source must be an absolute path without Docker mount delimiters".to_owned(),
        ));
    }
    let metadata = path.symlink_metadata()?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(RunnerError::UnsafeOciPolicy(
            "mount source must be a regular non-symlink file".to_owned(),
        ));
    }
    Ok(())
}

fn validate_exact_environment(
    policy: &OciSandboxPolicy,
    environment: &BTreeMap<String, String>,
) -> Result<(), RunnerError> {
    let mut expected = policy.environment_allowlist.clone();
    expected.sort();
    expected.dedup();
    let actual = environment.keys().cloned().collect::<Vec<_>>();
    if actual != expected {
        return Err(RunnerError::UnsafeOciPolicy(
            "exact environment keys do not match the policy allowlist".to_owned(),
        ));
    }
    if environment
        .iter()
        .any(|(name, value)| name.contains('=') || value.contains('\0'))
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "environment contains unsafe name or value bytes".to_owned(),
        ));
    }
    Ok(())
}

/// Execute an effect-free control-plane probe against the local Docker daemon.
///
/// The command receives no candidate, host mount, socket, credential, or
/// network. Output is discarded to make output flooding non-blocking. This
/// verifies daemon enforcement only; it is not a candidate verifier backend.
///
/// # Errors
///
/// Returns policy, Docker availability, create/start, timeout, or cleanup
/// failures. Timeout cleanup forcibly removes the isolated probe container.
pub fn run_docker_oci_probe(policy: &OciSandboxPolicy) -> Result<OciProbeResult, RunnerError> {
    validate_oci_policy(policy)?;
    let suffix = &policy.image_digest[7..19];
    let sequence = OCI_PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let container_name = format!("hsai-oci-probe-{}-{sequence}-{suffix}", std::process::id());
    let arguments = docker_create_args(policy, &container_name)?;
    let create = Command::new("docker").args(&arguments).output()?;
    if !create.status.success() {
        return Err(RunnerError::OciUnavailable(
            String::from_utf8_lossy(&create.stderr).trim().to_owned(),
        ));
    }
    let started = Instant::now();
    let child = Command::new("docker")
        .args(["start", "--attach", &container_name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            remove_probe_container(&container_name, true);
            return Err(RunnerError::OciIo(error));
        }
    };
    let timeout = Duration::from_millis(policy.timeout_millis);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            remove_probe_container(&container_name, true);
            let _ = child.wait();
            return Err(RunnerError::OciUnavailable(
                "probe exceeded hard wall-clock timeout".to_owned(),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let cleanup = Command::new("docker")
        .args(["rm", &container_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !cleanup.success() {
        return Err(RunnerError::OciUnavailable(
            "probe container cleanup failed".to_owned(),
        ));
    }
    Ok(OciProbeResult {
        image_reference: format!("{}@{}", policy.image_repository, policy.image_digest),
        success: status.success(),
        wall_clock_millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

impl VerificationRunner for DockerVerificationRunner {
    // The ordered create/inspect/start/drain/kill/inspect/cleanup sequence is
    // intentionally visible in one place so cleanup cannot be skipped by a
    // hidden early return.
    #[allow(clippy::too_many_lines)]
    fn execute(&mut self, request: &RunnerRequest<'_>) -> Result<ExecutionManifestV1, RunnerError> {
        self.last_public_output = None;
        validate_runner_request(request)?;
        validate_docker_policy_against_verification(
            &self.policy,
            &request.claim.verification_policy,
        )?;
        let environment_binding = docker_environment_binding(
            &self.policy,
            &request.claim.verification_policy.locale,
            &request.claim.verification_policy.timezone,
        )?;
        let environment_binding_digest =
            domain_separated_digest("hsai.docker-environment-binding.v1", &environment_binding)?;
        if environment_binding_digest != request.claim.verification_policy.environment_digest {
            return Err(RunnerError::DockerEnvironmentIdentityMismatch);
        }
        if plain_digest(&self.verifier_expected_bytes)
            != request.claim.verification_policy.validator_artifact_digest
        {
            return Err(RunnerError::UnsafeOciPolicy(
                "verifier expected bytes do not match the frozen validator digest".to_owned(),
            ));
        }
        if self.policy.timeout_millis
            > request
                .claim
                .verification_policy
                .resource_limits
                .wall_clock_millis
        {
            return Err(RunnerError::UnsafeOciPolicy(
                "Docker timeout exceeds the frozen claim wall-clock limit".to_owned(),
            ));
        }

        let environment = environment_binding.environment.clone();
        let candidate_file = read_only_input_file("candidate", request.candidate_bytes)?;
        let verifier_file = read_only_input_file("verifier", &self.verifier_expected_bytes)?;
        let sequence = OCI_PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let suffix = &self.policy.image_digest[7..19];
        let container_name = format!("hsai-verifier-{}-{sequence}-{suffix}", std::process::id());
        let arguments = docker_verifier_create_args(
            &self.policy,
            &container_name,
            candidate_file.path(),
            verifier_file.path(),
            &environment,
        )?;
        let create = Command::new(&self.docker_binary)
            .args(&arguments)
            .output()?;
        if !create.status.success() {
            remove_named_container(&self.docker_binary, &container_name, true);
            return Err(RunnerError::OciUnavailable(bounded_control_error(
                &create.stderr,
            )));
        }
        let mut guard = ContainerGuard::new(self.docker_binary.clone(), container_name.clone());
        let inspection = inspect_container(&self.docker_binary, &container_name)?;
        let control_facts = validate_daemon_controls(
            &self.docker_binary,
            &inspection,
            &self.policy,
            candidate_file.path(),
            verifier_file.path(),
            &environment,
        )?;

        let started = Instant::now();
        let mut child = match Command::new(&self.docker_binary)
            .args(["start", "--attach", &container_name])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                guard.cleanup_force()?;
                return Err(RunnerError::OciIo(error));
            }
        };
        let stdout = child.stdout.take().ok_or_else(|| {
            RunnerError::OciContract("Docker attach stdout pipe is absent".to_owned())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            RunnerError::OciContract("Docker attach stderr pipe is absent".to_owned())
        })?;
        let output_limit = request
            .claim
            .verification_policy
            .resource_limits
            .output_bytes;
        let total_output = Arc::new(AtomicU64::new(0));
        let output_flooded = Arc::new(AtomicBool::new(false));
        let stdout_reader = spawn_output_reader(
            stdout,
            Arc::clone(&total_output),
            Arc::clone(&output_flooded),
            output_limit,
            self.public_preview_bytes_per_stream,
        );
        let stderr_reader = spawn_output_reader(
            stderr,
            Arc::clone(&total_output),
            Arc::clone(&output_flooded),
            output_limit,
            self.public_preview_bytes_per_stream,
        );

        let stop = wait_for_attached_process(
            &mut child,
            started,
            Duration::from_millis(self.policy.timeout_millis),
            &output_flooded,
        )?;
        if stop != ProcessStop::Exited {
            let _ = Command::new(&self.docker_binary)
                .args(["kill", &container_name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = child.kill();
            guard.cleanup_force()?;
            let _ = child.wait();
        }

        let stdout = join_output_reader(stdout_reader)?;
        let stderr = join_output_reader(stderr_reader)?;
        self.last_public_output = Some(DockerPublicOutput {
            stdout: stdout.clone(),
            stderr: stderr.clone(),
        });
        let elapsed_millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        let state = if stop == ProcessStop::Exited {
            let state = inspect_container_state(&self.docker_binary, &container_name)?;
            guard.cleanup_force()?;
            state
        } else {
            ContainerState {
                exit_code: None,
                oom_killed: false,
                runtime_error: None,
            }
        };
        build_docker_manifest(
            request,
            &self.policy,
            &container_name,
            environment_binding_digest,
            &environment,
            &control_facts,
            &stdout,
            &stderr,
            &state,
            stop,
            elapsed_millis,
        )
    }
}

fn validate_runner_request(request: &RunnerRequest<'_>) -> Result<(), RunnerError> {
    if request.admission.decision != AdmissionDecisionKind::Allow {
        return Err(RunnerError::AdmissionBlocked(request.admission.decision));
    }
    if request.candidate.artifact_byte_length != request.candidate_bytes.len() as u64
        || plain_digest(request.candidate_bytes) != request.candidate.artifact_digest
    {
        return Err(RunnerError::ArtifactLengthMismatch);
    }
    Ok(())
}

fn validate_docker_policy_against_verification(
    policy: &OciSandboxPolicy,
    verification_policy: &VerificationPolicyV1,
) -> Result<(), RunnerError> {
    validate_oci_policy(policy)?;
    if policy.argv != verification_policy.argv {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker policy argv differs from the frozen verification argv".to_owned(),
        ));
    }
    if verification_policy.network_policy != NetworkPolicy::Disabled || !policy.network_disabled {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker verifier currently requires a frozen disabled-network policy".to_owned(),
        ));
    }
    let (_, architecture) = split_docker_platform(&policy.platform)?;
    if canonical_cpu_architecture(&verification_policy.cpu_architecture) != Some(architecture) {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker platform architecture differs from the frozen verification architecture"
                .to_owned(),
        ));
    }
    let mut policy_keys = policy.environment_allowlist.clone();
    policy_keys.sort();
    let mut frozen_keys = verification_policy.environment_allowlist.clone();
    frozen_keys.sort();
    if policy_keys != frozen_keys {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker environment allowlist differs from the frozen verification allowlist"
                .to_owned(),
        ));
    }
    Ok(())
}

fn exact_policy_environment(
    locale: &str,
    timezone: &str,
    policy: &OciSandboxPolicy,
) -> Result<BTreeMap<String, String>, RunnerError> {
    let mut policy_keys = policy.environment_allowlist.clone();
    policy_keys.sort();
    if policy_keys.windows(2).any(|keys| keys[0] == keys[1]) {
        return Err(RunnerError::UnsafeOciPolicy(
            "environment allowlist contains duplicate keys".to_owned(),
        ));
    }
    if policy_keys != ["LANG", "TZ"] {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker environment allowlist must contain exactly LANG and TZ".to_owned(),
        ));
    }
    if locale.is_empty() || timezone.is_empty() || locale.contains('\0') || timezone.contains('\0')
    {
        return Err(RunnerError::UnsafeOciPolicy(
            "Docker locale and timezone must be non-empty and contain no NUL".to_owned(),
        ));
    }
    let mut environment = BTreeMap::new();
    for name in &policy.environment_allowlist {
        let value = match name.as_str() {
            "LANG" => locale.to_owned(),
            "TZ" => timezone.to_owned(),
            _ => {
                return Err(RunnerError::UnsafeOciPolicy(format!(
                    "no exact frozen value is defined for environment key {name}"
                )));
            }
        };
        environment.insert(name.clone(), value);
    }
    validate_exact_environment(policy, &environment)?;
    Ok(environment)
}

fn read_only_input_file(label: &str, bytes: &[u8]) -> Result<tempfile::NamedTempFile, RunnerError> {
    let mut file = tempfile::Builder::new()
        .prefix(&format!("hsai-{label}-"))
        .tempfile()?;
    file.write_all(bytes)?;
    file.flush()?;
    file.as_file().sync_all()?;
    let mut permissions = file.as_file().metadata()?.permissions();
    permissions.set_readonly(true);
    file.as_file().set_permissions(permissions)?;
    Ok(file)
}

fn bounded_control_error(bytes: &[u8]) -> String {
    const LIMIT: usize = 4_096;
    String::from_utf8_lossy(&bytes[..bytes.len().min(LIMIT)])
        .trim()
        .to_owned()
}

#[derive(Debug)]
struct ContainerGuard {
    docker_binary: PathBuf,
    name: String,
    active: bool,
}

impl ContainerGuard {
    fn new(docker_binary: PathBuf, name: String) -> Self {
        Self {
            docker_binary,
            name,
            active: true,
        }
    }

    fn cleanup_force(&mut self) -> Result<(), RunnerError> {
        if !self.active {
            return Ok(());
        }
        let cleanup = Command::new(&self.docker_binary)
            .args(docker_force_remove_args(&self.name))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !cleanup.success() {
            return Err(RunnerError::OciUnavailable(
                "verifier container cleanup failed".to_owned(),
            ));
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        if self.active {
            remove_named_container(&self.docker_binary, &self.name, true);
        }
    }
}

fn remove_named_container(docker_binary: &Path, container_name: &str, force: bool) {
    let mut command = Command::new(docker_binary);
    command.arg("rm");
    if force {
        command.arg("--force");
    }
    let _ = command
        .arg(container_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn docker_force_remove_args(container_name: &str) -> [String; 3] {
    [
        "rm".to_owned(),
        "--force".to_owned(),
        container_name.to_owned(),
    ]
}

#[derive(Debug, Clone)]
struct ContainerInspection {
    value: serde_json::Value,
}

fn inspect_container(
    docker_binary: &Path,
    container_name: &str,
) -> Result<ContainerInspection, RunnerError> {
    let output = Command::new(docker_binary)
        .args(["inspect", container_name])
        .output()?;
    if !output.status.success() {
        return Err(RunnerError::OciUnavailable(bounded_control_error(
            &output.stderr,
        )));
    }
    let documents: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        RunnerError::OciContract(format!("invalid Docker inspect JSON: {error}"))
    })?;
    let value = documents
        .as_array()
        .and_then(|values| values.first())
        .cloned()
        .ok_or_else(|| RunnerError::OciContract("Docker inspect result is empty".to_owned()))?;
    Ok(ContainerInspection { value })
}

fn json_path<'a>(
    value: &'a serde_json::Value,
    path: &[&str],
) -> Result<&'a serde_json::Value, RunnerError> {
    path.iter().try_fold(value, |current, segment| {
        current.get(*segment).ok_or_else(|| {
            RunnerError::OciContract(format!("Docker inspect field {} is absent", path.join(".")))
        })
    })
}

fn validate_daemon_controls(
    docker_binary: &Path,
    inspection: &ContainerInspection,
    policy: &OciSandboxPolicy,
    candidate_path: &Path,
    verifier_path: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, RunnerError> {
    let value = &inspection.value;
    let image_reference = format!("{}@{}", policy.image_repository, policy.image_digest);
    require_json_string(value, &["Config", "Image"], &image_reference)?;
    require_json_string(
        value,
        &["Config", "User"],
        &format!("{}:{}", policy.user_id, policy.user_id),
    )?;
    require_json_bool(value, &["HostConfig", "ReadonlyRootfs"], true)?;
    require_json_string(value, &["HostConfig", "NetworkMode"], "none")?;
    require_json_i64(
        value,
        &["HostConfig", "PidsLimit"],
        i64::from(policy.pids_limit),
    )?;
    require_json_i64(
        value,
        &["HostConfig", "Memory"],
        i64::try_from(policy.memory_bytes).map_err(|_| {
            RunnerError::UnsafeOciPolicy("memory limit exceeds Docker's signed range".to_owned())
        })?,
    )?;
    require_json_i64(
        value,
        &["HostConfig", "NanoCpus"],
        i64::from(policy.cpu_millis) * 1_000_000,
    )?;
    require_json_array_contains(value, &["HostConfig", "CapDrop"], "ALL")?;
    require_json_array_contains(value, &["HostConfig", "SecurityOpt"], "no-new-privileges")?;
    let tmpfs = json_path(value, &["HostConfig", "Tmpfs"])?;
    if tmpfs
        .get("/tmp")
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        return Err(RunnerError::OciContract(
            "Docker did not apply the isolated /tmp tmpfs".to_owned(),
        ));
    }
    let mounts = json_path(value, &["Mounts"])?
        .as_array()
        .ok_or_else(|| RunnerError::OciContract("Docker mounts are not an array".to_owned()))?;
    validate_read_only_file_mount(mounts, candidate_path, CONTAINER_CANDIDATE_PATH)?;
    validate_read_only_file_mount(mounts, verifier_path, CONTAINER_VERIFIER_PATH)?;
    if mounts.len() != 2 {
        return Err(RunnerError::OciContract(
            "Docker applied mounts beyond the two read-only input files".to_owned(),
        ));
    }
    let expected_command = ["-i".to_owned()]
        .into_iter()
        .chain(
            environment
                .iter()
                .map(|(name, value)| format!("{name}={value}")),
        )
        .chain(policy.argv.iter().cloned())
        .collect::<Vec<_>>();
    require_json_string_array(value, &["Config", "Entrypoint"], &["/usr/bin/env"])?;
    require_json_string_array_owned(value, &["Config", "Cmd"], &expected_command)?;

    let image_id = json_path(value, &["Image"])?
        .as_str()
        .ok_or_else(|| RunnerError::OciContract("Docker image ID is not text".to_owned()))?;
    let (expected_os, expected_architecture) = split_docker_platform(&policy.platform)?;
    require_json_string(value, &["Platform"], expected_os)?;
    let (actual_os, actual_architecture) = inspect_image_platform(docker_binary, image_id)?;
    if actual_os != expected_os || actual_architecture != expected_architecture {
        return Err(RunnerError::OciContract(
            "resolved image OS/architecture differs from the claim-bound Docker platform"
                .to_owned(),
        ));
    }
    Ok(BTreeMap::from([
        ("container_image_id".to_owned(), image_id.to_owned()),
        ("image_reference".to_owned(), image_reference),
        ("image_os".to_owned(), actual_os),
        ("image_architecture".to_owned(), actual_architecture),
        ("requested_platform".to_owned(), policy.platform.clone()),
        ("network_mode".to_owned(), "none".to_owned()),
        ("root_filesystem".to_owned(), "read-only".to_owned()),
        ("capabilities".to_owned(), "drop-all".to_owned()),
        ("no_new_privileges".to_owned(), "true".to_owned()),
        ("input_mounts".to_owned(), "two-read-only-files".to_owned()),
        (
            "environment_wrapper".to_owned(),
            "/usr/bin/env -i".to_owned(),
        ),
    ]))
}

fn inspect_image_platform(
    docker_binary: &Path,
    image_id: &str,
) -> Result<(String, String), RunnerError> {
    let output = Command::new(docker_binary)
        .args(["image", "inspect", image_id])
        .output()?;
    if !output.status.success() {
        return Err(RunnerError::OciUnavailable(bounded_control_error(
            &output.stderr,
        )));
    }
    let documents: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        RunnerError::OciContract(format!("invalid Docker image inspect JSON: {error}"))
    })?;
    let image = documents
        .as_array()
        .and_then(|values| values.first())
        .ok_or_else(|| {
            RunnerError::OciContract("Docker image inspect result is empty".to_owned())
        })?;
    let operating_system = image
        .get("Os")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| RunnerError::OciContract("Docker image OS is absent".to_owned()))?;
    let architecture = image
        .get("Architecture")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            RunnerError::OciContract("Docker image architecture is absent".to_owned())
        })?;
    Ok((operating_system.to_owned(), architecture.to_owned()))
}

fn require_json_string(
    value: &serde_json::Value,
    path: &[&str],
    expected: &str,
) -> Result<(), RunnerError> {
    if json_path(value, path)?.as_str() == Some(expected) {
        Ok(())
    } else {
        Err(RunnerError::OciContract(format!(
            "Docker inspect field {} does not match the requested value",
            path.join(".")
        )))
    }
}

fn require_json_bool(
    value: &serde_json::Value,
    path: &[&str],
    expected: bool,
) -> Result<(), RunnerError> {
    if json_path(value, path)?.as_bool() == Some(expected) {
        Ok(())
    } else {
        Err(RunnerError::OciContract(format!(
            "Docker inspect field {} does not match the requested value",
            path.join(".")
        )))
    }
}

fn require_json_i64(
    value: &serde_json::Value,
    path: &[&str],
    expected: i64,
) -> Result<(), RunnerError> {
    if json_path(value, path)?.as_i64() == Some(expected) {
        Ok(())
    } else {
        Err(RunnerError::OciContract(format!(
            "Docker inspect field {} does not match the requested limit",
            path.join(".")
        )))
    }
}

fn require_json_array_contains(
    value: &serde_json::Value,
    path: &[&str],
    expected: &str,
) -> Result<(), RunnerError> {
    let present = json_path(value, path)?.as_array().is_some_and(|values| {
        values.iter().any(|value| {
            value.as_str().is_some_and(|actual| {
                actual == expected
                    || (expected == "no-new-privileges" && actual.starts_with("no-new-privileges:"))
            })
        })
    });
    if present {
        Ok(())
    } else {
        Err(RunnerError::OciContract(format!(
            "Docker inspect field {} omits {expected}",
            path.join(".")
        )))
    }
}

fn require_json_string_array(
    value: &serde_json::Value,
    path: &[&str],
    expected: &[&str],
) -> Result<(), RunnerError> {
    let expected = expected
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    require_json_string_array_owned(value, path, &expected)
}

fn require_json_string_array_owned(
    value: &serde_json::Value,
    path: &[&str],
    expected: &[String],
) -> Result<(), RunnerError> {
    let actual = json_path(value, path)?.as_array().and_then(|values| {
        values
            .iter()
            .map(|value| value.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
    });
    if actual.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(RunnerError::OciContract(format!(
            "Docker inspect field {} does not match exact structured argv",
            path.join(".")
        )))
    }
}

fn validate_read_only_file_mount(
    mounts: &[serde_json::Value],
    source: &Path,
    destination: &str,
) -> Result<(), RunnerError> {
    let expected_source = source.to_string_lossy();
    let matching = mounts.iter().any(|mount| {
        let actual_source = mount
            .get("Source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let source_matches = actual_source == expected_source
            || actual_source.ends_with(expected_source.trim_start_matches('/'));
        mount.get("Type").and_then(serde_json::Value::as_str) == Some("bind")
            && source_matches
            && mount.get("Destination").and_then(serde_json::Value::as_str) == Some(destination)
            && mount.get("RW").and_then(serde_json::Value::as_bool) == Some(false)
    });
    if matching {
        Ok(())
    } else {
        Err(RunnerError::OciContract(format!(
            "Docker omitted read-only file mount at {destination}"
        )))
    }
}

type OutputReader = thread::JoinHandle<Result<BoundedOutputStream, std::io::Error>>;

fn spawn_output_reader<R>(
    reader: R,
    total_output: Arc<AtomicU64>,
    output_flooded: Arc<AtomicBool>,
    output_limit: u64,
    public_preview_limit: usize,
) -> OutputReader
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        drain_output_stream(
            reader,
            &total_output,
            &output_flooded,
            output_limit,
            public_preview_limit,
        )
    })
}

fn drain_output_stream<R: Read>(
    mut reader: R,
    total_output: &AtomicU64,
    output_flooded: &AtomicBool,
    output_limit: u64,
    public_preview_limit: usize,
) -> Result<BoundedOutputStream, std::io::Error> {
    let mut hasher = Sha256::new();
    let mut byte_count = 0_u64;
    let mut public_preview = Vec::with_capacity(public_preview_limit.min(16_384));
    let mut buffer = [0_u8; 16_384];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        byte_count = byte_count.saturating_add(count as u64);
        let previous = total_output.fetch_add(count as u64, Ordering::Relaxed);
        if previous.saturating_add(count as u64) > output_limit {
            output_flooded.store(true, Ordering::Release);
        }
        if public_preview.len() < public_preview_limit {
            let remaining = public_preview_limit - public_preview.len();
            public_preview.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    }
    Ok(BoundedOutputStream {
        digest: Sha256Digest::from_bytes(hasher.finalize().into()),
        byte_count,
        public_preview,
    })
}

fn join_output_reader(reader: OutputReader) -> Result<BoundedOutputStream, RunnerError> {
    reader
        .join()
        .map_err(|_| RunnerError::OciOutputReader)?
        .map_err(RunnerError::OciIo)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessStop {
    Exited,
    TimedOut,
    OutputFlood,
}

fn wait_for_attached_process(
    child: &mut Child,
    started: Instant,
    timeout: Duration,
    output_flooded: &AtomicBool,
) -> Result<ProcessStop, RunnerError> {
    loop {
        if child.try_wait()?.is_some() {
            return Ok(ProcessStop::Exited);
        }
        if output_flooded.load(Ordering::Acquire) {
            return Ok(ProcessStop::OutputFlood);
        }
        if started.elapsed() >= timeout {
            return Ok(ProcessStop::TimedOut);
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContainerState {
    exit_code: Option<i32>,
    oom_killed: bool,
    runtime_error: Option<String>,
}

fn inspect_container_state(
    docker_binary: &Path,
    container_name: &str,
) -> Result<ContainerState, RunnerError> {
    let inspection = inspect_container(docker_binary, container_name)?;
    let state = json_path(&inspection.value, &["State"])?;
    let exit_code = state
        .get("ExitCode")
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok());
    let oom_killed = state
        .get("OOMKilled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let runtime_error = state
        .get("Error")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    Ok(ContainerState {
        exit_code,
        oom_killed,
        runtime_error,
    })
}

fn normalize_docker_outcome(
    state: &ContainerState,
    stop: ProcessStop,
) -> (ExecutionOutcome, Option<String>) {
    match stop {
        ProcessStop::TimedOut => (ExecutionOutcome::Error, Some("TIMEOUT".to_owned())),
        ProcessStop::OutputFlood => (
            ExecutionOutcome::Error,
            Some("OUTPUT_LIMIT_EXCEEDED".to_owned()),
        ),
        ProcessStop::Exited if state.oom_killed => {
            (ExecutionOutcome::Error, Some("OUT_OF_MEMORY".to_owned()))
        }
        ProcessStop::Exited if state.runtime_error.is_some() => (
            ExecutionOutcome::Error,
            Some("CONTAINER_RUNTIME_ERROR".to_owned()),
        ),
        ProcessStop::Exited => match state.exit_code {
            Some(0) => (ExecutionOutcome::Pass, None),
            Some(1) => (ExecutionOutcome::Fail, None),
            Some(_) => (
                ExecutionOutcome::Error,
                Some("VERIFIER_EXIT_NON_CONTRACT".to_owned()),
            ),
            None => (
                ExecutionOutcome::Error,
                Some("MISSING_CONTAINER_EXIT_STATUS".to_owned()),
            ),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn build_docker_manifest(
    request: &RunnerRequest<'_>,
    policy: &OciSandboxPolicy,
    container_name: &str,
    environment_binding_digest: Sha256Digest,
    environment: &BTreeMap<String, String>,
    control_facts: &BTreeMap<String, String>,
    stdout: &BoundedOutputStream,
    stderr: &BoundedOutputStream,
    state: &ContainerState,
    stop: ProcessStop,
    wall_clock_millis: u64,
) -> Result<ExecutionManifestV1, RunnerError> {
    let (normalized_outcome, infrastructure_classification) = normalize_docker_outcome(state, stop);
    let input_manifest_digest = domain_separated_digest(
        "hsai.runner-input-manifest.v1",
        &(
            request.candidate.artifact_digest,
            request.candidate.artifact_byte_length,
            request.claim.verification_policy.validator_artifact_digest,
            environment_binding_digest,
            control_facts,
        ),
    )?;
    let output_manifest_digest = domain_separated_digest(
        "hsai.runner-output-manifest.v1",
        &(
            normalized_outcome,
            stdout.digest,
            stdout.byte_count,
            stderr.digest,
            stderr.byte_count,
            state.exit_code,
            stop == ProcessStop::TimedOut,
            state.oom_killed,
            &infrastructure_classification,
        ),
    )?;
    let mut host_compatibility_facts = control_facts.clone();
    host_compatibility_facts.extend([
        (
            "execution_profile".to_owned(),
            "docker-cli-verifier-v1".to_owned(),
        ),
        ("container_removed".to_owned(), "true".to_owned()),
        ("container_name".to_owned(), container_name.to_owned()),
        ("stdout_bytes".to_owned(), stdout.byte_count.to_string()),
        ("stderr_bytes".to_owned(), stderr.byte_count.to_string()),
        (
            "docker_environment_binding_digest".to_owned(),
            environment_binding_digest.to_hex(),
        ),
    ]);
    let actual_argv = ["/usr/bin/env".to_owned(), "-i".to_owned()]
        .into_iter()
        .chain(
            environment
                .iter()
                .map(|(name, value)| format!("{name}={value}")),
        )
        .chain(policy.argv.iter().cloned())
        .collect();
    Ok(ExecutionManifestV1 {
        schema_version: "hsai.execution-manifest.v1".to_owned(),
        run_id: request.run_id.to_owned(),
        claim_spec_digest: request.claim.claim_spec_digest().map_err(|error| {
            RunnerError::UnsafeOciPolicy(format!(
                "invalid frozen claim supplied to Docker runner: {error}"
            ))
        })?,
        candidate_digest: request.candidate.artifact_digest,
        admission_decision_digest: request.admission.decision_digest,
        verifier_digest: request.claim.verification_policy.validator_artifact_digest,
        environment_digest: request.claim.verification_policy.environment_digest,
        runner_implementation: "hsai-docker-cli-verifier".to_owned(),
        runner_build_version: env!("CARGO_PKG_VERSION").to_owned(),
        host_compatibility_facts,
        argv: actual_argv,
        environment: environment.clone(),
        sandbox_policy_digest: request.admission.sandbox_policy_digest,
        input_manifest_digest,
        output_manifest_digest: Some(output_manifest_digest),
        resource_limits_digest: request.admission.resource_budget_digest,
        resource_usage: ResourceUsageV1 {
            cpu_millis: 0,
            peak_memory_bytes: 0,
            output_bytes: stdout.byte_count.saturating_add(stderr.byte_count),
            disk_bytes: 0,
            wall_clock_millis,
        },
        started_at: request.started_at.clone(),
        ended_at: request.ended_at.clone(),
        logical_time: request.logical_time.clone(),
        raw_exit_status: state.exit_code,
        signal: None,
        timed_out: stop == ProcessStop::TimedOut,
        out_of_memory: state.oom_killed,
        infrastructure_classification,
        stdout_digest: stdout.digest,
        stderr_digest: stderr.digest,
        normalized_outcome,
        normalization_policy_version: request
            .claim
            .verification_policy
            .output_normalization_version
            .clone(),
        determinism_tier: request.claim.verification_policy.determinism_tier,
        declared_nondeterminism: Vec::new(),
    })
}

fn remove_probe_container(container_name: &str, force: bool) {
    let mut command = Command::new("docker");
    command.arg("rm");
    if force {
        command.arg("--force");
    }
    let _ = command
        .arg(container_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn format_cpu_quota(cpu_millis: u32) -> String {
    format!("{}.{:03}", cpu_millis / 1_000, cpu_millis % 1_000)
}

fn build_manifest(
    request: &RunnerRequest<'_>,
    outcome: ExecutionOutcome,
) -> Result<ExecutionManifestV1, RunnerError> {
    let stdout_digest = domain_separated_digest("hsai.runner-stdout.v1", &format!("{outcome:?}"))?;
    let stderr_digest = domain_separated_digest("hsai.runner-stderr.v1", &"")?;
    let input_manifest_digest = domain_separated_digest(
        "hsai.runner-input-manifest.v1",
        &(
            request.candidate.artifact_digest,
            request.candidate.artifact_byte_length,
        ),
    )?;
    let output_manifest_digest = domain_separated_digest(
        "hsai.runner-output-manifest.v1",
        &(outcome, stdout_digest, stderr_digest),
    )?;
    Ok(ExecutionManifestV1 {
        schema_version: "hsai.execution-manifest.v1".to_owned(),
        run_id: request.run_id.to_owned(),
        claim_spec_digest: request.claim.claim_spec_digest().map_err(|error| {
            RunnerError::UnsafeOciPolicy(format!(
                "invalid frozen claim supplied to runner: {error}"
            ))
        })?,
        candidate_digest: request.candidate.artifact_digest,
        admission_decision_digest: request.admission.decision_digest,
        verifier_digest: request.claim.verification_policy.validator_artifact_digest,
        environment_digest: request.claim.verification_policy.environment_digest,
        runner_implementation: "hsai-scripted-runner".to_owned(),
        runner_build_version: env!("CARGO_PKG_VERSION").to_owned(),
        host_compatibility_facts: BTreeMap::from([(
            "execution_profile".to_owned(),
            "hermetic-scripted-fixture".to_owned(),
        )]),
        argv: request.claim.verification_policy.argv.clone(),
        environment: BTreeMap::from([
            (
                "LANG".to_owned(),
                request.claim.verification_policy.locale.clone(),
            ),
            (
                "TZ".to_owned(),
                request.claim.verification_policy.timezone.clone(),
            ),
        ]),
        sandbox_policy_digest: request.admission.sandbox_policy_digest,
        input_manifest_digest,
        output_manifest_digest: Some(output_manifest_digest),
        resource_limits_digest: request.admission.resource_budget_digest,
        resource_usage: ResourceUsageV1 {
            cpu_millis: 1,
            peak_memory_bytes: 1_048_576,
            output_bytes: 16,
            disk_bytes: 0,
            wall_clock_millis: 1,
        },
        started_at: request.started_at.clone(),
        ended_at: request.ended_at.clone(),
        logical_time: request.logical_time.clone(),
        raw_exit_status: (outcome != ExecutionOutcome::Error).then_some(0),
        signal: None,
        timed_out: false,
        out_of_memory: false,
        infrastructure_classification: (outcome == ExecutionOutcome::Error)
            .then(|| "INJECTED_RUNNER_ERROR".to_owned()),
        stdout_digest,
        stderr_digest,
        normalized_outcome: outcome,
        normalization_policy_version: request
            .claim
            .verification_policy
            .output_normalization_version
            .clone(),
        determinism_tier: request.claim.verification_policy.determinism_tier,
        declared_nondeterminism: Vec::new(),
    })
}

fn plain_digest(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use outcomes_domain::{ClaimId, fixtures::local_demo_claim_draft};

    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn candidate(claim_id: ClaimId, bytes: &[u8]) -> Result<CandidateEnvelopeV1, RunnerError> {
        let digest = plain_digest(bytes);
        Ok(CandidateEnvelopeV1 {
            schema_version: "hsai.candidate-envelope.v1".to_owned(),
            candidate_id: "candidate-test".to_owned(),
            claim_id,
            source_namespace: "local.demo".to_owned(),
            source_candidate_ref: "submission-1".to_owned(),
            source_revision: "1".to_owned(),
            author_or_team_id: None,
            observed_source_time: timestamp("2026-07-10T10:30:00Z").map_err(|error| {
                RunnerError::UnsafeOciPolicy(format!("bad test timestamp: {error}"))
            })?,
            received_at: timestamp("2026-07-10T10:30:01Z").map_err(|error| {
                RunnerError::UnsafeOciPolicy(format!("bad test timestamp: {error}"))
            })?,
            artifact_digest: digest,
            artifact_byte_length: bytes.len() as u64,
            artifact_media_type: "application/octet-stream".to_owned(),
            source_provenance_digest: digest,
        })
    }

    #[test]
    fn contract_pass_fail_and_error_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        for (index, (bytes, expected)) in [
            (b"hsai-pass-v1".as_slice(), ExecutionOutcome::Pass),
            (b"other".as_slice(), ExecutionOutcome::Fail),
            (b"hsai-error-v1".as_slice(), ExecutionOutcome::Error),
        ]
        .into_iter()
        .enumerate()
        {
            let candidate = candidate(claim.claim_id.clone(), bytes)?;
            let admission = evaluate_admission(&claim, &candidate)?;
            let started = timestamp("2026-07-10T10:31:00Z")?;
            let ended = timestamp("2026-07-10T10:31:01Z")?;
            let mut runner = ScriptedRunner::exact_bytes_fixture();
            let manifest = runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: bytes,
                admission: &admission,
                run_id: &format!("run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            })?;
            assert_eq!(manifest.normalized_outcome, expected);
        }
        Ok(())
    }

    #[test]
    fn contract_denied_admission_blocks_runner() -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let mut candidate = candidate(claim.claim_id.clone(), b"hsai-pass-v1")?;
        candidate.artifact_media_type = "application/x-denied".to_owned();
        let admission = evaluate_admission(&claim, &candidate)?;
        assert_eq!(admission.decision, AdmissionDecisionKind::Deny);
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;
        let mut runner = ScriptedRunner::exact_bytes_fixture();
        assert!(matches!(
            runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: b"hsai-pass-v1",
                admission: &admission,
                run_id: "run-denied",
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            }),
            Err(RunnerError::AdmissionBlocked(AdmissionDecisionKind::Deny))
        ));
        Ok(())
    }

    #[test]
    fn contract_divergent_replays_do_not_establish_outcome()
    -> Result<(), Box<dyn std::error::Error>> {
        let claim = local_demo_claim_draft()?.freeze()?;
        let candidate = candidate(claim.claim_id.clone(), b"hsai-pass-v1")?;
        let admission = evaluate_admission(&claim, &candidate)?;
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;
        let mut runner = ScriptedRunner::new(vec![ExecutionOutcome::Pass, ExecutionOutcome::Fail]);
        let mut manifests = Vec::new();
        for index in 0..2 {
            manifests.push(runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: b"hsai-pass-v1",
                admission: &admission,
                run_id: &format!("run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            })?);
        }
        let assessment = assess_replays(&manifests, DeterminismTier::SemanticIdentical);
        assert!(!assessment.agreed);
        assert_eq!(assessment.normalized_outcome, None);
        Ok(())
    }

    #[test]
    fn contract_oci_policy_rejects_root_and_socket_mounts() {
        let unsafe_policy = OciSandboxPolicy {
            image_repository: "example.invalid/validator".to_owned(),
            image_digest: format!("sha256:{}", "a".repeat(64)),
            platform: "linux/arm64".to_owned(),
            user_id: 0,
            read_only_root: true,
            drop_all_capabilities: true,
            network_disabled: true,
            host_socket_mounted: true,
            argv: vec!["/validator/check".to_owned()],
            environment_allowlist: Vec::new(),
            pids_limit: 32,
            memory_bytes: 64 * 1024 * 1024,
            cpu_millis: 250,
            scratch_bytes: 4 * 1024 * 1024,
            timeout_millis: 5_000,
        };
        assert!(validate_oci_policy(&unsafe_policy).is_err());
    }

    #[test]
    fn contract_docker_arguments_enforce_controls_without_shell_or_mounts()
    -> Result<(), Box<dyn std::error::Error>> {
        let policy = OciSandboxPolicy {
            image_repository: "alpine".to_owned(),
            image_digest: format!("sha256:{}", "a".repeat(64)),
            platform: "linux/arm64".to_owned(),
            user_id: 65_534,
            read_only_root: true,
            drop_all_capabilities: true,
            network_disabled: true,
            host_socket_mounted: false,
            argv: vec!["/bin/true".to_owned()],
            environment_allowlist: vec!["LANG".to_owned(), "TZ".to_owned()],
            pids_limit: 32,
            memory_bytes: 64 * 1024 * 1024,
            cpu_millis: 250,
            scratch_bytes: 4 * 1024 * 1024,
            timeout_millis: 5_000,
        };
        let arguments = docker_create_args(&policy, "hsai-test-probe")?;
        for required in [
            "none",
            "--read-only",
            "--cap-drop",
            "ALL",
            "no-new-privileges",
            "--pids-limit",
            "--memory",
            "--cpus",
            "--tmpfs",
        ] {
            assert!(arguments.iter().any(|argument| argument == required));
        }
        for forbidden in [
            "--volume",
            "--mount",
            "/var/run/docker.sock",
            "/bin/sh",
            "-c",
        ] {
            assert!(!arguments.iter().any(|argument| argument == forbidden));
        }
        assert_eq!(
            arguments.last().map(String::as_str),
            Some("alpine@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        Ok(())
    }

    fn verifier_policy(argv: Vec<String>) -> OciSandboxPolicy {
        OciSandboxPolicy {
            image_repository: "alpine".to_owned(),
            image_digest: format!("sha256:{}", "a".repeat(64)),
            platform: "linux/arm64".to_owned(),
            user_id: 65_534,
            read_only_root: true,
            drop_all_capabilities: true,
            network_disabled: true,
            host_socket_mounted: false,
            argv,
            environment_allowlist: vec!["LANG".to_owned(), "TZ".to_owned()],
            pids_limit: 16,
            memory_bytes: 64 * 1024 * 1024,
            cpu_millis: 250,
            scratch_bytes: 4 * 1024 * 1024,
            timeout_millis: 5_000,
        }
    }

    #[test]
    fn contract_verifier_arguments_use_exact_env_and_two_read_only_files()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate_file = read_only_input_file("candidate-test", b"same")?;
        let verifier_file = read_only_input_file("verifier-test", b"same")?;
        let policy = verifier_policy(vec![
            "/usr/bin/cmp".to_owned(),
            CONTAINER_CANDIDATE_PATH.to_owned(),
            CONTAINER_VERIFIER_PATH.to_owned(),
        ]);
        let environment = BTreeMap::from([
            ("LANG".to_owned(), "C.UTF-8".to_owned()),
            ("TZ".to_owned(), "UTC".to_owned()),
        ]);
        let arguments = docker_verifier_create_args(
            &policy,
            "hsai-verifier-test",
            candidate_file.path(),
            verifier_file.path(),
            &environment,
        )?;
        assert!(
            arguments
                .windows(2)
                .any(|values| values == ["--pull", "never"])
        );
        assert!(arguments.iter().any(|value| {
            value.starts_with("type=bind,src=")
                && value.ends_with(&format!("dst={CONTAINER_CANDIDATE_PATH},readonly"))
        }));
        assert!(arguments.iter().any(|value| {
            value.starts_with("type=bind,src=")
                && value.ends_with(&format!("dst={CONTAINER_VERIFIER_PATH},readonly"))
        }));
        let image_index = arguments
            .iter()
            .position(|value| value.starts_with("alpine@sha256:"))
            .ok_or("missing image")?;
        assert_eq!(
            &arguments[image_index + 1..],
            [
                "-i",
                "LANG=C.UTF-8",
                "TZ=UTC",
                "/usr/bin/cmp",
                CONTAINER_CANDIDATE_PATH,
                CONTAINER_VERIFIER_PATH,
            ]
        );
        assert!(
            !arguments
                .iter()
                .any(|value| value == "/bin/sh" || value == "-c")
        );
        Ok(())
    }

    #[test]
    fn contract_shell_executable_is_rejected() {
        let policy = verifier_policy(vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "cmp /input/candidate /verifier/expected".to_owned(),
        ]);
        assert!(matches!(
            validate_oci_policy(&policy),
            Err(RunnerError::UnsafeOciPolicy(reason)) if reason.contains("shell")
        ));
    }

    #[test]
    fn contract_exit_mapping_preserves_pass_fail_and_infrastructure_errors() {
        let state = |exit_code, oom_killed, runtime_error| ContainerState {
            exit_code,
            oom_killed,
            runtime_error,
        };
        assert_eq!(
            normalize_docker_outcome(&state(Some(0), false, None), ProcessStop::Exited),
            (ExecutionOutcome::Pass, None)
        );
        assert_eq!(
            normalize_docker_outcome(&state(Some(1), false, None), ProcessStop::Exited),
            (ExecutionOutcome::Fail, None)
        );
        assert_eq!(
            normalize_docker_outcome(&state(Some(9), false, None), ProcessStop::Exited),
            (
                ExecutionOutcome::Error,
                Some("VERIFIER_EXIT_NON_CONTRACT".to_owned())
            )
        );
        assert_eq!(
            normalize_docker_outcome(&state(Some(137), true, None), ProcessStop::Exited),
            (ExecutionOutcome::Error, Some("OUT_OF_MEMORY".to_owned()))
        );
        assert_eq!(
            normalize_docker_outcome(&state(None, false, None), ProcessStop::TimedOut),
            (ExecutionOutcome::Error, Some("TIMEOUT".to_owned()))
        );
    }

    #[test]
    fn contract_output_flood_is_fully_hashed_but_publicly_bounded()
    -> Result<(), Box<dyn std::error::Error>> {
        let bytes = vec![b'x'; 65_537];
        let total = AtomicU64::new(0);
        let flooded = AtomicBool::new(false);
        let output =
            drain_output_stream(std::io::Cursor::new(&bytes), &total, &flooded, 1_024, 32)?;
        assert_eq!(output.byte_count, bytes.len() as u64);
        assert_eq!(output.digest, plain_digest(&bytes));
        assert_eq!(output.public_preview, vec![b'x'; 32]);
        assert!(flooded.load(Ordering::Acquire));
        assert_eq!(
            normalize_docker_outcome(
                &ContainerState {
                    exit_code: None,
                    oom_killed: false,
                    runtime_error: None,
                },
                ProcessStop::OutputFlood,
            ),
            (
                ExecutionOutcome::Error,
                Some("OUTPUT_LIMIT_EXCEEDED".to_owned())
            )
        );
        Ok(())
    }

    #[test]
    fn contract_cleanup_is_forceful_and_name_scoped() {
        assert_eq!(
            docker_force_remove_args("hsai-verifier-123"),
            ["rm", "--force", "hsai-verifier-123"]
        );
    }

    #[test]
    fn contract_constructor_requires_fixed_input_paths() {
        let policy = verifier_policy(vec!["/usr/bin/cmp".to_owned()]);
        assert!(matches!(
            DockerVerificationRunner::new(policy, b"expected".to_vec()),
            Err(RunnerError::UnsafeOciPolicy(reason)) if reason.contains("fixed read-only input paths")
        ));
    }

    fn binding_fixture()
    -> Result<(OciSandboxPolicy, VerificationPolicyV1), Box<dyn std::error::Error>> {
        let argv = vec![
            "/usr/bin/cmp".to_owned(),
            CONTAINER_CANDIDATE_PATH.to_owned(),
            CONTAINER_VERIFIER_PATH.to_owned(),
        ];
        let policy = verifier_policy(argv.clone());
        let mut verification_policy = local_demo_claim_draft()?.verification_policy;
        verification_policy.argv = argv;
        verification_policy.resource_limits.wall_clock_millis = policy.timeout_millis;
        Ok((policy, verification_policy))
    }

    #[test]
    fn contract_docker_environment_binding_has_stable_golden_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        let (policy, verification_policy) = binding_fixture()?;
        let binding = docker_environment_binding(
            &policy,
            &verification_policy.locale,
            &verification_policy.timezone,
        )?;
        assert_eq!(binding.schema_version, "hsai.docker-environment-binding.v1");
        assert_eq!(binding.environment["LANG"], "C.UTF-8");
        assert_eq!(binding.environment["TZ"], "UTC");
        assert_eq!(
            docker_environment_digest(
                &policy,
                &verification_policy.locale,
                &verification_policy.timezone,
            )?
            .to_hex(),
            "3e3bc79af396f6151769f1cdf4470b30a30028bf5a6b2ed8f2c1e9680add1190"
        );
        Ok(())
    }

    #[test]
    fn contract_every_mutable_environment_identity_field_changes_or_fails_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let (policy, verification_policy) = binding_fixture()?;
        let baseline = docker_environment_digest(
            &policy,
            &verification_policy.locale,
            &verification_policy.timezone,
        )?;

        let mut mutations = Vec::new();
        let mut changed = policy.clone();
        changed.image_repository = "example.invalid/other".to_owned();
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        changed.image_digest = format!("sha256:{}", "b".repeat(64));
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        let mut changed_verification = verification_policy.clone();
        changed.platform = "linux/amd64".to_owned();
        changed_verification.cpu_architecture = "x86_64".to_owned();
        mutations.push((changed, changed_verification));
        let mut changed = policy.clone();
        changed.user_id = 65_533;
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        let mut changed_verification = verification_policy.clone();
        changed.argv.push("--strict".to_owned());
        changed_verification.argv.push("--strict".to_owned());
        mutations.push((changed, changed_verification));
        let mut changed_verification = verification_policy.clone();
        changed_verification.locale = "C".to_owned();
        mutations.push((policy.clone(), changed_verification));
        let mut changed_verification = verification_policy.clone();
        changed_verification.timezone = "Etc/UTC".to_owned();
        mutations.push((policy.clone(), changed_verification));
        let mut changed = policy.clone();
        changed.pids_limit += 1;
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        changed.memory_bytes += 4_096;
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        changed.cpu_millis += 1;
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        changed.scratch_bytes += 4_096;
        mutations.push((changed, verification_policy.clone()));
        let mut changed = policy.clone();
        changed.timeout_millis -= 1;
        mutations.push((changed, verification_policy.clone()));

        let mut observed = std::collections::BTreeSet::new();
        observed.insert(baseline.to_hex());
        for (changed_policy, changed_verification) in mutations {
            observed.insert(
                docker_environment_digest(
                    &changed_policy,
                    &changed_verification.locale,
                    &changed_verification.timezone,
                )?
                .to_hex(),
            );
        }
        assert_eq!(observed.len(), 13);

        let mut no_read_only = policy.clone();
        no_read_only.read_only_root = false;
        let mut capabilities_retained = policy.clone();
        capabilities_retained.drop_all_capabilities = false;
        let mut network_enabled = policy.clone();
        network_enabled.network_disabled = false;
        let mut socket_mounted = policy.clone();
        socket_mounted.host_socket_mounted = true;
        for unsafe_policy in [
            no_read_only,
            capabilities_retained,
            network_enabled,
            socket_mounted,
        ] {
            assert!(
                docker_environment_digest(
                    &unsafe_policy,
                    &verification_policy.locale,
                    &verification_policy.timezone,
                )
                .is_err()
            );
        }
        let mut wrong_allowlist = policy.clone();
        wrong_allowlist.environment_allowlist = vec!["LANG".to_owned()];
        assert!(
            docker_environment_digest(
                &wrong_allowlist,
                &verification_policy.locale,
                &verification_policy.timezone,
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn contract_environment_mismatch_is_rejected_before_docker_spawn()
    -> Result<(), Box<dyn std::error::Error>> {
        let expected = b"expected";
        let (policy, mut verification_policy) = binding_fixture()?;
        verification_policy.validator_artifact_digest = plain_digest(expected);
        let mut draft = local_demo_claim_draft()?;
        draft.verification_policy = verification_policy;
        // Deliberately retain a digest unrelated to the Docker binding.
        draft.verification_policy.environment_digest = Sha256Digest::from_bytes([9; 32]);
        let claim = draft.freeze()?;
        let candidate = candidate(claim.claim_id.clone(), expected)?;
        let admission = evaluate_admission(&claim, &candidate)?;
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;
        let mut runner = DockerVerificationRunner::new(policy, expected.to_vec())?;
        runner.docker_binary = PathBuf::from("/definitely/absent/hsai-docker-test");
        assert!(matches!(
            runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: expected,
                admission: &admission,
                run_id: "docker-identity-mismatch",
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            }),
            Err(RunnerError::DockerEnvironmentIdentityMismatch)
        ));
        assert!(runner.last_public_output().is_none());
        Ok(())
    }

    #[test]
    fn contract_platform_and_claim_architecture_must_match()
    -> Result<(), Box<dyn std::error::Error>> {
        let (mut policy, verification_policy) = binding_fixture()?;
        policy.platform = "linux/amd64".to_owned();
        assert!(matches!(
            validate_docker_policy_against_verification(&policy, &verification_policy),
            Err(RunnerError::UnsafeOciPolicy(reason)) if reason.contains("architecture")
        ));
        policy.platform = "linux/arm/v7".to_owned();
        assert!(
            docker_environment_digest(
                &policy,
                &verification_policy.locale,
                &verification_policy.timezone,
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    #[ignore = "requires an explicitly selected preloaded local Alpine image"]
    fn external_docker_verifier_pass_fail_and_cleanup() -> Result<(), Box<dyn std::error::Error>> {
        let (Ok(image_repository), Ok(image_digest), Ok(platform)) = (
            std::env::var("HSAI_TEST_OCI_IMAGE_REPOSITORY"),
            std::env::var("HSAI_TEST_OCI_IMAGE_DIGEST"),
            std::env::var("HSAI_TEST_OCI_PLATFORM"),
        ) else {
            return Ok(());
        };
        let expected = b"hsai-docker-verifier-expected\n";
        let argv = vec![
            "/usr/bin/cmp".to_owned(),
            CONTAINER_CANDIDATE_PATH.to_owned(),
            CONTAINER_VERIFIER_PATH.to_owned(),
        ];
        let mut draft = local_demo_claim_draft()?;
        draft.verification_policy.validator_artifact_digest = plain_digest(expected);
        draft.verification_policy.argv.clone_from(&argv);
        draft.verification_policy.cpu_architecture = platform
            .strip_prefix("linux/")
            .unwrap_or(&platform)
            .to_owned();
        draft.verification_policy.resource_limits.wall_clock_millis = 5_000;
        let mut policy = verifier_policy(argv);
        policy.image_repository = image_repository;
        policy.image_digest = image_digest;
        policy.platform = platform;
        draft.verification_policy.environment_digest = docker_environment_digest(
            &policy,
            &draft.verification_policy.locale,
            &draft.verification_policy.timezone,
        )?;
        let claim = draft.freeze()?;
        let mut runner = DockerVerificationRunner::new(policy, expected.to_vec())?;
        let started = timestamp("2026-07-10T10:31:00Z")?;
        let ended = timestamp("2026-07-10T10:31:01Z")?;

        for (index, (candidate_bytes, expected_outcome, expected_exit)) in [
            (expected.as_slice(), ExecutionOutcome::Pass, 0),
            (b"different\n".as_slice(), ExecutionOutcome::Fail, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let candidate = candidate(claim.claim_id.clone(), candidate_bytes)?;
            let admission = evaluate_admission(&claim, &candidate)?;
            let manifest = runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes,
                admission: &admission,
                run_id: &format!("docker-run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            })?;
            assert_eq!(manifest.normalized_outcome, expected_outcome);
            assert_eq!(manifest.raw_exit_status, Some(expected_exit));
            assert_eq!(manifest.runner_implementation, "hsai-docker-cli-verifier");
            assert_eq!(
                manifest.host_compatibility_facts["container_removed"],
                "true"
            );
            let name = &manifest.host_compatibility_facts["container_name"];
            let inspect = Command::new("docker")
                .args(["inspect", name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            assert!(!inspect.success());
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires an explicitly selected preloaded local Alpine image"]
    fn external_docker_verifier_timeout_output_flood_and_cleanup()
    -> Result<(), Box<dyn std::error::Error>> {
        let (Ok(image_repository), Ok(image_digest), Ok(platform)) = (
            std::env::var("HSAI_TEST_OCI_IMAGE_REPOSITORY"),
            std::env::var("HSAI_TEST_OCI_IMAGE_DIGEST"),
            std::env::var("HSAI_TEST_OCI_PLATFORM"),
        ) else {
            return Ok(());
        };
        let expected = b"hsai-docker-verifier-expected\n";
        let argv = vec![
            "/usr/bin/yes".to_owned(),
            CONTAINER_CANDIDATE_PATH.to_owned(),
            CONTAINER_VERIFIER_PATH.to_owned(),
        ];
        for (index, (timeout_millis, output_bytes, classification)) in [
            (25, 256 * 1024 * 1024, "TIMEOUT"),
            (5_000, 1_024, "OUTPUT_LIMIT_EXCEEDED"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut draft = local_demo_claim_draft()?;
            draft.verification_policy.validator_artifact_digest = plain_digest(expected);
            draft.verification_policy.argv.clone_from(&argv);
            draft.verification_policy.cpu_architecture = platform
                .strip_prefix("linux/")
                .unwrap_or(&platform)
                .to_owned();
            draft.verification_policy.resource_limits.wall_clock_millis = timeout_millis;
            draft.verification_policy.resource_limits.output_bytes = output_bytes;
            let mut policy = verifier_policy(argv.clone());
            policy.image_repository.clone_from(&image_repository);
            policy.image_digest.clone_from(&image_digest);
            policy.platform.clone_from(&platform);
            policy.timeout_millis = timeout_millis;
            draft.verification_policy.environment_digest = docker_environment_digest(
                &policy,
                &draft.verification_policy.locale,
                &draft.verification_policy.timezone,
            )?;
            let claim = draft.freeze()?;
            let candidate = candidate(claim.claim_id.clone(), expected)?;
            let admission = evaluate_admission(&claim, &candidate)?;
            let mut runner = DockerVerificationRunner::new(policy, expected.to_vec())?
                .with_public_preview_limit(64);
            let started = timestamp("2026-07-10T10:31:00Z")?;
            let ended = timestamp("2026-07-10T10:31:01Z")?;
            let manifest = runner.execute(&RunnerRequest {
                claim: &claim,
                candidate: &candidate,
                candidate_bytes: expected,
                admission: &admission,
                run_id: &format!("docker-limit-run-{index}"),
                started_at: &started,
                ended_at: &ended,
                logical_time: &started,
            })?;
            assert_eq!(manifest.normalized_outcome, ExecutionOutcome::Error);
            assert_eq!(
                manifest.infrastructure_classification.as_deref(),
                Some(classification)
            );
            assert_eq!(manifest.timed_out, classification == "TIMEOUT");
            let output = runner
                .last_public_output()
                .ok_or("missing bounded output")?;
            assert!(output.stdout.public_preview.len() <= 64);
            let name = &manifest.host_compatibility_facts["container_name"];
            let inspect = Command::new("docker")
                .args(["inspect", name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            assert!(!inspect.success());
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires an explicitly selected preloaded local Docker image"]
    fn external_docker_daemon_enforces_oci_probe() -> Result<(), Box<dyn std::error::Error>> {
        let image_repository = std::env::var("HSAI_TEST_OCI_IMAGE_REPOSITORY")?;
        let image_digest = std::env::var("HSAI_TEST_OCI_IMAGE_DIGEST")?;
        let platform = std::env::var("HSAI_TEST_OCI_PLATFORM")?;
        let policy = OciSandboxPolicy {
            image_repository,
            image_digest,
            platform,
            user_id: 65_534,
            read_only_root: true,
            drop_all_capabilities: true,
            network_disabled: true,
            host_socket_mounted: false,
            argv: vec!["/bin/true".to_owned()],
            environment_allowlist: Vec::new(),
            pids_limit: 32,
            memory_bytes: 64 * 1024 * 1024,
            cpu_millis: 250,
            scratch_bytes: 4 * 1024 * 1024,
            timeout_millis: 5_000,
        };
        let result = run_docker_oci_probe(&policy)?;
        assert!(result.success);
        assert!(result.wall_clock_millis <= policy.timeout_millis);
        Ok(())
    }
}
