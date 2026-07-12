//! Deterministic retained-artifact inventory for controlled local beta runs.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use outcomes_domain::{Sha256Digest, domain_separated_digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const INVENTORY_FILE: &str = "beta-artifact-inventory.json";
const INVENTORY_SCHEMA: &str = "hsai.local-beta-artifact-inventory.v1";
const INVENTORY_DOMAIN: &str = "hsai.local-beta-artifact-inventory.v1";
const MANIFEST_FILE: &str = "beta-run-manifest.json";
const PROFILE_FILE: &str = "local-beta-profile.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryError {
    pub code: &'static str,
    pub message: String,
}

impl InventoryError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for InventoryError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactInventoryEntryV1 {
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactInventoryBodyV1 {
    schema_version: String,
    run_id: String,
    manifest_digest: Sha256Digest,
    entries: Vec<ArtifactInventoryEntryV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactInventoryV1 {
    pub schema_version: String,
    pub run_id: String,
    pub manifest_digest: Sha256Digest,
    pub entries: Vec<ArtifactInventoryEntryV1>,
    pub inventory_digest: Sha256Digest,
}

impl ArtifactInventoryV1 {
    fn body(&self) -> ArtifactInventoryBodyV1 {
        ArtifactInventoryBodyV1 {
            schema_version: self.schema_version.clone(),
            run_id: self.run_id.clone(),
            manifest_digest: self.manifest_digest,
            entries: self.entries.clone(),
        }
    }
}

/// Seal a successful beta run with an exact-set content inventory.
///
/// State slices: `EvidenceStoreV1`, `OperationsV1`, and `DeliveryV1` retained
/// artifact verification. This function writes only the inventory file.
pub fn finalize_run_inventory(
    run_root: &Path,
    run_id: &str,
    expected_profile_digest: Sha256Digest,
) -> Result<ArtifactInventoryV1, InventoryError> {
    let inventory_path = run_root.join(INVENTORY_FILE);
    if fs::symlink_metadata(&inventory_path).is_ok() {
        return Err(InventoryError::new(
            "BETA_ALREADY_FINALIZED",
            "inventory already exists; verify the sealed run instead",
        ));
    }
    let manifest_digest = validate_run_prerequisites(run_root, run_id, expected_profile_digest)?;
    let entries = scan_entries(run_root)?;
    let body = ArtifactInventoryBodyV1 {
        schema_version: INVENTORY_SCHEMA.to_owned(),
        run_id: run_id.to_owned(),
        manifest_digest,
        entries,
    };
    let inventory_digest = domain_separated_digest(INVENTORY_DOMAIN, &body).map_err(|error| {
        InventoryError::new("BETA_INVENTORY_ENCODING_FAILED", error.to_string())
    })?;
    let inventory = ArtifactInventoryV1 {
        schema_version: body.schema_version,
        run_id: body.run_id,
        manifest_digest: body.manifest_digest,
        entries: body.entries,
        inventory_digest,
    };
    let bytes = serde_json::to_vec_pretty(&inventory).map_err(|error| {
        InventoryError::new("BETA_INVENTORY_ENCODING_FAILED", error.to_string())
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&inventory_path)
        .map_err(|error| InventoryError::new("BETA_INVENTORY_CREATE_FAILED", error.to_string()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| InventoryError::new("BETA_INVENTORY_WRITE_FAILED", error.to_string()))?;
    Ok(inventory)
}

/// Verify a sealed beta inventory without modifying the run directory.
pub fn verify_run_inventory(
    run_root: &Path,
    run_id: &str,
    expected_profile_digest: Sha256Digest,
) -> Result<ArtifactInventoryV1, InventoryError> {
    let inventory_path = run_root.join(INVENTORY_FILE);
    require_regular_file(&inventory_path, "BETA_INVENTORY_MISSING")?;
    let inventory: ArtifactInventoryV1 = read_json(&inventory_path, "BETA_INVENTORY_INVALID")?;
    if inventory.schema_version != INVENTORY_SCHEMA || inventory.run_id != run_id {
        return Err(InventoryError::new(
            "BETA_INVENTORY_IDENTITY_MISMATCH",
            "inventory schema or run ID does not match the requested run",
        ));
    }
    validate_ordered_entries(&inventory.entries)?;
    let computed_inventory_digest = domain_separated_digest(INVENTORY_DOMAIN, &inventory.body())
        .map_err(|error| {
            InventoryError::new("BETA_INVENTORY_ENCODING_FAILED", error.to_string())
        })?;
    if computed_inventory_digest != inventory.inventory_digest {
        return Err(InventoryError::new(
            "BETA_INVENTORY_DIGEST_MISMATCH",
            "inventory body digest does not match",
        ));
    }
    let manifest_digest = validate_run_prerequisites(run_root, run_id, expected_profile_digest)?;
    if manifest_digest != inventory.manifest_digest {
        return Err(InventoryError::new(
            "BETA_MANIFEST_DIGEST_MISMATCH",
            "manifest digest differs from the sealed inventory",
        ));
    }
    let observed = scan_entries(run_root)?;
    if observed != inventory.entries {
        return Err(InventoryError::new(
            "BETA_ARTIFACT_SET_MISMATCH",
            "retained file set, lengths, or content digests differ from the inventory",
        ));
    }
    Ok(inventory)
}

fn validate_run_prerequisites(
    run_root: &Path,
    run_id: &str,
    expected_profile_digest: Sha256Digest,
) -> Result<Sha256Digest, InventoryError> {
    let root_metadata = fs::symlink_metadata(run_root)
        .map_err(|error| InventoryError::new("BETA_RUN_MISSING", error.to_string()))?;
    if !root_metadata.file_type().is_dir() || root_metadata.file_type().is_symlink() {
        return Err(InventoryError::new(
            "BETA_RUN_UNSAFE_ROOT",
            "run root must be a real directory",
        ));
    }
    let manifest_path = run_root.join(MANIFEST_FILE);
    require_regular_file(&manifest_path, "BETA_MANIFEST_MISSING")?;
    let manifest: Value = read_json(&manifest_path, "BETA_MANIFEST_INVALID")?;
    if manifest.get("status").and_then(Value::as_str) != Some("PASS")
        || manifest.get("run_id").and_then(Value::as_str) != Some(run_id)
    {
        return Err(InventoryError::new(
            "BETA_MANIFEST_NOT_FINALIZABLE",
            "manifest must be PASS and bind the requested run ID",
        ));
    }
    let profile_path = run_root.join(PROFILE_FILE);
    require_regular_file(&profile_path, "BETA_PROFILE_MISSING")?;
    let profile: Value = read_json(&profile_path, "BETA_PROFILE_INVALID")?;
    let profile_domain = match profile.get("schema_version").and_then(Value::as_str) {
        Some("hsai.local-beta-profile.v1") => "hsai.local-beta-profile.v1",
        Some("hsai.local-beta-profile.v2") => "hsai.local-beta-profile.v2",
        _ => {
            return Err(InventoryError::new(
                "BETA_PROFILE_INVALID",
                "profile schema is not supported by the sealed inventory contract",
            ));
        }
    };
    let observed_profile_digest = domain_separated_digest(profile_domain, &profile)
        .map_err(|error| InventoryError::new("BETA_PROFILE_INVALID", error.to_string()))?;
    let manifest_profile_digest: Sha256Digest =
        serde_json::from_value(manifest.get("profile_digest").cloned().ok_or_else(|| {
            InventoryError::new("BETA_MANIFEST_INVALID", "profile digest missing")
        })?)
        .map_err(|error| InventoryError::new("BETA_MANIFEST_INVALID", error.to_string()))?;
    if observed_profile_digest != expected_profile_digest
        || manifest_profile_digest != expected_profile_digest
    {
        return Err(InventoryError::new(
            "BETA_PROFILE_DIGEST_MISMATCH",
            "copied profile, embedded profile, and manifest digest must match",
        ));
    }
    raw_file_digest(&manifest_path)
}

fn scan_entries(run_root: &Path) -> Result<Vec<ArtifactInventoryEntryV1>, InventoryError> {
    let mut paths = Vec::new();
    collect_files(run_root, run_root, &mut paths)?;
    paths.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    let mut entries = Vec::with_capacity(paths.len());
    for (relative_path, absolute_path) in paths {
        let (sha256, byte_length) = raw_file_identity(&absolute_path)?;
        entries.push(ArtifactInventoryEntryV1 {
            relative_path,
            byte_length,
            sha256,
        });
    }
    validate_ordered_entries(&entries)?;
    Ok(entries)
}

fn collect_files(
    run_root: &Path,
    directory: &Path,
    paths: &mut Vec<(String, PathBuf)>,
) -> Result<(), InventoryError> {
    let entries = fs::read_dir(directory)
        .map_err(|error| InventoryError::new("BETA_ARTIFACT_READ_FAILED", error.to_string()))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| InventoryError::new("BETA_ARTIFACT_READ_FAILED", error.to_string()))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| InventoryError::new("BETA_ARTIFACT_READ_FAILED", error.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(InventoryError::new(
                "BETA_ARTIFACT_UNSAFE_TYPE",
                "symbolic links are forbidden in sealed beta runs",
            ));
        }
        if metadata.file_type().is_dir() {
            collect_files(run_root, &path, paths)?;
        } else if metadata.file_type().is_file() {
            let relative = path.strip_prefix(run_root).map_err(|_| {
                InventoryError::new("BETA_ARTIFACT_UNSAFE_PATH", "file escaped run root")
            })?;
            let normalized = normalize_relative_path(relative)?;
            if normalized != INVENTORY_FILE {
                paths.push((normalized, path));
            }
        } else {
            return Err(InventoryError::new(
                "BETA_ARTIFACT_UNSAFE_TYPE",
                "only regular files and directories are permitted",
            ));
        }
    }
    Ok(())
}

fn normalize_relative_path(path: &Path) -> Result<String, InventoryError> {
    let mut segments = Vec::new();
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(InventoryError::new(
                "BETA_ARTIFACT_UNSAFE_PATH",
                "artifact path must be relative without traversal",
            ));
        };
        let segment = segment.to_str().ok_or_else(|| {
            InventoryError::new("BETA_ARTIFACT_UNSAFE_PATH", "artifact path must be UTF-8")
        })?;
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains('\\') {
            return Err(InventoryError::new(
                "BETA_ARTIFACT_UNSAFE_PATH",
                "artifact path contains an unsafe component",
            ));
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        return Err(InventoryError::new(
            "BETA_ARTIFACT_UNSAFE_PATH",
            "artifact path cannot be empty",
        ));
    }
    Ok(segments.join("/"))
}

fn validate_ordered_entries(entries: &[ArtifactInventoryEntryV1]) -> Result<(), InventoryError> {
    let mut previous: Option<&str> = None;
    for entry in entries {
        let normalized = normalize_relative_path(Path::new(&entry.relative_path))?;
        if normalized != entry.relative_path || entry.relative_path == INVENTORY_FILE {
            return Err(InventoryError::new(
                "BETA_INVENTORY_UNSAFE_PATH",
                "inventory contains a non-canonical or self-referential path",
            ));
        }
        if previous.is_some_and(|value| value.as_bytes() >= entry.relative_path.as_bytes()) {
            return Err(InventoryError::new(
                "BETA_INVENTORY_ORDER_INVALID",
                "inventory paths must be unique and strictly byte-sorted",
            ));
        }
        previous = Some(&entry.relative_path);
    }
    Ok(())
}

fn require_regular_file(path: &Path, missing_code: &'static str) -> Result<(), InventoryError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| InventoryError::new(missing_code, error.to_string()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(InventoryError::new(
            "BETA_ARTIFACT_UNSAFE_TYPE",
            "required artifact must be a regular file",
        ));
    }
    Ok(())
}

