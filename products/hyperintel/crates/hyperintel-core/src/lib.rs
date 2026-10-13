//! Canonical identities and inference-native protocol records.
//!
//! This crate is pure data. It performs no network, filesystem, provider,
//! wallet, or blockchain operations.

mod canonical;
mod exact;
mod identifiers;
mod model;
mod timestamp;

pub use canonical::{CanonicalizationError, canonical_json, domain_separated_digest};
pub use exact::{AtomicAmount, BasisPoints, ExactArithmeticError, PartsPerMillion};
pub use identifiers::{IdentifierError, ProtocolId, Sha256Digest};
pub use model::{
    AssuranceMaturity, AssuranceProperty, CapacityOfferV1, InferenceCapability,
    InferenceRequestPolicyV1, MeteringMethod, PrivacyMode, ProviderIdentityV1, ProviderStatus,
    RetentionMode, ServiceSpecV1,
};
pub use timestamp::{TimestampError, UtcTimestamp};
