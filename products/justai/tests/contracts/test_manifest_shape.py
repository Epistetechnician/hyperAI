import json
from pathlib import Path
import unittest

from tools.verify_monorepo import validate


class ManifestShapeTests(unittest.TestCase):
    def test_source_and_migration_manifests_are_unique_and_parseable(self):
        root = Path(__file__).resolve().parents[2]
        validate(root)
        source = json.loads((root / "manifests/source-repositories/manifest.json").read_text())
        migration = json.loads((root / "manifests/migration/manifest.json").read_text())
        self.assertEqual(source["schema"], "justai.source_repository_inventory.v1")
        self.assertEqual(migration["state_slice"], "justai-governed-discovery-monorepo-v1")
        owners = [entry["owner"] for entry in migration["capabilities"]]
        self.assertEqual(len(owners), len(set(owners)))
        self.assertEqual(len(source["source_repositories"]), 8)
        self.assertTrue(all(entry["head"] for entry in source["source_repositories"]))
        ai_discovery = next(entry for entry in source["source_repositories"] if entry["name"] == "ai-discovery-os")
        self.assertEqual(
            ai_discovery["current_head_on_followup_review"],
            "38a3d354999a17646a0da47c8751c791db082614",
        )
        self.assertIn(
            ai_discovery["current_head_on_followup_review"],
            ai_discovery["observed_followup_heads"],
        )
        for package in (
            "decision-contracts",
            "evidence-contracts",
            "intuition-review",
            "serendipity",
            "promotion",
            "custody",
            "claim-ceilings",
        ):
            self.assertTrue((root / "packages" / package / "package.json").is_file())
            self.assertTrue((root / "packages" / package / "README.md").is_file())
            self.assertTrue((root / "packages" / package / "src/index.ts").is_file())


if __name__ == "__main__":
    unittest.main()
