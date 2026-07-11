//! Durable job, lease, outbox, and recovery composition.

use outcomes_domain::{Sha256Digest, UtcTimestamp};
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
}
