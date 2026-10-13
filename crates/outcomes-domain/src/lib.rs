//! Stable, backend-neutral outcome protocol types and invariants.
//!
//! This crate is intentionally free of HTTP, database, object-store, cloud,
//! container-runtime, Hyperliquid SDK, and challenge-specific dependencies.

mod authorization;
mod identifiers;
mod model;
mod mutation;
mod receipt_identity;
mod records;
mod state;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub use authorization::*;
pub use hyperai_foundation::{
    CanonicalizationError, Sha256Digest, TimestampError, UtcTimestamp, canonical_json,
    domain_separated_digest,
};
pub use identifiers::ClaimId;
pub use model::*;
pub use mutation::{MutationContext, MutationScope, StateSlice, UndeclaredMutation};
pub use receipt_identity::submission_receipt_digest;
pub use records::*;
pub use state::{
    ClaimTruth, ExecutionOutcome, FinalityTransitionError, ResolutionFinality, SettlementLifecycle,
    SettlementTransitionError,
};
