//! Contract tests for the hermetic route-to-settlement evidence slice.

use std::collections::{BTreeMap, BTreeSet};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceClaimV1, AssuranceEnvelopeV1, AssuranceEvidenceSetV1, AssuranceRootClassV1,
    AssuranceRootV1, EvidenceClassification, RegisteredVerificationKeyV1, SignatureError,
    SignedStatementV1, ValidityWindowV1, VerificationKeyRegistry, build_assurance_evidence_set_v1,
    evidence_blob_ref_v1,
};
use hyperintel_core::{
    AssuranceMaturity, AssuranceProperty, AtomicAmount, BasisPoints, CapacityOfferV1,
    InferenceCapability, InferenceRequestPolicyV1, MeteringMethod, PartsPerMillion, PrivacyMode,
    ProtocolId, ProviderIdentityV1, ProviderStatus, RetentionMode, ServiceSpecV1, Sha256Digest,
    UtcTimestamp,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    BuyerReplayStateV1, EligibilityFailureV1, ExecutionEvidenceError, ExecutionStatusV1,
    MarketEvidenceBundleV1, MeteringReceiptDraftV1, ProviderExecutionReceiptDraftV1,
    ProviderExecutionStatementV1, ReliabilityArtifactDraftV1, ReliabilityArtifactError,
    ReliabilityArtifactVerifiedMarketEvidenceBundleV3, ReliabilityArtifactVerifiedRoutingContextV1,
    ReliabilityVerifierStatementV1, ReplayError, ReplayProtectedMarketEvidenceBundleV4,
    RouteDecisionV1, RouteEvaluationOutcomeV1, RouteEvaluationRecordV1, RoutingContextV1,
    RoutingError, SignedCapacityOfferV1, SignedInferenceRequestPolicyV1,
    SignedMarketEvidenceBundleV2, SignedProviderExecutionReceiptV1, SignedReliabilityArtifactV1,
    SignedRoutingContextV1, build_authenticated_execution_admission_v1,
    build_buyer_replay_state_v1, build_market_evidence_bundle_v1,
    build_metering_from_verified_execution_receipt_v1, build_metering_receipt_v1,
    build_provider_execution_receipt_v1, build_reliability_artifact_v1,
    build_reliability_artifact_verified_market_evidence_bundle_v3,
    build_replay_protected_market_evidence_bundle_v4, build_signed_market_evidence_bundle_v2,
    derive_observed_success_probability_v1, derive_replay_state_transition_v1,
    derive_settlement_intent_v1, evaluate_reliability_artifact_verified_route_v1,
    evaluate_signed_route_v1, select_reliability_artifact_verified_route_v1, select_route_v1,
    select_signed_route_v1, validate_route_evaluation_against_offers_v1,
    validate_route_evaluation_record_v1, verify_authenticated_execution_admission_v1,
    verify_market_evidence_bundle_v1,
    verify_reliability_artifact_verified_market_evidence_bundle_v3,
    verify_replay_protected_market_evidence_bundle_v4, verify_replay_state_transition_v1,
    verify_signed_inference_request_policy_v1, verify_signed_market_evidence_bundle_v2,
    verify_signed_provider_execution_receipt_v1, verify_signed_reliability_artifact_v1,
};

struct Fixture {
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    offers: Vec<CapacityOfferV1>,
    signed_offers: Vec<SignedCapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    registry: VerificationKeyRegistry,
    verification_keys: Vec<RegisteredVerificationKeyV1>,
}

struct ReliabilityFixture {
    fixture: Fixture,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[test]
fn contract_vertical_slice_reconstructs_route_metering_settlement_and_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let route = route(&fixture)?;
    assert_eq!(route.selected_provider_id.as_str(), "provider:beta");
    assert!(!route.authority_granted);

    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering_draft = MeteringReceiptDraftV1 {
        started_at: timestamp("2026-07-20T12:00:01Z")?,
        first_token_at: timestamp("2026-07-20T12:00:02Z")?,
        completed_at: timestamp("2026-07-20T12:00:04Z")?,
        input_tokens: 900,
        output_tokens: 400,
        execution_status: ExecutionStatusV1::Completed,
        prompt_digest: fixture.policy.prompt_digest,
        output_digest: Sha256Digest::from_bytes([31; 32]),
        provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
        buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
    };
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        metering_draft.clone(),
    )?;
    assert!(metering.actual_provider_charge < route.selected_cost.provider_quote_max);
    assert!(!metering.authority_granted);

    assert_authorization_windows(
        &fixture,
        provider,
        offer,
        &route,
        &metering_draft,
        &metering,
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    assert_eq!(
        settlement
            .provider_payout_amount
            .checked_add(settlement.protocol_fee_amount)?
            .checked_add(settlement.buyer_refund_amount)?,
        settlement.escrow_amount
    );
    assert!(!settlement.authority_granted);

    let bundle = build_market_evidence_bundle_v1(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        fixture.offers.clone(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        vec![
            "provider authorization is evaluated only from supplied signed evidence".to_owned(),
            "success probability is an evidence-qualified declared input".to_owned(),
        ],
    )?;
    verify_market_evidence_bundle_v1(&bundle, &fixture.assurance_blobs, &fixture.registry)?;
    let mut undisclosed_keys = bundle.clone();
    undisclosed_keys.verification_keys.clear();
    assert!(
        verify_market_evidence_bundle_v1(
            &undisclosed_keys,
            &fixture.assurance_blobs,
            &fixture.registry
        )
        .is_err()
    );
    assert!(
        verify_market_evidence_bundle_v1(
            &bundle,
            &fixture.assurance_blobs,
            &VerificationKeyRegistry::default()
        )
        .is_err()
    );

    let encoded = serde_json::to_vec(&bundle)?;
    let decoded: MarketEvidenceBundleV1 = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, bundle);
    Ok(())
}

#[test]
fn contract_signed_offer_v2_reconstructs_and_has_a_stable_golden_digest()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let bundle = signed_bundle(&fixture)?;
    verify_signed_market_evidence_bundle_v2(&bundle, &fixture.assurance_blobs, &fixture.registry)?;
    assert!(
        !bundle
            .explicit_nonclaims
            .iter()
            .any(|claim| claim == "no-capacity-offer-signature-verification")
    );
    assert_eq!(
        bundle.bundle_digest.to_hex(),
        "781fab0921e649922f53ceb67f9037ddfd0f039922d3663ef54b29f7bd042cab"
    );
    let encoded = serde_json::to_vec(&bundle)?;
    let decoded: SignedMarketEvidenceBundleV2 = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, bundle);

    let mut reversed = fixture.signed_offers.clone();
    reversed.reverse();
    assert_eq!(
        signed_route(&fixture)?,
        signed_route_with(&fixture, &reversed, &fixture.registry)?
    );
    Ok(())
}

#[test]
fn contract_reliability_v3_reconstructs_and_has_stable_golden_digests()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = reliability_fixture()?;
    let bundle = reliability_bundle(&fixture)?;
    verify_reliability_artifact_verified_market_evidence_bundle_v3(
        &bundle,
        &fixture.fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &fixture.fixture.registry,
    )?;
    assert_eq!(
        bundle.route.routing_algorithm,
        "expected-successful-cost-reliability-artifact-v1"
    );
    assert_eq!(bundle.route.selected_provider_id.as_str(), "provider:beta");
    assert!(
        !bundle
            .explicit_nonclaims
            .iter()
            .any(|claim| claim == "no-reliability-evidence-verification")
    );
    assert!(bundle.explicit_nonclaims.iter().any(|claim| {
        claim == "no-observation-source-verification-or-future-reliability-accuracy"
    }));
    assert_eq!(
        fixture
            .reliability_artifacts
            .values()
            .find(|artifact| artifact.artifact.provider_id.as_str() == "provider:beta")
            .ok_or("beta reliability artifact missing")?
            .artifact
            .artifact_digest
            .to_hex(),
        "285feddb9407af039f26eac03d2eb6246a1b54dcf4eacc6f52fc06e6dbbc92f4"
    );
    assert_eq!(
        bundle.bundle_digest.to_hex(),
        "59db4b80a9fec85a2fa8e72d7e6cb36d446918c8a85a896883be7d83d50c8fbf"
    );

    let encoded = serde_json::to_vec(&bundle)?;
    let decoded: ReliabilityArtifactVerifiedMarketEvidenceBundleV3 =
        serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, bundle);

    let mut reversed_offers = fixture.fixture.signed_offers.clone();
    reversed_offers.reverse();
    assert_eq!(
        reliability_route_with(
            &fixture.fixture,
            &reversed_offers,
            &fixture.reliability_artifacts,
        )?,
        reliability_route(&fixture)?,
    );
    Ok(())
}

#[test]
fn contract_v1_v2_v3_rejection_fallback_bundle_semantics_remain_frozen()
-> Result<(), Box<dyn std::error::Error>> {
    let mut v1_v2_fixture = fixture()?;
    remove_provider_assurance(&mut v1_v2_fixture, "provider:beta")?;

    let v1_bundle = market_bundle(&v1_v2_fixture)?;
    assert_eq!(
        v1_bundle.route.selected_provider_id.as_str(),
        "provider:alpha"
    );
    assert_eq!(
        rejected_candidate_report_digest(&v1_bundle.route, "provider:beta")?.to_hex(),
        "0831c3c91843faddc790737dac16aa8f13d00144faf566d6d88cc09e65819017"
    );
    assert_eq!(
        v1_bundle.bundle_digest.to_hex(),
        "663b688e151535a74a9c370263b7536082f9e4a6a5bb542cd19103b8c303fa39"
    );
    verify_market_evidence_bundle_v1(
        &v1_bundle,
        &v1_v2_fixture.assurance_blobs,
        &v1_v2_fixture.registry,
    )?;

    let v2_bundle = signed_bundle(&v1_v2_fixture)?;
    assert_eq!(
        v2_bundle.route.selected_provider_id.as_str(),
        "provider:alpha"
    );
    assert_eq!(
        rejected_candidate_report_digest(&v2_bundle.route, "provider:beta")?.to_hex(),
        "d3ddb7762b360d5d704ded597daace907f12313f999e820b41b613f4945a9734"
    );
    assert_eq!(
        v2_bundle.bundle_digest.to_hex(),
        "c4d146a84758ca790e71cca8bd3d3a60f75d93c9dd6905c932dce1af1ce2719d"
    );
    verify_signed_market_evidence_bundle_v2(
        &v2_bundle,
        &v1_v2_fixture.assurance_blobs,
        &v1_v2_fixture.registry,
    )?;

    let mut v3_fixture = reliability_fixture()?;
    remove_provider_assurance(&mut v3_fixture.fixture, "provider:beta")?;
    let v3_bundle = reliability_bundle(&v3_fixture)?;
    assert_eq!(
        v3_bundle.route.selected_provider_id.as_str(),
        "provider:alpha"
    );
    assert_eq!(
        rejected_candidate_report_digest(&v3_bundle.route, "provider:beta")?.to_hex(),
        "06eecda1c2e8a2874e197d0b7b9504aac2f63ec45c6749c5a10548c03e8d5c62"
    );
    assert_eq!(
        v3_bundle.bundle_digest.to_hex(),
        "6d0a49cf0b10211ac9a54020b43d223f7bfa4af8b89871795157f47ee1282919"
    );
    verify_reliability_artifact_verified_market_evidence_bundle_v3(
        &v3_bundle,
        &v3_fixture.fixture.assurance_blobs,
        &v3_fixture.reliability_artifacts,
        &v3_fixture.fixture.registry,
    )?;
    Ok(())
}

