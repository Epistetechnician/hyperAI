use justai_contracts::{
    assemble_release, canonical_bytes, parse_json_no_duplicates, parse_seed_hex, seal_record,
    sign_release, validate_release, validate_typed_record, verify_signatures,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn usage() {
    eprintln!(
        "Usage:\n  justai-contracts validate <release.json>\n  justai-contracts digest <record-or-release.json>\n  justai-contracts sign <input.json> <output.json> <key-id> <32-byte-seed-hex>\n  justai-contracts assemble <input-dir> <output-dir> <publisher-id> <created-at> <release-id> <key-id> <32-byte-seed-hex>"
    );
}

fn load_records(input_dir: &Path) -> Result<Vec<serde_json::Value>, String> {
    const GROUPS: &[&str] = &[
        "sources",
        "artifacts",
        "evaluations",
        "disclosures",
        "projections",
        "policy",
        "records",
        "reviews",
        "syndicates",
        "disputes",
    ];
    let mut paths = Vec::new();
    for group in GROUPS {
        let directory = input_dir.join(group);
        if !directory.exists() {
            continue;
        }
        if !directory.is_dir() {
            return Err(format!(
                "record group is not a directory: {}",
                directory.display()
            ));
        }
        for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                return Err(format!(
                    "symlinked record is not allowed: {}",
                    entry.path().display()
                ));
            }
            if file_type.is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            {
                paths.push(entry.path());
            }
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err("input directory contains no JSON records".into());
    }
    let mut records = Vec::with_capacity(paths.len());
    for path in paths {
        let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut value = parse_json_no_duplicates(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        seal_record(&mut value).map_err(|error| format!("{}: {error}", path.display()))?;
        validate_typed_record(&value).map_err(|error| format!("{}: {error}", path.display()))?;
        records.push(value);
    }
    Ok(records)
}

fn write_immutable(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        let existing = fs::read(path).map_err(|error| error.to_string())?;
        if existing != bytes {
            return Err(format!(
                "refusing to overwrite immutable artifact: {}",
                path.display()
            ));
        }
        return Ok(());
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(|| "missing command".to_string())?;
    match command.as_str() {
        "validate" => {
            let path = args
                .next()
                .ok_or_else(|| "missing release path".to_string())?;
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let value = parse_json_no_duplicates(&bytes).map_err(|error| error.to_string())?;
            let release = validate_release(&value, true).map_err(|error| error.to_string())?;
            verify_signatures(&value).map_err(|error| error.to_string())?;
            println!(
                "valid release {} with {} records",
                release.record_id,
                release.records.len()
            );
        }
        "digest" => {
            let path = args.next().ok_or_else(|| "missing JSON path".to_string())?;
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let value = parse_json_no_duplicates(&bytes).map_err(|error| error.to_string())?;
            let digest =
                justai_contracts::content_digest(&value).map_err(|error| error.to_string())?;
            println!("{digest}");
        }
        "sign" => {
            let input = args
                .next()
                .ok_or_else(|| "missing input path".to_string())?;
            let output = args
                .next()
                .ok_or_else(|| "missing output path".to_string())?;
            let key_id = args.next().ok_or_else(|| "missing key id".to_string())?;
            let seed = args.next().ok_or_else(|| "missing seed".to_string())?;
            let bytes = fs::read(&input).map_err(|error| error.to_string())?;
            let mut value = parse_json_no_duplicates(&bytes).map_err(|error| error.to_string())?;
            let signing_key = parse_seed_hex(&seed).map_err(|error| error.to_string())?;
            sign_release(&mut value, &key_id, &signing_key).map_err(|error| error.to_string())?;
            let output_bytes =
                justai_contracts::canonical_bytes(&value).map_err(|error| error.to_string())?;
            fs::write(&output, output_bytes).map_err(|error| error.to_string())?;
            println!("signed release written to {output}");
        }
        "assemble" => {
            let input_dir = PathBuf::from(
                args.next()
                    .ok_or_else(|| "missing input directory".to_string())?,
            );
            let output_dir = PathBuf::from(
                args.next()
                    .ok_or_else(|| "missing output directory".to_string())?,
            );
            let publisher_id = args
                .next()
                .ok_or_else(|| "missing publisher id".to_string())?;
            let created_at = args
                .next()
                .ok_or_else(|| "missing created_at".to_string())?;
            let release_id = args
                .next()
                .ok_or_else(|| "missing release id".to_string())?;
            let key_id = args.next().ok_or_else(|| "missing key id".to_string())?;
            let seed = args.next().ok_or_else(|| "missing seed".to_string())?;
            let records = load_records(&input_dir)?;
            let mut release = assemble_release(records, &publisher_id, &created_at, &release_id)
                .map_err(|error| error.to_string())?;
            let signing_key = parse_seed_hex(&seed).map_err(|error| error.to_string())?;
            sign_release(&mut release, &key_id, &signing_key).map_err(|error| error.to_string())?;
            validate_release(&release, true).map_err(|error| error.to_string())?;
            verify_signatures(&release).map_err(|error| error.to_string())?;
            let digest = release
                .get("content_digest")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "assembled release has no content digest".to_string())?;
            let bundle_dir = output_dir.join(digest);
            fs::create_dir_all(&bundle_dir).map_err(|error| error.to_string())?;
            let release_path = bundle_dir.join("release.json");
            let bytes = canonical_bytes(&release).map_err(|error| error.to_string())?;
            write_immutable(&release_path, &bytes)?;
            println!("assembled signed bundle {}", release_path.display());
        }
        _ => return Err(format!("unknown command: {command}")),
    }
    Ok(())
}

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        usage();
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
