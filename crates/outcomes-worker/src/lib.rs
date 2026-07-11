//! Durable job, lease, outbox, and recovery composition.

use outcomes_domain::{
    ClaimTruth, ExecutionOutcome, ResolutionFinality, SettlementLifecycle, Sha256Digest,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use thiserror::Error;

/// `OpenMetrics` 1.0 content type for [`OperationalSnapshot::to_openmetrics`].
pub const OPENMETRICS_CONTENT_TYPE: &str =
    "application/openmetrics-text; version=1.0.0; charset=utf-8";

/// Hard ceiling for one fixed-schema operational snapshot export.
///
/// The current schema is substantially smaller. This limit prevents an
/// accidental future change from turning the dependency-free renderer into an
/// unbounded serialization surface.
pub const MAX_OPENMETRICS_EXPORT_BYTES: usize = 16 * 1024;

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

/// Failure to validate or render a fixed-schema operational snapshot.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetricsExportError {
    /// A counter group overflowed while its projection invariant was checked.
    #[error("operational snapshot {group} counter sum overflowed")]
    CounterOverflow {
        /// Fixed counter group name; never externally supplied.
        group: &'static str,
    },
    /// Every observed claim has exactly one truth and one finality projection.
    #[error(
        "operational snapshot claim/finality totals differ: claims={claims}, finality={finality}"
    )]
    ClaimFinalityMismatch {
        /// Number of projected claims.
        claims: u64,
        /// Number of projected finality states.
        finality: u64,
    },
    /// A safe constructor invariant for the canonical timestamp was violated.
    #[error("operational snapshot generated_at is invalid")]
    InvalidGeneratedAt,
    /// The fixed-schema output exceeded its compile-time safety ceiling.
    #[error("operational snapshot export exceeds {MAX_OPENMETRICS_EXPORT_BYTES} bytes")]
    OutputTooLarge,
    /// Writing to the in-memory string unexpectedly failed.
    #[error("operational snapshot formatting failed")]
    Formatting,
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

    /// Validate cross-projection invariants before telemetry export.
    ///
    /// Claim truth and finality are recorded by the same operation, so their
    /// totals must agree. Dead-letter state has only one authoritative source,
    /// [`JobCounters::dead_letter_count`], and therefore needs no mirror check.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsExportError`] if a sum overflows or the claim and
    /// finality projections do not describe the same number of claims.
    pub fn validate_metrics(&self) -> Result<(), MetricsExportError> {
        let claims = checked_sum(
            "claims",
            &[
                self.claims.true_count,
                self.claims.false_count,
                self.claims.unresolved_count,
            ],
        )?;
        let finality = checked_sum(
            "finality",
            &[
                self.finality.none_count,
                self.finality.provisional_count,
                self.finality.disputed_count,
                self.finality.final_count,
            ],
        )?;
        if claims != finality {
            return Err(MetricsExportError::ClaimFinalityMismatch { claims, finality });
        }
        Ok(())
    }

    /// Render one deterministic, fixed-cardinality `OpenMetrics` 1.0 document.
    ///
    /// Metric names, help text, and types are compile-time constants. Samples
    /// have no labels, and the only rendered values are validated integers.
    /// Consequently claim IDs, candidate IDs, wallet/signer identifiers,
    /// filesystem paths, and raw errors cannot enter this output.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsExportError`] if snapshot validation, timestamp
    /// conversion, formatting, or the fixed output-size ceiling fails.
    pub fn to_openmetrics(&self) -> Result<String, MetricsExportError> {
        self.validate_metrics()?;
        let generated_at = self
            .generated_at
            .to_datetime()
            .map_err(|_| MetricsExportError::InvalidGeneratedAt)?
            .unix_timestamp();
        let mut output = String::with_capacity(8 * 1024);
        let metrics = projection_metrics(self, generated_at)
            .into_iter()
            .chain(verification_metrics(self))
            .chain(job_metrics(self))
            .chain(settlement_metrics(self))
            .chain(misc_metrics(self));
        for item in metrics {
            item.write_to(&mut output)?;
        }
        output.push_str("# EOF\n");
        if output.len() > MAX_OPENMETRICS_EXPORT_BYTES {
            return Err(MetricsExportError::OutputTooLarge);
        }
        Ok(output)
    }
}

