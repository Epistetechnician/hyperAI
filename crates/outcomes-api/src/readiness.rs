//! Fixed-schema, fail-closed API dependency readiness.

use std::{fmt, sync::Arc, time::SystemTime};

use serde::Serialize;
use thiserror::Error;

/// Maximum permitted freshness window for a readiness snapshot.
pub const MAX_READINESS_AGE_SECONDS: u64 = 3_600;

/// Fixed readiness state for one required dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyState {
    /// The dependency is available for required work.
    Ready,
    /// The dependency responds but does not meet the readiness contract.
    Degraded,
    /// The dependency is unavailable.
    Unavailable,
    /// No current authoritative state is available.
    Unknown,
}

/// Fixed dependency dimensions required by the API readiness contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DependencyReadiness {
    /// Database connectivity and required schema compatibility.
    pub database_schema: DependencyState,
    /// Transactional outbox availability.
    pub outbox: DependencyState,
    /// Immutable evidence-store availability.
    pub evidence_store: DependencyState,
    /// Required signer registry/key references.
    pub signer_registry: DependencyState,
    /// Required source adapters for active work.
    pub required_sources: DependencyState,
}

impl DependencyReadiness {
    /// Construct all fixed dimensions with the same state.
    #[must_use]
    pub const fn all(state: DependencyState) -> Self {
        Self {
            database_schema: state,
            outbox: state,
            evidence_store: state,
            signer_registry: state,
            required_sources: state,
        }
    }

    pub(crate) const fn all_ready(self) -> bool {
        matches!(self.database_schema, DependencyState::Ready)
            && matches!(self.outbox, DependencyState::Ready)
            && matches!(self.evidence_store, DependencyState::Ready)
            && matches!(self.signer_registry, DependencyState::Ready)
            && matches!(self.required_sources, DependencyState::Ready)
    }
}

/// Precomputed fixed-schema dependency snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessSnapshot {
    /// Snapshot observation time as whole Unix seconds.
    pub observed_at_unix_seconds: u64,
    /// Fixed dependency states.
    pub dependencies: DependencyReadiness,
}

/// Sanitized provider failure without raw error content.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[error("readiness snapshot unavailable")]
pub struct ReadinessProviderError;

/// Non-blocking provider of a previously observed readiness snapshot.
///
/// Implementations must return cached state and must not perform network,
/// filesystem, signer, or database probes in the request path.
pub trait ReadinessSnapshotProvider: Send + Sync {
    /// Return the current cached snapshot.
    ///
    /// # Errors
    ///
    /// Returns a sanitized availability error. Raw dependency failures must be
    /// logged and retained outside this public API contract.
    fn snapshot(&self) -> Result<ReadinessSnapshot, ReadinessProviderError>;
}

/// Sanitized clock failure without raw error content.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[error("readiness clock unavailable")]
pub struct ReadinessClockError;

/// Clock contract injected so freshness is evaluated on every request.
pub trait ReadinessClock: Send + Sync {
    /// Return current whole Unix seconds.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error when current time is unavailable.
    fn now_unix_seconds(&self) -> Result<u64, ReadinessClockError>;
}

/// System clock used by the default API composition.
#[derive(Debug, Default)]
pub struct SystemReadinessClock;

impl ReadinessClock for SystemReadinessClock {
    fn now_unix_seconds(&self) -> Result<u64, ReadinessClockError> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| ReadinessClockError)
            .map(|duration| duration.as_secs())
    }
}

/// Fixed freshness classification exposed only by the protected detail route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadinessFreshness {
    /// Snapshot age is within the configured inclusive window.
    Fresh,
    /// Snapshot is older than the configured window.
    Stale,
    /// Current time predates the snapshot observation time.
    ClockRegression,
    /// Snapshot or clock state could not be obtained.
    Unavailable,
}

/// Fixed readiness evaluation generated per request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadinessEvaluation {
    pub(crate) freshness: ReadinessFreshness,
    pub(crate) dependencies: DependencyReadiness,
}

impl ReadinessEvaluation {
    pub(crate) const fn is_ready(self) -> bool {
        matches!(self.freshness, ReadinessFreshness::Fresh) && self.dependencies.all_ready()
    }
}

/// Invalid readiness monitor configuration.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum ReadinessConfigError {
    /// Freshness must be bounded and nonzero.
    #[error("readiness max age must be between 1 and {MAX_READINESS_AGE_SECONDS} seconds")]
    InvalidMaxAge,
}

/// Per-request evaluator over injected cached state and time.
#[derive(Clone)]
pub struct ReadinessMonitor {
    provider: Arc<dyn ReadinessSnapshotProvider>,
    clock: Arc<dyn ReadinessClock>,
    max_age_seconds: u64,
}

impl fmt::Debug for ReadinessMonitor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReadinessMonitor")
            .field("provider", &"<injected>")
            .field("clock", &"<injected>")
            .field("max_age_seconds", &self.max_age_seconds)
            .finish()
    }
}

impl ReadinessMonitor {
    /// Construct a bounded readiness monitor.
    ///
    /// # Errors
    ///
    /// Returns [`ReadinessConfigError::InvalidMaxAge`] for a zero or unbounded
    /// freshness window.
    pub fn new(
        provider: Arc<dyn ReadinessSnapshotProvider>,
        clock: Arc<dyn ReadinessClock>,
        max_age_seconds: u64,
    ) -> Result<Self, ReadinessConfigError> {
        if max_age_seconds == 0 || max_age_seconds > MAX_READINESS_AGE_SECONDS {
            return Err(ReadinessConfigError::InvalidMaxAge);
        }
        Ok(Self {
            provider,
            clock,
            max_age_seconds,
        })
    }

    /// Evaluate dependency state and freshness against time obtained now.
    #[must_use]
    pub(crate) fn evaluate(&self) -> ReadinessEvaluation {
        let Ok(snapshot) = self.provider.snapshot() else {
            return unavailable_evaluation();
        };
        let Ok(now) = self.clock.now_unix_seconds() else {
            return ReadinessEvaluation {
                freshness: ReadinessFreshness::Unavailable,
                dependencies: snapshot.dependencies,
            };
        };
        let freshness = if now < snapshot.observed_at_unix_seconds {
            ReadinessFreshness::ClockRegression
        } else if now - snapshot.observed_at_unix_seconds > self.max_age_seconds {
            ReadinessFreshness::Stale
        } else {
            ReadinessFreshness::Fresh
        };
        ReadinessEvaluation {
            freshness,
            dependencies: snapshot.dependencies,
        }
    }

    pub(crate) fn unconfigured() -> Self {
        Self {
            provider: Arc::new(UnavailableReadinessProvider),
            clock: Arc::new(SystemReadinessClock),
            max_age_seconds: 30,
        }
    }
}

#[derive(Debug)]
struct UnavailableReadinessProvider;

impl ReadinessSnapshotProvider for UnavailableReadinessProvider {
    fn snapshot(&self) -> Result<ReadinessSnapshot, ReadinessProviderError> {
        Err(ReadinessProviderError)
    }
}

const fn unavailable_evaluation() -> ReadinessEvaluation {
    ReadinessEvaluation {
        freshness: ReadinessFreshness::Unavailable,
        dependencies: DependencyReadiness::all(DependencyState::Unknown),
    }
}