fn raw_file_digest(path: &Path) -> Result<Sha256Digest, InventoryError> {
    raw_file_identity(path).map(|(digest, _)| digest)
}

fn raw_file_identity(path: &Path) -> Result<(Sha256Digest, u64), InventoryError> {
    let mut file = File::open(path)
        .map_err(|error| InventoryError::new("BETA_ARTIFACT_READ_FAILED", error.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut byte_length = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| InventoryError::new("BETA_ARTIFACT_READ_FAILED", error.to_string()))?;
        if count == 0 {
            break;
        }
        byte_length = byte_length.checked_add(count as u64).ok_or_else(|| {
            InventoryError::new("BETA_ARTIFACT_TOO_LARGE", "artifact byte length overflowed")
        })?;
        hasher.update(&buffer[..count]);
    }
    Ok((
        Sha256Digest::from_bytes(hasher.finalize().into()),
        byte_length,
    ))
}

fn read_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    code: &'static str,
) -> Result<T, InventoryError> {
    let bytes = fs::read(path).map_err(|error| InventoryError::new(code, error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| InventoryError::new(code, error.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn frozen_profile() -> Value {
        json!({
            "schema_version": "hsai.local-beta-profile.v1",
            "profile_id": "hermetic-trusted-operator-v1",
            "network": "MOCK"
        })
    }

    fn profile_digest() -> Sha256Digest {
        domain_separated_digest("hsai.local-beta-profile.v1", &frozen_profile()).unwrap()
    }

    fn create_run(run_id: &str, order: &[&str]) -> TempDir {
        let root = tempfile::tempdir().unwrap();
        for path in order {
            let target = root.path().join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(target, format!("payload:{path}")).unwrap();
        }
        fs::write(
            root.path().join(PROFILE_FILE),
            serde_json::to_vec_pretty(&frozen_profile()).unwrap(),
        )
        .unwrap();
        fs::write(
            root.path().join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&json!({
                "schema_version": "hsai.local-beta-run-manifest.v1",
                "run_id": run_id,
                "status": "PASS",
                "profile_digest": profile_digest()
            }))
            .unwrap(),
        )
        .unwrap();
        root
    }

    #[test]
    fn finalize_is_creation_order_independent_and_verify_is_read_only() {
        let first = create_run("beta-1", &["z/file", "a/file"]);
        let second = create_run("beta-1", &["a/file", "z/file"]);
        let first_inventory =
            finalize_run_inventory(first.path(), "beta-1", profile_digest()).unwrap();
        let second_inventory =
            finalize_run_inventory(second.path(), "beta-1", profile_digest()).unwrap();
        assert_eq!(first_inventory, second_inventory);
        let before = fs::read(first.path().join(INVENTORY_FILE)).unwrap();
        assert_eq!(
            verify_run_inventory(first.path(), "beta-1", profile_digest()).unwrap(),
            first_inventory
        );
        assert_eq!(before, fs::read(first.path().join(INVENTORY_FILE)).unwrap());
        assert!(
            first_inventory
                .entries
                .iter()
                .any(|entry| entry.relative_path == MANIFEST_FILE)
        );
        assert!(
            first_inventory
                .entries
                .iter()
                .any(|entry| entry.relative_path == PROFILE_FILE)
        );
    }

    #[test]
    fn verify_rejects_mutation_addition_and_deletion() {
        for change in ["mutate", "add", "delete"] {
            let run = create_run("beta-2", &["scenario/evidence.json"]);
            finalize_run_inventory(run.path(), "beta-2", profile_digest()).unwrap();
            match change {
                "mutate" => {
                    fs::write(run.path().join("scenario/evidence.json"), b"changed").unwrap();
                }
                "add" => fs::write(run.path().join("added"), b"added").unwrap(),
                "delete" => fs::remove_file(run.path().join("scenario/evidence.json")).unwrap(),
                _ => unreachable!(),
            }
            assert_eq!(
                verify_run_inventory(run.path(), "beta-2", profile_digest())
                    .unwrap_err()
                    .code,
                "BETA_ARTIFACT_SET_MISMATCH"
            );
        }
    }

    #[test]
    fn verify_rejects_inventory_tamper_and_noncanonical_entries() {
        let run = create_run("beta-3", &["scenario/evidence.json"]);
        finalize_run_inventory(run.path(), "beta-3", profile_digest()).unwrap();
        let path = run.path().join(INVENTORY_FILE);
        let mut inventory: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        inventory["inventory_digest"] = json!(Sha256Digest::from_bytes([9; 32]));
        fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
        assert_eq!(
            verify_run_inventory(run.path(), "beta-3", profile_digest())
                .unwrap_err()
                .code,
            "BETA_INVENTORY_DIGEST_MISMATCH"
        );

        let run = create_run("beta-4", &["a", "b"]);
        finalize_run_inventory(run.path(), "beta-4", profile_digest()).unwrap();
        let path = run.path().join(INVENTORY_FILE);
        let mut inventory: ArtifactInventoryV1 = read_json(&path, "test").unwrap();
        inventory.entries.swap(0, 1);
        inventory.inventory_digest =
            domain_separated_digest(INVENTORY_DOMAIN, &inventory.body()).unwrap();
        fs::write(path, serde_json::to_vec(&inventory).unwrap()).unwrap();
        assert_eq!(
            verify_run_inventory(run.path(), "beta-4", profile_digest())
                .unwrap_err()
                .code,
            "BETA_INVENTORY_ORDER_INVALID"
        );
    }

    #[test]
    fn finalize_rejects_collision_partial_nonpass_and_identity_mismatch() {
        let run = create_run("beta-5", &[]);
        finalize_run_inventory(run.path(), "beta-5", profile_digest()).unwrap();
        assert_eq!(
            finalize_run_inventory(run.path(), "beta-5", profile_digest())
                .unwrap_err()
                .code,
            "BETA_ALREADY_FINALIZED"
        );

        let partial = tempfile::tempdir().unwrap();
        assert_eq!(
            finalize_run_inventory(partial.path(), "partial", profile_digest())
                .unwrap_err()
                .code,
            "BETA_MANIFEST_MISSING"
        );

        let nonpass = create_run("beta-6", &[]);
        let manifest_path = nonpass.path().join(MANIFEST_FILE);
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["status"] = json!("FAILED");
        fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(
            finalize_run_inventory(nonpass.path(), "beta-6", profile_digest())
                .unwrap_err()
                .code,
            "BETA_MANIFEST_NOT_FINALIZABLE"
        );

        let mismatch = create_run("actual", &[]);
        assert_eq!(
            finalize_run_inventory(mismatch.path(), "requested", profile_digest())
                .unwrap_err()
                .code,
            "BETA_MANIFEST_NOT_FINALIZABLE"
        );
    }

    #[cfg(unix)]
    #[test]
    fn finalize_rejects_symlinks_and_nonregular_files() {
        use std::os::unix::{fs::symlink, net::UnixListener};

        let linked = create_run("beta-7", &[]);
        symlink(PROFILE_FILE, linked.path().join("link")).unwrap();
        assert_eq!(
            finalize_run_inventory(linked.path(), "beta-7", profile_digest())
                .unwrap_err()
                .code,
            "BETA_ARTIFACT_UNSAFE_TYPE"
        );

        let socket = create_run("beta-8", &[]);
        let _listener = UnixListener::bind(socket.path().join("socket")).unwrap();
        assert_eq!(
            finalize_run_inventory(socket.path(), "beta-8", profile_digest())
                .unwrap_err()
                .code,
            "BETA_ARTIFACT_UNSAFE_TYPE"
        );
    }

    #[test]
    fn entry_validation_rejects_duplicate_traversal_absolute_and_self_paths() {
        let digest = Sha256Digest::from_bytes([0; 32]);
        for paths in [
            vec!["a", "a"],
            vec!["../escape"],
            vec!["/absolute"],
            vec![INVENTORY_FILE],
            vec!["b", "a"],
        ] {
            let entries = paths
                .into_iter()
                .map(|relative_path| ArtifactInventoryEntryV1 {
                    relative_path: relative_path.to_owned(),
                    byte_length: 0,
                    sha256: digest,
                })
                .collect::<Vec<_>>();
            assert!(validate_ordered_entries(&entries).is_err());
        }
    }
}