fn projection_metrics(snapshot: &OperationalSnapshot, generated_at: i64) -> [FixedMetric; 8] {
    [
        metric(
            "hsai_outcomes_snapshot_generated_at_seconds",
            "Unix time when this operational snapshot was generated.",
            MetricType::Gauge,
            generated_at,
        ),
        gauge(
            "hsai_outcomes_claims_true",
            "Claims currently projected as true.",
            snapshot.claims.true_count,
        ),
        gauge(
            "hsai_outcomes_claims_false",
            "Claims currently projected as false.",
            snapshot.claims.false_count,
        ),
        gauge(
            "hsai_outcomes_claims_unresolved",
            "Claims currently projected as unresolved.",
            snapshot.claims.unresolved_count,
        ),
        gauge(
            "hsai_outcomes_finality_none",
            "Claims currently having no resolution proposal.",
            snapshot.finality.none_count,
        ),
        gauge(
            "hsai_outcomes_finality_provisional",
            "Claims currently having provisional resolution.",
            snapshot.finality.provisional_count,
        ),
        gauge(
            "hsai_outcomes_finality_disputed",
            "Claims currently having disputed resolution.",
            snapshot.finality.disputed_count,
        ),
        gauge(
            "hsai_outcomes_finality_final",
            "Claims currently having final resolution.",
            snapshot.finality.final_count,
        ),
    ]
}

fn verification_metrics(snapshot: &OperationalSnapshot) -> [FixedMetric; 4] {
    [
        counter(
            "hsai_outcomes_verifications_pass",
            "hsai_outcomes_verifications_pass_total",
            "Normalized verifier pass outcomes.",
            snapshot.verification.pass_count,
        ),
        counter(
            "hsai_outcomes_verifications_fail",
            "hsai_outcomes_verifications_fail_total",
            "Normalized verifier fail outcomes.",
            snapshot.verification.fail_count,
        ),
        counter(
            "hsai_outcomes_verifications_inconclusive",
            "hsai_outcomes_verifications_inconclusive_total",
            "Normalized verifier inconclusive outcomes.",
            snapshot.verification.inconclusive_count,
        ),
        counter(
            "hsai_outcomes_verifications_error",
            "hsai_outcomes_verifications_error_total",
            "Normalized verifier error outcomes.",
            snapshot.verification.error_count,
        ),
    ]
}

fn job_metrics(snapshot: &OperationalSnapshot) -> [FixedMetric; 5] {
    [
        gauge(
            "hsai_outcomes_jobs_queued",
            "Durable jobs currently queued.",
            snapshot.jobs.queued_count,
        ),
        gauge(
            "hsai_outcomes_jobs_running",
            "Durable jobs currently running.",
            snapshot.jobs.running_count,
        ),
        gauge(
            "hsai_outcomes_jobs_retry_wait",
            "Durable jobs currently waiting for retry.",
            snapshot.jobs.retry_wait_count,
        ),
        gauge(
            "hsai_outcomes_jobs_completed",
            "Durable jobs currently completed.",
            snapshot.jobs.completed_count,
        ),
        gauge(
            "hsai_outcomes_jobs_dead_letter",
            "Durable jobs currently quarantined as dead letters.",
            snapshot.jobs.dead_letter_count,
        ),
    ]
}

