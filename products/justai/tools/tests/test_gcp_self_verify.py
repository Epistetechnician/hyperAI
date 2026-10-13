import unittest

from tools.gcp_self_verify import packet_basis, resource_names


class GcpSelfVerifyTests(unittest.TestCase):
    def test_resource_names_are_run_scoped(self):
        names = resource_names("20260909-170000")
        self.assertEqual(
            names,
            {
                "network": "justai-selfverify-vpc-20260909-170000",
                "subnet": "justai-selfverify-subnet-20260909-170000",
                "firewall": "justai-selfverify-iap-20260909-170000",
            },
        )

    def test_packet_basis_is_local_quorum_metadata_only(self):
        instances = [
            {
                "instance_id": "instance-a",
                "machine_digest": "1" * 64,
                "environment_digest": "2" * 64,
                "runner_digest": "3" * 64,
                "observed_result_digest": "4" * 64,
                "output_digest": "5" * 64,
                "status": "pass",
                "started_at": "2099-01-01T12:00:00Z",
                "completed_at": "2099-01-01T12:01:00Z",
            }
        ]
        packet = packet_basis("f" * 40, "a" * 64, "record-1", instances)
        self.assertEqual(packet["state_slice"], "justai-local-quorum-v1")
        self.assertEqual(packet["claim_ceiling"], "local-replay")
        self.assertEqual(packet["signature_refs"], [])
        self.assertNotIn("raw_payload", packet)


if __name__ == "__main__":
    unittest.main()
