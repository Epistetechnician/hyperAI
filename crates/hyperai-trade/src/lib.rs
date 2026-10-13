//! Deterministic research calculations over a versioned synthetic Data release.
//!
//! This crate does not provide financial instruments, orders, fills, or price
//! oracles. Asking prices and capacities remain synthetic observations.

use std::collections::{BTreeMap, BTreeSet};

use hyperai_data::{
    DataError, SyntheticMarketTimeSeriesPointV1, SyntheticMarketTimeSeriesV1,
    synthetic_market_time_series_v1,
};
use hyperai_foundation::{CanonicalizationError, Sha256Digest, domain_separated_digest};
use serde::{Deserialize, Serialize};

/// State slice that owns the local Trade research workspace.
pub const TRADE_STATE_SLICE_V1: &str = "hyperai.bazaar.trade_research.v1";
/// Stable identity for the synthetic regional hourly dataset.
pub const TRADE_DATASET_ID_V1: &str = "hyperai.synthetic.compute-hourly-series.v1";
/// Series-analysis method version.
pub const TRADE_SERIES_METHOD_V1: &str = "hyperai.trade.contiguous-step-variability.v1";
/// Regional asking-price basis method version.
pub const TRADE_BASIS_METHOD_V1: &str = "hyperai.trade.regional-asking-basis.v1";
/// Deterministic scenario method version.
pub const TRADE_SCENARIO_METHOD_V1: &str = "hyperai.trade.synthetic-shock-scenario.v1";
/// Deterministic paper procurement-cost replay method version.
pub const TRADE_PAPER_METHOD_V1: &str = "hyperai.trade.cheapest-known-capacity-replay.v1";

const MAX_PRICE_SHOCK_BPS: i32 = 5_000;
const MAX_AVAILABILITY_DELTA: i64 = 10;
const MAX_REQUESTED_UNITS: u64 = 8;
const BPS_DENOMINATOR: u128 = 10_000;
const CLAIM_CEILING: &str =
    "Deterministic research plumbing over one HyperAI-authored synthetic series release only.";
const NONCLAIMS: &str = "No live market observation, tradable index, forecast, seller availability, executed fill, realized saving, trade return, payment entitlement, or independent verification is established.";