fn settlement_metrics(snapshot: &OperationalSnapshot) -> [FixedMetric; 9] {
    [
        gauge(
            "hsai_outcomes_settlement_unbound",
            "Settlements currently unbound.",
            snapshot.settlement.unbound_count,
        ),
        gauge(
            "hsai_outcomes_settlement_bound",
            "Settlements currently bound.",
            snapshot.settlement.bound_count,
        ),
        gauge(
            "hsai_outcomes_settlement_not_ready",
            "Settlements currently blocked by preflight.",
            snapshot.settlement.not_ready_count,
        ),
        gauge(
            "hsai_outcomes_settlement_ready",
            "Settlements currently ready.",
            snapshot.settlement.ready_count,
        ),
        gauge(
            "hsai_outcomes_settlement_intent_built",
            "Settlements currently having a persisted unsigned intent.",
            snapshot.settlement.intent_built_count,
        ),
        gauge(
            "hsai_outcomes_settlement_submitted",
            "Settlements currently submitted.",
            snapshot.settlement.submitted_count,
        ),
        gauge(
            "hsai_outcomes_settlement_confirmed",
            "Settlements currently confirmed.",
            snapshot.settlement.confirmed_count,
        ),
        gauge(
            "hsai_outcomes_settlement_reconciliation_failed",
            "Settlements currently having failed or ambiguous reconciliation.",
            snapshot.settlement.reconciliation_failed_count,
        ),
        gauge(
            "hsai_outcomes_settlement_paused",
            "Settlements currently paused by an operator.",
            snapshot.settlement.paused_count,
        ),
    ]
}

