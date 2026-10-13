#!/usr/bin/env python3
"""Verify the JustAI migration topology without inspecting source contents."""

from __future__ import annotations

import json
import hashlib
from pathlib import Path
import subprocess
from typing import Any


EXPECTED_SOURCES = {
    "ai-discovery-os",
    "aligned-holistic-continual-learning",
    "composed-zk-benchmark-os",
    "security-alignment-os",
    "recursive-meta-harness",
    "composed-platform",
    "statebook",
    "lusis-mesh",
}


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain an object")
    return value


def git(path: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(path), *args], text=True).strip()


def validate_observed_heads(
    checkout: Path,
    entry: dict[str, Any],
    *,
    revalidate_checkout: bool,
) -> set[str]:
    """Validate recorded drift metadata without treating a moving checkout as input."""

    pinned = entry["head"]
    observed = entry.get("observed_followup_heads", [])
    if not isinstance(observed, list) or any(
        not isinstance(head, str)
        or len(head) != 40
        or any(character not in "0123456789abcdef" for character in head)
        for head in observed
    ):
        raise ValueError(f"observed source heads must be lowercase Git commits: {entry['name']}")
    legacy = entry.get("current_head_on_followup_review")
    if legacy is not None:
        if not isinstance(legacy, str) or len(legacy) != 40 or any(
            character not in "0123456789abcdef" for character in legacy
        ):
            raise ValueError(f"current follow-up head is invalid: {entry['name']}")
        observed = [*observed, legacy]
    if revalidate_checkout:
        for head in {pinned, *observed}:
            if subprocess.run(
                ["git", "-C", str(checkout), "cat-file", "-e", f"{head}^{{commit}}"],
                check=False,
            ).returncode:
                raise ValueError(f"recorded source head is unavailable: {entry['name']}:{head}")
    return {pinned, *observed}


def verify_source_provenance(root: Path, source: dict[str, Any], migration: dict[str, Any]) -> None:
    entries = {entry["name"]: entry for entry in source["source_repositories"]}
    available_checkouts = [Path(entry["path"]).is_dir() for entry in entries.values()]
    if any(available_checkouts) and not all(available_checkouts):
        raise ValueError("source checkout availability is partial; provide all eight or none")
    revalidate_checkouts = all(available_checkouts)
    if not revalidate_checkouts:
        print(
            "source checkouts unavailable; validating recorded provenance metadata only; "
            "live source commit and file digests are not revalidated"
        )
    for entry in entries.values():
        checkout = Path(entry["path"])
        if revalidate_checkouts:
            if git(checkout, "branch", "--show-current") != entry["branch"]:
                raise ValueError(f"source branch drift: {entry['name']}")
        pinned = entry["head"]
        allowed_heads = validate_observed_heads(
            checkout,
            entry,
            revalidate_checkout=revalidate_checkouts,
        )
        if revalidate_checkouts:
            current = git(checkout, "rev-parse", "HEAD")
            if current not in allowed_heads:
                print(
                    f"observed source head drift beyond recorded metadata: {entry['name']}:{current}; "
                    f"pinned input remains {pinned}"
                )
            elif current != pinned:
                print(f"observed source head drift: {entry['name']}:{current}; pinned input remains {pinned}")
            for reference in entry["reference_paths"]:
                if subprocess.run(["git", "-C", str(checkout), "cat-file", "-e", f"{pinned}:{reference}"], check=False).returncode:
                    raise ValueError(f"missing pinned source reference: {entry['name']}:{reference}")

    provenance_path = root / migration["source_file_provenance_manifest"]
    provenance = load(provenance_path)
    if provenance.get("state_slice") != migration["state_slice"]:
        raise ValueError("source file provenance state slice drift")
    provenance_by_key: dict[tuple[str, str], dict[str, Any]] = {}
    for reference in provenance.get("references", []):
        key = (reference.get("source_repository", ""), reference.get("source_path", ""))
        if key in provenance_by_key:
            raise ValueError(f"duplicate source file provenance: {key}")
        provenance_by_key[key] = reference
        entry = entries.get(key[0])
        if entry is None or key[1] not in entry["reference_paths"]:
            raise ValueError(f"source file provenance is not allowlisted: {key}")
        if reference.get("source_commit") != entry["head"]:
            raise ValueError(f"source file provenance commit mismatch: {key}")
        if not isinstance(reference.get("source_digest"), str) or len(reference["source_digest"]) != 64:
            raise ValueError(f"source file provenance digest is invalid: {key}")
        if revalidate_checkouts:
            checkout = Path(entry["path"])
            content = subprocess.check_output(["git", "-C", str(checkout), "show", f"{entry['head']}:{key[1]}"])
            actual_digest = hashlib.sha256(content).hexdigest()
            if reference.get("source_digest") != actual_digest:
                raise ValueError(f"source file provenance digest mismatch: {key}")

    for capability in migration["capabilities"]:
        for source_ref in capability["source_refs"]:
            name, path = source_ref.split(":", 1)
            if (name, path) not in provenance_by_key:
                raise ValueError(f"capability lacks exact file provenance: {source_ref}")


def validate(root: Path) -> None:
    source = load(root / "manifests/source-repositories/manifest.json")
    migration = load(root / "manifests/migration/manifest.json")
    if source.get("schema") != "justai.source_repository_inventory.v1":
        raise ValueError("unexpected source inventory schema")
    if migration.get("schema") != "justai.migration_manifest.v1":
        raise ValueError("unexpected migration manifest schema")
    if source.get("state_slice") != "justai-governed-discovery-monorepo-v1":
        raise ValueError("source inventory state slice drift")
    entries = source.get("source_repositories")
    if not isinstance(entries, list) or {entry.get("name") for entry in entries} != EXPECTED_SOURCES:
        raise ValueError("source inventory must contain exactly the eight requested repositories")
    for entry in entries:
        for field in ("path", "branch", "head", "working_tree", "migration_mode", "reference_paths", "excluded_material"):
            if not entry.get(field):
                raise ValueError(f"source inventory entry missing {field}: {entry.get('name')}")
        if any(not isinstance(path, str) or not path or path.startswith("/") for path in entry["reference_paths"]):
            raise ValueError(f"reference paths must be repository-relative: {entry['name']}")
    capabilities = migration.get("capabilities")
    if not isinstance(capabilities, list) or not capabilities:
        raise ValueError("migration capabilities must be non-empty")
    owners = [item.get("owner") for item in capabilities]
    if any(not owner for owner in owners) or len(owners) != len(set(owners)):
        raise ValueError("migration capability owners must be unique")
    if not migration.get("source_repositories_preserved"):
        raise ValueError("source repositories must remain preserved")
    verify_source_provenance(root, source, migration)


def main() -> int:
    validate(Path(__file__).resolve().parents[1])
    print("valid JustAI monorepo inventory and ownership manifest")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
