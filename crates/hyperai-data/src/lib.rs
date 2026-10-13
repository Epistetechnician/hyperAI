//! HyperAI-owned local dataset catalog and bounded query contracts.
//!
//! This crate starts with a deterministic synthetic snapshot. It does not
//! connect to data sources or interpret market observations as execution,
//! payment, or outcome evidence.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, File, OpenOptions},
    future::Future,
    io::{self, Read, Write},
    ops::ControlFlow,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use datafusion::{
    arrow::{
        array::{Array, RecordBatch, StringArray, UInt64Array},
        datatypes::{DataType, Field, Schema},
        util::display::{ArrayFormatter, FormatOptions},
    },
    datasource::empty::EmptyTable,
    execution::{
        SessionStateBuilder, SessionStateDefaults, object_store::ObjectStoreUrl,
        runtime_env::RuntimeEnvBuilder,
    },
    parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder,
    physical_plan::SendableRecordBatchStream,
    prelude::{NdJsonReadOptions, ParquetReadOptions, SessionConfig, SessionContext},
};
use fs2::FileExt;
use hyperai_foundation::{Sha256Digest, domain_separated_digest};
use object_store::{ObjectStore, memory::InMemory, path::Path as ObjectStorePath};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlparser::{
    ast::{
        Expr, FunctionArg, FunctionArgExpr, FunctionArgumentClause, FunctionArguments, GroupByExpr,
        LimitClause, OrderBy, OrderByKind, Query, Select, SelectFlavor, SelectItem, SetExpr,
        Statement, TableFactor, Visit, Visitor,
    },
    dialect::GenericDialect,
    parser::Parser,
};
use thiserror::Error;

const MANIFEST_SCHEMA: &str = "hyperai.data.manifest.v1";
const JSON_DATASET_ID: &str = "hyperai.synthetic_market_observations_json.v1";
const PARQUET_DATASET_ID: &str = "hyperai.synthetic_market_observations_parquet.v1";
const GPUDIFF_DATASET_ID: &str = "hyperai.gpudiff_public_gpu_offers.v1";
/// Fixed public endpoint used by the owner-triggered `GPUDiff` snapshot import.
pub const GPUDIFF_SOURCE_URL_V1: &str = "https://gpudiff.com/api/v1/offers.json";
const GPUDIFF_SCHEMA: &str = "id:string,provider:string,sku:string,gpu_model:string?,price_decimal:string,price_micros_per_source_unit:uint64?,unit:string,pricing_type:string,region:string,metric:string?,sample_size:string?,vram_gb:string?,attrs_json:string,provenance_url:string,observed_at:string,source_raw_sha256:string,fixture:boolean";
const GPUDIFF_SOURCE_LABEL: &str = "GPUDiff public GPU offer feed; CC BY 4.0; cite gpudiff.com";
const GPUDIFF_FRESHNESS: &str = "publisher_observed_per_row";
const GPUDIFF_CLAIM_CEILING: &str = "source-attributed public price records and publisher timestamps only; no direct provider, executable offer, current capacity, seller identity, computation correctness, payment, settlement, or production claim";
const GPUDIFF_MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const GPUDIFF_MAX_ROWS: usize = 4_096;
const GPUDIFF_MAX_CAPTURES: usize = 16;
const SYNTHETIC_DERIVATION_METHOD: &str = "hyperai.synthetic-market-aggregation.v1";
const SAMPLE_FILE: &str = "synthetic-market-observations.json";
const PARQUET_FILE: &str = "synthetic-market-observations.parquet";
const SYNTHETIC_TIME_SERIES_ID: &str = "hyperai.synthetic.compute-hourly-series.v1";
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SQL_BYTES: usize = 4 * 1024;
const MAX_RESULT_ROWS: usize = 1_000;
const MAX_RESULT_BYTES: usize = 256 * 1024;
const MAX_COLUMNS: usize = 64;
const MAX_CATALOG_DATASET_IDS: usize = 3;
const MAX_MANIFEST_DIRECTORY_ENTRIES: usize = 64;
const QUERY_MEMORY_BYTES: usize = 64 * 1024 * 1024;
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);
const SAMPLE_JSON: &str = concat!(
    "{\"instance_id\":\"synthetic-a100-01\",\"gpu_model\":\"A100\",\"region\":\"us-east\",\"asking_usd_micros_per_hour\":1200000,\"available_count\":8,\"observed_at\":\"2026-01-01T00:00:00Z\"}\n",
    "{\"instance_id\":\"synthetic-h100-01\",\"gpu_model\":\"H100\",\"region\":\"us-west\",\"asking_usd_micros_per_hour\":3500000,\"available_count\":2,\"observed_at\":\"2026-01-01T00:00:00Z\"}\n",
    "{\"instance_id\":\"synthetic-mi300-01\",\"gpu_model\":\"MI300X\",\"region\":\"eu-west\",\"asking_usd_micros_per_hour\":2100000,\"available_count\":null,\"observed_at\":\"2026-01-01T00:00:00Z\"}\n",
);
const SYNTHETIC_SCHEMA: &str = "instance_id:string,gpu_model:string,region:string,asking_usd_micros_per_hour:uint64,available_count:uint64?,observed_at:string";

/// Errors returned by local catalog and bounded query operations.
#[derive(Debug, Error)]
pub enum DataError {
    /// Local file access failed or encountered an unsafe path.
    #[error("local data storage is unavailable or unsafe")]
    Storage(#[source] io::Error),
    /// A catalog or dataset contract failed validation.
    #[error("local data catalog or manifest is invalid")]
    InvalidCatalog,
    /// A selected dataset does not exist in this local catalog.
    #[error("dataset is not present in the local catalog")]
    DatasetNotFound,
    /// Dataset bytes or the saved manifest no longer match their digest.
    #[error("local dataset content does not match its manifest")]
    IntegrityMismatch,
    /// The query or source/result exceeds an explicit boundary.
    #[error("local data operation exceeded a configured limit")]
    LimitExceeded,
    /// The source-capture retention capacity is full; prior captures are preserved.
    #[error("public source-capture retention capacity is full")]
    CaptureRetentionFull,
    /// SQL is not in the supported read-only query subset.
    #[error("query is outside the read-only SQL subset")]
    QueryRejected,
    /// The query engine could not plan or execute an accepted query.
    #[error("local query could not be planned or executed")]
    QueryFailed(#[source] datafusion::error::DataFusionError),
    /// The caller canceled the bounded query before it returned.
    #[error("local query was canceled")]
    Cancelled,
    /// The built-in synthetic Parquet fixture could not be encoded.
    #[error("synthetic dataset fixture could not be encoded")]
    FixtureEncoding,
    /// Canonical model or view digest could not be computed.
    #[error("saved data definition could not be encoded")]
    DefinitionEncoding,
    /// A local idempotent write conflicts with existing bytes.
    #[error("local data definition conflicts with existing bytes")]
    Conflict,
}

/// Versioned descriptor for one local dataset snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetManifestV1 {
    /// Manifest schema identifier.
    pub schema_version: String,
    /// Stable dataset ID.
    pub dataset_id: String,
    /// Format used by the local scan.
    pub format: String,
    /// Dataset file path relative to the owned data root.
    pub relative_path: String,
    /// Exact byte length of the dataset file.
    pub byte_length: u64,
    /// SHA-256 of the dataset file bytes.
    pub content_digest: Sha256Digest,
    /// SHA-256 of the documented field schema.
    pub schema_digest: Sha256Digest,
    /// Number of source records in this snapshot.
    pub row_count: u64,
    /// Source/provenance label.
    pub source_label: String,
    /// Fixed timestamp describing the generated synthetic snapshot.
    pub snapshot_at: String,
    /// Freshness label that prevents this fixture being read as market data.
    pub freshness: String,
    /// Claim ceiling for this dataset.
    pub claim_ceiling: String,
}

/// Receipt for one locally retained `GPUDiff` public-feed capture.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GpudiffImportReceiptV1 {
    /// Receipt schema identifier.
    pub schema_version: String,
    /// Fixed public source endpoint.
    pub source_url: String,
    /// Required source attribution and license note.
    pub attribution: String,
    /// SHA-256 of the exact upstream response bytes.
    pub raw_source_digest: Sha256Digest,
    /// SHA-256 of normalized NDJSON bytes used by `DataFusion`.
    pub normalized_digest: Sha256Digest,
    /// Number of non-fixture offer records.
    pub row_count: u64,
    /// Earliest publisher observation timestamp in the capture.
    pub observed_at_min: String,
    /// Latest publisher observation timestamp in the capture.
    pub observed_at_max: String,
    /// Local Unix time when this digest was first captured.
    pub captured_unix_seconds: u64,
    /// Explicit retention policy for distinct exact source captures.
    pub retention_policy: String,
    /// Publisher cache validator, when supplied.
    pub etag: Option<String>,
    /// Publisher modification timestamp, when supplied.
    pub last_modified: Option<String>,
}

/// Result of validating and retaining one `GPUDiff` feed body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GpudiffImportV1 {
    /// Current Data catalog manifest.
    pub manifest: DatasetManifestV1,
    /// Receipt binding raw source bytes to normalized dataset bytes.
    pub receipt: GpudiffImportReceiptV1,
}

struct VerifiedDatasetSnapshot {
    manifest: DatasetManifestV1,
    bytes: Vec<u8>,
}

/// A catalog entry whose on-disk content was checked against its manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DatasetCatalogEntryV1 {
    /// Stable dataset identifier.
    pub dataset_id: String,
    /// File format used by the reader.
    pub format: String,
    /// Source/provenance label.
    pub source_label: String,
    /// SHA-256 of exact dataset bytes.
    pub snapshot_digest: Sha256Digest,
    /// SHA-256 of its field schema description.
    pub schema_digest: Sha256Digest,
    /// Number of source rows.
    pub row_count: u64,
    /// Freshness state.
    pub freshness: String,
    /// Claim ceiling.
    pub claim_ceiling: String,
}

/// Versioned local catalog response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataCatalogV1 {
    /// API schema identifier.
    pub schema_version: &'static str,
    /// Whether the owned catalog has been seeded.
    pub status: &'static str,
    /// Verified local data entries.
    pub datasets: Vec<DatasetCatalogEntryV1>,
    /// Explicitly states this catalog is not a remote source.
    pub effect_boundary: &'static str,
}

/// Bounded owner result for inspecting one dataset in the local catalog.
///
/// This is an in-process result, not a serialized Data catalog or manifest
/// schema. Only `Bound` carries a source-verified content digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatasetReadStatusV1 {
    /// No local root, manifest directory, or requested dataset is present.
    NotSeeded,
    /// The local manifest and exact dataset bytes were verified.
    Bound(Sha256Digest),
    /// A manifest was present but its referenced local file was absent.
    Missing,
    /// The catalog, manifest, path, or content-integrity contract was rejected.
    Conflicting,
    /// The operating system denied a local read.
    Denied,
    /// Another bounded local error prevented classification.
    Unavailable,
}

/// One verified input manifest reference in a local derived-data release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BronzeReferenceV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Stable input dataset identifier.
    pub dataset_id: String,
    /// Exact source-byte digest.
    pub snapshot_digest: Sha256Digest,
    /// Field-schema digest.
    pub schema_digest: Sha256Digest,
    /// Human-readable source attribution.
    pub source_label: String,
    /// Number of source records.
    pub row_count: u64,
    /// Freshness classification from the source manifest.
    pub freshness: String,
    /// Limits on what this source can support.
    pub claim_ceiling: String,
}

/// Source-attributed normalized observation with explicit integer units.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NormalizedObservationV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Source-owned observation identity.
    pub source_instance_id: String,
    /// GPU model label from the source.
    pub gpu_model: String,
    /// Region label from the source.
    pub region: String,
    /// Asking price in integer USD micros per hour.
    pub asking_usd_micros_per_hour: u64,
    /// Reported available-unit count; null remains unknown.
    pub available_count: Option<u64>,
    /// Source observation timestamp, preserved as supplied.
    pub observed_at: String,
    /// Digest of the source snapshot.
    pub source_snapshot_digest: Sha256Digest,
    /// Freshness classification inherited from the source manifest.
    pub freshness: String,
}

/// Synthetic-only grouped summary; it is not a market index or price oracle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SyntheticGroupSummaryV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// GPU model dimension preserved from source records.
    pub gpu_model: String,
    /// Region dimension preserved from source records.
    pub region: String,
    /// Number of source observations in this group.
    pub record_count: u64,
    /// Lowest asking price in integer USD micros per hour.
    pub min_asking_usd_micros_per_hour: u64,
    /// Highest asking price in integer USD micros per hour.
    pub max_asking_usd_micros_per_hour: u64,
    /// Number of records with a known availability value.
    pub known_availability_records: u64,
    /// Number of records where availability is unknown.
    pub unknown_availability_records: u64,
    /// Sum of available units only when every record is known.
    pub available_units_total: Option<u64>,
}

/// On-demand Bronze/Silver/Gold projection over the owned synthetic snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SyntheticDataReleaseV1 {
    /// Contract schema identifier.
    pub schema_version: &'static str,
    /// Stable transform implementation identifier.
    pub method_version: &'static str,
    /// Metadata-only pointer to the verified input snapshot.
    pub bronze: BronzeReferenceV1,
    /// Normalized input records.
    pub silver: Vec<NormalizedObservationV1>,
    /// Grouped synthetic summaries.
    pub gold: Vec<SyntheticGroupSummaryV1>,
    /// Digest of the method, source snapshot, and derived records.
    pub output_digest: Sha256Digest,
    /// Explicit evidence ceiling.
    pub claim_ceiling: &'static str,
    /// No writes or external effects are part of this transform.
    pub effect_boundary: &'static str,
}

/// One HyperAI-authored synthetic hourly asking-price observation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SyntheticMarketTimeSeriesPointV1 {
    /// Stable identity within the synthetic source release.
    pub observation_id: String,
    /// Research instrument key; it is not a tradable product.
    pub instrument_id: String,
    /// GPU model label.
    pub gpu_model: String,
    /// Synthetic region label.
    pub region: String,
    /// Zero-based slot on the fixed UTC hourly sampling grid.
    pub period_index: u8,
    /// Fixed source observation time in UTC.
    pub observed_at: String,
    /// Asking price in integer USD micros per GPU-hour.
    pub asking_usd_micros_per_hour: u64,
    /// Reported available units; None is unknown, while Some(0) is zero.
    pub available_count: Option<u64>,
}

/// Versioned immutable synthetic time-series release owned by `HyperAI` Data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SyntheticMarketTimeSeriesV1 {
    /// Data contract schema identifier.
    pub schema_version: &'static str,
    /// Stable Data dataset identifier.
    pub dataset_id: &'static str,
    /// Revision of the HyperAI-authored fixture bytes and shape.
    pub dataset_revision: &'static str,
    /// Source attribution.
    pub source_label: &'static str,
    /// Fixed sampling interval in seconds.
    pub interval_seconds: u32,
    /// Expected time slots for each instrument.
    pub expected_period_count: u8,
    /// Exact UTC sampling grid, including absent source observations.
    pub periods: Vec<String>,
    /// Observed source records in stable instrument/period order.
    pub observations: Vec<SyntheticMarketTimeSeriesPointV1>,
    /// Digest binding schema, grid, source, and exact ordered observations.
    pub snapshot_digest: Sha256Digest,
    /// Freshness classification.
    pub freshness: &'static str,
    /// Source revision policy.
    pub revision_policy: &'static str,
    /// Explicit evidence ceiling.
    pub claim_ceiling: &'static str,
    /// Explicit effect boundary.
    pub effect_boundary: &'static str,
}

#[derive(Serialize)]
struct SyntheticMarketTimeSeriesIdentityV1<'a> {
    schema_version: &'static str,
    dataset_id: &'static str,
    dataset_revision: &'static str,
    source_label: &'static str,
    interval_seconds: u32,
    expected_period_count: u8,
    periods: &'a [String],
    observations: &'a [SyntheticMarketTimeSeriesPointV1],
}

struct SyntheticInstrumentSeriesFixtureV1 {
    instrument_id: &'static str,
    region: &'static str,
    asking_prices: [Option<u64>; 8],
    available_counts: [Option<u64>; 8],
}

const SYNTHETIC_TIME_SERIES_PERIODS: [&str; 8] = [
    "2026-01-01T00:00:00Z",
    "2026-01-01T01:00:00Z",
    "2026-01-01T02:00:00Z",
    "2026-01-01T03:00:00Z",
    "2026-01-01T04:00:00Z",
    "2026-01-01T05:00:00Z",
    "2026-01-01T06:00:00Z",
    "2026-01-01T07:00:00Z",
];

const SYNTHETIC_INSTRUMENT_SERIES: [SyntheticInstrumentSeriesFixtureV1; 3] = [
    SyntheticInstrumentSeriesFixtureV1 {
        instrument_id: "a100-eu-west",
        region: "eu-west",
        asking_prices: [
            Some(1_150_000),
            Some(1_175_000),
            Some(1_180_000),
            Some(1_200_000),
            Some(1_220_000),
            Some(1_210_000),
            Some(1_230_000),
            Some(1_240_000),
        ],
        available_counts: [
            Some(2),
            Some(2),
            Some(2),
            Some(1),
            Some(1),
            Some(1),
            Some(0),
            Some(0),
        ],
    },
    SyntheticInstrumentSeriesFixtureV1 {
        instrument_id: "a100-us-east",
        region: "us-east",
        asking_prices: [
            Some(1_200_000),
            Some(1_220_000),
            Some(1_210_000),
            Some(1_250_000),
            None,
            Some(1_270_000),
            Some(1_285_000),
            Some(1_275_000),
        ],
        available_counts: [
            Some(8),
            Some(7),
            Some(7),
            Some(4),
            None,
            Some(3),
            Some(4),
            Some(4),
        ],
    },
    SyntheticInstrumentSeriesFixtureV1 {
        instrument_id: "a100-us-west",
        region: "us-west",
        asking_prices: [
            Some(1_300_000),
            Some(1_290_000),
            Some(1_320_000),
            Some(1_330_000),
            Some(1_340_000),
            Some(1_320_000),
            Some(1_310_000),
            Some(1_350_000),
        ],
        available_counts: [
            Some(5),
            Some(5),
            Some(4),
            Some(4),
            Some(4),
            None,
            Some(2),
            Some(2),
        ],
    },
];

/// Return the immutable HyperAI-authored regional hourly series used by
/// bounded Trade research.
///
/// The release is compiled in, deterministic, synthetic, and separate from
/// the local JSON/Parquet Data seed. Its digest binds the exact grid and
/// ordered observations.
///
/// # Errors
///
/// Returns an error if the typed release identity cannot be canonically
/// encoded.
pub fn synthetic_market_time_series_v1() -> Result<SyntheticMarketTimeSeriesV1, DataError> {
    let periods = SYNTHETIC_TIME_SERIES_PERIODS
        .iter()
        .map(|period| (*period).to_owned())
        .collect::<Vec<_>>();
    let mut observations = Vec::with_capacity(23);
    for instrument in &SYNTHETIC_INSTRUMENT_SERIES {
        for (period_index, asking_price) in instrument.asking_prices.iter().enumerate() {
            let Some(asking_usd_micros_per_hour) = asking_price else {
                continue;
            };
            let period_index = u8::try_from(period_index).map_err(|_| DataError::LimitExceeded)?;
            observations.push(SyntheticMarketTimeSeriesPointV1 {
                observation_id: format!("{}.hour.{period_index:02}", instrument.instrument_id),
                instrument_id: instrument.instrument_id.to_owned(),
                gpu_model: "A100".to_owned(),
                region: instrument.region.to_owned(),
                period_index,
                observed_at: periods[usize::from(period_index)].clone(),
                asking_usd_micros_per_hour: *asking_usd_micros_per_hour,
                available_count: instrument.available_counts[usize::from(period_index)],
            });
        }
    }
    let identity = SyntheticMarketTimeSeriesIdentityV1 {
        schema_version: "hyperai.data.synthetic-hourly-series.v1",
        dataset_id: SYNTHETIC_TIME_SERIES_ID,
        dataset_revision: "hyperai-synthetic-a100-regional-series-r1",
        source_label: "HyperAI authored synthetic hourly observations",
        interval_seconds: 3_600,
        expected_period_count: 8,
        periods: &periods,
        observations: &observations,
    };
    let snapshot_digest =
        domain_separated_digest("hyperai.data.synthetic-market-time-series.v1", &identity)
            .map_err(|_| DataError::DefinitionEncoding)?;

    Ok(SyntheticMarketTimeSeriesV1 {
        schema_version: "hyperai.data.synthetic-hourly-series.v1",
        dataset_id: SYNTHETIC_TIME_SERIES_ID,
        dataset_revision: "hyperai-synthetic-a100-regional-series-r1",
        source_label: "HyperAI authored synthetic hourly observations",
        interval_seconds: 3_600,
        expected_period_count: 8,
        periods,
        observations,
        snapshot_digest,
        freshness: "synthetic_not_market_data",
        revision_policy: "immutable_fixture_revision_v1",
        claim_ceiling: "HyperAI synthetic scenario research only; no live price, availability, seller, or execution claim",
        effect_boundary: "LOCAL_SYNTHETIC_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    })
}

/// Supported output representation for a read-only query.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryFormat {
    /// JSON object containing ordered columns and rows.
    Json,
    /// RFC-compatible CSV text with a header row.
    Csv,
}

/// Bounded read-only local query request.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryRequestV1 {
    /// Target local dataset.
    pub dataset_id: String,
    /// One read-only `SELECT` statement.
    pub sql: String,
    /// Bounded result format.
    pub format: QueryFormat,
    /// Optional result row ceiling, which can only lower the fixed maximum.
    pub max_rows: Option<usize>,
}

