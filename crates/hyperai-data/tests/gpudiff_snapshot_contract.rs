//! Bounded `GPUDiff` import contract tests using fixed non-live fixtures.

use std::{error::Error, fs};

use hyperai_data::{
    DataError, QueryFormat, QueryRequestV1, catalog, import_gpudiff_snapshot_v1, query,
};

const DATASET_ID: &str = "hyperai.gpudiff_public_gpu_offers.v1";

fn offer(id: &str, price: &str, fixture: bool) -> String {
    format!(
        r#"{{"id":"{id}","provider":"test-provider","sku":"gpu-x","price":{price},"unit":"USD/GPU-hour","pricing_type":"ondemand","region":"test-region","attrs":{{"gpu_model":"GPU-X","metric":"list_price","sample_size":2,"vram_gb":24}},"provenance":{{"url":"https://provider.example/offers/gpu-x","observed_at":"2026-10-05T12:00:00Z"}},"fixture":{fixture}}}"#
    )
}

fn feed(records: &[String]) -> Vec<u8> {
    format!("[{}]", records.join(",")).into_bytes()
}

#[tokio::test]
async fn imports_exact_bytes_normalizes_and_queries_through_datafusion()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    let raw = feed(&[offer("offer-1", "1.234567", false)]);
    let imported = import_gpudiff_snapshot_v1(
        &root,
        &raw,
        Some("\"feed-v1\""),
        Some("Mon, 05 Oct 2026 12:00:00 GMT"),
    )?;

    assert_eq!(imported.manifest.dataset_id, DATASET_ID);
    assert_eq!(imported.manifest.row_count, 1);
    assert_eq!(imported.receipt.row_count, 1);
    assert_eq!(
        imported.receipt.raw_source_digest.to_hex(),
        sha256_hex(&raw)
    );
    assert_eq!(
        fs::read(
            root.join("sources/gpudiff")
                .join(format!("{}.json", sha256_hex(&raw)))
        )?,
        raw
    );
    assert_eq!(catalog(&root)?.datasets.len(), 1);

    let result = query(
        &root,
        &QueryRequestV1 {
            dataset_id: DATASET_ID.to_owned(),
            sql: "SELECT id, price_decimal, price_micros_per_source_unit, provenance_url FROM dataset".to_owned(),
            format: QueryFormat::Json,
            max_rows: Some(10),
        },
    )
    .await?;
    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][0].as_deref(), Some("offer-1"));
    assert_eq!(result.rows[0][1].as_deref(), Some("1.234567"));
    assert_eq!(result.rows[0][2].as_deref(), Some("1234567"));
    assert_eq!(
        result.rows[0][3].as_deref(),
        Some("https://provider.example/offers/gpu-x")
    );

    let repeated = import_gpudiff_snapshot_v1(&root, &raw, None, None)?;
    assert_eq!(repeated, imported);
    assert_eq!(
        catalog(&root)?.datasets[0].snapshot_digest,
        imported.manifest.content_digest
    );
    Ok(())
}

#[test]
fn rejects_fixture_duplicate_and_unknown_schema_rows_before_writing() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    let fixture = feed(&[offer("fixture-1", "1.0", true)]);
    assert!(matches!(
        import_gpudiff_snapshot_v1(&root, &fixture, None, None),
        Err(DataError::InvalidCatalog)
    ));

    let duplicate = feed(&[
        offer("duplicate-1", "1.0", false),
        offer("duplicate-1", "2.0", false),
    ]);
    assert!(matches!(
        import_gpudiff_snapshot_v1(&root, &duplicate, None, None),
        Err(DataError::InvalidCatalog)
    ));

    let unknown = offer("unknown-1", "1.0", false)
        .replace("\"fixture\":false", "\"fixture\":false,\"new_field\":true");
    assert!(matches!(
        import_gpudiff_snapshot_v1(&root, format!("[{unknown}]").as_bytes(), None, None),
        Err(DataError::InvalidCatalog)
    ));
    assert!(!root.exists());
    Ok(())
}

#[test]
fn changed_captures_are_immutable_and_retention_fails_closed() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    let mut current = None;
    let mut first_path = None;
    for index in 0..16 {
        let raw = feed(&[offer("same-offer", &format!("1.{index:06}"), false)]);
        let imported = import_gpudiff_snapshot_v1(&root, &raw, None, None)?;
        if index == 0 {
            first_path = Some(imported.manifest.relative_path.clone());
        }
        current = Some(imported);
    }
    let current = current.ok_or("expected retained capture")?;
    let current_catalog = catalog(&root)?;
    assert_eq!(
        current_catalog.datasets[0].snapshot_digest,
        current.manifest.content_digest
    );
    let first_path = first_path.ok_or("expected initial immutable snapshot")?;
    assert!(root.join(first_path).exists());

    let seventeenth = feed(&[offer("same-offer", "2.000000", false)]);
    assert!(matches!(
        import_gpudiff_snapshot_v1(&root, &seventeenth, None, None),
        Err(DataError::CaptureRetentionFull)
    ));
    assert_eq!(
        catalog(&root)?.datasets[0].snapshot_digest,
        current.manifest.content_digest
    );
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hyperai_foundation::Sha256Digest::from_bytes(Sha256::digest(bytes).into()).to_hex()
}
