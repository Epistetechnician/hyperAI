//! Process-bound bearer and short-lived browser-session capabilities.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use subtle::ConstantTimeEq;
use thiserror::Error;
use zeroize::Zeroizing;

const BROWSER_SESSION_TTL_SECONDS: u64 = 1_800;

/// Authentication capability state for one running service instance.
pub struct AuthState {
    bearer: Zeroizing<String>,
    browser: Mutex<BrowserSession>,
    agent_tools: Mutex<Option<Zeroizing<String>>>,
}

struct BrowserSession {
    session: Zeroizing<String>,
    csrf: Zeroizing<String>,
    expires_at_unix_seconds: u64,
}

/// Service-session cookie header value.
pub struct SessionCookie(Zeroizing<String>);

/// Errors while creating or rotating a service capability.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Random-number generation failed.
    #[error("secure random capability generation failed: {0}")]
    Random(#[from] getrandom::Error),
    /// The capability file operation failed.
    #[error("service capability file operation failed: {0}")]
    Io(#[from] io::Error),
    /// Browser session state was poisoned after a panic.
    #[error("browser session state is unavailable")]
    Poisoned,
}

impl AuthState {
    /// Generates a process-bound bearer and initial browser session state.
    ///
    /// # Errors
    ///
    /// Returns an error if the operating system cannot provide secure random
    /// bytes.
    pub fn generate() -> Result<Self, AuthError> {
        Ok(Self {
            bearer: Zeroizing::new(generate_secret()?),
            browser: Mutex::new(BrowserSession {
                session: Zeroizing::new(generate_secret()?),
                csrf: Zeroizing::new(generate_secret()?),
                expires_at_unix_seconds: now_unix_seconds() + BROWSER_SESSION_TTL_SECONDS,
            }),
            agent_tools: Mutex::new(None),
        })
    }

    /// Returns the service bearer for the current process.
    pub fn bearer(&self) -> &str {
        &self.bearer
    }

    /// Tests a supplied bearer without a variable-time byte comparison.
    pub fn bearer_matches(&self, candidate: &str) -> bool {
        secret_matches(&self.bearer, candidate)
    }

    /// Create or replace the volatile read-only agent-tool capability.
    ///
    /// This capability is accepted only on the local agent-tool route prefix;
    /// it is distinct from the owner's full service bearer.
    ///
    /// # Errors
    ///
    /// Returns an error if secure token generation fails or the capability
    /// store is poisoned.
    pub fn issue_agent_tool_capability(&self) -> Result<String, AuthError> {
        let capability = Zeroizing::new(generate_secret()?);
        let mut stored = self.agent_tools.lock().map_err(|_| AuthError::Poisoned)?;
        let exposed = capability.to_string();
        *stored = Some(capability);
        Ok(exposed)
    }

    /// Compare a scoped tool capability without a variable-time byte comparison.
    pub fn agent_tool_capability_matches(&self, candidate: &str) -> bool {
        self.agent_tools.lock().is_ok_and(|stored| {
            stored
                .as_ref()
                .is_some_and(|expected| secret_matches(expected, candidate))
        })
    }

    /// Revoke the active read-only agent-tool capability.
    ///
    /// # Errors
    ///
    /// Returns an error if the capability store is poisoned.
    pub fn revoke_agent_tool_capability(&self) -> Result<(), AuthError> {
        let mut stored = self.agent_tools.lock().map_err(|_| AuthError::Poisoned)?;
        *stored = None;
        Ok(())
    }

    /// Tests a supplied browser cookie without a variable-time comparison.
    pub fn session_matches(&self, candidate: &str) -> bool {
        self.browser.lock().is_ok_and(|browser| {
            now_unix_seconds() <= browser.expires_at_unix_seconds
                && secret_matches(&browser.session, candidate)
        })
    }

    /// Tests a supplied CSRF header without a variable-time comparison.
    pub fn csrf_matches(&self, candidate: &str) -> bool {
        self.browser.lock().is_ok_and(|browser| {
            now_unix_seconds() <= browser.expires_at_unix_seconds
                && secret_matches(&browser.csrf, candidate)
        })
    }

    /// Creates the owner-only bearer file used by CLI and TUI clients.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is unsafe, permissions cannot be
    /// restricted, or the capability cannot be written durably.
    pub fn write_bearer_file(&self, path: &Path) -> Result<(), AuthError> {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "service capability path is not a regular file",
                )
                .into());
            }
            fs::remove_file(path)?;
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(self.bearer.as_bytes())?;
        file.sync_all()?;
        restrict_capability_file(path)?;
        Ok(())
    }

    /// Reads an existing local bearer capability.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is missing, unsafe, accessible to other
    /// Unix users, or contains an invalid capability.
    pub fn read_bearer_file(path: &Path) -> Result<String, AuthError> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "service capability path is not a regular file",
            )
            .into());
        }
        restrict_capability_file(path)?;
        let token = fs::read_to_string(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "service capability file is accessible to group or other users",
                )
                .into());
            }
        }
        if token.len() < 40
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "service capability file has invalid contents",
            )
            .into());
        }
        Ok(token)
    }

    /// Removes the current bearer file after graceful service shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is unsafe or cannot be removed.
    pub fn remove_bearer_file(path: &Path) -> Result<(), AuthError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "service capability path is not a regular file",
                )
                .into())
            }
            Ok(_) => {
                fs::remove_file(path)?;
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Rotates the browser session and returns its cookie and in-memory CSRF value.
    ///
    /// # Errors
    ///
    /// Returns an error if secure random generation fails or session state has
    /// been poisoned.
    pub fn refresh_browser_session(&self) -> Result<(SessionCookie, Zeroizing<String>), AuthError> {
        let session = Zeroizing::new(generate_secret()?);
        let csrf = Zeroizing::new(generate_secret()?);
        let mut browser = self.browser.lock().map_err(|_| AuthError::Poisoned)?;
        *browser = BrowserSession {
            session: Zeroizing::new(session.as_str().to_owned()),
            csrf: Zeroizing::new(csrf.as_str().to_owned()),
            expires_at_unix_seconds: now_unix_seconds() + BROWSER_SESSION_TTL_SECONDS,
        };
        let cookie = SessionCookie(Zeroizing::new(format!(
            "hyperai_session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={BROWSER_SESSION_TTL_SECONDS}",
            browser.session.as_str()
        )));
        Ok((cookie, csrf))
    }
}

fn restrict_capability_file(path: &Path) -> Result<(), AuthError> {
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

impl SessionCookie {
    /// Returns the cookie header contents.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn generate_secret() -> Result<String, AuthError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn secret_matches(expected: &str, candidate: &str) -> bool {
    expected.len() == candidate.len() && bool::from(expected.as_bytes().ct_eq(candidate.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tool_capability_is_separate_and_revocable() -> Result<(), AuthError> {
        let auth = AuthState::generate()?;
        let capability = auth.issue_agent_tool_capability()?;
        assert_ne!(auth.bearer(), capability);
        assert!(auth.agent_tool_capability_matches(&capability));
        assert!(!auth.bearer_matches(&capability));
        auth.revoke_agent_tool_capability()?;
        assert!(!auth.agent_tool_capability_matches(&capability));
        Ok(())
    }
}