/// Bounded query result over one verified local dataset snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryResultV1 {
    /// Query response schema identifier.
    pub schema_version: &'static str,
    /// The local dataset queried.
    pub dataset_id: String,
    /// Exact source snapshot digest.
    pub snapshot_digest: Sha256Digest,
    /// Stable field names in query output order.
    pub columns: Vec<String>,
    /// Stable, typed-as-text row cells; JSON null remains `None`.
    pub rows: Vec<Vec<Option<String>>>,
    /// Output form requested by the caller.
    pub format: QueryFormat,
    /// CSV output when the format is `csv`.
    pub csv: Option<String>,
    /// Number of rows returned.
    pub row_count: usize,
    /// UTF-8 output byte count.
    pub output_bytes: usize,
    /// Local query resource ceilings applied.
    pub limits: QueryLimitsV1,
    /// Explicit evidence and authority limits.
    pub claim_ceiling: &'static str,
    /// External effects are not part of this query.
    pub effect_boundary: &'static str,
}

/// Query resource limits applied by this build.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryLimitsV1 {
    /// Maximum source file bytes.
    pub max_input_bytes: u64,
    /// Maximum SQL query bytes.
    pub max_sql_bytes: usize,
    /// Maximum wall time.
    pub max_duration_ms: u64,
    /// `DataFusion` memory-pool ceiling.
    pub memory_bytes: usize,
    /// Maximum execution partitions.
    pub partitions: usize,
    /// Maximum Arrow batch rows.
    pub batch_rows: usize,
    /// Maximum serialized result rows.
    pub max_result_rows: usize,
    /// Maximum serialized response bytes.
    pub max_result_bytes: usize,
}

/// Versioned query model persisted without source or result payloads.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataModelV1 {
    /// Model schema identifier.
    pub schema_version: String,
    /// Stable user-selected name.
    pub model_id: String,
    /// Dataset whose snapshot digest is bound to this SQL.
    pub dataset_id: String,
    /// Exact manifest snapshot used to preview the model.
    pub snapshot_digest: Sha256Digest,
    /// Validated, read-only SQL.
    pub sql: String,
}

/// Versioned view definition containing display metadata only.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataViewV1 {
    /// View schema identifier.
    pub schema_version: String,
    /// Stable user-selected name.
    pub view_id: String,
    /// Digest of the saved model shown by this view.
    pub model_digest: Sha256Digest,
    /// Table or chart presentation mode.
    pub presentation: String,
    /// Ordered output columns shown in the view.
    pub columns: Vec<String>,
}

/// One locally saved model and its current input-snapshot state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataModelRecordV1 {
    /// Saved model definition; it contains SQL and digests but no row payload.
    pub model: DataModelV1,
    /// Digest of the exact persisted definition bytes.
    pub definition_digest: Sha256Digest,
    /// `current`, `stale`, or `missing` relative to the local catalog.
    pub input_state: String,
}

/// Versioned inventory of locally saved query models.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataModelCatalogV1 {
    /// Catalog schema identifier.
    pub schema_version: &'static str,
    /// Saved model definitions.
    pub models: Vec<DataModelRecordV1>,
    /// Local-only read boundary.
    pub effect_boundary: &'static str,
}

/// One locally saved view and its model-link state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataViewRecordV1 {
    /// Saved view display definition.
    pub view: DataViewV1,
    /// Digest of the exact persisted definition bytes.
    pub definition_digest: Sha256Digest,
    /// `linked` or `orphaned` relative to the saved model inventory.
    pub model_state: String,
}

/// Versioned inventory of locally saved views.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataViewCatalogV1 {
    /// Catalog schema identifier.
    pub schema_version: &'static str,
    /// Saved view definitions.
    pub views: Vec<DataViewRecordV1>,
    /// Local-only read boundary.
    pub effect_boundary: &'static str,
}

/// Digest preview for a model or view write.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DefinitionPreviewV1 {
    /// Preview schema identifier.
    pub schema_version: &'static str,
    /// Domain-separated digest the client must bind to its write request.
    pub preview_digest: Sha256Digest,
    /// Name of the definition that would be written.
    pub definition_id: String,
    /// Local effect to be performed after confirmation.
    pub effect_boundary: &'static str,
}

/// Saved definition receipt after a user-confirmed local write.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DefinitionSavedV1 {
    /// Receipt schema identifier.
    pub schema_version: &'static str,
    /// Name of the saved model or view.
    pub definition_id: String,
    /// Digest of the persisted canonical definition bytes.
    pub definition_digest: Sha256Digest,
    /// Local effect statement.
    pub effect_boundary: &'static str,
}

/// Preview describing the fixed deterministic local sample seed operation.
///
/// # Errors
///
/// Returns an error if the canonical synthetic-seed identity cannot be hashed.
pub fn synthetic_seed_preview() -> Result<DefinitionPreviewV1, DataError> {
    definition_preview(
        "synthetic-seed",
        "synthetic-market-observations",
        &json!({
            "schema_version": "hyperai.data.synthetic-seed.v1",
            "dataset_ids": [JSON_DATASET_ID, PARQUET_DATASET_ID],
            "source_label": "HyperAI deterministic synthetic sample",
            "formats": ["json", "parquet"],
        }),
    )
}

/// Deterministically create the `HyperAI` synthetic sample release.
///
/// This writes only the fixed sample bytes and manifest under `data_root`.
/// Existing identical bytes are returned as an idempotent success; changed
/// local bytes are preserved and produce a conflict.
///
/// # Errors
///
/// Returns an error if the path is unsafe, storage fails, or existing bytes
/// conflict with the deterministic sample.
pub fn seed_synthetic(data_root: &Path) -> Result<DatasetManifestV1, DataError> {
    ensure_data_root(data_root)?;
    let data_root = fs::canonicalize(data_root).map_err(DataError::Storage)?;
    let manifests_dir = data_root.join("manifests");
    ensure_child_directory(&data_root, &manifests_dir)?;
    let schema_digest = sha256(SYNTHETIC_SCHEMA.as_bytes());
    atomic_create_or_compare(&data_root.join(SAMPLE_FILE), SAMPLE_JSON.as_bytes())?;
    let parquet_bytes = parquet_sample_bytes()?;
    atomic_create_or_compare(&data_root.join(PARQUET_FILE), &parquet_bytes)?;
    let json_manifest = build_manifest(
        JSON_DATASET_ID,
        "json",
        SAMPLE_FILE,
        SAMPLE_JSON.as_bytes(),
        schema_digest,
        3,
    );
    let parquet_manifest = build_manifest(
        PARQUET_DATASET_ID,
        "parquet",
        PARQUET_FILE,
        &parquet_bytes,
        schema_digest,
        3,
    );
    write_manifest(&manifests_dir, &json_manifest)?;
    write_manifest(&manifests_dir, &parquet_manifest)?;
    Ok(json_manifest)
}

/// Validate and atomically retain one exact `GPUDiff` public-feed response.
///
/// The caller owns the fixed-source HTTPS request and supplies its bounded
/// response bytes and cache metadata. This function rejects fixtures, schema
/// drift, duplicate offer IDs, oversized input, and unsafe local state before
/// advancing the single current-dataset manifest.
///
/// # Errors
///
/// Returns an error if the feed is invalid, retention is full, or local
/// storage cannot preserve immutable source and normalized bytes.
pub fn import_gpudiff_snapshot_v1(
    data_root: &Path,
    raw_bytes: &[u8],
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<GpudiffImportV1, DataError> {
    if raw_bytes.is_empty() || raw_bytes.len() > GPUDIFF_MAX_BODY_BYTES {
        return Err(DataError::LimitExceeded);
    }
    let raw_source_digest = sha256(raw_bytes);
    let normalized = normalize_gpudiff_feed(raw_bytes, raw_source_digest)?;
    if normalized.bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(DataError::LimitExceeded);
    }

    ensure_data_root(data_root)?;
    let root = fs::canonicalize(data_root).map_err(DataError::Storage)?;
    let lock_path = root.join("gpudiff.lock");
    reject_path_symlinks(&root, &lock_path)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(DataError::Storage)?;
    lock.lock_exclusive().map_err(DataError::Storage)?;
    let result = import_gpudiff_snapshot_locked(
        &root,
        raw_bytes,
        raw_source_digest,
        normalized,
        etag,
        last_modified,
    );
    lock.unlock().map_err(DataError::Storage)?;
    result
}

struct NormalizedGpudiffFeed {
    bytes: Vec<u8>,
    row_count: u64,
    observed_at_min: String,
    observed_at_max: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GpudiffOfferRaw {
    id: String,
    provider: String,
    sku: String,
    price: Box<serde_json::value::RawValue>,
    unit: String,
    pricing_type: String,
    region: String,
    attrs: Box<serde_json::value::RawValue>,
    provenance: GpudiffProvenanceRaw,
    fixture: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GpudiffProvenanceRaw {
    url: String,
    observed_at: String,
}

fn normalize_gpudiff_feed(
    raw_bytes: &[u8],
    raw_source_digest: Sha256Digest,
) -> Result<NormalizedGpudiffFeed, DataError> {
    let raw = std::str::from_utf8(raw_bytes).map_err(|_| DataError::InvalidCatalog)?;
    let rows: Vec<Box<serde_json::value::RawValue>> =
        serde_json::from_str(raw).map_err(|_| DataError::InvalidCatalog)?;
    if rows.is_empty() || rows.len() > GPUDIFF_MAX_ROWS {
        return Err(DataError::LimitExceeded);
    }

    let mut output = Vec::with_capacity(raw_bytes.len());
    let mut ids = BTreeSet::new();
    let mut observed_at_min: Option<String> = None;
    let mut observed_at_max: Option<String> = None;
    for raw_row in &rows {
        let offer: GpudiffOfferRaw =
            serde_json::from_str(raw_row.get()).map_err(|_| DataError::InvalidCatalog)?;
        if offer.fixture
            || offer.id.is_empty()
            || offer.id.len() > 256
            || !ids.insert(offer.id.clone())
            || offer.provider.is_empty()
            || offer.provider.len() > 128
            || offer.sku.is_empty()
            || offer.sku.len() > 512
            || offer.unit.is_empty()
            || offer.unit.len() > 128
            || offer.pricing_type.is_empty()
            || offer.pricing_type.len() > 128
            || offer.region.len() > 128
            || !offer.provenance.url.starts_with("https://")
            || offer.provenance.url.len() > 2_048
            || offer.provenance.observed_at.is_empty()
            || offer.provenance.observed_at.len() > 64
        {
            return Err(DataError::InvalidCatalog);
        }

        let price_decimal = offer.price.get();
        let _: serde_json::Number =
            serde_json::from_str(price_decimal).map_err(|_| DataError::InvalidCatalog)?;
        let attrs: serde_json::Value =
            serde_json::from_str(offer.attrs.get()).map_err(|_| DataError::InvalidCatalog)?;
        let attrs_object = attrs.as_object().ok_or(DataError::InvalidCatalog)?;
        let attrs_field = |key: &str| -> Option<String> {
            attrs_object.get(key).and_then(|value| match value {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
        };
        let price_micros = exact_micros(price_decimal);
        let normalized_row = json!({
            "id": offer.id,
            "provider": offer.provider,
            "sku": offer.sku,
            "gpu_model": attrs_field("gpu_model"),
            "price_decimal": price_decimal,
            "price_micros_per_source_unit": price_micros,
            "unit": offer.unit,
            "pricing_type": offer.pricing_type,
            "region": offer.region,
            "metric": attrs_field("metric"),
            "sample_size": attrs_field("sample_size"),
            "vram_gb": attrs_field("vram_gb"),
            "attrs_json": offer.attrs.get(),
            "provenance_url": offer.provenance.url,
            "observed_at": offer.provenance.observed_at,
            "source_raw_sha256": raw_source_digest.to_hex(),
            "fixture": false,
        });
        serde_json::to_writer(&mut output, &normalized_row)
            .map_err(|_| DataError::InvalidCatalog)?;
        output.push(b'\n');

        let observed_at = normalized_row["observed_at"]
            .as_str()
            .ok_or(DataError::InvalidCatalog)?
            .to_owned();
        if observed_at_min
            .as_ref()
            .is_none_or(|current| observed_at < *current)
        {
            observed_at_min = Some(observed_at.clone());
        }
        if observed_at_max
            .as_ref()
            .is_none_or(|current| observed_at > *current)
        {
            observed_at_max = Some(observed_at);
        }
    }

    Ok(NormalizedGpudiffFeed {
        bytes: output,
        row_count: u64::try_from(rows.len()).map_err(|_| DataError::LimitExceeded)?,
        observed_at_min: observed_at_min.ok_or(DataError::InvalidCatalog)?,
        observed_at_max: observed_at_max.ok_or(DataError::InvalidCatalog)?,
    })
}

fn exact_micros(decimal: &str) -> Option<u64> {
    if decimal.starts_with('-') || decimal.starts_with('+') {
        return None;
    }
    let (mantissa, exponent) = match decimal.find(['e', 'E']) {
        Some(index) => {
            let exponent = decimal.get(index + 1..)?.parse::<i32>().ok()?;
            (decimal.get(..index)?, exponent)
        }
        None => (decimal, 0),
    };
    if !(-9..=9).contains(&exponent) {
        return None;
    }
    let (whole, fraction) = match mantissa.split_once('.') {
        Some(parts) => parts,
        None => (mantissa, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let mut digits = format!("{whole}{fraction}");
    let scale = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    let micros_scale = 6_i32.checked_sub(scale)?;
    if micros_scale < 0 {
        let discard = usize::try_from(-micros_scale).ok()?;
        if discard >= digits.len()
            || digits.as_bytes()[digits.len() - discard..]
                .iter()
                .any(|byte| *byte != b'0')
        {
            return None;
        }
        digits.truncate(digits.len() - discard);
    } else {
        let append = usize::try_from(micros_scale).ok()?;
        if digits.len().checked_add(append)? > 32 {
            return None;
        }
        digits.extend(std::iter::repeat_n('0', append));
    }
    digits.parse::<u64>().ok()
}

fn import_gpudiff_snapshot_locked(
    root: &Path,
    raw_bytes: &[u8],
    raw_source_digest: Sha256Digest,
    normalized: NormalizedGpudiffFeed,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<GpudiffImportV1, DataError> {
    let source_dir = root.join("sources").join("gpudiff");
    let snapshot_dir = root.join("snapshots");
    let receipt_dir = root.join("receipts").join("gpudiff");
    let manifests_dir = root.join("manifests");
    for directory in [
        &root.join("sources"),
        &source_dir,
        &snapshot_dir,
        &root.join("receipts"),
        &receipt_dir,
        &manifests_dir,
    ] {
        ensure_child_directory(root, directory)?;
    }
    let raw_name = format!("{}.json", raw_source_digest.to_hex());
    let raw_path = source_dir.join(&raw_name);
    let existing_capture = read_existing(&raw_path)?;
    if existing_capture.is_none() && count_json_files(&source_dir)? >= GPUDIFF_MAX_CAPTURES {
        return Err(DataError::CaptureRetentionFull);
    }
    atomic_create_or_compare(&raw_path, raw_bytes)?;

    let normalized_digest = sha256(&normalized.bytes);
    let normalized_path = format!("snapshots/gpudiff-{}.ndjson", normalized_digest.to_hex());
    let dataset_path = root.join(&normalized_path);
    atomic_create_or_compare(&dataset_path, &normalized.bytes)?;

    let receipt_path = receipt_dir.join(&raw_name);
    let proposed_receipt = GpudiffImportReceiptV1 {
        schema_version: "hyperai.data.gpudiff_import_receipt.v1".to_owned(),
        source_url: GPUDIFF_SOURCE_URL_V1.to_owned(),
        attribution: GPUDIFF_SOURCE_LABEL.to_owned(),
        raw_source_digest,
        normalized_digest,
        row_count: normalized.row_count,
        observed_at_min: normalized.observed_at_min,
        observed_at_max: normalized.observed_at_max,
        captured_unix_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| DataError::InvalidCatalog)?
            .as_secs(),
        retention_policy:
            "retain up to 16 distinct source bodies; fail closed at capacity; no automatic deletion"
                .to_owned(),
        etag: bounded_header_value(etag)?,
        last_modified: bounded_header_value(last_modified)?,
    };
    let receipt = if let Some(bytes) = read_existing(&receipt_path)? {
        let existing: GpudiffImportReceiptV1 =
            serde_json::from_slice(&bytes).map_err(|_| DataError::InvalidCatalog)?;
        if existing.schema_version != proposed_receipt.schema_version
            || existing.source_url != proposed_receipt.source_url
            || existing.attribution != proposed_receipt.attribution
            || existing.raw_source_digest != proposed_receipt.raw_source_digest
            || existing.normalized_digest != proposed_receipt.normalized_digest
            || existing.row_count != proposed_receipt.row_count
            || existing.observed_at_min != proposed_receipt.observed_at_min
            || existing.observed_at_max != proposed_receipt.observed_at_max
            || existing.retention_policy != proposed_receipt.retention_policy
        {
            return Err(DataError::Conflict);
        }
        existing
    } else {
        let bytes = serde_json::to_vec(&proposed_receipt).map_err(|_| DataError::InvalidCatalog)?;
        atomic_create_or_compare(&receipt_path, &bytes)?;
        proposed_receipt
    };

    let manifest = DatasetManifestV1 {
        schema_version: MANIFEST_SCHEMA.to_owned(),
        dataset_id: GPUDIFF_DATASET_ID.to_owned(),
        format: "json".to_owned(),
        relative_path: normalized_path,
        byte_length: fs::metadata(&dataset_path)
            .map_err(DataError::Storage)?
            .len(),
        content_digest: normalized_digest,
        schema_digest: sha256(GPUDIFF_SCHEMA.as_bytes()),
        row_count: normalized.row_count,
        source_label: GPUDIFF_SOURCE_LABEL.to_owned(),
        snapshot_at: receipt.observed_at_max.clone(),
        freshness: GPUDIFF_FRESHNESS.to_owned(),
        claim_ceiling: GPUDIFF_CLAIM_CEILING.to_owned(),
    };
    let manifest_bytes = serde_json::to_vec(&manifest).map_err(|_| DataError::InvalidCatalog)?;
    let manifest_path = manifests_dir.join(format!("{GPUDIFF_DATASET_ID}.json"));
    atomic_replace(&manifest_path, &manifest_bytes)?;
    Ok(GpudiffImportV1 { manifest, receipt })
}

fn bounded_header_value(value: Option<&str>) -> Result<Option<String>, DataError> {
    value
        .map(|value| {
            if value.len() > 512 || value.chars().any(char::is_control) {
                Err(DataError::InvalidCatalog)
            } else {
                Ok(value.to_owned())
            }
        })
        .transpose()
}

fn count_json_files(directory: &Path) -> Result<usize, DataError> {
    let mut count = 0_usize;
    for entry in fs::read_dir(directory).map_err(DataError::Storage)? {
        let entry = entry.map_err(DataError::Storage)?;
        if entry.file_type().map_err(DataError::Storage)?.is_symlink() {
            return Err(DataError::InvalidCatalog);
        }
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            count = count.checked_add(1).ok_or(DataError::LimitExceeded)?;
        }
    }
    Ok(count)
}

/// Apply the fixed local seed after a digest-bound preview and idempotency key.
///
/// # Errors
///
/// Returns an error if the preview changed, the key conflicts, or local bytes
/// cannot be safely created without overwriting existing content.
pub fn seed_synthetic_confirmed(
    data_root: &Path,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DatasetManifestV1, DataError> {
    validate_idempotency_key(idempotency_key)?;
    let preview = synthetic_seed_preview()?;
    if preview.preview_digest != preview_digest {
        return Err(DataError::Conflict);
    }
    ensure_data_root(data_root)?;
    let data_root = fs::canonicalize(data_root).map_err(DataError::Storage)?;
    let lock_path = data_root.join("definitions.lock");
    reject_path_symlinks(&data_root, &lock_path)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(DataError::Storage)?;
    lock.lock_exclusive().map_err(DataError::Storage)?;
    let result = seed_synthetic_locked(&data_root, idempotency_key, preview_digest);
    lock.unlock().map_err(DataError::Storage)?;
    result
}

fn seed_synthetic_locked(
    data_root: &Path,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DatasetManifestV1, DataError> {
    let key_digest = sha256(idempotency_key.as_bytes()).to_hex();
    let operations_dir = data_root.join("operations");
    let operation_path = operations_dir.join(format!("{key_digest}.json"));
    let operation = json!({
        "request_digest": preview_digest,
        "kind": "synthetic_seed",
        "name": "synthetic-market-observations",
    });
    let operation_bytes =
        serde_json::to_vec(&operation).map_err(|_| DataError::DefinitionEncoding)?;
    if let Some(existing) = read_existing(&operation_path)? {
        if existing != operation_bytes {
            return Err(DataError::Conflict);
        }
        return load_manifest(data_root, JSON_DATASET_ID);
    }
    let manifest = seed_synthetic(data_root)?;
    ensure_child_directory(data_root, &operations_dir)?;
    atomic_create_or_compare(&operation_path, &operation_bytes)?;
    Ok(manifest)
}

fn build_manifest(
    dataset_id: &str,
    format: &str,
    relative_path: &str,
    contents: &[u8],
    schema_digest: Sha256Digest,
    row_count: u64,
) -> DatasetManifestV1 {
    DatasetManifestV1 {
        schema_version: MANIFEST_SCHEMA.to_owned(),
        dataset_id: dataset_id.to_owned(),
        format: format.to_owned(),
        relative_path: relative_path.to_owned(),
        byte_length: contents.len() as u64,
        content_digest: sha256(contents),
        schema_digest,
        row_count,
        source_label: "HyperAI deterministic synthetic sample".to_owned(),
        snapshot_at: "2026-01-01T00:00:00Z".to_owned(),
        freshness: "synthetic_not_market_data".to_owned(),
        claim_ceiling: "synthetic local query behavior only; no live price, availability, seller, or execution claim".to_owned(),
    }
}

fn write_manifest(directory: &Path, manifest: &DatasetManifestV1) -> Result<(), DataError> {
    let bytes = serde_json::to_vec(manifest).map_err(|_| DataError::InvalidCatalog)?;
    atomic_create_or_compare(
        &directory.join(format!("{}.json", manifest.dataset_id)),
        &bytes,
    )
}

fn synthetic_arrow_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("instance_id", DataType::Utf8, false),
        Field::new("gpu_model", DataType::Utf8, false),
        Field::new("region", DataType::Utf8, false),
        Field::new("asking_usd_micros_per_hour", DataType::UInt64, false),
        Field::new("available_count", DataType::UInt64, true),
        Field::new("observed_at", DataType::Utf8, false),
    ]))
}

fn gpudiff_arrow_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("provider", DataType::Utf8, false),
        Field::new("sku", DataType::Utf8, false),
        Field::new("gpu_model", DataType::Utf8, true),
        Field::new("price_decimal", DataType::Utf8, false),
        Field::new("price_micros_per_source_unit", DataType::UInt64, true),
        Field::new("unit", DataType::Utf8, false),
        Field::new("pricing_type", DataType::Utf8, false),
        Field::new("region", DataType::Utf8, false),
        Field::new("metric", DataType::Utf8, true),
        Field::new("sample_size", DataType::Utf8, true),
        Field::new("vram_gb", DataType::Utf8, true),
        Field::new("attrs_json", DataType::Utf8, false),
        Field::new("provenance_url", DataType::Utf8, false),
        Field::new("observed_at", DataType::Utf8, false),
        Field::new("source_raw_sha256", DataType::Utf8, false),
        Field::new("fixture", DataType::Boolean, false),
    ]))
}

