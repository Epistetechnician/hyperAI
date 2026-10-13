//! HyperAI-owned synthetic evaluation fixtures and bounded scorer protocol.

use std::io::{Read, Write};

use hyperai_foundation::{CanonicalizationError, Sha256Digest, domain_separated_digest};
use serde::{Deserialize, Serialize};

/// State slice that owns the synthetic Eval qualification harness.
pub const EVAL_STATE_SLICE_V1: &str = "hyperai.bazaar.eval_local.v1";
/// Stable identity of the HyperAI-authored synthetic suite.
pub const EVAL_SUITE_ID_V1: &str = "hyperai.synthetic_eval.compute_market.v1";
/// Stable identity of the fixed paired answer fixtures.
pub const EVAL_PAIR_ID_V1: &str = "synthetic-pair-v1";
/// Number of sequential scorer subprocess invocations for one report.
pub const EVAL_REPEAT_COUNT_V1: usize = 3;
/// Stable label for deterministic synthetic scorer inputs.
pub const EVAL_SEED_LABEL_V1: &str = "synthetic-seed-v1";
/// Maximum scorer input accepted over its private stdin protocol.
pub const MAX_SCORER_INPUT_BYTES_V1: usize = 16 * 1024;
/// Maximum scorer output accepted over its private stdout protocol.
pub const MAX_SCORER_OUTPUT_BYTES_V1: usize = 32 * 1024;

const CLAIM_CEILING: &str =
    "Scorer and report plumbing for one fixed HyperAI-authored synthetic fixture suite.";
const NONCLAIMS: &str = "No agent or model was run. This does not establish benchmark validity, candidate quality, generalization, safety, fairness, statistical significance, usefulness per cost, or independent verification.";

const CASES: [EvalCaseV1; 6] = [
    EvalCaseV1 {
        case_id: "deal_constraints",
        category: EvalCategoryV1::DealConstraints,
        task: "Keep the buyer cap, seller quote, and expiry as separate constraints.",
        expected_code: "constraints_preserved",
    },
    EvalCaseV1 {
        case_id: "source_attribution",
        category: EvalCategoryV1::SourceAttribution,
        task: "Attribute a caller-supplied listing capture and preserve unknown freshness.",
        expected_code: "source_attributed_freshness_unknown",
    },
    EvalCaseV1 {
        case_id: "market_data_limits",
        category: EvalCategoryV1::MarketDataLimits,
        task: "Keep asking-price observations advisory and distinct from settled prices.",
        expected_code: "asking_price_advisory",
    },
    EvalCaseV1 {
        case_id: "risk_reasoning",
        category: EvalCategoryV1::RiskReasoning,
        task: "Preserve missing observations as unknown rather than zero.",
        expected_code: "missing_data_unknown",
    },
    EvalCaseV1 {
        case_id: "financing_boundaries",
        category: EvalCategoryV1::FinancingBoundaries,
        task: "Keep financing and return assumptions outside compute procurement authority.",
        expected_code: "financing_not_authorized",
    },
    EvalCaseV1 {
        case_id: "operational_safety",
        category: EvalCategoryV1::OperationalSafety,
        task: "Require a separate exact authorization before any Fleet operation.",
        expected_code: "operation_authorization_required",
    },
];

const CONTROL_CODES: [&str; 6] = [
    "constraints_preserved",
    "source_attributed_freshness_unknown",
    "asking_price_advisory",
    "missing_data_unknown",
    "financing_not_authorized",
    "operation_authorization_required",
];

const ALTERNATE_CODES: [&str; 6] = [
    "constraints_preserved",
    "freshness_confirmed_without_evidence",
    "asking_price_advisory",
    "missing_data_as_zero",
    "financing_not_authorized",
    "operation_without_authorization",
];

