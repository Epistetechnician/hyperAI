//! Hyperliquid HIP-4 metadata, binding, mock, and fail-closed capability adapter.

use std::collections::{BTreeMap, HashMap};

use outcomes_domain::{
    ExternalSideV1, MarketBindingV1, ReconciliationResultV1, ReconciliationStatus,
    ResolutionRecordV1, SettlementLifecycle, SettlementNetwork, Sha256Digest, SubmissionReceiptV1,
    UnsignedSettlementIntentV1, UtcTimestamp, domain_separated_digest,
};
use outcomes_settlement::{BackendCapability, CapabilityMatrix, ExternalMarket};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

const ACTION_ASSET_OFFSET: u64 = 100_000_000;

/// Checked identifiers derived from one network-scoped binary outcome side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hip4AssetIds {
    /// Network-scoped outcome ID.
    pub outcome_id: u64,
    /// Documented binary side index (zero or one).
    pub side_index: u8,
    /// `10 * outcome_id + side_index`.
    pub encoding: u64,
    /// Perpetual-style coin identifier.
    pub coin: String,
    /// Spot-token identifier.
    pub token: String,
    /// Exchange action asset ID.
    pub action_asset_id: u64,
}

/// Typed metadata retained from an `outcomeMeta` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hip4OutcomeMeta {
    /// Outcome ID; its identity is scoped by the selected network.
    #[serde(alias = "id")]
    pub outcome: u64,
    /// Display name, excluded from binding identity decisions.
    pub name: Option<String>,
    /// Opaque display description, excluded from binding identity decisions.
    pub description: Option<String>,
    /// Optional question/group identifier.
    pub question_id: Option<String>,
    /// Ordered display side specifications. Order defines side indices.
    pub side_specs: Vec<Hip4SideSpec>,
    /// Outcome-specific quote token, when present.
    pub quote_token: Option<String>,
    /// Whether the retained metadata marks this outcome settled.
    #[serde(default)]
    pub settled: bool,
    /// Expected settlement authority, when explicitly available.
    pub settlement_authority: Option<String>,
    /// Display-only fields retained but excluded from binding identity decisions.
    #[serde(flatten)]
    pub display_extensions: BTreeMap<String, serde_json::Value>,
}

/// One ordered HIP-4 display side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hip4SideSpec {
    /// Display label; it is never assumed to be YES or NO.
    pub name: String,
    /// Display-only fields retained for diagnostics.
    #[serde(flatten)]
    pub display_extensions: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutcomeMetaEnvelope {
    outcomes: Vec<Hip4OutcomeMeta>,
    #[serde(default, rename = "questions")]
    _questions: Vec<serde_json::Value>,
}

