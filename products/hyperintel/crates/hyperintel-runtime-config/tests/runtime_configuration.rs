//! Contract coverage for strict local runtime configuration and secret references.

use std::{fs, path::Path};

use hyperintel_core::{ProtocolId, Sha256Digest};
use hyperintel_runtime_config::{
    DatabasePathRejectionV1, MAX_RUNTIME_CONFIGURATION_BYTES, RuntimeConfigurationError,
    SECRET_REFERENCE_SCHEMA_V1, SecretReferenceError, SecretReferenceV1,
    load_runtime_configuration_v1, parse_runtime_configuration_v1,
};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const EXPECTED_EXAMPLE_DIGEST: &str =
    "875cd38d950250ec5afe7fba6d047a642768294f775c753dd104c8bd328155cc";

#[test]
fn contract_safe_example_parses_and_has_stable_canonical_identity() -> TestResult {
    let example = include_bytes!("../../../config/examples/local-hermetic.json");
    let validated = parse_runtime_configuration_v1(example)?;
    assert_eq!(
        validated.document().runtime_id.as_str(),
        "runtime:local-hermetic-1"
    );
    assert_eq!(
        validated.replay_database_path(),
        Path::new("/var/lib/hyperintel/replay.sqlite3")
    );
    assert_eq!(
        validated.execution_journal_database_path(),
        Path::new("/var/lib/hyperintel/execution-journal.sqlite3")
    );
    assert_eq!(
        validated.configuration_digest().to_string(),
        EXPECTED_EXAMPLE_DIGEST
    );

    let reordered = br#"{
      "local_persistence": {
        "execution_journal_database": {
          "max_connections": 4,
          "path": "/var/lib/hyperintel/execution-journal.sqlite3"
        },
        "replay_database": {
          "max_connections": 4,
          "path": "/var/lib/hyperintel/replay.sqlite3"
        }
      },
      "profile": "LOCAL_HERMETIC",
      "runtime_id": "runtime:local-hermetic-1",
      "schema_version": "hyperintel.runtime-configuration.v1"
    }"#;
    assert_eq!(
        parse_runtime_configuration_v1(reordered)?.configuration_digest(),
        validated.configuration_digest()
    );
    Ok(())
}

#[test]
fn contract_empty_oversized_and_malformed_documents_fail_closed() {
    assert!(matches!(
        parse_runtime_configuration_v1(&[]),
        Err(RuntimeConfigurationError::EmptyDocument)
    ));
    let oversized = vec![b' '; MAX_RUNTIME_CONFIGURATION_BYTES + 1];
    assert!(matches!(
        parse_runtime_configuration_v1(&oversized),
        Err(RuntimeConfigurationError::DocumentTooLarge { .. })
    ));
    assert!(matches!(
        parse_runtime_configuration_v1(b"{"),
        Err(RuntimeConfigurationError::Json(_))
    ));
}

#[test]
fn contract_schema_profile_identity_and_strict_fields_fail_closed() -> TestResult {
    let mut unknown = configuration_value(
        "/var/lib/hyperintel/replay.sqlite3",
        "/var/lib/hyperintel/journal.sqlite3",
        4,
        4,
    );
    unknown["unknown"] = json!(true);
    assert_json_error(&serde_json::to_vec(&unknown)?);

    for pointer in [
        "/local_persistence",
        "/local_persistence/replay_database",
        "/local_persistence/execution_journal_database",
    ] {
        let mut nested_unknown = configuration_value(
            "/var/lib/hyperintel/replay.sqlite3",
            "/var/lib/hyperintel/journal.sqlite3",
            4,
            4,
        );
        nested_unknown
            .pointer_mut(pointer)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("nested configuration is not an object")?
            .insert("unknown".to_owned(), json!(true));
        assert_json_error(&serde_json::to_vec(&nested_unknown)?);
    }

    let mut missing = configuration_value(
        "/var/lib/hyperintel/replay.sqlite3",
        "/var/lib/hyperintel/journal.sqlite3",
        4,
        4,
    );
    missing
        .as_object_mut()
        .ok_or("configuration is not an object")?
        .remove("profile");
    assert_json_error(&serde_json::to_vec(&missing)?);

    let mut floating = configuration_value(
        "/var/lib/hyperintel/replay.sqlite3",
        "/var/lib/hyperintel/journal.sqlite3",
        4,
        4,
    );
    floating["local_persistence"]["replay_database"]["max_connections"] = json!(1.5);
    assert_json_error(&serde_json::to_vec(&floating)?);

    let mut wrong_schema = configuration_value(
        "/var/lib/hyperintel/replay.sqlite3",
        "/var/lib/hyperintel/journal.sqlite3",
        4,
        4,
    );
    wrong_schema["schema_version"] = json!("hyperintel.runtime-configuration.v2");
    assert!(matches!(
        parse_runtime_configuration_v1(&serde_json::to_vec(&wrong_schema)?),
        Err(RuntimeConfigurationError::WrongSchemaVersion)
    ));

    let mut unsupported_profile = configuration_value(
        "/var/lib/hyperintel/replay.sqlite3",
        "/var/lib/hyperintel/journal.sqlite3",
        4,
        4,
    );
    unsupported_profile["profile"] = json!("PRODUCTION");
    assert_json_error(&serde_json::to_vec(&unsupported_profile)?);

    let mut noncanonical_id = configuration_value(
        "/var/lib/hyperintel/replay.sqlite3",
        "/var/lib/hyperintel/journal.sqlite3",
        4,
        4,
    );
    noncanonical_id["runtime_id"] = json!("Runtime Local");
    assert_json_error(&serde_json::to_vec(&noncanonical_id)?);
    Ok(())
}

