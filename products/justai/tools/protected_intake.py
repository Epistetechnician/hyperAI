#!/usr/bin/env python3
"""Fail-closed verifier for metadata-only externally controlled evidence handoffs."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCHEMA = "justai.protected_intake.v1"
STATE_SLICE = "justai-protected-intake-v1"
SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
FORBIDDEN_FIELDS = {
    "raw_payload",
    "raw_bytes",
    "private_locator",
    "secret",
    "credential",
    "restricted_payload",
}
BOUND_RECORD_SCHEMAS = {
    "evaluation_receipt": "justai.evaluation_receipt.v1",
    "disclosure_decision": "justai.disclosure_decision.v1",
    "claim_projection": "justai.claim_projection.v1",
    "review_receipt": "justai.review_receipt.v1",
    "reviewer_manifest": "justai.syndicate.v1",
}
INTAKE_ATTESTATION_ROLES = {"custodian", "authorizer", "witness"}


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
        raise ValueError("protected intake packet must be an object")
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


def require_object(value: dict[str, Any], key: str) -> dict[str, Any]:
    item = value.get(key)
    if not isinstance(item, dict):
        raise ValueError(f"{key} must be an object")
    return item


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


def require_string_list(value: dict[str, Any], key: str, *, nonempty: bool = False) -> list[str]:
    item = value.get(key)
    if not isinstance(item, list) or any(not isinstance(entry, str) or not entry for entry in item):
        raise ValueError(f"{key} must be a string array")
    if nonempty and not item:
        raise ValueError(f"{key} must not be empty")
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
                raise ValueError(f"protected intake contains forbidden disclosure field: {key}")
            reject_forbidden_fields(item)


def signature_payload(digest: str, purpose: str) -> bytes:
    return f"justai:{SCHEMA}:{digest}:{purpose}".encode("utf-8")


def verify_signature(reference: dict[str, Any], digest: str, expected_role: str, expected_purpose: str) -> str:
    role = require_string(reference, "role")
    if role != expected_role:
        raise ValueError(f"signature role must be {expected_role}")
    principal_id = require_string(reference, "principal_id")
    purpose = require_string(reference, "purpose")
    if purpose != expected_purpose:
        raise ValueError(f"signature purpose must be {expected_purpose}")
    public_key = require_hex(reference, "public_key")
    signature = require_hex(reference, "signature", 128)
    with tempfile.TemporaryDirectory(prefix="justai-protected-intake-") as directory:
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
        raise ValueError(f"signature verification failed for {principal_id}")
    return principal_id


def require_bound_record(
    bound_records: dict[str, Any],
    reference: str,
    digest: str,
    purpose: str,
) -> dict[str, Any]:
    record = bound_records.get(reference)
    if not isinstance(record, dict):
        raise ValueError(f"unresolved {purpose} reference: {reference}")
    if require_hex(record, "content_digest") != digest:
        raise ValueError(f"{purpose} digest does not match bound record: {reference}")
    return record


def validate_packet(packet: dict[str, Any]) -> dict[str, Any]:
    reject_forbidden_fields(packet)
    if require_string(packet, "schema") != SCHEMA:
        raise ValueError(f"schema must be {SCHEMA}")
    if require_string(packet, "state_slice") != STATE_SLICE:
        raise ValueError(f"state_slice must be {STATE_SLICE}")
    if require_string(packet, "status") != "Ready":
        raise ValueError("protected intake packet must have Ready status")
    require_string(packet, "record_id")
    require_hex(packet, "artifact_digest")
    require_string(packet, "evidence_basis")
    require_string(packet, "claim_ceiling")
    packet_digest = require_hex(packet, "content_digest")
    if content_digest(packet) != packet_digest:
        raise ValueError("protected intake content digest mismatch")

    identity_registry = require_object(packet, "identity_registry")
    if require_string(identity_registry, "control_mode") != "external":
        raise ValueError("identity registry must be externally controlled")
    require_string(identity_registry, "registry_ref")
    require_hex(identity_registry, "registry_digest")

    custody = require_object(packet, "custody")
    if require_string(custody, "control_mode") != "external":
        raise ValueError("custody must be externally controlled")
    if require_string(custody, "access_class") not in {"restricted", "protected"}:
        raise ValueError("protected intake requires restricted or protected access")
    require_string(custody, "custodian_id")
    if require_hex(custody, "artifact_digest") != packet["artifact_digest"]:
        raise ValueError("custody artifact digest does not match packet")
    require_hex(custody, "custody_receipt_digest")
    require_timestamp(custody, "received_at")
    require_string(custody, "retention_rule")

    authorization = require_object(packet, "authorization")
    require_string(authorization, "authority_id")
    require_hex(authorization, "authorization_digest")
    require_string(authorization, "scope")
    issued_at = require_timestamp(authorization, "issued_at")
    expires_at = require_timestamp(authorization, "expires_at")
    if expires_at <= issued_at:
        raise ValueError("authorization expires_at must be after issued_at")
    if expires_at <= datetime.now(timezone.utc):
        raise ValueError("authorization is expired")

    witnessed = require_object(packet, "witnessed_evaluation")
    evaluation_ref = require_string(witnessed, "evaluation_record_ref")
    evaluation_digest = require_hex(witnessed, "evaluation_record_digest")
    bound_records = require_object(packet, "bound_records")
    evaluation = require_bound_record(bound_records, evaluation_ref, evaluation_digest, "evaluation")
    if evaluation.get("schema") != BOUND_RECORD_SCHEMAS["evaluation_receipt"]:
        raise ValueError("evaluation reference has the wrong schema")
    if require_hex(witnessed, "artifact_digest") != packet["artifact_digest"]:
        raise ValueError("witnessed evaluation artifact digest does not match packet")
    for key in ("environment_digest", "runner_digest", "observed_result_digest", "witness_attestation_digest"):
        require_hex(witnessed, key)
    require_string(witnessed, "evaluation_scope")
    started_at = require_timestamp(witnessed, "started_at")
    completed_at = require_timestamp(witnessed, "completed_at")
    if completed_at < started_at:
        raise ValueError("witnessed evaluation completed_at precedes started_at")
    witness_ids = require_string_list(witnessed, "witness_ids", nonempty=True)

    manifest_attestations = packet.get("reviewer_manifest_attestations")
    if not isinstance(manifest_attestations, list) or len(manifest_attestations) < 2:
        raise ValueError("at least two signed reviewer manifest attestations are required")
    manifests: dict[str, dict[str, Any]] = {}
    for attestation in manifest_attestations:
        if not isinstance(attestation, dict):
            raise ValueError("reviewer manifest attestation must be an object")
        reference = require_string(attestation, "manifest_ref")
        digest = require_hex(attestation, "manifest_digest")
        organization = require_string(attestation, "reviewer_org")
        manifest = require_bound_record(bound_records, reference, digest, "reviewer manifest")
        if manifest.get("schema") != BOUND_RECORD_SCHEMAS["reviewer_manifest"]:
            raise ValueError(f"reviewer manifest has the wrong schema: {reference}")
        signature_ref = require_object(attestation, "signature_ref")
        signed_principal = verify_signature(signature_ref, digest, "reviewer-manifest", "syndicate-manifest")
        if signed_principal != organization:
            raise ValueError(f"reviewer manifest signature is not bound to {organization}")
        if reference in manifests:
            raise ValueError(f"duplicate reviewer manifest reference: {reference}")
        manifests[reference] = attestation
    if len({item["reviewer_org"] for item in manifests.values()}) < 2:
        raise ValueError("reviewer manifest attestations require two organizations")

    reviews = packet.get("independent_reviews")
    if not isinstance(reviews, list) or len(reviews) < 2:
        raise ValueError("at least two independent reviews are required")
    review_organizations: set[str] = set()
    for review in reviews:
        if not isinstance(review, dict):
            raise ValueError("independent review must be an object")
        review_ref = require_string(review, "review_receipt_ref")
        review_digest = require_hex(review, "review_receipt_digest")
        review_record = require_bound_record(bound_records, review_ref, review_digest, "review receipt")
        if review_record.get("schema") != BOUND_RECORD_SCHEMAS["review_receipt"]:
            raise ValueError(f"review receipt has the wrong schema: {review_ref}")
        organization = require_string(review, "reviewer_org")
        manifest_ref = require_string(review, "reviewer_manifest_ref")
        manifest_digest = require_hex(review, "reviewer_manifest_digest")
        manifest = manifests.get(manifest_ref)
        if manifest is None or manifest["manifest_digest"] != manifest_digest or manifest["reviewer_org"] != organization:
            raise ValueError(f"reviewer manifest is not bound for {review_ref}")
        if require_string(review, "result") != "accept":
            raise ValueError(f"independent review is not accepted: {review_ref}")
        if review.get("conflict_of_interest") is not False:
            raise ValueError(f"independent review has a conflict: {review_ref}")
        signature_ref = require_object(review, "signature_ref")
        signed_principal = verify_signature(signature_ref, review_digest, "reviewer", "review")
        if signed_principal != organization:
            raise ValueError(f"review signature is not bound to {organization}")
        review_organizations.add(organization)
    if len(review_organizations) < 2:
        raise ValueError("independent reviews require two organizations")

    disclosure = require_object(packet, "disclosure_decision")
    decision_ref = require_string(disclosure, "decision_record_ref")
    decision_digest = require_hex(disclosure, "decision_record_digest")
    decision_record = require_bound_record(bound_records, decision_ref, decision_digest, "disclosure decision")
    if decision_record.get("schema") != BOUND_RECORD_SCHEMAS["disclosure_decision"]:
        raise ValueError("disclosure decision reference has the wrong schema")
    decision = require_string(disclosure, "decision")
    if decision not in {"public", "aggregate_only", "withhold", "unknown"}:
        raise ValueError(f"unsupported disclosure decision: {decision}")
    require_string(disclosure, "reason")
    if decision in {"public", "aggregate_only"}:
        projection_ref = require_string(disclosure, "public_projection_ref")
        projection_digest = require_hex(disclosure, "public_projection_digest")
        projection = require_bound_record(bound_records, projection_ref, projection_digest, "public projection")
        if projection.get("schema") != BOUND_RECORD_SCHEMAS["claim_projection"]:
            raise ValueError("public projection reference has the wrong schema")

    attestations = packet.get("signature_refs")
    if not isinstance(attestations, list) or not attestations:
        raise ValueError("protected intake requires signed attestations")
    seen_roles: dict[str, list[str]] = {}
    for reference in attestations:
        if not isinstance(reference, dict):
            raise ValueError("protected intake signature reference must be an object")
        role = require_string(reference, "role")
        if role not in INTAKE_ATTESTATION_ROLES:
            raise ValueError(f"unsupported protected intake signature role: {role}")
        principal = verify_signature(reference, packet_digest, role, f"intake:{role}")
        seen_roles.setdefault(role, []).append(principal)
    if len(seen_roles.get("custodian", [])) != 1:
        raise ValueError("exactly one custodian attestation is required")
    if len(seen_roles.get("authorizer", [])) != 1:
        raise ValueError("exactly one authorizer attestation is required")
    if len(set(seen_roles.get("witness", []))) < 2:
        raise ValueError("two distinct witness attestations are required")
    if seen_roles["custodian"][0] != custody["custodian_id"]:
        raise ValueError("custody custodian_id is not bound to its signature")
    if seen_roles["authorizer"][0] != authorization["authority_id"]:
        raise ValueError("authorization authority_id is not bound to its signature")
    if set(seen_roles["witness"]) != set(witness_ids):
        raise ValueError("witness_ids are not bound to witness signatures")
    return packet


def publication_eligible(packet: dict[str, Any]) -> bool:
    return packet.get("status") == "Ready" and packet.get("disclosure_decision", {}).get("decision") in {"public", "aggregate_only"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("packet", type=Path, help="metadata-only protected intake packet")
    args = parser.parse_args()
    try:
        packet = validate_packet(read_packet(args.packet))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(f"valid protected intake packet: {packet['record_id']}")
    print(f"artifact_digest={packet['artifact_digest']}")
    print(f"content_digest={packet['content_digest']}")
    print(f"publication_eligible={str(publication_eligible(packet)).lower()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