/// Failures from checked HIP-4 parsing and mock settlement behavior.
#[derive(Debug, Error)]
pub enum Hip4Error {
    /// Side indices outside the documented binary contract are forbidden.
    #[error("HIP-4 binary side index must be 0 or 1")]
    InvalidSide,
    /// Checked ID arithmetic overflowed.
    #[error("HIP-4 asset ID arithmetic overflow")]
    AssetIdOverflow,
    /// An identifier has the wrong prefix or decimal body.
    #[error("invalid HIP-4 asset identifier")]
    InvalidAssetId,
    /// Metadata JSON did not match the recorded contract.
    #[error("invalid HIP-4 outcomeMeta payload: {0}")]
    InvalidMetadata(#[from] serde_json::Error),
    /// Exactly two ordered sides are required by this binary MVP.
    #[error("HIP-4 binary metadata must contain exactly two distinct sides")]
    InvalidMetadataSides,
    /// The documented list did not include the requested outcome ID.
    #[error("HIP-4 outcome ID not present in outcomeMeta response")]
    OutcomeNotFound,
    /// Mainnet mutation is structurally unavailable.
    #[error("mainnet mutation is structurally disabled")]
    MainnetMutationDisabled,
    /// Only the mock network accepts mock submissions.
    #[error("mock backend requires MOCK network")]
    WrongNetwork,
    /// Failure-injection interrupted submission.
    #[error("injected mock submission failure")]
    InjectedFailure,
    /// Intent has expired.
    #[error("settlement intent expired")]
    ExpiredIntent,
    /// Canonical digest construction failed.
    #[error("HIP-4 canonicalization failed: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Live read-only support was not compiled.
    #[error("live read-only HIP-4 support requires the live-readonly feature")]
    LiveReadOnlyDisabled,
    /// A bounded read-only request failed.
    #[cfg(feature = "live-readonly")]
    #[error("HIP-4 read-only request failed: {0}")]
    Http(#[from] reqwest::Error),
}

/// Compute all official binary identifiers using checked arithmetic.
///
/// # Errors
///
/// Returns [`Hip4Error::InvalidSide`] or [`Hip4Error::AssetIdOverflow`].
pub fn encode_asset_ids(outcome_id: u64, side_index: u8) -> Result<Hip4AssetIds, Hip4Error> {
    if side_index > 1 {
        return Err(Hip4Error::InvalidSide);
    }
    let encoding = outcome_id
        .checked_mul(10)
        .and_then(|value| value.checked_add(u64::from(side_index)))
        .ok_or(Hip4Error::AssetIdOverflow)?;
    let action_asset_id = ACTION_ASSET_OFFSET
        .checked_add(encoding)
        .ok_or(Hip4Error::AssetIdOverflow)?;
    Ok(Hip4AssetIds {
        outcome_id,
        side_index,
        encoding,
        coin: format!("#{encoding}"),
        token: format!("+{encoding}"),
        action_asset_id,
    })
}

/// Decode a documented coin or token identifier and verify its side.
///
/// # Errors
///
/// Returns an invalid-ID or invalid-side error.
pub fn decode_asset_id(value: &str) -> Result<Hip4AssetIds, Hip4Error> {
    let body = value
        .strip_prefix('#')
        .or_else(|| value.strip_prefix('+'))
        .ok_or(Hip4Error::InvalidAssetId)?;
    let encoding = body.parse::<u64>().map_err(|_| Hip4Error::InvalidAssetId)?;
    let side_index = u8::try_from(encoding % 10).map_err(|_| Hip4Error::InvalidSide)?;
    encode_asset_ids(encoding / 10, side_index)
}

/// Parse and retain exact recorded metadata bytes for explicit binding.
///
/// # Errors
///
/// Rejects malformed JSON or a non-binary/duplicate side list.
pub fn parse_outcome_meta(
    network: SettlementNetwork,
    raw: &[u8],
) -> Result<ExternalMarket, Hip4Error> {
    parse_selected_outcome_meta(network, raw, None)
}

fn parse_selected_outcome_meta(
    network: SettlementNetwork,
    raw: &[u8],
    selected_outcome_id: Option<u64>,
) -> Result<ExternalMarket, Hip4Error> {
    let envelope: OutcomeMetaEnvelope = serde_json::from_slice(raw)?;
    let metadata = match selected_outcome_id {
        Some(outcome_id) => envelope
            .outcomes
            .into_iter()
            .find(|metadata| metadata.outcome == outcome_id)
            .ok_or(Hip4Error::OutcomeNotFound)?,
        None => envelope
            .outcomes
            .into_iter()
            .next()
            .ok_or(Hip4Error::OutcomeNotFound)?,
    };
    if metadata.side_specs.len() != 2 || metadata.side_specs[0].name == metadata.side_specs[1].name
    {
        return Err(Hip4Error::InvalidMetadataSides);
    }
    let raw_payload_digest = raw_sha256(raw);
    let metadata_digest = domain_separated_digest(
        "hsai.hip4-outcome-meta.v1",
        &(
            network,
            metadata.outcome,
            &metadata.question_id,
            metadata
                .side_specs
                .iter()
                .map(|side| &side.name)
                .collect::<Vec<_>>(),
            &metadata.quote_token,
            metadata.settled,
            &metadata.settlement_authority,
        ),
    )?;
    let sides = metadata
        .side_specs
        .into_iter()
        .enumerate()
        .map(|(index, side)| {
            let side_index = u8::try_from(index).map_err(|_| Hip4Error::InvalidSide)?;
            Ok(ExternalSideV1 {
                side_index,
                side_id: encode_asset_ids(metadata.outcome, side_index)?.coin,
                display_name: side.name,
            })
        })
        .collect::<Result<Vec<_>, Hip4Error>>()?;
    Ok(ExternalMarket {
        backend: "hyperliquid-hip4".to_owned(),
        network,
        market_id: metadata.outcome.to_string(),
        question_id: metadata.question_id,
        sides,
        quote_token: metadata.quote_token,
        metadata_digest,
        raw_payload_digest,
        settlement_authority: metadata.settlement_authority,
        settled: metadata.settled,
    })
}

/// Dated capability status for the current verified external contract.
#[must_use]
pub fn capability_matrix(
    network: SettlementNetwork,
    verified_at: UtcTimestamp,
) -> CapabilityMatrix {
    let supported = match network {
        SettlementNetwork::Mock => [
            BackendCapability::ReadActiveMetadata,
            BackendCapability::ReadSettledOutcome,
            BackendCapability::MockSubmission,
        ]
        .into_iter()
        .collect(),
        SettlementNetwork::HyperliquidTestnet | SettlementNetwork::HyperliquidMainnetReadonly => {
            [BackendCapability::ReadActiveMetadata]
                .into_iter()
                .collect()
        }
    };
    CapabilityMatrix {
        backend: "hyperliquid-hip4".to_owned(),
        network,
        supported,
        blockers: if network == SettlementNetwork::Mock {
            Vec::new()
        } else {
            vec![
                "UNSUPPORTED_UNVERIFIED: no official create-market or resolution-publish action, signing domain, and authorized broadcaster role were documented on 2026-07-10".to_owned(),
                "settledOutcome was observed read-only but its typed response client is not implemented in this build".to_owned(),
            ]
        },
        verified_at,
    }
}

/// Deterministic mock HIP-4 backend used by all effectful tests.
#[derive(Debug, Default)]
pub struct Hip4MockBackend {
    receipts: HashMap<String, SubmissionReceiptV1>,
    intents: HashMap<String, UnsignedSettlementIntentV1>,
    fail_next_submission: bool,
}

impl Hip4MockBackend {
    /// Inject one failure before the mock effect is accepted.
    pub fn fail_next_submission(&mut self) {
        self.fail_next_submission = true;
    }

    /// Submit at most one mock effect for an idempotency key.
    ///
    /// # Errors
    ///
    /// Rejects non-mock/mainnet networks, expired intents, injected failures,
    /// or payloads that cannot be committed.
    pub fn submit(
        &mut self,
        intent: &UnsignedSettlementIntentV1,
        accepted_at: UtcTimestamp,
    ) -> Result<SubmissionReceiptV1, Hip4Error> {
        if intent.network == SettlementNetwork::HyperliquidMainnetReadonly {
            return Err(Hip4Error::MainnetMutationDisabled);
        }
        if intent.network != SettlementNetwork::Mock {
            return Err(Hip4Error::WrongNetwork);
        }
        if accepted_at > intent.expires_at {
            return Err(Hip4Error::ExpiredIntent);
        }
        if let Some(receipt) = self.receipts.get(&intent.idempotency_key) {
            return Ok(receipt.clone());
        }
        if self.fail_next_submission {
            self.fail_next_submission = false;
            return Err(Hip4Error::InjectedFailure);
        }
        let request_digest = domain_separated_digest("hsai.mock-hip4-request.v1", intent)?;
        let response_digest = domain_separated_digest(
            "hsai.mock-hip4-response.v1",
            &(&intent.idempotency_key, "CONFIRMED"),
        )?;
        let receipt = SubmissionReceiptV1 {
            schema_version: "hsai.submission-receipt.v1".to_owned(),
            intent_id: intent.intent_id.clone(),
            idempotency_key: intent.idempotency_key.clone(),
            backend_receipt_id: format!("mock_{}", &request_digest.to_hex()[..32]),
            accepted_at,
            request_digest,
            response_digest,
            external_sequence: Some("1".to_owned()),
        };
        self.intents
            .insert(intent.idempotency_key.clone(), intent.clone());
        self.receipts
            .insert(intent.idempotency_key.clone(), receipt.clone());
        Ok(receipt)
    }

    /// Reconcile authoritative mock state without requiring a persisted receipt.
    ///
    /// This models recovery after backend acceptance but before local receipt
    /// persistence and therefore prevents blind resubmission.
    ///
    /// # Errors
    ///
    /// Returns a canonicalization error if status evidence cannot be committed.
    pub fn reconcile(
        &self,
        binding: &MarketBindingV1,
        resolution: &ResolutionRecordV1,
        checked_at: UtcTimestamp,
    ) -> Result<ReconciliationResultV1, Hip4Error> {
        let found = self.intents.values().find(|intent| {
            intent.binding_digest == binding.binding_digest
                && intent.final_resolution_digest == resolution.record_digest
        });
        let (status, lifecycle, reason, receipt_digest) = if let Some(intent) = found {
            let receipt = self.receipts.get(&intent.idempotency_key);
            (
                ReconciliationStatus::Confirmed,
                SettlementLifecycle::Confirmed,
                "mock authoritative state matches final resolution",
                receipt
                    .map(|value| domain_separated_digest("hsai.submission-receipt.v1", value))
                    .transpose()?,
            )
        } else {
            (
                ReconciliationStatus::Unknown,
                SettlementLifecycle::ReconciliationFailed,
                "mock authoritative state has no matching effect",
                None,
            )
        };
        Ok(ReconciliationResultV1 {
            schema_version: "hsai.reconciliation-result.v1".to_owned(),
            binding_digest: binding.binding_digest,
            final_resolution_digest: resolution.record_digest,
            receipt_digest,
            external_status_digest: domain_separated_digest(
                "hsai.mock-hip4-status.v1",
                &(binding.binding_digest, resolution.record_digest, status),
            )?,
            status,
            settlement_lifecycle: lifecycle,
            checked_at,
            exact_reason: reason.to_owned(),
        })
    }

    /// Number of distinct accepted external effects.
    #[must_use]
    pub fn effect_count(&self) -> usize {
        self.receipts.len()
    }
}

/// Read-only client configuration. It never contains signing material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hip4ReadOnlyBackend {
    /// Selected network; mainnet is read-only by type.
    pub network: SettlementNetwork,
    /// Official Info endpoint URL.
    pub info_url: String,
}

impl Hip4ReadOnlyBackend {
    /// Parse a retained response without network access.
    ///
    /// # Errors
    ///
    /// Returns typed metadata validation failures.
    pub fn parse_recorded(&self, raw: &[u8]) -> Result<ExternalMarket, Hip4Error> {
        parse_outcome_meta(self.network, raw)
    }

