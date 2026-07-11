//! Durable job, lease, outbox, and recovery composition.

use outcomes_domain::{
    ClaimTruth, ExecutionOutcome, ResolutionFinality, SettlementLifecycle, Sha256Digest,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Durable worker job status persisted in the `durable_jobs` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    /// Available for a worker lease.
    Queued,
    /// Leased to one worker with heartbeat expiry.
    Running,
    /// Waiting for classified bounded retry time.
    RetryWait,
    /// Handler effect and transaction completed.
    Completed,
    /// Terminal quarantine requiring operator visibility.
    DeadLetter,
}

/// Fixed-cardinality operational snapshot safe for metrics export.
///
/// It intentionally contains no claim IDs, candidate IDs, wallet addresses,
/// signer references, filesystem paths, or raw error payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalSnapshot {
    /// Snapshot generation time.
    pub generated_at: UtcTimestamp,
    /// Claim truth counters.
    pub claims: ClaimCounters,
    /// Resolution finality counters.
    pub finality: FinalityCounters,
    /// Normalized verifier outcome counters.
    pub verification: VerificationCounters,
    /// Durable job-state counters.
    pub jobs: JobCounters,
    /// Settlement lifecycle counters.
    pub settlement: SettlementCounters,
    /// Current transactional-outbox backlog.
    pub outbox_backlog: u64,
    /// Denied operator/API requests.
    pub unauthorized_attempts: u64,
}

/// Fixed claim-truth dimensions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimCounters {
    /// Final or provisional TRUE observations.
    pub true_count: u64,
    /// Final or provisional FALSE observations.
    pub false_count: u64,
    /// Claims whose evidence cannot currently decide truth.
    pub unresolved_count: u64,
}

/// Fixed finality dimensions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityCounters {
    /// No proposal.
    pub none_count: u64,
    /// Provisional records.
    pub provisional_count: u64,
    /// Actively disputed records.
    pub disputed_count: u64,
    /// Final records.
    pub final_count: u64,
}

/// Fixed verifier dimensions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCounters {
    /// Accepted executions.
    pub pass_count: u64,
    /// Correct deterministic rejections.
    pub fail_count: u64,
    /// Semantically undecidable executions.
    pub inconclusive_count: u64,
    /// Infrastructure/runner/adapter failures.
    pub error_count: u64,
}

/// Fixed durable-job dimensions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCounters {
    /// Queued jobs.
    pub queued_count: u64,
    /// Leased jobs.
    pub running_count: u64,
    /// Jobs awaiting bounded retry.
    pub retry_wait_count: u64,
    /// Completed jobs.
    pub completed_count: u64,
    /// Quarantined terminal jobs.
    pub dead_letter_count: u64,
}

/// Fixed settlement lifecycle dimensions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementCounters {
    /// Unbound claims.
    pub unbound_count: u64,
    /// Bound claims.
    pub bound_count: u64,
    /// Blocked preflight states.
    pub not_ready_count: u64,
    /// Ready states.
    pub ready_count: u64,
    /// Persisted unsigned intents.
    pub intent_built_count: u64,
    /// Submitted effects.
    pub submitted_count: u64,
    /// Confirmed effects.
    pub confirmed_count: u64,
    /// Failed/ambiguous reconciliation.
    pub reconciliation_failed_count: u64,
    /// Operator-paused states.
    pub paused_count: u64,
}

impl OperationalSnapshot {
    /// Construct an empty snapshot at an injected canonical time.
    #[must_use]
    pub fn new(generated_at: UtcTimestamp) -> Self {
        Self {
            generated_at,
            claims: ClaimCounters::default(),
            finality: FinalityCounters::default(),
            verification: VerificationCounters::default(),
            jobs: JobCounters::default(),
            settlement: SettlementCounters::default(),
            outbox_backlog: 0,
            unauthorized_attempts: 0,
        }
    }

    /// Record one claim projection using fixed enum dimensions.
    pub fn observe_claim(&mut self, truth: ClaimTruth, finality: ResolutionFinality) {
        match truth {
            ClaimTruth::True => increment(&mut self.claims.true_count),
            ClaimTruth::False => increment(&mut self.claims.false_count),
            ClaimTruth::Unresolved => increment(&mut self.claims.unresolved_count),
        }
        match finality {
            ResolutionFinality::None => increment(&mut self.finality.none_count),
            ResolutionFinality::Provisional => increment(&mut self.finality.provisional_count),
            ResolutionFinality::Disputed => increment(&mut self.finality.disputed_count),
            ResolutionFinality::Final => increment(&mut self.finality.final_count),
        }
    }

    /// Record one normalized verifier outcome.
    pub fn observe_verification(&mut self, outcome: ExecutionOutcome) {
        match outcome {
            ExecutionOutcome::Pass => increment(&mut self.verification.pass_count),
            ExecutionOutcome::Fail => increment(&mut self.verification.fail_count),
            ExecutionOutcome::Inconclusive => {
                increment(&mut self.verification.inconclusive_count);
            }
            ExecutionOutcome::Error => increment(&mut self.verification.error_count),
        }
    }

