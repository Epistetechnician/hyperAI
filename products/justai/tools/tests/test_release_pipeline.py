import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from tools.independent_validate import content_digest, validate_release


class ReleasePipelineTests(unittest.TestCase):
    def assemble_fixture(self, root: Path, input_dir: str, output_dir: Path, release_id: str) -> dict:
        subprocess.run(
            [
                "python3",
                "tools/publish_release.py",
                "--input-dir",
                input_dir,
                "--output-dir",
                str(output_dir),
                "--publisher-id",
                "justai-pilot-syndicate",
                "--created-at",
                "2026-09-09T12:00:00Z",
                "--release-id",
                release_id,
                "--key-id",
                "bridge-test-key",
                "--seed-hex",
                "0" * 64,
            ],
            cwd=root,
            check=True,
        )
        release_files = sorted(output_dir.glob("*/release.json"))
        self.assertEqual(len(release_files), 1)
        release = json.loads(release_files[0].read_text(encoding="utf-8"))
        self.assertEqual(release_files[0].parent.name, release["content_digest"])
        return release

    def test_pilot_assembles_validates_and_renders(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix="justai-pipeline-") as directory:
            output_dir = Path(directory)
            subprocess.run(
                [
                    "python3",
                    "tools/publish_release.py",
                    "--input-dir",
                    "fixtures/pilot/input",
                    "--output-dir",
                    str(output_dir),
                    "--publisher-id",
                    "justai-pilot-syndicate",
                    "--created-at",
                    "2026-09-09T12:00:00Z",
                    "--release-id",
                    "release-pilot-test",
                    "--key-id",
                    "pilot-test-key",
                    "--seed-hex",
                    "0" * 64,
                ],
                cwd=root,
                check=True,
            )
            release_files = sorted(output_dir.glob("*/release.json"))
            self.assertEqual(len(release_files), 1)
            bundle = release_files[0].parent
            release = json.loads(release_files[0].read_text(encoding="utf-8"))
            self.assertEqual(bundle.name, release["content_digest"])
            html = bundle / "index.html"
            self.assertTrue(html.is_file())
            rendered = html.read_text(encoding="utf-8")
            self.assertIn("locator-only-development-pilot", rendered)
            self.assertIn("uncertainty", rendered)
            self.assertIn("Contested", rendered)

    def test_public_and_restricted_bridge_fixtures_share_receipt_shape(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix="justai-bridge-pipeline-") as directory:
            output_root = Path(directory)
            public = self.assemble_fixture(
                root,
                "fixtures/evidence-bridge/public/input",
                output_root / "public",
                "release-bridge-public-test",
            )
            restricted = self.assemble_fixture(
                root,
                "fixtures/evidence-bridge/restricted/input",
                output_root / "restricted",
                "release-bridge-restricted-test",
            )
            public_bridge = {record["schema"]: record for record in public["records"] if record["schema"].startswith("justai.") and "bridge" in record["state_slice"]}
            restricted_bridge = {record["schema"]: record for record in restricted["records"] if record["schema"].startswith("justai.") and "bridge" in record["state_slice"]}
            self.assertEqual(set(public_bridge), set(restricted_bridge))
            for schema in public_bridge:
                self.assertEqual(set(public_bridge[schema]), set(restricted_bridge[schema]))
            public_html = next((output_root / "public").glob("*/index.html")).read_text(encoding="utf-8")
            restricted_html = next((output_root / "restricted").glob("*/index.html")).read_text(encoding="utf-8")
            self.assertIn("open-reproducible", public_html)
            self.assertIn("restricted-attested", restricted_html)
            self.assertIn("Raw restricted bytes", restricted_html)
            self.assertNotIn("raw restricted", public_html.lower())

    def test_bridge_rejects_digest_reference_and_disclosure_failures(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix="justai-bridge-adversarial-") as directory:
            release = self.assemble_fixture(
                root,
                "fixtures/evidence-bridge/restricted/input",
                Path(directory),
                "release-bridge-adversarial-test",
            )

        def record(value: dict, schema: str) -> dict:
            return next(item for item in value["records"] if item["schema"] == schema)

        def refresh(value: dict, record_id: str) -> None:
            target = next(item for item in value["records"] if item["record_id"] == record_id)
            target["content_digest"] = content_digest(target)
            value["content_digest"] = content_digest(value)

        broken_digest = copy.deepcopy(release)
        record(broken_digest, "justai.artifact_manifest.v1")["content_digest"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "content digest mismatch"):
            validate_release(broken_digest)

        wrong_manifest = copy.deepcopy(release)
        review = next(item for item in wrong_manifest["records"] if item["schema"] == "justai.review_receipt.v1")
        review["syndicate_manifest_ref"] = "syndicate-bridge-publisher"
        refresh(wrong_manifest, review["record_id"])
        with self.assertRaisesRegex(ValueError, "reviewer manifest is not bound"):
            validate_release(wrong_manifest)

        unresolved_reference = copy.deepcopy(release)
        projection = record(unresolved_reference, "justai.claim_projection.v1")
        projection["disclosure_decision_refs"] = ["missing-disclosure"]
        refresh(unresolved_reference, projection["record_id"])
        with self.assertRaisesRegex(ValueError, "unknown record reference"):
            validate_release(unresolved_reference)

        disclosure_leak = copy.deepcopy(release)
        artifact = record(disclosure_leak, "justai.artifact_manifest.v1")
        artifact["raw_payload"] = "restricted bytes must not cross the seam"
        refresh(disclosure_leak, artifact["record_id"])
        with self.assertRaisesRegex(ValueError, "forbidden disclosure field"):
            validate_release(disclosure_leak)


if __name__ == "__main__":
    unittest.main()