#[test]
fn contract_replay_protected_v4_reconstructs_selected_route_and_nonce_transition()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, before_state) = replay_protected_fixture()?;
    let bundle = replay_protected_bundle(&fixture, &before_state)?;
    verify_replay_protected_market_evidence_bundle_v4(
        &bundle,
        &fixture.fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &fixture.fixture.registry,
    )?;
    let RouteEvaluationOutcomeV1::Selected(route) = &bundle.route_evaluation.outcome else {
        return Err("selected V4 fixture produced a denial".into());
    };
    assert_eq!(route.selected_provider_id.as_str(), "provider:beta");
    assert_eq!(bundle.replay_transition.before_state, before_state);
    assert_eq!(bundle.replay_transition.consumed_nonce, 1);
    assert_eq!(bundle.replay_transition.after_state.next_nonce, 2);
    assert!(!bundle.replay_transition.authority_granted);
    assert!(bundle.metering.is_some());
    assert!(bundle.settlement.is_some());
    assert_eq!(
        bundle.route_evaluation.evaluation_digest.to_hex(),
        "7a56c7d39c5d31c062abe8d113bbda1e7f7da01f500712da473ffdb451116e38"
    );
    assert_eq!(
        bundle.replay_transition.transition_digest.to_hex(),
        "3e930e8b696a348207dc2662959b6471afbed1d4327eb9f13e735dc7e145f6c9"
    );
    assert_eq!(
        bundle.bundle_digest.to_hex(),
        "05397586672fb65f764812cf45176653caabd7a316968f268d5111a4a6a2daa8"
    );
    assert!(bundle.explicit_nonclaims.iter().any(|claim| {
        claim == "no-buyer-policy-authentication-or-durable-atomic-replay-state-enforcement"
    }));
    assert!(
        !bundle
            .explicit_nonclaims
            .iter()
            .any(|claim| claim == "no-consumed-nonce-or-replay-state-enforcement")
    );

    let encoded = serde_json::to_vec(&bundle)?;
    let decoded: ReplayProtectedMarketEvidenceBundleV4 = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, bundle);

    let mut reversed_offers = fixture.fixture.signed_offers.clone();
    reversed_offers.reverse();
    let rebuilt = evaluate_reliability_artifact_verified_route_v1(
        &ReliabilityArtifactVerifiedRoutingContextV1 {
            evaluated_at: &fixture.fixture.evaluated_at,
            service: &fixture.fixture.service,
            providers: &fixture.fixture.providers,
            offers: &reversed_offers,
            policy: &fixture.fixture.policy,
            assurance_sets: &fixture.fixture.assurance_sets,
            assurance_blobs: &fixture.fixture.assurance_blobs,
            reliability_artifacts: &fixture.reliability_artifacts,
            verification_keys: &fixture.fixture.registry,
        },
    )?;
    assert_eq!(rebuilt, bundle.route_evaluation);
    Ok(())
}

#[test]
fn contract_authenticated_execution_receipt_reconstructs_metering_without_authority()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, before_state) = replay_protected_fixture()?;
    let evaluation = reliability_evaluation(&fixture)?;
    let transition = derive_replay_state_transition_v1(
        &before_state,
        &fixture.fixture.policy,
        evaluation.evaluation_digest,
        &fixture.fixture.evaluated_at,
    )?;
    let buyer_signing_key = SigningKey::from_bytes(&[80; 32]);
    let receipt_signing_key = SigningKey::from_bytes(&[81; 32]);
    let signed_policy = signed_buyer_policy(&fixture.fixture.policy, &buyer_signing_key)?;
    let registry = execution_registry(
        &fixture.fixture.verification_keys,
        &buyer_signing_key,
        &receipt_signing_key,
    )?;
    let routing = ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &fixture.fixture.evaluated_at,
        service: &fixture.fixture.service,
        providers: &fixture.fixture.providers,
        offers: &fixture.fixture.signed_offers,
        policy: &fixture.fixture.policy,
        assurance_sets: &fixture.fixture.assurance_sets,
        assurance_blobs: &fixture.fixture.assurance_blobs,
        reliability_artifacts: &fixture.reliability_artifacts,
        verification_keys: &registry,
    };
    let context = AuthenticatedExecutionContextV1 {
        routing,
        replay_transition: &transition,
        signed_policy: &signed_policy,
    };
    let admission = build_authenticated_execution_admission_v1(
        &context,
        &registry,
        timestamp("2026-07-20T12:00:00Z")?,
        timestamp("2026-07-20T12:05:00Z")?,
    )?;
    verify_authenticated_execution_admission_v1(&admission, &context, &registry)?;
    assert!(!admission.authority_granted);
    assert_eq!(
        admission.admission_digest.to_hex(),
        "a77397ce31098f5e38474baf344e933e4b976c6f5556558a0d08f1b9cde57117"
    );

    let receipt = build_provider_execution_receipt_v1(
        &admission,
        &context.routing,
        ProviderExecutionReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            runtime_image_digest: fixture.fixture.service.runtime_image_digest,
        },
    )?;
    let statement =
        signed_provider_execution_statement(&receipt, &admission, &receipt_signing_key, 1)?;
    let signed_receipt = SignedProviderExecutionReceiptV1 {
        schema_version: "hyperintel.signed-provider-execution-receipt.v1".to_owned(),
        receipt,
        statement,
    };
    verify_signed_provider_execution_receipt_v1(&signed_receipt, &admission, &context, &registry)?;
    assert!(!signed_receipt.receipt.authority_granted);
    assert_eq!(
        signed_receipt.receipt.receipt_digest.to_hex(),
        "5c8cc50fcca0c0aea007d43a76855d5ea592b1b22e62f7cba42bc8eb04487e67"
    );

    assert_authenticated_execution_outputs(
        &fixture,
        &evaluation,
        &context,
        &registry,
        &admission,
        &signed_receipt,
    )?;
    Ok(())
}

fn assert_authenticated_execution_outputs(
    fixture: &ReliabilityFixture,
    evaluation: &RouteEvaluationRecordV1,
    context: &AuthenticatedExecutionContextV1<'_>,
    registry: &VerificationKeyRegistry,
    admission: &AuthenticatedExecutionAdmissionV1,
    signed_receipt: &SignedProviderExecutionReceiptV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let metering = build_metering_from_verified_execution_receipt_v1(
        signed_receipt,
        admission,
        context,
        registry,
        Some(Sha256Digest::from_bytes([33; 32])),
    )?;
    assert_eq!(
        metering.provider_metering_evidence_digest,
        signed_receipt.receipt.receipt_digest
    );
    let RouteEvaluationOutcomeV1::Selected(route) = &evaluation.outcome else {
        return Err("selected execution fixture produced a denial".into());
    };
    let provider = fixture
        .fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected execution provider missing")?;
    let offer = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected execution offer missing")?;
    let settlement = derive_settlement_intent_v1(
        &fixture.fixture.service,
        provider,
        offer,
        &fixture.fixture.policy,
        route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    assert_eq!(
        settlement
            .provider_payout_amount
            .checked_add(settlement.protocol_fee_amount)?
            .checked_add(settlement.buyer_refund_amount)?,
        settlement.escrow_amount
    );

    let mut tampered = signed_receipt.clone();
    tampered.receipt.output_digest = Sha256Digest::from_bytes([99; 32]);
    assert!(
        verify_signed_provider_execution_receipt_v1(&tampered, admission, context, registry,)
            .is_err()
    );

    let mut invalid_receipt_signature = signed_receipt.clone();
    let invalid_prefix = if invalid_receipt_signature
        .statement
        .signature_hex
        .starts_with("00")
    {
        "01"
    } else {
        "00"
    };
    invalid_receipt_signature
        .statement
        .signature_hex
        .replace_range(0..2, invalid_prefix);
    assert!(
        verify_signed_provider_execution_receipt_v1(
            &invalid_receipt_signature,
            admission,
            context,
            registry,
        )
        .is_err()
    );

    let mut invalid_policy = context.signed_policy.clone();
    let invalid_prefix = if invalid_policy.statement.signature_hex.starts_with("00") {
        "01"
    } else {
        "00"
    };
    invalid_policy
        .statement
        .signature_hex
        .replace_range(0..2, invalid_prefix);
    assert!(
        verify_signed_inference_request_policy_v1(
            &invalid_policy,
            registry,
            &fixture.fixture.evaluated_at,
        )
        .is_err()
    );

    let encoded = serde_json::to_vec(&signed_receipt)?;
    let decoded: SignedProviderExecutionReceiptV1 = serde_json::from_slice(&encoded)?;
    assert_eq!(&decoded, signed_receipt);
    Ok(())
}

#[test]
fn contract_authenticated_execution_admission_rejects_an_all_denied_route()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut fixture, before_state) = replay_protected_fixture()?;
    fixture.fixture.policy.minimum_success_probability = PartsPerMillion::new(1_000_000)?;
    let evaluation = reliability_evaluation(&fixture)?;
    assert!(matches!(
        evaluation.outcome,
        RouteEvaluationOutcomeV1::NoEligibleOffer
    ));
    let transition = derive_replay_state_transition_v1(
        &before_state,
        &fixture.fixture.policy,
        evaluation.evaluation_digest,
        &fixture.fixture.evaluated_at,
    )?;
    let buyer_signing_key = SigningKey::from_bytes(&[80; 32]);
    let receipt_signing_key = SigningKey::from_bytes(&[81; 32]);
    let signed_policy = signed_buyer_policy(&fixture.fixture.policy, &buyer_signing_key)?;
    let registry = execution_registry(
        &fixture.fixture.verification_keys,
        &buyer_signing_key,
        &receipt_signing_key,
    )?;
    let context = AuthenticatedExecutionContextV1 {
        routing: ReliabilityArtifactVerifiedRoutingContextV1 {
            evaluated_at: &fixture.fixture.evaluated_at,
            service: &fixture.fixture.service,
            providers: &fixture.fixture.providers,
            offers: &fixture.fixture.signed_offers,
            policy: &fixture.fixture.policy,
            assurance_sets: &fixture.fixture.assurance_sets,
            assurance_blobs: &fixture.fixture.assurance_blobs,
            reliability_artifacts: &fixture.reliability_artifacts,
            verification_keys: &registry,
        },
        replay_transition: &transition,
        signed_policy: &signed_policy,
    };
    assert!(matches!(
        build_authenticated_execution_admission_v1(
            &context,
            &registry,
            timestamp("2026-07-20T12:00:00Z")?,
            timestamp("2026-07-20T12:05:00Z")?,
        ),
        Err(ExecutionEvidenceError::NoSelectedRoute)
    ));
    Ok(())
}