    /// Record one durable job projection.
    pub fn observe_job(&mut self, status: JobStatus) {
        match status {
            JobStatus::Queued => increment(&mut self.jobs.queued_count),
            JobStatus::Running => increment(&mut self.jobs.running_count),
            JobStatus::RetryWait => increment(&mut self.jobs.retry_wait_count),
            JobStatus::Completed => increment(&mut self.jobs.completed_count),
            JobStatus::DeadLetter => increment(&mut self.jobs.dead_letter_count),
        }
    }

    /// Record one settlement lifecycle projection.
    pub fn observe_settlement(&mut self, lifecycle: SettlementLifecycle) {
        match lifecycle {
            SettlementLifecycle::Unbound => increment(&mut self.settlement.unbound_count),
            SettlementLifecycle::Bound => increment(&mut self.settlement.bound_count),
            SettlementLifecycle::NotReady => increment(&mut self.settlement.not_ready_count),
            SettlementLifecycle::Ready => increment(&mut self.settlement.ready_count),
            SettlementLifecycle::IntentBuilt => increment(&mut self.settlement.intent_built_count),
            SettlementLifecycle::Submitted => increment(&mut self.settlement.submitted_count),
            SettlementLifecycle::Confirmed => increment(&mut self.settlement.confirmed_count),
            SettlementLifecycle::ReconciliationFailed => {
                increment(&mut self.settlement.reconciliation_failed_count);
            }
            SettlementLifecycle::Paused => increment(&mut self.settlement.paused_count),
        }
    }

    /// Record one denied operator action without retaining actor identity.
    pub fn observe_unauthorized_attempt(&mut self) {
        increment(&mut self.unauthorized_attempts);
    }
}

fn increment(value: &mut u64) {
    *value = value.saturating_add(1);
}

/// Versioned durable job projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableJob {
    /// Stable job identity.
    pub job_id: String,
    /// Versioned handler kind.
    pub job_type: String,
    /// Idempotency identity for the intended effect.
    pub idempotency_key: String,
    /// Immutable canonical payload digest.
    pub payload_digest: Sha256Digest,
    /// Current status.
    pub status: JobStatus,
    /// Number of failed attempts.
    pub attempt_count: u16,
    /// Maximum failed attempts before quarantine.
    pub max_attempts: u16,
    /// Current lease owner.
    pub lease_owner: Option<String>,
    /// Current lease expiry.
    pub lease_expires_at: Option<UtcTimestamp>,
    /// Last heartbeat.
    pub heartbeat_at: Option<UtcTimestamp>,
    /// Earliest next retry time.
    pub next_attempt_at: Option<UtcTimestamp>,
    /// Last typed error code; never a raw sensitive payload.
    pub last_error_code: Option<String>,
}

/// Job transition failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JobError {
    /// Job is not currently leasable.
    #[error("job is not leasable")]
    NotLeasable,
    /// Worker does not own the active lease.
    #[error("worker does not own the active lease")]
    LeaseOwnerMismatch,
    /// Operation requires a running job.
    #[error("job is not running")]
    NotRunning,
    /// Timestamp arithmetic overflowed.
    #[error("job timestamp arithmetic failed")]
    Timestamp(#[from] outcomes_domain::TimestampError),
}

impl DurableJob {
    /// Lease a queued or due retry job to one worker.
    ///
    /// # Errors
    ///
    /// Returns [`JobError::NotLeasable`] if state, retry time, or an active lease
    /// prevents acquisition.
    pub fn lease(
        &mut self,
        worker_id: &str,
        now: &UtcTimestamp,
        lease_seconds: u64,
    ) -> Result<(), JobError> {
        let state_allows = matches!(self.status, JobStatus::Queued | JobStatus::RetryWait);
        let retry_due = self
            .next_attempt_at
            .as_ref()
            .is_none_or(|retry| retry <= now);
        let prior_lease_expired = self
            .lease_expires_at
            .as_ref()
            .is_none_or(|expiry| expiry <= now);
        if !state_allows || !retry_due || !prior_lease_expired {
            return Err(JobError::NotLeasable);
        }
        self.status = JobStatus::Running;
        self.lease_owner = Some(worker_id.to_owned());
        self.heartbeat_at = Some(now.clone());
        self.lease_expires_at = Some(now.checked_add_seconds(lease_seconds)?);
        self.next_attempt_at = None;
        Ok(())
    }

    /// Extend a live lease owned by the same worker.
    ///
    /// # Errors
    ///
    /// Returns a running-state, ownership, or timestamp error.
    pub fn heartbeat(
        &mut self,
        worker_id: &str,
        now: &UtcTimestamp,
        lease_seconds: u64,
    ) -> Result<(), JobError> {
        self.require_owner(worker_id)?;
        self.heartbeat_at = Some(now.clone());
        self.lease_expires_at = Some(now.checked_add_seconds(lease_seconds)?);
        Ok(())
    }

