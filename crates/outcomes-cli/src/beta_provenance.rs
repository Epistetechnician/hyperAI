//! Bounded source and tool provenance for controlled local beta runs.

use std::{fs, path::Path, process::Command};

use outcomes_domain::Sha256Digest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const INPUT_FILES: [(&str, &str); 4] = [
    ("cargo_lock_sha256", "Cargo.lock"),
    ("pnpm_lock_sha256", "pnpm-lock.yaml"),
    ("rust_toolchain_sha256", "rust-toolchain.toml"),
    ("package_json_sha256", "package.json"),
];

/// Sanitized, machine-verifiable provenance for one beta rehearsal.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BetaRunProvenanceV1 {
    pub(crate) schema_version: String,
    pub(crate) source: BetaSourceProvenanceV1,
    pub(crate) inputs: BetaInputProvenanceV1,
    pub(crate) tools: BetaToolProvenanceV1,
}

/// Exact source revision and required clean state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BetaSourceProvenanceV1 {
    pub(crate) git_commit: String,
    pub(crate) tree_state: String,
}

/// Raw byte digests for the allowlisted build inputs.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)]
pub(crate) struct BetaInputProvenanceV1 {
    pub(crate) cargo_lock_sha256: Sha256Digest,
    pub(crate) pnpm_lock_sha256: Sha256Digest,
    pub(crate) rust_toolchain_sha256: Sha256Digest,
    pub(crate) package_json_sha256: Sha256Digest,
}

/// Version-only tool identities. Host and platform details are excluded.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BetaToolProvenanceV1 {
    pub(crate) rustc: String,
    pub(crate) cargo: String,
    pub(crate) pnpm: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProvenanceError {
    pub(crate) code: &'static str,
}

pub(crate) trait BetaProvenanceProvider {
    fn collect(&self, repository_root: &Path) -> Result<BetaRunProvenanceV1, ProvenanceError>;
}

pub(crate) struct CommandBetaProvenanceProvider;

impl BetaProvenanceProvider for CommandBetaProvenanceProvider {
    fn collect(&self, repository_root: &Path) -> Result<BetaRunProvenanceV1, ProvenanceError> {
        let commit_output =
            command_output(repository_root, "git", &["rev-parse", "--verify", "HEAD"])?;
        let git_commit = parse_git_commit(&commit_output)?;
        let status = command_output(
            repository_root,
            "git",
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?;
        if !status.is_empty() {
            return Err(ProvenanceError {
                code: "BETA_SOURCE_DIRTY",
            });
        }

        let rustc = parse_tool_version(
            "rustc",
            &command_output(repository_root, "rustc", &["--version"])?,
        )?;
        let cargo = parse_tool_version(
            "cargo",
            &command_output(repository_root, "cargo", &["--version"])?,
        )?;
        let user_agent = std::env::var("npm_config_user_agent").map_err(|_| ProvenanceError {
            code: "BETA_PNPM_PROVENANCE_UNAVAILABLE",
        })?;
        let pnpm = parse_pnpm_user_agent(&user_agent)?;

        let inputs = hash_allowlisted_inputs(repository_root)?;

        Ok(BetaRunProvenanceV1 {
            schema_version: "hsai.local-beta-run-provenance.v1".to_owned(),
            source: BetaSourceProvenanceV1 {
                git_commit,
                tree_state: "CLEAN".to_owned(),
            },
            inputs,
            tools: BetaToolProvenanceV1 { rustc, cargo, pnpm },
        })
    }
}

/// Validate recorded provenance semantics without consulting the host.
pub(crate) fn validate_recorded_provenance(
    provenance: &BetaRunProvenanceV1,
) -> Result<(), ProvenanceError> {
    if provenance.schema_version != "hsai.local-beta-run-provenance.v1" {
        return Err(ProvenanceError {
            code: "BETA_PROVENANCE_SCHEMA_INVALID",
        });
    }
    if provenance.source.tree_state != "CLEAN" {
        return Err(ProvenanceError {
            code: "BETA_SOURCE_STATE_INVALID",
        });
    }
    parse_git_commit(&provenance.source.git_commit)?;
    for version in [
        &provenance.tools.rustc,
        &provenance.tools.cargo,
        &provenance.tools.pnpm,
    ] {
        validate_version(version)?;
    }
    Ok(())
}

fn hash_allowlisted_inputs(root: &Path) -> Result<BetaInputProvenanceV1, ProvenanceError> {
    let mut digests = Vec::with_capacity(INPUT_FILES.len());
    for (_, relative_path) in INPUT_FILES {
        let bytes = fs::read(root.join(relative_path)).map_err(|_| ProvenanceError {
            code: "BETA_INPUT_PROVENANCE_UNAVAILABLE",
        })?;
        digests.push(Sha256Digest::from_bytes(Sha256::digest(bytes).into()));
    }
    Ok(BetaInputProvenanceV1 {
        cargo_lock_sha256: digests[0],
        pnpm_lock_sha256: digests[1],
        rust_toolchain_sha256: digests[2],
        package_json_sha256: digests[3],
    })
}

fn command_output(root: &Path, program: &str, args: &[&str]) -> Result<String, ProvenanceError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|_| ProvenanceError {
            code: "BETA_TOOL_PROVENANCE_UNAVAILABLE",
        })?;
    if !output.status.success() {
        return Err(ProvenanceError {
            code: "BETA_TOOL_PROVENANCE_UNAVAILABLE",
        });
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| ProvenanceError {
            code: "BETA_TOOL_PROVENANCE_INVALID",
        })
}