#[test]
fn contract_replay_protected_v4_retains_all_typed_rejections_without_fill_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut fixture, before_state) = replay_protected_fixture()?;
    fixture.fixture.policy.minimum_success_probability = PartsPerMillion::new(1_000_000)?;
    let bundle = replay_protected_bundle(&fixture, &before_state)?;
    verify_replay_protected_market_evidence_bundle_v4(
        &bundle,
        &fixture.fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &fixture.fixture.registry,
    )?;
    assert!(matches!(
        bundle.route_evaluation.outcome,
        RouteEvaluationOutcomeV1::NoEligibleOffer
    ));
    assert_eq!(bundle.route_evaluation.eligibility_reports.len(), 2);
    assert!(bundle
        .route_evaluation
        .eligibility_reports
        .iter()
        .all(|report| report.failures == [EligibilityFailureV1::SuccessProbabilityBelowMinimum]));
    assert!(bundle.metering.is_none());
    assert!(bundle.settlement.is_none());
    assert_eq!(bundle.replay_transition.after_state.next_nonce, 2);

    let mut reordered = bundle.clone();
    reordered.route_evaluation.eligibility_reports.reverse();
    assert!(validate_route_evaluation_record_v1(&reordered.route_evaluation).is_err());

    let mut fabricated_fill = bundle;
    let (selected_fixture, selected_state) = replay_protected_fixture()?;
    fabricated_fill.metering =
        replay_protected_bundle(&selected_fixture, &selected_state)?.metering;
    assert!(
        verify_replay_protected_market_evidence_bundle_v4(
            &fabricated_fill,
            &fixture.fixture.assurance_blobs,
            &fixture.reliability_artifacts,
            &fixture.fixture.registry,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn contract_replay_transition_fails_closed_on_stale_gap_exhausted_or_tampered_state()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, before_state) = replay_protected_fixture()?;
    let evaluation = reliability_evaluation(&fixture)?;
    let transition = derive_replay_state_transition_v1(
        &before_state,
        &fixture.fixture.policy,
        evaluation.evaluation_digest,
        &fixture.fixture.evaluated_at,
    )?;
    verify_replay_state_transition_v1(
        &transition,
        &fixture.fixture.policy,
        evaluation.evaluation_digest,
        &fixture.fixture.evaluated_at,
    )?;

    let mut stale_policy = fixture.fixture.policy.clone();
    stale_policy.replay_snapshot_digest = transition.after_state.state_digest;
    assert!(matches!(
        derive_replay_state_transition_v1(
            &transition.after_state,
            &stale_policy,
            evaluation.evaluation_digest,
            &fixture.fixture.evaluated_at,
        ),
        Err(ReplayError::NonceAlreadyConsumed)
    ));

    let gap_state = build_buyer_replay_state_v1(stale_policy.buyer_id.clone(), 0)?;
    let mut gap_policy = stale_policy.clone();
    gap_policy.replay_snapshot_digest = gap_state.state_digest;
    assert!(matches!(
        derive_replay_state_transition_v1(
            &gap_state,
            &gap_policy,
            evaluation.evaluation_digest,
            &fixture.fixture.evaluated_at,
        ),
        Err(ReplayError::NonceGap)
    ));

    let exhausted_state = build_buyer_replay_state_v1(stale_policy.buyer_id.clone(), u64::MAX)?;
    let mut exhausted_policy = stale_policy;
    exhausted_policy.nonce = u64::MAX;
    exhausted_policy.replay_snapshot_digest = exhausted_state.state_digest;
    assert!(matches!(
        derive_replay_state_transition_v1(
            &exhausted_state,
            &exhausted_policy,
            evaluation.evaluation_digest,
            &fixture.fixture.evaluated_at,
        ),
        Err(ReplayError::NonceExhausted)
    ));

    let mut tampered = transition;
    tampered.after_state.next_nonce = 3;
    assert!(matches!(
        verify_replay_state_transition_v1(
            &tampered,
            &fixture.fixture.policy,
            evaluation.evaluation_digest,
            &fixture.fixture.evaluated_at,
        ),
        Err(ReplayError::TransitionDigestMismatch)
    ));
    Ok(())
}

#[test]
fn contract_cli_v1_through_v4_fixtures_match_canonical_bundles()
-> Result<(), Box<dyn std::error::Error>> {
    let (replay_fixture, before_state) = replay_protected_fixture()?;
    let bundle = replay_protected_bundle(&replay_fixture, &before_state)?;
    let evidence = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "bundle": bundle,
            "assurance_blobs": replay_fixture.fixture.assurance_blobs,
            "reliability_artifacts": replay_fixture.reliability_artifacts,
        }))?
    );
    let trust = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "trusted_verification_keys": replay_fixture.fixture.verification_keys,
        }))?
    );
    assert_eq!(
        evidence.as_bytes(),
        include_bytes!("../../hyperintel-cli/tests/fixtures/evidence-v4.json")
    );
    assert_eq!(
        trust.as_bytes(),
        include_bytes!("../../hyperintel-cli/tests/fixtures/trusted-keys-v4.json")
    );
    let legacy_fixture = fixture()?;
    let v1_evidence = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "bundle": market_bundle(&legacy_fixture)?,
            "assurance_blobs": legacy_fixture.assurance_blobs,
        }))?
    );
    let v2_evidence = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "bundle": signed_bundle(&legacy_fixture)?,
            "assurance_blobs": legacy_fixture.assurance_blobs,
        }))?
    );
    let v1_v2_trust = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "trusted_verification_keys": legacy_fixture.verification_keys,
        }))?
    );
    let reliability_fixture = reliability_fixture()?;
    let v3_evidence = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "bundle": reliability_bundle(&reliability_fixture)?,
            "assurance_blobs": reliability_fixture.fixture.assurance_blobs,
            "reliability_artifacts": reliability_fixture.reliability_artifacts,
        }))?
    );
    assert_eq!(
        v1_evidence.as_bytes(),
        include_bytes!("../../hyperintel-cli/tests/fixtures/evidence-v1.json")
    );
    assert_eq!(
        v2_evidence.as_bytes(),
        include_bytes!("../../hyperintel-cli/tests/fixtures/evidence-v2.json")
    );
    assert_eq!(
        v3_evidence.as_bytes(),
        include_bytes!("../../hyperintel-cli/tests/fixtures/evidence-v3.json")
    );
    assert_eq!(
        v1_v2_trust.as_bytes(),
        include_bytes!("../../hyperintel-cli/tests/fixtures/trusted-keys-v1-v2.json")
    );
    Ok(())
}

#[test]
fn contract_v4_builder_rejects_a_self_digested_suboptimal_route()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, before_state) = replay_protected_fixture()?;
    let mut evaluation = reliability_evaluation(&fixture)?;
    let RouteEvaluationOutcomeV1::Selected(selected) = &evaluation.outcome else {
        return Err("selected V4 fixture produced a denial".into());
    };
    assert_eq!(selected.selected_provider_id.as_str(), "provider:beta");

    let alpha_offer = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.provider_id.as_str() == "provider:alpha")
        .ok_or("alpha offer missing")?;
    let alpha_offer_digest = alpha_offer.digest()?;
    let alpha_provider = fixture
        .fixture
        .providers
        .get(&alpha_offer.provider_id)
        .ok_or("alpha provider missing")?;
    let alpha_report = evaluation
        .eligibility_reports
        .iter()
        .find(|report| report.offer_digest == alpha_offer_digest)
        .ok_or("alpha report missing")?;
    let alpha_cost = alpha_report.cost.ok_or("alpha cost missing")?;

    let mut suboptimal = (**selected).clone();
    suboptimal.selected_provider_id = alpha_offer.provider_id.clone();
    suboptimal.selected_offer_digest = alpha_offer_digest;
    suboptimal.selected_provider_identity_digest = alpha_provider.digest()?;
    suboptimal.selected_cost = alpha_cost;
    suboptimal.route_digest = suboptimal.digest()?;
    evaluation.outcome = RouteEvaluationOutcomeV1::Selected(Box::new(suboptimal));
    evaluation.evaluation_digest = evaluation.digest()?;

    validate_route_evaluation_record_v1(&evaluation)?;
    assert!(
        validate_route_evaluation_against_offers_v1(&evaluation, &fixture.fixture.offers).is_err()
    );
    let replay_transition = derive_replay_state_transition_v1(
        &before_state,
        &fixture.fixture.policy,
        evaluation.evaluation_digest,
        &fixture.fixture.evaluated_at,
    )?;
    assert!(
        build_replay_protected_market_evidence_bundle_v4(
            fixture.fixture.evaluated_at.clone(),
            fixture.fixture.service.clone(),
            fixture.fixture.providers.clone(),
            fixture.fixture.signed_offers.clone(),
            fixture.fixture.policy.clone(),
            fixture.fixture.assurance_sets.clone(),
            fixture.fixture.verification_keys.clone(),
            evaluation,
            replay_transition,
            None,
            None,
            Vec::new(),
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn contract_route_reports_distinguish_assurance_evidence_policy_and_signature_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;

    let no_blobs = BTreeMap::new();
    let missing_evidence = evaluate_signed_route_v1(&SignedRoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers: &fixture.signed_offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &no_blobs,
        verification_keys: &fixture.registry,
    })?;
    assert!(missing_evidence.eligibility_reports.iter().all(|report| {
        report
            .failures
            .contains(&EligibilityFailureV1::AssuranceEvidenceInvalid)
            && !report
                .failures
                .contains(&EligibilityFailureV1::AssurancePolicyRejected)
            && !report
                .failures
                .contains(&EligibilityFailureV1::OfferSignatureInvalid)
    }));

    let mut stronger_policy = fixture.policy.clone();
    stronger_policy.minimum_assurance_maturity = AssuranceMaturity::LocallyChecked;
    let policy_rejected = evaluate_signed_route_v1(&SignedRoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers: &fixture.signed_offers,
        policy: &stronger_policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        verification_keys: &fixture.registry,
    })?;
    assert!(policy_rejected.eligibility_reports.iter().all(|report| {
        report
            .failures
            .contains(&EligibilityFailureV1::AssurancePolicyRejected)
            && !report
                .failures
                .contains(&EligibilityFailureV1::AssuranceEvidenceInvalid)
            && !report
                .failures
                .contains(&EligibilityFailureV1::OfferSignatureInvalid)
    }));

    let mut invalid_signature_offers = fixture.signed_offers.clone();
    let invalid = invalid_signature_offers
        .iter_mut()
        .find(|signed| signed.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;
    let replacement = if invalid.statement.signature_hex.starts_with("00") {
        "01"
    } else {
        "00"
    };
    invalid
        .statement
        .signature_hex
        .replace_range(0..2, replacement);
    let invalid_signature = evaluate_signed_route_v1(&SignedRoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers: &invalid_signature_offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        verification_keys: &fixture.registry,
    })?;
    let beta_report = invalid_signature
        .eligibility_reports
        .iter()
        .find(|report| {
            fixture.signed_offers.iter().any(|signed| {
                signed.offer.provider_id.as_str() == "provider:beta"
                    && signed.offer.digest().ok() == Some(report.offer_digest)
            })
        })
        .ok_or("beta eligibility report missing")?;
    assert_eq!(
        beta_report.failures,
        [EligibilityFailureV1::OfferSignatureInvalid]
    );
    Ok(())
}

