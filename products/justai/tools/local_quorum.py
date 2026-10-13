#!/usr/bin/env python3
"""Verify repeated local execution without treating it as independent review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCHEMA = "justai.local_quorum.v1"
STATE_SLICE = "justai-local-quorum-v1"
CLAIM_CEILING = "local-replay"
SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
FORBIDDEN_FIELDS = {
    "raw_payload",
    "raw_bytes",
    "private_locator",
    "secret",
    "credential",
    "restricted_payload",
}
ALLOWED_PACKET_FIELDS = {
    "schema",
    "record_id",
    "state_slice",
    "status",
    "verification_mode",
    "claim_ceiling",
    "artifact_digest",
    "evaluation_scope",
    "expected_result_digest",
    "content_digest",
    "signature_refs",
    "instances",
}
ALLOWED_INSTANCE_FIELDS = {
    "instance_id",
    "machine_digest",
    "environment_digest",
    "runner_digest",
    "observed_result_digest",
    "output_digest",
    "status",
    "started_at",
    "completed_at",
}
ALLOWED_SIGNATURE_FIELDS = {
    "role",
    "instance_id",
    "principal_id",
    "key_id",
    "purpose",
    "public_key",
    "signature",
}


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def read_packet(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    if not isinstance(value, dict):
        raise ValueError("local quorum packet must be an object")
    return value


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def content_digest(value: dict[str, Any]) -> str:
    content = {key: item for key, item in value.items() if key not in {"content_digest", "signature_refs"}}
    return hashlib.sha256(canonical_json(content)).hexdigest()


def require_string(value: dict[str, Any], key: str) -> str:
    item = value.get(key)
    if not isinstance(item, str) or not item:
        raise ValueError(f"{key} must be a non-empty string")
    return item


def require_hex(value: dict[str, Any], key: str, length: int = 64) -> str:
    item = require_string(value, key)
    if len(item) != length or any(character not in "0123456789abcdef" for character in item):
        raise ValueError(f"{key} must be lowercase hexadecimal")
    return item


def require_timestamp(value: dict[str, Any], key: str) -> datetime:
    raw = require_string(value, key)
    try:
        parsed = datetime.fromisoformat(raw.replace("Z", "+00:00"))
    except ValueError as error:
        raise ValueError(f"{key} must be an RFC 3339 timestamp") from error
    if parsed.tzinfo is None:
        raise ValueError(f"{key} must include a timezone")
    return parsed.astimezone(timezone.utc)


def reject_forbidden_fields(value: Any) -> None:
    if isinstance(value, list):
        for item in value:
            reject_forbidden_fields(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            if key in FORBIDDEN_FIELDS:
                raise ValueError(f"local quorum contains forbidden disclosure field: {key}")
            reject_forbidden_fields(item)


def signature_payload(digest: str, purpose: str) -> bytes:
    return f"justai:{SCHEMA}:{digest}:{purpose}".encode("utf-8")


def verify_signature(reference: dict[str, Any], digest: str, instance_id: str) -> str:
    unknown_fields = set(reference) - ALLOWED_SIGNATURE_FIELDS
    if unknown_fields:
        raise ValueError(f"unsupported local quorum signature fields: {', '.join(sorted(unknown_fields))}")
    if require_string(reference, "role") != "instance":
        raise ValueError("local quorum signature role must be instance")
    if require_string(reference, "instance_id") != instance_id:
        raise ValueError(f"signature is not bound to instance {instance_id}")
    principal_id = require_string(reference, "principal_id")
    require_string(reference, "key_id")
    purpose = require_string(reference, "purpose")
    expected_purpose = f"instance:{instance_id}"
    if purpose != expected_purpose:
        raise ValueError(f"signature purpose must be {expected_purpose}")
    public_key = require_hex(reference, "public_key")
    signature = require_hex(reference, "signature", 128)
    with tempfile.TemporaryDirectory(prefix="justai-local-quorum-") as directory:
        directory_path = Path(directory)
        public_key_path = directory_path / "public.der"
        signature_path = directory_path / "signature.bin"
        payload_path = directory_path / "payload.bin"
        public_key_path.write_bytes(SPKI_PREFIX + bytes.fromhex(public_key))
        signature_path.write_bytes(bytes.fromhex(signature))
        payload_path.write_bytes(signature_payload(digest, purpose))
        result = subprocess.run(
            [
                "openssl",
                "pkeyutl",
                "-verify",
                "-rawin",
                "-pubin",
                "-inkey",
                str(public_key_path),
                "-keyform",
                "DER",
                "-sigfile",
                str(signature_path),
                "-in",
                str(payload_path),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    if result.returncode != 0:
        raise ValueError(f"signature verification failed for instance {instance_id}")
    return principal_id


def validate_packet(packet: dict[str, Any]) -> dict[str, Any]:
    reject_forbidden_fields(packet)
    unknown_fields = set(packet) - ALLOWED_PACKET_FIELDS
    if unknown_fields:
        raise ValueError(f"unsupported local quorum fields: {', '.join(sorted(unknown_fields))}")
    if require_string(packet, "schema") != SCHEMA:
        raise ValueError(f"schema must be {SCHEMA}")
    if require_string(packet, "state_slice") != STATE_SLICE:
        raise ValueError(f"state_slice must be {STATE_SLICE}")
    if require_string(packet, "status") != "Verified":
        raise ValueError("local quorum packet must have Verified status")
    if require_string(packet, "verification_mode") != "local-quorum":
        raise ValueError("verification_mode must be local-quorum")
    if require_string(packet, "claim_ceiling") != CLAIM_CEILING:
        raise ValueError("local quorum claim ceiling must be local-replay")
    require_string(packet, "record_id")
    require_hex(packet, "artifact_digest")
    require_string(packet, "evaluation_scope")
    expected_result_digest = require_hex(packet, "expected_result_digest")
    packet_digest = require_hex(packet, "content_digest")
    if content_digest(packet) != packet_digest:
        raise ValueError("local quorum content digest mismatch")

    instances = packet.get("instances")
    if not isinstance(instances, list) or len(instances) < 3:
        raise ValueError("local quorum requires at least three instances")
    instance_ids: set[str] = set()
    machine_digests: set[str] = set()
    instance_digests: dict[str, dict[str, Any]] = {}
    for instance in instances:
        if not isinstance(instance, dict):
            raise ValueError("local quorum instance must be an object")
        unknown_fields = set(instance) - ALLOWED_INSTANCE_FIELDS
        if unknown_fields:
            raise ValueError(f"unsupported local quorum instance fields: {', '.join(sorted(unknown_fields))}")
        instance_id = require_string(instance, "instance_id")
        if instance_id in instance_ids:
            raise ValueError(f"duplicate local quorum instance: {instance_id}")
        instance_ids.add(instance_id)
        machine_digests.add(require_hex(instance, "machine_digest"))
        require_hex(instance, "environment_digest")
        require_hex(instance, "runner_digest")
        if require_hex(instance, "observed_result_digest") != expected_result_digest:
            raise ValueError(f"instance {instance_id} observed result does not match expected digest")
        require_hex(instance, "output_digest")
        if require_string(instance, "status") != "pass":
            raise ValueError(f"instance {instance_id} did not pass")
        started_at = require_timestamp(instance, "started_at")
        completed_at = require_timestamp(instance, "completed_at")
        if completed_at < started_at:
            raise ValueError(f"instance {instance_id} completed_at precedes started_at")
        instance_digests[instance_id] = instance
    if len(machine_digests) < 2:
        raise ValueError("local quorum requires two distinct machine digests")

    signatures = packet.get("signature_refs")
    if not isinstance(signatures, list) or len(signatures) != len(instances):
        raise ValueError("one signed instance attestation is required per instance")
    signatures_by_instance: dict[str, str] = {}
    for reference in signatures:
        if not isinstance(reference, dict):
            raise ValueError("local quorum signature reference must be an object")
        instance_id = require_string(reference, "instance_id")
        if instance_id in signatures_by_instance:
            raise ValueError(f"duplicate local quorum signature: {instance_id}")
        principal_id = verify_signature(reference, packet_digest, instance_id)
        signatures_by_instance[instance_id] = principal_id
    if set(signatures_by_instance) != instance_ids:
        raise ValueError("signed instance attestations do not cover every instance")
    return packet


def publication_eligible(packet: dict[str, Any]) -> bool:
    del packet
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("packet", type=Path, help="metadata-only local quorum packet")
    args = parser.parse_args()
    try:
        packet = validate_packet(read_packet(args.packet))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(f"valid local quorum packet: {packet['record_id']}")
    print(f"artifact_digest={packet['artifact_digest']}")
    print(f"content_digest={packet['content_digest']}")
    print("claim_ceiling=local-replay")
    print("publication_eligible=false")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
