//! Synthetic Bronze/Silver/Gold derivation and source-integrity checks.

use std::error::Error;

use hyperai_data::{DataError, derive_synthetic_release, seed_synthetic};

fn seeded_root() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;
    Ok((temporary, root))
}

#[tokio::test]
async fn synthetic_release_is_deterministic_and_preserves_unknown_availability()
-> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    let release = derive_synthetic_release(&root).await?;
    let repeated = derive_synthetic_release(&root).await?;

    assert_eq!(release, repeated);
    assert_eq!(release.schema_version, "hyperai.data.synthetic-release.v1");
    assert_eq!(
        release.bronze.dataset_id,
        "hyperai.synthetic_market_observations_json.v1"
    );
    assert_eq!(release.silver.len(), 3);
    assert_eq!(release.gold.len(), 3);
    assert_eq!(
        release.claim_ceiling,
        "HyperAI-authored synthetic observation derivation only; not live market data"
    );
    assert_eq!(
        release.effect_boundary,
        "LOCAL_READ_ONLY_NO_NETWORK_NO_PROVIDER_NO_FUNDS"
    );

    let observation = release
        .silver
        .iter()
        .find(|row| row.gpu_model == "MI300X")
        .ok_or("synthetic observation missing")?;
    assert_eq!(observation.available_count, None);
    assert_eq!(observation.asking_usd_micros_per_hour, 2_100_000);
    assert_eq!(observation.freshness, "synthetic_not_market_data");

    let summary = release
        .gold
        .iter()
        .find(|row| row.gpu_model == "MI300X")
        .ok_or("synthetic group summary missing")?;
    assert_eq!(summary.unknown_availability_records, 1);
    assert_eq!(summary.known_availability_records, 0);
    assert_eq!(summary.available_units_total, None);
    Ok(())
}

#[tokio::test]
async fn modified_input_fails_without_returning_a_partial_release() -> Result<(), Box<dyn Error>> {
    let (_temporary, root) = seeded_root()?;
    std::fs::write(
        root.join("synthetic-market-observations.json"),
        br#"{"instance_id":"altered"}"#,
    )?;

    assert!(matches!(
        derive_synthetic_release(&root).await,
        Err(DataError::IntegrityMismatch)
    ));
    Ok(())
}