#[test]
fn contract_invalid_reliability_artifact_cannot_win_or_poison_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = reliability_fixture()?;
    let beta_digest = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.provider_id.as_str() == "provider:beta")
        .map(|offer| offer.reliability_evidence_digest)
        .ok_or("beta offer missing")?;

    let mut missing = fixture.reliability_artifacts.clone();
    missing.remove(&beta_digest);
    let fallback =
        reliability_route_with(&fixture.fixture, &fixture.fixture.signed_offers, &missing)?;
    assert_eq!(fallback.selected_provider_id.as_str(), "provider:alpha");
    assert!(
        fallback
            .candidates
            .iter()
            .find(|candidate| candidate.provider_id.as_str() == "provider:beta")
            .is_some_and(|candidate| !candidate.eligible)
    );

    let mut tamper_cases = Vec::new();
    let mut count_tamper = fixture.reliability_artifacts.clone();
    count_tamper
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?
        .artifact
        .successful_observations = 98;
    tamper_cases.push(count_tamper);

    let mut source_tamper = fixture.reliability_artifacts.clone();
    source_tamper
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?
        .artifact
        .source_observation_digest = Sha256Digest::from_bytes([0; 32]);
    tamper_cases.push(source_tamper);

    let mut signature_tamper = fixture.reliability_artifacts.clone();
    let signature = &mut signature_tamper
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?
        .statement
        .signature_hex;
    let replacement = if signature.starts_with("00") {
        "01"
    } else {
        "00"
    };
    signature.replace_range(0..2, replacement);
    tamper_cases.push(signature_tamper);

    let mut stale = fixture.reliability_artifacts.clone();
    stale
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?
        .artifact
        .valid_until = timestamp("2026-07-20T11:59:59Z")?;
    tamper_cases.push(stale);

    for artifacts in tamper_cases {
        let route =
            reliability_route_with(&fixture.fixture, &fixture.fixture.signed_offers, &artifacts)?;
        assert_eq!(route.selected_provider_id.as_str(), "provider:alpha");
    }

    let mut invalid_signed_offers = fixture.fixture.signed_offers.clone();
    let beta_signed = invalid_signed_offers
        .iter_mut()
        .find(|offer| offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;
    let replacement = if beta_signed.statement.signature_hex.starts_with("00") {
        "01"
    } else {
        "00"
    };
    beta_signed
        .statement
        .signature_hex
        .replace_range(0..2, replacement);
    let invalid_offer_fallback = reliability_route_with(
        &fixture.fixture,
        &invalid_signed_offers,
        &fixture.reliability_artifacts,
    )?;
    assert_eq!(
        invalid_offer_fallback.selected_provider_id.as_str(),
        "provider:alpha"
    );

    assert!(matches!(
        select_reliability_artifact_verified_route_v1(
            &ReliabilityArtifactVerifiedRoutingContextV1 {
                evaluated_at: &fixture.fixture.evaluated_at,
                service: &fixture.fixture.service,
                providers: &fixture.fixture.providers,
                offers: &fixture.fixture.signed_offers,
                policy: &fixture.fixture.policy,
                assurance_sets: &fixture.fixture.assurance_sets,
                assurance_blobs: &fixture.fixture.assurance_blobs,
                reliability_artifacts: &BTreeMap::new(),
                verification_keys: &fixture.fixture.registry,
            },
        ),
        Err(RoutingError::NoEligibleOffer)
    ));
    Ok(())
}

#[test]
fn contract_reliability_probability_uses_exact_floor_arithmetic()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(derive_observed_success_probability_v1(1, 3)?.get(), 333_333);
    assert_eq!(derive_observed_success_probability_v1(2, 3)?.get(), 666_666);
    assert_eq!(
        derive_observed_success_probability_v1(u64::MAX, u64::MAX)?.get(),
        1_000_000
    );
    assert_eq!(
        derive_observed_success_probability_v1(u64::MAX - 1, u64::MAX)?.get(),
        999_999
    );
    assert!(derive_observed_success_probability_v1(0, 0).is_err());
    assert!(derive_observed_success_probability_v1(2, 1).is_err());
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn contract_reliability_artifact_rejects_validly_resigned_semantic_mismatches()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = reliability_fixture()?;
    let beta_offer = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta offer missing")?
        .clone();
    let beta_artifact = fixture
        .reliability_artifacts
        .get(&beta_offer.reliability_evidence_digest)
        .ok_or("beta reliability artifact missing")?
        .clone();
    let signing_key = SigningKey::from_bytes(&[70; 32]);

    let mut wrong_provider_offer = beta_offer.clone();
    let mut wrong_provider = beta_artifact.clone();
    wrong_provider.artifact.provider_id = "provider:alpha".parse()?;
    refresh_reliability_artifact_signature(
        &mut wrong_provider,
        &mut wrong_provider_offer,
        &signing_key,
    )?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &wrong_provider,
            wrong_provider.artifact.artifact_digest,
            &wrong_provider_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::ProviderBindingMismatch)
    ));

    let mut wrong_service_offer = beta_offer.clone();
    let mut wrong_service = beta_artifact.clone();
    wrong_service.artifact.service_spec_digest = Sha256Digest::from_bytes([98; 32]);
    refresh_reliability_artifact_signature(
        &mut wrong_service,
        &mut wrong_service_offer,
        &signing_key,
    )?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &wrong_service,
            wrong_service.artifact.artifact_digest,
            &wrong_service_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::ServiceBindingMismatch)
    ));

    let mut wrong_probability_offer = beta_offer.clone();
    wrong_probability_offer.success_probability = PartsPerMillion::new(980_000)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &beta_artifact,
            beta_artifact.artifact.artifact_digest,
            &wrong_probability_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::ProbabilityMismatch)
    ));

    let mut invalid_window_offer = beta_offer.clone();
    let mut invalid_window = beta_artifact.clone();
    invalid_window.artifact.observation_window_start =
        invalid_window.artifact.observation_window_end.clone();
    refresh_reliability_artifact_signature(
        &mut invalid_window,
        &mut invalid_window_offer,
        &signing_key,
    )?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &invalid_window,
            invalid_window.artifact.artifact_digest,
            &invalid_window_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::InvalidObservationWindow)
    ));

    let mut uncontained_offer = beta_offer.clone();
    let mut uncontained = beta_artifact.clone();
    uncontained.artifact.valid_from = timestamp("2026-07-20T11:00:00Z")?;
    refresh_reliability_artifact_signature(&mut uncontained, &mut uncontained_offer, &signing_key)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &uncontained,
            uncontained.artifact.artifact_digest,
            &uncontained_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::OfferValidityNotContained)
    ));

    let mut wrong_statement = beta_artifact.clone();
    wrong_statement.statement.observation_policy_digest = Sha256Digest::from_bytes([97; 32]);
    resign_reliability_statement(&mut wrong_statement, &signing_key)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &wrong_statement,
            wrong_statement.artifact.artifact_digest,
            &beta_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::StatementBindingMismatch)
    ));

    let mut wrong_statement_schema = beta_artifact.clone();
    wrong_statement_schema.statement.schema_version =
        "hyperintel.reliability-verifier-statement.v2".to_owned();
    resign_reliability_statement(&mut wrong_statement_schema, &signing_key)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &wrong_statement_schema,
            wrong_statement_schema.artifact.artifact_digest,
            &beta_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::StatementBindingMismatch)
    ));

    let mut wrong_statement_validity = beta_artifact.clone();
    wrong_statement_validity.statement.issued_at = timestamp("2026-07-20T01:00:00Z")?;
    resign_reliability_statement(&mut wrong_statement_validity, &signing_key)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &wrong_statement_validity,
            wrong_statement_validity.artifact.artifact_digest,
            &beta_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::StatementValidityMismatch)
    ));

    let mut zero_sequence = beta_artifact.clone();
    zero_sequence.statement.sequence = 0;
    resign_reliability_statement(&mut zero_sequence, &signing_key)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &zero_sequence,
            zero_sequence.artifact.artifact_digest,
            &beta_offer,
            &fixture.fixture.evaluated_at,
            &fixture.fixture.registry,
        ),
        Err(ReliabilityArtifactError::InvalidSequence)
    ));

    let mut wrong_class_registry = VerificationKeyRegistry::default();
    wrong_class_registry.register(reliability_key(AssuranceRootClassV1::ProviderKey, None)?)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &beta_artifact,
            beta_artifact.artifact.artifact_digest,
            &beta_offer,
            &fixture.fixture.evaluated_at,
            &wrong_class_registry,
        ),
        Err(ReliabilityArtifactError::TrustRootMismatch)
    ));

    let mut revoked_registry = VerificationKeyRegistry::default();
    revoked_registry.register(reliability_key(
        AssuranceRootClassV1::VerifierKey,
        Some(fixture.fixture.evaluated_at.clone()),
    )?)?;
    assert!(matches!(
        verify_signed_reliability_artifact_v1(
            &beta_artifact,
            beta_artifact.artifact.artifact_digest,
            &beta_offer,
            &fixture.fixture.evaluated_at,
            &revoked_registry,
        ),
        Err(ReliabilityArtifactError::Signature(SignatureError::Revoked))
    ));
    Ok(())
}