fn misc_metrics(snapshot: &OperationalSnapshot) -> [FixedMetric; 2] {
    [
        gauge(
            "hsai_outcomes_outbox_backlog",
            "Current transactional outbox backlog.",
            snapshot.outbox_backlog,
        ),
        counter(
            "hsai_outcomes_unauthorized_attempts",
            "hsai_outcomes_unauthorized_attempts_total",
            "Denied operator or API requests.",
            snapshot.unauthorized_attempts,
        ),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricType {
    Gauge,
    Counter,
}

impl MetricType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Gauge => "gauge",
            Self::Counter => "counter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FixedMetric {
    family_name: &'static str,
    sample_name: &'static str,
    help: &'static str,
    metric_type: MetricType,
    value: i128,
}

impl FixedMetric {
    fn write_to(self, output: &mut String) -> Result<(), MetricsExportError> {
        writeln!(output, "# HELP {} {}", self.family_name, self.help)
            .map_err(|_| MetricsExportError::Formatting)?;
        writeln!(
            output,
            "# TYPE {} {}",
            self.family_name,
            self.metric_type.as_str()
        )
        .map_err(|_| MetricsExportError::Formatting)?;
        writeln!(output, "{} {}", self.sample_name, self.value)
            .map_err(|_| MetricsExportError::Formatting)
    }
}

const fn metric(
    name: &'static str,
    help: &'static str,
    metric_type: MetricType,
    value: i64,
) -> FixedMetric {
    FixedMetric {
        family_name: name,
        sample_name: name,
        help,
        metric_type,
        value: value as i128,
    }
}

const fn gauge(name: &'static str, help: &'static str, value: u64) -> FixedMetric {
    FixedMetric {
        family_name: name,
        sample_name: name,
        help,
        metric_type: MetricType::Gauge,
        value: value as i128,
    }
}

const fn counter(
    family_name: &'static str,
    sample_name: &'static str,
    help: &'static str,
    value: u64,
) -> FixedMetric {
    FixedMetric {
        family_name,
        sample_name,
        help,
        metric_type: MetricType::Counter,
        value: value as i128,
    }
}

fn checked_sum(group: &'static str, values: &[u64]) -> Result<u64, MetricsExportError> {
    values.iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(*value)
            .ok_or(MetricsExportError::CounterOverflow { group })
    })
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

    #[test]
    fn contract_openmetrics_export_matches_bounded_golden_schema()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut snapshot = OperationalSnapshot::new(timestamp("2026-07-11T01:00:00Z")?);
        snapshot.claims = ClaimCounters {
            true_count: 1,
            false_count: 2,
            unresolved_count: 3,
        };
        snapshot.finality = FinalityCounters {
            none_count: 1,
            provisional_count: 1,
            disputed_count: 1,
            final_count: 3,
        };
        snapshot.verification = VerificationCounters {
            pass_count: 4,
            fail_count: 5,
            inconclusive_count: 6,
            error_count: 7,
        };
        snapshot.jobs = JobCounters {
            queued_count: 8,
            running_count: 9,
            retry_wait_count: 10,
            completed_count: 11,
            dead_letter_count: 12,
        };
        snapshot.settlement = SettlementCounters {
            unbound_count: 13,
            bound_count: 14,
            not_ready_count: 15,
            ready_count: 16,
            intent_built_count: 17,
            submitted_count: 18,
            confirmed_count: 19,
            reconciliation_failed_count: 20,
            paused_count: 21,
        };
        snapshot.outbox_backlog = 22;
        snapshot.unauthorized_attempts = 23;

        let rendered = snapshot.to_openmetrics()?;
        let parsed = parse_openmetrics_bounded(&rendered, MAX_OPENMETRICS_EXPORT_BYTES, 28)?;
        let golden = [
            (
                "hsai_outcomes_snapshot_generated_at_seconds",
                "gauge",
                1_783_731_600,
            ),
            ("hsai_outcomes_claims_true", "gauge", 1),
            ("hsai_outcomes_claims_false", "gauge", 2),
            ("hsai_outcomes_claims_unresolved", "gauge", 3),
            ("hsai_outcomes_finality_none", "gauge", 1),
            ("hsai_outcomes_finality_provisional", "gauge", 1),
            ("hsai_outcomes_finality_disputed", "gauge", 1),
            ("hsai_outcomes_finality_final", "gauge", 3),
            ("hsai_outcomes_verifications_pass_total", "counter", 4),
            ("hsai_outcomes_verifications_fail_total", "counter", 5),
            (
                "hsai_outcomes_verifications_inconclusive_total",
                "counter",
                6,
            ),
            ("hsai_outcomes_verifications_error_total", "counter", 7),
            ("hsai_outcomes_jobs_queued", "gauge", 8),
            ("hsai_outcomes_jobs_running", "gauge", 9),
            ("hsai_outcomes_jobs_retry_wait", "gauge", 10),
            ("hsai_outcomes_jobs_completed", "gauge", 11),
            ("hsai_outcomes_jobs_dead_letter", "gauge", 12),
            ("hsai_outcomes_settlement_unbound", "gauge", 13),
            ("hsai_outcomes_settlement_bound", "gauge", 14),
            ("hsai_outcomes_settlement_not_ready", "gauge", 15),
            ("hsai_outcomes_settlement_ready", "gauge", 16),
            ("hsai_outcomes_settlement_intent_built", "gauge", 17),
            ("hsai_outcomes_settlement_submitted", "gauge", 18),
            ("hsai_outcomes_settlement_confirmed", "gauge", 19),
            (
                "hsai_outcomes_settlement_reconciliation_failed",
                "gauge",
                20,
            ),
            ("hsai_outcomes_settlement_paused", "gauge", 21),
            ("hsai_outcomes_outbox_backlog", "gauge", 22),
            ("hsai_outcomes_unauthorized_attempts_total", "counter", 23),
        ];
        assert_eq!(parsed, golden);
        assert_eq!(
            rendered.matches("hsai_outcomes_jobs_dead_letter ").count(),
            3
        );
        assert!(rendered.ends_with("# EOF\n"));
        assert!(rendered.len() <= MAX_OPENMETRICS_EXPORT_BYTES);
        Ok(())
    }

    #[test]
    fn contract_openmetrics_rejects_invalid_projection_and_overflow()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut mismatch = OperationalSnapshot::new(timestamp("2026-07-11T01:00:00Z")?);
        mismatch.claims.true_count = 1;
        assert_eq!(
            mismatch.to_openmetrics(),
            Err(MetricsExportError::ClaimFinalityMismatch {
                claims: 1,
                finality: 0,
            })
        );

        let mut overflow = OperationalSnapshot::new(timestamp("2026-07-11T01:00:00Z")?);
        overflow.claims.true_count = u64::MAX;
        overflow.claims.false_count = 1;
        assert_eq!(
            overflow.to_openmetrics(),
            Err(MetricsExportError::CounterOverflow { group: "claims" })
        );
        Ok(())
    }

    #[test]
    fn contract_openmetrics_cannot_render_sensitive_or_injected_content()
    -> Result<(), Box<dyn std::error::Error>> {
        let hostile = [
            "claim_SECRET\ninjected_claim_metric 1",
            "candidate_SECRET{label=\"x\"}",
            "wallet_SECRET",
            "signer_SECRET",
            "/private/path_SECRET",
            "raw_error_SECRET",
        ];
        let mut hostile_job = job();
        hostile_job.job_id = hostile[0].to_owned();
        hostile_job.job_type = hostile[1].to_owned();
        hostile_job.idempotency_key = hostile[2].to_owned();
        hostile_job.lease_owner = Some(hostile[3].to_owned());
        hostile_job.last_error_code = Some(format!("{}:{}", hostile[4], hostile[5]));
        hostile_job.status = JobStatus::DeadLetter;

        let mut snapshot = OperationalSnapshot::new(timestamp("2026-07-11T01:00:00Z")?);
        snapshot.observe_job(hostile_job.status);
        let rendered = snapshot.to_openmetrics()?;
        for value in hostile {
            assert!(!rendered.contains(value));
        }
        assert!(!rendered.contains('{'));
        assert!(!rendered.contains('"'));
        assert!(!rendered.contains("injected_claim_metric"));
        Ok(())
    }

    fn parse_openmetrics_bounded(
        input: &str,
        max_bytes: usize,
        max_metrics: usize,
    ) -> Result<Vec<(&str, &str, i128)>, &'static str> {
        if input.len() > max_bytes {
            return Err("input exceeds byte ceiling");
        }
        let mut lines = input.lines();
        let mut parsed = Vec::with_capacity(max_metrics);
        loop {
            let line = lines.next().ok_or("missing EOF")?;
            if line == "# EOF" {
                if lines.next().is_some() {
                    return Err("content after EOF");
                }
                return Ok(parsed);
            }
            if parsed.len() >= max_metrics {
                return Err("metric count exceeds ceiling");
            }
            let help = line.strip_prefix("# HELP ").ok_or("missing HELP")?;
            let (help_family, help_text) = help.split_once(' ').ok_or("malformed HELP")?;
            if help_text.is_empty() || help_family.contains(['{', '}', '"', '\\']) {
                return Err("unsafe HELP");
            }
            let type_line = lines.next().ok_or("missing TYPE")?;
            let metric_type = type_line.strip_prefix("# TYPE ").ok_or("malformed TYPE")?;
            let (type_family, metric_type) = metric_type.split_once(' ').ok_or("malformed TYPE")?;
            if help_family != type_family || !matches!(metric_type, "gauge" | "counter") {
                return Err("metric family mismatch");
            }
            let sample = lines.next().ok_or("missing sample")?;
            let (sample_name, value) = sample.split_once(' ').ok_or("malformed sample")?;
            if sample_name.contains(['{', '}', '"', '\\']) || value.contains(' ') {
                return Err("unsafe sample");
            }
            if metric_type == "gauge" && sample_name != help_family {
                return Err("gauge sample name mismatch");
            }
            if metric_type == "counter" && sample_name.strip_suffix("_total") != Some(help_family) {
                return Err("counter sample name mismatch");
            }
            let value = value.parse().map_err(|_| "non-integer sample")?;
            if parsed.iter().any(|(name, _, _)| *name == sample_name) {
                return Err("duplicate sample");
            }
            parsed.push((sample_name, metric_type, value));
        }
    }
}