/// Trade research contract errors.
#[derive(Debug, thiserror::Error)]
pub enum TradeError {
    /// The compiled-in Data source could not be created.
    #[error(transparent)]
    Data(#[from] DataError),
    /// The requested data release digest is stale or does not match.
    #[error("the requested synthetic dataset digest does not match")]
    DatasetDigestMismatch,
    /// The instrument is not in the fixed synthetic research catalog.
    #[error("the research instrument is not in the catalog")]
    InstrumentNotFound,
    /// The instruments do not support a same-model regional comparison.
    #[error("the requested instruments cannot be compared")]
    IncompatibleInstruments,
    /// A request exceeds the frozen local research contract.
    #[error("the research request is invalid")]
    InvalidRequest,
    /// Checked integer arithmetic exceeded the report representation.
    #[error("the research calculation exceeded its integer bounds")]
    ArithmeticOverflow,
    /// A versioned research result could not be canonically identified.
    #[error(transparent)]
    Digest(#[from] CanonicalizationError),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// One research-only GPU/region label from the fixed synthetic release.
pub struct TradeInstrumentV1 {
    /// Stable research key; this is not a tradable instrument identifier.
    pub instrument_id: String,
    /// GPU model held constant for regional basis comparisons.
    pub gpu_model: String,
    /// Synthetic region label.
    pub region: String,
    /// Unit for source prices.
    pub price_unit: &'static str,
    /// Explicitly false for every record in this catalog.
    pub tradable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Per-instrument fixture composition and expected sampling coverage.
pub struct TradeFixtureCompositionV1 {
    /// Research instrument key.
    pub instrument_id: String,
    /// Number of source rows in this instrument's fixture series.
    pub observed_rows: usize,
    /// Share of all rows in this synthetic fixture, in basis points.
    pub fixture_row_share_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Versioned Trade workspace catalog, methods, and non-authoritative source.
pub struct TradeCatalogV1 {
    /// Response schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this workspace.
    pub state_slice: &'static str,
    /// Stable source dataset identifier.
    pub dataset_id: &'static str,
    /// Exact Data release revision.
    pub dataset_revision: &'static str,
    /// Digest binding the exact Data release.
    pub dataset_digest: Sha256Digest,
    /// `HyperAI` source description.
    pub source_label: &'static str,
    /// Synthetic freshness label.
    pub freshness: &'static str,
    /// Source revision policy.
    pub revision_policy: &'static str,
    /// Fixed sample interval in seconds.
    pub interval_seconds: u32,
    /// Expected grid slots per instrument.
    pub expected_period_count: u8,
    /// Number of observed rows in the fixed source release.
    pub observed_row_count: usize,
    /// Number of rows expected at full fixture coverage.
    pub expected_row_count: usize,
    /// Total observed-row coverage across the fixed fixture, in basis points.
    pub fixture_coverage_bps: u64,
    /// Research instruments derived from the Data-owned records.
    pub instruments: Vec<TradeInstrumentV1>,
    /// Per-instrument composition of this fixture, not market participation.
    pub fixture_composition: Vec<TradeFixtureCompositionV1>,
    /// HHI-style concentration of rows in this fixture, in basis points.
    pub fixture_composition_hhi_bps: u64,
    /// Versioned methods available through this workspace.
    pub method_versions: Vec<&'static str>,
    /// Explicit claim ceiling.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
    /// External-effect boundary.
    pub effect_boundary: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Descriptive, gap-aware analysis of one source series.
pub struct TradeSeriesReportV1 {
    /// Response schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this report.
    pub state_slice: &'static str,
    /// Source dataset identifier.
    pub dataset_id: &'static str,
    /// Exact source release digest.
    pub dataset_digest: Sha256Digest,
    /// Stable series-analysis method version.
    pub method_version: &'static str,
    /// Research instrument metadata.
    pub instrument: TradeInstrumentV1,
    /// Expected sample slots.
    pub expected_period_count: u8,
    /// Observed price rows.
    pub observed_period_count: usize,
    /// Observed slots divided by expected slots, in basis points.
    pub price_coverage_bps: u64,
    /// Source rows in timestamp order; absent periods remain listed below.
    pub observations: Vec<SyntheticMarketTimeSeriesPointV1>,
    /// UTC slots without an asking-price observation.
    pub missing_periods: Vec<String>,
    /// Number of rows with known availability.
    pub known_availability_count: usize,
    /// Number of rows where availability is unknown.
    pub unknown_availability_count: usize,
    /// Number of known zero-capacity rows.
    pub zero_availability_count: usize,
    /// Mean absolute price change across adjacent observed hourly pairs.
    pub mean_absolute_contiguous_change_bps: Option<u64>,
    /// Number of adjacent one-hour pairs included in the descriptive metric.
    pub contiguous_change_sample_count: usize,
    /// Metric interpretation.
    pub metric_label: &'static str,
    /// Explicit claim ceiling.
    pub claim_ceiling: &'static str,
    /// External-effect boundary.
    pub effect_boundary: &'static str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Bounded price and capacity assumptions for one local scenario.
pub struct TradeScenarioRequestV1 {
    /// Exact synthetic release digest from the catalog.
    pub expected_dataset_digest: Sha256Digest,
    /// Primary research series to shock.
    pub primary_instrument_id: String,
    /// Same-model regional reference series.
    pub comparison_instrument_id: String,
    /// Assumed percentage price change in basis points, bounded to plus/minus 50%.
    pub price_shock_bps: i32,
    /// Assumed capacity change in units, bounded to plus/minus ten.
    pub availability_delta_units: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Deterministic sensitivity checks for the latest complete regional sample.
pub struct TradeSensitivityV1 {
    /// Latest complete cross-region UTC slot.
    pub latest_complete_period: String,
    /// Median asking price at the latest complete slot.
    pub reference_median_usd_micros_per_hour: u64,
    /// Instrument carrying the highest input quote in that slot.
    pub stressed_instrument_id: String,
    /// Price multiplier applied to that quote, in basis points.
    pub stressed_quote_change_bps: u32,
    /// Median after the high quote shock.
    pub stressed_reference_median_usd_micros_per_hour: u64,
    /// Difference between the stressed and original reference medians.
    pub outlier_median_change_usd_micros_per_hour: i64,
    /// Immediately previous complete UTC slot.
    pub prior_complete_period: String,
    /// Median at the prior complete slot.
    pub prior_reference_median_usd_micros_per_hour: u64,
    /// Latest median minus prior median; a lag sensitivity, not a freshness decision.
    pub one_period_lag_change_usd_micros_per_hour: i64,
    /// Explicit interpretation of these fixture-only checks.
    pub method_label: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Local asking-price scenario result; no action or durable record is created.
pub struct TradeScenarioReportV1 {
    /// Response schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this report.
    pub state_slice: &'static str,
    /// Exact source release digest.
    pub dataset_digest: Sha256Digest,
    /// Digest binding request, source, method, and result.
    pub scenario_digest: Sha256Digest,
    /// Scenario calculation method version.
    pub method_version: &'static str,
    /// Basis method version.
    pub basis_method_version: &'static str,
    /// Primary research instrument.
    pub primary_instrument: TradeInstrumentV1,
    /// Regional reference instrument.
    pub comparison_instrument: TradeInstrumentV1,
    /// Latest common UTC observation time.
    pub observed_at: String,
    /// Number of common observed periods across the full series.
    pub common_period_count: usize,
    /// Common periods divided by expected periods, in basis points.
    pub common_coverage_bps: u64,
    /// Primary observed asking price at the latest common period.
    pub primary_asking_usd_micros_per_hour: u64,
    /// Comparison observed asking price at the latest common period.
    pub comparison_asking_usd_micros_per_hour: u64,
    /// Primary minus comparison price, in USD micros per GPU-hour.
    pub regional_basis_usd_micros_per_hour: i64,
    /// Assumed price shock from the request.
    pub assumed_price_shock_bps: i32,
    /// Shocked primary asking-price assumption using floor integer rounding.
    pub scenario_primary_asking_usd_micros_per_hour: u64,
    /// Primary scenario price minus comparison price.
    pub scenario_regional_basis_usd_micros_per_hour: i64,
    /// Base availability; unknown remains absent.
    pub primary_available_units: Option<u64>,
    /// Assumed availability delta.
    pub assumed_availability_delta_units: i64,
    /// Scenario availability; remains unknown when the source is unknown.
    pub scenario_available_units: Option<u64>,
    /// Descriptive price variability for the primary source series.
    pub mean_absolute_contiguous_change_bps: Option<u64>,
    /// Outlier and lag sensitivity over the fixed regional sample.
    pub sensitivity: TradeSensitivityV1,
    /// Explicit synthetic/advisory classification.
    pub measurement_type: &'static str,
    /// Explicit claim ceiling.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
    /// External-effect boundary.
    pub effect_boundary: &'static str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Request for the fixed one-unit-cost-method paper replay.
pub struct TradePaperReplayRequestV1 {
    /// Exact synthetic release digest from the catalog.
    pub expected_dataset_digest: Sha256Digest,
    /// Fixed research series used as the comparison baseline.
    pub baseline_instrument_id: String,
    /// Requested hypothetical GPU units per hour, bounded to one through eight.
    pub requested_units: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Status for one hourly procurement-cost comparison slot.
pub enum TradePaperPeriodStatusV1 {
    /// Both strategy and baseline have known sufficient capacity and price.
    Comparable,
    /// The named baseline has no source price in this slot.
    MissingBaselinePrice,
    /// The named baseline capacity is unknown.
    BaselineAvailabilityUnknown,
    /// The baseline has fewer units than requested.
    BaselineInsufficientCapacity,
    /// No region has known sufficient capacity and a price.
    NoEligibleCapacity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// One bounded slot in the deterministic procurement-cost replay.
pub struct TradePaperPeriodV1 {
    /// UTC time slot.
    pub observed_at: String,
    /// Status for the paired cost comparison.
    pub status: TradePaperPeriodStatusV1,
    /// Lowest known sufficiently available region, when present.
    pub selected_instrument_id: Option<String>,
    /// Selected synthetic asking price, when present.
    pub selected_asking_usd_micros_per_hour: Option<u64>,
    /// Baseline synthetic asking price, when present.
    pub baseline_asking_usd_micros_per_hour: Option<u64>,
    /// Baseline asking cost minus selected asking cost for requested units.
    pub baseline_minus_strategy_asking_cost_microusd: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// Deterministic paper procurement-cost replay, with no simulated fill.
pub struct TradePaperReplayReportV1 {
    /// Response schema identifier.
    pub schema_version: &'static str,
    /// State slice that owns this report.
    pub state_slice: &'static str,
    /// Exact source release digest.
    pub dataset_digest: Sha256Digest,
    /// Digest binding request, source, method, and rows.
    pub replay_digest: Sha256Digest,
    /// Paper method version.
    pub method_version: &'static str,
    /// Fixed tie-breaking rule.
    pub tie_breaking: &'static str,
    /// Baseline research instrument.
    pub baseline_instrument: TradeInstrumentV1,
    /// Requested hypothetical units.
    pub requested_units: u64,
    /// Count of paired periods included in aggregate asking costs.
    pub comparable_period_count: usize,
    /// Count of periods with no eligible strategy capacity.
    pub no_eligible_capacity_count: usize,
    /// Count of absent regional price observations across the input grid.
    pub missing_price_observation_count: usize,
    /// Count of observed records whose availability is unknown.
    pub unknown_capacity_observation_count: usize,
    /// Count of observed records below the requested capacity.
    pub insufficient_capacity_observation_count: usize,
    /// Count of periods with unknown baseline capacity.
    pub unknown_baseline_capacity_count: usize,
    /// Count of periods where baseline capacity is below request.
    pub insufficient_baseline_capacity_count: usize,
    /// Count of periods with no baseline asking-price observation.
    pub missing_baseline_price_count: usize,
    /// Baseline sum on comparable periods, in integer USD micros.
    pub baseline_asking_cost_microusd: u64,
    /// Strategy sum on the same comparable periods, in integer USD micros.
    pub strategy_asking_cost_microusd: u64,
    /// Baseline asking cost minus strategy asking cost, signed integer micros.
    pub baseline_minus_strategy_asking_cost_microusd: i64,
    /// Per-period selection and exclusion records.
    pub periods: Vec<TradePaperPeriodV1>,
    /// Explicit interpretation of the paper result.
    pub measurement_type: &'static str,
    /// Explicit claim ceiling.
    pub claim_ceiling: &'static str,
    /// Explicit nonclaims.
    pub nonclaims: &'static str,
    /// External-effect boundary.
    pub effect_boundary: &'static str,
}

/// Return the bounded research catalog over the Data-owned synthetic release.
///
/// # Errors
///
/// Returns an error if the fixed Data release or its canonical digest fails.
pub fn trade_catalog_v1() -> Result<TradeCatalogV1, TradeError> {
    let release = synthetic_market_time_series_v1()?;
    let instruments = instruments_v1(&release)?;
    let expected_row_count = instruments
        .len()
        .checked_mul(usize::from(release.expected_period_count))
        .ok_or(TradeError::ArithmeticOverflow)?;
    let observed_row_count = release.observations.len();
    let fixture_coverage_bps = ratio_bps(observed_row_count, expected_row_count)?;
    let mut fixture_composition = Vec::with_capacity(instruments.len());
    let mut hhi_numerator = 0_u128;
    for instrument in &instruments {
        let observed_rows = release
            .observations
            .iter()
            .filter(|point| point.instrument_id == instrument.instrument_id)
            .count();
        let fixture_row_share_bps = ratio_bps(observed_rows, observed_row_count)?;
        hhi_numerator = hhi_numerator
            .checked_add(u128::from(fixture_row_share_bps).pow(2))
            .ok_or(TradeError::ArithmeticOverflow)?;
        fixture_composition.push(TradeFixtureCompositionV1 {
            instrument_id: instrument.instrument_id.clone(),
            observed_rows,
            fixture_row_share_bps,
        });
    }
    let fixture_composition_hhi_bps = u64::try_from(hhi_numerator / BPS_DENOMINATOR)
        .map_err(|_| TradeError::ArithmeticOverflow)?;

    Ok(TradeCatalogV1 {
        schema_version: "hyperai.trade.catalog.v1",
        state_slice: TRADE_STATE_SLICE_V1,
        dataset_id: release.dataset_id,
        dataset_revision: release.dataset_revision,
        dataset_digest: release.snapshot_digest,
        source_label: release.source_label,
        freshness: release.freshness,
        revision_policy: release.revision_policy,
        interval_seconds: release.interval_seconds,
        expected_period_count: release.expected_period_count,
        observed_row_count,
        expected_row_count,
        fixture_coverage_bps,
        instruments,
        fixture_composition,
        fixture_composition_hhi_bps,
        method_versions: vec![
            TRADE_SERIES_METHOD_V1,
            TRADE_BASIS_METHOD_V1,
            TRADE_SCENARIO_METHOD_V1,
            TRADE_PAPER_METHOD_V1,
        ],
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
        effect_boundary: release.effect_boundary,
    })
}

/// Return a gap-aware series report for one fixed research instrument.
///
/// # Errors
///
/// Returns an error for an unknown instrument or invalid bounded calculation.
pub fn trade_series_v1(instrument_id: &str) -> Result<TradeSeriesReportV1, TradeError> {
    let release = synthetic_market_time_series_v1()?;
    let instrument = instruments_v1(&release)?
        .into_iter()
        .find(|instrument| instrument.instrument_id == instrument_id)
        .ok_or(TradeError::InstrumentNotFound)?;
    let mut observations = release
        .observations
        .iter()
        .filter(|point| point.instrument_id == instrument_id)
        .cloned()
        .collect::<Vec<_>>();
    observations.sort_by_key(|point| point.period_index);
    let observed_period_count = observations.len();
    let price_coverage_bps = ratio_bps(
        observed_period_count,
        usize::from(release.expected_period_count),
    )?;
    let observed_periods = observations
        .iter()
        .map(|point| point.period_index)
        .collect::<BTreeSet<_>>();
    let missing_periods = release
        .periods
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            u8::try_from(*index).is_ok_and(|period| !observed_periods.contains(&period))
        })
        .map(|(_, period)| period.clone())
        .collect::<Vec<_>>();
    let known_availability_count = observations
        .iter()
        .filter(|point| point.available_count.is_some())
        .count();
    let unknown_availability_count = observed_period_count - known_availability_count;
    let zero_availability_count = observations
        .iter()
        .filter(|point| point.available_count == Some(0))
        .count();
    let (mean_absolute_contiguous_change_bps, contiguous_change_sample_count) =
        contiguous_change_metric(&observations)?;

    Ok(TradeSeriesReportV1 {
        schema_version: "hyperai.trade.series.v1",
        state_slice: TRADE_STATE_SLICE_V1,
        dataset_id: release.dataset_id,
        dataset_digest: release.snapshot_digest,
        method_version: TRADE_SERIES_METHOD_V1,
        instrument,
        expected_period_count: release.expected_period_count,
        observed_period_count,
        price_coverage_bps,
        observations,
        missing_periods,
        known_availability_count,
        unknown_availability_count,
        zero_availability_count,
        mean_absolute_contiguous_change_bps,
        contiguous_change_sample_count,
        metric_label: "mean_absolute_contiguous_hourly_change_bps_not_forecast_volatility",
        claim_ceiling: CLAIM_CEILING,
        effect_boundary: release.effect_boundary,
    })
}

/// Compute a bounded regional price-shock and capacity scenario.
///
/// # Errors
///
/// Returns an error for a stale digest, incompatible instruments, invalid
/// assumptions, or checked-arithmetic overflow.
pub fn trade_scenario_v1(
    request: &TradeScenarioRequestV1,
) -> Result<TradeScenarioReportV1, TradeError> {
    let release = synthetic_market_time_series_v1()?;
    validate_dataset_digest(&release, request.expected_dataset_digest)?;
    validate_scenario_request(request)?;
    let instruments = instruments_v1(&release)?;
    let primary_instrument = find_instrument(&instruments, &request.primary_instrument_id)?;
    let comparison_instrument = find_instrument(&instruments, &request.comparison_instrument_id)?;
    if primary_instrument.gpu_model != comparison_instrument.gpu_model
        || primary_instrument.instrument_id == comparison_instrument.instrument_id
    {
        return Err(TradeError::IncompatibleInstruments);
    }
    let common = common_observations(
        &release,
        &primary_instrument.instrument_id,
        &comparison_instrument.instrument_id,
    );
    let (latest_primary, latest_comparison) = common
        .last()
        .copied()
        .ok_or(TradeError::IncompatibleInstruments)?;
    let regional_basis = signed_difference(
        latest_primary.asking_usd_micros_per_hour,
        latest_comparison.asking_usd_micros_per_hour,
    )?;
    let scenario_primary_price = apply_price_shock(
        latest_primary.asking_usd_micros_per_hour,
        request.price_shock_bps,
    )?;
    let scenario_basis = signed_difference(
        scenario_primary_price,
        latest_comparison.asking_usd_micros_per_hour,
    )?;
    let scenario_available_units = apply_availability_delta(
        latest_primary.available_count,
        request.availability_delta_units,
    )?;
    let primary_series = trade_series_v1(&primary_instrument.instrument_id)?;
    let sensitivity = sensitivity_v1(&release, &instruments)?;
    let scenario_digest = scenario_digest(&ScenarioDigestPayload {
        request,
        dataset_digest: release.snapshot_digest,
        observed_at: &latest_primary.observed_at,
        regional_basis,
        scenario_price: scenario_primary_price,
        scenario_basis,
        scenario_available_units,
        sensitivity: &sensitivity,
    })?;

    Ok(TradeScenarioReportV1 {
        schema_version: "hyperai.trade.scenario.v1",
        state_slice: TRADE_STATE_SLICE_V1,
        dataset_digest: release.snapshot_digest,
        scenario_digest,
        method_version: TRADE_SCENARIO_METHOD_V1,
        basis_method_version: TRADE_BASIS_METHOD_V1,
        primary_instrument,
        comparison_instrument,
        observed_at: latest_primary.observed_at.clone(),
        common_period_count: common.len(),
        common_coverage_bps: ratio_bps(common.len(), usize::from(release.expected_period_count))?,
        primary_asking_usd_micros_per_hour: latest_primary.asking_usd_micros_per_hour,
        comparison_asking_usd_micros_per_hour: latest_comparison.asking_usd_micros_per_hour,
        regional_basis_usd_micros_per_hour: regional_basis,
        assumed_price_shock_bps: request.price_shock_bps,
        scenario_primary_asking_usd_micros_per_hour: scenario_primary_price,
        scenario_regional_basis_usd_micros_per_hour: scenario_basis,
        primary_available_units: latest_primary.available_count,
        assumed_availability_delta_units: request.availability_delta_units,
        scenario_available_units,
        mean_absolute_contiguous_change_bps: primary_series.mean_absolute_contiguous_change_bps,
        sensitivity,
        measurement_type: "synthetic_assumption_scenario_not_forecast_or_quote",
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
        effect_boundary: release.effect_boundary,
    })
}

/// Compute the deterministic hourly cheapest-known-capacity paper replay.
///
/// # Errors
///
/// Returns an error for a stale digest, unknown instrument, invalid unit
/// request, or checked-arithmetic overflow.
#[allow(clippy::too_many_lines)] // One deterministic replay keeps selection, skip counts, and digest inputs together.
pub fn trade_paper_replay_v1(
    request: &TradePaperReplayRequestV1,
) -> Result<TradePaperReplayReportV1, TradeError> {
    let release = synthetic_market_time_series_v1()?;
    validate_dataset_digest(&release, request.expected_dataset_digest)?;
    if !(1..=MAX_REQUESTED_UNITS).contains(&request.requested_units) {
        return Err(TradeError::InvalidRequest);
    }
    let instruments = instruments_v1(&release)?;
    let baseline_instrument = find_instrument(&instruments, &request.baseline_instrument_id)?;
    let mut periods = Vec::with_capacity(release.periods.len());
    let mut totals = PaperTotals::default();
    let expected_source_rows = instruments
        .len()
        .checked_mul(usize::from(release.expected_period_count))
        .ok_or(TradeError::ArithmeticOverflow)?;
    let missing_price_observation_count = expected_source_rows
        .checked_sub(release.observations.len())
        .ok_or(TradeError::ArithmeticOverflow)?;
    let unknown_capacity_observation_count = release
        .observations
        .iter()
        .filter(|point| point.available_count.is_none())
        .count();
    let insufficient_capacity_observation_count = release
        .observations
        .iter()
        .filter(|point| {
            point
                .available_count
                .is_some_and(|available| available < request.requested_units)
        })
        .count();

    for (period_index, observed_at) in release.periods.iter().enumerate() {
        let period_index =
            u8::try_from(period_index).map_err(|_| TradeError::ArithmeticOverflow)?;
        let mut candidates = release
            .observations
            .iter()
            .filter(|point| {
                point.period_index == period_index
                    && point
                        .available_count
                        .is_some_and(|available| available >= request.requested_units)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.asking_usd_micros_per_hour
                .cmp(&right.asking_usd_micros_per_hour)
                .then_with(|| left.instrument_id.cmp(&right.instrument_id))
        });
        let selected = candidates.first().copied();
        let baseline = release.observations.iter().find(|point| {
            point.instrument_id == baseline_instrument.instrument_id
                && point.period_index == period_index
        });
        let selected_asking_price = selected.map(|point| point.asking_usd_micros_per_hour);
        let baseline_asking_price = baseline.map(|point| point.asking_usd_micros_per_hour);
        let (status, difference) =
            paper_period_difference(selected, baseline, request.requested_units, &mut totals)?;
        periods.push(TradePaperPeriodV1 {
            observed_at: observed_at.clone(),
            status,
            selected_instrument_id: selected.map(|point| point.instrument_id.clone()),
            selected_asking_usd_micros_per_hour: selected_asking_price,
            baseline_asking_usd_micros_per_hour: baseline_asking_price,
            baseline_minus_strategy_asking_cost_microusd: difference,
        });
    }
    let baseline_minus_strategy_asking_cost_microusd =
        i64::try_from(totals.difference_total).map_err(|_| TradeError::ArithmeticOverflow)?;
    let replay_digest = replay_digest(
        request,
        &release,
        &periods,
        totals.baseline_cost_total,
        totals.strategy_cost_total,
        baseline_minus_strategy_asking_cost_microusd,
    )?;

    Ok(TradePaperReplayReportV1 {
        schema_version: "hyperai.trade.paper-replay.v1",
        state_slice: TRADE_STATE_SLICE_V1,
        dataset_digest: release.snapshot_digest,
        replay_digest,
        method_version: TRADE_PAPER_METHOD_V1,
        tie_breaking: "lowest asking price then ascending instrument_id",
        baseline_instrument,
        requested_units: request.requested_units,
        comparable_period_count: totals.comparable_period_count,
        no_eligible_capacity_count: totals.no_eligible_capacity_count,
        missing_price_observation_count,
        unknown_capacity_observation_count,
        insufficient_capacity_observation_count,
        unknown_baseline_capacity_count: totals.unknown_baseline_capacity_count,
        insufficient_baseline_capacity_count: totals.insufficient_baseline_capacity_count,
        missing_baseline_price_count: totals.missing_baseline_price_count,
        baseline_asking_cost_microusd: totals.baseline_cost_total,
        strategy_asking_cost_microusd: totals.strategy_cost_total,
        baseline_minus_strategy_asking_cost_microusd,
        periods,
        measurement_type: "synthetic_paper_procurement_asking_cost_replay_not_fill_or_pnl",
        claim_ceiling: CLAIM_CEILING,
        nonclaims: NONCLAIMS,
        effect_boundary: release.effect_boundary,
    })
}

fn paper_period_difference(
    selected: Option<&SyntheticMarketTimeSeriesPointV1>,
    baseline: Option<&SyntheticMarketTimeSeriesPointV1>,
    requested_units: u64,
    totals: &mut PaperTotals,
) -> Result<(TradePaperPeriodStatusV1, Option<i64>), TradeError> {
    let Some(selected) = selected else {
        increment(&mut totals.no_eligible_capacity_count)?;
        return Ok((TradePaperPeriodStatusV1::NoEligibleCapacity, None));
    };
    let Some(baseline) = baseline else {
        increment(&mut totals.missing_baseline_price_count)?;
        return Ok((TradePaperPeriodStatusV1::MissingBaselinePrice, None));
    };
    let Some(available) = baseline.available_count else {
        increment(&mut totals.unknown_baseline_capacity_count)?;
        return Ok((TradePaperPeriodStatusV1::BaselineAvailabilityUnknown, None));
    };
    if available < requested_units {
        increment(&mut totals.insufficient_baseline_capacity_count)?;
        return Ok((TradePaperPeriodStatusV1::BaselineInsufficientCapacity, None));
    }
    let baseline_cost = baseline
        .asking_usd_micros_per_hour
        .checked_mul(requested_units)
        .ok_or(TradeError::ArithmeticOverflow)?;
    let strategy_cost = selected
        .asking_usd_micros_per_hour
        .checked_mul(requested_units)
        .ok_or(TradeError::ArithmeticOverflow)?;
    let difference = signed_difference(baseline_cost, strategy_cost)?;
    totals.baseline_cost_total = totals
        .baseline_cost_total
        .checked_add(baseline_cost)
        .ok_or(TradeError::ArithmeticOverflow)?;
    totals.strategy_cost_total = totals
        .strategy_cost_total
        .checked_add(strategy_cost)
        .ok_or(TradeError::ArithmeticOverflow)?;
    totals.difference_total = totals
        .difference_total
        .checked_add(i128::from(difference))
        .ok_or(TradeError::ArithmeticOverflow)?;
    increment(&mut totals.comparable_period_count)?;
    Ok((TradePaperPeriodStatusV1::Comparable, Some(difference)))
}

#[derive(Default)]
struct PaperTotals {
    comparable_period_count: usize,
    no_eligible_capacity_count: usize,
    unknown_baseline_capacity_count: usize,
    insufficient_baseline_capacity_count: usize,
    missing_baseline_price_count: usize,
    baseline_cost_total: u64,
    strategy_cost_total: u64,
    difference_total: i128,
}

#[derive(Serialize)]
struct ScenarioDigestPayload<'a> {
    request: &'a TradeScenarioRequestV1,
    dataset_digest: Sha256Digest,
    observed_at: &'a str,
    regional_basis: i64,
    scenario_price: u64,
    scenario_basis: i64,
    scenario_available_units: Option<u64>,
    sensitivity: &'a TradeSensitivityV1,
}

#[derive(Serialize)]
struct ReplayDigestPayload<'a> {
    request: &'a TradePaperReplayRequestV1,
    dataset_digest: Sha256Digest,
    periods: &'a [TradePaperPeriodV1],
    baseline_cost: u64,
    strategy_cost: u64,
    difference: i64,
}

fn scenario_digest(payload: &ScenarioDigestPayload<'_>) -> Result<Sha256Digest, TradeError> {
    Ok(domain_separated_digest(
        "hyperai.trade.scenario-report.v1",
        payload,
    )?)
}

fn replay_digest(
    request: &TradePaperReplayRequestV1,
    release: &SyntheticMarketTimeSeriesV1,
    periods: &[TradePaperPeriodV1],
    baseline_cost: u64,
    strategy_cost: u64,
    difference: i64,
) -> Result<Sha256Digest, TradeError> {
    Ok(domain_separated_digest(
        "hyperai.trade.paper-replay-report.v1",
        &ReplayDigestPayload {
            request,
            dataset_digest: release.snapshot_digest,
            periods,
            baseline_cost,
            strategy_cost,
            difference,
        },
    )?)
}

fn instruments_v1(
    release: &SyntheticMarketTimeSeriesV1,
) -> Result<Vec<TradeInstrumentV1>, TradeError> {
    let mut instruments = BTreeMap::<String, (String, String)>::new();
    for point in &release.observations {
        match instruments.get(&point.instrument_id) {
            Some((gpu_model, region))
                if gpu_model != &point.gpu_model || region != &point.region =>
            {
                return Err(TradeError::IncompatibleInstruments);
            }
            Some(_) => {}
            None => {
                instruments.insert(
                    point.instrument_id.clone(),
                    (point.gpu_model.clone(), point.region.clone()),
                );
            }
        }
    }
    if instruments.len() != 3 {
        return Err(TradeError::IncompatibleInstruments);
    }
    Ok(instruments
        .into_iter()
        .map(|(instrument_id, (gpu_model, region))| TradeInstrumentV1 {
            instrument_id,
            gpu_model,
            region,
            price_unit: "usd_micros_per_gpu_hour",
            tradable: false,
        })
        .collect())
}

fn find_instrument(
    instruments: &[TradeInstrumentV1],
    instrument_id: &str,
) -> Result<TradeInstrumentV1, TradeError> {
    instruments
        .iter()
        .find(|instrument| instrument.instrument_id == instrument_id)
        .cloned()
        .ok_or(TradeError::InstrumentNotFound)
}

fn validate_dataset_digest(
    release: &SyntheticMarketTimeSeriesV1,
    expected: Sha256Digest,
) -> Result<(), TradeError> {
    if release.dataset_id != TRADE_DATASET_ID_V1 || release.snapshot_digest != expected {
        return Err(TradeError::DatasetDigestMismatch);
    }
    Ok(())
}

fn validate_scenario_request(request: &TradeScenarioRequestV1) -> Result<(), TradeError> {
    if !(-MAX_PRICE_SHOCK_BPS..=MAX_PRICE_SHOCK_BPS).contains(&request.price_shock_bps)
        || !(-MAX_AVAILABILITY_DELTA..=MAX_AVAILABILITY_DELTA)
            .contains(&request.availability_delta_units)
    {
        return Err(TradeError::InvalidRequest);
    }
    Ok(())
}

fn common_observations<'a>(
    release: &'a SyntheticMarketTimeSeriesV1,
    primary_instrument_id: &str,
    comparison_instrument_id: &str,
) -> Vec<(
    &'a SyntheticMarketTimeSeriesPointV1,
    &'a SyntheticMarketTimeSeriesPointV1,
)> {
    let mut common = Vec::new();
    for period_index in 0..release.expected_period_count {
        let primary = release.observations.iter().find(|point| {
            point.instrument_id == primary_instrument_id && point.period_index == period_index
        });
        let comparison = release.observations.iter().find(|point| {
            point.instrument_id == comparison_instrument_id && point.period_index == period_index
        });
        if let (Some(primary), Some(comparison)) = (primary, comparison) {
            common.push((primary, comparison));
        }
    }
    common
}

fn sensitivity_v1(
    release: &SyntheticMarketTimeSeriesV1,
    instruments: &[TradeInstrumentV1],
) -> Result<TradeSensitivityV1, TradeError> {
    let mut complete_periods = Vec::new();
    for period_index in 0..release.expected_period_count {
        let points = instruments
            .iter()
            .filter_map(|instrument| {
                release.observations.iter().find(|point| {
                    point.instrument_id == instrument.instrument_id
                        && point.period_index == period_index
                })
            })
            .collect::<Vec<_>>();
        if points.len() == instruments.len() {
            complete_periods.push((period_index, points));
        }
    }
    let (latest_index, latest_points) = complete_periods
        .last()
        .ok_or(TradeError::IncompatibleInstruments)?;
    let (prior_index, prior_points) = complete_periods
        .get(complete_periods.len().saturating_sub(2))
        .ok_or(TradeError::IncompatibleInstruments)?;
    let mut latest_prices = latest_points
        .iter()
        .map(|point| {
            (
                point.asking_usd_micros_per_hour,
                point.instrument_id.as_str(),
            )
        })
        .collect::<Vec<_>>();
    latest_prices.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)));
    let reference_median = latest_prices
        .get(1)
        .map(|point| point.0)
        .ok_or(TradeError::IncompatibleInstruments)?;
    let (highest_price, stressed_instrument_id) = latest_prices
        .last()
        .copied()
        .ok_or(TradeError::IncompatibleInstruments)?;
    let stressed_price = highest_price
        .checked_mul(2)
        .ok_or(TradeError::ArithmeticOverflow)?;
    let mut stressed_prices = latest_prices.clone();
    let highest = stressed_prices
        .iter_mut()
        .find(|(price, instrument_id)| {
            *price == highest_price && *instrument_id == stressed_instrument_id
        })
        .ok_or(TradeError::IncompatibleInstruments)?;
    highest.0 = stressed_price;
    stressed_prices.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)));
    let stressed_median = stressed_prices
        .get(1)
        .map(|point| point.0)
        .ok_or(TradeError::IncompatibleInstruments)?;
    let mut prior_prices = prior_points
        .iter()
        .map(|point| point.asking_usd_micros_per_hour)
        .collect::<Vec<_>>();
    prior_prices.sort_unstable();
    let prior_median = *prior_prices
        .get(1)
        .ok_or(TradeError::IncompatibleInstruments)?;
    let latest_period = release
        .periods
        .get(usize::from(*latest_index))
        .cloned()
        .ok_or(TradeError::IncompatibleInstruments)?;
    let prior_period = release
        .periods
        .get(usize::from(*prior_index))
        .cloned()
        .ok_or(TradeError::IncompatibleInstruments)?;

    Ok(TradeSensitivityV1 {
        latest_complete_period: latest_period,
        reference_median_usd_micros_per_hour: reference_median,
        stressed_instrument_id: stressed_instrument_id.to_owned(),
        stressed_quote_change_bps: 10_000,
        stressed_reference_median_usd_micros_per_hour: stressed_median,
        outlier_median_change_usd_micros_per_hour: signed_difference(
            stressed_median,
            reference_median,
        )?,
        prior_complete_period: prior_period,
        prior_reference_median_usd_micros_per_hour: prior_median,
        one_period_lag_change_usd_micros_per_hour: signed_difference(
            reference_median,
            prior_median,
        )?,
        method_label: "fixture_median_outlier_and_one_period_lag_sensitivity_not_market_manipulation_or_staleness_detection",
    })
}

fn contiguous_change_metric(
    observations: &[SyntheticMarketTimeSeriesPointV1],
) -> Result<(Option<u64>, usize), TradeError> {
    let mut sum = 0_u128;
    let mut count = 0_u128;
    for pair in observations.windows(2) {
        if pair[0].period_index.checked_add(1) != Some(pair[1].period_index) {
            continue;
        }
        let previous = u128::from(pair[0].asking_usd_micros_per_hour);
        let current = u128::from(pair[1].asking_usd_micros_per_hour);
        if previous == 0 {
            return Err(TradeError::InvalidRequest);
        }
        let change_bps = current
            .abs_diff(previous)
            .checked_mul(BPS_DENOMINATOR)
            .ok_or(TradeError::ArithmeticOverflow)?
            / previous;
        sum = sum
            .checked_add(change_bps)
            .ok_or(TradeError::ArithmeticOverflow)?;
        count = count.checked_add(1).ok_or(TradeError::ArithmeticOverflow)?;
    }
    if count == 0 {
        return Ok((None, 0));
    }
    let mean = u64::try_from(sum / count).map_err(|_| TradeError::ArithmeticOverflow)?;
    let count = usize::try_from(count).map_err(|_| TradeError::ArithmeticOverflow)?;
    Ok((Some(mean), count))
}

fn apply_price_shock(price: u64, shock_bps: i32) -> Result<u64, TradeError> {
    let factor = i64::from(10_000_i32 + shock_bps);
    let adjusted = u128::from(price)
        .checked_mul(u128::try_from(factor).map_err(|_| TradeError::ArithmeticOverflow)?)
        .ok_or(TradeError::ArithmeticOverflow)?
        / BPS_DENOMINATOR;
    u64::try_from(adjusted).map_err(|_| TradeError::ArithmeticOverflow)
}

fn apply_availability_delta(
    availability: Option<u64>,
    delta: i64,
) -> Result<Option<u64>, TradeError> {
    availability
        .map(|available| {
            let adjusted = i128::from(available)
                .checked_add(i128::from(delta))
                .ok_or(TradeError::ArithmeticOverflow)?;
            u64::try_from(adjusted).map_err(|_| TradeError::InvalidRequest)
        })
        .transpose()
}

fn signed_difference(left: u64, right: u64) -> Result<i64, TradeError> {
    i64::try_from(i128::from(left) - i128::from(right)).map_err(|_| TradeError::ArithmeticOverflow)
}

fn ratio_bps(numerator: usize, denominator: usize) -> Result<u64, TradeError> {
    if denominator == 0 {
        return Err(TradeError::InvalidRequest);
    }
    let numerator = u128::try_from(numerator).map_err(|_| TradeError::ArithmeticOverflow)?;
    let denominator = u128::try_from(denominator).map_err(|_| TradeError::ArithmeticOverflow)?;
    let scaled = numerator
        .checked_mul(BPS_DENOMINATOR)
        .ok_or(TradeError::ArithmeticOverflow)?
        / denominator;
    u64::try_from(scaled).map_err(|_| TradeError::ArithmeticOverflow)
}

fn increment(value: &mut usize) -> Result<(), TradeError> {
    *value = value.checked_add(1).ok_or(TradeError::ArithmeticOverflow)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario_request() -> Result<TradeScenarioRequestV1, TradeError> {
        let catalog = trade_catalog_v1()?;
        Ok(TradeScenarioRequestV1 {
            expected_dataset_digest: catalog.dataset_digest,
            primary_instrument_id: "a100-us-east".to_owned(),
            comparison_instrument_id: "a100-us-west".to_owned(),
            price_shock_bps: 0,
            availability_delta_units: 0,
        })
    }

    #[test]
    fn catalog_is_stable_and_labels_fixture_composition_as_non_market_data()
    -> Result<(), TradeError> {
        let first = trade_catalog_v1()?;
        let second = trade_catalog_v1()?;
        assert_eq!(first, second);
        assert_eq!(first.observed_row_count, 23);
        assert_eq!(first.expected_row_count, 24);
        assert_eq!(first.fixture_coverage_bps, 9_583);
        assert!(
            first
                .instruments
                .iter()
                .all(|instrument| !instrument.tradable)
        );
        assert!(first.nonclaims.contains("No live market observation"));
        assert!(first.fixture_composition_hhi_bps > 3_000);
        assert!(first.fixture_composition_hhi_bps < 3_500);
        Ok(())
    }

    #[test]
    fn series_preserves_gaps_unknown_capacity_and_zero_capacity() -> Result<(), TradeError> {
        let east = trade_series_v1("a100-us-east")?;
        assert_eq!(east.observed_period_count, 7);
        assert_eq!(east.price_coverage_bps, 8_750);
        assert_eq!(east.missing_periods, ["2026-01-01T04:00:00Z"]);
        assert_eq!(east.contiguous_change_sample_count, 5);
        let west = trade_series_v1("a100-us-west")?;
        assert_eq!(west.unknown_availability_count, 1);
        let europe = trade_series_v1("a100-eu-west")?;
        assert_eq!(europe.zero_availability_count, 2);
        Ok(())
    }

    #[test]
    fn scenario_is_deterministic_digest_bound_and_reports_sensitivity() -> Result<(), TradeError> {
        let request = scenario_request()?;
        let first = trade_scenario_v1(&request)?;
        let second = trade_scenario_v1(&request)?;
        assert_eq!(first, second);
        assert_eq!(first.observed_at, "2026-01-01T07:00:00Z");
        assert_eq!(first.regional_basis_usd_micros_per_hour, -75_000);
        assert_eq!(first.scenario_available_units, Some(4));
        assert_eq!(
            first.sensitivity.outlier_median_change_usd_micros_per_hour,
            0
        );
        Ok(())
    }

    #[test]
    fn scenario_rejects_stale_digest_incompatible_instruments_and_unbounded_shocks()
    -> Result<(), TradeError> {
        let mut request = scenario_request()?;
        request.expected_dataset_digest = Sha256Digest::from_bytes([0; 32]);
        assert!(matches!(
            trade_scenario_v1(&request),
            Err(TradeError::DatasetDigestMismatch)
        ));

        request = scenario_request()?;
        request.primary_instrument_id = "unknown".to_owned();
        assert!(matches!(
            trade_scenario_v1(&request),
            Err(TradeError::InstrumentNotFound)
        ));

        request = scenario_request()?;
        request.price_shock_bps = 5_001;
        assert!(matches!(
            trade_scenario_v1(&request),
            Err(TradeError::InvalidRequest)
        ));

        request = scenario_request()?;
        request.availability_delta_units = -5;
        request.primary_instrument_id = "a100-eu-west".to_owned();
        assert!(matches!(
            trade_scenario_v1(&request),
            Err(TradeError::InvalidRequest)
        ));
        Ok(())
    }

    #[test]
    fn paper_replay_uses_known_capacity_and_is_repeatable() -> Result<(), TradeError> {
        let catalog = trade_catalog_v1()?;
        let request = TradePaperReplayRequestV1 {
            expected_dataset_digest: catalog.dataset_digest,
            baseline_instrument_id: "a100-us-west".to_owned(),
            requested_units: 1,
        };
        let first = trade_paper_replay_v1(&request)?;
        let second = trade_paper_replay_v1(&request)?;
        assert_eq!(first, second);
        assert_eq!(first.periods.len(), 8);
        assert_eq!(first.comparable_period_count, 7);
        assert_eq!(first.unknown_baseline_capacity_count, 1);
        assert_eq!(first.missing_price_observation_count, 1);
        assert_eq!(first.unknown_capacity_observation_count, 1);
        assert_eq!(first.insufficient_capacity_observation_count, 2);
        assert!(first.baseline_minus_strategy_asking_cost_microusd > 0);
        assert!(first.measurement_type.contains("not_fill_or_pnl"));
        Ok(())
    }

    #[test]
    fn paper_replay_rejects_units_outside_fixed_fixture_bounds() -> Result<(), TradeError> {
        let catalog = trade_catalog_v1()?;
        let request = TradePaperReplayRequestV1 {
            expected_dataset_digest: catalog.dataset_digest,
            baseline_instrument_id: "a100-us-west".to_owned(),
            requested_units: MAX_REQUESTED_UNITS + 1,
        };
        assert!(matches!(
            trade_paper_replay_v1(&request),
            Err(TradeError::InvalidRequest)
        ));
        Ok(())
    }

    #[test]
    fn deterministic_scenario_input_sweep_stays_within_the_research_contract()
    -> Result<(), TradeError> {
        let catalog = trade_catalog_v1()?;
        let instrument_ids = [
            "a100-us-east",
            "a100-us-west",
            "a100-eu-west",
            "unknown",
            "",
        ];
        let mut seed = 0x4859_5045_5241_4901_u64;

        for case in 0..1_024 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let primary = instrument_ids[((seed >> 16) as usize) % instrument_ids.len()];
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let comparison = instrument_ids[((seed >> 24) as usize) % instrument_ids.len()];
            let price_shock_bps = ((seed >> 32) % 200_001) as i32 - 100_000;
            let availability_delta_units = i64::try_from((seed >> 40) % 4_001)
                .map_err(|_| TradeError::ArithmeticOverflow)?
                - 2_000;
            let expected_dataset_digest = if case % 17 == 0 {
                Sha256Digest::from_bytes([0; 32])
            } else {
                catalog.dataset_digest
            };
            let request = TradeScenarioRequestV1 {
                expected_dataset_digest,
                primary_instrument_id: primary.to_owned(),
                comparison_instrument_id: comparison.to_owned(),
                price_shock_bps,
                availability_delta_units,
            };

            match trade_scenario_v1(&request) {
                Ok(report) => {
                    assert!(report.scenario_primary_asking_usd_micros_per_hour > 0);
                    assert!(
                        report
                            .scenario_available_units
                            .is_none_or(|units| units <= 10)
                    );
                    assert!(report.nonclaims.contains("No live market observation"));
                    assert!(report.claim_ceiling.contains("synthetic series release"));
                }
                Err(
                    TradeError::DatasetDigestMismatch
                    | TradeError::InstrumentNotFound
                    | TradeError::IncompatibleInstruments
                    | TradeError::InvalidRequest,
                ) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