#[test]
fn contract_reliability_sequence_equivocation_blocks_distinct_artifacts_but_allows_reuse()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = reliability_fixture()?;
    let signing_key = SigningKey::from_bytes(&[70; 32]);
    let mut equivocated = fixture.reliability_artifacts.clone();
    let beta_digest = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.provider_id.as_str() == "provider:beta")
        .map(|offer| offer.reliability_evidence_digest)
        .ok_or("beta offer missing")?;
    let beta_artifact = equivocated
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?;
    beta_artifact.statement.sequence = 1;
    resign_reliability_statement(beta_artifact, &signing_key)?;
    assert!(matches!(
        reliability_route_with(
            &fixture.fixture,
            &fixture.fixture.signed_offers,
            &equivocated,
        ),
        Err(RoutingError::VerifiedReliabilitySigningSequenceEquivocation)
    ));

    let alpha = fixture
        .fixture
        .signed_offers
        .iter()
        .find(|offer| offer.offer.provider_id.as_str() == "provider:alpha")
        .ok_or("alpha signed offer missing")?;
    let mut reused_offer = alpha.offer.clone();
    reused_offer.offer_id = "offer:alpha-reuses-reliability".parse()?;
    reused_offer.fixed_request_fee = AtomicAmount(999);
    let reused = signed_capacity_offer(
        reused_offer,
        50,
        alpha.statement.assurance_envelope_digest,
        99,
    )?;
    let mut offers = fixture.fixture.signed_offers.clone();
    offers.push(reused);
    assert!(
        reliability_route_with(&fixture.fixture, &offers, &fixture.reliability_artifacts).is_ok()
    );
    Ok(())
}

#[test]
fn contract_v3_bundle_rejects_reliability_and_nonclaim_tamper()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = reliability_fixture()?;
    let bundle = reliability_bundle(&fixture)?;
    let beta_digest = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.provider_id.as_str() == "provider:beta")
        .map(|offer| offer.reliability_evidence_digest)
        .ok_or("beta offer missing")?;

    let mut missing = fixture.reliability_artifacts.clone();
    missing.remove(&beta_digest);
    assert!(
        verify_reliability_artifact_verified_market_evidence_bundle_v3(
            &bundle,
            &fixture.fixture.assurance_blobs,
            &missing,
            &fixture.fixture.registry,
        )
        .is_err()
    );

    let mut tampered = fixture.reliability_artifacts.clone();
    tampered
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?
        .artifact
        .total_observations = 101;
    assert!(
        verify_reliability_artifact_verified_market_evidence_bundle_v3(
            &bundle,
            &fixture.fixture.assurance_blobs,
            &tampered,
            &fixture.fixture.registry,
        )
        .is_err()
    );

    let mut missing_nonclaim = bundle;
    missing_nonclaim.explicit_nonclaims.retain(|claim| {
        claim != "no-observation-source-verification-or-future-reliability-accuracy"
    });
    assert!(
        verify_reliability_artifact_verified_market_evidence_bundle_v3(
            &missing_nonclaim,
            &fixture.fixture.assurance_blobs,
            &fixture.reliability_artifacts,
            &fixture.fixture.registry,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn contract_rejected_reliability_key_claim_does_not_become_trusted()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = reliability_fixture()?;
    let mut artifacts = fixture.reliability_artifacts.clone();
    let beta_digest = fixture
        .fixture
        .offers
        .iter()
        .find(|offer| offer.provider_id.as_str() == "provider:beta")
        .map(|offer| offer.reliability_evidence_digest)
        .ok_or("beta offer missing")?;
    artifacts
        .get_mut(&beta_digest)
        .ok_or("beta artifact missing")?
        .statement
        .key_id = "key:attacker-claimed".parse()?;

    let bundle = reliability_bundle_with(
        &fixture.fixture,
        &artifacts,
        &fixture.fixture.verification_keys,
    )?;
    assert_eq!(bundle.route.selected_provider_id.as_str(), "provider:alpha");
    assert!(
        !bundle
            .verification_keys
            .iter()
            .any(|key| key.key_id.as_str() == "key:attacker-claimed")
    );
    verify_reliability_artifact_verified_market_evidence_bundle_v3(
        &bundle,
        &fixture.fixture.assurance_blobs,
        &artifacts,
        &fixture.fixture.registry,
    )?;
    Ok(())
}

#[test]
fn contract_invalid_or_tampered_capacity_offer_signature_cannot_win()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    assert_eq!(
        signed_route(&fixture)?.selected_provider_id.as_str(),
        "provider:beta"
    );

    let beta_index = fixture
        .signed_offers
        .iter()
        .position(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;

    let mut cases = Vec::new();

    let mut subject_tamper = fixture.signed_offers.clone();
    subject_tamper[beta_index].statement.subject_digest = Sha256Digest::from_bytes([99; 32]);
    cases.push(subject_tamper);

    let mut wrong_signer = fixture.signed_offers.clone();
    wrong_signer[beta_index].statement.signer_id = "provider:alpha".parse()?;
    cases.push(wrong_signer);

    let mut wrong_key = fixture.signed_offers.clone();
    wrong_key[beta_index].statement.key_id = "key:alpha-offer".parse()?;
    cases.push(wrong_key);

    let mut expired = fixture.signed_offers.clone();
    expired[beta_index].statement.expires_at = timestamp("2026-07-20T11:59:59Z")?;
    cases.push(expired);

    let mut offer_tamper = fixture.signed_offers.clone();
    offer_tamper[beta_index].offer.fixed_request_fee = AtomicAmount(0);
    cases.push(offer_tamper);

    for invalid_offers in cases {
        let decision = signed_route_with(&fixture, &invalid_offers, &fixture.registry)?;
        assert_eq!(decision.selected_provider_id.as_str(), "provider:alpha");
    }

    let mut missing_statement = serde_json::to_value(&fixture.signed_offers[beta_index])?;
    missing_statement
        .as_object_mut()
        .ok_or("signed offer must serialize as an object")?
        .remove("statement");
    assert!(serde_json::from_value::<SignedCapacityOfferV1>(missing_statement).is_err());

    let mut revoked_registry = VerificationKeyRegistry::default();
    for mut key in fixture.verification_keys.clone() {
        if key.signer_id.as_str() == "provider:beta" && key.key_id.as_str() == "key:beta-offer" {
            key.revoked_at = Some(fixture.evaluated_at.clone());
        }
        revoked_registry.register(key)?;
    }
    let revoked_route = signed_route_with(&fixture, &fixture.signed_offers, &revoked_registry)?;
    assert_eq!(
        revoked_route.selected_provider_id.as_str(),
        "provider:alpha"
    );
    Ok(())
}

#[test]
fn contract_only_verified_offer_sequence_equivocation_blocks_the_batch()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let alpha_index = fixture
        .signed_offers
        .iter()
        .position(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:alpha")
        .ok_or("alpha signed offer missing")?;
    let beta_index = fixture
        .signed_offers
        .iter()
        .position(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;
    let alpha = &fixture.signed_offers[alpha_index];

    let mut second_alpha_offer = alpha.offer.clone();
    second_alpha_offer.offer_id = "offer:alpha-equivocation".parse()?;
    second_alpha_offer.fixed_request_fee = second_alpha_offer
        .fixed_request_fee
        .checked_add(AtomicAmount(1))?;
    let second_alpha = signed_capacity_offer(
        second_alpha_offer,
        50,
        alpha.statement.assurance_envelope_digest,
        alpha.statement.sequence,
    )?;
    let mut verified_equivocation = fixture.signed_offers.clone();
    verified_equivocation.push(second_alpha);
    assert!(matches!(
        signed_route_with(
            &fixture,
            &verified_equivocation,
            &fixture.registry
        ),
        Err(error)
            if error
                .downcast_ref::<RoutingError>()
                .is_some_and(|routing_error| matches!(
                    routing_error,
                    RoutingError::VerifiedOfferSigningSequenceEquivocation
                ))
    ));

    let mut spoofed_duplicate = fixture.signed_offers.clone();
    spoofed_duplicate[beta_index].statement.signer_id = alpha.statement.signer_id.clone();
    spoofed_duplicate[beta_index].statement.key_id = alpha.statement.key_id.clone();
    spoofed_duplicate[beta_index].statement.sequence = alpha.statement.sequence;
    let fallback = signed_route_with(&fixture, &spoofed_duplicate, &fixture.registry)?;
    assert_eq!(fallback.selected_provider_id.as_str(), "provider:alpha");
    Ok(())
}

#[test]
fn contract_signed_bundle_rejects_offer_signature_and_offer_tamper()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let bundle = signed_bundle(&fixture)?;

    let mut signature_tamper = bundle.clone();
    let replacement = if signature_tamper.offers[0]
        .statement
        .signature_hex
        .starts_with("00")
    {
        "01"
    } else {
        "00"
    };
    signature_tamper.offers[0]
        .statement
        .signature_hex
        .replace_range(0..2, replacement);
    assert!(
        verify_signed_market_evidence_bundle_v2(
            &signature_tamper,
            &fixture.assurance_blobs,
            &fixture.registry
        )
        .is_err()
    );

    let mut offer_tamper = bundle;
    offer_tamper.offers[0].offer.available_request_units = offer_tamper.offers[0]
        .offer
        .available_request_units
        .checked_add(1)
        .ok_or("fixture capacity overflow")?;
    assert!(
        verify_signed_market_evidence_bundle_v2(
            &offer_tamper,
            &fixture.assurance_blobs,
            &fixture.registry
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn contract_v2_bundle_reconstructs_rejected_unknown_key_without_trusting_it()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let mut offers = fixture.signed_offers.clone();
    let beta = offers
        .iter_mut()
        .find(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;
    beta.statement.key_id = "key:attacker-claimed".parse()?;

    let bundle = signed_bundle_with(&fixture, &offers)?;
    assert_eq!(bundle.route.selected_provider_id.as_str(), "provider:alpha");
    assert!(
        !bundle
            .verification_keys
            .iter()
            .any(|key| key.key_id.as_str() == "key:attacker-claimed")
    );
    verify_signed_market_evidence_bundle_v2(&bundle, &fixture.assurance_blobs, &fixture.registry)?;
    Ok(())
}

#[test]
fn contract_provider_authorization_is_bound_to_declared_provider_key()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let alpha = fixture
        .providers
        .get(&"provider:alpha".parse()?)
        .ok_or("alpha provider missing")?
        .clone();
    let alpha_claim = AssuranceClaimV1 {
        subject_digest: alpha.digest()?,
        property: AssuranceProperty::ProviderAuthorization,
    };
    let original_set_digest = fixture
        .assurance_sets
        .iter()
        .find(|(_, set)| set.envelope.guarantees.contains(&alpha_claim))
        .map(|(digest, _)| *digest)
        .ok_or("alpha assurance set missing")?;
    fixture.assurance_sets.remove(&original_set_digest);

    let cross_signed = signed_evidence_set(
        51,
        "provider:beta".parse()?,
        "key:beta-offer".parse()?,
        "provider-identity".parse()?,
        &alpha_claim,
        AssuranceRootClassV1::ProviderKey,
        b"cross-signed alpha authorization".to_vec(),
    )?;
    fixture
        .assurance_blobs
        .insert(cross_signed.1.0, cross_signed.1.1);
    fixture
        .assurance_sets
        .insert(cross_signed.0.evidence_set_digest, cross_signed.0);

    let decision = route(&fixture)?;
    let alpha_candidate = decision
        .candidates
        .iter()
        .find(|candidate| candidate.provider_id == alpha.provider_id)
        .ok_or("alpha route candidate missing")?;
    assert!(!alpha_candidate.eligible);
    Ok(())
}

#[test]
fn contract_policy_compliance_assurance_is_unsupported_in_r0()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    fixture
        .policy
        .required_assurance_properties
        .insert(AssuranceProperty::PolicyCompliance);
    assert!(route(&fixture).is_err());
    Ok(())
}

#[test]
fn contract_route_is_permutation_invariant_and_ineligible_offer_cannot_win()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let baseline = route(&fixture)?;
    fixture.offers.reverse();
    let reversed = route(&fixture)?;
    assert_eq!(baseline, reversed);

    let mut ineligible = fixture.offers[0].clone();
    ineligible.offer_id = "offer:ineligible".parse()?;
    ineligible.region = "region:forbidden".parse()?;
    ineligible.fixed_request_fee = AtomicAmount(0);
    ineligible.input_price_per_million = AtomicAmount(0);
    ineligible.output_price_per_million = AtomicAmount(0);
    fixture.offers.push(ineligible);
    let with_ineligible = route(&fixture)?;
    assert_eq!(
        baseline.selected_offer_digest,
        with_ineligible.selected_offer_digest
    );
    assert_eq!(
        baseline.selected_provider_id,
        with_ineligible.selected_provider_id
    );
    Ok(())
}

#[test]
fn contract_bundle_rejects_semantic_tamper_even_with_original_bundle_digest()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let route = route(&fixture)?;
    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        MeteringReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            execution_status: ExecutionStatusV1::Completed,
            prompt_digest: fixture.policy.prompt_digest,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
            buyer_observation_digest: None,
        },
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    let mut bundle = build_market_evidence_bundle_v1(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        fixture.offers.clone(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        Vec::new(),
    )?;
    bundle.settlement.buyer_refund_amount = AtomicAmount(
        bundle
            .settlement
            .buyer_refund_amount
            .0
            .checked_add(1)
            .ok_or("fixture overflow")?,
    );
    assert!(
        verify_market_evidence_bundle_v1(&bundle, &fixture.assurance_blobs, &fixture.registry)
            .is_err()
    );
    Ok(())
}

