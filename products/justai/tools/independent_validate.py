#!/usr/bin/env python3
"""Independent release validator using Python and the system OpenSSL binary."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any


SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
KNOWN_RECORD_SCHEMAS = {
    "justai.decision.v1",
    "justai.serendipity.v1",
    "justai.intuition_review.v1",
    "justai.dissent.v1",
    "justai.promotion_decision.v1",
    "justai.unknown.v1",
    "justai.evidence_manifest.v1",
    "justai.integration_result.v1",
    "justai.integration_handoff_seal.v1",
    "justai.source.v1",
    "justai.artifact_manifest.v1",
    "justai.evaluation_receipt.v1",
    "justai.disclosure_decision.v1",
    "justai.claim_projection.v1",
    "justai.policy_trace.v1",
    "justai.claim.v1",
    "justai.incident.v1",
    "justai.commitment.v1",
    "justai.review_receipt.v1",
    "justai.dispute.v1",
    "justai.syndicate.v1",
}

ACCESS_CLASSES = {"public", "restricted", "protected"}
CUSTODY_MODES = {"reproducible", "attested", "corroborated", "unknown"}
FORBIDDEN_DISCLOSURE_FIELDS = {
    "raw_payload",
    "raw_bytes",
    "private_locator",
    "secret",
    "credential",
    "restricted_payload",
}
GOVERNED_DISPOSITIONS = {
    "proposed", "sampled", "reviewed", "dissent", "retrospective",
    "promote", "unknown", "quarantine",
}
GOVERNED_STATE_SLICES = {
    "justai-governed-discovery-monorepo-v1", "justai-contracts-v1",
    "justai-discovery-v1", "justai-continual-learning-v1", "justai-custody-v1",
    "justai-platform-v1", "justai-operations-v1", "justai-infrastructure-v1",
}
GOVERNED_CLAIM_CEILINGS = {
    "source-description-only", "bounded-description-only", "locator-only-development-pilot",
    "reviewer-identity-only", "identity-and-role-description-only", "local-proposal",
    "local-development-only", "bounded-public-observatory", "local-replay",
    "engineering-evidence", "independent-review-pending", "scientific-evidence",
    "production-authorized",
}
GOVERNED_CLAIM_CEILING_RANKS = {
    "source-description-only": 0,
    "bounded-description-only": 0,
    "locator-only-development-pilot": 0,
    "reviewer-identity-only": 0,
    "identity-and-role-description-only": 0,
    "local-proposal": 1,
    "local-development-only": 1,
    "bounded-public-observatory": 1,
    "local-replay": 2,
    "engineering-evidence": 3,
    "independent-review-pending": 4,
    "scientific-evidence": 5,
    "production-authorized": 6,
}
GOVERNED_SCHEMAS = {
    "justai.decision.v1",
    "justai.serendipity.v1",
    "justai.intuition_review.v1",
    "justai.dissent.v1",
    "justai.promotion_decision.v1",
    "justai.unknown.v1",
    "justai.evidence_manifest.v1",
    "justai.integration_result.v1",
    "justai.integration_handoff_seal.v1",
}
GOVERNED_COMMON_FIELDS = {
    "schema", "record_id", "created_at", "state_slice", "proposal_digest",
    "claim_ceiling", "provenance", "disposition", "evidence_refs", "content_digest",
}
GOVERNED_FIELDS = {
    "justai.decision.v1": {"algorithmic_proposal", "model_role", "reversible", "confidence", "alternative_refs", "requested_action"},
    "justai.serendipity.v1": {"sampling_method", "candidate_refs", "preserved_alternatives", "candidate_digests", "budget", "selected_candidate_ref"},
    "justai.intuition_review.v1": {"hypothesis", "confidence", "observed_evidence", "rejected_alternatives", "recommended_action_or_restraint", "reviewer_identity", "later_outcome", "intuition_added_value", "human_disposition"},
    "justai.dissent.v1": {"dissenting_reviewer", "reason", "alternatives_preserved", "resolution", "resolution_evidence_refs"},
    "justai.promotion_decision.v1": {"decision", "reason", "validator_identity", "validator_independence", "human_disposition_ref"},
    "justai.unknown.v1": {"unknown_reason", "missing_evidence", "next_review_condition", "preserved_alternatives", "owner"},
    "justai.evidence_manifest.v1": {"artifact_refs", "source_commit_refs", "custody_root_ref", "integrity_status", "excluded_artifacts"},
    "justai.integration_result.v1": {"phase_policy", "math", "linguistics", "evidence", "observed_ids", "required_digest_refs", "nonclaims", "external_seal_ref"},
    "justai.integration_handoff_seal.v1": {"result_digest", "sealed_payload_digest", "external_reviewer_id", "external_public_key", "signature"},
}
REQUIRED_NONCLAIMS = {
    "not independent review", "not scientific evidence", "not production readiness",
    "not authorization", "not profitability", "not hosted enforcement",
}
def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    if not isinstance(value, dict):
        raise ValueError("top-level JSON value must be an object")
    return value


def canonical_json(value: Any) -> bytes:
    def ensure_supported(item: Any) -> None:
        if isinstance(item, float):
            raise ValueError("floating-point JSON is not supported by the cross-language digest contract")
        if isinstance(item, dict):
            for key, nested in item.items():
                if not isinstance(key, str) or not key.isascii():
                    raise ValueError("JSON object keys must be ASCII for the cross-language digest contract")
                ensure_supported(nested)
        elif isinstance(item, list):
            for nested in item:
                ensure_supported(nested)

    ensure_supported(value)
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


def reject_disclosure_fields(value: Any) -> None:
    if isinstance(value, list):
        for item in value:
            reject_disclosure_fields(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            if key in FORBIDDEN_DISCLOSURE_FIELDS:
                raise ValueError(f"public release contains forbidden disclosure field: {key}")
            reject_disclosure_fields(item)


def require_string(value: dict[str, Any], key: str) -> str:
    item = value.get(key)
    if not isinstance(item, str) or not item:
        raise ValueError(f"{key} must be a non-empty string")
    return item


def require_list(value: dict[str, Any], key: str, *, nonempty: bool = False) -> list[Any]:
    item = value.get(key)
    if not isinstance(item, list) or any(not isinstance(entry, str) or not entry for entry in item):
        raise ValueError(f"{key} must be a string array")
    if nonempty and not item:
        raise ValueError(f"{key} must not be empty")
    return item


def require_custody_metadata(record: dict[str, Any]) -> None:
    access_class = require_string(record, "access_class")
    if access_class not in ACCESS_CLASSES:
        raise ValueError(f"unsupported access_class: {access_class}")
    custody_mode = require_string(record, "custody_mode")
    if custody_mode not in CUSTODY_MODES:
        raise ValueError(f"unsupported custody_mode: {custody_mode}")
    for key in ("license_basis", "custodian_id", "acquisition_timestamp", "retention_and_disclosure_rules"):
        require_string(record, key)
    require_list(record, "permitted_uses", nonempty=True)


def require_hex(value: str, key: str, length: int) -> None:
    if len(value) != length or any(character not in "0123456789abcdef" for character in value):
        raise ValueError(f"{key} must be lowercase hexadecimal")


def require_git_commit(value: str, key: str) -> None:
    if len(value) != 40 or any(character not in "0123456789abcdef" for character in value):
        raise ValueError(f"{key} must be a lowercase 40-character Git commit")


def require_bool(value: dict[str, Any], key: str) -> bool:
    item = value.get(key)
    if type(item) is not bool:
        raise ValueError(f"{key} must be a boolean")
    return item


def require_int_range(value: dict[str, Any], key: str, minimum: int, maximum: int) -> int:
    item = value.get(key)
    if type(item) is not int or not minimum <= item <= maximum:
        raise ValueError(f"{key} must be an integer from {minimum} through {maximum}")
    return item


def optional_string(value: dict[str, Any], key: str) -> None:
    item = value.get(key)
    if item is not None and (not isinstance(item, str) or not item):
        raise ValueError(f"{key} must be a non-empty string or null")


def validate_governed_record(record: dict[str, Any]) -> None:
    schema = require_string(record, "schema")
    if schema not in GOVERNED_SCHEMAS:
        raise ValueError(f"unsupported governed record schema: {schema}")
    for key in ("record_id", "created_at", "state_slice", "proposal_digest", "claim_ceiling", "disposition", "content_digest"):
        require_string(record, key)
    allowed = governed_allowed_fields(schema)
    missing = allowed - set(record)
    if missing:
        raise ValueError(f"{schema} requires fields: {', '.join(sorted(missing))}")
    unexpected = set(record) - allowed
    if unexpected:
        raise ValueError(f"unknown governed record fields: {', '.join(sorted(unexpected))}")
    if record["state_slice"] not in GOVERNED_STATE_SLICES:
        raise ValueError("governed state_slice must be an exact JustAI slice")
    require_hex(record["proposal_digest"], "proposal_digest", 64)
    require_hex(record["content_digest"], "content_digest", 64)
    if record["claim_ceiling"] not in GOVERNED_CLAIM_CEILINGS:
        raise ValueError(f"unknown claim ceiling: {record['claim_ceiling']}")
    if GOVERNED_CLAIM_CEILING_RANKS[record["claim_ceiling"]] > GOVERNED_CLAIM_CEILING_RANKS["local-replay"]:
        raise ValueError("local governed validation cannot emit a claim above local-replay")
    provenance = record.get("provenance")
    if not isinstance(provenance, list) or not provenance:
        raise ValueError("provenance must be a non-empty array")
    for entry in provenance:
        if not isinstance(entry, dict):
            raise ValueError("provenance entries must be objects")
        for key in ("source_repository", "source_commit", "source_path", "license_basis", "custody_ref"):
            require_string(entry, key)
        if entry["source_path"].startswith("/") or ".." in entry["source_path"]:
            raise ValueError("provenance source_path must be repository-relative")
        require_git_commit(entry["source_commit"], "source_commit")
        require_hex(require_string(entry, "source_digest"), "source_digest", 64)
    require_list(record, "evidence_refs", nonempty=True)
    if record["disposition"] not in GOVERNED_DISPOSITIONS:
        raise ValueError(f"unsupported governed disposition: {record['disposition']}")
    if content_digest(record) != record["content_digest"]:
        raise ValueError(f"content digest mismatch for {record['record_id']}")
    if schema == "justai.decision.v1":
        require_string(record, "algorithmic_proposal")
        if require_string(record, "model_role") != "proposal-only":
            raise ValueError("model_role must be proposal-only")
        require_bool(record, "reversible")
        require_int_range(record, "confidence", 0, 100)
        require_list(record, "alternative_refs", nonempty=True)
        require_string(record, "requested_action")
    elif schema == "justai.serendipity.v1":
        for key in ("sampling_method", "budget"):
            require_string(record, key)
        candidate_refs = require_list(record, "candidate_refs", nonempty=True)
        require_list(record, "preserved_alternatives", nonempty=True)
        candidate_digests = require_list(record, "candidate_digests", nonempty=True)
        if len(candidate_refs) != len(candidate_digests):
            raise ValueError("candidate_digests must bind every candidate reference")
        for item in candidate_digests:
            require_hex(item, "candidate_digest", 64)
        optional_string(record, "selected_candidate_ref")
    elif schema == "justai.intuition_review.v1":
        for key in ("hypothesis", "recommended_action_or_restraint", "reviewer_identity"):
            require_string(record, key)
        require_int_range(record, "confidence", 0, 100)
        require_list(record, "observed_evidence", nonempty=True)
        require_list(record, "rejected_alternatives", nonempty=True)
        optional_string(record, "later_outcome")
        if record.get("intuition_added_value") is not None:
            require_bool(record, "intuition_added_value")
        optional_string(record, "human_disposition")
        if record.get("human_disposition") not in {None, "promote", "unknown", "quarantine"}:
            raise ValueError("unsupported human disposition")
    elif schema == "justai.dissent.v1":
        for key in ("dissenting_reviewer", "reason"):
            require_string(record, key)
        require_list(record, "alternatives_preserved", nonempty=True)
        optional_string(record, "resolution")
        require_list(record, "resolution_evidence_refs")
    elif schema == "justai.promotion_decision.v1":
        decision = require_string(record, "decision")
        if decision not in {"promote", "unknown", "quarantine"} or decision != record["disposition"]:
            raise ValueError("promotion decision and disposition must agree")
        for key in ("reason", "validator_identity", "validator_independence"):
            require_string(record, key)
        optional_string(record, "human_disposition_ref")
    elif schema == "justai.unknown.v1":
        if record["disposition"] != "unknown":
            raise ValueError("UnknownRecord disposition must be unknown")
        for key in ("unknown_reason", "next_review_condition", "owner"):
            require_string(record, key)
        require_list(record, "missing_evidence", nonempty=True)
        require_list(record, "preserved_alternatives", nonempty=True)
    elif schema == "justai.evidence_manifest.v1":
        require_list(record, "artifact_refs", nonempty=True)
        require_list(record, "source_commit_refs", nonempty=True)
        for key in ("custody_root_ref", "integrity_status"):
            require_string(record, key)
        require_list(record, "excluded_artifacts")
    elif schema == "justai.integration_result.v1":
        for key in ("phase_policy", "math", "linguistics", "evidence"):
            require_string(record, key)
        require_list(record, "observed_ids", nonempty=True)
        required_digest_refs = require_list(record, "required_digest_refs", nonempty=True)
        for item in required_digest_refs:
            require_hex(item, "required_digest_ref", 64)
        nonclaims = require_list(record, "nonclaims", nonempty=True)
        if set(nonclaims) != REQUIRED_NONCLAIMS:
            raise ValueError("integration result must preserve the mandatory nonclaims")
        optional_string(record, "external_seal_ref")
    elif schema == "justai.integration_handoff_seal.v1":
        for key in ("external_reviewer_id",):
            require_string(record, key)
        require_hex(record["result_digest"], "result_digest", 64)
        require_hex(record["sealed_payload_digest"], "sealed_payload_digest", 64)
        require_hex(record["external_public_key"], "external_public_key", 64)
        require_hex(record["signature"], "signature", 128)
    return


def governed_allowed_fields(schema: str) -> set[str]:
    return GOVERNED_COMMON_FIELDS | GOVERNED_FIELDS[schema]


def validate_record(record: dict[str, Any]) -> None:
    schema = require_string(record, "schema")
    if schema not in KNOWN_RECORD_SCHEMAS:
        raise ValueError(f"unsupported record schema: {schema}")
    if schema in GOVERNED_SCHEMAS:
        validate_governed_record(record)
        return
    for key in ("record_id", "state_slice", "status", "claim_ceiling", "created_at", "publisher_id"):
        require_string(record, key)
    source_digests = require_list(record, "source_digests")
    for digest in source_digests:
        require_hex(digest, "source_digest", 64)
    digest = require_string(record, "content_digest")
    require_hex(digest, "content_digest", 64)
    if content_digest(record) != digest:
        raise ValueError(f"content digest mismatch for {record['record_id']}")
    if not isinstance(record.get("signature_refs"), list):
        raise ValueError("signature_refs must be an array")
    if schema in {
        "justai.artifact_manifest.v1", "justai.evaluation_receipt.v1", "justai.disclosure_decision.v1",
        "justai.claim_projection.v1", "justai.policy_trace.v1",
    } and record["state_slice"] != "justai-evidence-bridge-v1":
        raise ValueError(f"bridge record {record['record_id']} has the wrong state slice")
    if schema == "justai.source.v1":
        for key in (
            "title", "url", "accessed_at", "publication_date", "jurisdiction",
            "source_class", "license_or_terms", "snapshot_status",
        ):
            require_string(record, key)
        require_custody_metadata(record)
    elif schema == "justai.artifact_manifest.v1":
        if require_string(record, "artifact_kind") not in {"code", "model", "dataset", "document", "trace", "testimony"}:
            raise ValueError("unsupported artifact_kind")
        require_custody_metadata(record)
        if record["custody_mode"] == "unknown" and record["status"] not in {"Unknown", "NoCandidate"}:
            raise ValueError("unknown custody requires Unknown or NoCandidate status")
        require_hex(require_string(record, "artifact_digest"), "artifact_digest", 64)
        require_string(record, "evidence_basis")
    elif schema == "justai.evaluation_receipt.v1":
        require_list(record, "artifact_manifest_refs", nonempty=True)
        for key in ("authorization_ref", "evaluation_scope", "observed_result_summary", "evidence_basis", "uncertainty"):
            require_string(record, key)
        for key in ("environment_digest", "runner_digest", "observed_result_digest"):
            require_hex(require_string(record, key), key, 64)
        limits = record.get("resource_limits")
        if not isinstance(limits, dict) or not isinstance(limits.get("cpu_seconds"), int) or not isinstance(limits.get("memory_bytes"), int):
            raise ValueError("resource_limits must contain integer cpu_seconds and memory_bytes")
        require_string(limits, "network_policy")
        require_list(record, "nonclaims", nonempty=True)
        require_list(record, "review_receipt_refs")
        require_list(record, "disclosure_decision_refs")
    elif schema == "justai.disclosure_decision.v1":
        require_list(record, "subject_refs", nonempty=True)
        if require_string(record, "decision") not in {"public", "aggregate_only", "withhold", "unknown"}:
            raise ValueError("unsupported disclosure decision")
        for key in ("reason", "public_summary", "withheld_material", "missing_information", "contestation"):
            require_string(record, key)
        require_list(record, "review_receipt_refs")
    elif schema == "justai.claim_projection.v1":
        require_list(record, "evidence_refs", nonempty=True)
        for key in ("public_summary", "uncertainty", "missing_information", "contestation"):
            require_string(record, key)
        require_list(record, "nonclaims", nonempty=True)
        require_list(record, "disclosure_decision_refs", nonempty=True)
        require_list(record, "review_receipt_refs", nonempty=True)
    elif schema == "justai.policy_trace.v1":
        for key in ("artifact_refs", "evidence_refs", "evaluation_receipt_refs", "review_receipt_refs", "claim_projection_refs"):
            require_list(record, key, nonempty=True)
        for key in ("demand", "lawful_target", "response"):
            require_string(record, key)
    elif schema == "justai.review_receipt.v1":
        for key in ("reviewer_id", "reviewer_org", "reviewed_record_id", "syndicate_manifest_ref", "reviewed_digest", "result", "independence_basis"):
            require_string(record, key)
        require_hex(record["reviewed_digest"], "reviewed_digest", 64)
        if record["result"] not in {"accept", "needs_revision", "reject"}:
            raise ValueError(f"unsupported review result: {record['result']}")
        if not isinstance(record.get("conflict_of_interest"), bool):
            raise ValueError("conflict_of_interest must be boolean")
    elif schema == "justai.dispute.v1":
        for key in ("challenger_id", "disputed_record_id", "basis", "affected_party_response", "resolution", "contestation"):
            require_string(record, key)
    elif schema == "justai.syndicate.v1":
        for key in ("syndicate_id", "syndicate_name", "jurisdiction", "charter_version", "funding_disclosure", "conflict_disclosure"):
            require_string(record, key)
        require_list(record, "roles", nonempty=True)
        require_hex(require_string(record, "public_key"), "public_key", 64)


def verify_signatures(release: dict[str, Any]) -> None:
    for reference in release["signature_refs"]:
        if not isinstance(reference, dict):
            raise ValueError("signature reference must be an object")
        purpose = require_string(reference, "purpose")
        if purpose != "release":
            raise ValueError(f"unsupported signature purpose: {purpose}")
        key_id = require_string(reference, "key_id")
        public_key = require_string(reference, "public_key")
        signature = require_string(reference, "signature")
        require_hex(public_key, "public_key", 64)
        require_hex(signature, "signature", 128)
        payload = f"justai:{release['schema']}:{release['content_digest']}:{purpose}".encode("utf-8")
        with tempfile.TemporaryDirectory(prefix="justai-independent-") as directory:
            directory_path = Path(directory)
            key_path = directory_path / "public.der"
            signature_path = directory_path / "signature.bin"
            payload_path = directory_path / "payload.bin"
            key_path.write_bytes(SPKI_PREFIX + bytes.fromhex(public_key))
            signature_path.write_bytes(bytes.fromhex(signature))
            payload_path.write_bytes(payload)
            result = subprocess.run(
                [
                    "openssl",
                    "pkeyutl",
                    "-verify",
                    "-rawin",
                    "-pubin",
                    "-inkey",
                    str(key_path),
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
            raise ValueError(f"signature verification failed for {key_id}")


def verify_record_signatures(record: dict[str, Any]) -> None:
    signatures = record.get("signature_refs")
    if not isinstance(signatures, list) or not signatures:
        raise ValueError(f"record {record['record_id']} requires a signature")
    for reference in signatures:
        purpose = require_string(reference, "purpose")
        public_key = require_string(reference, "public_key")
        signature = require_string(reference, "signature")
        key_id = require_string(reference, "key_id")
        require_hex(public_key, "public_key", 64)
        require_hex(signature, "signature", 128)
        payload = f"justai:{record['schema']}:{record['content_digest']}:{purpose}".encode("utf-8")
        with tempfile.TemporaryDirectory(prefix="justai-independent-record-") as directory:
            directory_path = Path(directory)
            key_path = directory_path / "public.der"
            signature_path = directory_path / "signature.bin"
            payload_path = directory_path / "payload.bin"
            key_path.write_bytes(SPKI_PREFIX + bytes.fromhex(public_key))
            signature_path.write_bytes(bytes.fromhex(signature))
            payload_path.write_bytes(payload)
            result = subprocess.run(
                ["openssl", "pkeyutl", "-verify", "-rawin", "-pubin", "-inkey", str(key_path),
                 "-keyform", "DER", "-sigfile", str(signature_path), "-in", str(payload_path)],
                capture_output=True, text=True, check=False,
            )
        if result.returncode != 0:
            raise ValueError(f"signature verification failed for {key_id}")


def validate_release(release: dict[str, Any]) -> None:
    if release.get("schema") != "justai.release.v1":
        raise ValueError("unsupported release schema")
    for key in ("record_id", "state_slice", "status", "claim_ceiling", "created_at", "publisher_id"):
        require_string(release, key)
    digest = require_string(release, "content_digest")
    require_hex(digest, "content_digest", 64)
    if content_digest(release) != digest:
        raise ValueError("release content digest mismatch")
    records = release.get("records")
    if not isinstance(records, list) or not records:
        raise ValueError("release must contain records")
    reject_disclosure_fields(records)
    record_ids: set[str] = set()
    records_by_id: dict[str, dict[str, Any]] = {}
    known_digests: set[str] = set()
    reviewer_manifests: dict[str, dict[str, Any]] = {}
    reviewer_manifests_by_id: dict[str, dict[str, Any]] = {}
    reviews_by_target: dict[str, list[dict[str, Any]]] = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("release record must be an object")
        validate_record(record)
        record_id = record["record_id"]
        if record_id in record_ids:
            raise ValueError(f"duplicate record_id: {record_id}")
        record_ids.add(record_id)
        records_by_id[record_id] = record
        known_digests.add(record["content_digest"])
        if record["schema"] == "justai.source.v1":
            known_digests.update(record["source_digests"])
        if record["schema"] == "justai.artifact_manifest.v1":
            known_digests.add(record["artifact_digest"])
        if record["schema"] == "justai.syndicate.v1":
            reviewer_manifests[record["syndicate_id"]] = record
            reviewer_manifests_by_id[record["record_id"]] = record
        if record["schema"] == "justai.review_receipt.v1":
            reviews_by_target.setdefault(record["reviewed_record_id"], []).append(record)
    for record in records:
        schema = record["schema"]
        if schema == "justai.review_receipt.v1":
            target = records_by_id.get(record["reviewed_record_id"])
            if target is None or target["content_digest"] != record["reviewed_digest"]:
                raise ValueError(f"review digest does not match {record['reviewed_record_id']}")
            manifest = reviewer_manifests_by_id.get(record["syndicate_manifest_ref"])
            if manifest is None or manifest["syndicate_id"] != record["reviewer_org"]:
                raise ValueError(f"reviewer manifest is not bound for {record['record_id']}")
            verify_record_signatures(manifest)
        if schema in {"justai.claim.v1", "justai.incident.v1", "justai.commitment.v1", "justai.evaluation_receipt.v1", "justai.disclosure_decision.v1", "justai.claim_projection.v1", "justai.policy_trace.v1"}:
            if not record["source_digests"] or any(digest not in known_digests for digest in record["source_digests"]):
                raise ValueError(f"unresolved source provenance for {record['record_id']}")
            reviews = record.get("review_receipt_refs", [])
            if len(reviews) < 2:
                raise ValueError(f"{record['record_id']} requires two review receipts")
            accepted = []
            for reference in reviews:
                review = next((item for item in reviews_by_target.get(record["record_id"], []) if item["record_id"] == reference), None)
                if review is None or review["result"] != "accept" or review["conflict_of_interest"] or review["reviewer_org"] == release["publisher_id"]:
                    raise ValueError(f"invalid independent review {reference} for {record['record_id']}")
                accepted.append(review["reviewer_org"])
            if len(set(accepted)) < 2:
                raise ValueError(f"{record['record_id']} requires two reviewer organizations")
        for key in ("support_refs", "contradiction_refs", "evidence_refs", "subject_refs", "artifact_manifest_refs", "disclosure_decision_refs", "claim_projection_refs", "evaluation_receipt_refs", "artifact_refs"):
            for reference in record.get(key, []):
                if reference not in records_by_id:
                    raise ValueError(f"unknown record reference {reference} in {record['record_id']}")
        def require_schema(reference: str, expected: str) -> None:
            target = records_by_id[reference]
            if target["schema"] != expected:
                raise ValueError(f"{record['record_id']} reference {reference} must use {expected}")
        if schema == "justai.evaluation_receipt.v1":
            for reference in record["artifact_manifest_refs"]:
                require_schema(reference, "justai.artifact_manifest.v1")
                if records_by_id[reference]["artifact_digest"] not in record["source_digests"]:
                    raise ValueError(f"evaluation {record['record_id']} does not bind artifact digest")
            for reference in record["disclosure_decision_refs"]:
                require_schema(reference, "justai.disclosure_decision.v1")
        if schema == "justai.claim_projection.v1":
            for reference in record["disclosure_decision_refs"]:
                require_schema(reference, "justai.disclosure_decision.v1")
        if schema == "justai.policy_trace.v1":
            for reference in record["artifact_refs"]:
                require_schema(reference, "justai.artifact_manifest.v1")
                if records_by_id[reference]["artifact_digest"] not in record["source_digests"]:
                    raise ValueError(f"policy trace {record['record_id']} does not bind artifact digest")
            for reference in record["evaluation_receipt_refs"]:
                require_schema(reference, "justai.evaluation_receipt.v1")
            for reference in record["claim_projection_refs"]:
                require_schema(reference, "justai.claim_projection.v1")
        if schema == "justai.dispute.v1" and record["disputed_record_id"] not in records_by_id:
            raise ValueError(f"unknown disputed record {record['disputed_record_id']}")
    signatures = release.get("signature_refs")
    if not isinstance(signatures, list) or not signatures:
        raise ValueError("release requires at least one signature")
    verify_signatures(release)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("release", type=Path)
    args = parser.parse_args()
    try:
        validate_release(read_json(args.release))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(f"independently valid release: {args.release}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