#[test]
fn contract_database_paths_must_be_absolute_lexical_files() -> TestResult {
    for (path, reason) in [
        ("", DatabasePathRejectionV1::Empty),
        ("state/replay.sqlite3", DatabasePathRejectionV1::NotAbsolute),
        ("/", DatabasePathRejectionV1::DoesNotNameFile),
        (
            "/var/lib/hyperintel/",
            DatabasePathRejectionV1::DoesNotNameFile,
        ),
        (
            "/var/lib/./hyperintel/replay.sqlite3",
            DatabasePathRejectionV1::ContainsCurrentDirectory,
        ),
        (
            "/var/lib/../hyperintel/replay.sqlite3",
            DatabasePathRejectionV1::ContainsParentTraversal,
        ),
        (
            "/var/lib/hyperintel/replay\0.sqlite3",
            DatabasePathRejectionV1::ContainsNul,
        ),
    ] {
        let bytes = configuration_bytes(path, "/var/lib/hyperintel/journal.sqlite3", 4, 4)?;
        assert!(matches!(
            parse_runtime_configuration_v1(&bytes),
            Err(RuntimeConfigurationError::InvalidDatabasePath {
                reason: actual,
                ..
            }) if actual == reason
        ));
    }
    Ok(())
}

#[test]
fn contract_lexically_equal_database_paths_collide() -> TestResult {
    let bytes = configuration_bytes(
        "/var/lib/hyperintel//runtime.sqlite3",
        "/var/lib/hyperintel/runtime.sqlite3",
        4,
        4,
    )?;
    assert!(matches!(
        parse_runtime_configuration_v1(&bytes),
        Err(RuntimeConfigurationError::DatabasePathCollision)
    ));
    Ok(())
}

#[test]
fn contract_connection_counts_are_bounded() -> TestResult {
    for count in [1, 16] {
        let bytes = configuration_bytes(
            "/var/lib/hyperintel/replay.sqlite3",
            "/var/lib/hyperintel/journal.sqlite3",
            count,
            count,
        )?;
        assert!(parse_runtime_configuration_v1(&bytes).is_ok());
    }
    for count in [0, 17] {
        let bytes = configuration_bytes(
            "/var/lib/hyperintel/replay.sqlite3",
            "/var/lib/hyperintel/journal.sqlite3",
            count,
            4,
        )?;
        assert!(matches!(
            parse_runtime_configuration_v1(&bytes),
            Err(RuntimeConfigurationError::ConnectionCountOutOfRange { .. })
        ));
    }
    Ok(())
}

#[test]
fn contract_explicit_file_load_is_read_only_and_creates_no_database() -> TestResult {
    let directory = tempfile::tempdir()?;
    let config_path = directory.path().join("runtime.json");
    let replay_path = directory.path().join("replay.sqlite3");
    let journal_path = directory.path().join("journal.sqlite3");
    let bytes = configuration_bytes(
        replay_path.to_str().ok_or("replay path is not UTF-8")?,
        journal_path.to_str().ok_or("journal path is not UTF-8")?,
        4,
        4,
    )?;
    fs::write(&config_path, &bytes)?;
    let before = fs::read(&config_path)?;
    let validated = load_runtime_configuration_v1(&config_path)?;
    let after = fs::read(&config_path)?;

    assert_eq!(before, after);
    assert_eq!(validated.replay_database_path(), replay_path);
    assert_eq!(validated.execution_journal_database_path(), journal_path);
    assert!(!replay_path.exists());
    assert!(!journal_path.exists());
    Ok(())
}

#[test]
fn contract_explicit_file_rejects_max_plus_one_before_json_decoding() -> TestResult {
    let directory = tempfile::tempdir()?;
    let config_path = directory.path().join("oversized.json");
    fs::write(
        &config_path,
        vec![b' '; MAX_RUNTIME_CONFIGURATION_BYTES + 1],
    )?;
    assert!(matches!(
        load_runtime_configuration_v1(&config_path),
        Err(RuntimeConfigurationError::DocumentTooLarge { .. })
    ));
    Ok(())
}