fn route(fixture: &Fixture) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    Ok(select_route_v1(&RoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers: &fixture.offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        verification_keys: &fixture.registry,
    })?)
}

fn rejected_candidate_report_digest(
    route: &RouteDecisionV1,
    provider_id: &str,
) -> Result<Sha256Digest, Box<dyn std::error::Error>> {
    let candidate = route
        .candidates
        .iter()
        .find(|candidate| candidate.provider_id.as_str() == provider_id)
        .ok_or("rejected candidate missing")?;
    if candidate.eligible {
        return Err("expected rejected candidate".into());
    }
    Ok(candidate.eligibility_report_digest)
}

fn remove_provider_assurance(
    fixture: &mut Fixture,
    provider_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let provider_id: ProtocolId = provider_id.parse()?;
    let evidence_set_digest = fixture
        .assurance_sets
        .iter()
        .find_map(|(digest, set)| {
            set.signed_statements
                .iter()
                .any(|statement| statement.signer_id == provider_id)
                .then_some(*digest)
        })
        .ok_or("provider assurance set missing")?;
    let removed_set = fixture
        .assurance_sets
        .remove(&evidence_set_digest)
        .ok_or("provider assurance set disappeared")?;
    for blob in removed_set.evidence_blobs {
        fixture.assurance_blobs.remove(&blob.digest);
    }
    fixture
        .verification_keys
        .retain(|key| key.signer_id != provider_id);
    let mut registry = VerificationKeyRegistry::default();
    for key in &fixture.verification_keys {
        registry.register(key.clone())?;
    }
    fixture.registry = registry;
    Ok(())
}

fn market_bundle(fixture: &Fixture) -> Result<MarketEvidenceBundleV1, Box<dyn std::error::Error>> {
    let route = route(fixture)?;
    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        MeteringReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            execution_status: ExecutionStatusV1::Completed,
            prompt_digest: fixture.policy.prompt_digest,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
            buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
        },
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    Ok(build_market_evidence_bundle_v1(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        fixture.offers.clone(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        vec![
            "provider authorization is evaluated only from supplied signed evidence".to_owned(),
            "success probability is an evidence-qualified declared input".to_owned(),
        ],
    )?)
}

fn signed_route(fixture: &Fixture) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    signed_route_with(fixture, &fixture.signed_offers, &fixture.registry)
}

fn signed_route_with(
    fixture: &Fixture,
    offers: &[SignedCapacityOfferV1],
    registry: &VerificationKeyRegistry,
) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    Ok(select_signed_route_v1(&SignedRoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        verification_keys: registry,
    })?)
}

fn signed_bundle(
    fixture: &Fixture,
) -> Result<SignedMarketEvidenceBundleV2, Box<dyn std::error::Error>> {
    signed_bundle_with(fixture, &fixture.signed_offers)
}

fn signed_bundle_with(
    fixture: &Fixture,
    offers: &[SignedCapacityOfferV1],
) -> Result<SignedMarketEvidenceBundleV2, Box<dyn std::error::Error>> {
    let route = signed_route_with(fixture, offers, &fixture.registry)?;
    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        MeteringReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            execution_status: ExecutionStatusV1::Completed,
            prompt_digest: fixture.policy.prompt_digest,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
            buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
        },
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    Ok(build_signed_market_evidence_bundle_v2(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        offers.to_vec(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        vec![
            "capacity-offer authenticity is checked only against caller-supplied keys".to_owned(),
            "success probability remains an unverified declared input".to_owned(),
        ],
    )?)
}

fn reliability_route(
    fixture: &ReliabilityFixture,
) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    Ok(reliability_route_with(
        &fixture.fixture,
        &fixture.fixture.signed_offers,
        &fixture.reliability_artifacts,
    )?)
}

fn reliability_route_with(
    fixture: &Fixture,
    offers: &[SignedCapacityOfferV1],
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
) -> Result<RouteDecisionV1, RoutingError> {
    select_reliability_artifact_verified_route_v1(&ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        reliability_artifacts,
        verification_keys: &fixture.registry,
    })
}

fn reliability_bundle(
    fixture: &ReliabilityFixture,
) -> Result<ReliabilityArtifactVerifiedMarketEvidenceBundleV3, Box<dyn std::error::Error>> {
    reliability_bundle_with(
        &fixture.fixture,
        &fixture.reliability_artifacts,
        &fixture.fixture.verification_keys,
    )
}

fn reliability_bundle_with(
    fixture: &Fixture,
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    verification_keys: &[RegisteredVerificationKeyV1],
) -> Result<ReliabilityArtifactVerifiedMarketEvidenceBundleV3, Box<dyn std::error::Error>> {
    let route = reliability_route_with(fixture, &fixture.signed_offers, reliability_artifacts)?;
    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        MeteringReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            execution_status: ExecutionStatusV1::Completed,
            prompt_digest: fixture.policy.prompt_digest,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
            buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
        },
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    Ok(build_reliability_artifact_verified_market_evidence_bundle_v3(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        fixture.signed_offers.clone(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        verification_keys.to_vec(),
        route,
        metering,
        settlement,
        vec![
            "capacity-offer authenticity is checked only against caller-supplied keys".to_owned(),
            "reliability counts and source commitments are accepted only as verifier-signed supplied evidence".to_owned(),
        ],
    )?)
}

fn replay_protected_fixture()
-> Result<(ReliabilityFixture, BuyerReplayStateV1), Box<dyn std::error::Error>> {
    let mut fixture = reliability_fixture()?;
    let before_state = build_buyer_replay_state_v1(
        fixture.fixture.policy.buyer_id.clone(),
        fixture.fixture.policy.nonce,
    )?;
    fixture.fixture.policy.replay_snapshot_digest = before_state.state_digest;
    Ok((fixture, before_state))
}

fn reliability_evaluation(
    fixture: &ReliabilityFixture,
) -> Result<RouteEvaluationRecordV1, Box<dyn std::error::Error>> {
    Ok(evaluate_reliability_artifact_verified_route_v1(
        &ReliabilityArtifactVerifiedRoutingContextV1 {
            evaluated_at: &fixture.fixture.evaluated_at,
            service: &fixture.fixture.service,
            providers: &fixture.fixture.providers,
            offers: &fixture.fixture.signed_offers,
            policy: &fixture.fixture.policy,
            assurance_sets: &fixture.fixture.assurance_sets,
            assurance_blobs: &fixture.fixture.assurance_blobs,
            reliability_artifacts: &fixture.reliability_artifacts,
            verification_keys: &fixture.fixture.registry,
        },
    )?)
}

fn replay_protected_bundle(
    fixture: &ReliabilityFixture,
    before_state: &BuyerReplayStateV1,
) -> Result<ReplayProtectedMarketEvidenceBundleV4, Box<dyn std::error::Error>> {
    let evaluation = reliability_evaluation(fixture)?;
    let replay_transition = derive_replay_state_transition_v1(
        before_state,
        &fixture.fixture.policy,
        evaluation.evaluation_digest,
        &fixture.fixture.evaluated_at,
    )?;
    let (metering, settlement) = match &evaluation.outcome {
        RouteEvaluationOutcomeV1::Selected(route) => {
            let provider = fixture
                .fixture
                .providers
                .get(&route.selected_provider_id)
                .ok_or("selected provider missing")?;
            let offer = fixture
                .fixture
                .offers
                .iter()
                .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
                .ok_or("selected offer missing")?;
            let metering = build_metering_receipt_v1(
                &fixture.fixture.service,
                provider,
                offer,
                &fixture.fixture.policy,
                route,
                MeteringReceiptDraftV1 {
                    started_at: timestamp("2026-07-20T12:00:01Z")?,
                    first_token_at: timestamp("2026-07-20T12:00:02Z")?,
                    completed_at: timestamp("2026-07-20T12:00:04Z")?,
                    input_tokens: 900,
                    output_tokens: 400,
                    execution_status: ExecutionStatusV1::Completed,
                    prompt_digest: fixture.fixture.policy.prompt_digest,
                    output_digest: Sha256Digest::from_bytes([31; 32]),
                    provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
                    buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
                },
            )?;
            let settlement = derive_settlement_intent_v1(
                &fixture.fixture.service,
                provider,
                offer,
                &fixture.fixture.policy,
                route,
                &metering,
                timestamp("2026-07-20T13:00:00Z")?,
            )?;
            (Some(metering), Some(settlement))
        }
        RouteEvaluationOutcomeV1::NoEligibleOffer => (None, None),
    };
    Ok(build_replay_protected_market_evidence_bundle_v4(
        fixture.fixture.evaluated_at.clone(),
        fixture.fixture.service.clone(),
        fixture.fixture.providers.clone(),
        fixture.fixture.signed_offers.clone(),
        fixture.fixture.policy.clone(),
        fixture.fixture.assurance_sets.clone(),
        fixture.fixture.verification_keys.clone(),
        evaluation,
        replay_transition,
        metering,
        settlement,
        vec![
            "capacity-offer authenticity is checked only against caller-supplied keys".to_owned(),
            "replay advancement is pure data over caller-supplied state".to_owned(),
            "reliability counts and source commitments are accepted only as verifier-signed supplied evidence".to_owned(),
        ],
    )?)
}

