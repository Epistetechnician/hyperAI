//! Local catalog manifest, integrity, and path-boundary checks.

use std::{collections::BTreeSet, error::Error, fs, path::Path};

use hyperai_data::{
    DataError, DatasetReadStatusV1, catalog, describe, inspect_dataset_status_v1, seed_synthetic,
};

const JSON_DATASET_ID: &str = "hyperai.synthetic_market_observations_json.v1";
const JSON_SAMPLE: &str = "synthetic-market-observations.json";

#[test]
fn workload_trace_status_inspection_uses_the_verified_local_catalog() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");

    assert_eq!(
        inspect_dataset_status_v1(&root, JSON_DATASET_ID),
        DatasetReadStatusV1::NotSeeded
    );

    let manifest = seed_synthetic(&root)?;
    assert_eq!(
        inspect_dataset_status_v1(&root, JSON_DATASET_ID),
        DatasetReadStatusV1::Bound(manifest.content_digest)
    );

    let sample_path = root.join(JSON_SAMPLE);
    fs::remove_file(&sample_path)?;
    assert_eq!(
        inspect_dataset_status_v1(&root, JSON_DATASET_ID),
        DatasetReadStatusV1::Missing
    );

    seed_synthetic(&root)?;
    fs::write(&sample_path, b"changed local synthetic bytes\n")?;
    assert_eq!(
        inspect_dataset_status_v1(&root, JSON_DATASET_ID),
        DatasetReadStatusV1::Conflicting
    );
    assert_eq!(
        inspect_dataset_status_v1(&root, "unknown-dataset"),
        DatasetReadStatusV1::Unavailable
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let permission_root = temporary.path().join("permission-data");
        let permission_manifest = seed_synthetic(&permission_root)?;
        let permission_sample = permission_root.join(JSON_SAMPLE);
        let mut permissions = fs::metadata(&permission_sample)?.permissions();
        permissions.set_mode(0o0);
        fs::set_permissions(&permission_sample, permissions)?;
        let status = inspect_dataset_status_v1(&permission_root, JSON_DATASET_ID);
        let read_result = fs::read(&permission_sample);

        let mut permissions = fs::metadata(&permission_sample)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&permission_sample, permissions)?;

        match read_result {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                assert_eq!(status, DatasetReadStatusV1::Denied);
            }
            Ok(_) => assert_eq!(
                status,
                DatasetReadStatusV1::Bound(permission_manifest.content_digest)
            ),
            Err(error) => return Err(error.into()),
        }
    }

    Ok(())
}

#[test]
fn synthetic_seed_and_manifest_are_byte_deterministic() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");

    let empty = catalog(&root)?;
    assert_eq!(empty.status, "empty");
    assert!(empty.datasets.is_empty());

    let first = seed_synthetic(&root)?;
    let manifest_path = root
        .join("manifests")
        .join("hyperai.synthetic_market_observations_json.v1.json");
    let manifest_bytes = fs::read(&manifest_path)?;
    let data_bytes = fs::read(root.join(&first.relative_path))?;
    let second = seed_synthetic(&root)?;

    assert_eq!(first, second);
    assert_eq!(manifest_bytes, fs::read(&manifest_path)?);
    assert_eq!(data_bytes, fs::read(root.join(&first.relative_path))?);
    let ready = catalog(&root)?;
    assert_eq!(ready.status, "ready");
    assert_eq!(ready.datasets.len(), 2);
    assert_eq!(ready.datasets[0].row_count, 3);
    assert_eq!(ready.datasets[0].freshness, "synthetic_not_market_data");
    Ok(())
}

#[test]
fn changed_sample_bytes_are_refused_and_never_overwritten() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    let manifest = seed_synthetic(&root)?;
    let path = root.join(&manifest.relative_path);
    fs::write(&path, b"changed bytes\n")?;

    assert!(matches!(catalog(&root), Err(DataError::IntegrityMismatch)));
    assert!(matches!(seed_synthetic(&root), Err(DataError::Conflict)));
    assert_eq!(fs::read(path)?, b"changed bytes\n");
    Ok(())
}

#[test]
fn manifest_schema_and_dataset_identity_fail_closed() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;
    let manifest_path = root
        .join("manifests")
        .join("hyperai.synthetic_market_observations_json.v1.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest["schema_version"] = serde_json::Value::String("hyperai.data.manifest.v9".to_owned());
    fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;

    assert!(matches!(catalog(&root), Err(DataError::InvalidCatalog)));
    assert!(matches!(
        describe(&root, "../outside"),
        Err(DataError::DatasetNotFound)
    ));
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_dataset_path_is_rejected() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    let outside = temporary.path().join("outside.ndjson");
    fs::write(&outside, b"outside data\n")?;
    let manifest = seed_synthetic(&root)?;
    fs::remove_file(root.join(&manifest.relative_path))?;
    symlink(&outside, root.join(&manifest.relative_path))?;

    assert!(matches!(catalog(&root), Err(DataError::InvalidCatalog)));
    Ok(())
}