#[test]
fn contract_secret_reference_is_opaque_versioned_and_round_trips() -> TestResult {
    let reference = SecretReferenceV1::new(
        "fixture".parse::<ProtocolId>()?,
        "provider-receipt-signer-alpha".parse::<ProtocolId>()?,
        Sha256Digest::from_bytes([0x11; 32]),
    )?;
    assert_eq!(reference.provider_id().as_str(), "fixture");
    assert_eq!(
        reference.handle_id().as_str(),
        "provider-receipt-signer-alpha"
    );
    assert_eq!(
        reference.version_digest(),
        Sha256Digest::from_bytes([0x11; 32])
    );
    let encoded = serde_json::to_vec(&reference)?;
    assert_eq!(
        serde_json::from_slice::<SecretReferenceV1>(&encoded)?,
        reference
    );
    Ok(())
}

#[test]
fn contract_secret_reference_serializes_only_nonsecret_reference_metadata() -> TestResult {
    let reference = SecretReferenceV1::new(
        "fixture".parse::<ProtocolId>()?,
        "provider-receipt-signer-alpha".parse::<ProtocolId>()?,
        Sha256Digest::from_bytes([0x11; 32]),
    )?;
    let encoded = serde_json::to_value(reference)?;
    let fields = encoded
        .as_object()
        .ok_or("secret reference is not an object")?;
    assert_eq!(fields.len(), 4);
    for expected in [
        "schema_version",
        "provider_id",
        "handle_id",
        "version_digest",
    ] {
        assert!(fields.contains_key(expected));
    }
    for forbidden in [
        "value",
        "secret",
        "token",
        "credential",
        "private_key",
        "material",
    ] {
        assert!(!fields.contains_key(forbidden));
    }
    Ok(())
}

#[test]
fn contract_secret_reference_rejects_wrong_schema_aliases_and_material() -> TestResult {
    let base = json!({
        "schema_version": SECRET_REFERENCE_SCHEMA_V1,
        "provider_id": "fixture",
        "handle_id": "provider-receipt-signer-alpha",
        "version_digest": "1111111111111111111111111111111111111111111111111111111111111111"
    });

    let mut wrong_schema = base.clone();
    wrong_schema["schema_version"] = json!("hyperintel.secret-reference.v2");
    assert!(serde_json::from_value::<SecretReferenceV1>(wrong_schema).is_err());

    for alias in [
        "latest",
        "current",
        "unversioned",
        "active",
        "default",
        "primary",
        "v1",
        "provider-specific-alias",
    ] {
        let mut document = base.clone();
        document["version_digest"] = json!(alias);
        assert!(serde_json::from_value::<SecretReferenceV1>(document).is_err());
    }
    assert_eq!(
        SecretReferenceV1::new(
            "fixture".parse::<ProtocolId>()?,
            "handle".parse::<ProtocolId>()?,
            Sha256Digest::from_bytes([0; 32]),
        ),
        Err(SecretReferenceError::ZeroVersionDigest)
    );

    for field in ["value", "token", "private_key", "material"] {
        let mut document = base.clone();
        document[field] = json!("forbidden");
        assert!(serde_json::from_value::<SecretReferenceV1>(document).is_err());
    }
    Ok(())
}

#[test]
fn contract_secret_reference_debug_redacts_handle_and_version() -> TestResult {
    let reference = SecretReferenceV1::new(
        "fixture".parse::<ProtocolId>()?,
        "sensitive-handle".parse::<ProtocolId>()?,
        Sha256Digest::from_bytes([0x22; 32]),
    )?;
    let debug = format!("{reference:?}");
    assert!(debug.contains("fixture"));
    assert!(!debug.contains("sensitive-handle"));
    assert!(!debug.contains(&"22".repeat(32)));
    Ok(())
}

fn configuration_value(
    replay_path: &str,
    journal_path: &str,
    replay_connections: u32,
    journal_connections: u32,
) -> serde_json::Value {
    json!({
        "schema_version": "hyperintel.runtime-configuration.v1",
        "runtime_id": "runtime:local-hermetic-1",
        "profile": "LOCAL_HERMETIC",
        "local_persistence": {
            "replay_database": {
                "path": replay_path,
                "max_connections": replay_connections
            },
            "execution_journal_database": {
                "path": journal_path,
                "max_connections": journal_connections
            }
        }
    })
}

fn configuration_bytes(
    replay_path: &str,
    journal_path: &str,
    replay_connections: u32,
    journal_connections: u32,
) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&configuration_value(
        replay_path,
        journal_path,
        replay_connections,
        journal_connections,
    ))
}

fn assert_json_error(bytes: &[u8]) {
    assert!(matches!(
        parse_runtime_configuration_v1(bytes),
        Err(RuntimeConfigurationError::Json(_))
    ));
}