fn assert_authorization_windows(
    fixture: &Fixture,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
    route: &RouteDecisionV1,
    draft: &MeteringReceiptDraftV1,
    metering: &hyperintel_market::MeteringReceiptV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut late_draft = draft.clone();
    late_draft.completed_at = timestamp("2026-07-20T14:00:00Z")?;
    assert!(
        build_metering_receipt_v1(
            &fixture.service,
            provider,
            offer,
            &fixture.policy,
            route,
            late_draft
        )
        .is_err()
    );
    assert!(
        derive_settlement_intent_v1(
            &fixture.service,
            provider,
            offer,
            &fixture.policy,
            route,
            metering,
            metering.completed_at.clone()
        )
        .is_err()
    );
    let mut fabricated_metering = metering.clone();
    fabricated_metering.actual_provider_charge = AtomicAmount(
        fabricated_metering
            .actual_provider_charge
            .0
            .checked_add(1)
            .ok_or("fabricated charge overflow")?,
    );
    assert!(
        derive_settlement_intent_v1(
            &fixture.service,
            provider,
            offer,
            &fixture.policy,
            route,
            &fabricated_metering,
            timestamp("2026-07-20T13:00:00Z")?
        )
        .is_err()
    );
    Ok(())
}

#[allow(clippy::too_many_lines)] // A complete cross-crate fixture is intentionally explicit.
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let evaluated_at = timestamp("2026-07-20T12:00:00Z")?;
    let service = ServiceSpecV1 {
        schema_version: "hyperintel.service-spec.v1".to_owned(),
        service_id: "service:open-70b-v1".parse()?,
        model_family: "model:open-70b".parse()?,
        model_version: "version:1".parse()?,
        model_artifact_digest: Sha256Digest::from_bytes([1; 32]),
        tokenizer_digest: Sha256Digest::from_bytes([2; 32]),
        runtime_image_digest: Sha256Digest::from_bytes([3; 32]),
        region: "region:us-east".parse()?,
        privacy_mode: PrivacyMode::DirectSealed,
        retention_mode: RetentionMode::NoContentRetentionDeclared,
        capabilities: [
            InferenceCapability::Streaming,
            InferenceCapability::ToolCalls,
        ]
        .into_iter()
        .collect(),
        max_input_tokens: 4_096,
        max_output_tokens: 1_024,
        metering_method: MeteringMethod::TokenizerDigest,
    };
    let service_digest = service.digest()?;
    let mut providers = BTreeMap::new();
    for name in ["alpha", "beta"] {
        let provider_id: ProtocolId = format!("provider:{name}").parse()?;
        providers.insert(
            provider_id.clone(),
            ProviderIdentityV1 {
                schema_version: "hyperintel.provider-identity.v1".to_owned(),
                provider_id,
                status: ProviderStatus::Active,
                valid_from: timestamp("2026-07-20T00:00:00Z")?,
                valid_until: timestamp("2026-07-21T00:00:00Z")?,
                offer_key_id: format!("key:{name}-offer").parse()?,
                receipt_key_id: format!("key:{name}-receipt").parse()?,
                payout_account: format!("account:{name}").parse()?,
                authorized_service_digests: [service_digest].into_iter().collect(),
            },
        );
    }
    let offers = vec![
        CapacityOfferV1 {
            schema_version: "hyperintel.capacity-offer.v1".to_owned(),
            offer_id: "offer:alpha".parse()?,
            provider_id: "provider:alpha".parse()?,
            service_spec_digest: service_digest,
            valid_from: timestamp("2026-07-20T00:00:00Z")?,
            valid_until: timestamp("2026-07-21T00:00:00Z")?,
            available_request_units: 10,
            max_input_tokens: 4_096,
            max_output_tokens: 1_024,
            fixed_request_fee: AtomicAmount(1_000),
            input_price_per_million: AtomicAmount(10_000),
            output_price_per_million: AtomicAmount(20_000),
            success_probability: PartsPerMillion::new(800_000)?,
            p95_latency_ms: 5_000,
            settlement_asset: "asset:test-usdc".parse()?,
            region: "region:us-east".parse()?,
            reliability_evidence_digest: Sha256Digest::from_bytes([21; 32]),
        },
        CapacityOfferV1 {
            schema_version: "hyperintel.capacity-offer.v1".to_owned(),
            offer_id: "offer:beta".parse()?,
            provider_id: "provider:beta".parse()?,
            service_spec_digest: service_digest,
            valid_from: timestamp("2026-07-20T00:00:00Z")?,
            valid_until: timestamp("2026-07-21T00:00:00Z")?,
            available_request_units: 10,
            max_input_tokens: 4_096,
            max_output_tokens: 1_024,
            fixed_request_fee: AtomicAmount(1_100),
            input_price_per_million: AtomicAmount(10_000),
            output_price_per_million: AtomicAmount(20_000),
            success_probability: PartsPerMillion::new(990_000)?,
            p95_latency_ms: 3_000,
            settlement_asset: "asset:test-usdc".parse()?,
            region: "region:us-east".parse()?,
            reliability_evidence_digest: Sha256Digest::from_bytes([22; 32]),
        },
    ];
    let policy = InferenceRequestPolicyV1 {
        schema_version: "hyperintel.request-policy.v1".to_owned(),
        request_id: "request:demo-1".parse()?,
        buyer_id: "buyer:demo".parse()?,
        buyer_refund_account: "account:buyer-demo".parse()?,
        nonce: 1,
        valid_from: timestamp("2026-07-20T11:00:00Z")?,
        valid_until: timestamp("2026-07-20T13:00:00Z")?,
        required_service_spec_digest: service_digest,
        max_input_tokens: 1_000,
        max_output_tokens: 500,
        max_total_cost: AtomicAmount(2_000),
        settlement_asset: "asset:test-usdc".parse()?,
        allowed_regions: ["region:us-east".parse()?].into_iter().collect(),
        allowed_providers: BTreeSet::new(),
        denied_providers: BTreeSet::new(),
        required_capabilities: [InferenceCapability::Streaming].into_iter().collect(),
        minimum_privacy_mode: PrivacyMode::DirectSealed,
        minimum_assurance_maturity: AssuranceMaturity::Declared,
        required_assurance_properties: [
            AssuranceProperty::ProviderAuthorization,
            AssuranceProperty::ServiceManifestBinding,
        ]
        .into_iter()
        .collect(),
        minimum_success_probability: PartsPerMillion::new(750_000)?,
        max_p95_latency_ms: 6_000,
        protocol_fee: BasisPoints::new(400)?,
        protocol_fee_recipient: "account:hyperintel-fees".parse()?,
        prompt_digest: Sha256Digest::from_bytes([11; 32]),
        replay_snapshot_digest: Sha256Digest::from_bytes([12; 32]),
    };

    let mut assurance_sets = BTreeMap::new();
    let mut assurance_blobs = BTreeMap::new();
    let mut registry = VerificationKeyRegistry::default();
    let mut verification_keys = Vec::new();
    let mut provider_signing_material = BTreeMap::new();
    for (index, provider) in providers.values().enumerate() {
        let provider_digest = provider.digest()?;
        let key_byte = 50_u8
            .checked_add(u8::try_from(index)?)
            .ok_or("key overflow")?;
        let built = signed_evidence_set(
            key_byte,
            provider.provider_id.clone(),
            provider.offer_key_id.clone(),
            "provider-identity".parse()?,
            &AssuranceClaimV1 {
                subject_digest: provider_digest,
                property: AssuranceProperty::ProviderAuthorization,
            },
            AssuranceRootClassV1::ProviderKey,
            format!("provider authorization {index}").into_bytes(),
        )?;
        provider_signing_material.insert(
            provider.provider_id.clone(),
            (key_byte, built.0.envelope.digest()?),
        );
        registry.register(built.2.clone())?;
        verification_keys.push(built.2);
        assurance_blobs.insert(built.1.0, built.1.1);
        assurance_sets.insert(built.0.evidence_set_digest, built.0);
    }
    let service_evidence = signed_evidence_set(
        60,
        "verifier:service".parse()?,
        "key:service-1".parse()?,
        "service-spec".parse()?,
        &AssuranceClaimV1 {
            subject_digest: service_digest,
            property: AssuranceProperty::ServiceManifestBinding,
        },
        AssuranceRootClassV1::VerifierKey,
        b"service manifest binding".to_vec(),
    )?;
    registry.register(service_evidence.2.clone())?;
    verification_keys.push(service_evidence.2);
    assurance_blobs.insert(service_evidence.1.0, service_evidence.1.1);
    assurance_sets.insert(service_evidence.0.evidence_set_digest, service_evidence.0);

    let signed_offers = offers
        .iter()
        .enumerate()
        .map(|(index, offer)| {
            let (key_byte, envelope_digest) = provider_signing_material
                .get(&offer.provider_id)
                .copied()
                .ok_or("provider signing material missing")?;
            signed_capacity_offer(
                offer.clone(),
                key_byte,
                envelope_digest,
                u64::try_from(index)?
                    .checked_add(1)
                    .ok_or("sequence overflow")?,
            )
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

    Ok(Fixture {
        evaluated_at,
        service,
        providers,
        offers,
        signed_offers,
        policy,
        assurance_sets,
        assurance_blobs,
        registry,
        verification_keys,
    })
}

fn reliability_fixture() -> Result<ReliabilityFixture, Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let verifier_signing_key = SigningKey::from_bytes(&[70; 32]);
    let reliability_key = reliability_key(AssuranceRootClassV1::VerifierKey, None)?;
    fixture.registry.register(reliability_key.clone())?;
    fixture.verification_keys.push(reliability_key);

    let offer_signing_material = fixture
        .signed_offers
        .iter()
        .map(|signed_offer| {
            (
                signed_offer.offer.provider_id.clone(),
                (
                    signed_offer.statement.assurance_envelope_digest,
                    signed_offer.statement.sequence,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut reliability_artifacts = BTreeMap::new();
    for (index, offer) in fixture.offers.iter_mut().enumerate() {
        let (successful_observations, total_observations) =
            if offer.provider_id.as_str() == "provider:alpha" {
                (8, 10)
            } else {
                (99, 100)
            };
        let artifact = build_reliability_artifact_v1(ReliabilityArtifactDraftV1 {
            provider_id: offer.provider_id.clone(),
            service_spec_digest: offer.service_spec_digest,
            observation_policy_digest: Sha256Digest::from_bytes([40; 32]),
            source_observation_digest: Sha256Digest::from_bytes(
                [41_u8
                    .checked_add(u8::try_from(index)?)
                    .ok_or("reliability source digest byte overflow")?; 32],
            ),
            observation_window_start: timestamp("2026-07-18T00:00:00Z")?,
            observation_window_end: timestamp("2026-07-19T23:59:59Z")?,
            valid_from: offer.valid_from.clone(),
            valid_until: offer.valid_until.clone(),
            successful_observations,
            total_observations,
        })?;
        offer.reliability_evidence_digest = artifact.artifact_digest;
        let signed_artifact = signed_reliability_artifact(
            artifact,
            &verifier_signing_key,
            u64::try_from(index)?
                .checked_add(1)
                .ok_or("reliability sequence overflow")?,
        )?;
        reliability_artifacts.insert(signed_artifact.artifact.artifact_digest, signed_artifact);
    }
    fixture.signed_offers = fixture
        .offers
        .iter()
        .map(|offer| {
            let (assurance_envelope_digest, sequence) = offer_signing_material
                .get(&offer.provider_id)
                .copied()
                .ok_or("provider offer signing material missing")?;
            let key_byte = if offer.provider_id.as_str() == "provider:alpha" {
                50
            } else {
                51
            };
            signed_capacity_offer(offer.clone(), key_byte, assurance_envelope_digest, sequence)
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok(ReliabilityFixture {
        fixture,
        reliability_artifacts,
    })
}

type BuiltEvidence = (
    AssuranceEvidenceSetV1,
    (Sha256Digest, Vec<u8>),
    RegisteredVerificationKeyV1,
);

fn signed_evidence_set(
    key_byte: u8,
    signer_id: ProtocolId,
    key_id: ProtocolId,
    subject_kind: ProtocolId,
    claim: &AssuranceClaimV1,
    root_class: AssuranceRootClassV1,
    evidence_bytes: Vec<u8>,
) -> Result<BuiltEvidence, Box<dyn std::error::Error>> {
    let evidence_ref = evidence_blob_ref_v1(
        &evidence_bytes,
        "application/json",
        EvidenceClassification::Public,
    )?;
    let envelope = AssuranceEnvelopeV1 {
        schema_version: "hyperintel.assurance-envelope.v1".to_owned(),
        guarantees: [claim.clone()].into_iter().collect(),
        assumptions: BTreeSet::new(),
        exclusions: BTreeSet::new(),
        maturity: match root_class {
            AssuranceRootClassV1::VerifierKey => AssuranceMaturity::LocallyChecked,
            AssuranceRootClassV1::ProviderKey | AssuranceRootClassV1::BuyerPolicy => {
                AssuranceMaturity::Declared
            }
            AssuranceRootClassV1::HardwareVendor => AssuranceMaturity::Attested,
        },
        trust_roots: [AssuranceRootV1 {
            class: root_class,
            root_id: key_id.clone(),
        }]
        .into_iter()
        .collect(),
        valid: ValidityWindowV1 {
            start: timestamp("2026-07-20T00:00:00Z")?,
            end: timestamp("2026-07-21T00:00:00Z")?,
        },
        source_digests: [evidence_ref.digest].into_iter().collect(),
    };
    let signing_key = SigningKey::from_bytes(&[key_byte; 32]);
    let registered = RegisteredVerificationKeyV1 {
        signer_id: signer_id.clone(),
        key_id: key_id.clone(),
        trust_root_class: root_class,
        public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T00:00:00Z")?,
        valid_until: timestamp("2026-07-22T00:00:00Z")?,
        revoked_at: None,
    };
    let mut statement = SignedStatementV1 {
        schema_version: "hyperintel.signed-statement.v1".to_owned(),
        subject_kind,
        subject_digest: claim.subject_digest,
        assurance_envelope_digest: envelope.digest()?,
        issued_at: timestamp("2026-07-20T10:00:00Z")?,
        expires_at: timestamp("2026-07-21T00:00:00Z")?,
        sequence: 1,
        signer_id,
        key_id,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    let set = build_assurance_evidence_set_v1(
        envelope,
        [evidence_ref.clone()].into_iter().collect(),
        vec![statement],
        Sha256Digest::from_bytes([90; 32]),
    )?;
    Ok((set, (evidence_ref.digest, evidence_bytes), registered))
}

fn signed_capacity_offer(
    offer: CapacityOfferV1,
    key_byte: u8,
    provider_assurance_envelope_digest: Sha256Digest,
    sequence: u64,
) -> Result<SignedCapacityOfferV1, Box<dyn std::error::Error>> {
    let signing_key = SigningKey::from_bytes(&[key_byte; 32]);
    let mut statement = SignedStatementV1 {
        schema_version: "hyperintel.signed-statement.v1".to_owned(),
        subject_kind: "capacity-offer".parse()?,
        subject_digest: offer.digest()?,
        assurance_envelope_digest: provider_assurance_envelope_digest,
        issued_at: timestamp("2026-07-20T10:00:00Z")?,
        expires_at: timestamp("2026-07-20T23:59:59Z")?,
        sequence,
        signer_id: offer.provider_id.clone(),
        key_id: format!(
            "key:{}-offer",
            offer
                .provider_id
                .as_str()
                .strip_prefix("provider:")
                .ok_or("fixture provider prefix missing")?
        )
        .parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedCapacityOfferV1 {
        schema_version: "hyperintel.signed-capacity-offer.v1".to_owned(),
        offer,
        statement,
    })
}

fn signed_reliability_artifact(
    artifact: hyperintel_market::ReliabilityArtifactV1,
    signing_key: &SigningKey,
    sequence: u64,
) -> Result<SignedReliabilityArtifactV1, Box<dyn std::error::Error>> {
    let mut statement = ReliabilityVerifierStatementV1 {
        schema_version: "hyperintel.reliability-verifier-statement.v1".to_owned(),
        artifact_digest: artifact.artifact_digest,
        observation_policy_digest: artifact.observation_policy_digest,
        issued_at: artifact.valid_from.clone(),
        expires_at: artifact.valid_until.clone(),
        sequence,
        signer_id: "verifier:reliability".parse()?,
        key_id: "key:reliability-1".parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedReliabilityArtifactV1 {
        schema_version: "hyperintel.signed-reliability-artifact.v1".to_owned(),
        artifact,
        statement,
    })
}

fn reliability_key(
    trust_root_class: AssuranceRootClassV1,
    revoked_at: Option<UtcTimestamp>,
) -> Result<RegisteredVerificationKeyV1, Box<dyn std::error::Error>> {
    let signing_key = SigningKey::from_bytes(&[70; 32]);
    Ok(RegisteredVerificationKeyV1 {
        signer_id: "verifier:reliability".parse()?,
        key_id: "key:reliability-1".parse()?,
        trust_root_class,
        public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-19T00:00:00Z")?,
        valid_until: timestamp("2026-07-22T00:00:00Z")?,
        revoked_at,
    })
}

fn signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    signing_key: &SigningKey,
) -> Result<SignedInferenceRequestPolicyV1, Box<dyn std::error::Error>> {
    let mut statement = BuyerPolicyStatementV1 {
        schema_version: "hyperintel.buyer-policy-statement.v1".to_owned(),
        request_policy_digest: policy.digest()?,
        replay_snapshot_digest: policy.replay_snapshot_digest,
        issued_at: policy.valid_from.clone(),
        expires_at: policy.valid_until.clone(),
        sequence: policy.nonce,
        signer_id: policy.buyer_id.clone(),
        key_id: "key:buyer-policy-1".parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedInferenceRequestPolicyV1 {
        schema_version: "hyperintel.signed-inference-request-policy.v1".to_owned(),
        policy: policy.clone(),
        statement,
    })
}

fn signed_provider_execution_statement(
    receipt: &hyperintel_market::ProviderExecutionReceiptV1,
    admission: &hyperintel_market::AuthenticatedExecutionAdmissionV1,
    signing_key: &SigningKey,
    sequence: u64,
) -> Result<ProviderExecutionStatementV1, Box<dyn std::error::Error>> {
    let mut statement = ProviderExecutionStatementV1 {
        schema_version: "hyperintel.provider-execution-statement.v1".to_owned(),
        receipt_digest: receipt.receipt_digest,
        admission_digest: admission.admission_digest,
        issued_at: receipt.completed_at.clone(),
        expires_at: admission.deadline_at.clone(),
        sequence,
        signer_id: receipt.provider_id.clone(),
        key_id: "key:beta-receipt".parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(statement)
}

fn execution_registry(
    existing_keys: &[RegisteredVerificationKeyV1],
    buyer_signing_key: &SigningKey,
    receipt_signing_key: &SigningKey,
) -> Result<VerificationKeyRegistry, Box<dyn std::error::Error>> {
    let mut registry = VerificationKeyRegistry::default();
    for key in existing_keys {
        registry.register(key.clone())?;
    }
    registry.register(RegisteredVerificationKeyV1 {
        signer_id: "buyer:demo".parse()?,
        key_id: "key:buyer-policy-1".parse()?,
        trust_root_class: AssuranceRootClassV1::BuyerPolicy,
        public_key_hex: HEXLOWER.encode(buyer_signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })?;
    registry.register(RegisteredVerificationKeyV1 {
        signer_id: "provider:beta".parse()?,
        key_id: "key:beta-receipt".parse()?,
        trust_root_class: AssuranceRootClassV1::ProviderKey,
        public_key_hex: HEXLOWER.encode(receipt_signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })?;
    Ok(registry)
}

fn refresh_reliability_artifact_signature(
    signed: &mut SignedReliabilityArtifactV1,
    offer: &mut CapacityOfferV1,
    signing_key: &SigningKey,
) -> Result<(), Box<dyn std::error::Error>> {
    signed.artifact.artifact_digest = signed.artifact.digest()?;
    offer.reliability_evidence_digest = signed.artifact.artifact_digest;
    signed.statement.artifact_digest = signed.artifact.artifact_digest;
    signed.statement.observation_policy_digest = signed.artifact.observation_policy_digest;
    signed.statement.issued_at = signed.artifact.valid_from.clone();
    signed.statement.expires_at = signed.artifact.valid_until.clone();
    resign_reliability_statement(signed, signing_key)
}

fn resign_reliability_statement(
    signed: &mut SignedReliabilityArtifactV1,
    signing_key: &SigningKey,
) -> Result<(), Box<dyn std::error::Error>> {
    signed.statement.signature_hex.clear();
    signed.statement.signature_hex = HEXLOWER.encode(
        &signing_key
            .sign(&signed.statement.signing_message()?)
            .to_bytes(),
    );
    Ok(())
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