fn parquet_sample_bytes() -> Result<Vec<u8>, DataError> {
    use datafusion::parquet::arrow::ArrowWriter;

    let schema = synthetic_arrow_schema();
    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(StringArray::from(vec![
                "synthetic-a100-01",
                "synthetic-h100-01",
                "synthetic-mi300-01",
            ])),
            Arc::new(StringArray::from(vec!["A100", "H100", "MI300X"])),
            Arc::new(StringArray::from(vec!["us-east", "us-west", "eu-west"])),
            Arc::new(UInt64Array::from(vec![1_200_000, 3_500_000, 2_100_000])),
            Arc::new(UInt64Array::from(vec![Some(8), Some(2), None])),
            Arc::new(StringArray::from(vec![
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
            ])),
        ],
    )
    .map_err(|_| DataError::FixtureEncoding)?;
    let mut writer =
        ArrowWriter::try_new(Vec::new(), schema, None).map_err(|_| DataError::FixtureEncoding)?;
    writer
        .write(&batch)
        .map_err(|_| DataError::FixtureEncoding)?;
    writer.into_inner().map_err(|_| DataError::FixtureEncoding)
}

fn dataset_spec(dataset_id: &str) -> Result<(&'static str, Option<&'static str>), DataError> {
    match dataset_id {
        JSON_DATASET_ID => Ok(("json", Some(SAMPLE_FILE))),
        PARQUET_DATASET_ID => Ok(("parquet", Some(PARQUET_FILE))),
        GPUDIFF_DATASET_ID => Ok(("json", None)),
        _ => Err(DataError::DatasetNotFound),
    }
}

fn empty_catalog() -> DataCatalogV1 {
    DataCatalogV1 {
        schema_version: "hyperai.data.catalog.v1",
        status: "empty",
        datasets: Vec::new(),
        effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    }
}

/// Return a verified local catalog; a missing root is an empty catalog.
///
/// # Errors
///
/// Returns an error if the root or manifest is unsafe or the snapshot digest
/// does not match the bytes on disk, or the manifest directory exceeds its
/// supported entry count.
pub fn catalog(data_root: &Path) -> Result<DataCatalogV1, DataError> {
    if !path_is_present(data_root)? {
        return Ok(empty_catalog());
    }
    let root = checked_root(data_root)?;
    let manifests_dir = root.join("manifests");
    if !path_is_present(&manifests_dir)? {
        return Ok(empty_catalog());
    }
    reject_path_symlinks(&root, &manifests_dir)?;
    let mut dataset_ids = BTreeSet::new();
    for (entry_index, entry) in fs::read_dir(&manifests_dir)
        .map_err(DataError::Storage)?
        .enumerate()
    {
        let entry = entry.map_err(DataError::Storage)?;
        if entry_index >= MAX_MANIFEST_DIRECTORY_ENTRIES {
            return Err(DataError::InvalidCatalog);
        }
        if entry.file_type().map_err(DataError::Storage)?.is_symlink() {
            return Err(DataError::InvalidCatalog);
        }
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            if entry.metadata().map_err(DataError::Storage)?.len() > 16 * 1024 {
                return Err(DataError::LimitExceeded);
            }
            let manifest_bytes = fs::read(entry.path()).map_err(DataError::Storage)?;
            let manifest: DatasetManifestV1 =
                serde_json::from_slice(&manifest_bytes).map_err(|_| DataError::InvalidCatalog)?;
            if dataset_spec(&manifest.dataset_id).is_err() {
                return Err(DataError::InvalidCatalog);
            }
            if dataset_ids.contains(&manifest.dataset_id)
                || dataset_ids.len() >= MAX_CATALOG_DATASET_IDS
            {
                return Err(DataError::InvalidCatalog);
            }
            dataset_ids.insert(manifest.dataset_id);
        }
    }
    if dataset_ids.is_empty() {
        return Ok(empty_catalog());
    }
    let manifests = dataset_ids
        .iter()
        .map(|dataset_id| load_manifest(&root, dataset_id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DataCatalogV1 {
        schema_version: "hyperai.data.catalog.v1",
        status: "ready",
        datasets: manifests
            .into_iter()
            .map(|manifest| DatasetCatalogEntryV1 {
                dataset_id: manifest.dataset_id,
                format: manifest.format,
                source_label: manifest.source_label,
                snapshot_digest: manifest.content_digest,
                schema_digest: manifest.schema_digest,
                row_count: manifest.row_count,
                freshness: manifest.freshness,
                claim_ceiling: manifest.claim_ceiling,
            })
            .collect(),
        effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    })
}

/// Inspect one local dataset using the catalog's normal manifest and byte
/// verification. The return value contains no raw filesystem error or path.
#[must_use]
pub fn inspect_dataset_status_v1(data_root: &Path, dataset_id: &str) -> DatasetReadStatusV1 {
    if dataset_spec(dataset_id).is_err() {
        return DatasetReadStatusV1::Unavailable;
    }

    match catalog(data_root) {
        Ok(catalog) => catalog
            .datasets
            .iter()
            .find(|entry| entry.dataset_id == dataset_id)
            .map_or(DatasetReadStatusV1::NotSeeded, |entry| {
                DatasetReadStatusV1::Bound(entry.snapshot_digest)
            }),
        Err(error) => dataset_status_for_error(error),
    }
}

fn dataset_status_for_error(error: DataError) -> DatasetReadStatusV1 {
    match error {
        DataError::Storage(error) if error.kind() == io::ErrorKind::NotFound => {
            DatasetReadStatusV1::Missing
        }
        DataError::Storage(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            DatasetReadStatusV1::Denied
        }
        DataError::InvalidCatalog
        | DataError::DatasetNotFound
        | DataError::IntegrityMismatch
        | DataError::Conflict => DatasetReadStatusV1::Conflicting,
        _ => DatasetReadStatusV1::Unavailable,
    }
}

fn path_is_present(path: &Path) -> Result<bool, DataError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(DataError::Storage(error)),
    }
}

/// Read one verified manifest for the fixed local synthetic dataset.
///
/// # Errors
///
/// Returns an error for unknown dataset IDs, unsafe paths, or changed bytes.
pub fn describe(data_root: &Path, dataset_id: &str) -> Result<DatasetManifestV1, DataError> {
    load_manifest(data_root, dataset_id)
}

/// List saved models and mark whether their input snapshot remains current.
///
/// # Errors
///
/// Returns an error if the local catalog or a saved model definition is invalid.
pub fn saved_models(data_root: &Path) -> Result<DataModelCatalogV1, DataError> {
    let catalog = catalog(data_root)?;
    let mut models = Vec::new();
    for (name, bytes) in definition_files(data_root, "models")? {
        let model: DataModelV1 =
            serde_json::from_slice(&bytes).map_err(|_| DataError::InvalidCatalog)?;
        validate_name(&model.model_id)?;
        validate_query(&QueryRequestV1 {
            dataset_id: model.dataset_id.clone(),
            sql: model.sql.clone(),
            format: QueryFormat::Json,
            max_rows: Some(1),
        })?;
        if model.schema_version != "hyperai.data.model.v1" || model.model_id != name {
            return Err(DataError::InvalidCatalog);
        }
        let input_state = catalog
            .datasets
            .iter()
            .find(|dataset| dataset.dataset_id == model.dataset_id)
            .map_or_else(
                || "missing".to_owned(),
                |dataset| {
                    if dataset.snapshot_digest == model.snapshot_digest {
                        "current".to_owned()
                    } else {
                        "stale".to_owned()
                    }
                },
            );
        models.push(DataModelRecordV1 {
            model,
            definition_digest: sha256(&bytes),
            input_state,
        });
    }
    Ok(DataModelCatalogV1 {
        schema_version: "hyperai.data.models.v1",
        models,
        effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    })
}

/// List saved views and indicate whether each still links to a saved model.
///
/// # Errors
///
/// Returns an error if saved model or view definitions are invalid.
pub fn saved_views(data_root: &Path) -> Result<DataViewCatalogV1, DataError> {
    let models = saved_models(data_root)?;
    let mut views = Vec::new();
    for (name, bytes) in definition_files(data_root, "views")? {
        let view: DataViewV1 =
            serde_json::from_slice(&bytes).map_err(|_| DataError::InvalidCatalog)?;
        validate_name(&view.view_id)?;
        if view.schema_version != "hyperai.data.view.v1"
            || view.view_id != name
            || !matches!(view.presentation.as_str(), "table" | "bar" | "line")
            || view.columns.is_empty()
            || view.columns.len() > MAX_COLUMNS
            || view.columns.iter().any(|column| !valid_identifier(column))
        {
            return Err(DataError::InvalidCatalog);
        }
        let model_state = if models
            .models
            .iter()
            .any(|model| model.definition_digest == view.model_digest)
        {
            "linked"
        } else {
            "orphaned"
        };
        views.push(DataViewRecordV1 {
            view,
            definition_digest: sha256(&bytes),
            model_state: model_state.to_owned(),
        });
    }
    Ok(DataViewCatalogV1 {
        schema_version: "hyperai.data.views.v1",
        views,
        effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    })
}

/// Execute one validated, bounded query over a verified JSON or Parquet file.
///
/// # Errors
///
/// Returns a generic error when SQL, local bytes, query planning, execution,
/// cancellation, time, memory, row, or output limits fail.
pub async fn query(data_root: &Path, request: &QueryRequestV1) -> Result<QueryResultV1, DataError> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    let result = query_with_cancellation(data_root, request, receiver).await;
    drop(sender);
    result
}

/// Execute one read-only query with an explicit cancellation receiver.
///
/// Dropping the execution future or changing the watched boolean to `true`
/// stops returning results and drops the active `DataFusion` stream.
///
/// # Errors
///
/// Returns the same errors as [`query`] plus [`DataError::Cancelled`].
pub async fn query_with_cancellation(
    data_root: &Path,
    request: &QueryRequestV1,
    cancellation: tokio::sync::watch::Receiver<bool>,
) -> Result<QueryResultV1, DataError> {
    validate_query(request)?;
    if *cancellation.borrow() {
        return Err(DataError::Cancelled);
    }
    let mut cancellation_monitor = cancellation.clone();
    let execution = async {
        preflight_query_plan(request).await?;
        let snapshot = capture_dataset_snapshot(data_root, &request.dataset_id)?;
        execute_verified_snapshot_query(snapshot, request).await
    };
    await_bounded_query(execution, &mut cancellation_monitor, QUERY_TIMEOUT).await
}

fn capture_dataset_snapshot(
    data_root: &Path,
    dataset_id: &str,
) -> Result<VerifiedDatasetSnapshot, DataError> {
    let (manifest, data_path) = load_manifest_record(data_root, dataset_id)?;
    let bytes = read_verified_dataset_bytes(&data_path, &manifest)?;
    Ok(VerifiedDatasetSnapshot { manifest, bytes })
}

fn read_verified_dataset_bytes(
    data_path: &Path,
    manifest: &DatasetManifestV1,
) -> Result<Vec<u8>, DataError> {
    if manifest.byte_length > MAX_FILE_BYTES {
        return Err(DataError::LimitExceeded);
    }
    let file = File::open(data_path).map_err(DataError::Storage)?;
    let metadata = file.metadata().map_err(DataError::Storage)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(DataError::LimitExceeded);
    }
    if metadata.len() != manifest.byte_length {
        return Err(DataError::IntegrityMismatch);
    }
    let capacity = usize::try_from(manifest.byte_length).map_err(|_| DataError::LimitExceeded)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(DataError::Storage)?;
    if u64::try_from(bytes.len()).map_err(|_| DataError::LimitExceeded)? > MAX_FILE_BYTES {
        return Err(DataError::LimitExceeded);
    }
    if u64::try_from(bytes.len()).map_err(|_| DataError::LimitExceeded)? != manifest.byte_length
        || sha256(&bytes) != manifest.content_digest
    {
        return Err(DataError::IntegrityMismatch);
    }
    Ok(bytes)
}

async fn execute_verified_snapshot_query(
    snapshot: VerifiedDatasetSnapshot,
    request: &QueryRequestV1,
) -> Result<QueryResultV1, DataError> {
    if request.dataset_id != snapshot.manifest.dataset_id {
        return Err(DataError::InvalidCatalog);
    }
    let context = query_context_with_memory_limit(
        snapshot.bytes,
        &snapshot.manifest.format,
        QUERY_MEMORY_BYTES,
    )
    .await?;
    let (columns, rows) = execute_query(&context, request).await?;
    let csv = (request.format == QueryFormat::Csv).then(|| csv_encode(&columns, &rows));
    let output_bytes = serialized_size(request.format, &columns, &rows)?;
    let max_duration_ms =
        u64::try_from(QUERY_TIMEOUT.as_millis()).map_err(|_| DataError::LimitExceeded)?;
    let result = QueryResultV1 {
        schema_version: "hyperai.data.query-result.v1",
        dataset_id: snapshot.manifest.dataset_id,
        snapshot_digest: snapshot.manifest.content_digest,
        columns,
        row_count: rows.len(),
        rows,
        format: request.format,
        csv,
        output_bytes,
        limits: QueryLimitsV1 {
            max_input_bytes: MAX_FILE_BYTES,
            max_sql_bytes: MAX_SQL_BYTES,
            max_duration_ms,
            memory_bytes: QUERY_MEMORY_BYTES,
            partitions: 1,
            batch_rows: 1_024,
            max_result_rows: request.max_rows.unwrap_or(MAX_RESULT_ROWS),
            max_result_bytes: MAX_RESULT_BYTES,
        },
        claim_ceiling: "synthetic local query result only; no live source or compute claim",
        effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    };
    serialized_response_size(&result)?;
    Ok(result)
}

/// Derive deterministic Bronze/Silver/Gold metadata from the owned synthetic JSON snapshot.
///
/// # Errors
///
/// Returns an error for an absent, stale, unsafe, or malformed source, or when
/// the bounded source query fails.
pub async fn derive_synthetic_release(
    data_root: &Path,
) -> Result<SyntheticDataReleaseV1, DataError> {
    let manifest = load_manifest(data_root, JSON_DATASET_ID)?;
    let silver = read_synthetic_silver_layer(data_root, &manifest).await?;
    let gold = summarize_synthetic_silver_layer(&silver)?;
    let bronze = BronzeReferenceV1 {
        schema_version: "hyperai.data.bronze-reference.v1",
        dataset_id: manifest.dataset_id,
        snapshot_digest: manifest.content_digest,
        schema_digest: manifest.schema_digest,
        source_label: manifest.source_label,
        row_count: manifest.row_count,
        freshness: manifest.freshness,
        claim_ceiling: manifest.claim_ceiling,
    };
    let output_digest = synthetic_release_digest(&bronze, &silver, &gold)?;

    Ok(SyntheticDataReleaseV1 {
        schema_version: "hyperai.data.synthetic-release.v1",
        method_version: SYNTHETIC_DERIVATION_METHOD,
        bronze,
        silver,
        gold,
        output_digest,
        claim_ceiling: "HyperAI-authored synthetic observation derivation only; not live market data",
        effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
    })
}

async fn read_synthetic_silver_layer(
    data_root: &Path,
    manifest: &DatasetManifestV1,
) -> Result<Vec<NormalizedObservationV1>, DataError> {
    let result = query(
        data_root,
        &QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: "SELECT instance_id, gpu_model, region, asking_usd_micros_per_hour, available_count, observed_at FROM dataset ORDER BY instance_id".to_owned(),
            format: QueryFormat::Json,
            max_rows: Some(MAX_RESULT_ROWS),
        },
    )
    .await?;
    if result.snapshot_digest != manifest.content_digest
        || result.columns
            != [
                "instance_id",
                "gpu_model",
                "region",
                "asking_usd_micros_per_hour",
                "available_count",
                "observed_at",
            ]
    {
        return Err(DataError::InvalidCatalog);
    }
    let expected_rows =
        usize::try_from(manifest.row_count).map_err(|_| DataError::LimitExceeded)?;
    if result.rows.len() != expected_rows || result.rows.is_empty() {
        return Err(DataError::InvalidCatalog);
    }

    let mut silver = Vec::with_capacity(result.rows.len());
    for row in result.rows {
        if row.len() != 6 {
            return Err(DataError::InvalidCatalog);
        }
        let available_count = match row.get(4).ok_or(DataError::InvalidCatalog)? {
            Some(value) => Some(
                value
                    .parse::<u64>()
                    .map_err(|_| DataError::InvalidCatalog)?,
            ),
            None => None,
        };
        silver.push(NormalizedObservationV1 {
            schema_version: "hyperai.data.normalized_observation.v1",
            source_instance_id: required_cell(&row, 0)?.to_owned(),
            gpu_model: required_cell(&row, 1)?.to_owned(),
            region: required_cell(&row, 2)?.to_owned(),
            asking_usd_micros_per_hour: required_cell(&row, 3)?
                .parse::<u64>()
                .map_err(|_| DataError::InvalidCatalog)?,
            available_count,
            observed_at: required_cell(&row, 5)?.to_owned(),
            source_snapshot_digest: manifest.content_digest,
            freshness: manifest.freshness.clone(),
        });
    }
    Ok(silver)
}

fn summarize_synthetic_silver_layer(
    silver: &[NormalizedObservationV1],
) -> Result<Vec<SyntheticGroupSummaryV1>, DataError> {
    let mut groups: BTreeMap<(String, String), SyntheticGroupAccumulator> = BTreeMap::new();
    for observation in silver {
        let group = groups
            .entry((observation.gpu_model.clone(), observation.region.clone()))
            .or_default();
        group.record_count = checked_increment(group.record_count)?;
        group.min_price = Some(
            group
                .min_price
                .map_or(observation.asking_usd_micros_per_hour, |value| {
                    value.min(observation.asking_usd_micros_per_hour)
                }),
        );
        group.max_price = Some(
            group
                .max_price
                .map_or(observation.asking_usd_micros_per_hour, |value| {
                    value.max(observation.asking_usd_micros_per_hour)
                }),
        );
        match observation.available_count {
            Some(value) => {
                group.known_availability_records =
                    checked_increment(group.known_availability_records)?;
                group.known_available_units = group
                    .known_available_units
                    .checked_add(value)
                    .ok_or(DataError::LimitExceeded)?;
            }
            None => {
                group.unknown_availability_records =
                    checked_increment(group.unknown_availability_records)?;
            }
        }
    }

    let gold = groups
        .into_iter()
        .map(|((gpu_model, region), group)| {
            Ok(SyntheticGroupSummaryV1 {
                schema_version: "hyperai.data.synthetic_group_summary.v1",
                gpu_model,
                region,
                record_count: group.record_count,
                min_asking_usd_micros_per_hour: group.min_price.ok_or(DataError::InvalidCatalog)?,
                max_asking_usd_micros_per_hour: group.max_price.ok_or(DataError::InvalidCatalog)?,
                known_availability_records: group.known_availability_records,
                unknown_availability_records: group.unknown_availability_records,
                available_units_total: (group.unknown_availability_records == 0)
                    .then_some(group.known_available_units),
            })
        })
        .collect::<Result<Vec<_>, DataError>>()?;
    Ok(gold)
}

