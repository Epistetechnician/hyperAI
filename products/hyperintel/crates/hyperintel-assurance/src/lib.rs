//! Generic assurance algebra and signed evidence verification.
//!
//! This crate depends only on `hyperintel-core`. It deliberately knows nothing
//! about routing, metering economics, settlement, wallets, or blockchains.

mod envelope;
mod evidence;
mod signature;

pub use envelope::{
    AssuranceAcceptancePolicyV1, AssuranceClaimV1, AssuranceEnvelopeV1, AssuranceRejectionV1,
    AssuranceRootClassV1, AssuranceRootV1, ValidatedAssuranceV1, ValidityWindowV1,
    conjoin_envelopes_v1, evaluate_assurance_v1,
};
pub use evidence::{
    AssuranceEvidenceError, AssuranceEvidenceSetV1, EvidenceBlobRefV1, EvidenceClassification,
    build_assurance_evidence_set_v1, evidence_blob_ref_v1, verify_assurance_evidence_set_v1,
};
pub use signature::{
    Ed25519MessageVerificationInputV1, RegisteredVerificationKeyV1, SignatureError,
    SignedStatementV1, VerificationKeyRegistry, verify_ed25519_message_v1,
    verify_signed_statement_v1,
};
