#!/usr/bin/env python3
"""Verify imported product trees against their pinned local manifest."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "manifests/monorepo-imports.json"
EXCLUDED_PARTS = {
    ".git",
    ".dist",
    ".mypy_cache",
    ".pytest_cache",
    ".venv",
    "build",
    "target",
    "node_modules",
    "__pycache__",
}


def included(path: Path) -> bool:
    return (
        not any(part in EXCLUDED_PARTS for part in path.parts)
        and not any(part.endswith(".egg-info") for part in path.parts)
        and path.name != ".DS_Store"
        and path.suffix != ".pyc"
    )


def tree_digest(root: Path) -> tuple[int, str]:
    entries: list[tuple[str, bytes]] = []
    for path in root.rglob("*"):
        if path.is_file():
            relative = path.relative_to(root)
            if included(relative):
                entries.append((relative.as_posix(), hashlib.sha256(path.read_bytes()).digest()))
    entries.sort(key=lambda entry: entry[0])
    digest = hashlib.sha256()
    for relative, file_digest in entries:
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_digest)
        digest.update(b"\n")
    return len(entries), digest.hexdigest()


def main() -> int:
    try:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"IMPORT_MANIFEST_INVALID: {error}", file=sys.stderr)
        return 2

    failures = 0
    for imported in manifest.get("imports", []):
        root = ROOT / imported["path"]
        if not root.is_dir():
            print(f"MISSING {imported['name']}: {imported['path']}")
            failures += 1
            continue
        count, digest = tree_digest(root)
        matches = count == imported["file_count"] and digest == imported["tree_sha256"]
        status = "PASS" if matches else "MISMATCH"
        print(
            f"{status} {imported['name']}: files={count} sha256={digest} "
            f"revision={imported['revision']}"
        )
        failures += not matches

    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
