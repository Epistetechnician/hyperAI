#!/usr/bin/env python3
"""Bounded, read-only source normalizer for public JustAI evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


SOURCE_CLASSES = {"primary", "independent", "institutional", "community", "secondary"}


def canonical_json(value: object) -> bytes:
    """Serialize the v1 string/array/object subset deterministically."""
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def build_record(args: argparse.Namespace) -> dict[str, object]:
    source_path = Path(args.input).expanduser()
    if not source_path.is_file():
        raise ValueError("input must be an existing regular file")
    raw = source_path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) > args.max_bytes:
        raise ValueError(f"input exceeds bounded size of {args.max_bytes} bytes")
    record = {
        "schema": "justai.source.v1",
        "record_id": args.record_id,
        "state_slice": "justai-observatory-contracts-v1",
        "status": "Submitted",
        "claim_ceiling": "source-description-only",
        "created_at": args.accessed_at,
        "publisher_id": args.publisher_id,
        "source_digests": [digest],
        "content_digest": "",
        "signature_refs": [],
        "title": args.title,
        "url": args.url,
        "accessed_at": args.accessed_at,
        "publication_date": args.publication_date,
        "jurisdiction": args.jurisdiction,
        "source_class": args.source_class,
        "license_or_terms": args.license_or_terms,
        "access_class": "public",
        "custody_mode": "reproducible",
        "license_basis": args.license_or_terms,
        "custodian_id": args.publisher_id,
        "acquisition_timestamp": args.accessed_at,
        "permitted_uses": ["public-description", "bounded-evaluation"],
        "retention_and_disclosure_rules": "Retain locator and digest; do not mirror material without permission.",
        "snapshot_status": "available",
    }
    content = {key: value for key, value in record.items() if key not in {"content_digest", "signature_refs"}}
    record["content_digest"] = hashlib.sha256(canonical_json(content)).hexdigest()
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, help="local source snapshot; never fetched")
    parser.add_argument("--output", required=True, help="record JSON output path")
    parser.add_argument("--record-id", required=True)
    parser.add_argument("--publisher-id", required=True)
    parser.add_argument("--title", required=True)
    parser.add_argument("--url", required=True)
    parser.add_argument("--accessed-at", required=True)
    parser.add_argument("--publication-date", required=True)
    parser.add_argument("--jurisdiction", required=True)
    parser.add_argument("--source-class", choices=sorted(SOURCE_CLASSES), required=True)
    parser.add_argument("--license-or-terms", required=True)
    parser.add_argument("--max-bytes", type=int, default=5_000_000)
    args = parser.parse_args()
    try:
        record = build_record(args)
        Path(args.output).write_bytes(canonical_json(record) + b"\n")
    except (OSError, ValueError) as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
