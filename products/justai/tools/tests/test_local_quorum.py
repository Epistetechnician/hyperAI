import copy
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.local_quorum import (
    SPKI_PREFIX,
    content_digest,
    publication_eligible,
    signature_payload,
    validate_packet,
)


class LocalQuorumTests(unittest.TestCase):
    def make_identity(self, directory: Path, instance_id: str, digest: str) -> dict[str, str]:
        private_key = directory / f"{instance_id}.pem"
        public_key = directory / f"{instance_id}.der"
        payload = directory / f"{instance_id}.payload"
        signature = directory / f"{instance_id}.sig"
        subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(private_key)], check=True)
        subprocess.run(["openssl", "pkey", "-in", str(private_key), "-pubout", "-outform", "DER", "-out", str(public_key)], check=True)
        public_bytes = public_key.read_bytes()
        self.assertTrue(public_bytes.startswith(SPKI_PREFIX))
        purpose = f"instance:{instance_id}"
        payload.write_bytes(signature_payload(digest, purpose))
        subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(private_key), "-in", str(payload), "-out", str(signature)], check=True)
        return {
            "role": "instance",
            "instance_id": instance_id,
            "principal_id": "local-operator",
            "key_id": f"{instance_id}-key",
            "purpose": purpose,
            "public_key": public_bytes[len(SPKI_PREFIX):].hex(),
            "signature": signature.read_bytes().hex(),
        }

    def valid_packet(self, directory: Path) -> dict:
        packet = {
            "schema": "justai.local_quorum.v1",
            "record_id": "local-quorum-001",
            "state_slice": "justai-local-quorum-v1",
            "status": "Verified",
            "verification_mode": "local-quorum",
            "claim_ceiling": "local-replay",
            "artifact_digest": "a" * 64,
            "evaluation_scope": "bounded deterministic replay",
            "expected_result_digest": "d" * 64,
            "content_digest": "",
            "signature_refs": [],
            "instances": [
                {
                    "instance_id": "instance-a",
                    "machine_digest": "1" * 64,
                    "environment_digest": "4" * 64,
                    "runner_digest": "7" * 64,
                    "observed_result_digest": "d" * 64,
                    "output_digest": "a" * 64,
                    "status": "pass",
                    "started_at": "2099-01-01T12:00:00Z",
                    "completed_at": "2099-01-01T12:01:00Z",
                },
                {
                    "instance_id": "instance-b",
                    "machine_digest": "2" * 64,
                    "environment_digest": "5" * 64,
                    "runner_digest": "8" * 64,
                    "observed_result_digest": "d" * 64,
                    "output_digest": "a" * 64,
                    "status": "pass",
                    "started_at": "2099-01-01T12:00:00Z",
                    "completed_at": "2099-01-01T12:01:00Z",
                },
                {
                    "instance_id": "instance-c",
                    "machine_digest": "3" * 64,
                    "environment_digest": "6" * 64,
                    "runner_digest": "9" * 64,
                    "observed_result_digest": "d" * 64,
                    "output_digest": "a" * 64,
                    "status": "pass",
                    "started_at": "2099-01-01T12:00:00Z",
                    "completed_at": "2099-01-01T12:01:00Z",
                },
            ],
        }
        packet["content_digest"] = content_digest(packet)
        with tempfile.TemporaryDirectory(dir=directory, prefix="keys-") as key_directory:
            keys = Path(key_directory)
            packet["signature_refs"] = [
                self.make_identity(keys, instance["instance_id"], packet["content_digest"])
                for instance in packet["instances"]
            ]
        return packet

    def test_accepts_local_quorum_but_never_publication(self):
        with tempfile.TemporaryDirectory(prefix="justai-local-quorum-test-") as directory:
            packet = self.valid_packet(Path(directory))
            self.assertIs(validate_packet(packet), packet)
            self.assertFalse(publication_eligible(packet))

    def test_rejects_insufficient_machine_diversity_and_mismatched_result(self):
        with tempfile.TemporaryDirectory(prefix="justai-local-quorum-test-") as directory:
            packet = self.valid_packet(Path(directory))

            same_machine = copy.deepcopy(packet)
            same_machine["instances"][1]["machine_digest"] = same_machine["instances"][0]["machine_digest"]
            same_machine["instances"][2]["machine_digest"] = same_machine["instances"][0]["machine_digest"]
            same_machine["content_digest"] = content_digest(same_machine)
            with self.assertRaisesRegex(ValueError, "two distinct machine digests"):
                validate_packet(same_machine)

            wrong_result = copy.deepcopy(packet)
            wrong_result["instances"][1]["observed_result_digest"] = "e" * 64
            wrong_result["content_digest"] = content_digest(wrong_result)
            with self.assertRaisesRegex(ValueError, "does not match expected digest"):
                validate_packet(wrong_result)

    def test_rejects_disclosure_and_external_review_bypass_fields(self):
        with tempfile.TemporaryDirectory(prefix="justai-local-quorum-test-") as directory:
            packet = self.valid_packet(Path(directory))

            leak = copy.deepcopy(packet)
            leak["instances"][0]["raw_payload"] = "restricted bytes"
            with self.assertRaisesRegex(ValueError, "forbidden disclosure field"):
                validate_packet(leak)

            bypass = copy.deepcopy(packet)
            bypass["independent_reviews"] = []
            with self.assertRaisesRegex(ValueError, "unsupported local quorum fields"):
                validate_packet(bypass)

            nested = copy.deepcopy(packet)
            nested["instances"][0]["unlisted_field"] = "rejected"
            nested["content_digest"] = content_digest(nested)
            with self.assertRaisesRegex(ValueError, "unsupported local quorum instance fields"):
                validate_packet(nested)

            nested = copy.deepcopy(packet)
            nested["signature_refs"][0]["unlisted_field"] = "rejected"
            with self.assertRaisesRegex(ValueError, "unsupported local quorum signature fields"):
                validate_packet(nested)

    def test_rejects_broken_content_digest(self):
        with tempfile.TemporaryDirectory(prefix="justai-local-quorum-test-") as directory:
            packet = self.valid_packet(Path(directory))
            packet["content_digest"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "content digest mismatch"):
                validate_packet(packet)


if __name__ == "__main__":
    unittest.main()