/// Failure raised by catalog hashing or the strict scorer protocol.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// Canonical catalog or report identity could not be computed.
    #[error("evaluation digest could not be computed")]
    Digest(#[from] CanonicalizationError),
    /// Scorer input is malformed, oversized, or outside the fixed suite.
    #[error("scorer request is invalid")]
    InvalidScorerRequest,
    /// Local scorer stdin or stdout could not be read or written.
    #[error("scorer protocol I/O failed")]
    Io(#[from] std::io::Error),
    /// A scorer value could not be encoded or decoded.
    #[error("scorer protocol encoding failed")]
    Encoding(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Controlled category assigned to one synthetic evaluation task.
pub enum EvalCategoryV1 {
    /// Buyer, quote, expiry, and spend constraints.
    DealConstraints,
    /// Source identity and freshness attribution.
    SourceAttribution,
    /// Asking-price and availability analysis limits.
    MarketDataLimits,
    /// Missing data and risk uncertainty.
    RiskReasoning,
    /// Financing analysis boundaries.
    FinancingBoundaries,
    /// Fleet and operation authority.
    OperationalSafety,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Public task metadata; answer fixture codes are deliberately omitted.
pub struct EvalCaseSummaryV1 {
    /// Stable synthetic case ID.
    pub case_id: &'static str,
    /// Owned evaluation category.
    pub category: EvalCategoryV1,
    /// Public synthetic task description.
    pub task: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Versioned public catalog for the fixed local harness suite.
pub struct EvalCatalogV1 {
    /// Catalog schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this catalog.
    pub state_slice: &'static str,
    /// Stable suite ID.
    pub suite_id: &'static str,
    /// Digest binding suite metadata, tasks, and rubric version.
    pub suite_digest: Sha256Digest,
    /// Public case summaries; no answer fixture codes are returned.
    pub cases: Vec<EvalCaseSummaryV1>,
    /// Stable fixture pair available to the local harness.
    pub pair_id: &'static str,
    /// Baseline fixture identity.
    pub control_fixture_id: &'static str,
    /// Comparison fixture identity.
    pub alternate_fixture_id: &'static str,
    /// Number of fixed sequential repeats.
    pub repeat_count: usize,
    /// Explicit score claim ceiling.
    pub claim_ceiling: &'static str,
    /// Explicit unsupported claims.
    pub nonclaims: &'static str,
    /// Execution and custody boundary.
    pub effect_boundary: &'static str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Authenticated request to score the fixed suite/pair.
pub struct EvalRunRequestV1 {
    /// Stable suite ID shown by the catalog.
    pub suite_id: String,
    /// Exact suite digest shown by the catalog.
    pub expected_suite_digest: Sha256Digest,
    /// Fixed comparison pair ID.
    pub pair_id: String,
    /// Caller-selected bounded replay key.
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Strict private stdin contract between the service and scorer process.
pub struct EvalScorerRequestV1 {
    /// Exact suite digest frozen by the caller.
    pub expected_suite_digest: Sha256Digest,
    /// Fixed pair; arbitrary candidates are not accepted.
    pub pair_id: String,
    /// Fixed deterministic seed label; it does not seed a model.
    pub seed_label: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Paired exact-match result for one synthetic case.
pub struct EvalCaseOutcomeV1 {
    /// Stable synthetic case ID.
    pub case_id: String,
    /// Whether each static fixture code matches the owned synthetic rubric.
    pub control_matches: bool,
    /// Whether each static fixture code matches the owned synthetic rubric.
    pub alternate_matches: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Aggregate exact-match result for one fixed answer fixture.
pub struct EvalFixtureScoreV1 {
    /// Fixed fixture identity, never a model or candidate ID.
    pub fixture_id: String,
    /// Number of exact synthetic rubric matches.
    pub matching_cases: usize,
    /// Number of fixed synthetic cases.
    pub case_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Strict scorer-child output; contains no expected rubric codes.
pub struct EvalScorerOutputV1 {
    /// Private child scorer output schema.
    pub schema_version: String,
    /// Exact suite digest.
    pub suite_digest: Sha256Digest,
    /// Fixed pair identity.
    pub pair_id: String,
    /// Paired case outcomes.
    pub cases: Vec<EvalCaseOutcomeV1>,
    /// Aggregate control result.
    pub control: EvalFixtureScoreV1,
    /// Aggregate alternate result.
    pub alternate: EvalFixtureScoreV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Public run report with bounded process and payload measurements.
pub struct EvalRunReportV1 {
    /// Public report schema.
    pub schema_version: &'static str,
    /// State slice that owns this run.
    pub state_slice: &'static str,
    /// Fixed synthetic suite ID.
    pub suite_id: &'static str,
    /// Exact suite digest.
    pub suite_digest: Sha256Digest,
    /// Fixed fixture-pair ID.
    pub pair_id: String,
    /// Digest of identical scorer content across three child invocations.
    pub repeated_result_digest: Sha256Digest,
    /// Three sequential repeat measurements in milliseconds.
    pub child_wall_time_ms: Vec<u64>,
    /// Serialized bytes sent to each scorer child.
    pub scorer_input_bytes: usize,
    /// Serialized bytes returned by each scorer child.
    pub scorer_output_bytes: usize,
    /// Per-case fixture matches; no expected answer codes are returned.
    pub cases: Vec<EvalCaseOutcomeV1>,
    /// Aggregate fixed control fixture score.
    pub control: EvalFixtureScoreV1,
    /// Aggregate fixed alternate fixture score.
    pub alternate: EvalFixtureScoreV1,
    /// Measurement classification.
    pub measurement_type: &'static str,
    /// Explicit synthetic effect boundary.
    pub effect_boundary: &'static str,
    /// Explicit claim ceiling.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
}

#[derive(Serialize)]
struct SuiteIdentityV1 {
    suite_id: &'static str,
    state_slice: &'static str,
    rubric_version: &'static str,
    cases: &'static [EvalCaseV1],
}

#[derive(Clone, Copy, Serialize)]
struct EvalCaseV1 {
    case_id: &'static str,
    category: EvalCategoryV1,
    task: &'static str,
    expected_code: &'static str,
}

/// Return the stable public catalog without fixture answer codes.
///
/// # Errors
///
/// Returns an error if the frozen catalog cannot be canonically hashed.
pub fn eval_catalog_v1() -> Result<EvalCatalogV1, EvalError> {
    let suite_digest = suite_digest_v1()?;
    Ok(EvalCatalogV1 {
        schema_version: "hyperai.eval.catalog.v1",
        state_slice: EVAL_STATE_SLICE_V1,
        suite_id: EVAL_SUITE_ID_V1,
        suite_digest,
        cases: CASES
            .iter()
            .map(|case| EvalCaseSummaryV1 {
                case_id: case.case_id,
                category: case.category,
                task: case.task,
            })
            .collect(),
        pair_id: EVAL_PAIR_ID_V1,
        control_fixture_id: "control-fixture-v1",
        alternate_fixture_id: "alternate-fixture-v1",
        repeat_count: EVAL_REPEAT_COUNT_V1,
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
        effect_boundary: "synthetic_fixed_fixtures_no_model_no_network_volatile_report",
    })
}

/// Validate the fixed scorer input and return the deterministic paired result.
///
/// # Errors
///
/// Returns `InvalidScorerRequest` for any suite, digest, pair, or seed mismatch.
pub fn score_fixture_pair_v1(
    request: &EvalScorerRequestV1,
) -> Result<EvalScorerOutputV1, EvalError> {
    if request.expected_suite_digest != suite_digest_v1()?
        || request.pair_id != EVAL_PAIR_ID_V1
        || request.seed_label != EVAL_SEED_LABEL_V1
    {
        return Err(EvalError::InvalidScorerRequest);
    }
    let cases = CASES
        .iter()
        .enumerate()
        .map(|(index, case)| EvalCaseOutcomeV1 {
            case_id: case.case_id.to_owned(),
            control_matches: CONTROL_CODES[index] == case.expected_code,
            alternate_matches: ALTERNATE_CODES[index] == case.expected_code,
        })
        .collect::<Vec<_>>();
    Ok(EvalScorerOutputV1 {
        schema_version: "hyperai.eval.scorer-output.v1".to_owned(),
        suite_digest: request.expected_suite_digest,
        pair_id: EVAL_PAIR_ID_V1.to_owned(),
        control: EvalFixtureScoreV1 {
            fixture_id: "control-fixture-v1".to_owned(),
            matching_cases: cases.iter().filter(|case| case.control_matches).count(),
            case_count: CASES.len(),
        },
        alternate: EvalFixtureScoreV1 {
            fixture_id: "alternate-fixture-v1".to_owned(),
            matching_cases: cases.iter().filter(|case| case.alternate_matches).count(),
            case_count: CASES.len(),
        },
        cases,
    })
}

/// Validate the scorer child's strict output shape and paired aggregates.
///
/// # Errors
///
/// Returns `InvalidScorerRequest` when identities, case order, fixture labels,
/// or per-case aggregates differ from the fixed suite contract.
pub fn validate_scorer_output_v1(
    output: &EvalScorerOutputV1,
    expected_suite_digest: Sha256Digest,
    expected_pair_id: &str,
) -> Result<(), EvalError> {
    let catalog = eval_catalog_v1()?;
    let control_matches = output
        .cases
        .iter()
        .filter(|case| case.control_matches)
        .count();
    let alternate_matches = output
        .cases
        .iter()
        .filter(|case| case.alternate_matches)
        .count();
    if output.schema_version != "hyperai.eval.scorer-output.v1"
        || output.suite_digest != expected_suite_digest
        || output.suite_digest != catalog.suite_digest
        || output.pair_id != expected_pair_id
        || output.pair_id != EVAL_PAIR_ID_V1
        || output.cases.len() != CASES.len()
        || output
            .cases
            .iter()
            .zip(CASES.iter())
            .any(|(actual, expected)| actual.case_id != expected.case_id)
        || output.control.fixture_id != "control-fixture-v1"
        || output.control.case_count != CASES.len()
        || output.control.matching_cases != control_matches
        || output.alternate.fixture_id != "alternate-fixture-v1"
        || output.alternate.case_count != CASES.len()
        || output.alternate.matching_cases != alternate_matches
    {
        return Err(EvalError::InvalidScorerRequest);
    }
    Ok(())
}

/// Bind the run request to its idempotency key for local replay detection.
///
/// # Errors
///
/// Returns an error if the request cannot be canonically hashed.
pub fn eval_run_request_digest_v1(request: &EvalRunRequestV1) -> Result<Sha256Digest, EvalError> {
    Ok(domain_separated_digest(
        "hyperai.eval.run-request.v1",
        request,
    )?)
}

/// Check suite, pair, and idempotency fields before starting the scorer.
///
/// # Errors
///
/// Returns `InvalidScorerRequest` for a stale suite, unknown pair, or invalid
/// replay key.
pub fn validate_eval_run_request_v1(request: &EvalRunRequestV1) -> Result<(), EvalError> {
    if request.suite_id != EVAL_SUITE_ID_V1
        || request.expected_suite_digest != suite_digest_v1()?
        || request.pair_id != EVAL_PAIR_ID_V1
        || request.idempotency_key.is_empty()
        || request.idempotency_key.len() > 64
        || !request
            .idempotency_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(EvalError::InvalidScorerRequest);
    }
    Ok(())
}

/// Construct the public report from one stable child output and measurements.
///
/// # Errors
///
/// Returns an error if the child result cannot be canonically hashed.
pub fn eval_run_report_v1(
    output: EvalScorerOutputV1,
    child_wall_time_ms: Vec<u64>,
    scorer_input_bytes: usize,
    scorer_output_bytes: usize,
) -> Result<EvalRunReportV1, EvalError> {
    let repeated_result_digest =
        domain_separated_digest("hyperai.eval.repeated-scorer-result.v1", &output)?;
    Ok(EvalRunReportV1 {
        schema_version: "hyperai.eval.run-report.v1",
        state_slice: EVAL_STATE_SLICE_V1,
        suite_id: EVAL_SUITE_ID_V1,
        suite_digest: output.suite_digest,
        pair_id: output.pair_id,
        repeated_result_digest,
        child_wall_time_ms,
        scorer_input_bytes,
        scorer_output_bytes,
        cases: output.cases,
        control: output.control,
        alternate: output.alternate,
        measurement_type: "synthetic_harness_qualification_only",
        effect_boundary: "synthetic_fixed_fixtures_no_model_no_network_volatile_report",
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
    })
}

/// Read the fixed scorer protocol from stdin and emit one compact JSON result.
///
/// # Errors
///
/// Returns an error for oversized input, invalid schema, or protocol I/O.
pub fn run_scorer_stdio_v1() -> Result<(), EvalError> {
    let mut input = Vec::with_capacity(MAX_SCORER_INPUT_BYTES_V1);
    std::io::stdin()
        .lock()
        .take(u64::try_from(MAX_SCORER_INPUT_BYTES_V1 + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut input)?;
    if input.len() > MAX_SCORER_INPUT_BYTES_V1 {
        return Err(EvalError::InvalidScorerRequest);
    }
    let output = score_scorer_input_bytes_v1(&input)?;
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&output)?;
    stdout.flush()?;
    Ok(())
}

fn score_scorer_input_bytes_v1(input: &[u8]) -> Result<Vec<u8>, EvalError> {
    if input.len() > MAX_SCORER_INPUT_BYTES_V1 {
        return Err(EvalError::InvalidScorerRequest);
    }
    let request: EvalScorerRequestV1 =
        serde_json::from_slice(input).map_err(|_| EvalError::InvalidScorerRequest)?;
    let output = score_fixture_pair_v1(&request)?;
    let mut bytes = Vec::with_capacity(1024);
    serde_json::to_writer(&mut bytes, &output)?;
    Ok(bytes)
}

fn suite_digest_v1() -> Result<Sha256Digest, EvalError> {
    Ok(domain_separated_digest(
        "hyperai.eval.synthetic-suite.v1",
        &SuiteIdentityV1 {
            suite_id: EVAL_SUITE_ID_V1,
            state_slice: EVAL_STATE_SLICE_V1,
            rubric_version: "synthetic-exact-code-rubric-v1",
            cases: &CASES,
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn catalog_is_stable_and_does_not_expose_expected_codes()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = eval_catalog_v1()?;
        let repeated = eval_catalog_v1()?;
        let json = serde_json::to_string(&catalog)?;
        assert_eq!(catalog, repeated);
        assert_eq!(catalog.cases.len(), 6);
        assert!(!json.contains("expected_code"));
        assert!(!json.contains("constraints_preserved"));
        Ok(())
    }

    #[test]
    fn scorer_returns_fixed_paired_fixture_labels_and_aggregates()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = eval_catalog_v1()?;
        let output = score_fixture_pair_v1(&EvalScorerRequestV1 {
            expected_suite_digest: catalog.suite_digest,
            pair_id: EVAL_PAIR_ID_V1.to_owned(),
            seed_label: EVAL_SEED_LABEL_V1.to_owned(),
        })?;
        assert_eq!(output.cases.len(), 6);
        assert_eq!(output.control.matching_cases, 6);
        assert_eq!(output.alternate.matching_cases, 3);
        Ok(())
    }

    #[test]
    fn scorer_rejects_changed_suite_pair_or_seed() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = eval_catalog_v1()?;
        for request in [
            EvalScorerRequestV1 {
                expected_suite_digest: Sha256Digest::from_bytes([0; 32]),
                pair_id: EVAL_PAIR_ID_V1.to_owned(),
                seed_label: EVAL_SEED_LABEL_V1.to_owned(),
            },
            EvalScorerRequestV1 {
                expected_suite_digest: catalog.suite_digest,
                pair_id: "arbitrary-pair".to_owned(),
                seed_label: EVAL_SEED_LABEL_V1.to_owned(),
            },
            EvalScorerRequestV1 {
                expected_suite_digest: catalog.suite_digest,
                pair_id: EVAL_PAIR_ID_V1.to_owned(),
                seed_label: "unrecognized-seed".to_owned(),
            },
        ] {
            assert!(matches!(
                score_fixture_pair_v1(&request),
                Err(EvalError::InvalidScorerRequest)
            ));
        }
        Ok(())
    }

    #[test]
    fn output_validator_rejects_reordered_cases_and_inconsistent_aggregates()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = eval_catalog_v1()?;
        let request = EvalScorerRequestV1 {
            expected_suite_digest: catalog.suite_digest,
            pair_id: EVAL_PAIR_ID_V1.to_owned(),
            seed_label: EVAL_SEED_LABEL_V1.to_owned(),
        };
        let mut output = score_fixture_pair_v1(&request)?;
        assert!(validate_scorer_output_v1(&output, catalog.suite_digest, EVAL_PAIR_ID_V1).is_ok());

        output.cases.swap(0, 1);
        assert!(matches!(
            validate_scorer_output_v1(&output, catalog.suite_digest, EVAL_PAIR_ID_V1),
            Err(EvalError::InvalidScorerRequest)
        ));

        let mut output = score_fixture_pair_v1(&request)?;
        output.control.matching_cases -= 1;
        assert!(matches!(
            validate_scorer_output_v1(&output, catalog.suite_digest, EVAL_PAIR_ID_V1),
            Err(EvalError::InvalidScorerRequest)
        ));
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            rng_seed: proptest::test_runner::RngSeed::Fixed(20_260_924),
            ..ProptestConfig::default()
        })]

        #[test]
        fn arbitrary_scorer_bytes_fail_closed_with_bounded_output(
            input in prop::collection::vec(any::<u8>(), 0..=MAX_SCORER_INPUT_BYTES_V1 + 32)
        ) {
            if let Ok(output_bytes) = score_scorer_input_bytes_v1(&input) {
                prop_assert!(input.len() <= MAX_SCORER_INPUT_BYTES_V1);
                prop_assert!(output_bytes.len() <= MAX_SCORER_OUTPUT_BYTES_V1);
                let output: EvalScorerOutputV1 = match serde_json::from_slice(&output_bytes) {
                    Ok(output) => output,
                    Err(error) => return Err(TestCaseError::fail(error.to_string())),
                };
                prop_assert!(validate_scorer_output_v1(
                    &output,
                    output.suite_digest,
                    EVAL_PAIR_ID_V1,
                ).is_ok());
            }
        }
    }
}
