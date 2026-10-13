#!/usr/bin/env python3
"""Assemble, sign, independently validate, and render one static release."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--publisher-id", required=True)
    parser.add_argument("--created-at", required=True)
    parser.add_argument("--release-id", required=True)
    parser.add_argument("--key-id", required=True)
    parser.add_argument("--seed-hex", required=True, help="controlled release key; never commit or log it")
    args = parser.parse_args()

    assembled = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "justai-contracts",
            "--",
            "assemble",
            str(args.input_dir),
            str(args.output_dir),
            args.publisher_id,
            args.created_at,
            args.release_id,
            args.key_id,
            args.seed_hex,
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    match = re.search(r"assembled signed bundle (.+)$", assembled.stdout.strip())
    if not match:
        raise RuntimeError("assembler did not return a bundle path")
    release_path = Path(match.group(1))
    subprocess.run(["python3", "tools/independent_validate.py", str(release_path)], check=True)
    html_path = release_path.with_name("index.html")
    subprocess.run(
        ["node", "site/render.mjs", "--release", str(release_path), "--out", str(html_path)],
        check=True,
    )
    print(f"published static release: {html_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
