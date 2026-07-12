use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{CanonicalizationError, Sha256Digest, UtcTimestamp, domain_separated_digest};

/// Bounded operator action whose authorization was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorizationActionV1 {
    /// Read detailed dependency health.
    DependencyHealthRead,
    /// Execute one fixed local scenario.
    LocalScenarioRun,
    /// Inspect settlement readiness.
    SettlementPreflight,
    /// Execute the bounded mock workflow.
    MockWorkflowRun,
    /// Attempt a structurally forbidden mainnet mutation.
    MainnetMutationAttempt,
}

/// Sanitized class of the unauthenticated actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorizationActorClassV1 {
    /// No credential-like value was presented.
    Anonymous,
    /// A credential-like value was presented but not authenticated.
    UnauthenticatedCredentialPresenter,
}

/// Bounded reason for refusing an operator action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorizationDenialReasonV1 {
    /// The required role was absent.
    MissingRole,
    /// The presented role could not be parsed.
    MalformedRole,
    /// The parsed role did not authorize the action.
    RoleMismatch,
}

/// Sanitized append-only authorization-denial record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationDenialRecordV1 {
    /// Wire schema identifier.
    pub schema_version: String,
    /// Deterministic request-and-action identity.
    pub denial_id: String,
    /// Bounded correlation identifier.
    pub request_id: String,
    /// Sanitized stable actor identifier.
    pub actor_id: String,
    /// Sanitized actor classification.
    pub actor_class: AuthorizationActorClassV1,
    /// Denied bounded action.
    pub action: AuthorizationActionV1,
    /// Bounded denial reason.
    pub reason: AuthorizationDenialReasonV1,
    /// Canonical denial time.
    pub denied_at: UtcTimestamp,
    /// Digest of the complete semantic record.
    pub record_digest: Sha256Digest,
}

/// Invalid or altered authorization-denial identity.
#[derive(Debug, Error)]
pub enum AuthorizationDenialError {
    /// A bounded identifier was empty, too long, or contained forbidden bytes.
    #[error("authorization denial identifier is invalid")]
    InvalidIdentifier,
    /// A derived identifier or digest was altered.
    #[error("authorization denial identity does not match its payload")]
    IdentityMismatch,
    /// Canonical encoding failed.
    #[error("authorization denial is not canonicalizable: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

#[derive(Serialize)]
struct DenialIdentity<'a> {
    schema_version: &'static str,
    request_id: &'a str,
    actor_id: &'a str,
    actor_class: AuthorizationActorClassV1,
    action: AuthorizationActionV1,
    reason: AuthorizationDenialReasonV1,
    denied_at: &'a UtcTimestamp,
}

impl AuthorizationDenialRecordV1 {
    /// Construct a bounded record without accepting raw credential or request material.
    ///
    /// # Errors
    ///
    /// Returns an identifier or canonicalization error.
    pub fn new(
        request_id: String,
        actor_id: String,
        actor_class: AuthorizationActorClassV1,
        action: AuthorizationActionV1,
        reason: AuthorizationDenialReasonV1,
        denied_at: UtcTimestamp,
    ) -> Result<Self, AuthorizationDenialError> {
        validate_identifier(&request_id)?;
        validate_identifier(&actor_id)?;
        if actor_id != "public-reader" {
            return Err(AuthorizationDenialError::InvalidIdentifier);
        }
        let record_digest = denial_digest(
            &request_id,
            &actor_id,
            actor_class,
            action,
            reason,
            &denied_at,
        )?;
        let identity_digest =
            domain_separated_digest("hsai.authorization-denial-id.v1", &(&request_id, action))?;
        Ok(Self {
            schema_version: "hsai.authorization-denial-record.v1".to_owned(),
            denial_id: format!("denial_{}", &identity_digest.to_hex()[..32]),
            request_id,
            actor_id,
            actor_class,
            action,
            reason,
            denied_at,
            record_digest,
        })
    }

    /// Verify bounded fields and every derived identity.
    ///
    /// # Errors
    ///
    /// Returns an identifier, canonicalization, or identity mismatch error.
    pub fn verify_identity(&self) -> Result<(), AuthorizationDenialError> {
        validate_identifier(&self.request_id)?;
        validate_identifier(&self.actor_id)?;
        if self.actor_id != "public-reader" {
            return Err(AuthorizationDenialError::InvalidIdentifier);
        }
        let record_digest = denial_digest(
            &self.request_id,
            &self.actor_id,
            self.actor_class,
            self.action,
            self.reason,
            &self.denied_at,
        )?;
        let identity_digest = domain_separated_digest(
            "hsai.authorization-denial-id.v1",
            &(&self.request_id, self.action),
        )?;
        if self.schema_version == "hsai.authorization-denial-record.v1"
            && self.denial_id == format!("denial_{}", &identity_digest.to_hex()[..32])
            && self.record_digest == record_digest
        {
            Ok(())
        } else {
            Err(AuthorizationDenialError::IdentityMismatch)
        }
    }
}

fn denial_digest(
    request_id: &str,
    actor_id: &str,
    actor_class: AuthorizationActorClassV1,
    action: AuthorizationActionV1,
    reason: AuthorizationDenialReasonV1,
    denied_at: &UtcTimestamp,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hsai.authorization-denial-record.v1",
        &DenialIdentity {
            schema_version: "hsai.authorization-denial-record.v1",
            request_id,
            actor_id,
            actor_class,
            action,
            reason,
            denied_at,
        },
    )
}

fn validate_identifier(value: &str) -> Result<(), AuthorizationDenialError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
    {
        Err(AuthorizationDenialError::InvalidIdentifier)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_deterministic_and_rejects_changes() -> Result<(), Box<dyn std::error::Error>> {
        let record = AuthorizationDenialRecordV1::new(
            "request-1".to_owned(),
            "public-reader".to_owned(),
            AuthorizationActorClassV1::Anonymous,
            AuthorizationActionV1::DependencyHealthRead,
            AuthorizationDenialReasonV1::MissingRole,
            "2026-07-12T12:00:00Z".parse()?,
        )?;
        record.verify_identity()?;
        let mut altered = record;
        altered.reason = AuthorizationDenialReasonV1::RoleMismatch;
        assert!(matches!(
            altered.verify_identity(),
            Err(AuthorizationDenialError::IdentityMismatch)
        ));
        Ok(())
    }

    #[test]
    fn identifiers_are_bounded_and_sanitized() -> Result<(), Box<dyn std::error::Error>> {
        let error = AuthorizationDenialRecordV1::new(
            "raw header=value".to_owned(),
            "public-reader".to_owned(),
            AuthorizationActorClassV1::Anonymous,
            AuthorizationActionV1::LocalScenarioRun,
            AuthorizationDenialReasonV1::MalformedRole,
            "2026-07-12T12:00:00Z".parse()?,
        );
        assert!(matches!(
            error,
            Err(AuthorizationDenialError::InvalidIdentifier)
        ));
        Ok(())
    }
}