    /// Complete a running job owned by the worker.
    ///
    /// # Errors
    ///
    /// Returns a running-state or ownership error.
    pub fn complete(&mut self, worker_id: &str) -> Result<(), JobError> {
        self.require_owner(worker_id)?;
        self.status = JobStatus::Completed;
        self.clear_lease();
        Ok(())
    }

    /// Record a classified failure with bounded exponential backoff and jitter.
    ///
    /// # Errors
    ///
    /// Returns a running-state, ownership, or timestamp error.
    pub fn fail(
        &mut self,
        worker_id: &str,
        now: &UtcTimestamp,
        retryable: bool,
        error_code: &str,
    ) -> Result<(), JobError> {
        self.require_owner(worker_id)?;
        self.attempt_count = self.attempt_count.saturating_add(1);
        self.last_error_code = Some(error_code.to_owned());
        self.clear_lease();
        if !retryable || self.attempt_count >= self.max_attempts {
            self.status = JobStatus::DeadLetter;
            self.next_attempt_at = None;
            return Ok(());
        }
        let exponent = u32::from(self.attempt_count.min(8));
        let base = 1_u64 << exponent;
        let jitter = u64::from(self.payload_digest.as_bytes()[0] % 7);
        self.status = JobStatus::RetryWait;
        self.next_attempt_at = Some(now.checked_add_seconds((base + jitter).min(300))?);
        Ok(())
    }

    fn require_owner(&self, worker_id: &str) -> Result<(), JobError> {
        if self.status != JobStatus::Running {
            return Err(JobError::NotRunning);
        }
        if self.lease_owner.as_deref() != Some(worker_id) {
            return Err(JobError::LeaseOwnerMismatch);
        }
        Ok(())
    }

    fn clear_lease(&mut self) {
        self.lease_owner = None;
        self.lease_expires_at = None;
        self.heartbeat_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn job() -> DurableJob {
        DurableJob {
            job_id: "job-1".to_owned(),
            job_type: "VERIFY_CANDIDATE".to_owned(),
            idempotency_key: "verify-claim-candidate-attempt-1".to_owned(),
            payload_digest: Sha256Digest::from_bytes([3; 32]),
            status: JobStatus::Queued,
            attempt_count: 0,
            max_attempts: 3,
            lease_owner: None,
            lease_expires_at: None,
            heartbeat_at: None,
            next_attempt_at: None,
            last_error_code: None,
        }
    }

    #[test]
    fn contract_lease_owner_and_completion_are_enforced() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut job = job();
        job.lease("worker-a", &timestamp("2026-07-10T12:00:00Z")?, 30)?;
        assert_eq!(job.complete("worker-b"), Err(JobError::LeaseOwnerMismatch));
        job.complete("worker-a")?;
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(
            job.lease("worker-a", &timestamp("2026-07-10T12:01:00Z")?, 30),
            Err(JobError::NotLeasable)
        );
        Ok(())
    }

    #[test]
    fn contract_retries_are_bounded_and_dead_lettered() -> Result<(), Box<dyn std::error::Error>> {
        let mut job = job();
        for index in 0..3 {
            let now = timestamp(&format!("2026-07-10T12:0{index}:00Z"))?;
            job.lease("worker-a", &now, 30)?;
            job.fail("worker-a", &now, true, "TRANSIENT_SOURCE")?;
            if index < 2 {
                let retry = job.next_attempt_at.clone().ok_or("retry time must exist")?;
                job.next_attempt_at = Some(now);
                assert!(retry > timestamp("2026-07-10T12:00:00Z")?);
            }
        }
        assert_eq!(job.status, JobStatus::DeadLetter);
        assert_eq!(job.attempt_count, 3);
        Ok(())
    }

    #[test]
    fn contract_operational_snapshot_has_only_fixed_cardinality_dimensions()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut snapshot = OperationalSnapshot::new(timestamp("2026-07-11T01:00:00Z")?);
        snapshot.observe_claim(ClaimTruth::True, ResolutionFinality::Final);
        snapshot.observe_claim(ClaimTruth::Unresolved, ResolutionFinality::None);
        snapshot.observe_verification(ExecutionOutcome::Pass);
        snapshot.observe_verification(ExecutionOutcome::Error);
        snapshot.observe_job(JobStatus::DeadLetter);
        snapshot.observe_settlement(SettlementLifecycle::Confirmed);
        snapshot.observe_unauthorized_attempt();
        let json = serde_json::to_string(&snapshot)?;
        assert_eq!(snapshot.claims.true_count, 1);
        assert_eq!(snapshot.claims.unresolved_count, 1);
        assert_eq!(snapshot.verification.error_count, 1);
        assert_eq!(snapshot.jobs.dead_letter_count, 1);
        assert_eq!(snapshot.settlement.confirmed_count, 1);
        assert_eq!(snapshot.unauthorized_attempts, 1);
        for prohibited in [
            "claim_id",
            "candidate_id",
            "wallet",
            "raw_error",
            "signer_ref",
        ] {
            assert!(!json.contains(prohibited));
        }
        Ok(())
    }
}