#[test]
fn synthetic_manifest_path_mutations_are_rejected() -> Result<(), Box<dyn Error>> {
    const DATASETS: &[(&str, &str, usize)] = &[
        (
            "hyperai.synthetic_market_observations_json.v1",
            "synthetic-market-observations.json",
            497,
        ),
        (
            "hyperai.synthetic_market_observations_parquet.v1",
            "synthetic-market-observations.parquet",
            539,
        ),
    ];
    const INSERTION_TOKENS: &[&str] = &[
        ".", "..", "/", "//", "../", "./", "\\", "..\\", "%2e", "%2f", ":", "\0", "\u{2215}",
        "\u{ff0f}",
    ];

    fn path_mutations(relative_path: &str, outside_path: &Path) -> BTreeSet<String> {
        let mut mutations = BTreeSet::from([
            String::new(),
            ".".to_owned(),
            "..".to_owned(),
            "/".to_owned(),
            outside_path.to_string_lossy().into_owned(),
            format!("../{relative_path}"),
            format!("nested/../{relative_path}"),
            format!("./{relative_path}"),
            format!(r"C:\outside\{relative_path}"),
            format!(r"\\server\share\{relative_path}"),
            format!(r"\\?\C:\outside\{relative_path}"),
        ]);

        let boundaries = relative_path
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(relative_path.len()));
        for boundary in boundaries {
            for token in INSERTION_TOKENS {
                let mut mutation = relative_path.to_owned();
                mutation.insert_str(boundary, token);
                mutations.insert(mutation);
            }
        }

        mutations
    }

    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;
    assert_eq!(catalog(&root)?.datasets.len(), DATASETS.len());

    for (dataset_id, relative_path, expected_mutation_count) in DATASETS {
        let dataset_path = root.join(relative_path);
        let outside_path = temporary.path().join(relative_path);
        fs::write(&outside_path, fs::read(&dataset_path)?)?;

        let manifest_path = root.join("manifests").join(format!("{dataset_id}.json"));
        let original_bytes = fs::read(&manifest_path)?;
        let original_manifest: serde_json::Value = serde_json::from_slice(&original_bytes)?;
        assert_eq!(original_manifest["relative_path"], *relative_path);

        let mutations = path_mutations(relative_path, &outside_path);
        assert_eq!(mutations.len(), *expected_mutation_count);
        for mutation in mutations {
            let mut changed_manifest = original_manifest.clone();
            changed_manifest["relative_path"] = serde_json::Value::String(mutation.clone());
            fs::write(&manifest_path, serde_json::to_vec(&changed_manifest)?)?;
            let result = catalog(&root);
            fs::write(&manifest_path, &original_bytes)?;

            assert!(
                matches!(result, Err(DataError::InvalidCatalog)),
                "manifest path mutation must be rejected: {mutation:?}"
            );
        }
    }

    assert_eq!(catalog(&root)?.status, "ready");
    Ok(())
}

#[test]
fn duplicate_manifest_ids_are_rejected_by_catalog_and_owner_status() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;

    let manifests = root.join("manifests");
    let canonical_manifest = manifests.join(format!("{JSON_DATASET_ID}.json"));
    let manifest_bytes = fs::read(&canonical_manifest)?;
    let alias_manifest = manifests.join("shadow.json");
    fs::write(&alias_manifest, &manifest_bytes)?;

    assert!(matches!(catalog(&root), Err(DataError::InvalidCatalog)));
    assert_eq!(
        inspect_dataset_status_v1(&root, JSON_DATASET_ID),
        DatasetReadStatusV1::Conflicting
    );
    assert_eq!(fs::read(&canonical_manifest)?, manifest_bytes);
    assert_eq!(fs::read(alias_manifest)?, manifest_bytes);
    Ok(())
}

#[test]
fn third_distinct_manifest_id_is_rejected_by_catalog() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;

    let manifests = root.join("manifests");
    let source = manifests.join(format!("{JSON_DATASET_ID}.json"));
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(source)?)?;
    manifest["dataset_id"] =
        serde_json::Value::String("hyperai.synthetic_market_observations_third.v1".to_owned());
    fs::write(manifests.join("third.json"), serde_json::to_vec(&manifest)?)?;

    assert!(
        matches!(catalog(&root), Err(DataError::InvalidCatalog)),
        "a third distinct manifest identity must exceed the v1 catalog bound"
    );
    Ok(())
}

#[test]
fn manifest_directory_entry_limit_rejects_the_65th_entry() -> Result<(), Box<dyn Error>> {
    const MAX_EXPECTED_DIRECTORY_ENTRIES: usize = 64;
    const SEEDED_MANIFEST_COUNT: usize = 2;

    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("data");
    seed_synthetic(&root)?;
    let manifests = root.join("manifests");

    let overflow_index = MAX_EXPECTED_DIRECTORY_ENTRIES - SEEDED_MANIFEST_COUNT;
    for index in 0..overflow_index {
        fs::write(manifests.join(format!("metadata-{index:02}.txt")), [])?;
    }
    let at_limit = catalog(&root)?;
    assert_eq!(at_limit.status, "ready");
    assert_eq!(at_limit.datasets.len(), SEEDED_MANIFEST_COUNT);

    fs::write(
        manifests.join(format!("metadata-{overflow_index:02}.txt")),
        [],
    )?;
    assert!(
        matches!(catalog(&root), Err(DataError::InvalidCatalog)),
        "the 65th directory entry must be rejected"
    );
    Ok(())
}
