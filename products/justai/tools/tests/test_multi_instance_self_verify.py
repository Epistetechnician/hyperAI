import argparse
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.local_quorum import content_digest, validate_packet
from tools.multi_instance_self_verify import aggregate, sign_instance, write_json


class MultiInstanceSelfVerifyTests(unittest.TestCase):
    def test_aggregates_three_distinct_instance_outputs(self):
        with tempfile.TemporaryDirectory(prefix="justai-self-verify-test-") as directory:
            root = Path(directory)
            artifact_digest = "a" * 64
            expected_result_digest = "d" * 64
            instances = []
            instance_paths = []
            for index, machine_digest in enumerate(("1" * 64, "2" * 64, "1" * 64), start=1):
                instance = {
                    "instance_id": f"instance-{index}",
                    "machine_digest": machine_digest,
                    "environment_digest": f"{index}" * 64,
                    "runner_digest": f"{index + 3}" * 64,
                    "observed_result_digest": expected_result_digest,
                    "output_digest": artifact_digest,
                    "status": "pass",
                    "started_at": "2099-01-01T12:00:00Z",
                    "completed_at": "2099-01-01T12:01:00Z",
                }
                instances.append(instance)
                path = root / f"instance-{index}.json"
                write_json(path, {"instance": instance})
                instance_paths.append(path)

            packet_core = {
                "schema": "justai.local_quorum.v1",
                "record_id": "local-quorum-test",
                "state_slice": "justai-local-quorum-v1",
                "status": "Verified",
                "verification_mode": "local-quorum",
                "claim_ceiling": "local-replay",
                "artifact_digest": artifact_digest,
                "evaluation_scope": "target_commit=" + "f" * 40 + ";python-contract-and-boundary-tests-only",
                "expected_result_digest": expected_result_digest,
                "content_digest": "",
                "signature_refs": [],
                "instances": instances,
            }
            packet_digest = content_digest(packet_core)
            signature_paths = []
            for index in range(1, 4):
                key_path = root / f"instance-{index}.key"
                subprocess.run(
                    ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(key_path)],
                    check=True,
                )
                signature_path = root / f"signature-{index}.json"
                sign_instance(
                    argparse.Namespace(
                        key_path=key_path,
                        instance_id=f"instance-{index}",
                        principal_id="local-operator",
                        packet_digest=packet_digest,
                        output=signature_path,
                    )
                )
                signature_paths.append(signature_path)

            output = root / "packet.json"
            aggregate(
                argparse.Namespace(
                    expected_commit="f" * 40,
                    artifact_digest=artifact_digest,
                    record_id="local-quorum-test",
                    instance_outputs=instance_paths,
                    signature_outputs=signature_paths,
                    output=output,
                )
            )
            packet = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(validate_packet(packet), packet)
            self.assertEqual(packet["claim_ceiling"], "local-replay")


if __name__ == "__main__":
    unittest.main()
