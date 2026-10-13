import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.protected_intake import (
    SPKI_PREFIX,
    content_digest,
    publication_eligible,
    signature_payload,
    validate_packet,
)


class ProtectedIntakeTests(unittest.TestCase):
    def make_identity(self, directory: Path, role: str, principal: str, digest: str, purpose: str) -> dict[str, str]:
        private_key = directory / f"{principal}.pem"
        public_key = directory / f"{principal}.der"
        payload = directory / f"{principal}.payload"
        signature = directory / f"{principal}.sig"
        subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(private_key)], check=True)
        subprocess.run(["openssl", "pkey", "-in", str(private_key), "-pubout", "-outform", "DER", "-out", str(public_key)], check=True)
        public_bytes = public_key.read_bytes()
        self.assertTrue(public_bytes.startswith(SPKI_PREFIX))
        payload.write_bytes(signature_payload(digest, purpose))
        subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(private_key), "-in", str(payload), "-out", str(signature)], check=True)
        return {
            "role": role,
            "principal_id": principal,
            "key_id": f"{principal}-key",
            "purpose": purpose,
            "public_key": public_bytes[len(SPKI_PREFIX):].hex(),
            "signature": signature.read_bytes().hex(),
        }

    def valid_packet(self, directory: Path) -> dict:
        bound_records = {
            "evaluation-001": {"schema": "justai.evaluation_receipt.v1", "content_digest": "b" * 64},
            "disclosure-001": {"schema": "justai.disclosure_decision.v1", "content_digest": "c" * 64},
            "projection-001": {"schema": "justai.claim_projection.v1", "content_digest": "d" * 64},
            "review-a": {"schema": "justai.review_receipt.v1", "content_digest": "e" * 64},
            "review-b": {"schema": "justai.review_receipt.v1", "content_digest": "f" * 64},
            "manifest-a": {"schema": "justai.syndicate.v1", "content_digest": "1" * 64},
            "manifest-b": {"schema": "justai.syndicate.v1", "content_digest": "2" * 64},
        }
        with tempfile.TemporaryDirectory(dir=directory, prefix="keys-") as key_directory:
            keys = Path(key_directory)
            review_a = self.make_identity(keys, "reviewer", "reviewer-org-a", "e" * 64, "review")
            review_b = self.make_identity(keys, "reviewer", "reviewer-org-b", "f" * 64, "review")
            manifest_a = self.make_identity(keys, "reviewer-manifest", "reviewer-org-a", "1" * 64, "syndicate-manifest")
            manifest_b = self.make_identity(keys, "reviewer-manifest", "reviewer-org-b", "2" * 64, "syndicate-manifest")
            packet = {
                "schema": "justai.protected_intake.v1",
                "record_id": "intake-001",
                "state_slice": "justai-protected-intake-v1",
                "status": "Ready",
                "artifact_digest": "a" * 64,
                "evidence_basis": "externally-controlled-attested",
                "claim_ceiling": "bounded-observation-only",
                "content_digest": "",
                "signature_refs": [],
                "identity_registry": {
                    "control_mode": "external",
                    "registry_ref": "registry-001",
                    "registry_digest": "3" * 64,
                },
                "custody": {
                    "control_mode": "external",
                    "access_class": "restricted",
                    "custodian_id": "external-custodian",
                    "artifact_digest": "a" * 64,
                    "custody_receipt_digest": "4" * 64,
                    "received_at": "2026-09-09T12:00:00Z",
                    "retention_rule": "External custodian retains raw material; public release receives no bytes.",
                },
                "authorization": {
                    "authority_id": "external-authority",
                    "authorization_digest": "5" * 64,
                    "scope": "bounded defensive evaluation",
                    "issued_at": "2026-09-09T12:00:00Z",
                    "expires_at": "2099-01-01T00:00:00Z",
                },
                "witnessed_evaluation": {
                    "evaluation_record_ref": "evaluation-001",
                    "evaluation_record_digest": "b" * 64,
                    "artifact_digest": "a" * 64,
                    "evaluation_scope": "bounded sandbox",
                    "environment_digest": "6" * 64,
                    "runner_digest": "7" * 64,
                    "observed_result_digest": "8" * 64,
                    "witness_attestation_digest": "9" * 64,
                    "started_at": "2026-09-09T13:00:00Z",
                    "completed_at": "2026-09-09T14:00:00Z",
                    "witness_ids": ["witness-a", "witness-b"],
                },
                "bound_records": bound_records,
                "reviewer_manifest_attestations": [
                    {"manifest_ref": "manifest-a", "manifest_digest": "1" * 64, "reviewer_org": "reviewer-org-a", "signature_ref": manifest_a},
                    {"manifest_ref": "manifest-b", "manifest_digest": "2" * 64, "reviewer_org": "reviewer-org-b", "signature_ref": manifest_b},
                ],
                "independent_reviews": [
                    {"review_receipt_ref": "review-a", "review_receipt_digest": "e" * 64, "reviewer_org": "reviewer-org-a", "reviewer_manifest_ref": "manifest-a", "reviewer_manifest_digest": "1" * 64, "result": "accept", "conflict_of_interest": False, "signature_ref": review_a},
                    {"review_receipt_ref": "review-b", "review_receipt_digest": "f" * 64, "reviewer_org": "reviewer-org-b", "reviewer_manifest_ref": "manifest-b", "reviewer_manifest_digest": "2" * 64, "result": "accept", "conflict_of_interest": False, "signature_ref": review_b},
                ],
                "disclosure_decision": {
                    "decision_record_ref": "disclosure-001",
                    "decision_record_digest": "c" * 64,
                    "decision": "aggregate_only",
                    "reason": "Raw restricted material remains withheld.",
                    "public_projection_ref": "projection-001",
                    "public_projection_digest": "d" * 64,
                },
            }
            packet["content_digest"] = content_digest(packet)
            packet["signature_refs"] = [
                self.make_identity(keys, "custodian", "external-custodian", packet["content_digest"], "intake:custodian"),
                self.make_identity(keys, "authorizer", "external-authority", packet["content_digest"], "intake:authorizer"),
                self.make_identity(keys, "witness", "witness-a", packet["content_digest"], "intake:witness"),
                self.make_identity(keys, "witness", "witness-b", packet["content_digest"], "intake:witness"),
            ]
            return packet

    def test_accepts_fully_attested_packet_and_exposes_publication_decision(self):
        with tempfile.TemporaryDirectory(prefix="justai-protected-test-") as directory:
            packet = self.valid_packet(Path(directory))
            self.assertIs(validate_packet(packet), packet)
            self.assertTrue(publication_eligible(packet))

    def test_rejects_broken_digest_and_unresolved_reference(self):
        with tempfile.TemporaryDirectory(prefix="justai-protected-test-") as directory:
            packet = self.valid_packet(Path(directory))
            broken = copy.deepcopy(packet)
            broken["content_digest"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "content digest mismatch"):
                validate_packet(broken)

            unresolved = copy.deepcopy(packet)
            unresolved["disclosure_decision"]["decision_record_ref"] = "missing-disclosure"
            unresolved["content_digest"] = content_digest(unresolved)
            with self.assertRaisesRegex(ValueError, "unresolved disclosure decision reference"):
                validate_packet(unresolved)

    def test_rejects_wrong_reviewer_manifest_and_disclosure_leak(self):
        with tempfile.TemporaryDirectory(prefix="justai-protected-test-") as directory:
            packet = self.valid_packet(Path(directory))
            wrong_manifest = copy.deepcopy(packet)
            wrong_manifest["independent_reviews"][0]["reviewer_manifest_ref"] = "manifest-b"
            wrong_manifest["independent_reviews"][0]["reviewer_manifest_digest"] = "2" * 64
            wrong_manifest["content_digest"] = content_digest(wrong_manifest)
            with self.assertRaisesRegex(ValueError, "reviewer manifest is not bound"):
                validate_packet(wrong_manifest)

            leak = copy.deepcopy(packet)
            leak["disclosure_decision"]["raw_payload"] = "restricted bytes"
            with self.assertRaisesRegex(ValueError, "forbidden disclosure field"):
                validate_packet(leak)


if __name__ == "__main__":
    unittest.main()
