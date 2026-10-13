#!/usr/bin/env python3
"""Run the JustAI local-quorum checks on short-lived isolated GCP instances."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.local_quorum import content_digest, validate_packet
from tools.multi_instance_self_verify import aggregate, read_json, write_json


ZONES = ("us-east1-b", "us-east1-c", "us-east1-d")
MACHINE_TYPE = "e2-micro"
IMAGE_FAMILY = "ubuntu-2204-lts"
IMAGE_PROJECT = "ubuntu-os-cloud"
MINIMUM_INSTANCE_COUNT = 3


def gcloud(arguments: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["gcloud", *arguments],
        capture_output=True,
        text=True,
        check=False,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"gcloud command failed ({result.returncode}): {' '.join(arguments)}: {detail}")
    return result


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def git(*arguments: str) -> str:
    return subprocess.check_output(["git", *arguments], text=True).strip()


def make_archive(root: Path, output: Path) -> tuple[str, str]:
    commit = git("-C", str(root), "rev-parse", "HEAD")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise RuntimeError("target HEAD is not a full lowercase Git commit")
    subprocess.run(
        ["git", "-C", str(root), "archive", "--format=tar", "--output", str(output), commit],
        check=True,
    )
    return commit, sha256_file(output)


def resource_names(run_id: str) -> dict[str, str]:
    return {
        "network": f"justai-selfverify-vpc-{run_id}",
        "subnet": f"justai-selfverify-subnet-{run_id}",
        "firewall": f"justai-selfverify-iap-{run_id}",
    }


def create_network(project: str, names: dict[str, str]) -> None:
    gcloud(["services", "enable", "compute.googleapis.com", "iap.googleapis.com", "--project", project])
    gcloud(
        [
            "compute",
            "networks",
            "create",
            names["network"],
            "--project",
            project,
            "--subnet-mode",
            "custom",
            "--bgp-routing-mode",
            "regional",
            "--quiet",
        ]
    )
    gcloud(
        [
            "compute",
            "networks",
            "subnets",
            "create",
            names["subnet"],
            "--project",
            project,
            "--network",
            names["network"],
            "--region",
            "us-east1",
            "--range",
            "10.199.0.0/24",
            "--quiet",
        ]
    )
    gcloud(
        [
            "compute",
            "firewall-rules",
            "create",
            names["firewall"],
            "--project",
            project,
            "--network",
            names["network"],
            "--direction",
            "INGRESS",
            "--action",
            "ALLOW",
            "--rules",
            "tcp:22",
            "--source-ranges",
            "35.235.240.0/20",
            "--target-tags",
            "justai-selfverify",
            "--quiet",
        ]
    )


def create_instance(project: str, network: str, subnet: str, name: str, zone: str, run_id: str) -> None:
    gcloud(
        [
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
            IMAGE_PROJECT,
            "--network",
            network,
            "--subnet",
            subnet,
            "--no-address",
            "--no-service-account",
            "--no-scopes",
            "--tags",
            "justai-selfverify",
            "--labels",
            f"state_slice=justai-local-quorum-v1,run_id={run_id}",
            "--boot-disk-size",
            "10GB",
            "--quiet",
        ]
    )


def instance_id(project: str, name: str, zone: str) -> str:
    return gcloud(
        [
            "compute",
            "instances",
            "describe",
            name,
            "--project",
            project,
            "--zone",
            zone,
            "--format=value(id)",
        ]
    ).stdout.strip()


def wait_for_ssh(project: str, name: str, zone: str) -> None:
    deadline = time.time() + 180
    while time.time() < deadline:
        result = gcloud(
            [
                "compute",
                "ssh",
                name,
                "--project",
                project,
                "--zone",
                zone,
                "--tunnel-through-iap",
                "--command",
                "true",
                "--quiet",
            ],
            check=False,
        )
        if result.returncode == 0:
            return
        time.sleep(5)
    raise RuntimeError(f"IAP SSH did not become ready: {name}")


def copy_to_instance(project: str, name: str, zone: str, source: Path, destination: str) -> None:
    gcloud(
        [
            "compute",
            "scp",
            str(source),
            f"{name}:{destination}",
            "--project",
            project,
            "--zone",
            zone,
            "--tunnel-through-iap",
            "--quiet",
        ]
    )


def copy_from_instance(project: str, name: str, zone: str, source: str, destination: Path) -> None:
    gcloud(
        [
            "compute",
            "scp",
            f"{name}:{source}",
            str(destination),
            "--project",
            project,
            "--zone",
            zone,
            "--tunnel-through-iap",
            "--quiet",
        ]
    )


def run_instance(
    project: str,
    name: str,
    zone: str,
    expected_commit: str,
    artifact_digest: str,
    machine_id: str,
) -> None:
    root = "$HOME/justai-self-verify"
    command = " ".join(
        [
            "set -eu;",
            f"mkdir -p {root};",
            f"tar -xf $HOME/justai-bundle.tar -C {root};",
            "set +e;",
            f"python3 {root}/tools/multi_instance_self_verify.py instance",
            f"--bundle-root {root}",
            "--archive $HOME/justai-bundle.tar",
            f"--expected-commit {shlex.quote(expected_commit)}",
            f"--artifact-digest {shlex.quote(artifact_digest)}",
            f"--instance-id {shlex.quote(name)}",
            f"--machine-id {shlex.quote(machine_id)}",
            f"--key-path {root}/instance.key",
            f"--output {root}/instance.json",
            "; status=$?;",
            f"if [ ! -f {root}/instance.json ]; then printf 'guest self-verification produced no result\\n' >&2; exit $status; fi;",
            "exit 0;",
        ]
    )
    gcloud(
        [
            "compute",
            "ssh",
            name,
            "--project",
            project,
            "--zone",
            zone,
            "--tunnel-through-iap",
            "--command",
            command,
            "--quiet",
        ]
    )


def sign_instance(project: str, name: str, zone: str, packet_digest: str) -> None:
    root = "$HOME/justai-self-verify"
    command = " ".join(
        [
            f"python3 {root}/tools/multi_instance_self_verify.py sign",
            f"--key-path {root}/instance.key",
            f"--instance-id {shlex.quote(name)}",
            "--principal-id local-operator",
            f"--packet-digest {shlex.quote(packet_digest)}",
            f"--output {root}/signature.json",
        ]
    )
    gcloud(
        [
            "compute",
            "ssh",
            name,
            "--project",
            project,
            "--zone",
            zone,
            "--tunnel-through-iap",
            "--command",
            command,
            "--quiet",
        ]
    )


def packet_basis(
    expected_commit: str,
    artifact_digest: str,
    record_id: str,
    instances: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "schema": "justai.local_quorum.v1",
        "record_id": record_id,
        "state_slice": "justai-local-quorum-v1",
        "status": "Verified",
        "verification_mode": "local-quorum",
        "claim_ceiling": "local-replay",
        "artifact_digest": artifact_digest,
        "evaluation_scope": (
            f"target_commit={expected_commit};python-contract-and-boundary-tests-only"
        ),
        "expected_result_digest": instances[0]["observed_result_digest"],
        "content_digest": "",
        "signature_refs": [],
        "instances": instances,
    }


def cleanup(project: str, names: dict[str, str], instances: list[tuple[str, str]]) -> None:
    for name, zone in instances:
        gcloud(
            [
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
            check=False,
        )
    gcloud(
        [
            "compute",
            "firewall-rules",
            "delete",
            names["firewall"],
            "--project",
            project,
            "--quiet",
        ],
        check=False,
    )
    gcloud(
        [
            "compute",
            "networks",
            "subnets",
            "delete",
            names["subnet"],
            "--region",
            "us-east1",
            "--project",
            project,
            "--quiet",
        ],
        check=False,
    )
    gcloud(
        [
            "compute",
            "networks",
            "delete",
            names["network"],
            "--project",
            project,
            "--quiet",
        ],
        check=False,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", default="secops1219")
    parser.add_argument(
        "--run-id",
        default=datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S"),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="packet output; defaults to manifests/migration/gcp-local-quorum-<run-id>.json",
    )
    args = parser.parse_args()
    if not re.fullmatch(r"[a-z0-9-]+", args.run_id):
        parser.error("run-id must contain lowercase letters, digits, and hyphens")
    if len(args.run_id) > 24:
        parser.error("run-id is too long for GCP resource names")
    root = Path(__file__).resolve().parents[1]
    output = args.output or root / "manifests" / "migration" / f"gcp-local-quorum-{args.run_id}.json"
    names = resource_names(args.run_id)
    instance_pairs = [
        (f"justai-lq-{args.run_id}-{suffix}", zone)
        for suffix, zone in zip(("a", "b", "c"), ZONES)
    ]
    if len(instance_pairs) != MINIMUM_INSTANCE_COUNT:
        parser.error("exactly three instance zones are required")
    with tempfile.TemporaryDirectory(prefix=f"justai-gcp-self-verify-{args.run_id}-") as directory:
        scratch = Path(directory)
        archive = scratch / "justai-bundle.tar"
        expected_commit, artifact_digest = make_archive(root, archive)
        instance_outputs: list[Path] = []
        signature_outputs: list[Path] = []
        created: list[tuple[str, str]] = []
        try:
            print(f"target_commit={expected_commit}")
            print(f"artifact_digest={artifact_digest}")
            print(f"project={args.project}")
            print(f"run_id={args.run_id}")
            create_network(args.project, names)
            for name, zone in instance_pairs:
                create_instance(args.project, names["network"], names["subnet"], name, zone, args.run_id)
                created.append((name, zone))
                wait_for_ssh(args.project, name, zone)
                copy_to_instance(args.project, name, zone, archive, "~/justai-bundle.tar")
                run_instance(
                    args.project,
                    name,
                    zone,
                    expected_commit,
                    artifact_digest,
                    instance_id(args.project, name, zone),
                )
                local_output = scratch / f"{name}-instance.json"
                copy_from_instance(args.project, name, zone, "~/justai-self-verify/instance.json", local_output)
                instance_outputs.append(local_output)
                print(f"instance_passed={name}")
            instance_documents = [read_json(path) for path in instance_outputs]
            instances = [document["instance"] for document in instance_documents]
            for document in instance_documents:
                instance_summary = {
                    "instance_id": document["instance"].get("instance_id"),
                    "status": document["instance"].get("status"),
                    "checks": [
                        {
                            "name": check.get("name"),
                            "status": check.get("status"),
                            "returncode": check.get("returncode"),
                        }
                        for check in document.get("checks", [])
                    ],
                }
                print("instance_result=" + json.dumps(instance_summary, sort_keys=True))
            if any(instance["status"] != "pass" for instance in instances):
                for document in instance_documents:
                    failed_checks = [
                        {
                            "name": check["name"],
                            "returncode": check["returncode"],
                            "output_digest": check["output_digest"],
                            "failure_excerpt": check.get("failure_excerpt", ""),
                        }
                        for check in document.get("checks", [])
                        if check.get("status") != "pass"
                    ]
                    if failed_checks:
                        print(
                            f"instance_failed={document['instance']['instance_id']} "
                            f"checks={json.dumps(failed_checks, sort_keys=True)}"
                        )
                raise RuntimeError("at least one GCP self-verification instance failed")
            if len({instance["observed_result_digest"] for instance in instances}) != 1:
                raise RuntimeError("GCP self-verification instances did not agree on result digest")
            record_id = f"gcp-local-quorum-{args.run_id}"
            basis = packet_basis(expected_commit, artifact_digest, record_id, instances)
            packet_digest = content_digest(basis)
            print(f"packet_content_digest={packet_digest}")
            for name, zone in instance_pairs:
                sign_instance(args.project, name, zone, packet_digest)
                local_signature = scratch / f"{name}-signature.json"
                copy_from_instance(args.project, name, zone, "~/justai-self-verify/signature.json", local_signature)
                signature_outputs.append(local_signature)
                print(f"instance_signed={name}")
            aggregate(
                argparse.Namespace(
                    expected_commit=expected_commit,
                    artifact_digest=artifact_digest,
                    record_id=record_id,
                    instance_outputs=instance_outputs,
                    signature_outputs=signature_outputs,
                    output=output,
                )
            )
            packet = read_json(output)
            validate_packet(packet)
            if packet["content_digest"] != packet_digest:
                raise RuntimeError("final GCP packet digest changed after signed attestations")
            print(f"packet_validated={output}")
            print("publication_eligible=false")
            print("independent_review=false")
            return 0
        finally:
            cleanup(args.project, names, created)
            print("gcp_ephemeral_cleanup_complete=true")


if __name__ == "__main__":
    raise SystemExit(main())
