//! Submission observation, cursors, provenance, and completeness ports.

use std::collections::BTreeMap;

use outcomes_domain::{
    AdapterHealthStatus, AdapterHealthV1, CandidateEnvelopeV1, ClaimId, CompletenessStatus,
    ObservationSnapshotV1, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Immutable candidate content returned by a source adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateArtifact {
    /// Candidate metadata and content commitment.
    pub envelope: CandidateEnvelopeV1,
    /// Untrusted candidate bytes subject to the frozen size and type policy.
    pub bytes: Vec<u8>,
}

/// Result of a source provenance check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceResult {
    /// Whether the recorded source identity and content commitment agree.
    pub valid: bool,
    /// Exact reason for the result.
    pub reason: String,
}

/// Submission-source adapter failures.
#[derive(Debug, Error)]
pub enum SourceError {
    /// Cursor was malformed or beyond the ledger.
    #[error("invalid observation cursor {0}")]
    InvalidCursor(String),
    /// Requested candidate does not exist.
    #[error("unknown candidate {0}")]
    UnknownCandidate(String),
    /// Source identity was reused with changed content under immutable semantics.
    #[error("source identity conflict for {0}")]
    SourceIdentityConflict(String),
    /// Candidate was submitted to a ledger owned by another frozen claim.
    #[error("candidate claim {actual} does not match ledger claim {expected}")]
    ClaimMismatch {
        /// Claim bound to the ledger.
        expected: ClaimId,
        /// Claim supplied with the candidate.
        actual: ClaimId,
    },
    /// Payload could not be canonicalized.
    #[error("source payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
}

/// Versioned port for completeness-aware candidate observation.
// Each fallible method returns `SourceError`; implementations document the
// exact cursor, provenance, and completeness failures.
#[allow(clippy::missing_errors_doc)]
pub trait SubmissionSourceAdapter {
    /// Produce a replayable snapshot at a cutoff.
    fn snapshot(
        &self,
        cursor: Option<&str>,
        cutoff: &UtcTimestamp,
        received_at: &UtcTimestamp,
    ) -> Result<ObservationSnapshotV1, SourceError>;
    /// List candidates committed by the snapshot.
    fn list_candidates(
        &self,
        snapshot: &ObservationSnapshotV1,
    ) -> Result<Vec<CandidateEnvelopeV1>, SourceError>;
    /// Fetch one immutable candidate artifact.
    fn fetch_candidate(&self, candidate_id: &str) -> Result<&CandidateArtifact, SourceError>;
    /// Check source identity and transport/content commitments.
    fn verify_source_provenance(
        &self,
        candidate: &CandidateEnvelopeV1,
    ) -> Result<ProvenanceResult, SourceError>;
    /// Assess whether the source snapshot is complete through its cutoff.
    fn assess_completeness(
        &self,
        snapshot: &ObservationSnapshotV1,
    ) -> Result<CompletenessStatus, SourceError>;
    /// Return adapter health without mutating the ledger.
    fn health(&self) -> &AdapterHealthV1;
}

/// HSAI-owned append-only local submission ledger.
#[derive(Debug, Clone)]
pub struct LocalSubmissionLedger {
    claim_id: ClaimId,
    adapter_version: String,
    entries: Vec<CandidateArtifact>,
    identities: BTreeMap<(String, String), Sha256Digest>,
    finalized_through: Option<UtcTimestamp>,
    pagination_complete: bool,
    health: AdapterHealthV1,
}

impl LocalSubmissionLedger {
    /// Construct an empty hermetic source ledger.
    #[must_use]
    pub fn new(claim_id: ClaimId, now: UtcTimestamp) -> Self {
        Self {
            claim_id,
            adapter_version: "1.0.0".to_owned(),
            entries: Vec::new(),
            identities: BTreeMap::new(),
            finalized_through: None,
            pagination_complete: true,
            health: AdapterHealthV1 {
                adapter_id: "local-submission-ledger".to_owned(),
                status: AdapterHealthStatus::Healthy,
                checked_at: now,
                exact_reason: "append-only local ledger available".to_owned(),
            },
        }
    }

