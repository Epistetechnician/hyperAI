import argparse
import json
import subprocess
import unittest
from dataclasses import replace
from tempfile import TemporaryDirectory
from pathlib import Path

from apps.discovery.src.loop import (
    FrozenBudgets,
    GovernanceError,
    digest,
    persist_result,
    result_document,
    run_governed_loop,
)
from tools.independent_validate import validate_record
from tools.local_quorum import content_digest as quorum_content_digest
from tools.multi_instance_self_verify import aggregate, sign_instance, write_json


PROVENANCE = [
    {
        "source_repository": "ai-discovery-os",
        "source_commit": "057df44f3d3b9c6926fb3fe4e9c9cc0a01ce8d22",
        "source_path": "src/contracts/__init__.py",
        "source_digest": "a" * 64,
        "license_basis": "reference-only; clean-room reimplementation",
        "custody_ref": "manifest:ai-discovery-os",
    }
]


def proposal(**overrides):
    value = {
        "record_id": "decision-1",
        "created_at": "2026-09-09T12:00:00Z",
        "algorithmic_proposal": "retain candidate for independent validation",
        "model_role": "proposal-only",
        "reversible": True,
        "confidence": 95,
        "alternative_refs": ["alternative-a", "alternative-b"],
        "requested_action": "request independent validation",
    }
    value.update(overrides)
    return value


def intuition():
    return {
        "hypothesis": "The less obvious candidate may expose a missed failure mode.",
        "confidence": 70,
        "observed_evidence": ["alternative-b has a distinct failure signature"],
        "rejected_alternatives": ["select only the highest scored candidate"],
        "recommended_action_or_restraint": "preserve alternative-b for review",
        "reviewer_identity": "human-reviewer-1",
    }


def local_quorum_packet(directory: Path):
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
        path = directory / f"instance-{index}.json"
        write_json(path, {"instance": instance})
        instance_paths.append(path)
    packet_core = {
        "schema": "justai.local_quorum.v1",
        "record_id": "local-quorum-loop-test",
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
    packet_digest = quorum_content_digest(packet_core)
    signature_paths = []
    for index in range(1, 4):
        key_path = directory / f"instance-{index}.key"
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(key_path)],
            check=True,
        )
        signature_path = directory / f"signature-{index}.json"
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
    output = directory / "packet.json"
    aggregate(
        argparse.Namespace(
            expected_commit="f" * 40,
            artifact_digest=artifact_digest,
            record_id="local-quorum-loop-test",
            instance_outputs=instance_paths,
            signature_outputs=signature_paths,
            output=output,
        )
    )
    return json.loads(output.read_text(encoding="utf-8"))


def local_quorum_validation(proposed, packet):
    return {
        "record_id": "validation-local-quorum",
        "proposal_digest": digest(proposed),
        "accepted": True,
        "independent": False,
        "verification_mode": "local-quorum",
        "validator_identity": "local-quorum-aggregator",
        "independence_basis": "same-operator multi-instance self-verification",
        "local_quorum_proposal_digest": digest(proposed),
        "local_quorum_packet_digest": packet["content_digest"],
        "local_quorum_packet": packet,
        "local_quorum_instance_ids": ["instance-1", "instance-2", "instance-3"],
        "local_quorum_machine_digests": ["1" * 64, "2" * 64, "1" * 64],
    }