fn synthetic_release_digest(
    bronze: &BronzeReferenceV1,
    silver: &[NormalizedObservationV1],
    gold: &[SyntheticGroupSummaryV1],
) -> Result<Sha256Digest, DataError> {
    let payload = SyntheticReleaseDigestPayload {
        method_version: SYNTHETIC_DERIVATION_METHOD,
        bronze,
        silver,
        gold,
    };
    domain_separated_digest("hyperai.data.synthetic-release.v1", &payload)
        .map_err(|_| DataError::DefinitionEncoding)
}

#[derive(Default)]
struct SyntheticGroupAccumulator {
    record_count: u64,
    min_price: Option<u64>,
    max_price: Option<u64>,
    known_availability_records: u64,
    unknown_availability_records: u64,
    known_available_units: u64,
}

#[derive(Serialize)]
struct SyntheticReleaseDigestPayload<'a> {
    method_version: &'static str,
    bronze: &'a BronzeReferenceV1,
    silver: &'a [NormalizedObservationV1],
    gold: &'a [SyntheticGroupSummaryV1],
}

fn required_cell(row: &[Option<String>], index: usize) -> Result<&str, DataError> {
    row.get(index)
        .and_then(Option::as_deref)
        .ok_or(DataError::InvalidCatalog)
}

fn checked_increment(value: u64) -> Result<u64, DataError> {
    value.checked_add(1).ok_or(DataError::LimitExceeded)
}

async fn query_context_with_memory_limit(
    snapshot: Vec<u8>,
    format: &str,
    memory_limit_bytes: usize,
) -> Result<SessionContext, DataError> {
    let context = query_session_context(memory_limit_bytes)?;
    let object_store = Arc::new(InMemory::new());
    let object_path = ObjectStorePath::from(format!("dataset.{format}"));
    object_store
        .put(&object_path, snapshot.into())
        .await
        .map_err(|error| {
            DataError::QueryFailed(datafusion::error::DataFusionError::External(Box::new(
                error,
            )))
        })?;
    let store_url = ObjectStoreUrl::parse("memory://").map_err(map_query_engine_error)?;
    context.register_object_store(store_url.as_ref(), object_store);
    let table_path = format!("memory:///dataset.{format}");
    match format {
        "json" => context
            .register_json("dataset", table_path.as_str(), NdJsonReadOptions::default())
            .await
            .map_err(map_query_engine_error)?,
        "parquet" => context
            .register_parquet(
                "dataset",
                table_path.as_str(),
                ParquetReadOptions::default(),
            )
            .await
            .map_err(map_query_engine_error)?,
        _ => return Err(DataError::InvalidCatalog),
    }
    Ok(context)
}

fn query_session_context(memory_limit_bytes: usize) -> Result<SessionContext, DataError> {
    let config = SessionConfig::new()
        .with_target_partitions(1)
        .with_batch_size(1_024);
    let runtime_env = RuntimeEnvBuilder::new()
        .with_memory_limit(memory_limit_bytes, 1.0)
        .with_disk_manager_builder(
            datafusion::execution::disk_manager::DiskManagerBuilder::default()
                .with_mode(datafusion::execution::disk_manager::DiskManagerMode::Disabled),
        )
        .build_arc()
        .map_err(DataError::QueryFailed)?;
    let scalar_functions = SessionStateDefaults::default_scalar_functions()
        .into_iter()
        .filter(|function| matches!(function.name(), "abs" | "coalesce" | "round"))
        .collect();
    let aggregate_functions = SessionStateDefaults::default_aggregate_functions()
        .into_iter()
        .filter(|function| matches!(function.name(), "avg" | "count" | "max" | "min" | "sum"))
        .collect();
    let state = SessionStateBuilder::new()
        .with_config(config)
        .with_runtime_env(runtime_env)
        .with_expr_planners(SessionStateDefaults::default_expr_planners())
        .with_scalar_functions(scalar_functions)
        .with_aggregate_functions(aggregate_functions)
        .build();
    Ok(SessionContext::new_with_state(state))
}

async fn preflight_query_plan(request: &QueryRequestV1) -> Result<(), DataError> {
    let context = query_session_context(QUERY_MEMORY_BYTES)?;
    let schema = if request.dataset_id == GPUDIFF_DATASET_ID {
        gpudiff_arrow_schema()
    } else {
        synthetic_arrow_schema()
    };
    context
        .register_table("dataset", Arc::new(EmptyTable::new(schema)))
        .map_err(|error| map_query_preflight_error(&error))?;
    let dataframe = context
        .sql(&request.sql)
        .await
        .map_err(|error| map_query_preflight_error(&error))?;
    dataframe
        .create_physical_plan()
        .await
        .map_err(|error| map_query_preflight_error(&error))?;
    Ok(())
}

async fn await_bounded_query<T, F>(
    execution: F,
    cancellation: &mut tokio::sync::watch::Receiver<bool>,
    timeout_duration: Duration,
) -> Result<T, DataError>
where
    F: Future<Output = Result<T, DataError>>,
{
    tokio::select! {
        biased;
        _ = cancellation.changed() => Err(DataError::Cancelled),
        result = tokio::time::timeout(timeout_duration, execution) => {
            result.map_err(|_| DataError::LimitExceeded)?
        }
    }
}

async fn execute_query(
    context: &SessionContext,
    request: &QueryRequestV1,
) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), DataError> {
    let dataframe = context
        .sql(&request.sql)
        .await
        .map_err(map_query_engine_error)?;
    let stream = dataframe
        .execute_stream()
        .await
        .map_err(map_query_engine_error)?;
    collect_query_rows(
        stream,
        request.format,
        request.max_rows.unwrap_or(MAX_RESULT_ROWS),
    )
    .await
}

async fn collect_query_rows(
    mut stream: SendableRecordBatchStream,
    format: QueryFormat,
    row_limit: usize,
) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), DataError> {
    let mut columns: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut output_bytes = None;
    while let Some(batch) = futures_util::stream::StreamExt::next(&mut stream).await {
        let batch = batch.map_err(map_query_engine_error)?;
        if batch.num_columns() > MAX_COLUMNS {
            return Err(DataError::LimitExceeded);
        }
        if columns.is_none() {
            columns = Some(
                batch
                    .schema()
                    .fields()
                    .iter()
                    .map(|field| field.name().clone())
                    .collect(),
            );
        }
        if rows.len().saturating_add(batch.num_rows()) > row_limit {
            return Err(DataError::LimitExceeded);
        }
        let output_columns = columns.as_deref().unwrap_or_default();
        if output_bytes.is_none() {
            output_bytes = Some(serialized_size(format, output_columns, &rows)?);
        }
        let Some(output_bytes) = output_bytes.as_mut() else {
            return Err(DataError::InvalidCatalog);
        };
        append_batch_rows(&batch, format, output_columns, &mut rows, output_bytes)?;
    }
    Ok((columns.unwrap_or_default(), rows))
}

fn map_query_engine_error(error: datafusion::error::DataFusionError) -> DataError {
    if matches!(
        error.find_root(),
        datafusion::error::DataFusionError::ResourcesExhausted(_)
    ) {
        DataError::LimitExceeded
    } else {
        DataError::QueryFailed(error)
    }
}

fn map_query_preflight_error(error: &datafusion::error::DataFusionError) -> DataError {
    if matches!(
        error.find_root(),
        datafusion::error::DataFusionError::ResourcesExhausted(_)
    ) {
        DataError::LimitExceeded
    } else {
        DataError::QueryRejected
    }
}

fn append_batch_rows(
    batch: &RecordBatch,
    format: QueryFormat,
    columns: &[String],
    rows: &mut Vec<Vec<Option<String>>>,
    output_bytes: &mut usize,
) -> Result<(), DataError> {
    if batch.num_columns() != columns.len() {
        return Err(DataError::InvalidCatalog);
    }
    let format_options = FormatOptions::default().with_display_error(true);
    let formatters = batch
        .columns()
        .iter()
        .map(|column| {
            ArrayFormatter::try_new(column.as_ref(), &format_options)
                .map_err(|_| DataError::QueryRejected)
        })
        .collect::<Result<Vec<_>, _>>()?;

    for row_index in 0..batch.num_rows() {
        let row_separator_bytes = usize::from(format == QueryFormat::Json && !rows.is_empty());
        let row_container_bytes = match format {
            QueryFormat::Json => 2,
            QueryFormat::Csv => 1,
        };
        let mut row_bytes = row_container_bytes;
        let mut row = Vec::with_capacity(batch.num_columns());

        let initial_size = checked_size_add(*output_bytes, row_separator_bytes)?;
        if checked_size_add(initial_size, row_bytes)? > MAX_RESULT_BYTES {
            return Err(DataError::LimitExceeded);
        }

        for (column_index, (column, formatter)) in
            batch.columns().iter().zip(&formatters).enumerate()
        {
            if column_index > 0 {
                row_bytes = checked_size_add(row_bytes, 1)?;
            }
            let size_before_cell = checked_size_add(
                checked_size_add(*output_bytes, row_separator_bytes)?,
                row_bytes,
            )?;
            let remaining_bytes = MAX_RESULT_BYTES
                .checked_sub(size_before_cell)
                .ok_or(DataError::LimitExceeded)?;

            let value = if column.is_null(row_index) {
                let null_bytes = match format {
                    QueryFormat::Json => 4,
                    QueryFormat::Csv => 0,
                };
                row_bytes = checked_size_add(row_bytes, null_bytes)?;
                None
            } else {
                let cell_bytes =
                    formatted_cell_size(formatter, row_index, format, remaining_bytes)?;
                row_bytes = checked_size_add(row_bytes, cell_bytes)?;
                Some(formatter.value(row_index).to_string())
            };

            let next_size = checked_size_add(
                checked_size_add(*output_bytes, row_separator_bytes)?,
                row_bytes,
            )?;
            if next_size > MAX_RESULT_BYTES {
                return Err(DataError::LimitExceeded);
            }
            row.push(value);
        }

        let next_size = checked_size_add(
            checked_size_add(*output_bytes, row_separator_bytes)?,
            row_bytes,
        )?;
        if next_size > MAX_RESULT_BYTES {
            return Err(DataError::LimitExceeded);
        }
        rows.push(row);
        *output_bytes = next_size;
    }
    Ok(())
}

fn checked_size_add(current: usize, additional: usize) -> Result<usize, DataError> {
    current
        .checked_add(additional)
        .ok_or(DataError::LimitExceeded)
}

fn formatted_cell_size(
    formatter: &ArrayFormatter<'_>,
    row_index: usize,
    format: QueryFormat,
    maximum_bytes: usize,
) -> Result<usize, DataError> {
    let mut counter = EncodedCellSize::new(format, maximum_bytes)?;
    let cell_display = formatter.value(row_index);
    if fmt::Write::write_fmt(&mut counter, format_args!("{cell_display}")).is_err() {
        return if counter.exceeded {
            Err(DataError::LimitExceeded)
        } else {
            Err(DataError::QueryRejected)
        };
    }
    counter.finish()
}

struct EncodedCellSize {
    format: QueryFormat,
    maximum_bytes: usize,
    encoded_bytes: usize,
    csv_raw_bytes: usize,
    csv_quote_bytes: usize,
    csv_requires_quotes: bool,
    exceeded: bool,
}

impl EncodedCellSize {
    fn new(format: QueryFormat, maximum_bytes: usize) -> Result<Self, DataError> {
        let encoded_bytes = usize::from(format == QueryFormat::Json) * 2;
        if encoded_bytes > maximum_bytes {
            return Err(DataError::LimitExceeded);
        }
        Ok(Self {
            format,
            maximum_bytes,
            encoded_bytes,
            csv_raw_bytes: 0,
            csv_quote_bytes: 0,
            csv_requires_quotes: false,
            exceeded: false,
        })
    }

    fn finish(self) -> Result<usize, DataError> {
        let encoded_bytes = match self.format {
            QueryFormat::Json => self.encoded_bytes,
            QueryFormat::Csv => checked_size_add(
                checked_size_add(self.csv_raw_bytes, self.csv_quote_bytes)?,
                usize::from(self.csv_requires_quotes) * 2,
            )?,
        };
        if encoded_bytes > self.maximum_bytes {
            Err(DataError::LimitExceeded)
        } else {
            Ok(encoded_bytes)
        }
    }
}