    /// Perform a bounded official Info endpoint `outcomeMeta` read.
    ///
    /// # Errors
    ///
    /// Returns client construction, timeout, HTTP status, response-body, or
    /// typed HIP-4 metadata failures. This method never posts an exchange action.
    #[cfg(feature = "live-readonly")]
    pub fn discover(&self, outcome_id: u64) -> Result<ExternalMarket, Hip4Error> {
        use std::{thread, time::Duration};
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .build()?;
        let mut attempts = 0_u32;
        loop {
            attempts += 1;
            match client
                .post(&self.info_url)
                .json(&serde_json::json!({"type": "outcomeMeta"}))
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
            {
                Ok(response) => {
                    let raw = response.bytes()?;
                    return parse_selected_outcome_meta(self.network, &raw, Some(outcome_id));
                }
                Err(_) if attempts < 3 => {
                    thread::sleep(Duration::from_millis(u64::from(attempts) * 100));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Fail explicitly when live support was not selected at build time.
    ///
    /// # Errors
    ///
    /// Always returns [`Hip4Error::LiveReadOnlyDisabled`] in builds without
    /// the `live-readonly` feature.
    #[cfg(not(feature = "live-readonly"))]
    pub fn discover(&self, _outcome_id: u64) -> Result<ExternalMarket, Hip4Error> {
        Err(Hip4Error::LiveReadOnlyDisabled)
    }
}

fn raw_sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use outcomes_domain::{
        ClaimTruth, ResolutionFinality, SettlementNetwork, fixtures::local_demo_claim_draft,
    };
    use outcomes_settlement::{bind_market, build_intent, preflight};
    use proptest::prelude::*;

    use super::*;

    const META: &[u8] = include_bytes!("../../../fixtures/hip4/outcome-meta.json");
    type TestResult = Result<(), Box<dyn Error>>;

    fn time(value: &str) -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        value.parse()
    }

    fn resolution(
        claim_id: outcomes_domain::ClaimId,
    ) -> Result<ResolutionRecordV1, outcomes_domain::TimestampError> {
        Ok(ResolutionRecordV1 {
            schema_version: "hsai.resolution-record.v1".to_owned(),
            resolution_id: "resolution-test".to_owned(),
            claim_id,
            truth: ClaimTruth::True,
            finality: ResolutionFinality::Final,
            evidence_bundle_digest: Sha256Digest::from_bytes([8; 32]),
            attestation_digests: vec![Sha256Digest::from_bytes([9; 32])],
            dispute_window_start: time("2026-07-10T10:00:00Z")?,
            dispute_window_end: time("2026-07-10T11:00:00Z")?,
            record_digest: Sha256Digest::from_bytes([10; 32]),
        })
    }

    #[test]
    fn contract_metadata_preserves_order_labels_quote_and_raw_digest() -> TestResult {
        let market = parse_outcome_meta(SettlementNetwork::Mock, META)?;
        assert_eq!(market.market_id, "42");
        assert_eq!(market.sides[0].display_name, "Up");
        assert_eq!(market.sides[1].display_name, "Down");
        assert_eq!(market.quote_token.as_deref(), Some("USDC"));
        assert_eq!(market.raw_payload_digest, raw_sha256(META));
        Ok(())
    }

    #[test]
    fn contract_display_extensions_do_not_change_binding_metadata_digest() -> TestResult {
        let original = parse_outcome_meta(SettlementNetwork::Mock, META)?;
        let mut changed: serde_json::Value = serde_json::from_slice(META)?;
        changed["outcomes"][0]["description"] =
            serde_json::Value::String("Changed presentation only".to_owned());
        changed["outcomes"][0]["sideSpecs"][0]["displayColor"] =
            serde_json::Value::String("blue".to_owned());
        let changed_raw = serde_json::to_vec(&changed)?;
        let reparsed = parse_outcome_meta(SettlementNetwork::Mock, &changed_raw)?;
        assert_eq!(original.metadata_digest, reparsed.metadata_digest);
        assert_ne!(original.raw_payload_digest, reparsed.raw_payload_digest);
        Ok(())
    }

    #[test]
    fn contract_stale_binding_fails_preflight() -> TestResult {
        let draft = local_demo_claim_draft()?;
        let claim = draft.freeze()?;
        let market = parse_outcome_meta(SettlementNetwork::Mock, META)?;
        let caps = capability_matrix(SettlementNetwork::Mock, time("2026-07-10T10:00:00Z")?);
        let cap_digest = domain_separated_digest("hsai.backend-capability.v1", &caps)?;
        let binding = bind_market(
            &claim,
            &market,
            0,
            1,
            cap_digest,
            "operator-test",
            time("2026-07-10T10:00:00Z")?,
            "dev-signature".to_owned(),
        )?;
        let check = preflight(
            &binding,
            &resolution(claim.claim_id)?,
            Sha256Digest::from_bytes([99; 32]),
            &caps,
        );
        assert!(!check.ready);
        assert!(check.blockers.contains(&"STALE_BINDING".to_owned()));
        Ok(())
    }

    #[test]
    fn contract_mock_is_idempotent_and_recovers_after_receipt_loss() -> TestResult {
        let draft = local_demo_claim_draft()?;
        let claim = draft.freeze()?;
        let market = parse_outcome_meta(SettlementNetwork::Mock, META)?;
        let caps = capability_matrix(SettlementNetwork::Mock, time("2026-07-10T10:00:00Z")?);
        let binding = bind_market(
            &claim,
            &market,
            0,
            1,
            domain_separated_digest("hsai.backend-capability.v1", &caps)?,
            "operator-test",
            time("2026-07-10T10:00:00Z")?,
            "dev-signature".to_owned(),
        )?;
        let final_record = resolution(claim.claim_id)?;
        let check = preflight(&binding, &final_record, market.metadata_digest, &caps);
        let intent = build_intent(
            &binding,
            &final_record,
            &check,
            time("2026-07-11T10:00:00Z")?,
        )?;
        let mut backend = Hip4MockBackend::default();
        let first = backend.submit(&intent, time("2026-07-10T12:00:00Z")?)?;
        let second = backend.submit(&intent, time("2026-07-10T12:00:01Z")?)?;
        assert_eq!(first, second);
        assert_eq!(backend.effect_count(), 1);
        let recovered =
            backend.reconcile(&binding, &final_record, time("2026-07-10T12:00:02Z")?)?;
        assert_eq!(recovered.status, ReconciliationStatus::Confirmed);
        Ok(())
    }

    #[test]
    fn contract_mainnet_submit_is_unconditionally_refused() -> TestResult {
        let intent = UnsignedSettlementIntentV1 {
            schema_version: "hsai.unsigned-settlement-intent.v1".to_owned(),
            intent_id: "intent-mainnet".to_owned(),
            backend: "hyperliquid-hip4".to_owned(),
            network: SettlementNetwork::HyperliquidMainnetReadonly,
            binding_digest: Sha256Digest::from_bytes([1; 32]),
            final_resolution_digest: Sha256Digest::from_bytes([2; 32]),
            action_type: "ATTESTED_RESOLUTION".to_owned(),
            canonical_payload: "{}".to_owned(),
            payload_digest: Sha256Digest::from_bytes([3; 32]),
            idempotency_key: "settlement-mainnet".to_owned(),
            expires_at: time("2026-07-11T00:00:00Z")?,
        };
        assert!(matches!(
            Hip4MockBackend::default().submit(&intent, time("2026-07-10T12:00:00Z")?),
            Err(Hip4Error::MainnetMutationDisabled)
        ));
        Ok(())
    }

    proptest! {
        #[test]
        fn asset_ids_round_trip(outcome_id in 0_u64..9_000_000_000_000_000_u64, side in 0_u8..2) {
            let encoded = encode_asset_ids(outcome_id, side)
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(decode_asset_id(&encoded.coin).ok(), Some(encoded.clone()));
            prop_assert_eq!(decode_asset_id(&encoded.token).ok(), Some(encoded));
        }
    }

    #[test]
    fn asset_encoding_rejects_invalid_side_and_overflow() {
        assert!(matches!(
            encode_asset_ids(1, 2),
            Err(Hip4Error::InvalidSide)
        ));
        assert!(matches!(
            encode_asset_ids(u64::MAX, 0),
            Err(Hip4Error::AssetIdOverflow)
        ));
    }
}
