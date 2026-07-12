//! Stable, backend-neutral outcome protocol types and invariants.
//!
//! This crate is intentionally free of HTTP, database, object-store, cloud,
//! container-runtime, Hyperliquid SDK, and challenge-specific dependencies.

mod authorization;
mod canonical;
mod identifiers;
mod model;
mod mutation;
mod receipt_identity;
mod records;
mod state;
mod timestamp;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub use authorization::*;
pub use canonical::{CanonicalizationError, canonical_json, domain_separated_digest};
pub use identifiers::{ClaimId, Sha256Digest};
pub use model::*;
pub use mutation::{MutationContext, MutationScope, StateSlice, UndeclaredMutation};
pub use receipt_identity::submission_receipt_digest;
pub use records::*;
pub use state::{
    ClaimTruth, ExecutionOutcome, FinalityTransitionError, ResolutionFinality, SettlementLifecycle,
    SettlementTransitionError,
};
pub use timestamp::{TimestampError, UtcTimestamp};