impl fmt::Write for EncodedCellSize {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        match self.format {
            QueryFormat::Json => {
                for byte in value.bytes() {
                    let escaped_byte = match byte {
                        b'"' | b'\\' | b'\x08' | b'\t' | b'\n' | b'\x0c' | b'\r' => 2,
                        0x00..=0x1f => 6,
                        _ => 1,
                    };
                    let Some(next_size) = self.encoded_bytes.checked_add(escaped_byte) else {
                        self.exceeded = true;
                        return Err(fmt::Error);
                    };
                    if next_size > self.maximum_bytes {
                        self.exceeded = true;
                        return Err(fmt::Error);
                    }
                    self.encoded_bytes = next_size;
                }
            }
            QueryFormat::Csv => {
                for byte in value.bytes() {
                    let Some(raw_bytes) = self.csv_raw_bytes.checked_add(1) else {
                        self.exceeded = true;
                        return Err(fmt::Error);
                    };
                    self.csv_raw_bytes = raw_bytes;
                    if byte == b'"' {
                        let Some(quote_bytes) = self.csv_quote_bytes.checked_add(1) else {
                            self.exceeded = true;
                            return Err(fmt::Error);
                        };
                        self.csv_quote_bytes = quote_bytes;
                    }
                    if matches!(byte, b',' | b'"' | b'\n' | b'\r') {
                        self.csv_requires_quotes = true;
                    }
                    let encoded_bytes = self
                        .csv_raw_bytes
                        .checked_add(self.csv_quote_bytes)
                        .and_then(|size| {
                            size.checked_add(usize::from(self.csv_requires_quotes) * 2)
                        });
                    if encoded_bytes.is_none_or(|size| size > self.maximum_bytes) {
                        self.exceeded = true;
                        return Err(fmt::Error);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Preview a model write and bind it to a current local dataset snapshot.
///
/// # Errors
///
/// Returns an error if the model is malformed, SQL is outside the supported
/// subset, or the bound snapshot is stale.
pub fn preview_model(
    data_root: &Path,
    model: &DataModelV1,
) -> Result<DefinitionPreviewV1, DataError> {
    if model.schema_version != "hyperai.data.model.v1" {
        return Err(DataError::QueryRejected);
    }
    validate_name(&model.model_id)?;
    validate_query(&QueryRequestV1 {
        dataset_id: model.dataset_id.clone(),
        sql: model.sql.clone(),
        format: QueryFormat::Json,
        max_rows: Some(1),
    })?;
    let manifest = load_manifest(data_root, &model.dataset_id)?;
    if manifest.content_digest != model.snapshot_digest {
        return Err(DataError::IntegrityMismatch);
    }
    definition_preview("model", &model.model_id, model)
}

/// Preview a view definition after validating its fields and name.
///
/// # Errors
///
/// Returns an error if the view is malformed or names invalid fields.
pub fn preview_view(data_root: &Path, view: &DataViewV1) -> Result<DefinitionPreviewV1, DataError> {
    if view.schema_version != "hyperai.data.view.v1"
        || !matches!(view.presentation.as_str(), "table" | "bar" | "line")
        || view.columns.is_empty()
        || view.columns.len() > MAX_COLUMNS
        || view.columns.iter().any(|column| !valid_identifier(column))
    {
        return Err(DataError::QueryRejected);
    }
    validate_name(&view.view_id)?;
    if !saved_model_matches(data_root, view.model_digest)? {
        return Err(DataError::DatasetNotFound);
    }
    definition_preview("view", &view.view_id, view)
}

/// Save a previously previewed model after a validated idempotency key.
///
/// # Errors
///
/// Returns an error if the preview changed or the filesystem conflicts.
pub fn save_model(
    data_root: &Path,
    model: &DataModelV1,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DefinitionSavedV1, DataError> {
    let preview = preview_model(data_root, model)?;
    if preview.preview_digest != preview_digest {
        return Err(DataError::Conflict);
    }
    save_definition(
        data_root,
        "models",
        "model",
        &model.model_id,
        idempotency_key,
        model,
    )
}

/// Save a previously previewed view after a validated idempotency key.
///
/// # Errors
///
/// Returns an error if the preview changed or the filesystem conflicts.
pub fn save_view(
    data_root: &Path,
    view: &DataViewV1,
    idempotency_key: &str,
    preview_digest: Sha256Digest,
) -> Result<DefinitionSavedV1, DataError> {
    let preview = preview_view(data_root, view)?;
    if preview.preview_digest != preview_digest {
        return Err(DataError::Conflict);
    }
    save_definition(
        data_root,
        "views",
        "view",
        &view.view_id,
        idempotency_key,
        view,
    )
}

fn definition_preview<T: Serialize>(
    kind: &str,
    name: &str,
    definition: &T,
) -> Result<DefinitionPreviewV1, DataError> {
    let preview_digest =
        domain_separated_digest(&format!("hyperai.data.{kind}-preview.v1"), definition)
            .map_err(|_| DataError::DefinitionEncoding)?;
    Ok(DefinitionPreviewV1 {
        schema_version: "hyperai.data.definition-preview.v1",
        preview_digest,
        definition_id: name.to_owned(),
        effect_boundary: "LOCAL_DEFINITION_WRITE_ONLY_NO_INPUT_OR_RESULT_PAYLOAD",
    })
}

fn save_definition<T: Serialize>(
    data_root: &Path,
    folder: &str,
    kind: &str,
    name: &str,
    idempotency_key: &str,
    definition: &T,
) -> Result<DefinitionSavedV1, DataError> {
    validate_idempotency_key(idempotency_key)?;
    ensure_data_root(data_root)?;
    let data_root = fs::canonicalize(data_root).map_err(DataError::Storage)?;
    let lock_path = data_root.join("definitions.lock");
    reject_path_symlinks(&data_root, &lock_path)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(DataError::Storage)?;
    lock.lock_exclusive().map_err(DataError::Storage)?;
    let result =
        save_definition_locked(&data_root, folder, kind, name, idempotency_key, definition);
    lock.unlock().map_err(DataError::Storage)?;
    result
}

fn save_definition_locked<T: Serialize>(
    data_root: &Path,
    folder: &str,
    kind: &str,
    name: &str,
    idempotency_key: &str,
    definition: &T,
) -> Result<DefinitionSavedV1, DataError> {
    let definition_bytes =
        serde_json::to_vec(definition).map_err(|_| DataError::DefinitionEncoding)?;
    let digest = sha256(&definition_bytes);
    let key_digest = sha256(idempotency_key.as_bytes()).to_hex();
    let operation_path = data_root
        .join("operations")
        .join(format!("{key_digest}.json"));
    let operation = json!({ "request_digest": digest, "kind": kind, "name": name });
    let operation_bytes =
        serde_json::to_vec(&operation).map_err(|_| DataError::DefinitionEncoding)?;
    if let Some(existing) = read_existing(&operation_path)? {
        if existing != operation_bytes {
            return Err(DataError::Conflict);
        }
        return Ok(DefinitionSavedV1 {
            schema_version: "hyperai.data.definition-saved.v1",
            definition_id: name.to_owned(),
            definition_digest: digest,
            effect_boundary: "LOCAL_DEFINITION_WRITE_ONLY_NO_INPUT_OR_RESULT_PAYLOAD",
        });
    }
    let target_dir = data_root.join(folder);
    ensure_child_directory(data_root, &target_dir)?;
    let target = target_dir.join(format!("{name}.json"));
    atomic_create_or_compare(&target, &definition_bytes)?;
    ensure_child_directory(
        data_root,
        operation_path.parent().ok_or(DataError::InvalidCatalog)?,
    )?;
    atomic_create_or_compare(&operation_path, &operation_bytes)?;
    Ok(DefinitionSavedV1 {
        schema_version: "hyperai.data.definition-saved.v1",
        definition_id: name.to_owned(),
        definition_digest: digest,
        effect_boundary: "LOCAL_DEFINITION_WRITE_ONLY_NO_INPUT_OR_RESULT_PAYLOAD",
    })
}

fn saved_model_matches(data_root: &Path, expected_digest: Sha256Digest) -> Result<bool, DataError> {
    Ok(saved_models(data_root)?
        .models
        .iter()
        .any(|model| model.definition_digest == expected_digest))
}

fn definition_files(data_root: &Path, folder: &str) -> Result<Vec<(String, Vec<u8>)>, DataError> {
    if !data_root.exists() {
        reject_self_symlink(data_root)?;
        return Ok(Vec::new());
    }
    let root = checked_root(data_root)?;
    let directory = root.join(folder);
    reject_path_symlinks(&root, &directory)?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut definitions = Vec::new();
    for entry in fs::read_dir(&directory).map_err(DataError::Storage)? {
        let entry = entry.map_err(DataError::Storage)?;
        if entry.file_type().map_err(DataError::Storage)?.is_symlink() {
            return Err(DataError::InvalidCatalog);
        }
        if entry
            .path()
            .extension()
            .is_none_or(|extension| extension != "json")
        {
            continue;
        }
        let metadata = entry.metadata().map_err(DataError::Storage)?;
        if metadata.len() > 16 * 1024 {
            return Err(DataError::LimitExceeded);
        }
        let name = entry
            .path()
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or(DataError::InvalidCatalog)?
            .to_owned();
        let bytes = fs::read(entry.path()).map_err(DataError::Storage)?;
        definitions.push((name, bytes));
    }
    definitions.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(definitions)
}

fn validate_query(request: &QueryRequestV1) -> Result<(), DataError> {
    if dataset_spec(&request.dataset_id).is_err()
        || request.sql.trim().is_empty()
        || request.sql.len() > MAX_SQL_BYTES
        || request
            .max_rows
            .is_some_and(|rows| rows == 0 || rows > MAX_RESULT_ROWS)
    {
        return Err(DataError::LimitExceeded);
    }
    let statements = Parser::parse_sql(&GenericDialect {}, &request.sql)
        .map_err(|_| DataError::QueryRejected)?;
    if statements.len() != 1 {
        return Err(DataError::QueryRejected);
    }
    let Statement::Query(query) = &statements[0] else {
        return Err(DataError::QueryRejected);
    };
    let SetExpr::Select(select) = query.body.as_ref() else {
        return Err(DataError::QueryRejected);
    };
    if query.with.is_some() || has_unhandled_select_syntax(select) {
        return Err(DataError::QueryRejected);
    }
    let mut validator = QueryValidator {
        query_count: 0,
        dataset_count: 0,
        found_dataset: false,
    };
    if query.visit(&mut validator).is_break()
        || validator.query_count != 1
        || !validator.found_dataset
    {
        return Err(DataError::QueryRejected);
    }
    Ok(())
}

fn has_unhandled_select_syntax(select: &Select) -> bool {
    select.into.is_some()
        || select.top.is_some()
        || select.top_before_distinct
        || select.exclude.is_some()
        || matches!(
            select.distinct.as_ref(),
            Some(sqlparser::ast::Distinct::On(_))
        )
        || !select.lateral_views.is_empty()
        || select.prewhere.is_some()
        || !select.cluster_by.is_empty()
        || !select.sort_by.is_empty()
        || !select.distribute_by.is_empty()
        || !select.named_window.is_empty()
        || select.qualify.is_some()
        || select.window_before_qualify
        || select.value_table_mode.is_some()
        || select.connect_by.is_some()
        || select.flavor != SelectFlavor::Standard
        || select.projection.iter().any(has_unhandled_wildcard_options)
        || has_unhandled_group_by_syntax(&select.group_by)
}

fn has_unhandled_wildcard_options(item: &SelectItem) -> bool {
    match item {
        SelectItem::Wildcard(options) | SelectItem::QualifiedWildcard(_, options) => {
            options.opt_ilike.is_some()
                || options.opt_exclude.is_some()
                || options.opt_except.is_some()
                || options.opt_replace.is_some()
                || options.opt_rename.is_some()
        }
        SelectItem::UnnamedExpr(_) | SelectItem::ExprWithAlias { .. } => false,
    }
}

fn has_unhandled_group_by_syntax(group_by: &GroupByExpr) -> bool {
    match group_by {
        GroupByExpr::All(_) => true,
        GroupByExpr::Expressions(expressions, modifiers) => {
            !modifiers.is_empty()
                || expressions.iter().any(|expression| {
                    matches!(
                        expression,
                        Expr::GroupingSets(_) | Expr::Cube(_) | Expr::Rollup(_)
                    )
                })
        }
    }
}

fn has_unhandled_order_by_syntax(order_by: &OrderBy) -> bool {
    order_by.interpolate.is_some()
        || match &order_by.kind {
            OrderByKind::All(_) => true,
            OrderByKind::Expressions(expressions) => expressions
                .iter()
                .any(|expression| expression.with_fill.is_some()),
        }
}

fn has_supported_function_argument_shape(name: &str, arguments: &FunctionArguments) -> bool {
    match arguments {
        FunctionArguments::List(arguments) => {
            let positional_expressions = !arguments.args.is_empty()
                && arguments.args.iter().all(|argument| {
                    matches!(argument, FunctionArg::Unnamed(FunctionArgExpr::Expr(_)))
                });
            positional_expressions
                || (name == "count"
                    && arguments.duplicate_treatment.is_none()
                    && arguments.clauses.is_empty()
                    && matches!(
                        arguments.args.as_slice(),
                        [FunctionArg::Unnamed(FunctionArgExpr::Wildcard)]
                    ))
        }
        FunctionArguments::None | FunctionArguments::Subquery(_) => false,
    }
}

fn has_unsupported_function_argument_clauses(
    arguments: &FunctionArguments,
    scalar_function: bool,
) -> bool {
    match arguments {
        FunctionArguments::List(arguments) => arguments.clauses.iter().any(|clause| match clause {
            FunctionArgumentClause::OrderBy(_) => scalar_function,
            FunctionArgumentClause::IgnoreOrRespectNulls(_)
            | FunctionArgumentClause::Limit(_)
            | FunctionArgumentClause::OnOverflow(_)
            | FunctionArgumentClause::Having(_)
            | FunctionArgumentClause::Separator(_)
            | FunctionArgumentClause::JsonNullClause(_)
            | FunctionArgumentClause::JsonReturningClause(_) => true,
        }),
        FunctionArguments::None | FunctionArguments::Subquery(_) => false,
    }
}

struct QueryValidator {
    query_count: usize,
    dataset_count: usize,
    found_dataset: bool,
}

impl Visitor for QueryValidator {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        self.query_count = self.query_count.saturating_add(1);
        if self.query_count > 1
            || query.with.is_some()
            || !matches!(query.body.as_ref(), SetExpr::Select(_))
            || query
                .order_by
                .as_ref()
                .is_some_and(has_unhandled_order_by_syntax)
            || matches!(
                query.limit_clause.as_ref(),
                Some(LimitClause::LimitOffset { limit_by, .. }) if !limit_by.is_empty()
            )
            || query.fetch.is_some()
            || !query.locks.is_empty()
            || query.for_clause.is_some()
            || query.settings.is_some()
            || query.format_clause.is_some()
            || !query.pipe_operators.is_empty()
        {
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<Self::Break> {
        match factor {
            TableFactor::Table {
                name,
                alias,
                args,
                with_hints,
                version,
                with_ordinality,
                partitions,
                json_path,
                sample,
                index_hints,
                ..
            } if alias.as_ref().is_none_or(|table_alias| {
                table_alias
                    .columns
                    .iter()
                    .all(|column| column.data_type.is_none())
            }) && args.is_none()
                && with_hints.is_empty()
                && version.is_none()
                && !with_ordinality
                && partitions.is_empty()
                && json_path.is_none()
                && sample.is_none()
                && index_hints.is_empty()
                && name.to_string().eq_ignore_ascii_case("dataset") =>
            {
                self.dataset_count = self.dataset_count.saturating_add(1);
                if self.dataset_count > 1 {
                    return ControlFlow::Break(());
                }
                self.found_dataset = true;
                ControlFlow::Continue(())
            }
            _ => ControlFlow::Break(()),
        }
    }

    fn pre_visit_expr(&mut self, expression: &Expr) -> ControlFlow<Self::Break> {
        if matches!(
            expression,
            Expr::Collate { .. }
                | Expr::IsNormalized { .. }
                | Expr::MemberOf(_)
                | Expr::Convert { .. }
                | Expr::Extract { .. }
                | Expr::Floor { .. }
                | Expr::Ceil { .. }
                | Expr::Position { .. }
                | Expr::Substring { .. }
                | Expr::Trim { .. }
                | Expr::Overlay { .. }
                | Expr::Cast {
                    format: Some(_),
                    ..
                }
        ) {
            return ControlFlow::Break(());
        }
        if let Expr::Function(function) = expression {
            let name = function.name.to_string().to_ascii_lowercase();
            let scalar_function = matches!(name.as_str(), "abs" | "coalesce" | "round");
            if !matches!(
                name.as_str(),
                "abs" | "avg" | "coalesce" | "count" | "max" | "min" | "round" | "sum"
            ) || !matches!(&function.parameters, FunctionArguments::None)
                || !has_supported_function_argument_shape(&name, &function.args)
                || function.over.is_some()
                || !function.within_group.is_empty()
                || function.null_treatment.is_some()
                || has_unsupported_function_argument_clauses(&function.args, scalar_function)
                || (scalar_function
                    && (function.filter.is_some()
                        || matches!(
                            &function.args,
                            FunctionArguments::List(arguments)
                                if matches!(
                                    arguments.duplicate_treatment,
                                    Some(sqlparser::ast::DuplicateTreatment::Distinct)
                                )
                        )))
            {
                return ControlFlow::Break(());
            }
        }
        ControlFlow::Continue(())
    }
}

fn load_manifest(data_root: &Path, dataset_id: &str) -> Result<DatasetManifestV1, DataError> {
    let (manifest, dataset_path) = load_manifest_record(data_root, dataset_id)?;
    let metadata = fs::metadata(&dataset_path).map_err(DataError::Storage)?;
    if metadata.len() != manifest.byte_length {
        return Err(DataError::IntegrityMismatch);
    }
    if sha256_file(&dataset_path, MAX_FILE_BYTES)? != manifest.content_digest {
        return Err(DataError::IntegrityMismatch);
    }
    Ok(manifest)
}

fn load_manifest_record(
    data_root: &Path,
    dataset_id: &str,
) -> Result<(DatasetManifestV1, PathBuf), DataError> {
    let (expected_format, expected_path) = dataset_spec(dataset_id)?;
    let root = checked_root(data_root)?;
    let manifest_path = root.join("manifests").join(format!("{dataset_id}.json"));
    reject_path_symlinks(&root, &manifest_path)?;
    if fs::metadata(&manifest_path)
        .map_err(DataError::Storage)?
        .len()
        > 16 * 1024
    {
        return Err(DataError::LimitExceeded);
    }
    let bytes = fs::read(&manifest_path).map_err(DataError::Storage)?;
    let manifest: DatasetManifestV1 =
        serde_json::from_slice(&bytes).map_err(|_| DataError::InvalidCatalog)?;
    let source_contract_matches = if dataset_id == GPUDIFF_DATASET_ID {
        manifest.source_label == GPUDIFF_SOURCE_LABEL
            && manifest.freshness == GPUDIFF_FRESHNESS
            && manifest.claim_ceiling == GPUDIFF_CLAIM_CEILING
            && manifest.schema_digest == sha256(GPUDIFF_SCHEMA.as_bytes())
            && gpudiff_snapshot_path_matches(&manifest)
            && validate_gpudiff_manifest_receipt(&root, &manifest)?
    } else {
        manifest.relative_path == expected_path.ok_or(DataError::InvalidCatalog)?
            && manifest.source_label == "HyperAI deterministic synthetic sample"
            && manifest.freshness == "synthetic_not_market_data"
            && manifest.schema_digest == sha256(SYNTHETIC_SCHEMA.as_bytes())
    };
    if manifest.schema_version != MANIFEST_SCHEMA
        || manifest.dataset_id != dataset_id
        || manifest.format != expected_format
        || !source_contract_matches
    {
        return Err(DataError::InvalidCatalog);
    }
    if manifest.byte_length > MAX_FILE_BYTES {
        return Err(DataError::LimitExceeded);
    }
    let dataset_path = checked_dataset_path(&root, &manifest)?;
    let metadata = fs::metadata(&dataset_path).map_err(DataError::Storage)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(DataError::LimitExceeded);
    }
    if metadata.len() != manifest.byte_length {
        return Err(DataError::IntegrityMismatch);
    }
    if count_source_records(&dataset_path, expected_format)? != manifest.row_count {
        return Err(DataError::InvalidCatalog);
    }
    Ok((manifest, dataset_path))
}

fn validate_gpudiff_manifest_receipt(
    root: &Path,
    manifest: &DatasetManifestV1,
) -> Result<bool, DataError> {
    let receipts_dir = root.join("receipts/gpudiff");
    if !path_is_present(&receipts_dir)? {
        return Ok(false);
    }
    reject_path_symlinks(root, &receipts_dir)?;
    let mut matched: Option<GpudiffImportReceiptV1> = None;
    let mut receipt_count = 0_usize;
    for entry in fs::read_dir(&receipts_dir).map_err(DataError::Storage)? {
        let entry = entry.map_err(DataError::Storage)?;
        if entry.file_type().map_err(DataError::Storage)?.is_symlink() {
            return Err(DataError::InvalidCatalog);
        }
        if entry
            .path()
            .extension()
            .is_none_or(|extension| extension != "json")
        {
            continue;
        }
        receipt_count = receipt_count
            .checked_add(1)
            .ok_or(DataError::LimitExceeded)?;
        if receipt_count > GPUDIFF_MAX_CAPTURES
            || entry.metadata().map_err(DataError::Storage)?.len() > 16 * 1024
        {
            return Err(DataError::InvalidCatalog);
        }
        let bytes = fs::read(entry.path()).map_err(DataError::Storage)?;
        let receipt: GpudiffImportReceiptV1 =
            serde_json::from_slice(&bytes).map_err(|_| DataError::InvalidCatalog)?;
        if receipt.normalized_digest == manifest.content_digest
            && matched.replace(receipt).is_some()
        {
            return Err(DataError::InvalidCatalog);
        }
    }
    let Some(receipt) = matched else {
        return Ok(false);
    };
    if receipt.schema_version != "hyperai.data.gpudiff_import_receipt.v1"
        || receipt.source_url != GPUDIFF_SOURCE_URL_V1
        || receipt.attribution != GPUDIFF_SOURCE_LABEL
        || receipt.row_count != manifest.row_count
        || receipt.observed_at_max != manifest.snapshot_at
        || receipt.captured_unix_seconds == 0
        || receipt.retention_policy
            != "retain up to 16 distinct source bodies; fail closed at capacity; no automatic deletion"
        || bounded_header_value(receipt.etag.as_deref())? != receipt.etag
        || bounded_header_value(receipt.last_modified.as_deref())? != receipt.last_modified
    {
        return Ok(false);
    }
    let raw_path = root
        .join("sources/gpudiff")
        .join(format!("{}.json", receipt.raw_source_digest.to_hex()));
    reject_path_symlinks(root, &raw_path)?;
    if sha256_file(&raw_path, GPUDIFF_MAX_BODY_BYTES as u64)? != receipt.raw_source_digest {
        return Ok(false);
    }
    let raw_bytes = fs::read(&raw_path).map_err(DataError::Storage)?;
    let normalized = normalize_gpudiff_feed(&raw_bytes, receipt.raw_source_digest)?;
    Ok(normalized.row_count == receipt.row_count
        && normalized.observed_at_min == receipt.observed_at_min
        && normalized.observed_at_max == receipt.observed_at_max
        && sha256(&normalized.bytes) == receipt.normalized_digest
        && receipt.normalized_digest == manifest.content_digest)
}

fn count_source_records(data_path: &Path, format: &str) -> Result<u64, DataError> {
    match format {
        "json" => count_ndjson_records(data_path),
        "parquet" => {
            let file = File::open(data_path).map_err(DataError::Storage)?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(file)
                .map_err(|_| DataError::InvalidCatalog)?;
            u64::try_from(reader.metadata().file_metadata().num_rows())
                .map_err(|_| DataError::InvalidCatalog)
        }
        _ => Err(DataError::InvalidCatalog),
    }
}

fn count_ndjson_records(data_path: &Path) -> Result<u64, DataError> {
    let mut file = File::open(data_path).map_err(DataError::Storage)?;
    let mut buffer = vec![0_u8; 32 * 1024];
    let mut row_count = 0_u64;
    let mut current_line_nonempty = false;
    loop {
        let bytes_read = file.read(&mut buffer).map_err(DataError::Storage)?;
        if bytes_read == 0 {
            break;
        }
        for byte in &buffer[..bytes_read] {
            match *byte {
                b'\n' => {
                    if current_line_nonempty {
                        row_count = row_count.checked_add(1).ok_or(DataError::LimitExceeded)?;
                        current_line_nonempty = false;
                    }
                }
                b'\r' => {}
                _ => current_line_nonempty = true,
            }
        }
    }
    if current_line_nonempty {
        row_count = row_count.checked_add(1).ok_or(DataError::LimitExceeded)?;
    }
    Ok(row_count)
}

fn checked_dataset_path(
    data_root: &Path,
    manifest: &DatasetManifestV1,
) -> Result<PathBuf, DataError> {
    let root = checked_root(data_root)?;
    let relative = Path::new(&manifest.relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !(matches!(manifest.relative_path.as_str(), SAMPLE_FILE | PARQUET_FILE)
            || (manifest.dataset_id == GPUDIFF_DATASET_ID
                && gpudiff_snapshot_path_matches(manifest)))
    {
        return Err(DataError::InvalidCatalog);
    }
    let path = root.join(relative);
    reject_path_symlinks(&root, &path)?;
    let canonical = fs::canonicalize(&path).map_err(DataError::Storage)?;
    if !canonical.starts_with(&root) {
        return Err(DataError::InvalidCatalog);
    }
    Ok(canonical)
}

fn gpudiff_snapshot_path_matches(manifest: &DatasetManifestV1) -> bool {
    manifest.relative_path
        == format!(
            "snapshots/gpudiff-{}.ndjson",
            manifest.content_digest.to_hex()
        )
}

fn checked_root(data_root: &Path) -> Result<PathBuf, DataError> {
    reject_self_symlink(data_root)?;
    let root = fs::canonicalize(data_root).map_err(DataError::Storage)?;
    if !root.is_dir() {
        return Err(DataError::InvalidCatalog);
    }
    Ok(root)
}

fn ensure_data_root(data_root: &Path) -> Result<(), DataError> {
    if data_root.exists() {
        reject_self_symlink(data_root)?;
        if !data_root.is_dir() {
            return Err(DataError::InvalidCatalog);
        }
    } else {
        let parent = data_root.parent().ok_or(DataError::InvalidCatalog)?;
        fs::create_dir(data_root).map_err(DataError::Storage)?;
        reject_self_symlink(parent)?;
        reject_self_symlink(data_root)?;
    }
    Ok(())
}

fn ensure_child_directory(root: &Path, path: &Path) -> Result<(), DataError> {
    let canonical_root = checked_root(root)?;
    let relative = path
        .strip_prefix(&canonical_root)
        .map_err(|_| DataError::InvalidCatalog)?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(DataError::InvalidCatalog);
    }
    reject_path_symlinks(&canonical_root, path)?;
    if !path.exists() {
        fs::create_dir(path).map_err(DataError::Storage)?;
    }
    if !path.is_dir() {
        return Err(DataError::InvalidCatalog);
    }
    Ok(())
}

fn reject_path_symlinks(root: &Path, path: &Path) -> Result<(), DataError> {
    let root = fs::canonicalize(root).map_err(DataError::Storage)?;
    let relative = path
        .strip_prefix(&root)
        .map_err(|_| DataError::InvalidCatalog)?;
    let mut current = root;
    for component in relative.components() {
        if let Component::Normal(name) = component {
            current.push(name);
        } else {
            return Err(DataError::InvalidCatalog);
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(DataError::InvalidCatalog);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(DataError::Storage(error)),
        }
    }
    Ok(())
}

fn reject_self_symlink(path: &Path) -> Result<(), DataError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(DataError::InvalidCatalog),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DataError::Storage(error)),
    }
}

fn atomic_create_or_compare(path: &Path, bytes: &[u8]) -> Result<(), DataError> {
    if let Some(existing) = read_existing(path)? {
        return if existing == bytes {
            Ok(())
        } else {
            Err(DataError::Conflict)
        };
    }
    let parent = path.parent().ok_or(DataError::InvalidCatalog)?;
    let file_name = path
        .file_name()
        .ok_or(DataError::InvalidCatalog)?
        .to_string_lossy();
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(DataError::Storage)?;
    file.write_all(bytes).map_err(DataError::Storage)?;
    file.sync_all().map_err(DataError::Storage)?;
    match fs::hard_link(&temporary, path) {
        Ok(()) => {
            fs::remove_file(&temporary).map_err(DataError::Storage)?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(&temporary);
            match read_existing(path)? {
                Some(existing) if existing == bytes => Ok(()),
                _ => Err(DataError::Conflict),
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(DataError::Storage(error))
        }
    }
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), DataError> {
    let parent = path.parent().ok_or(DataError::InvalidCatalog)?;
    reject_self_symlink(path)?;
    let file_name = path
        .file_name()
        .ok_or(DataError::InvalidCatalog)?
        .to_string_lossy();
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    reject_self_symlink(&temporary)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(DataError::Storage)?;
    file.write_all(bytes).map_err(DataError::Storage)?;
    file.sync_all().map_err(DataError::Storage)?;
    fs::rename(&temporary, path).map_err(DataError::Storage)?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(DataError::Storage)?;
    Ok(())
}

fn read_existing(path: &Path) -> Result<Option<Vec<u8>>, DataError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(DataError::Storage(error)),
    }
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn sha256_file(path: &Path, max_bytes: u64) -> Result<Sha256Digest, DataError> {
    let mut file = File::open(path).map_err(DataError::Storage)?;
    let length = file.metadata().map_err(DataError::Storage)?.len();
    if length > max_bytes {
        return Err(DataError::LimitExceeded);
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8_192];
    let mut total = 0_u64;
    loop {
        let read = file.read(&mut buffer).map_err(DataError::Storage)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > max_bytes {
            return Err(DataError::LimitExceeded);
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

fn serialized_size(
    format: QueryFormat,
    columns: &[String],
    rows: &[Vec<Option<String>>],
) -> Result<usize, DataError> {
    let bytes = match format {
        QueryFormat::Json => serde_json::to_vec(&json!({ "columns": columns, "rows": rows }))
            .map_err(|_| DataError::QueryRejected)?,
        QueryFormat::Csv => csv_encode(columns, rows).into_bytes(),
    };
    Ok(bytes.len())
}

fn serialized_response_size(result: &QueryResultV1) -> Result<usize, DataError> {
    let mut counter = SerializedResponseCounter {
        bytes: 0,
        exceeded: false,
    };
    let serialized = serde_json::to_writer(&mut counter, result);
    if counter.exceeded {
        return Err(DataError::LimitExceeded);
    }
    serialized.map_err(|_| DataError::QueryRejected)?;
    Ok(counter.bytes)
}

struct SerializedResponseCounter {
    bytes: usize,
    exceeded: bool,
}

impl Write for SerializedResponseCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(next) = self.bytes.checked_add(bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::from(io::ErrorKind::WriteZero));
        };
        if next > MAX_RESULT_BYTES {
            self.exceeded = true;
            return Err(io::Error::from(io::ErrorKind::WriteZero));
        }
        self.bytes = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn csv_encode(columns: &[String], rows: &[Vec<Option<String>>]) -> String {
    let mut output = String::new();
    output.push_str(
        &columns
            .iter()
            .map(|column| csv_cell(Some(column)))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push('\n');
    for row in rows {
        output.push_str(
            &row.iter()
                .map(|cell| csv_cell(cell.as_deref()))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

fn csv_cell(value: Option<&str>) -> String {
    let value = value.unwrap_or_default();
    if value
        .bytes()
        .any(|byte| matches!(byte, b',' | b'"' | b'\n' | b'\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn validate_name(name: &str) -> Result<(), DataError> {
    if !valid_identifier(name) || name.len() > 64 {
        return Err(DataError::QueryRejected);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

fn validate_idempotency_key(value: &str) -> Result<(), DataError> {
    if (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        Ok(())
    } else {
        Err(DataError::QueryRejected)
    }
}

#[cfg(test)]
mod bounded_contract_tests {
    use super::*;
    use datafusion::execution::{
        memory_pool::{MemoryConsumer, MemoryLimit},
        runtime_env::RuntimeEnvBuilder,
    };
    use proptest::prelude::*;
    use sqlparser::{
        ast::{DuplicateTreatment, FunctionArgumentClause, GroupByWithModifier},
        dialect::{BigQueryDialect, Dialect, RedshiftSqlDialect},
    };

    const MEMORY_STRESS_FIXTURE_SHA256: &str =
        "541516e964efa456f7b836c3495e61303da5ea9fe2b531737455aca1e4cde81b";

    fn parse_select_with_dialect(
        dialect: &dyn Dialect,
        sql: &str,
    ) -> Result<Select, Box<dyn std::error::Error>> {
        let statements = Parser::parse_sql(dialect, sql)?;
        let Some(Statement::Query(query)) = statements.first() else {
            return Err(io::Error::other("fixed SQL should parse as one query").into());
        };
        let SetExpr::Select(select) = query.body.as_ref() else {
            return Err(io::Error::other("fixed SQL should parse with a SELECT body").into());
        };
        Ok(select.as_ref().clone())
    }

    #[test]
    fn dataset_status_error_mapping_is_bounded() {
        assert_eq!(
            dataset_status_for_error(DataError::Storage(
                io::Error::from(io::ErrorKind::NotFound,)
            )),
            DatasetReadStatusV1::Missing
        );
        assert_eq!(
            dataset_status_for_error(DataError::Storage(io::Error::from(
                io::ErrorKind::PermissionDenied,
            ))),
            DatasetReadStatusV1::Denied
        );
        assert_eq!(
            dataset_status_for_error(DataError::Storage(io::Error::other("opaque failure"))),
            DatasetReadStatusV1::Unavailable
        );
        assert_eq!(
            dataset_status_for_error(DataError::IntegrityMismatch),
            DatasetReadStatusV1::Conflicting
        );
        assert_eq!(
            dataset_status_for_error(DataError::LimitExceeded),
            DatasetReadStatusV1::Unavailable
        );
    }

    #[test]
    fn select_policy_explicitly_rejects_optional_dialect_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let redshift = RedshiftSqlDialect {};

        let select_exclude = parse_select_with_dialect(
            &redshift,
            "SELECT *, gpu_model EXCLUDE region FROM dataset",
        )?;
        assert!(select_exclude.exclude.is_some());
        assert!(has_unhandled_select_syntax(&select_exclude));

        let mut top_before_distinct =
            parse_select_with_dialect(&redshift, "SELECT TOP 1 DISTINCT gpu_model FROM dataset")?;
        assert!(top_before_distinct.top_before_distinct);
        top_before_distinct.top = None;
        top_before_distinct.distinct = None;
        assert!(has_unhandled_select_syntax(&top_before_distinct));

        let bigquery = BigQueryDialect {};
        let select_value_mode =
            parse_select_with_dialect(&bigquery, "SELECT AS VALUE gpu_model FROM dataset")?;
        assert!(select_value_mode.value_table_mode.is_some());
        assert!(has_unhandled_select_syntax(&select_value_mode));

        let mut window_before_qualify = parse_select_with_dialect(
            &GenericDialect {},
            "SELECT gpu_model FROM dataset WINDOW w AS (PARTITION BY gpu_model) QUALIFY gpu_model = 'A100'",
        )?;
        assert!(window_before_qualify.window_before_qualify);
        window_before_qualify.named_window.clear();
        window_before_qualify.qualify = None;
        assert!(has_unhandled_select_syntax(&window_before_qualify));

        let supported =
            parse_select_with_dialect(&GenericDialect {}, "SELECT gpu_model FROM dataset")?;
        assert!(!has_unhandled_select_syntax(&supported));
        assert!(
            validate_query(&QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: "SELECT gpu_model FROM dataset".to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            })
            .is_ok()
        );
        Ok(())
    }

    #[test]
    fn query_validator_rejects_table_sampling() -> Result<(), Box<dyn std::error::Error>> {
        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: "SELECT gpu_model FROM dataset TABLESAMPLE BERNOULLI (50)".to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        let statements = Parser::parse_sql(&GenericDialect {}, &request.sql)?;
        let statement = statements
            .first()
            .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
        let Statement::Query(query) = statement else {
            return Err(io::Error::other("expected one query statement").into());
        };
        let SetExpr::Select(select) = query.body.as_ref() else {
            return Err(io::Error::other("expected a SELECT body").into());
        };
        let relation = &select
            .from
            .first()
            .ok_or_else(|| io::Error::other("expected a FROM relation"))?
            .relation;
        assert!(matches!(
            relation,
            TableFactor::Table {
                sample: Some(_),
                ..
            }
        ));

        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));
        Ok(())
    }

    #[test]
    fn query_validator_rejects_typed_table_alias_columns() -> Result<(), Box<dyn std::error::Error>>
    {
        let sql = "SELECT d.gpu_model FROM dataset AS d (gpu_model INT)";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let Some(Statement::Query(parsed_query)) = statements.first() else {
            return Err(io::Error::other("expected one parsed query statement").into());
        };
        let SetExpr::Select(select) = parsed_query.body.as_ref() else {
            return Err(io::Error::other("expected a SELECT body").into());
        };
        let relation = &select
            .from
            .first()
            .ok_or_else(|| io::Error::other("expected a FROM relation"))?
            .relation;
        let TableFactor::Table {
            alias: Some(alias), ..
        } = relation
        else {
            return Err(io::Error::other("expected an aliased table factor").into());
        };
        assert_eq!(alias.columns.len(), 1);
        assert!(alias.columns[0].data_type.is_some());

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));

        for sql in [
            "SELECT d.gpu_model FROM dataset AS d",
            "SELECT d.model FROM dataset AS d (model)",
        ] {
            let ordinary_alias = QueryRequestV1 {
                sql: sql.to_owned(),
                ..request.clone()
            };
            assert!(validate_query(&ordinary_alias).is_ok(), "{sql}");
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_parametric_function_calls() -> Result<(), Box<dyn std::error::Error>>
    {
        let sql = "SELECT ABS(999)(gpu_model) FROM dataset";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let Some(Statement::Query(query)) = statements.first() else {
            return Err(io::Error::other("expected one parsed query statement").into());
        };
        let SetExpr::Select(select) = query.body.as_ref() else {
            return Err(io::Error::other("expected a SELECT body").into());
        };
        let Some(SelectItem::UnnamedExpr(Expr::Function(function))) = select.projection.first()
        else {
            return Err(io::Error::other("expected a function projection").into());
        };
        let FunctionArguments::List(parameters) = &function.parameters else {
            return Err(io::Error::other("expected a function parameter list").into());
        };
        assert_eq!(parameters.args.len(), 1);
        assert!(matches!(
            &function.args,
            FunctionArguments::List(args) if args.args.len() == 1
        ));

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));

        let ordinary = QueryRequestV1 {
            sql: "SELECT ABS(gpu_model) FROM dataset".to_owned(),
            ..request
        };
        assert!(validate_query(&ordinary).is_ok());
        Ok(())
    }

    #[test]
    fn odbc_function_syntax_keeps_the_existing_name_allowlist()
    -> Result<(), Box<dyn std::error::Error>> {
        for (sql, expected_name, allowed) in [
            ("SELECT { fn ABS(-2) } FROM dataset", "abs", true),
            ("SELECT { fn RANDOM() } FROM dataset", "random", false),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            assert_eq!(
                statements.len(),
                1,
                "expected one parsed statement for {sql}"
            );
            let Some(Statement::Query(query)) = statements.first() else {
                return Err(io::Error::other("expected one parsed query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let Some(SelectItem::UnnamedExpr(Expr::Function(function))) = select.projection.first()
            else {
                return Err(io::Error::other("expected an ODBC function projection").into());
            };

            assert!(function.uses_odbc_syntax, "expected ODBC syntax for {sql}");
            assert_eq!(
                function.name.to_string().to_ascii_lowercase(),
                expected_name
            );

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert_eq!(validate_query(&request).is_ok(), allowed, "{sql}");
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_unsupported_function_argument_shapes()
    -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            ("SELECT ABS() FROM dataset", "empty"),
            ("SELECT COUNT() FROM dataset", "empty"),
            ("SELECT ABS(*) FROM dataset", "wildcard"),
            ("SELECT SUM(*) FROM dataset", "wildcard"),
            ("SELECT ABS(dataset.*) FROM dataset", "qualified_wildcard"),
            ("SELECT COUNT(dataset.*) FROM dataset", "qualified_wildcard"),
            ("SELECT ABS(value := gpu_model) FROM dataset", "named"),
            ("SELECT COUNT(value := gpu_model) FROM dataset", "named"),
            ("SELECT COUNT(DISTINCT *) FROM dataset", "modified_wildcard"),
            ("SELECT COUNT(ALL *) FROM dataset", "modified_wildcard"),
        ];

        for (sql, expected_shape) in cases {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let Some(Statement::Query(query)) = statements.first() else {
                return Err(io::Error::other("expected one parsed query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let Some(SelectItem::UnnamedExpr(Expr::Function(function))) = select.projection.first()
            else {
                return Err(io::Error::other("expected a function projection").into());
            };
            let FunctionArguments::List(arguments) = &function.args else {
                return Err(io::Error::other("expected a function argument list").into());
            };
            let parsed_shape_matches = match expected_shape {
                "empty" => arguments.args.is_empty(),
                "wildcard" => matches!(
                    arguments.args.as_slice(),
                    [FunctionArg::Unnamed(FunctionArgExpr::Wildcard)]
                ),
                "qualified_wildcard" => matches!(
                    arguments.args.as_slice(),
                    [FunctionArg::Unnamed(FunctionArgExpr::QualifiedWildcard(_))]
                ),
                "named" => matches!(arguments.args.as_slice(), [FunctionArg::Named { .. }]),
                "modified_wildcard" => {
                    arguments.duplicate_treatment.is_some()
                        && matches!(
                            arguments.args.as_slice(),
                            [FunctionArg::Unnamed(FunctionArgExpr::Wildcard)]
                        )
                }
                _ => false,
            };
            assert!(parsed_shape_matches, "unexpected AST shape for {sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "expected pre-planning rejection for {sql}"
            );
        }

        for sql in [
            "SELECT ABS(gpu_model) FROM dataset",
            "SELECT COUNT(*) FROM dataset",
            "SELECT COUNT(DISTINCT gpu_model) FROM dataset",
        ] {
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                validate_query(&request).is_ok(),
                "rejected supported query {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_unsupported_aggregate_argument_clauses()
    -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            ("SELECT SUM(available_count LIMIT 1) FROM dataset", "limit"),
            (
                "SELECT SUM(available_count IGNORE NULLS) FROM dataset",
                "null_treatment_argument",
            ),
            (
                "SELECT SUM(available_count) IGNORE NULLS FROM dataset",
                "null_treatment_function",
            ),
            (
                "SELECT SUM(available_count HAVING MAX available_count) FROM dataset",
                "having",
            ),
            (
                "SELECT SUM(available_count SEPARATOR ',') FROM dataset",
                "separator",
            ),
            (
                "SELECT SUM(available_count ON OVERFLOW ERROR) FROM dataset",
                "on_overflow",
            ),
            (
                "SELECT SUM(available_count ABSENT ON NULL) FROM dataset",
                "json_null",
            ),
            (
                "SELECT SUM(available_count RETURNING BIGINT) FROM dataset",
                "json_returning",
            ),
        ];

        for (sql, expected_clause) in cases {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let Some(Statement::Query(query)) = statements.first() else {
                return Err(io::Error::other("expected one parsed query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let Some(SelectItem::UnnamedExpr(Expr::Function(function))) = select.projection.first()
            else {
                return Err(io::Error::other("expected an aggregate projection").into());
            };
            let clause_is_populated = match expected_clause {
                "null_treatment_function" => function.null_treatment.is_some(),
                _ => matches!(
                    &function.args,
                    FunctionArguments::List(arguments)
                        if arguments.clauses.iter().any(|clause| match expected_clause {
                            "limit" => matches!(clause, FunctionArgumentClause::Limit(_)),
                            "null_treatment_argument" => matches!(
                                clause,
                                FunctionArgumentClause::IgnoreOrRespectNulls(_)
                            ),
                            "having" => matches!(clause, FunctionArgumentClause::Having(_)),
                            "separator" => matches!(clause, FunctionArgumentClause::Separator(_)),
                            "on_overflow" => matches!(clause, FunctionArgumentClause::OnOverflow(_)),
                            "json_null" => matches!(clause, FunctionArgumentClause::JsonNullClause(_)),
                            "json_returning" => matches!(
                                clause,
                                FunctionArgumentClause::JsonReturningClause(_)
                            ),
                            _ => false,
                        })
                ),
            };
            assert!(
                clause_is_populated,
                "parser did not populate {expected_clause}: {sql}"
            );

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted unsupported aggregate clause: {sql}"
            );
        }

        for sql in [
            "SELECT SUM(available_count) FILTER (WHERE available_count > 0) FROM dataset",
            "SELECT SUM(available_count ORDER BY gpu_model) FROM dataset",
            "SELECT COUNT(DISTINCT gpu_model) FROM dataset",
        ] {
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                validate_query(&request).is_ok(),
                "rejected supported aggregate query: {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_scalar_function_modifiers() -> Result<(), Box<dyn std::error::Error>>
    {
        let cases = [
            (
                "SELECT ABS(asking_usd_micros_per_hour) FILTER (WHERE asking_usd_micros_per_hour < 0) FROM dataset",
                "filter",
            ),
            (
                "SELECT ABS(asking_usd_micros_per_hour) IGNORE NULLS FROM dataset",
                "null_treatment",
            ),
            (
                "SELECT ABS(asking_usd_micros_per_hour ORDER BY asking_usd_micros_per_hour) FROM dataset",
                "argument_order_by",
            ),
            (
                "SELECT ABS(DISTINCT asking_usd_micros_per_hour) FROM dataset",
                "distinct",
            ),
        ];

        for (sql, expected_field) in cases {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let Some(Statement::Query(query)) = statements.first() else {
                return Err(io::Error::other("expected one parsed query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let Some(SelectItem::UnnamedExpr(Expr::Function(function))) = select.projection.first()
            else {
                return Err(io::Error::other("expected a function projection").into());
            };
            let field_is_populated = match expected_field {
                "filter" => function.filter.is_some(),
                "null_treatment" => function.null_treatment.is_some(),
                "argument_order_by" => matches!(
                    &function.args,
                    FunctionArguments::List(arguments)
                        if arguments.clauses.iter().any(|clause| matches!(
                            clause,
                            FunctionArgumentClause::OrderBy(_)
                        ))
                ),
                "distinct" => matches!(
                    &function.args,
                    FunctionArguments::List(arguments)
                        if matches!(arguments.duplicate_treatment, Some(DuplicateTreatment::Distinct))
                ),
                _ => false,
            };
            assert!(
                field_is_populated,
                "parser did not populate {expected_field}: {sql}"
            );

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted scalar modifier query: {sql}"
            );
        }

        for sql in [
            "SELECT ABS(asking_usd_micros_per_hour) FROM dataset",
            "SELECT SUM(asking_usd_micros_per_hour) FILTER (WHERE available_count > 0) FROM dataset",
            "SELECT COUNT(DISTINCT gpu_model) FROM dataset",
        ] {
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                validate_query(&request).is_ok(),
                "rejected supported query: {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_unallowlisted_special_expression_forms()
    -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            (
                "SELECT CONVERT(gpu_model USING UTF8) FROM dataset",
                "convert",
            ),
            (
                "SELECT CAST(gpu_model AS VARCHAR FORMAT 'x') FROM dataset",
                "cast_format",
            ),
            ("SELECT gpu_model COLLATE utf8_bin FROM dataset", "collate"),
            (
                "SELECT gpu_model IS NFC NORMALIZED FROM dataset",
                "is_normalized",
            ),
            (
                "SELECT EXTRACT(DAY FROM observed_at) FROM dataset",
                "extract",
            ),
            ("SELECT POSITION('A' IN gpu_model) FROM dataset", "position"),
            (
                "SELECT SUBSTRING(gpu_model FROM 1 FOR 2) FROM dataset",
                "substring",
            ),
            ("SELECT TRIM(gpu_model) FROM dataset", "trim"),
            (
                "SELECT OVERLAY(gpu_model PLACING 'X' FROM 1) FROM dataset",
                "overlay",
            ),
            (
                "SELECT FLOOR(asking_usd_micros_per_hour TO DAY) FROM dataset",
                "floor",
            ),
            (
                "SELECT CEIL(asking_usd_micros_per_hour, 2) FROM dataset",
                "ceil",
            ),
        ];

        for (sql, expected_variant) in cases {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let Some(Statement::Query(query)) = statements.first() else {
                return Err(io::Error::other("expected one parsed query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let Some(SelectItem::UnnamedExpr(expression)) = select.projection.first() else {
                return Err(io::Error::other("expected an expression projection").into());
            };
            let variant_matches = match expected_variant {
                "convert" => matches!(expression, Expr::Convert { .. }),
                "cast_format" => matches!(
                    expression,
                    Expr::Cast {
                        format: Some(_),
                        ..
                    }
                ),
                "collate" => matches!(expression, Expr::Collate { .. }),
                "is_normalized" => matches!(expression, Expr::IsNormalized { .. }),
                "extract" => matches!(expression, Expr::Extract { .. }),
                "position" => matches!(expression, Expr::Position { .. }),
                "substring" => matches!(expression, Expr::Substring { .. }),
                "trim" => matches!(expression, Expr::Trim { .. }),
                "overlay" => matches!(expression, Expr::Overlay { .. }),
                "floor" => matches!(expression, Expr::Floor { .. }),
                "ceil" => matches!(expression, Expr::Ceil { .. }),
                _ => false,
            };
            assert!(
                variant_matches,
                "parser did not populate {expected_variant}: {sql}"
            );

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted unallowlisted special expression: {sql}"
            );
        }

        for sql in [
            "SELECT CAST(asking_usd_micros_per_hour AS VARCHAR) FROM dataset",
            "SELECT TRY_CAST(asking_usd_micros_per_hour AS VARCHAR) FROM dataset",
        ] {
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                validate_query(&request).is_ok(),
                "rejected ordinary cast: {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_member_of_ast_before_planning()
    -> Result<(), Box<dyn std::error::Error>> {
        let sql = "SELECT gpu_model MEMBER OF (ARRAY['A100']) FROM dataset";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let Some(Statement::Query(query)) = statements.first() else {
            return Err(io::Error::other("expected one parsed query statement").into());
        };
        let SetExpr::Select(select) = query.body.as_ref() else {
            return Err(io::Error::other("expected a SELECT body").into());
        };
        assert!(matches!(
            select.projection.first(),
            Some(SelectItem::UnnamedExpr(Expr::MemberOf(_)))
        ));

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn planner_preflight_rejects_parsed_in_unnest_without_dataset_access()
    -> Result<(), Box<dyn std::error::Error>> {
        let sql = "SELECT gpu_model IN UNNEST(ARRAY['A100']) FROM dataset";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let Some(Statement::Query(parsed_query)) = statements.first() else {
            return Err(io::Error::other("expected one parsed query statement").into());
        };
        let SetExpr::Select(select) = parsed_query.body.as_ref() else {
            return Err(io::Error::other("expected a SELECT body").into());
        };
        assert!(matches!(
            select.projection.first(),
            Some(SelectItem::UnnamedExpr(Expr::InUnnest { .. }))
        ));

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(validate_query(&request).is_ok());

        let temporary = tempfile::tempdir()?;
        let absent_root = temporary.path().join("data");
        assert!(matches!(
            query(&absent_root, &request).await,
            Err(DataError::QueryRejected)
        ));
        assert!(!absent_root.exists());
        Ok(())
    }

    #[test]
    fn query_validator_rejects_distinct_on_and_preserves_distinct()
    -> Result<(), Box<dyn std::error::Error>> {
        let request = |sql: &str| QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };

        for (sql, expected_empty) in [
            ("SELECT DISTINCT ON () gpu_model FROM dataset", true),
            (
                "SELECT DISTINCT ON (gpu_model) gpu_model FROM dataset ORDER BY gpu_model",
                false,
            ),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let Some(Statement::Query(query)) = statements.first() else {
                return Err(io::Error::other("expected one parsed query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            assert!(matches!(
                select.distinct.as_ref(),
                Some(sqlparser::ast::Distinct::On(on_expressions))
                    if on_expressions.is_empty() == expected_empty
            ));
            assert!(matches!(
                validate_query(&request(sql)),
                Err(DataError::QueryRejected)
            ));
        }

        assert!(validate_query(&request("SELECT DISTINCT gpu_model FROM dataset")).is_ok());
        Ok(())
    }

    #[test]
    fn query_validator_rejects_ignored_query_clauses() -> Result<(), Box<dyn std::error::Error>> {
        for (sql, has_for_clause, has_format_clause, has_fetch) in [
            (
                "SELECT gpu_model FROM dataset FOR JSON AUTO",
                true,
                false,
                false,
            ),
            (
                "SELECT gpu_model FROM dataset FOR XML RAW",
                true,
                false,
                false,
            ),
            (
                "SELECT gpu_model FROM dataset FORMAT JSON",
                false,
                true,
                false,
            ),
            (
                "SELECT gpu_model FROM dataset FETCH FIRST 1 ROWS ONLY",
                false,
                false,
                true,
            ),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let statement = statements
                .first()
                .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
            let Statement::Query(query) = statement else {
                return Err(io::Error::other("expected one query statement").into());
            };
            assert_eq!(query.for_clause.is_some(), has_for_clause, "{sql}");
            assert_eq!(query.format_clause.is_some(), has_format_clause, "{sql}");
            assert_eq!(query.fetch.is_some(), has_fetch, "{sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted query: {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_unhandled_select_clauses() -> Result<(), Box<dyn std::error::Error>>
    {
        for (sql, field) in [
            (
                "SELECT gpu_model FROM dataset PREWHERE gpu_model = 'GPU A'",
                "prewhere",
            ),
            ("SELECT TOP (1) gpu_model FROM dataset", "top"),
            (
                "SELECT gpu_model FROM dataset LATERAL VIEW explode(array(1)) lv AS x",
                "lateral_views",
            ),
            (
                "SELECT gpu_model FROM dataset CLUSTER BY gpu_model",
                "cluster_by",
            ),
            ("SELECT gpu_model FROM dataset SORT BY gpu_model", "sort_by"),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let statement = statements
                .first()
                .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
            let Statement::Query(query) = statement else {
                return Err(io::Error::other("expected one query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let parsed = match field {
                "prewhere" => select.prewhere.is_some(),
                "top" => select.top.is_some(),
                "lateral_views" => !select.lateral_views.is_empty(),
                "cluster_by" => !select.cluster_by.is_empty(),
                "sort_by" => !select.sort_by.is_empty(),
                _ => false,
            };
            assert!(parsed, "SQLparser did not populate {field} for {sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted query: {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_select_into_from_the_parsed_ast()
    -> Result<(), Box<dyn std::error::Error>> {
        let sql = "SELECT gpu_model INTO TABLE exported FROM dataset";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let Some(Statement::Query(query)) = statements.first() else {
            return Err(io::Error::other("expected one parsed query statement").into());
        };
        let SetExpr::Select(select) = query.body.as_ref() else {
            return Err(io::Error::other("expected a SELECT body").into());
        };
        assert!(select.into.is_some());

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));
        Ok(())
    }

    #[test]
    fn query_validator_rejects_unhandled_generic_query_ast_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        for (sql, field) in [
            (
                "SELECT gpu_model FROM dataset DISTRIBUTE BY gpu_model",
                "distribute_by",
            ),
            (
                "SELECT gpu_model FROM dataset WINDOW w AS (ORDER BY gpu_model)",
                "named_window",
            ),
            ("SELECT gpu_model FROM dataset QUALIFY TRUE", "qualify"),
            (
                "SELECT gpu_model FROM dataset START WITH TRUE CONNECT BY TRUE",
                "connect_by",
            ),
            ("FROM dataset SELECT gpu_model", "from_first"),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let statement = statements
                .first()
                .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
            let Statement::Query(query) = statement else {
                return Err(io::Error::other("expected one query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let parsed = match field {
                "distribute_by" => !select.distribute_by.is_empty(),
                "named_window" => !select.named_window.is_empty(),
                "qualify" => select.qualify.is_some(),
                "connect_by" => select.connect_by.is_some(),
                "from_first" => select.flavor == SelectFlavor::FromFirst,
                _ => false,
            };
            assert!(parsed, "SQLparser did not populate {field} for {sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted query: {sql}"
            );
        }
        Ok(())
    }

    #[test]
    fn query_validator_rejects_per_key_limit_clause() -> Result<(), Box<dyn std::error::Error>> {
        let sql = "SELECT gpu_model FROM dataset LIMIT 1 BY gpu_model";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let statement = statements
            .first()
            .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
        let Statement::Query(query) = statement else {
            return Err(io::Error::other("expected one query statement").into());
        };
        assert!(matches!(
            query.limit_clause.as_ref(),
            Some(LimitClause::LimitOffset { limit_by, .. }) if !limit_by.is_empty()
        ));

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));

        let bounded_limit = QueryRequestV1 {
            sql: "SELECT gpu_model FROM dataset LIMIT 1".to_owned(),
            ..request
        };
        assert!(validate_query(&bounded_limit).is_ok());
        Ok(())
    }

    #[test]
    fn query_validator_rejects_unhandled_wildcard_options() -> Result<(), Box<dyn std::error::Error>>
    {
        for (sql, field) in [
            ("SELECT * ILIKE 'gpu%' FROM dataset", "ilike"),
            ("SELECT * EXCLUDE gpu_model FROM dataset", "exclude"),
            ("SELECT * EXCEPT (gpu_model) FROM dataset", "except"),
            (
                "SELECT * REPLACE ('replacement' AS gpu_model) FROM dataset",
                "replace",
            ),
            ("SELECT * RENAME gpu_model AS model FROM dataset", "rename"),
            (
                "SELECT dataset.* EXCEPT (gpu_model) FROM dataset",
                "except_qualified",
            ),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let statement = statements
                .first()
                .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
            let Statement::Query(query) = statement else {
                return Err(io::Error::other("expected one query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let projection = select
                .projection
                .first()
                .ok_or_else(|| io::Error::other("expected a wildcard projection"))?;
            let (SelectItem::Wildcard(options) | SelectItem::QualifiedWildcard(_, options)) =
                projection
            else {
                return Err(io::Error::other("expected a wildcard projection").into());
            };
            let parsed = match field {
                "ilike" => options.opt_ilike.is_some(),
                "exclude" => options.opt_exclude.is_some(),
                "except" | "except_qualified" => options.opt_except.is_some(),
                "replace" => options.opt_replace.is_some(),
                "rename" => options.opt_rename.is_some(),
                _ => false,
            };
            assert!(parsed, "SQLparser did not populate {field} for {sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted query: {sql}"
            );
        }

        let ordinary_wildcard = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: "SELECT * FROM dataset".to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(validate_query(&ordinary_wildcard).is_ok());
        Ok(())
    }

    #[test]
    fn query_validator_rejects_advanced_group_by_syntax() -> Result<(), Box<dyn std::error::Error>>
    {
        for (sql, field) in [
            ("SELECT gpu_model FROM dataset GROUP BY ALL", "all"),
            (
                "SELECT gpu_model FROM dataset GROUP BY ROLLUP (gpu_model)",
                "rollup_expr",
            ),
            (
                "SELECT gpu_model FROM dataset GROUP BY CUBE (gpu_model)",
                "cube_expr",
            ),
            (
                "SELECT gpu_model FROM dataset GROUP BY GROUPING SETS ((gpu_model), (gpu_model))",
                "grouping_sets_expr",
            ),
            (
                "SELECT gpu_model FROM dataset GROUP BY gpu_model WITH ROLLUP",
                "with_rollup",
            ),
            (
                "SELECT gpu_model FROM dataset GROUP BY gpu_model WITH CUBE",
                "with_cube",
            ),
            (
                "SELECT gpu_model FROM dataset GROUP BY gpu_model WITH TOTALS",
                "with_totals",
            ),
            (
                "SELECT gpu_model FROM dataset GROUP BY gpu_model GROUPING SETS ((gpu_model), (gpu_model))",
                "grouping_sets_modifier",
            ),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let statement = statements
                .first()
                .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
            let Statement::Query(query) = statement else {
                return Err(io::Error::other("expected one query statement").into());
            };
            let SetExpr::Select(select) = query.body.as_ref() else {
                return Err(io::Error::other("expected a SELECT body").into());
            };
            let parsed = match (&select.group_by, field) {
                (GroupByExpr::All(_), "all") => true,
                (GroupByExpr::Expressions(expressions, _), "rollup_expr") => expressions
                    .iter()
                    .any(|expression| matches!(expression, Expr::Rollup(_))),
                (GroupByExpr::Expressions(expressions, _), "cube_expr") => expressions
                    .iter()
                    .any(|expression| matches!(expression, Expr::Cube(_))),
                (GroupByExpr::Expressions(expressions, _), "grouping_sets_expr") => expressions
                    .iter()
                    .any(|expression| matches!(expression, Expr::GroupingSets(_))),
                (GroupByExpr::Expressions(_, modifiers), "with_rollup") => modifiers
                    .iter()
                    .any(|modifier| matches!(modifier, GroupByWithModifier::Rollup)),
                (GroupByExpr::Expressions(_, modifiers), "with_cube") => modifiers
                    .iter()
                    .any(|modifier| matches!(modifier, GroupByWithModifier::Cube)),
                (GroupByExpr::Expressions(_, modifiers), "with_totals") => modifiers
                    .iter()
                    .any(|modifier| matches!(modifier, GroupByWithModifier::Totals)),
                (GroupByExpr::Expressions(_, modifiers), "grouping_sets_modifier") => modifiers
                    .iter()
                    .any(|modifier| matches!(modifier, GroupByWithModifier::GroupingSets(_))),
                _ => false,
            };
            assert!(parsed, "SQLparser did not populate {field} for {sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted query: {sql}"
            );
        }

        let ordinary_grouping = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: "SELECT gpu_model FROM dataset GROUP BY gpu_model".to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(validate_query(&ordinary_grouping).is_ok());
        Ok(())
    }

    #[test]
    fn query_validator_rejects_order_by_fill_and_interpolate()
    -> Result<(), Box<dyn std::error::Error>> {
        for (sql, field) in [
            (
                "SELECT gpu_model FROM dataset ORDER BY gpu_model WITH FILL FROM 1 TO 3 STEP 1",
                "with_fill",
            ),
            (
                "SELECT gpu_model FROM dataset ORDER BY gpu_model INTERPOLATE (gpu_model)",
                "interpolate",
            ),
        ] {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            let statement = statements
                .first()
                .ok_or_else(|| io::Error::other("expected a parsed query statement"))?;
            let Statement::Query(query) = statement else {
                return Err(io::Error::other("expected one query statement").into());
            };
            let order_by = query
                .order_by
                .as_ref()
                .ok_or_else(|| io::Error::other("expected an ORDER BY clause"))?;
            let parsed = match field {
                "with_fill" => match &order_by.kind {
                    OrderByKind::Expressions(expressions) => expressions
                        .iter()
                        .any(|expression| expression.with_fill.is_some()),
                    OrderByKind::All(_) => false,
                },
                "interpolate" => order_by.interpolate.is_some(),
                _ => false,
            };
            assert!(parsed, "SQLparser did not populate {field} for {sql}");

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            assert!(
                matches!(validate_query(&request), Err(DataError::QueryRejected)),
                "accepted query: {sql}"
            );
        }

        let ordinary_ordering = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: "SELECT gpu_model FROM dataset ORDER BY gpu_model".to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(validate_query(&ordinary_ordering).is_ok());
        Ok(())
    }

    #[test]
    fn query_validator_rejects_with_fill_on_later_order_expression()
    -> Result<(), Box<dyn std::error::Error>> {
        let sql = "SELECT gpu_model, region FROM dataset ORDER BY gpu_model, region WITH FILL FROM 1 TO 3 STEP 1";
        let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
        let Some(Statement::Query(query)) = statements.first() else {
            return Err(io::Error::other("expected one parsed query statement").into());
        };
        let order_by = query
            .order_by
            .as_ref()
            .ok_or_else(|| io::Error::other("expected an ORDER BY clause"))?;
        let OrderByKind::Expressions(expressions) = &order_by.kind else {
            return Err(io::Error::other("expected expression-based ORDER BY").into());
        };
        assert_eq!(expressions.len(), 2);
        assert!(expressions[0].with_fill.is_none());
        assert!(expressions[1].with_fill.is_some());

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql: sql.to_owned(),
            format: QueryFormat::Json,
            max_rows: None,
        };
        assert!(matches!(
            validate_query(&request),
            Err(DataError::QueryRejected)
        ));

        let ordinary = QueryRequestV1 {
            sql: "SELECT gpu_model, region FROM dataset ORDER BY gpu_model, region".to_owned(),
            ..request
        };
        assert!(validate_query(&ordinary).is_ok());
        Ok(())
    }

    fn write_memory_stress_fixture(root: &Path) -> io::Result<PathBuf> {
        let data_path = root.join(SAMPLE_FILE);
        let mut file = File::create(&data_path)?;
        let padding = "x".repeat(11_879);
        for row in 0..MAX_RESULT_ROWS {
            writeln!(
                file,
                r#"{{"instance_id":"synthetic-{row:04}-{padding}","gpu_model":"H100","region":"us-east","asking_usd_micros_per_hour":1,"available_count":1,"observed_at":"2026-01-01T00:00:00Z"}}"#
            )?;
        }
        Ok(data_path)
    }

    fn memory_stress_request() -> QueryRequestV1 {
        QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            // Return only small integers while the aggregate retains the long
            // unique key, so row and serialized-byte caps cannot mask the pool.
            sql: "SELECT asking_usd_micros_per_hour FROM dataset GROUP BY instance_id, asking_usd_micros_per_hour".to_owned(),
            format: QueryFormat::Json,
            max_rows: Some(MAX_RESULT_ROWS),
        }
    }

    #[test]
    fn serialized_output_limit_is_enforced_for_json_and_csv()
    -> Result<(), Box<dyn std::error::Error>> {
        let columns = vec!["payload".to_owned()];
        let payload = "x".repeat(MAX_RESULT_BYTES);
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new(
                "payload",
                DataType::Utf8,
                false,
            )])),
            vec![Arc::new(StringArray::from(vec![payload.as_str()]))],
        )?;

        for format in [QueryFormat::Json, QueryFormat::Csv] {
            let mut rows = vec![vec![Some("prior".to_owned())]];
            let previous_rows = rows.clone();
            let mut output_bytes = serialized_size(format, &columns, &rows)?;
            let previous_size = output_bytes;
            assert!(matches!(
                append_batch_rows(&batch, format, &columns, &mut rows, &mut output_bytes,),
                Err(DataError::LimitExceeded)
            ));
            assert_eq!(rows, previous_rows);
            assert_eq!(output_bytes, previous_size);
        }
        Ok(())
    }

    #[test]
    fn complete_query_response_size_accepts_exact_limit_and_rejects_one_more_byte()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut result = QueryResultV1 {
            schema_version: "hyperai.data.query-result.v1",
            dataset_id: JSON_DATASET_ID.to_owned(),
            snapshot_digest: Sha256Digest::from_bytes([0; 32]),
            columns: vec!["payload".to_owned()],
            rows: vec![vec![Some("x".to_owned())]],
            format: QueryFormat::Json,
            csv: None,
            row_count: 1,
            output_bytes: 1,
            limits: QueryLimitsV1 {
                max_input_bytes: MAX_FILE_BYTES,
                max_sql_bytes: MAX_SQL_BYTES,
                max_duration_ms: 2_000,
                memory_bytes: QUERY_MEMORY_BYTES,
                partitions: 1,
                batch_rows: 1_024,
                max_result_rows: MAX_RESULT_ROWS,
                max_result_bytes: MAX_RESULT_BYTES,
            },
            claim_ceiling: "synthetic local query result only; no live source or compute claim",
            effect_boundary: "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS",
        };
        let initial_bytes = serde_json::to_vec(&result)?.len();
        assert!(initial_bytes < MAX_RESULT_BYTES);
        assert_eq!(serialized_response_size(&result)?, initial_bytes);

        let exact_payload_bytes = MAX_RESULT_BYTES - initial_bytes + 1;
        result.rows[0][0] = Some("x".repeat(exact_payload_bytes));
        assert_eq!(serde_json::to_vec(&result)?.len(), MAX_RESULT_BYTES);
        assert_eq!(serialized_response_size(&result)?, MAX_RESULT_BYTES);

        result.rows[0][0]
            .as_mut()
            .ok_or("fixed response payload is unexpectedly null")?
            .push('x');
        assert_eq!(serde_json::to_vec(&result)?.len(), MAX_RESULT_BYTES + 1);
        assert!(matches!(
            serialized_response_size(&result),
            Err(DataError::LimitExceeded)
        ));
        Ok(())
    }

    #[test]
    fn incremental_output_size_matches_json_and_csv_serializers()
    -> Result<(), Box<dyn std::error::Error>> {
        let columns = vec!["payload".to_owned(), "count".to_owned()];
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                Field::new("payload", DataType::Utf8, true),
                Field::new("count", DataType::UInt64, true),
            ])),
            vec![
                Arc::new(StringArray::from(vec![
                    Some("comma, and \"quote\" \\"),
                    Some("line\ncarriage\r\ttab\u{1} unicode 雪"),
                    None,
                ])),
                Arc::new(UInt64Array::from(vec![Some(7), None, Some(u64::MAX)])),
            ],
        )?;

        for format in [QueryFormat::Json, QueryFormat::Csv] {
            let mut rows = Vec::new();
            let mut output_bytes = serialized_size(format, &columns, &rows)?;
            append_batch_rows(&batch, format, &columns, &mut rows, &mut output_bytes)?;

            assert_eq!(
                output_bytes,
                serialized_size(format, &columns, &rows)?,
                "incremental byte count for {format:?}"
            );
            assert_eq!(
                rows,
                vec![
                    vec![
                        Some("comma, and \"quote\" \\".to_owned()),
                        Some("7".to_owned())
                    ],
                    vec![
                        Some("line\ncarriage\r\ttab\u{1} unicode 雪".to_owned()),
                        None,
                    ],
                    vec![None, Some(u64::MAX.to_string())],
                ]
            );
        }
        Ok(())
    }

    #[test]
    fn output_limit_accepts_exact_boundary_and_rejects_one_more_byte()
    -> Result<(), Box<dyn std::error::Error>> {
        let columns = vec!["payload".to_owned()];
        let schema = Arc::new(Schema::new(vec![Field::new(
            "payload",
            DataType::Utf8,
            false,
        )]));

        for format in [QueryFormat::Json, QueryFormat::Csv] {
            let base_size = serialized_size(format, &columns, &[])?;
            let fixed_row_bytes = match format {
                QueryFormat::Json => 4,
                QueryFormat::Csv => 1,
            };
            let payload_bytes = MAX_RESULT_BYTES
                .checked_sub(base_size + fixed_row_bytes)
                .ok_or("output header exceeded the configured ceiling")?;
            let payload = "x".repeat(payload_bytes);
            let exact_batch = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(StringArray::from(vec![payload.as_str()]))],
            )?;
            let mut rows = Vec::new();
            let mut output_bytes = base_size;
            append_batch_rows(&exact_batch, format, &columns, &mut rows, &mut output_bytes)?;
            assert_eq!(output_bytes, MAX_RESULT_BYTES);
            assert_eq!(output_bytes, serialized_size(format, &columns, &rows)?);

            let over_limit_payload = format!("{payload}x");
            let over_limit_batch = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(StringArray::from(vec![
                    over_limit_payload.as_str(),
                ]))],
            )?;
            let mut empty_rows = Vec::new();
            let mut base_output_bytes = base_size;
            assert!(matches!(
                append_batch_rows(
                    &over_limit_batch,
                    format,
                    &columns,
                    &mut empty_rows,
                    &mut base_output_bytes,
                ),
                Err(DataError::LimitExceeded)
            ));
            assert!(empty_rows.is_empty());
            assert_eq!(base_output_bytes, base_size);
        }
        Ok(())
    }

    #[test]
    fn configured_datafusion_memory_pool_rejects_growth_past_its_ceiling()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_limit(QUERY_MEMORY_BYTES, 1.0)
            .build_arc()?;
        assert!(matches!(
            runtime.memory_pool.memory_limit(),
            MemoryLimit::Finite(limit) if limit == QUERY_MEMORY_BYTES
        ));

        let mut reservation =
            MemoryConsumer::new("hyperai-data-contract-test").register(&runtime.memory_pool);
        reservation.try_grow(QUERY_MEMORY_BYTES)?;
        assert!(reservation.try_grow(1).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn bounded_query_wait_returns_limit_error_after_timeout() {
        let (_sender, mut cancellation) = tokio::sync::watch::channel(false);
        let execution = std::future::pending::<Result<(), DataError>>();

        assert!(matches!(
            await_bounded_query(execution, &mut cancellation, Duration::from_millis(1)).await,
            Err(DataError::LimitExceeded)
        ));
    }

    #[tokio::test]
    async fn query_memory_pool_rejects_aggregation_without_partial_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let data_path = write_memory_stress_fixture(directory.path())?;
        assert_eq!(fs::metadata(&data_path)?.len(), 12_039_000);
        assert_eq!(
            sha256_file(&data_path, MAX_FILE_BYTES)?.to_hex(),
            MEMORY_STRESS_FIXTURE_SHA256
        );

        let context =
            query_context_with_memory_limit(fs::read(&data_path)?, "json", 1024 * 1024).await?;
        let request = memory_stress_request();
        validate_query(&request)?;
        let result =
            tokio::time::timeout(Duration::from_secs(10), execute_query(&context, &request))
                .await?;

        assert!(
            matches!(result, Err(DataError::LimitExceeded)),
            "expected a stable limit error with no partial result, got {result:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn query_memory_stress_runs_under_configured_64_mib_pool()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let data_path = write_memory_stress_fixture(directory.path())?;
        let contents = fs::read(&data_path)?;
        assert_eq!(contents.len(), 12_039_000);
        assert_eq!(sha256(&contents).to_hex(), MEMORY_STRESS_FIXTURE_SHA256);
        let manifest = build_manifest(
            JSON_DATASET_ID,
            "json",
            SAMPLE_FILE,
            &contents,
            sha256(SYNTHETIC_SCHEMA.as_bytes()),
            MAX_RESULT_ROWS as u64,
        );
        let manifests_dir = directory.path().join("manifests");
        fs::create_dir_all(&manifests_dir)?;
        write_manifest(&manifests_dir, &manifest)?;
        let catalog = catalog(directory.path())?;
        assert_eq!(catalog.datasets[0].row_count, MAX_RESULT_ROWS as u64);
        let request = memory_stress_request();
        let result = query(directory.path(), &request).await?;

        assert_eq!(result.columns, ["asking_usd_micros_per_hour"]);
        assert_eq!(result.row_count, MAX_RESULT_ROWS);
        assert_eq!(result.snapshot_digest, manifest.content_digest);
        assert_eq!(result.limits.memory_bytes, QUERY_MEMORY_BYTES);
        assert!(result.rows.iter().all(|row| row == &[Some("1".to_owned())]));
        Ok(())
    }

    #[tokio::test]
    async fn catalog_and_query_reject_manifest_row_count_mismatch_for_json_and_parquet()
    -> Result<(), Box<dyn std::error::Error>> {
        for dataset_id in [JSON_DATASET_ID, PARQUET_DATASET_ID] {
            let directory = tempfile::tempdir()?;
            seed_synthetic(directory.path())?;
            let manifest_path = directory
                .path()
                .join("manifests")
                .join(format!("{dataset_id}.json"));
            let mut manifest: DatasetManifestV1 =
                serde_json::from_slice(&fs::read(&manifest_path)?)?;
            manifest.row_count = 2;
            fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;

            assert!(matches!(
                catalog(directory.path()),
                Err(DataError::InvalidCatalog)
            ));
            let request = QueryRequestV1 {
                dataset_id: dataset_id.to_owned(),
                sql: "SELECT COUNT(*) FROM dataset".to_owned(),
                format: QueryFormat::Json,
                max_rows: Some(1),
            };
            assert!(matches!(
                query(directory.path(), &request).await,
                Err(DataError::InvalidCatalog)
            ));
        }
        Ok(())
    }

    #[tokio::test]
    async fn query_scans_the_verified_snapshot_after_source_replacement()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        seed_synthetic(directory.path())?;

        for dataset_id in [JSON_DATASET_ID, PARQUET_DATASET_ID] {
            let request = QueryRequestV1 {
                dataset_id: dataset_id.to_owned(),
                sql: "SELECT gpu_model FROM dataset ORDER BY instance_id".to_owned(),
                format: QueryFormat::Json,
                max_rows: None,
            };
            let expected = query(directory.path(), &request).await?;
            let snapshot = capture_dataset_snapshot(directory.path(), dataset_id)?;
            let data_path = checked_dataset_path(directory.path(), &snapshot.manifest)?;
            fs::write(&data_path, b"changed after immutable snapshot capture")?;

            let actual = execute_verified_snapshot_query(snapshot, &request).await?;
            assert_eq!(actual, expected, "dataset format: {dataset_id}");
        }
        Ok(())
    }

    #[test]
    fn verified_snapshot_capture_rejects_changed_and_oversized_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let manifest = seed_synthetic(directory.path())?;
        let data_path = checked_dataset_path(directory.path(), &manifest)?;

        fs::write(&data_path, b"changed before capture")?;
        assert!(matches!(
            read_verified_dataset_bytes(&data_path, &manifest),
            Err(DataError::IntegrityMismatch)
        ));

        let oversized_length = usize::try_from(MAX_FILE_BYTES + 1)?;
        fs::write(&data_path, vec![0_u8; oversized_length])?;
        assert!(matches!(
            read_verified_dataset_bytes(&data_path, &manifest),
            Err(DataError::LimitExceeded)
        ));
        Ok(())
    }

    #[test]
    fn near_limit_nested_parentheses_do_not_panic() {
        const PREFIX: &str = "SELECT ";
        const EXPRESSION: &str = "available_count";
        const SUFFIX: &str = " FROM dataset";
        let nesting_depth =
            (MAX_SQL_BYTES - 1 - PREFIX.len() - EXPRESSION.len() - SUFFIX.len()) / 2;

        let sql = format!(
            "{PREFIX}{}{EXPRESSION}{}{SUFFIX}",
            "(".repeat(nesting_depth),
            ")".repeat(nesting_depth),
        );
        assert_eq!(nesting_depth, 2_030);
        assert_eq!(sql.len(), MAX_SQL_BYTES - 1);

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql,
            format: QueryFormat::Json,
            max_rows: Some(1),
        };
        let validation = std::panic::catch_unwind(|| validate_query(&request));
        assert!(
            matches!(validation, Ok(Ok(()) | Err(DataError::QueryRejected))),
            "validator panicked or returned an unexpected error for bounded nested SQL",
        );
    }

    #[test]
    fn near_limit_left_associative_expression_chain_does_not_panic() {
        const TERM_COUNT: usize = 582;

        let expression = vec!["1=1"; TERM_COUNT].join(" OR ");
        let sql = format!("SELECT {expression} FROM dataset");
        assert!(sql.len() < MAX_SQL_BYTES);

        let request = QueryRequestV1 {
            dataset_id: JSON_DATASET_ID.to_owned(),
            sql,
            format: QueryFormat::Json,
            max_rows: Some(1),
        };
        let validation = std::panic::catch_unwind(|| validate_query(&request));
        assert!(
            matches!(validation, Ok(Ok(()))),
            "validator panicked or rejected the supported bounded expression chain",
        );
    }

    #[tokio::test]
    async fn deterministic_sql_grammar_combinations_validate_and_plan()
    -> Result<(), Box<dyn std::error::Error>> {
        let columns = [
            "gpu_model",
            "region",
            "asking_usd_micros_per_hour",
            "available_count",
        ];
        let predicates = [
            "",
            " WHERE gpu_model = 'A100'",
            " WHERE available_count IS NULL",
        ];
        let orderings = ["", " ORDER BY gpu_model", " ORDER BY available_count DESC"];
        let limits = ["", " LIMIT 1", " LIMIT 3 OFFSET 1"];
        let relations = [("dataset", false), ("dataset AS d", true)];
        let mut positive_statements = std::collections::HashSet::new();

        for (relation, qualified) in relations {
            for column in columns {
                let projection = if qualified {
                    format!("d.{column}")
                } else {
                    column.to_owned()
                };
                for predicate in predicates {
                    for ordering in orderings {
                        for limit in limits {
                            let sql = format!(
                                "SELECT {projection} FROM {relation}{predicate}{ordering}{limit}"
                            );
                            assert!(sql.len() <= MAX_SQL_BYTES, "oversized generated SQL");
                            let request = QueryRequestV1 {
                                dataset_id: JSON_DATASET_ID.to_owned(),
                                sql: sql.clone(),
                                format: QueryFormat::Json,
                                max_rows: Some(10),
                            };
                            let validation = std::panic::catch_unwind(|| validate_query(&request));
                            assert!(
                                matches!(validation, Ok(Ok(()))),
                                "supported grammar combination was not accepted: {sql}; {validation:?}"
                            );
                            let plan = preflight_query_plan(&request).await;
                            assert!(
                                plan.is_ok(),
                                "validated grammar combination failed schema-only planning: {sql}; {plan:?}"
                            );
                            assert!(positive_statements.insert(sql));
                        }
                    }
                }
            }
        }
        assert_eq!(positive_statements.len(), 216);

        let negative_statements = [
            "SELECT gpu_model FROM dataset TABLESAMPLE BERNOULLI (50)",
            "SELECT gpu_model FROM other_dataset",
            "SELECT RANDOM() FROM dataset",
            "SELECT d.gpu_model FROM dataset AS d (gpu_model INT)",
            "SELECT gpu_model INTO TABLE copied FROM dataset",
            "WITH source AS (SELECT gpu_model FROM dataset) SELECT gpu_model FROM source",
            "SELECT gpu_model FROM dataset UNION SELECT gpu_model FROM dataset",
            "SELECT a.gpu_model FROM dataset AS a JOIN dataset AS b ON a.instance_id = b.instance_id",
            "SELECT gpu_model FROM dataset FETCH FIRST 1 ROW ONLY",
            "SELECT gpu_model FROM dataset GROUP BY ALL",
            "SELECT gpu_model FROM dataset LIMIT 1 BY gpu_model",
        ];

        for sql in negative_statements {
            let statements = Parser::parse_sql(&GenericDialect {}, sql)?;
            assert_eq!(
                statements.len(),
                1,
                "negative corpus must parse once: {sql}"
            );
            assert!(
                matches!(statements.first(), Some(Statement::Query(_))),
                "negative corpus must contain a query statement: {sql}"
            );
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.to_owned(),
                format: QueryFormat::Json,
                max_rows: Some(1),
            };
            let validation = std::panic::catch_unwind(|| validate_query(&request));
            assert!(
                matches!(validation, Ok(Err(DataError::QueryRejected))),
                "unsupported parsed grammar was not rejected: {sql}; {validation:?}"
            );
        }
        assert_eq!(negative_statements.len(), 11);
        Ok(())
    }

    #[derive(Clone, Copy)]
    struct GrammarFuzzRng(u16);

    impl GrammarFuzzRng {
        fn index(&mut self, upper_bound: usize) -> usize {
            self.0 ^= self.0 << 7;
            self.0 ^= self.0 >> 9;
            self.0 ^= self.0 << 8;
            usize::from(self.0) % upper_bound
        }
    }

    fn grammar_keyword(rng: &mut GrammarFuzzRng, keyword: &str) -> String {
        match rng.index(3) {
            0 => keyword.to_ascii_uppercase(),
            1 => keyword.to_ascii_lowercase(),
            _ => keyword
                .chars()
                .enumerate()
                .map(|(index, character)| {
                    if index % 2 == 0 {
                        character.to_ascii_lowercase()
                    } else {
                        character.to_ascii_uppercase()
                    }
                })
                .collect(),
        }
    }

    fn join_grammar_tokens(rng: &mut GrammarFuzzRng, tokens: Vec<String>) -> String {
        let mut sql = String::new();
        for (index, token) in tokens.into_iter().enumerate() {
            if index > 0 {
                sql.push_str(match rng.index(3) {
                    0 => " ",
                    1 => "\n",
                    _ => "\t",
                });
            }
            sql.push_str(&token);
        }
        sql
    }

    fn generated_supported_sql(rng: &mut GrammarFuzzRng) -> (String, [usize; 5]) {
        const COLUMNS: [&str; 4] = [
            "gpu_model",
            "region",
            "asking_usd_micros_per_hour",
            "available_count",
        ];

        let column_index = rng.index(COLUMNS.len());
        let alias_index = rng.index(2);
        let predicate_index = rng.index(3);
        let ordering_index = rng.index(3);
        let limit_index = rng.index(3);
        let column = COLUMNS[column_index];
        let mut tokens = vec![grammar_keyword(rng, "SELECT")];
        tokens.push(if alias_index == 1 {
            format!("d.{column}")
        } else {
            column.to_owned()
        });
        tokens.push(grammar_keyword(rng, "FROM"));
        tokens.push("dataset".to_owned());
        if alias_index == 1 {
            tokens.push(grammar_keyword(rng, "AS"));
            tokens.push("d".to_owned());
        }

        match predicate_index {
            1 => {
                tokens.push(grammar_keyword(rng, "WHERE"));
                tokens.push("gpu_model".to_owned());
                tokens.push("=".to_owned());
                tokens.push(format!("'GPU-{}'", rng.index(257)));
            }
            2 => {
                tokens.push(grammar_keyword(rng, "WHERE"));
                tokens.push("available_count".to_owned());
                tokens.push(grammar_keyword(rng, "IS"));
                tokens.push(grammar_keyword(rng, "NULL"));
            }
            _ => {}
        }

        match ordering_index {
            1 => {
                tokens.push(grammar_keyword(rng, "ORDER"));
                tokens.push(grammar_keyword(rng, "BY"));
                tokens.push("gpu_model".to_owned());
            }
            2 => {
                tokens.push(grammar_keyword(rng, "ORDER"));
                tokens.push(grammar_keyword(rng, "BY"));
                tokens.push("available_count".to_owned());
                tokens.push(grammar_keyword(rng, "DESC"));
            }
            _ => {}
        }

        match limit_index {
            1 => {
                tokens.push(grammar_keyword(rng, "LIMIT"));
                tokens.push((1 + rng.index(3)).to_string());
            }
            2 => {
                tokens.push(grammar_keyword(rng, "LIMIT"));
                tokens.push((1 + rng.index(3)).to_string());
                tokens.push(grammar_keyword(rng, "OFFSET"));
                tokens.push(rng.index(4).to_string());
            }
            _ => {}
        }

        (
            join_grammar_tokens(rng, tokens),
            [
                column_index,
                alias_index,
                predicate_index,
                ordering_index,
                limit_index,
            ],
        )
    }

    fn generated_unsupported_sql(rng: &mut GrammarFuzzRng) -> (String, usize) {
        let variant = rng.index(5);
        let number = 1 + rng.index(100);
        let tokens = match variant {
            0 => vec![
                grammar_keyword(rng, "SELECT"),
                "gpu_model".to_owned(),
                grammar_keyword(rng, "FROM"),
                format!("unregistered_{number}"),
            ],
            1 => vec![
                grammar_keyword(rng, "SELECT"),
                format!("untrusted_function_{number}()"),
                grammar_keyword(rng, "FROM"),
                "dataset".to_owned(),
            ],
            2 => vec![
                grammar_keyword(rng, "SELECT"),
                "gpu_model".to_owned(),
                grammar_keyword(rng, "FROM"),
                "dataset".to_owned(),
                grammar_keyword(rng, "TABLESAMPLE"),
                grammar_keyword(rng, "BERNOULLI"),
                format!("({number})"),
            ],
            3 => vec![
                grammar_keyword(rng, "SELECT"),
                "gpu_model".to_owned(),
                grammar_keyword(rng, "FROM"),
                "dataset".to_owned(),
                grammar_keyword(rng, "LIMIT"),
                number.to_string(),
                grammar_keyword(rng, "BY"),
                "available_count".to_owned(),
            ],
            _ => vec![
                grammar_keyword(rng, "SELECT"),
                "gpu_model".to_owned(),
                grammar_keyword(rng, "FROM"),
                "dataset".to_owned(),
                grammar_keyword(rng, "GROUP"),
                grammar_keyword(rng, "BY"),
                grammar_keyword(rng, "ALL"),
            ],
        };
        (join_grammar_tokens(rng, tokens), variant)
    }

    #[test]
    fn fixed_seed_grammar_sql_fuzz_stays_within_policy() {
        const CASES_PER_CLASS: usize = 4_096;
        let mut rng = GrammarFuzzRng(0x4932);
        let mut positive_coverage = [
            vec![0_usize; 4],
            vec![0_usize; 2],
            vec![0_usize; 3],
            vec![0_usize; 3],
            vec![0_usize; 3],
        ];

        for _ in 0..CASES_PER_CLASS {
            let (sql, axes) = generated_supported_sql(&mut rng);
            assert!(
                sql.len() <= MAX_SQL_BYTES,
                "generated SQL exceeded byte cap"
            );
            for (coverage, selected) in positive_coverage.iter_mut().zip(axes) {
                coverage[selected] += 1;
            }
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.clone(),
                format: QueryFormat::Json,
                max_rows: Some(10),
            };
            let validation =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| validate_query(&request)));
            assert!(
                matches!(validation, Ok(Ok(()))),
                "generated supported SQL failed validation: {sql}; {validation:?}"
            );
        }

        for (axis, coverage) in positive_coverage.iter().enumerate() {
            assert!(
                coverage.iter().all(|count| *count > 0),
                "fixed-seed generator missed a supported production on axis {axis}: {coverage:?}"
            );
        }

        let dialect = GenericDialect {};
        let mut negative_coverage = [0_usize; 5];
        for _ in 0..CASES_PER_CLASS {
            let (sql, variant) = generated_unsupported_sql(&mut rng);
            assert!(
                sql.len() <= MAX_SQL_BYTES,
                "generated SQL exceeded byte cap"
            );
            negative_coverage[variant] += 1;
            let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Parser::parse_sql(&dialect, &sql)
            }));
            assert!(
                matches!(
                    parsed,
                    Ok(Ok(ref statements))
                        if statements.len() == 1 && matches!(statements.first(), Some(Statement::Query(_)))
                ),
                "generated negative SQL did not parse as exactly one query: {sql}; {parsed:?}"
            );

            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: sql.clone(),
                format: QueryFormat::Json,
                max_rows: Some(10),
            };
            let validation =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| validate_query(&request)));
            assert!(
                matches!(validation, Ok(Err(DataError::QueryRejected))),
                "generated unsupported SQL escaped the read-only policy: {sql}; {validation:?}"
            );
        }

        assert!(negative_coverage.iter().all(|count| *count > 0));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            rng_seed: proptest::test_runner::RngSeed::Fixed(0x4859_5045_5241_4931),
            ..ProptestConfig::default()
        })]

        #[test]
        fn bounded_arbitrary_sql_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..=MAX_SQL_BYTES)) {
            let request = QueryRequestV1 {
                dataset_id: JSON_DATASET_ID.to_owned(),
                sql: String::from_utf8_lossy(&bytes).into_owned(),
                format: QueryFormat::Json,
                max_rows: Some(1),
            };
            let _ = validate_query(&request);
        }
    }
}

#[cfg(test)]
mod synthetic_market_time_series_tests {
    use super::*;

    #[test]
    fn synthetic_hourly_release_is_stable_and_preserves_missingness()
    -> Result<(), Box<dyn std::error::Error>> {
        let first = synthetic_market_time_series_v1()?;
        let second = synthetic_market_time_series_v1()?;
        assert_eq!(first, second);
        assert_eq!(first.dataset_id, SYNTHETIC_TIME_SERIES_ID);
        assert_eq!(first.periods.len(), 8);
        assert_eq!(first.observations.len(), 23);
        assert_eq!(first.freshness, "synthetic_not_market_data");
        assert!(!first.snapshot_digest.to_hex().is_empty());
        assert!(
            !first
                .observations
                .iter()
                .any(|point| { point.instrument_id == "a100-us-east" && point.period_index == 4 })
        );
        assert!(first.observations.iter().any(|point| {
            point.instrument_id == "a100-us-west"
                && point.period_index == 5
                && point.available_count.is_none()
        }));
        assert!(first.observations.iter().any(|point| {
            point.instrument_id == "a100-eu-west"
                && point.period_index == 6
                && point.available_count == Some(0)
        }));
        Ok(())
    }
}
