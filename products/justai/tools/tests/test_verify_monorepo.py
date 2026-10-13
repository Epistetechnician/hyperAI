import json
import tempfile
import unittest
from pathlib import Path

from tools.verify_monorepo import verify_source_provenance


SOURCE_NAMES = (
    "ai-discovery-os",
    "aligned-holistic-continual-learning",
    "composed-zk-benchmark-os",
    "security-alignment-os",
    "recursive-meta-harness",
    "composed-platform",
    "statebook",
    "lusis-mesh",
)


def source_entry(name: str, path: str) -> dict:
    return {
        "name": name,
        "path": path,
        "branch": "main",
        "head": "a" * 40,
        "working_tree": "preserved",
        "migration_mode": "reference-only",
        "reference_paths": ["src/module.py"],
        "excluded_material": ["credentials"],
    }


class VerifyMonorepoTests(unittest.TestCase):
    def test_all_source_checkouts_absent_uses_metadata_only_mode(self):
        with tempfile.TemporaryDirectory(prefix="justai-provenance-test-") as directory:
            root = Path(directory)
            (root / "provenance.json").write_text(
                json.dumps({"state_slice": "justai-governed-discovery-monorepo-v1", "references": []}),
                encoding="utf-8",
            )
            source = {
                "source_repositories": [
                    source_entry(name, str(root / f"missing-{index}"))
                    for index, name in enumerate(SOURCE_NAMES)
                ]
            }
            migration = {
                "state_slice": "justai-governed-discovery-monorepo-v1",
                "source_file_provenance_manifest": "provenance.json",
                "capabilities": [],
            }
            verify_source_provenance(root, source, migration)

    def test_partial_source_checkout_availability_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="justai-provenance-test-") as directory:
            root = Path(directory)
            available = root / "source-a"
            available.mkdir()
            source = {
                "source_repositories": [
                    source_entry(
                        name,
                        str(available) if index == 0 else str(root / f"missing-{index}"),
                    )
                    for index, name in enumerate(SOURCE_NAMES)
                ]
            }
            migration = {
                "state_slice": "justai-governed-discovery-monorepo-v1",
                "source_file_provenance_manifest": "provenance.json",
                "capabilities": [],
            }
            (root / "provenance.json").write_text(
                json.dumps({"state_slice": "justai-governed-discovery-monorepo-v1", "references": []}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "availability is partial"):
                verify_source_provenance(root, source, migration)


if __name__ == "__main__":
    unittest.main()
