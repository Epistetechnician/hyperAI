import unittest

from apps.operations.src.recovery import (
    RecoveryError,
    ReplaySafeJournal,
    bind_known_baseline,
    manual_fallback,
    rollback_to_known_baseline,
)
from apps.platform.src.runtime import HostedRuntime, PlatformBoundaryError, ProjectStore


class BoundaryTests(unittest.TestCase):
    def test_replay_safe_journal_and_manual_fallback(self):
        journal = ReplaySafeJournal()
        journal.append("operation-1", "attempt-1", {"value": "bounded"})
        journal.verify()
        restored = ReplaySafeJournal.from_snapshot(journal.snapshot())
        restored.verify()
        with self.assertRaises(RecoveryError):
            journal.append("operation-1", "attempt-1", {"value": "bounded"})
        fallback = manual_fallback("rollback baseline is ambiguous", ["candidate-a", "candidate-b"])
        self.assertEqual(fallback["disposition"], "quarantine")
        self.assertTrue(fallback["manual_required"])
        receipt = journal.receipts[0]
        baseline_entry = bind_known_baseline(receipt.receipt_digest, {"state": "known"})
        baseline = rollback_to_known_baseline(
            receipt.receipt_digest,
            {receipt.receipt_digest: baseline_entry},
            journal,
        )
        self.assertEqual(baseline, {"state": "known"})
        with self.assertRaises(RecoveryError):
            rollback_to_known_baseline("a" * 64, {"a" * 64: baseline_entry}, journal)
        baseline_entry["baseline"]["state"] = "tampered"
        with self.assertRaises(RecoveryError):
            rollback_to_known_baseline(receipt.receipt_digest, {receipt.receipt_digest: baseline_entry}, journal)

    def test_projects_cannot_read_each_other_and_export_is_reproducible(self):
        store = ProjectStore()
        self.assertIsNone(store.create("project-a"))
        self.assertIsNone(store.create("project-b"))
        store.append("project-a", {"record_id": "a-1"})
        store.append("project-b", {"record_id": "b-1"})
        self.assertEqual(store.read("project-a"), [{"record_id": "a-1"}])
        self.assertEqual(store.read("project-b"), [{"record_id": "b-1"}])
        self.assertNotIn(b"b-1", store.export("project-a"))
        self.assertNotIn(b"a-1", store.export("project-a"))
        self.assertIn(b'"record_count":1', store.export("project-a"))
        self.assertEqual(store.export("project-a"), store.export("project-a"))

    def test_hosted_runtime_is_non_authorizing(self):
        runtime = HostedRuntime()
        self.assertFalse(runtime.status()["authorizes_external_execution"])
        with self.assertRaises(PlatformBoundaryError):
            HostedRuntime(authorizes_external_execution=True)
        with self.assertRaises(PlatformBoundaryError):
            runtime.execute({"action": "external"})


if __name__ == "__main__":
    unittest.main()