class GovernedLoopTests(unittest.TestCase):
    def test_fast_path_promotes_only_after_independent_validation(self):
        proposed = proposal()
        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition=intuition(),
            validation={
                "record_id": "validation-1",
                "proposal_digest": digest(proposed),
                "accepted": True,
                "independent": True,
                "validator_identity": "validator-external",
                "independence_basis": "separate organization",
            },
            provenance=PROVENANCE,
            budgets=FrozenBudgets(serendipity_count=2),
        )
        self.assertEqual(result.path, "fast")
        self.assertEqual(result.disposition, "promote")
        self.assertEqual(result.metrics["cost_and_latency"]["proposal_count"], 1)
        self.assertEqual(result.metrics["cost_and_latency"]["proposal_count_budget"], 8)
        self.assertEqual(result.metrics["cost_and_latency"]["serendipity_count_used"], 2)
        self.assertEqual(result.metrics["cost_and_latency"]["recovery_attempts"], 0)
        self.assertEqual(result.metrics["cost_and_latency"]["recovery_attempts_budget"], 2)
        self.assertFalse(any(record["schema"] == "justai.dissent.v1" for record in result.records))
        for record in result.records:
            for key in ("state_slice", "proposal_digest", "claim_ceiling", "provenance", "disposition", "evidence_refs"):
                self.assertIn(key, record)

    def test_fast_path_accepts_bounded_local_quorum_without_external_claim(self):
        proposed = proposal()
        with TemporaryDirectory(prefix="justai-local-quorum-loop-") as directory:
            result = run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation=local_quorum_validation(proposed, local_quorum_packet(Path(directory))),
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )
        self.assertEqual(result.path, "fast")
        self.assertEqual(result.disposition, "promote")
        self.assertEqual(
            result.retrospective["validation_receipt"]["verification_mode"],
            "local-quorum",
        )
        self.assertIn("same-operator", result.metrics["common_mode_failure_exposure"])

    def test_local_quorum_is_bound_to_proposal_and_never_claims_independence(self):
        proposed = proposal()
        with TemporaryDirectory(prefix="justai-local-quorum-loop-") as directory:
            packet = local_quorum_packet(Path(directory))
            invalid = local_quorum_validation(proposed, packet)
            invalid["local_quorum_proposal_digest"] = "c" * 64
            with self.assertRaises(GovernanceError):
                run_governed_loop(
                    proposal=proposed,
                    alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                    intuition=intuition(),
                    validation=invalid,
                    provenance=PROVENANCE,
                    budgets=FrozenBudgets(serendipity_count=2),
                )

            invalid = local_quorum_validation(proposed, packet)
            invalid["independent"] = True
            with self.assertRaises(GovernanceError):
                run_governed_loop(
                    proposal=proposed,
                    alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                    intuition=intuition(),
                    validation=invalid,
                    provenance=PROVENANCE,
                    budgets=FrozenBudgets(serendipity_count=2),
                )

    def test_slow_path_preserves_dissent_and_defaults_to_unknown(self):
        proposed = proposal(disagreement=True, novel_candidate=True, reversible=False)
        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition=intuition(),
            validation={
                "record_id": "validation-2",
                "proposal_digest": digest(proposed),
                "accepted": False,
                "independent": True,
                "disagreement": True,
                "validator_identity": "validator-external",
                "independence_basis": "separate organization",
            },
            provenance=PROVENANCE,
            budgets=FrozenBudgets(serendipity_count=2),
        )
        self.assertEqual(result.path, "slow")
        self.assertEqual(result.disposition, "unknown")
        self.assertIn("algorithmic_disagreement", result.slow_reasons)
        self.assertTrue(any(record["schema"] == "justai.dissent.v1" for record in result.records))
        unknown = next(record for record in result.records if record["schema"] == "justai.unknown.v1")
        self.assertEqual(unknown["disposition"], "unknown")
        self.assertEqual(unknown["preserved_alternatives"], ["alternative-a", "alternative-b"])

    def test_slow_path_requires_human_disposition_and_allows_bounded_promotion(self):
        proposed = proposal(novel_candidate=True)
        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition=intuition(),
            validation={
                "record_id": "validation-3",
                "proposal_digest": digest(proposed),
                "accepted": True,
                "independent": True,
                "validator_identity": "validator-external",
                "independence_basis": "separate organization",
            },
            provenance=PROVENANCE,
            human_disposition="promote",
            budgets=FrozenBudgets(serendipity_count=2),
        )
        self.assertEqual(result.path, "slow")
        self.assertEqual(result.disposition, "promote")
        self.assertEqual(result.metrics["novelty_yield"], 1)

    def test_model_cannot_select_governed_authority(self):
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposal(evaluator_id="hidden-evaluator"),
                alternatives=[{"id": "alternative-a"}],
                intuition=intuition(),
                validation={"accepted": True, "independent": True},
                provenance=PROVENANCE,
            )

    def test_intuition_record_persists_with_retrospective_fields(self):
        proposed = proposal(alternative_refs=["alternative-a", "alternative-b"])
        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition={
                **intuition(),
                "later_outcome": "alternative-b exposed a missing case",
                "intuition_added_value": True,
            },
            validation={
                "record_id": "validation-4",
                "proposal_digest": digest(proposed),
                "accepted": True,
                "independent": True,
                "validator_identity": "validator-external",
                "independence_basis": "separate organization",
            },
            provenance=PROVENANCE,
            budgets=FrozenBudgets(serendipity_count=2),
        )
        with TemporaryDirectory() as directory:
            path = Path(directory) / "result.json"
            persist_result(path, result)
            document = json.loads(path.read_text())
        record = next(item for item in document["records"] if item["schema"] == "justai.intuition_review.v1")
        self.assertEqual(record["later_outcome"], "alternative-b exposed a missing case")
        self.assertTrue(record["intuition_added_value"])
        self.assertEqual(document["retrospective"]["intuition_added_value"], True)

    def test_fast_path_honors_human_veto(self):
        proposed = proposal()
        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition=intuition(),
            validation={
                "record_id": "validation-veto",
                "proposal_digest": digest(proposed),
                "accepted": True,
                "independent": True,
                "validator_identity": "validator-external",
                "independence_basis": "separate organization",
            },
            provenance=PROVENANCE,
            human_disposition="quarantine",
            budgets=FrozenBudgets(serendipity_count=2),
        )
        self.assertEqual(result.path, "slow")
        self.assertEqual(result.disposition, "quarantine")
        intuition_record = next(
            record for record in result.records if record["schema"] == "justai.intuition_review.v1"
        )
        self.assertEqual(intuition_record["human_disposition"], "quarantine")

    def test_rejects_coercible_safety_values_and_high_claim_ceiling(self):
        proposed = proposal()
        common = {
            "record_id": "validation-adversarial",
            "proposal_digest": digest(proposed),
            "accepted": True,
            "independent": True,
            "validator_identity": "validator-external",
            "independence_basis": "separate organization",
        }
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposal(reversible="false"),
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation=common,
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )

        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition=intuition(),
            validation=common,
            provenance=PROVENANCE,
            budgets=FrozenBudgets(serendipity_count=2),
        )
        record = dict(result.records[0])
        record["claim_ceiling"] = "scientific-evidence"
        record["content_digest"] = digest(
            {key: value for key, value in record.items() if key != "content_digest"}
        )
        with self.assertRaises(ValueError):
            validate_record(record)
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposal(confidence=101),
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation=common,
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation={**common, "accepted": "true"},
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation=common,
                provenance=PROVENANCE,
                claim_ceiling="production-authorized",
                budgets=FrozenBudgets(serendipity_count=2),
            )

    def test_frozen_budget_usage_rejects_proposal_and_recovery_overruns(self):
        proposed = proposal()
        validation = {
            "record_id": "validation-budget",
            "proposal_digest": digest(proposed),
            "accepted": True,
            "independent": True,
            "validator_identity": "validator-external",
            "independence_basis": "separate organization",
        }
        arguments = {
            "proposal": proposed,
            "alternatives": [{"id": "alternative-a"}, {"id": "alternative-b"}],
            "intuition": intuition(),
            "validation": validation,
            "provenance": PROVENANCE,
            "budgets": FrozenBudgets(proposal_count=1, serendipity_count=2, recovery_attempts=2),
        }
        with self.assertRaises(GovernanceError):
            run_governed_loop(**arguments, proposal_count_used=2)
        with self.assertRaises(GovernanceError):
            run_governed_loop(**arguments, recovery_attempts_used=3)

    def test_independent_validation_requires_distinct_reviewer_identity(self):
        proposed = proposal()
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation={
                    "record_id": "validation-same-reviewer",
                    "proposal_digest": digest(proposed),
                    "accepted": True,
                    "independent": True,
                    "validator_identity": "human-reviewer-1",
                    "independence_basis": "separate organization",
                },
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )

    def test_candidate_diversity_requires_two_distinct_alternatives(self):
        proposed = proposal(alternative_refs=["alternative-a"])
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "alternative-a"}],
                intuition=intuition(),
                validation={
                    "record_id": "validation-diversity",
                    "proposal_digest": digest(proposed),
                    "accepted": True,
                    "independent": True,
                    "validator_identity": "validator-external",
                    "independence_basis": "separate organization",
                },
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )

    def test_validation_and_alternatives_are_bound_to_the_proposal(self):
        proposed = proposal()
        validation = {
            "record_id": "validation-stale",
            "proposal_digest": "0" * 64,
            "accepted": True,
            "independent": True,
            "validator_identity": "validator-external",
            "independence_basis": "separate organization",
        }
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
                intuition=intuition(),
                validation=validation,
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )
        validation["proposal_digest"] = digest(proposed)
        with self.assertRaises(GovernanceError):
            run_governed_loop(
                proposal=proposed,
                alternatives=[{"id": "different-a"}, {"id": "different-b"}],
                intuition=intuition(),
                validation=validation,
                provenance=PROVENANCE,
                budgets=FrozenBudgets(serendipity_count=2),
            )

    def test_result_persistence_isolated_and_immutable(self):
        proposed = proposal()
        result = run_governed_loop(
            proposal=proposed,
            alternatives=[{"id": "alternative-a"}, {"id": "alternative-b"}],
            intuition=intuition(),
            validation={
                "record_id": "validation-persist",
                "proposal_digest": digest(proposed),
                "accepted": True,
                "independent": True,
                "validator_identity": "validator-external",
                "independence_basis": "separate organization",
            },
            provenance=PROVENANCE,
            budgets=FrozenBudgets(serendipity_count=2),
        )
        snapshot = result_document(result)
        snapshot["records"][0]["algorithmic_proposal"] = "tampered snapshot"
        self.assertEqual(result.records[0]["algorithmic_proposal"], proposed["algorithmic_proposal"])
        with TemporaryDirectory() as directory:
            path = Path(directory) / "result.json"
            persist_result(path, result)
            with self.assertRaises(GovernanceError):
                tampered_records = list(result.records)
                tampered_records[0] = {
                    **tampered_records[0],
                    "algorithmic_proposal": "tampered persisted record",
                }
                tampered = replace(result, records=tuple(tampered_records))
                persist_result(path, tampered)


if __name__ == "__main__":
    unittest.main()