    /// Append an immutable candidate or return the existing exact duplicate.
    ///
    /// # Errors
    ///
    /// Returns a source-identity conflict when the same source reference and
    /// revision are reused with different bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn ingest(
        &mut self,
        claim_id: ClaimId,
        source_namespace: &str,
        source_candidate_ref: &str,
        source_revision: &str,
        author_or_team_id: Option<String>,
        observed_source_time: UtcTimestamp,
        received_at: UtcTimestamp,
        media_type: &str,
        bytes: Vec<u8>,
    ) -> Result<CandidateEnvelopeV1, SourceError> {
        if claim_id != self.claim_id {
            return Err(SourceError::ClaimMismatch {
                expected: self.claim_id.clone(),
                actual: claim_id,
            });
        }
        let artifact_digest = plain_digest(&bytes);
        let identity = (source_candidate_ref.to_owned(), source_revision.to_owned());
        if let Some(existing) = self.identities.get(&identity) {
            if existing == &artifact_digest {
                let candidate = self
                    .entries
                    .iter()
                    .find(|candidate| {
                        candidate.envelope.source_candidate_ref == source_candidate_ref
                            && candidate.envelope.source_revision == source_revision
                    })
                    .ok_or_else(|| {
                        SourceError::SourceIdentityConflict(source_candidate_ref.to_owned())
                    })?;
                return Ok(candidate.envelope.clone());
            }
            return Err(SourceError::SourceIdentityConflict(
                source_candidate_ref.to_owned(),
            ));
        }
        let provenance_digest = domain_separated_digest(
            "hsai.source-provenance.v1",
            &(
                source_namespace,
                source_candidate_ref,
                source_revision,
                &observed_source_time,
                artifact_digest,
            ),
        )?;
        let candidate_identity = domain_separated_digest(
            "hsai.candidate-identity.v1",
            &(
                source_namespace,
                source_candidate_ref,
                source_revision,
                artifact_digest,
            ),
        )?;
        let envelope = CandidateEnvelopeV1 {
            schema_version: "hsai.candidate-envelope.v1".to_owned(),
            candidate_id: format!("candidate_{}", &candidate_identity.to_hex()[..32]),
            claim_id,
            source_namespace: source_namespace.to_owned(),
            source_candidate_ref: source_candidate_ref.to_owned(),
            source_revision: source_revision.to_owned(),
            author_or_team_id,
            observed_source_time,
            received_at,
            artifact_digest,
            artifact_byte_length: bytes.len() as u64,
            artifact_media_type: media_type.to_owned(),
            source_provenance_digest: provenance_digest,
        };
        self.identities.insert(identity, artifact_digest);
        self.entries.push(CandidateArtifact {
            envelope: envelope.clone(),
            bytes,
        });
        Ok(envelope)
    }

    /// Mark the local ledger final through an authoritative cutoff.
    pub fn finalize_through(&mut self, cutoff: UtcTimestamp) {
        self.finalized_through = Some(cutoff);
    }

    /// Inject pagination completeness for source-gap tests.
    pub fn set_pagination_complete(&mut self, complete: bool) {
        self.pagination_complete = complete;
    }
}

impl SubmissionSourceAdapter for LocalSubmissionLedger {
    fn snapshot(
        &self,
        cursor: Option<&str>,
        cutoff: &UtcTimestamp,
        received_at: &UtcTimestamp,
    ) -> Result<ObservationSnapshotV1, SourceError> {
        let start = parse_cursor(cursor, self.entries.len())?;
        let end = self
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take_while(|(_, candidate)| candidate.envelope.observed_source_time <= *cutoff)
            .map(|(index, _)| index + 1)
            .last()
            .unwrap_or(start);
        let committed: Vec<_> = self.entries[start..end]
            .iter()
            .map(|candidate| candidate.envelope.artifact_digest)
            .collect();
        let raw_digest = domain_separated_digest(
            "hsai.local-observation-response.v1",
            &(start, end, cutoff, &committed),
        )?;
        let complete = self.pagination_complete
            && self
                .finalized_through
                .as_ref()
                .is_some_and(|finalized| finalized >= cutoff);
        let completeness = if complete {
            CompletenessStatus::Complete
        } else {
            CompletenessStatus::Incomplete
        };
        let snapshot_digest = domain_separated_digest(
            "hsai.observation-snapshot.v1",
            &(start, end, cutoff, raw_digest, completeness),
        )?;
        Ok(ObservationSnapshotV1 {
            schema_version: "hsai.observation-snapshot.v1".to_owned(),
            snapshot_id: format!("observation_{}", &snapshot_digest.to_hex()[..32]),
            claim_id: self.claim_id.clone(),
            source_adapter_id: "local-submission-ledger".to_owned(),
            source_adapter_version: self.adapter_version.clone(),
            cursor_start: cursor.map(str::to_owned),
            cursor_end: end.to_string(),
            cutoff: cutoff.clone(),
            request_parameters_digest: domain_separated_digest(
                "hsai.observation-request.v1",
                &(cursor, cutoff),
            )?,
            raw_response_digests: vec![raw_digest],
            received_at: received_at.clone(),
            latest_source_time: self.entries[..end]
                .last()
                .map(|candidate| candidate.envelope.observed_source_time.clone()),
            pagination_complete: self.pagination_complete,
            retry_count: 0,
            completeness,
            completeness_reason: if complete {
                "local ledger finalized through cutoff".to_owned()
            } else {
                "ledger not finalized through cutoff or pagination gap injected".to_owned()
            },
        })
    }

    fn list_candidates(
        &self,
        snapshot: &ObservationSnapshotV1,
    ) -> Result<Vec<CandidateEnvelopeV1>, SourceError> {
        let start = parse_cursor(snapshot.cursor_start.as_deref(), self.entries.len())?;
        let end = parse_cursor(Some(&snapshot.cursor_end), self.entries.len())?;
        if end < start {
            return Err(SourceError::InvalidCursor(snapshot.cursor_end.clone()));
        }
        Ok(self.entries[start..end]
            .iter()
            .map(|candidate| candidate.envelope.clone())
            .collect())
    }

