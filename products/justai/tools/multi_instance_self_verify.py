#!/usr/bin/env python3
"""Run and aggregate bounded same-operator verification across instances."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.local_quorum import (
    SPKI_PREFIX,
    content_digest as packet_content_digest,
    signature_payload,
    unique_object,
    validate_packet,
)


INSTANCE_SCHEMA = "justai.self_verification_instance_result.v1"
SELF_VERIFICATION_SCOPE = "python-contract-and-boundary-tests-only"
REQUIRED_INSTANCE_COUNT = 3
SELF_VERIFICATION_TEST_MODULES = (
    "tools.tests.test_gcp_self_verify",
    "tools.tests.test_local_quorum",
    "tools.tests.test_multi_instance_self_verify",
    "tests.custody.test_boundaries",
    "tests.integration.test_governed_loop",
)


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def file_digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def timestamp() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def run_check(root: Path, name: str, command: list[str]) -> dict[str, Any]:
    result = subprocess.run(
        command,
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    output = f"stdout={result.stdout}\nstderr={result.stderr}"
    return {
        "name": name,
        "status": "pass" if result.returncode == 0 else "fail",
        "returncode": result.returncode,
        "output_digest": hashlib.sha256(output.encode("utf-8")).hexdigest(),
        "failure_excerpt": output[-4000:] if result.returncode != 0 else "",
    }


def environment_digest() -> str:
    version_results = []
    for command in (["python3", "--version"], ["openssl", "version"]):
        result = subprocess.run(command, capture_output=True, text=True, check=False)
        version_results.append(f"{command[0]}:{result.returncode}:{result.stdout}:{result.stderr}")
    return hashlib.sha256(
        "\n".join([platform.system(), platform.machine(), *version_results]).encode("utf-8")
    ).hexdigest()


def instance_record(args: argparse.Namespace) -> dict[str, Any]:
    root = args.bundle_root.resolve()
    archive = args.archive.resolve()
    if not root.is_dir():
        raise ValueError(f"bundle root does not exist: {root}")
    if file_digest(archive) != args.artifact_digest:
        raise ValueError("bundle archive digest does not match artifact_digest")
    started_at = timestamp()
    checks = [
        run_check(
            root,
            "governed-contract-and-boundary-tests",
            ["python3", "-m", "unittest", *SELF_VERIFICATION_TEST_MODULES],
        ),
        run_check(
            root,
            "python-compile",
            [
                "python3",
                "-m",
                "py_compile",
                "tools/local_quorum.py",
                "tools/multi_instance_self_verify.py",
                "tools/verify_monorepo.py",
                "apps/discovery/src/loop.py",
                "apps/platform/src/runtime.py",
                "apps/operations/src/recovery.py",
            ],
        ),
    ]
    completed_at = timestamp()
    result_summary = {
        "schema": INSTANCE_SCHEMA,
        "target_commit": args.expected_commit,
        "artifact_digest": args.artifact_digest,
        "evaluation_scope": SELF_VERIFICATION_SCOPE,
        "checks": [{"name": item["name"], "status": item["status"]} for item in checks],
    }
    observed_result_digest = digest(result_summary)
    instance = {
        "instance_id": args.instance_id,
        "machine_digest": hashlib.sha256(args.machine_id.encode("utf-8")).hexdigest(),
        "environment_digest": environment_digest(),
        "runner_digest": file_digest(Path(__file__).resolve()),
        "observed_result_digest": observed_result_digest,
        "output_digest": digest({"summary": result_summary, "checks": checks}),
        "status": "pass" if all(item["status"] == "pass" for item in checks) else "fail",
        "started_at": started_at,
        "completed_at": completed_at,
    }
    return {"instance": instance, "summary": result_summary, "checks": checks}


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_json(value) + b"\n")


def run_instance(args: argparse.Namespace) -> int:
    result = instance_record(args)
    key_path = args.key_path.resolve()
    key_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(key_path)], check=True)
    os.chmod(key_path, 0o600)
    write_json(args.output.resolve(), result)
    return 0 if result["instance"]["status"] == "pass" else 1


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def sign_instance(args: argparse.Namespace) -> int:
    key_path = args.key_path.resolve()
    if not key_path.is_file():
        raise ValueError(f"instance key does not exist: {key_path}")
    purpose = f"instance:{args.instance_id}"
    payload = signature_payload(args.packet_digest, purpose)
    payload_path = key_path.with_suffix(".payload")
    signature_path = key_path.with_suffix(".signature")
    public_path = key_path.with_suffix(".public.der")
    payload_path.write_bytes(payload)
    subprocess.run(
        [
            "openssl",
            "pkeyutl",
            "-sign",
            "-rawin",
            "-inkey",
            str(key_path),
            "-in",
            str(payload_path),
            "-out",
            str(signature_path),
        ],
        check=True,
    )
    subprocess.run(
        [
            "openssl",
            "pkey",
            "-in",
            str(key_path),
            "-pubout",
            "-outform",
            "DER",
            "-out",
            str(public_path),
        ],
        check=True,
    )
    public_bytes = public_path.read_bytes()
    if not public_bytes.startswith(SPKI_PREFIX):
        raise ValueError("unexpected Ed25519 public-key encoding")
    write_json(
        args.output.resolve(),
        {
            "role": "instance",
            "instance_id": args.instance_id,
            "principal_id": args.principal_id,
            "key_id": f"{args.instance_id}-ephemeral-ed25519",
            "purpose": purpose,
            "public_key": public_bytes[len(SPKI_PREFIX) :].hex(),
            "signature": signature_path.read_bytes().hex(),
        },
    )
    return 0


def aggregate(args: argparse.Namespace) -> int:
    instance_documents = [read_json(path) for path in args.instance_outputs]
    if len(instance_documents) != REQUIRED_INSTANCE_COUNT:
        raise ValueError(f"exactly {REQUIRED_INSTANCE_COUNT} instance outputs are required")
    instances = [document.get("instance") for document in instance_documents]
    if any(not isinstance(instance, dict) for instance in instances):
        raise ValueError("instance output is missing its instance record")
    result_digests = {instance["observed_result_digest"] for instance in instances}
    if len(result_digests) != 1:
        raise ValueError("instances did not produce one common result digest")
    if any(instance["status"] != "pass" for instance in instances):
        raise ValueError("a self-verification instance failed")
    instance_ids = [instance["instance_id"] for instance in instances]
    if len(set(instance_ids)) != len(instance_ids):
        raise ValueError("instance IDs must be unique")
    signatures = [read_json(path) for path in args.signature_outputs]
    packet = {
        "schema": "justai.local_quorum.v1",
        "record_id": args.record_id,
        "state_slice": "justai-local-quorum-v1",
        "status": "Verified",
        "verification_mode": "local-quorum",
        "claim_ceiling": "local-replay",
        "artifact_digest": args.artifact_digest,
        "evaluation_scope": f"target_commit={args.expected_commit};{SELF_VERIFICATION_SCOPE}",
        "expected_result_digest": next(iter(result_digests)),
        "content_digest": "",
        "signature_refs": signatures,
        "instances": instances,
    }
    packet["content_digest"] = packet_content_digest(packet)
    validate_packet(packet)
    write_json(args.output.resolve(), packet)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    instance_parser = subparsers.add_parser("instance", help="run the fixed checks on one instance")
    instance_parser.add_argument("--bundle-root", type=Path, required=True)
    instance_parser.add_argument("--archive", type=Path, required=True)
    instance_parser.add_argument("--expected-commit", required=True)
    instance_parser.add_argument("--artifact-digest", required=True)
    instance_parser.add_argument("--instance-id", required=True)
    instance_parser.add_argument("--machine-id", required=True)
    instance_parser.add_argument("--key-path", type=Path, required=True)
    instance_parser.add_argument("--output", type=Path, required=True)
    instance_parser.set_defaults(handler=run_instance)

    sign_parser = subparsers.add_parser("sign", help="sign an instance attestation")
    sign_parser.add_argument("--key-path", type=Path, required=True)
    sign_parser.add_argument("--instance-id", required=True)
    sign_parser.add_argument("--principal-id", default="local-operator")
    sign_parser.add_argument("--packet-digest", required=True)
    sign_parser.add_argument("--output", type=Path, required=True)
    sign_parser.set_defaults(handler=sign_instance)

    aggregate_parser = subparsers.add_parser("aggregate", help="aggregate three instance outputs")
    aggregate_parser.add_argument("--expected-commit", required=True)
    aggregate_parser.add_argument("--artifact-digest", required=True)
    aggregate_parser.add_argument("--record-id", required=True)
    aggregate_parser.add_argument("--instance-outputs", type=Path, nargs=REQUIRED_INSTANCE_COUNT, required=True)
    aggregate_parser.add_argument("--signature-outputs", type=Path, nargs=REQUIRED_INSTANCE_COUNT, required=True)
    aggregate_parser.add_argument("--output", type=Path, required=True)
    aggregate_parser.set_defaults(handler=aggregate)

    args = parser.parse_args()
    try:
        return args.handler(args)
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.error(str(error))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
