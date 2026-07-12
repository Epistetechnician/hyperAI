use crate::{CanonicalizationError, Sha256Digest, SubmissionReceiptV1, domain_separated_digest};

/// Protocol domain separator for the canonical submission-receipt identity.
const SUBMISSION_RECEIPT_DIGEST_DOMAIN_V1: &str = "hsai.submission-receipt.v1";

/// Derive the canonical protocol identity of a submission receipt.
///
/// Backends, persistence adapters, and reconciliation records must use this
/// shared identity instead of introducing storage-specific receipt digests.
///
/// # Errors
///
/// Returns an error when the receipt cannot be safely canonicalized.
pub fn submission_receipt_digest(
    receipt: &SubmissionReceiptV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(SUBMISSION_RECEIPT_DIGEST_DOMAIN_V1, receipt)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::UtcTimestamp;

    fn receipt() -> Result<SubmissionReceiptV1, Box<dyn std::error::Error>> {
        Ok(SubmissionReceiptV1 {
            schema_version: "hsai.submission-receipt.v1".to_owned(),
            intent_id: "intent_receipt_identity".to_owned(),
            idempotency_key: "settlement_receipt_identity".to_owned(),
            backend_receipt_id: "mock_receipt_identity".to_owned(),
            accepted_at: UtcTimestamp::from_str("2026-07-10T12:00:00Z")?,
            request_digest: Sha256Digest::from_bytes([17; 32]),
            response_digest: Sha256Digest::from_bytes([34; 32]),
            external_sequence: Some("1".to_owned()),
        })
    }

    #[test]
    fn contract_submission_receipt_digest_is_stable() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = receipt()?;
        let digest = submission_receipt_digest(&receipt)?;

        assert_eq!(digest, submission_receipt_digest(&receipt.clone())?);
        assert_eq!(
            digest.to_hex(),
            "20a4ee8eda7c1b7101e9be349e143e1c911b51fa152bfb8bf2d3b95a780c5ed9"
        );
        Ok(())
    }

    #[test]
    fn contract_submission_receipt_digest_commits_to_receipt()
    -> Result<(), Box<dyn std::error::Error>> {
        let receipt = receipt()?;
        let mut changed = receipt.clone();
        changed.external_sequence = Some("2".to_owned());

        assert_ne!(
            submission_receipt_digest(&receipt)?,
            submission_receipt_digest(&changed)?
        );
        Ok(())
    }
}
