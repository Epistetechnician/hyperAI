//! Bounded, authority-negative broker journal.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BrokerError {
    #[error("replayed attempt: {0}")]
    Replay(String),
    #[error("invalid receipt chain")]
    Chain,
    #[error("receipt digest mismatch")]
    Digest,
    #[error("operation is not reversible")]
    Irreversible,
    #[error("receipt field is invalid: {0}")]
    Invalid(&'static str),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationReceipt {
    pub state_slice: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub input_digest: String,
    pub output_digest: String,
    pub previous_receipt_digest: String,
    pub receipt_digest: String,
    pub accepted_evidence_write: bool,
}

#[derive(Debug, Default)]
pub struct BrokerJournal {
    receipts: Vec<OperationReceipt>,
}

impl BrokerJournal {
    pub fn append(
        &mut self,
        state_slice: &str,
        operation_id: &str,
        attempt_id: &str,
        input: &[u8],
        output: &[u8],
    ) -> Result<OperationReceipt, BrokerError> {
        if state_slice != "justai-custody-v1" {
            return Err(BrokerError::Invalid("state_slice"));
        }
        if operation_id.trim().is_empty() {
            return Err(BrokerError::Invalid("operation_id"));
        }
        if attempt_id.trim().is_empty() {
            return Err(BrokerError::Invalid("attempt_id"));
        }
        if self
            .receipts
            .iter()
            .any(|receipt| receipt.attempt_id == attempt_id)
        {
            return Err(BrokerError::Replay(attempt_id.into()));
        }
        let input_digest = digest(input);
        let output_digest = digest(output);
        let previous_receipt_digest = self
            .receipts
            .last()
            .map(|receipt| receipt.receipt_digest.clone())
            .unwrap_or_else(|| "0".repeat(64));
        let body = serde_json::json!({
            "state_slice": state_slice,
            "operation_id": operation_id,
            "attempt_id": attempt_id,
            "input_digest": input_digest,
            "output_digest": output_digest,
            "previous_receipt_digest": previous_receipt_digest,
            "accepted_evidence_write": false
        });
        let receipt = OperationReceipt {
            state_slice: state_slice.into(),
            operation_id: operation_id.into(),
            attempt_id: attempt_id.into(),
            input_digest,
            output_digest,
            previous_receipt_digest,
            receipt_digest: digest(&canonical_json(&body)),
            accepted_evidence_write: false,
        };
        self.receipts.push(receipt.clone());
        Ok(receipt)
    }

    pub fn verify(&self) -> Result<(), BrokerError> {
        let mut previous = "0".repeat(64);
        for receipt in &self.receipts {
            if receipt.state_slice != "justai-custody-v1"
                || receipt.operation_id.trim().is_empty()
                || receipt.attempt_id.trim().is_empty()
                || !is_digest(&receipt.input_digest)
                || !is_digest(&receipt.output_digest)
                || !is_digest(&receipt.previous_receipt_digest)
                || !is_digest(&receipt.receipt_digest)
                || receipt.previous_receipt_digest != previous
                || receipt.accepted_evidence_write
            {
                return Err(BrokerError::Chain);
            }
            let body = serde_json::json!({
                "state_slice": receipt.state_slice,
                "operation_id": receipt.operation_id,
                "attempt_id": receipt.attempt_id,
                "input_digest": receipt.input_digest,
                "output_digest": receipt.output_digest,
                "previous_receipt_digest": receipt.previous_receipt_digest,
                "accepted_evidence_write": false
            });
            if digest(&canonical_json(&body)) != receipt.receipt_digest {
                return Err(BrokerError::Digest);
            }
            previous = receipt.receipt_digest.clone();
        }
        Ok(())
    }

    pub fn receipts(&self) -> &[OperationReceipt] {
        &self.receipts
    }
}

pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn canonical_json(value: &serde_json::Value) -> Vec<u8> {
    serde_jcs::to_vec(value).expect("JSON values are serializable")
}

pub fn require_reversible(reversible: bool) -> Result<(), BrokerError> {
    if reversible {
        Ok(())
    } else {
        Err(BrokerError::Irreversible)
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_replay_protected_authority_negative_receipt() {
        let mut journal = BrokerJournal::default();
        let receipt = journal
            .append("justai-custody-v1", "op-1", "attempt-1", b"in", b"out")
            .unwrap();
        assert!(!receipt.accepted_evidence_write);
        assert_eq!(journal.receipts().len(), 1);
        assert!(journal.verify().is_ok());
        assert_eq!(
            journal.append("justai-custody-v1", "op-1", "attempt-1", b"in", b"out"),
            Err(BrokerError::Replay("attempt-1".into()))
        );
    }

    #[test]
    fn rejects_irreversible_operations_before_broker_use() {
        assert_eq!(require_reversible(false), Err(BrokerError::Irreversible));
        assert!(require_reversible(true).is_ok());
    }
}