    fn fetch_candidate(&self, candidate_id: &str) -> Result<&CandidateArtifact, SourceError> {
        self.entries
            .iter()
            .find(|candidate| candidate.envelope.candidate_id == candidate_id)
            .ok_or_else(|| SourceError::UnknownCandidate(candidate_id.to_owned()))
    }

    fn verify_source_provenance(
        &self,
        candidate: &CandidateEnvelopeV1,
    ) -> Result<ProvenanceResult, SourceError> {
        let stored = self.fetch_candidate(&candidate.candidate_id)?;
        let valid = stored.envelope == *candidate
            && plain_digest(&stored.bytes) == candidate.artifact_digest;
        Ok(ProvenanceResult {
            valid,
            reason: if valid {
                "local append-only identity and content digest agree".to_owned()
            } else {
                "candidate metadata or bytes differ from the ledger".to_owned()
            },
        })
    }

    fn assess_completeness(
        &self,
        snapshot: &ObservationSnapshotV1,
    ) -> Result<CompletenessStatus, SourceError> {
        let _ = parse_cursor(Some(&snapshot.cursor_end), self.entries.len())?;
        Ok(snapshot.completeness)
    }

    fn health(&self) -> &AdapterHealthV1 {
        &self.health
    }
}

fn parse_cursor(cursor: Option<&str>, maximum: usize) -> Result<usize, SourceError> {
    let value = match cursor {
        Some(cursor) => cursor
            .parse::<usize>()
            .map_err(|_| SourceError::InvalidCursor(cursor.to_owned()))?,
        None => 0,
    };
    if value <= maximum {
        Ok(value)
    } else {
        Err(SourceError::InvalidCursor(value.to_string()))
    }
}

fn plain_digest(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn claim_id() -> ClaimId {
        ClaimId::from_digest(Sha256Digest::from_bytes([9; 32]))
    }

    #[test]
    fn contract_cursor_dedup_and_complete_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let mut ledger = LocalSubmissionLedger::new(claim_id(), timestamp("2026-07-10T10:00:00Z")?);
        let candidate = ledger.ingest(
            claim_id(),
            "local.demo",
            "submission-1",
            "1",
            None,
            timestamp("2026-07-10T10:30:00Z")?,
            timestamp("2026-07-10T10:30:01Z")?,
            "application/octet-stream",
            b"hsai-pass-v1".to_vec(),
        )?;
        let duplicate = ledger.ingest(
            claim_id(),
            "local.demo",
            "submission-1",
            "1",
            None,
            timestamp("2026-07-10T10:30:00Z")?,
            timestamp("2026-07-10T10:30:01Z")?,
            "application/octet-stream",
            b"hsai-pass-v1".to_vec(),
        )?;
        assert_eq!(candidate, duplicate);
        ledger.finalize_through(timestamp("2026-07-10T12:05:00Z")?);
        let snapshot = ledger.snapshot(
            None,
            &timestamp("2026-07-10T12:00:00Z")?,
            &timestamp("2026-07-10T12:05:01Z")?,
        )?;
        assert_eq!(snapshot.completeness, CompletenessStatus::Complete);
        assert_eq!(ledger.list_candidates(&snapshot)?.len(), 1);
        assert!(ledger.verify_source_provenance(&candidate)?.valid);
        Ok(())
    }

    #[test]
    fn contract_identity_reuse_with_changed_bytes_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut ledger = LocalSubmissionLedger::new(claim_id(), timestamp("2026-07-10T10:00:00Z")?);
        ledger.ingest(
            claim_id(),
            "local.demo",
            "submission-1",
            "1",
            None,
            timestamp("2026-07-10T10:30:00Z")?,
            timestamp("2026-07-10T10:30:01Z")?,
            "application/octet-stream",
            b"first".to_vec(),
        )?;
        assert!(matches!(
            ledger.ingest(
                claim_id(),
                "local.demo",
                "submission-1",
                "1",
                None,
                timestamp("2026-07-10T10:30:00Z")?,
                timestamp("2026-07-10T10:30:01Z")?,
                "application/octet-stream",
                b"changed".to_vec(),
            ),
            Err(SourceError::SourceIdentityConflict(_))
        ));
        Ok(())
    }

    #[test]
    fn contract_incomplete_snapshot_cannot_claim_no_solution()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut ledger = LocalSubmissionLedger::new(claim_id(), timestamp("2026-07-10T10:00:00Z")?);
        ledger.set_pagination_complete(false);
        let snapshot = ledger.snapshot(
            None,
            &timestamp("2026-07-10T12:00:00Z")?,
            &timestamp("2026-07-10T12:05:01Z")?,
        )?;
        assert_eq!(snapshot.completeness, CompletenessStatus::Incomplete);
        Ok(())
    }
}
