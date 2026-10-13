import argparse
import hashlib
import tempfile
import unittest
from pathlib import Path

from tools.source_intake import build_record, canonical_json


class SourceIntakeTests(unittest.TestCase):
    def test_normalizes_local_source_without_network_access(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.txt"
            source.write_bytes(b"public source snapshot")
            args = argparse.Namespace(
                input=str(source),
                record_id="source-001",
                publisher_id="syndicate-001",
                title="Example source",
                url="https://example.test/source",
                accessed_at="2026-09-09T12:00:00Z",
                publication_date="2026-09-09",
                jurisdiction="US",
                source_class="primary",
                license_or_terms="public terms",
                max_bytes=1000,
            )
            record = build_record(args)
            source_digest = hashlib.sha256(b"public source snapshot").hexdigest()
            content = {key: value for key, value in record.items() if key not in {"content_digest", "signature_refs"}}
            content_digest = hashlib.sha256(canonical_json(content)).hexdigest()
            self.assertEqual(record["content_digest"], content_digest)
            self.assertEqual(record["source_digests"], [source_digest])
            self.assertNotEqual(record["content_digest"], source_digest)
            self.assertEqual(record["access_class"], "public")
            self.assertEqual(record["custody_mode"], "reproducible")
            self.assertEqual(record["license_basis"], "public terms")
            self.assertEqual(record["permitted_uses"], ["public-description", "bounded-evaluation"])
            self.assertEqual(canonical_json(record), canonical_json(record))

    def test_rejects_oversized_source(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.txt"
            source.write_bytes(b"12345")
            args = argparse.Namespace(
                input=str(source),
                record_id="source-002",
                publisher_id="syndicate-001",
                title="Example source",
                url="https://example.test/source",
                accessed_at="2026-09-09T12:00:00Z",
                publication_date="2026-09-09",
                jurisdiction="US",
                source_class="primary",
                license_or_terms="public terms",
                max_bytes=4,
            )
            with self.assertRaises(ValueError):
                build_record(args)


if __name__ == "__main__":
    unittest.main()
