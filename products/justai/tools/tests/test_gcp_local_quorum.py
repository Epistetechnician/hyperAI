import copy
import subprocess
import unittest
from unittest.mock import patch

from tools import gcp_local_quorum as runner


class GcpLocalQuorumTests(unittest.TestCase):
    def make_records(self):
        artifact_digest, result_digest, _source, runner_digest = runner.runner_material()
        names = (
            "justai-lq-test-a",
            "justai-lq-test-b",
            "justai-lq-test-c",
        )
        records = {}
        for index, name in enumerate(names, start=1):
            machine_digest = f"{index:064x}"
            environment_digest = f"{(10 + index):064x}"
            output_payload = (
                f"instance_id={name}|machine_digest={machine_digest}|"
                f"environment_digest={environment_digest}|runner_digest={runner_digest}|"
                f"observed_result_digest={result_digest}|status=pass"
            )
            records[name] = {
                "JUSTAI_MACHINE_DIGEST": machine_digest,
                "JUSTAI_ENVIRONMENT_DIGEST": environment_digest,
                "JUSTAI_RUNNER_DIGEST": runner_digest,
                "JUSTAI_OBSERVED_RESULT_DIGEST": result_digest,
                "JUSTAI_OUTPUT_DIGEST": runner.digest(output_payload),
                "JUSTAI_STATUS": "pass",
                "JUSTAI_STARTED_AT": "2026-09-09T12:00:00Z",
                "JUSTAI_COMPLETED_AT": "2026-09-09T12:00:01Z",
                "JUSTAI_PUBLIC_KEY": f"{(20 + index):064x}",
            }
        return names, artifact_digest, result_digest, runner_digest, records

    def test_parser_extracts_markers_after_gcp_serial_prefix(self):
        output = "\n".join(
            (
                "2026-09-09T12:00:00Z vm google_metadata_script_runner: JUSTAI_INSTANCE_ID=instance-a",
                "2026-09-09T12:00:00Z vm google_metadata_script_runner: JUSTAI_STATUS=pass",
                "2026-09-09T12:00:00Z vm google_metadata_script_runner: JUSTAI_PUBLIC_KEY=" + "a" * 64,
            )
        )
        fields = runner.guest_fields(output)
        self.assertEqual(
            fields,
            {
                "JUSTAI_INSTANCE_ID": "instance-a",
                "JUSTAI_STATUS": "pass",
                "JUSTAI_PUBLIC_KEY": "a" * 64,
            },
        )

    def test_packet_assembly_keeps_digest_stable_when_signatures_are_added(self):
        names, artifact_digest, result_digest, _runner_digest, records = self.make_records()
        pairs = list(zip(names, ("us-east4-a", "us-east4-b", "us-east4-c")))
        unsigned = runner.build_packet(
            "test-run",
            artifact_digest,
            result_digest,
            records,
            pairs,
            "secops1219",
        )
        self.assertEqual(unsigned["signature_refs"], [])

        signed_records = copy.deepcopy(records)
        for index, name in enumerate(names, start=1):
            signed_records[name]["JUSTAI_SIGNATURE"] = f"{(30 + index):0128x}"
        signed = runner.build_packet(
            "test-run",
            artifact_digest,
            result_digest,
            signed_records,
            pairs,
            "secops1219",
        )
        self.assertEqual(unsigned["content_digest"], signed["content_digest"])
        self.assertEqual(len(signed["signature_refs"]), 3)

    @patch.object(
        runner,
        "instance_exists",
        side_effect=[True, True, True, False, False, False],
    )
    @patch.object(
        runner,
        "run_command",
        return_value=subprocess.CompletedProcess([], 0, "", ""),
    )
    def test_cleanup_deletes_only_exact_instances_and_boot_disks(
        self,
        run_command,
        _instance_exists,
    ):
        pairs = [
            ("justai-lq-test-a", "us-east4-a"),
            ("justai-lq-test-b", "us-east4-b"),
            ("justai-lq-test-c", "us-east4-c"),
        ]
        runner.delete_instances(pairs, "secops1219")

        delete_calls = [
            call for call in run_command.call_args_list
            if call.args[0][0:4] == [
                "gcloud",
                "compute",
                "instances",
                "delete",
            ]
        ]
        self.assertEqual(
            {call.args[0][4] for call in delete_calls},
            {pair[0] for pair in pairs},
        )
        self.assertEqual(len(delete_calls), len(pairs))
        for call in delete_calls:
            self.assertIn("--delete-disks", call.args[0])
            self.assertIn("all", call.args[0])

    @patch.object(runner, "instance_exists", side_effect=[True, True, True, True, False, False])
    @patch.object(
        runner,
        "run_command",
        return_value=subprocess.CompletedProcess([], 0, "", ""),
    )
    def test_cleanup_fails_if_an_exact_instance_remains(
        self,
        _run_command,
        _instance_exists,
    ):
        pairs = [
            ("justai-lq-test-a", "us-east4-a"),
            ("justai-lq-test-b", "us-east4-b"),
            ("justai-lq-test-c", "us-east4-c"),
        ]
        with self.assertRaisesRegex(RuntimeError, "instances remain after cleanup"):
            runner.delete_instances(pairs, "secops1219")


if __name__ == "__main__":
    unittest.main()