pub(crate) fn parse_git_commit(value: &str) -> Result<String, ProvenanceError> {
    let value = value.trim();
    if value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(value.to_owned())
    } else {
        Err(ProvenanceError {
            code: "BETA_GIT_COMMIT_INVALID",
        })
    }
}

pub(crate) fn parse_tool_version(tool: &str, value: &str) -> Result<String, ProvenanceError> {
    let mut parts = value.split_ascii_whitespace();
    if parts.next() != Some(tool) {
        return Err(ProvenanceError {
            code: "BETA_TOOL_VERSION_INVALID",
        });
    }
    let version = parts.next().ok_or(ProvenanceError {
        code: "BETA_TOOL_VERSION_INVALID",
    })?;
    validate_version(version)
}

pub(crate) fn parse_pnpm_user_agent(value: &str) -> Result<String, ProvenanceError> {
    let mut matches = value
        .split_ascii_whitespace()
        .filter_map(|part| part.strip_prefix("pnpm/"));
    let version = matches.next().ok_or(ProvenanceError {
        code: "BETA_PNPM_PROVENANCE_INVALID",
    })?;
    if matches.next().is_some() {
        return Err(ProvenanceError {
            code: "BETA_PNPM_PROVENANCE_INVALID",
        });
    }
    validate_version(version).map_err(|_| ProvenanceError {
        code: "BETA_PNPM_PROVENANCE_INVALID",
    })
}

fn validate_version(value: &str) -> Result<String, ProvenanceError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        Ok(value.to_owned())
    } else {
        Err(ProvenanceError {
            code: "BETA_TOOL_VERSION_INVALID",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_parsers_reject_malformed_or_overbroad_identity() {
        assert!(parse_git_commit("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(parse_git_commit("0123456789ABCDEF0123456789ABCDEF01234567").is_err());
        assert_eq!(
            parse_tool_version("rustc", "rustc 1.93.1 (abc 2026-01-01)")
                .ok()
                .as_deref(),
            Some("1.93.1")
        );
        assert!(parse_tool_version("rustc", "cargo 1.93.1").is_err());
        assert!(parse_tool_version("rustc", "rustc latest").is_err());
        assert_eq!(
            parse_pnpm_user_agent("pnpm/11.7.0 npm/? node/v24.0.0")
                .ok()
                .as_deref(),
            Some("11.7.0")
        );
        assert!(parse_pnpm_user_agent("npm/11.7.0 node/v24.0.0").is_err());
        assert!(parse_pnpm_user_agent("pnpm/11.7.0 pnpm/11.7.1").is_err());
    }

    #[test]
    fn contract_allowlisted_input_hash_changes_with_file_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        for (_, relative_path) in INPUT_FILES {
            fs::write(root.path().join(relative_path), relative_path.as_bytes())?;
        }
        let before = hash_allowlisted_inputs(root.path())
            .map_err(|error| std::io::Error::other(error.code))?;
        fs::write(root.path().join("Cargo.lock"), b"changed")?;
        let after = hash_allowlisted_inputs(root.path())
            .map_err(|error| std::io::Error::other(error.code))?;
        assert_ne!(before.cargo_lock_sha256, after.cargo_lock_sha256);
        assert_eq!(before.pnpm_lock_sha256, after.pnpm_lock_sha256);
        Ok(())
    }

    #[test]
    fn contract_recorded_provenance_rejects_semantic_mutations() {
        let valid = BetaRunProvenanceV1 {
            schema_version: "hsai.local-beta-run-provenance.v1".to_owned(),
            source: BetaSourceProvenanceV1 {
                git_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
                tree_state: "CLEAN".to_owned(),
            },
            inputs: BetaInputProvenanceV1 {
                cargo_lock_sha256: Sha256Digest::from_bytes([1; 32]),
                pnpm_lock_sha256: Sha256Digest::from_bytes([2; 32]),
                rust_toolchain_sha256: Sha256Digest::from_bytes([3; 32]),
                package_json_sha256: Sha256Digest::from_bytes([4; 32]),
            },
            tools: BetaToolProvenanceV1 {
                rustc: "1.93.1".to_owned(),
                cargo: "1.93.1".to_owned(),
                pnpm: "11.7.0".to_owned(),
            },
        };
        assert!(validate_recorded_provenance(&valid).is_ok());

        let mut mutation = valid.clone();
        mutation.schema_version = "hsai.local-beta-run-provenance.v2".to_owned();
        assert_eq!(
            validate_recorded_provenance(&mutation)
                .err()
                .map(|error| error.code),
            Some("BETA_PROVENANCE_SCHEMA_INVALID")
        );
        mutation = valid.clone();
        mutation.source.tree_state = "DIRTY".to_owned();
        assert_eq!(
            validate_recorded_provenance(&mutation)
                .err()
                .map(|error| error.code),
            Some("BETA_SOURCE_STATE_INVALID")
        );
        mutation = valid.clone();
        mutation.source.git_commit = "ABC".to_owned();
        assert_eq!(
            validate_recorded_provenance(&mutation)
                .err()
                .map(|error| error.code),
            Some("BETA_GIT_COMMIT_INVALID")
        );
        for invalid in ["1.93", "v1.93.1", "1.93.1-beta", "1.93.1 host"] {
            mutation = valid.clone();
            mutation.tools.rustc = invalid.to_owned();
            assert_eq!(
                validate_recorded_provenance(&mutation)
                    .err()
                    .map(|error| error.code),
                Some("BETA_TOOL_VERSION_INVALID")
            );
        }
    }
}
