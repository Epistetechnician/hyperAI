#!/usr/bin/env python3
"""Run a bounded synthetic local quorum on short-lived GCP instances."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import re
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from tools.local_quorum import canonical_json, validate_packet
except ModuleNotFoundError:
    from local_quorum import canonical_json, validate_packet


SCHEMA = "justai.local_quorum.v1"
STATE_SLICE = "justai-local-quorum-v1"
CLAIM_CEILING = "local-replay"
DEFAULT_PROJECT = "secops1219"
DEFAULT_BUCKET = "justai-protected-intake-secops1219"
DEFAULT_ZONES = ("us-east4-a", "us-east4-b", "us-east4-c")
MACHINE_TYPE = "e2-micro"
IMAGE_FAMILY = "debian-12"
INPUT_PAYLOAD = "justai-local-quorum-synthetic-input-v1|bounded-arithmetic-replay|value=42"


def digest(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def run_command(arguments: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(arguments, check=False, text=True, capture_output=True)
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(arguments)}: {detail}")
    return result


def runner_material() -> tuple[str, str, str, str]:
    artifact_digest = digest(INPUT_PAYLOAD)
    result_payload = f"justai-local-quorum-result-v1|artifact={artifact_digest}|value=42|status=pass"
    expected_result_digest = digest(result_payload)
    runner_source = (
        f"justai-gcp-local-quorum-runner-v1|input={artifact_digest}|"
        f"result=sha256({result_payload})|network=metadata-only"
    )
    return artifact_digest, expected_result_digest, runner_source, digest(runner_source)


def startup_script(
    instance_name: str,
    artifact_digest: str,
    expected_result_digest: str,
    runner_source: str,
    runner_digest: str,
) -> str:
    return f'''#!/bin/bash
set -eu
METADATA_BASE="http://metadata.google.internal/computeMetadata/v1"
get_meta() {{
  curl -fsS -H "Metadata-Flavor: Google" "$METADATA_BASE/$1"
}}
sha256_text() {{
  printf "%s" "$1" | sha256sum | cut -d" " -f1
}}
INSTANCE_NAME="{instance_name}"
INPUT_PAYLOAD="{INPUT_PAYLOAD}"
EXPECTED_ARTIFACT_DIGEST="{artifact_digest}"
EXPECTED_RESULT_DIGEST="{expected_result_digest}"
EXPECTED_RUNNER_DIGEST="{runner_digest}"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
INSTANCE_ID="$(get_meta instance/id)"
ZONE_PATH="$(get_meta instance/zone)"
MACHINE_TYPE_PATH="$(get_meta instance/machine-type)"
MACHINE_DESCRIPTOR="id=$INSTANCE_ID|name=$INSTANCE_NAME|zone=$ZONE_PATH|machine_type=$MACHINE_TYPE_PATH"
MACHINE_DIGEST="$(sha256_text "$MACHINE_DESCRIPTOR")"
PYTHON_VERSION="$(python3 --version 2>&1 || true)"
OPENSSL_VERSION="$(openssl version 2>/dev/null || true)"
ENV_DESCRIPTOR="image=debian-12|kernel=$(uname -srmo)|arch=$(uname -m)|python=$PYTHON_VERSION|openssl=$OPENSSL_VERSION"
ENVIRONMENT_DIGEST="$(sha256_text "$ENV_DESCRIPTOR")"
RUNNER_SOURCE="{runner_source}"
RUNNER_DIGEST="$(sha256_text "$RUNNER_SOURCE")"
LOCAL_ARTIFACT_DIGEST="$(sha256_text "$INPUT_PAYLOAD")"
RESULT_PAYLOAD="justai-local-quorum-result-v1|artifact=$LOCAL_ARTIFACT_DIGEST|value=42|status=pass"
OBSERVED_RESULT_DIGEST="$(sha256_text "$RESULT_PAYLOAD")"
if [ "$LOCAL_ARTIFACT_DIGEST" != "$EXPECTED_ARTIFACT_DIGEST" ]; then
  echo "JUSTAI_ERROR=artifact_digest_mismatch"
  exit 1
fi
if [ "$OBSERVED_RESULT_DIGEST" != "$EXPECTED_RESULT_DIGEST" ]; then
  echo "JUSTAI_ERROR=expected_result_digest_mismatch"
  exit 1
fi
if [ "$RUNNER_DIGEST" != "$EXPECTED_RUNNER_DIGEST" ]; then
  echo "JUSTAI_ERROR=runner_digest_mismatch"
  exit 1
fi
KEY_PATH="/run/justai-local-quorum-ed25519.pem"
PAYLOAD_PATH="/run/justai-local-quorum-payload"
SIGNATURE_PATH="/run/justai-local-quorum-signature"
openssl genpkey -algorithm ED25519 -out "$KEY_PATH" 2>/dev/null
PUBLIC_KEY="$(openssl pkey -in "$KEY_PATH" -pubout -outform DER 2>/dev/null | od -An -tx1 -v | tr -d " \\n" | cut -c25-88)"
OUTPUT_PAYLOAD="instance_id=$INSTANCE_NAME|machine_digest=$MACHINE_DIGEST|environment_digest=$ENVIRONMENT_DIGEST|runner_digest=$RUNNER_DIGEST|observed_result_digest=$OBSERVED_RESULT_DIGEST|status=pass"
OUTPUT_DIGEST="$(sha256_text "$OUTPUT_PAYLOAD")"
COMPLETED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "JUSTAI_INSTANCE_ID=$INSTANCE_NAME"
echo "JUSTAI_MACHINE_DIGEST=$MACHINE_DIGEST"
echo "JUSTAI_ENVIRONMENT_DIGEST=$ENVIRONMENT_DIGEST"
echo "JUSTAI_RUNNER_DIGEST=$RUNNER_DIGEST"
echo "JUSTAI_OBSERVED_RESULT_DIGEST=$OBSERVED_RESULT_DIGEST"
echo "JUSTAI_OUTPUT_DIGEST=$OUTPUT_DIGEST"
echo "JUSTAI_STATUS=pass"
echo "JUSTAI_STARTED_AT=$STARTED_AT"
echo "JUSTAI_COMPLETED_AT=$COMPLETED_AT"
echo "JUSTAI_PUBLIC_KEY=$PUBLIC_KEY"
PACKET_DIGEST=""
for attempt in $(seq 1 120); do
  PACKET_DIGEST="$(get_meta instance/attributes/packet-digest || true)"
  if printf "%s" "$PACKET_DIGEST" | grep -Eq "^[0-9a-f]{{64}}$"; then
    break
  fi
  sleep 1
done
if ! printf "%s" "$PACKET_DIGEST" | grep -Eq "^[0-9a-f]{{64}}$"; then
  echo "JUSTAI_ERROR=packet_digest_timeout"
  exit 1
fi
PURPOSE="instance:$INSTANCE_NAME"
printf "justai:justai.local_quorum.v1:%s:%s" "$PACKET_DIGEST" "$PURPOSE" > "$PAYLOAD_PATH"
openssl pkeyutl -sign -rawin -inkey "$KEY_PATH" -in "$PAYLOAD_PATH" -out "$SIGNATURE_PATH" 2>/dev/null
SIGNATURE="$(od -An -tx1 -v "$SIGNATURE_PATH" | tr -d " \\n")"
echo "JUSTAI_PACKET_DIGEST=$PACKET_DIGEST"
echo "JUSTAI_SIGNATURE=$SIGNATURE"
echo "JUSTAI_COMPLETED_AT=$COMPLETED_AT"
echo "JUSTAI_DONE=1"
'''


def guest_fields(output: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for raw_line in output.splitlines():
        line = raw_line.strip()
        marker = line.find("JUSTAI_")
        if marker >= 0 and "=" in line[marker:]:
            key, value = line[marker:].split("=", 1)
            fields[key] = value
    return fields


def validate_base_fields(
    name: str,
    fields: dict[str, str],
    runner_digest: str,
    expected_result_digest: str,
) -> None:
    if fields.get("JUSTAI_ERROR"):
        raise RuntimeError(f"{name}: guest error {fields['JUSTAI_ERROR']}")
    required = (
        "JUSTAI_INSTANCE_ID",
        "JUSTAI_MACHINE_DIGEST",
        "JUSTAI_ENVIRONMENT_DIGEST",
        "JUSTAI_RUNNER_DIGEST",
        "JUSTAI_OBSERVED_RESULT_DIGEST",
        "JUSTAI_OUTPUT_DIGEST",
        "JUSTAI_STATUS",
        "JUSTAI_STARTED_AT",
        "JUSTAI_COMPLETED_AT",
        "JUSTAI_PUBLIC_KEY",
    )
    missing = [key for key in required if not fields.get(key)]
    if missing:
        raise RuntimeError(f"{name}: missing guest fields {missing}")
    if fields["JUSTAI_INSTANCE_ID"] != name:
        raise RuntimeError(f"{name}: instance name mismatch")
    if fields["JUSTAI_STATUS"] != "pass":
        raise RuntimeError(f"{name}: guest did not pass")
    if fields["JUSTAI_RUNNER_DIGEST"] != runner_digest:
        raise RuntimeError(f"{name}: runner digest mismatch")
    if fields["JUSTAI_OBSERVED_RESULT_DIGEST"] != expected_result_digest:
        raise RuntimeError(f"{name}: result digest mismatch")
    if not re.fullmatch(r"[0-9a-f]{64}", fields["JUSTAI_PUBLIC_KEY"]):
        raise RuntimeError(f"{name}: invalid public key")
    output_payload = (
        f"instance_id={name}|machine_digest={fields['JUSTAI_MACHINE_DIGEST']}|"
        f"environment_digest={fields['JUSTAI_ENVIRONMENT_DIGEST']}|runner_digest={fields['JUSTAI_RUNNER_DIGEST']}|"
        f"observed_result_digest={fields['JUSTAI_OBSERVED_RESULT_DIGEST']}|status=pass"
    )
    if digest(output_payload) != fields.get("JUSTAI_OUTPUT_DIGEST"):
        raise RuntimeError(f"{name}: output digest mismatch")


def parse_complete_fields(
    name: str,
    fields: dict[str, str],
    runner_digest: str,
    expected_result_digest: str,
    packet_digest: str,
) -> dict[str, str]:
    validate_base_fields(name, fields, runner_digest, expected_result_digest)
    required = ("JUSTAI_PACKET_DIGEST", "JUSTAI_SIGNATURE", "JUSTAI_DONE")
    missing = [key for key in required if not fields.get(key)]
    if missing:
        raise RuntimeError(f"{name}: missing signed guest fields {missing}")
    if fields["JUSTAI_PACKET_DIGEST"] != packet_digest or fields["JUSTAI_DONE"] != "1":
        raise RuntimeError(f"{name}: signed wrong packet digest or incomplete run")
    if not re.fullmatch(r"[0-9a-f]{128}", fields["JUSTAI_SIGNATURE"]):
        raise RuntimeError(f"{name}: invalid signature")
    return fields


def serial_output(name: str, zone: str, project: str) -> str:
    result = run_command(
        [
            "gcloud",
            "compute",
            "instances",
            "get-serial-port-output",
            name,
            "--project",
            project,
            "--zone",
            zone,
            "--port",
            "1",
        ],
        check=False,
    )
    return result.stdout + "\n" + result.stderr


def create_instance(name: str, zone: str, project: str, run_id: str, script: str) -> None:
    with tempfile.NamedTemporaryFile(
        "w",
        encoding="utf-8",
        prefix=f"{name}-",
        suffix=".sh",
    ) as handle:
        handle.write(script)
        handle.flush()
        run_command(
            [
                "gcloud",
                "compute",
                "instances",
                "create",
                name,
                "--project",
                project,
                "--zone",
                zone,
                "--machine-type",
                MACHINE_TYPE,
                "--image-family",
                IMAGE_FAMILY,
                "--image-project",
                "debian-cloud",
                "--network",
                "default",
                "--no-address",
                "--no-service-account",
                "--no-scopes",
                "--metadata-from-file",
                f"startup-script={handle.name}",
                "--metadata",
                "serial-port-enable=1",
                "--labels",
                f"state_slice={STATE_SLICE},run_id={run_id}",
                "--boot-disk-size",
                "10GB",
                "--quiet",
            ]
        )


def instance_exists(name: str, zone: str, project: str) -> bool:
    return run_command(
        [
            "gcloud",
            "compute",
            "instances",
            "describe",
            name,
            "--project",
            project,
            "--zone",
            zone,
        ],
        check=False,
    ).returncode == 0


def delete_instances(pairs: list[tuple[str, str]], project: str) -> None:
    existing = [pair for pair in pairs if instance_exists(pair[0], pair[1], project)]
    if not existing:
        print("ephemeral_instances_remaining=0")
        return
    print("deleting_ephemeral_instances=" + ",".join(name for name, _zone in existing))
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(existing)) as pool:
        futures = {
            pool.submit(
                run_command,
                [
                    "gcloud",
                    "compute",
                    "instances",
                    "delete",
                    name,
                    "--project",
                    project,
                    "--zone",
                    zone,
                    "--delete-disks",
                    "all",
                    "--quiet",
                ],
                False,
            ): name
            for name, zone in existing
        }
        for future in concurrent.futures.as_completed(futures):
            name = futures[future]
            result = future.result()
            if result.returncode == 0:
                print(f"deleted={name}")
            else:
                print(f"delete_failed={name}: {result.stderr.strip()}")
    remaining = [pair[0] for pair in pairs if instance_exists(pair[0], pair[1], project)]
    if remaining:
        raise RuntimeError(f"ephemeral instances remain after cleanup: {remaining}")
    print("ephemeral_instances_remaining=0")


def build_packet(
    run_id: str,
    artifact_digest: str,
    expected_result_digest: str,
    records: dict[str, dict[str, str]],
    pairs: list[tuple[str, str]],
    project: str,
) -> dict[str, Any]:
    instances = []
    for name, _zone in pairs:
        fields = records[name]
        instances.append(
            {
                "instance_id": name,
                "machine_digest": fields["JUSTAI_MACHINE_DIGEST"],
                "environment_digest": fields["JUSTAI_ENVIRONMENT_DIGEST"],
                "runner_digest": fields["JUSTAI_RUNNER_DIGEST"],
                "observed_result_digest": fields["JUSTAI_OBSERVED_RESULT_DIGEST"],
                "output_digest": fields["JUSTAI_OUTPUT_DIGEST"],
                "status": "pass",
                "started_at": fields["JUSTAI_STARTED_AT"],
                "completed_at": fields["JUSTAI_COMPLETED_AT"],
            }
        )
    packet: dict[str, Any] = {
        "schema": SCHEMA,
        "record_id": f"local-quorum-gcp-{run_id}",
        "state_slice": STATE_SLICE,
        "status": "Verified",
        "verification_mode": "local-quorum",
        "claim_ceiling": CLAIM_CEILING,
        "artifact_digest": artifact_digest,
        "evaluation_scope": "bounded synthetic deterministic replay on three operator-controlled GCP e2-micro instances",
        "expected_result_digest": expected_result_digest,
        "content_digest": "",
        "signature_refs": [],
        "instances": instances,
    }
    packet_basis = {
        key: value
        for key, value in packet.items()
        if key not in {"content_digest", "signature_refs"}
    }
    packet["content_digest"] = hashlib.sha256(canonical_json(packet_basis)).hexdigest()
    signatures = []
    for name, _zone in pairs:
        fields = records[name]
        if not fields.get("JUSTAI_SIGNATURE"):
            continue
        signatures.append(
            {
                "role": "instance",
                "instance_id": name,
                "principal_id": f"gcp:{project}:{name}",
                "key_id": f"{name}-ed25519-v1",
                "purpose": f"instance:{name}",
                "public_key": fields["JUSTAI_PUBLIC_KEY"],
                "signature": fields["JUSTAI_SIGNATURE"],
            }
        )
    packet["signature_refs"] = signatures
    return packet


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", default=DEFAULT_PROJECT)
    parser.add_argument("--bucket", default=DEFAULT_BUCKET)
    parser.add_argument(
        "--run-id",
        default=datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S"),
    )
    args = parser.parse_args()
    if not re.fullmatch(r"[a-z0-9-]+", args.run_id):
        parser.error("run-id must contain lowercase letters, digits, and hyphens")

    artifact_digest, expected_result_digest, runner_source, runner_digest = runner_material()
    names = [f"justai-lq-{args.run_id}-{suffix}" for suffix in ("a", "b", "c")]
    pairs = list(zip(names, DEFAULT_ZONES))
    remote_object = f"gs://{args.bucket}/local-quorum/{args.run_id}/packet.json"
    print(f"run_id={args.run_id}")
    print(f"artifact_digest={artifact_digest}")
    print(f"expected_result_digest={expected_result_digest}")
    print(f"runner_digest={runner_digest}")
    print("compute_budget=3 x e2-micro x one bounded run; guests wait at most 120 seconds")
    try:
        scripts = {
            name: startup_script(
                name,
                artifact_digest,
                expected_result_digest,
                runner_source,
                runner_digest,
            )
            for name, _zone in pairs
        }
        print("creating_instances=" + ",".join(names))
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
            futures = {
                pool.submit(
                    create_instance,
                    name,
                    zone,
                    args.project,
                    args.run_id,
                    scripts[name],
                ): (name, zone)
                for name, zone in pairs
            }
            for future in concurrent.futures.as_completed(futures):
                name, zone = futures[future]
                future.result()
                print(f"created={name} zone={zone}")

        records: dict[str, dict[str, str]] = {}
        deadline = time.time() + 150
        while len(records) < len(pairs) and time.time() < deadline:
            with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
                futures = {
                    pool.submit(serial_output, name, zone, args.project): (name, zone)
                    for name, zone in pairs
                    if name not in records
                }
                for future in concurrent.futures.as_completed(futures):
                    name, _zone = futures[future]
                    fields = guest_fields(future.result())
                    if fields.get("JUSTAI_ERROR"):
                        raise RuntimeError(f"{name}: guest error {fields['JUSTAI_ERROR']}")
                    if fields.get("JUSTAI_PUBLIC_KEY") and fields.get("JUSTAI_OUTPUT_DIGEST"):
                        validate_base_fields(
                            name,
                            fields,
                            runner_digest,
                            expected_result_digest,
                        )
                        records[name] = fields
                        print(f"guest_ready={name}")
            if len(records) < len(pairs):
                time.sleep(3)
        if len(records) != len(pairs):
            missing = [name for name, _zone in pairs if name not in records]
            raise RuntimeError(f"guest startup timeout; missing {missing}")

        packet = build_packet(
            args.run_id,
            artifact_digest,
            expected_result_digest,
            records,
            pairs,
            args.project,
        )
        packet_digest = packet["content_digest"]
        print(f"packet_content_digest={packet_digest}")
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
            futures = {
                pool.submit(
                    run_command,
                    [
                        "gcloud",
                        "compute",
                        "instances",
                        "add-metadata",
                        name,
                        "--project",
                        args.project,
                        "--zone",
                        zone,
                        "--metadata",
                        f"packet-digest={packet_digest}",
                        "--quiet",
                    ],
                ): name
                for name, zone in pairs
            }
            for future in concurrent.futures.as_completed(futures):
                future.result()
                print(f"packet_digest_sent={futures[future]}")

        signed_records: dict[str, dict[str, str]] = {}
        deadline = time.time() + 150
        while len(signed_records) < len(pairs) and time.time() < deadline:
            with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
                futures = {
                    pool.submit(serial_output, name, zone, args.project): (name, zone)
                    for name, zone in pairs
                    if name not in signed_records
                }
                for future in concurrent.futures.as_completed(futures):
                    name, _zone = futures[future]
                    fields = guest_fields(future.result())
                    if fields.get("JUSTAI_ERROR"):
                        raise RuntimeError(f"{name}: guest error {fields['JUSTAI_ERROR']}")
                    if fields.get("JUSTAI_DONE") == "1":
                        signed_records[name] = parse_complete_fields(
                            name,
                            fields,
                            runner_digest,
                            expected_result_digest,
                            packet_digest,
                        )
                        print(f"guest_signed={name}")
            if len(signed_records) < len(pairs):
                time.sleep(3)
        if len(signed_records) != len(pairs):
            missing = [name for name, _zone in pairs if name not in signed_records]
            raise RuntimeError(f"guest signing timeout; missing {missing}")

        packet = build_packet(
            args.run_id,
            artifact_digest,
            expected_result_digest,
            signed_records,
            pairs,
            args.project,
        )
        if packet["content_digest"] != packet_digest:
            raise RuntimeError("packet content digest changed after signed attestation collection")
        validate_packet(packet)

        with tempfile.TemporaryDirectory(prefix=f"justai-local-quorum-{args.run_id}-") as directory:
            packet_path = Path(directory) / "packet.json"
            packet_bytes = canonical_json(packet) + b"\n"
            packet_path.write_bytes(packet_bytes)
            validation = run_command(["python3", "tools/local_quorum.py", str(packet_path)])
            print(validation.stdout.strip())
            run_command(["gcloud", "storage", "cp", str(packet_path), remote_object])
            remote = run_command(["gcloud", "storage", "cat", remote_object])
            if remote.stdout.encode("utf-8") != packet_bytes:
                raise RuntimeError("remote packet bytes differ from validated local packet")
            print("remote_packet_byte_match=true")
            metadata = run_command(
                [
                    "gcloud",
                    "storage",
                    "objects",
                    "describe",
                    remote_object,
                    "--format",
                    "yaml(name,size,md5Hash,metadata)",
                ]
            )
            print(metadata.stdout.strip())
        print(f"remote_metadata_object={remote_object}")
    finally:
        delete_instances(pairs, args.project)
        print("cleanup_complete=true")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
