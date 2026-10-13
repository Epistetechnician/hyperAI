//! Generic, backend-neutral values shared by `HyperAI` protocol domains.

mod canonical;
mod digest;
mod json;
mod timestamp;

pub use canonical::{CanonicalizationError, canonical_json, domain_separated_digest};
pub use digest::{DigestParseError, Sha256Digest};
pub use json::parse_json_without_duplicate_keys;
pub use timestamp::{TimestampError, UtcTimestamp};
