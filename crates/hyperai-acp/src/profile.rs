//! Explicit local ACP profile validation.

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_PROFILE_NAME_BYTES: usize = 80;
const MAX_PROFILE_ARGS: usize = 64;
const MAX_ARG_BYTES: usize = 4_096;
const MAX_ENVIRONMENT_ENTRIES: usize = 8;
const MAX_ENVIRONMENT_VALUE_BYTES: usize = 512;

/// User-selected executable and working directory for one local ACP session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProfileV1 {
    /// Human-readable profile name shown before connection.
    pub name: String,
    /// Absolute executable path. No shell parsing is performed.
    pub executable: PathBuf,
    /// Arguments passed as separate values to the executable.
    #[serde(default)]
    pub args: Vec<String>,
    /// Absolute working directory sent to the agent session.
    pub working_directory: PathBuf,
    /// Explicit non-secret environment allowlist.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

/// Non-secret identity shown to the user for the active volatile session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProfileSummaryV1 {
    /// Human-readable profile name.
    pub name: String,
    /// Exact executable path selected by the user.
    pub executable: PathBuf,
    /// Working directory supplied to the process.
    pub working_directory: PathBuf,
}

impl From<&AgentProfileV1> for AgentProfileSummaryV1 {
    fn from(profile: &AgentProfileV1) -> Self {
        Self {
            name: profile.name.clone(),
            executable: profile.executable.clone(),
            working_directory: profile.working_directory.clone(),
        }
    }
}

/// An explicitly approved non-secret environment variable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileEnvironmentV1 {
    /// Environment variable name.
    pub name: String,
    /// Non-secret value.
    pub value: String,
}

/// A profile failed validation before any process was started.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ProfileError {
    /// A required name or path is missing or invalid.
    #[error("profile name and absolute executable and working directory are required")]
    InvalidIdentity,
    /// A bounded field exceeds its configured limit.
    #[error("profile exceeds a field size or count limit")]
    LimitExceeded,
    /// The profile contains an environment name outside the safe allowlist.
    #[error("profile environment is not in the non-secret allowlist")]
    EnvironmentDenied,
}

impl AgentProfileV1 {
    /// Check the profile without launching its executable.
    ///
    /// # Errors
    ///
    /// Returns `ProfileError` when a path, field limit, or environment name is
    /// outside the local profile contract.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.name.trim().is_empty()
            || self.name.len() > MAX_PROFILE_NAME_BYTES
            || !self.executable.is_absolute()
            || !self.working_directory.is_absolute()
        {
            return Err(ProfileError::InvalidIdentity);
        }
        if self.args.len() > MAX_PROFILE_ARGS
            || self.args.iter().any(|arg| arg.len() > MAX_ARG_BYTES)
            || self.environment.len() > MAX_ENVIRONMENT_ENTRIES
        {
            return Err(ProfileError::LimitExceeded);
        }
        for (name, value) in &self.environment {
            if value.len() > MAX_ENVIRONMENT_VALUE_BYTES {
                return Err(ProfileError::LimitExceeded);
            }
            if !matches!(name.as_str(), "LANG" | "LC_ALL" | "NO_COLOR" | "TERM") {
                return Err(ProfileError::EnvironmentDenied);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> AgentProfileV1 {
        AgentProfileV1 {
            name: "Local fixture".to_owned(),
            executable: PathBuf::from("/usr/bin/fixture"),
            args: vec!["--synthetic".to_owned()],
            working_directory: PathBuf::from("/tmp"),
            environment: BTreeMap::new(),
        }
    }

    #[test]
    fn permits_absolute_profile_without_extra_environment() {
        assert_eq!(profile().validate(), Ok(()));
    }

    #[test]
    fn rejects_shell_secret_and_path_environment_names() {
        let mut candidate = profile();
        candidate
            .environment
            .insert("OPENAI_API_KEY".to_owned(), "secret".to_owned());
        assert_eq!(candidate.validate(), Err(ProfileError::EnvironmentDenied));
    }

    #[test]
    fn rejects_relative_executables() {
        let mut candidate = profile();
        candidate.executable = PathBuf::from("agent --flag");
        assert_eq!(candidate.validate(), Err(ProfileError::InvalidIdentity));
    }
}
