"""Deterministic, proposal-only governed discovery orchestration.

The loop accepts model output as data. It never executes a model, selects an
evaluator, changes policy, writes an evidence ledger, chooses custody, signs,
releases, or authorizes an external action.
"""

from __future__ import annotations

import copy
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
from typing import Any, Mapping, Sequence

from tools.local_quorum import validate_packet as validate_local_quorum_packet


STATE_SLICE = "justai-discovery-v1"
GOVERNED_STATE_SLICE = "justai-governed-discovery-monorepo-v1"
SELF_VERIFICATION_MODE = "local-quorum"
EXTERNAL_VERIFICATION_MODE = "external-independent"
MIN_SELF_VERIFICATION_INSTANCES = 3
MIN_SELF_VERIFICATION_MACHINES = 2
ALLOWED_DISPOSITIONS = {"promote", "unknown", "quarantine"}
KNOWN_STATE_SLICES = {
    "justai-governed-discovery-monorepo-v1",
    "justai-contracts-v1",
    "justai-discovery-v1",
    "justai-continual-learning-v1",
    "justai-custody-v1",
    "justai-platform-v1",
    "justai-operations-v1",
    "justai-infrastructure-v1",
    "justai-local-quorum-v1",
}
KNOWN_CLAIM_CEILINGS = {
    "source-description-only",
    "bounded-description-only",
    "locator-only-development-pilot",
    "reviewer-identity-only",
    "identity-and-role-description-only",
    "local-proposal",
    "local-development-only",
    "bounded-public-observatory",
    "local-replay",
    "engineering-evidence",
    "independent-review-pending",
    "scientific-evidence",
    "production-authorized",
}
CLAIM_CEILING_RANKS = {
    "source-description-only": 0,
    "bounded-description-only": 0,
    "locator-only-development-pilot": 0,
    "reviewer-identity-only": 0,
    "identity-and-role-description-only": 0,
    "local-proposal": 1,
    "local-development-only": 1,
    "bounded-public-observatory": 1,
    "local-replay": 2,
    "engineering-evidence": 3,
    "independent-review-pending": 4,
    "scientific-evidence": 5,
    "production-authorized": 6,
}
LOCAL_MAX_CLAIM_CEILING = "local-replay"
FORBIDDEN_MODEL_FIELDS = {
    "evaluator",
    "evaluator_id",
    "validator",
    "validator_id",
    "policy",
    "evidence_ledger",
    "custody_root",
    "signer",
    "release_authority",
    "kill_path",
    "state_slice",
    "claim_ceiling",
    "disposition",
    "external_seal_ref",
}
SLOW_REASON_FIELDS = (
    ("disagreement", "algorithmic_disagreement"),
    ("unexpected_result", "unexpected_result"),
    ("novel_candidate", "novel_candidate"),
    ("missing_evidence", "missing_evidence"),
    ("stale_source", "stale_or_changed_source"),
    ("irreversible", "irreversible_action"),
    ("human_skill_concern", "human_skill_or_safety_concern"),
    ("ambiguous_rollback", "ambiguous_rollback"),
    ("infrastructure_uncertainty", "infrastructure_uncertainty"),
    ("custody_ambiguous", "custody_ambiguity"),
    ("provenance_ambiguous", "provenance_ambiguity"),
)


class GovernanceError(ValueError):
    """Raised when a proposal or path violates a governed invariant."""


@dataclass(frozen=True)
class FrozenBudgets:
    """Predeclared budgets; results cannot mutate these limits."""

    proposal_count: int = 8
    serendipity_count: int = 3
    intuition_minutes: int = 20
    recovery_attempts: int = 2

    def __post_init__(self) -> None:
        values = (
            self.proposal_count,
            self.serendipity_count,
            self.intuition_minutes,
            self.recovery_attempts,
        )
        if any(type(value) is not int or value <= 0 for value in values):
            raise GovernanceError("predeclared budgets must be positive")


@dataclass(frozen=True)
class GovernedLoopResult:
    path: str
    disposition: str
    records: tuple[dict[str, Any], ...]
    metrics: dict[str, Any]
    slow_reasons: tuple[str, ...]
    retrospective: dict[str, Any]


def canonical_json(value: Any) -> bytes:
    """Serialize JSON data deterministically for local engineering records."""

    def ensure_supported(value: Any) -> None:
        if isinstance(value, float):
            raise GovernanceError("floating-point JSON is not supported by the cross-language digest contract")
        if isinstance(value, Mapping):
            for key, item in value.items():
                if not isinstance(key, str) or not key.isascii():
                    raise GovernanceError("JSON object keys must be ASCII for the cross-language digest contract")
                ensure_supported(item)
        elif isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
            for item in value:
                ensure_supported(item)

    ensure_supported(value)

    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def result_document(result: GovernedLoopResult) -> dict[str, Any]:
    """Return the immutable, JSON-shaped loop result for local persistence."""

    return {
        "schema": "justai.discovery_loop_result.v1",
        "state_slice": GOVERNED_STATE_SLICE,
        "path": result.path,
        "disposition": result.disposition,
        "slow_reasons": list(result.slow_reasons),
        "records": copy.deepcopy(list(result.records)),
        "metrics": copy.deepcopy(result.metrics),
        "retrospective": copy.deepcopy(result.retrospective),
    }


def persist_result(path: str | Path, result: GovernedLoopResult) -> None:
    """Persist only the caller-provided local result; no accepted ledger write occurs."""

    document = result_document(result)
    for record in document["records"]:
        expected = record.get("content_digest")
        if not isinstance(expected, str) or digest(
            {key: value for key, value in record.items() if key != "content_digest"}
        ) != expected:
            raise GovernanceError(f"cannot persist record with stale content digest: {record.get('record_id')}")
    payload = canonical_json(document) + b"\n"
    destination = Path(path)
    if destination.exists():
        if destination.read_bytes() != payload:
            raise GovernanceError("immutable result persistence refuses overwrite")
        return
    destination.write_bytes(payload)


def _text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise GovernanceError(f"{field} must be a non-empty string")
    return value


def _refs(value: Any, field: str) -> list[str]:
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes)):
        raise GovernanceError(f"{field} must be a string sequence")
    result = [item for item in value if isinstance(item, str) and item.strip()]
    if not result or len(result) != len(value):
        raise GovernanceError(f"{field} must contain non-empty references")
    return result


def _bool(value: Any, field: str) -> bool:
    if type(value) is not bool:
        raise GovernanceError(f"{field} must be a boolean")
    return value


def _confidence(value: Any, field: str) -> int:
    if type(value) is not int or not 0 <= value <= 100:
        raise GovernanceError(f"{field} must be an integer from 0 through 100")
    return value


def _budget_usage(value: Any, field: str, maximum: int, *, minimum: int = 0) -> int:
    if type(value) is not int or not minimum <= value <= maximum:
        raise GovernanceError(f"{field} must be an integer from {minimum} through {maximum}")
    return value


def _sha256(value: Any, field: str) -> str:
    value = _text(value, field)
    if len(value) != 64 or any(character not in "0123456789abcdef" for character in value):
        raise GovernanceError(f"{field} must be lowercase SHA-256")
    return value


def _git_commit(value: Any, field: str) -> str:
    value = _text(value, field)
    if len(value) != 40 or any(character not in "0123456789abcdef" for character in value):
        raise GovernanceError(f"{field} must be a lowercase 40-character Git commit")
    return value


def _optional_text(value: Any, field: str) -> str | None:
    if value is None:
        return None
    return _text(value, field)


def _optional_bool(value: Any, field: str) -> bool | None:
    if value is None:
        return None
    return _bool(value, field)


def _provenance(value: Sequence[Mapping[str, Any]]) -> list[dict[str, Any]]:
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes)) or not value:
        raise GovernanceError("provenance must not be empty")
    result: list[dict[str, Any]] = []
    for index, item in enumerate(value):
        if not isinstance(item, Mapping):
            raise GovernanceError(f"provenance[{index}] must be an object")
        source_path = _text(item.get("source_path"), "source_path")
        if source_path.startswith("/") or ".." in source_path:
            raise GovernanceError("source_path must be repository-relative")
        result.append(
            {
                "source_repository": _text(item.get("source_repository"), "source_repository"),
                "source_commit": _git_commit(item.get("source_commit"), "source_commit"),
                "source_path": source_path,
                "source_digest": _sha256(item.get("source_digest"), "source_digest"),
                "license_basis": _text(item.get("license_basis"), "license_basis"),
                "custody_ref": _text(item.get("custody_ref"), "custody_ref"),
            }
        )
    return result


def _meta(
    *,
    state_slice: str,
    proposal_digest: str,
    claim_ceiling: str,
    provenance: list[dict[str, Any]],
    disposition: str,
    evidence_refs: list[str],
) -> dict[str, Any]:
    if state_slice not in KNOWN_STATE_SLICES:
        raise GovernanceError("state_slice must be an exact JustAI state slice")
    if len(proposal_digest) != 64 or any(c not in "0123456789abcdef" for c in proposal_digest):
        raise GovernanceError("proposal_digest must be lowercase SHA-256")
    _text(claim_ceiling, "claim_ceiling")
    if claim_ceiling not in KNOWN_CLAIM_CEILINGS:
        raise GovernanceError(f"unknown claim ceiling: {claim_ceiling}")
    if CLAIM_CEILING_RANKS[claim_ceiling] > CLAIM_CEILING_RANKS[LOCAL_MAX_CLAIM_CEILING]:
        raise GovernanceError("local governed loop cannot emit a claim above local-replay")
    if disposition not in ALLOWED_DISPOSITIONS | {
        "proposed",
        "sampled",
        "reviewed",
        "dissent",
        "retrospective",
    }:
        raise GovernanceError(f"unsupported disposition: {disposition}")
    return {
        "state_slice": state_slice,
        "proposal_digest": proposal_digest,
        "claim_ceiling": claim_ceiling,
        "provenance": provenance,
        "disposition": disposition,
        "evidence_refs": _refs(evidence_refs, "evidence_refs"),
        "content_digest": "",
    }


def _record(fields: Mapping[str, Any]) -> dict[str, Any]:
    record = dict(fields)
    record["content_digest"] = ""
    record["content_digest"] = digest(
        {key: value for key, value in record.items() if key != "content_digest"}
    )
    return record


def _slow_reasons(
    proposal: Mapping[str, Any],
    validation: Mapping[str, Any],
    validation_receipt: Mapping[str, Any],
) -> list[str]:
    reasons: list[str] = []
    for source, reason in SLOW_REASON_FIELDS:
        proposal_flag = _bool(proposal[source], f"proposal.{source}") if source in proposal else False
        validation_flag = _bool(validation[source], f"validation.{source}") if source in validation else False
        if proposal_flag or validation_flag:
            reasons.append(reason)
    if not _bool(proposal.get("reversible"), "proposal.reversible"):
        reasons.append("irreversible_action")
    if validation_receipt["verification_mode"] == SELF_VERIFICATION_MODE:
        if not validation_receipt["self_verified"]:
            reasons.append("multi_instance_self_verification_missing")
    elif not _bool(validation.get("independent"), "validation.independent"):
        reasons.append("independent_validation_missing")
    if _confidence(proposal.get("confidence"), "proposal.confidence") < 80:
        reasons.append("confidence_below_fast_path_threshold")
    return list(dict.fromkeys(reasons))


def _assert_model_proposal(proposal: Mapping[str, Any]) -> None:
    if proposal.get("model_role") != "proposal-only":
        raise GovernanceError("model output must have model_role=proposal-only")
    forbidden = sorted(FORBIDDEN_MODEL_FIELDS.intersection(proposal))
    if forbidden:
        raise GovernanceError(f"model cannot select governed fields: {', '.join(forbidden)}")


def _candidate_records(
    alternatives: Sequence[Mapping[str, Any]], alternative_refs: Sequence[str]
) -> list[dict[str, str]]:
    if not alternatives or not isinstance(alternatives, Sequence):
        raise GovernanceError("alternatives must be a non-empty sequence")
    if len(alternatives) < 2:
        raise GovernanceError("at least two distinct alternatives are required")
    candidates: list[dict[str, str]] = []
    seen: set[str] = set()
    for index, candidate in enumerate(alternatives):
        if not isinstance(candidate, Mapping):
            raise GovernanceError(f"alternatives[{index}] must be an object")
        candidate_id = _text(candidate.get("id"), f"alternatives[{index}].id")
        if candidate_id in seen:
            raise GovernanceError("alternative candidate ids must be unique")
        seen.add(candidate_id)
        candidates.append({"id": candidate_id, "digest": digest(dict(candidate))})
    if list(alternative_refs) != [candidate["id"] for candidate in candidates]:
        raise GovernanceError("alternative_refs must exactly match sampled candidate ids and order")
    return candidates


def _validation_receipt(validation: Mapping[str, Any], proposal_digest: str) -> dict[str, Any]:
    if not isinstance(validation, Mapping):
        raise GovernanceError("validation must be an object")
    verification_mode = validation.get("verification_mode", EXTERNAL_VERIFICATION_MODE)
    if not isinstance(verification_mode, str) or verification_mode not in {
        EXTERNAL_VERIFICATION_MODE,
        SELF_VERIFICATION_MODE,
    }:
        raise GovernanceError("validation.verification_mode must be external-independent or local-quorum")
    receipt = {
        "record_id": _text(validation.get("record_id"), "validation.record_id"),
        "proposal_digest": _sha256(validation.get("proposal_digest"), "validation.proposal_digest"),
        "accepted": _bool(validation.get("accepted"), "validation.accepted"),
        "independent": _bool(validation.get("independent"), "validation.independent"),
        "verification_mode": verification_mode,
        "validator_identity": _text(
            validation.get("validator_identity"), "validation.validator_identity"
        ),
        "independence_basis": _text(
            validation.get("independence_basis"), "validation.independence_basis"
        ),
    }
    if receipt["proposal_digest"] != proposal_digest:
        raise GovernanceError("validation receipt is not bound to this proposal")
    receipt["self_verified"] = False
    if verification_mode == SELF_VERIFICATION_MODE:
        if receipt["independent"]:
            raise GovernanceError("local-quorum validation must not claim independent review")
        if _sha256(validation.get("local_quorum_proposal_digest"), "local_quorum_proposal_digest") != proposal_digest:
            raise GovernanceError("local-quorum validation is not bound to this proposal")
        packet_digest = _sha256(
            validation.get("local_quorum_packet_digest"), "local_quorum_packet_digest"
        )
        packet = validation.get("local_quorum_packet")
        if not isinstance(packet, Mapping):
            raise GovernanceError("local-quorum validation requires its signed packet")
        try:
            validated_packet = validate_local_quorum_packet(copy.deepcopy(dict(packet)))
        except (OSError, ValueError) as error:
            raise GovernanceError("local-quorum signed packet is invalid") from error
        if validated_packet["content_digest"] != packet_digest:
            raise GovernanceError("local-quorum packet digest is not bound to its signed packet")
        instance_ids = _refs(validation.get("local_quorum_instance_ids"), "local_quorum_instance_ids")
        machine_digests = [
            _sha256(item, "local_quorum_machine_digest")
            for item in _refs(
                validation.get("local_quorum_machine_digests"), "local_quorum_machine_digests"
            )
        ]
        if len(instance_ids) < MIN_SELF_VERIFICATION_INSTANCES or len(set(instance_ids)) != len(instance_ids):
            raise GovernanceError("local-quorum validation requires three distinct instances")
        if len(machine_digests) < MIN_SELF_VERIFICATION_MACHINES or len(set(machine_digests)) < MIN_SELF_VERIFICATION_MACHINES:
            raise GovernanceError("local-quorum validation requires two distinct machine digests")
        packet_instance_ids = [item["instance_id"] for item in validated_packet["instances"]]
        packet_machine_digests = [item["machine_digest"] for item in validated_packet["instances"]]
        if instance_ids != packet_instance_ids or machine_digests != packet_machine_digests:
            raise GovernanceError("local-quorum validation references do not match its signed packet")
        receipt.update(
            {
                "local_quorum_packet_digest": packet_digest,
                "local_quorum_instance_ids": instance_ids,
                "local_quorum_machine_digests": machine_digests,
                "self_verified": True,
            }
        )
    if "human_override" in validation:
        receipt["human_override"] = _bool(validation["human_override"], "validation.human_override")
    return {**receipt, "receipt_digest": digest(receipt)}


def _metric_dimensions(
    *,
    proposal: Mapping[str, Any],
    validation: Mapping[str, Any],
    disposition: str,
    slow_reasons: Sequence[str],
    budgets: FrozenBudgets,
    proposal_count_used: int,
    recovery_attempts_used: int,
    verification_mode: str,
) -> dict[str, Any]:
    confidence = _confidence(proposal.get("confidence"), "proposal.confidence")
    return {
        "outcome_quality": "not_assessed" if disposition != "promote" else "locally_promoted",
        "cost_and_latency": {
            "proposal_count": proposal_count_used,
            "proposal_count_budget": budgets.proposal_count,
            "serendipity_count": budgets.serendipity_count,
            "serendipity_count_used": len(_refs(proposal.get("alternative_refs"), "alternative_refs")),
            "intuition_minutes_budget": budgets.intuition_minutes,
            "recovery_attempts": recovery_attempts_used,
            "recovery_attempts_budget": budgets.recovery_attempts,
        },
        "novelty_yield": int(_bool(proposal.get("novel_candidate"), "proposal.novel_candidate"))
        if "novel_candidate" in proposal
        else 0,
        "candidate_diversity": len(_refs(proposal.get("alternative_refs"), "alternative_refs")),
        "calibration": {"confidence_percent": confidence, "scale": "0-100"},
        "useful_human_overrides": int(
            disposition == "promote"
            and _bool(validation["human_override"], "validation.human_override")
        )
        if "human_override" in validation
        else 0,
        "skill_retention": "manual-intuition-required",
        "recovery_success": None,
        "reversibility": _bool(proposal.get("reversible"), "proposal.reversible"),
        "unresolved_unknown_rate": int(disposition == "unknown"),
        "disagreement_preserved_and_resolved": {
            "preserved": "algorithmic_disagreement" in slow_reasons
            or len(proposal.get("alternative_refs", [])) > 1,
            "resolved": "algorithmic_disagreement" in slow_reasons and disposition == "promote",
        },
        "common_mode_failure_exposure": (
            "same-operator multi-instance self-verification; external independence absent"
            if verification_mode == SELF_VERIFICATION_MODE
            else "not_collapsed; evaluator and validator remain external"
        ),
    }


def run_governed_loop(
    *,
    proposal: Mapping[str, Any],
    alternatives: Sequence[Mapping[str, Any]],
    intuition: Mapping[str, Any],
    validation: Mapping[str, Any],
    provenance: Sequence[Mapping[str, Any]],
    human_disposition: str | None = None,
    state_slice: str = STATE_SLICE,
    claim_ceiling: str = "local-replay",
    budgets: FrozenBudgets | None = None,
    proposal_count_used: int = 1,
    recovery_attempts_used: int = 0,
) -> GovernedLoopResult:
    """Run one bounded loop and return immutable, evidence-bearing records."""

    _assert_model_proposal(proposal)
    proposal_text = _text(proposal.get("algorithmic_proposal"), "algorithmic_proposal")
    alternative_refs = _refs(proposal.get("alternative_refs"), "alternative_refs")
    budgets = budgets or FrozenBudgets()
    proposal_count_used = _budget_usage(
        proposal_count_used,
        "proposal_count_used",
        budgets.proposal_count,
        minimum=1,
    )
    recovery_attempts_used = _budget_usage(
        recovery_attempts_used,
        "recovery_attempts_used",
        budgets.recovery_attempts,
    )
    if not isinstance(alternatives, Sequence) or isinstance(alternatives, (str, bytes)):
        raise GovernanceError("alternatives must be a non-empty sequence")
    if len(alternatives) > budgets.serendipity_count:
        raise GovernanceError("serendipity candidates exceed the predeclared budget")
    candidate_records = _candidate_records(alternatives, alternative_refs)
    reversible = _bool(proposal.get("reversible"), "proposal.reversible")
    proposal_confidence = _confidence(proposal.get("confidence"), "proposal.confidence")
    provenance_data = _provenance(provenance)
    proposal_digest = digest(
        {key: value for key, value in proposal.items() if key != "proposal_digest"}
    )
    validation_receipt = _validation_receipt(validation, proposal_digest)
    validation_id = validation_receipt["record_id"]
    if human_disposition is not None and human_disposition not in ALLOWED_DISPOSITIONS:
        raise GovernanceError("human_disposition must be promote, unknown, or quarantine")
    evidence_refs = [validation_id, *alternative_refs]
    decision = _record(
        {
            "schema": "justai.decision.v1",
            "record_id": _text(proposal.get("record_id", "decision-1"), "proposal.record_id"),
            "created_at": _text(proposal.get("created_at"), "proposal.created_at"),
            **_meta(
                state_slice=state_slice,
                proposal_digest=proposal_digest,
                claim_ceiling=claim_ceiling,
                provenance=provenance_data,
                disposition="proposed",
                evidence_refs=evidence_refs,
            ),
            "algorithmic_proposal": proposal_text,
            "model_role": "proposal-only",
            "reversible": reversible,
            "confidence": proposal_confidence,
            "alternative_refs": alternative_refs,
            "requested_action": _text(proposal.get("requested_action"), "requested_action"),
        }
    )
    serendipity = _record(
        {
            "schema": "justai.serendipity.v1",
            "record_id": "serendipity-1",
            "created_at": decision["created_at"],
            **_meta(
                state_slice=state_slice,
                proposal_digest=proposal_digest,
                claim_ceiling=claim_ceiling,
                provenance=provenance_data,
                disposition="sampled",
                evidence_refs=[decision["record_id"], *alternative_refs],
            ),
            "sampling_method": "caller-supplied-alternative-sample-under-locked-budget",
            "sampling_authority": "caller-supplied; no model or local selector",
            "candidate_refs": alternative_refs,
            "preserved_alternatives": alternative_refs,
            "candidate_digests": [candidate["digest"] for candidate in candidate_records],
            "budget": f"serendipity_count={budgets.serendipity_count}",
            "selected_candidate_ref": None,
        }
    )
    intuition_record = _record(
        {
            "schema": "justai.intuition_review.v1",
            "record_id": _text(intuition.get("record_id", "intuition-1"), "intuition.record_id"),
            "created_at": decision["created_at"],
            **_meta(
                state_slice=state_slice,
                proposal_digest=proposal_digest,
                claim_ceiling=claim_ceiling,
                provenance=provenance_data,
                disposition="reviewed",
                evidence_refs=[decision["record_id"], serendipity["record_id"]],
            ),
            "hypothesis": _text(intuition.get("hypothesis"), "intuition.hypothesis"),
            "confidence": _confidence(intuition.get("confidence"), "intuition.confidence"),
            "observed_evidence": _refs(intuition.get("observed_evidence"), "observed_evidence"),
            "rejected_alternatives": _refs(
                intuition.get("rejected_alternatives"), "rejected_alternatives"
            ),
            "recommended_action_or_restraint": _text(
                intuition.get("recommended_action_or_restraint"),
                "recommended_action_or_restraint",
            ),
            "reviewer_identity": _text(intuition.get("reviewer_identity"), "reviewer_identity"),
            "later_outcome": _optional_text(intuition.get("later_outcome"), "intuition.later_outcome"),
            "intuition_added_value": _optional_bool(
                intuition.get("intuition_added_value"), "intuition.intuition_added_value"
            ),
            "human_disposition": human_disposition,
        }
    )
    if (
        validation_receipt["independent"]
        and validation_receipt["validator_identity"] == intuition_record["reviewer_identity"]
    ):
        raise GovernanceError("independent validator must differ from the intuition reviewer")
    slow_reasons = _slow_reasons(proposal, validation, validation_receipt)
    if human_disposition is not None:
        slow_reasons.append("explicit_human_disposition")
    fast = not slow_reasons and human_disposition is None and validation_receipt["accepted"]
    if fast:
        path = "fast"
        disposition = "promote"
    else:
        path = "slow"
        if human_disposition is None:
            disposition = "unknown"
        else:
            disposition = human_disposition
        if disposition == "promote" and not (
            validation_receipt["accepted"]
            and (validation_receipt["independent"] or validation_receipt["self_verified"])
        ):
            disposition = "unknown"
            slow_reasons.append("promotion_requires_accepted_validation")
    dissent_record = None
    if "algorithmic_disagreement" in slow_reasons:
        dissent_record = _record(
            {
                "schema": "justai.dissent.v1",
                "record_id": "dissent-1",
                "created_at": decision["created_at"],
                **_meta(
                    state_slice=state_slice,
                    proposal_digest=proposal_digest,
                    claim_ceiling=claim_ceiling,
                    provenance=provenance_data,
                    disposition="dissent",
                    evidence_refs=[decision["record_id"], serendipity["record_id"], validation_id],
                ),
                "dissenting_reviewer": _text(
                    validation.get(
                        "dissenting_reviewer",
                        "local-quorum-instance"
                        if validation_receipt["verification_mode"] == SELF_VERIFICATION_MODE
                        else "independent-validator",
                    ),
                    "dissenting_reviewer",
                ),
                "reason": "algorithmic and validation outcomes disagree",
                "alternatives_preserved": alternative_refs,
                "resolution": disposition if disposition != "unknown" else None,
                "resolution_evidence_refs": [validation_id],
            }
        )
    promotion = _record(
        {
            "schema": "justai.promotion_decision.v1",
            "record_id": "promotion-1",
            "created_at": decision["created_at"],
            **_meta(
                state_slice=state_slice,
                proposal_digest=proposal_digest,
                claim_ceiling=claim_ceiling,
                provenance=provenance_data,
                disposition=disposition,
                evidence_refs=[decision["record_id"], intuition_record["record_id"], validation_id],
            ),
            "decision": disposition,
            "reason": "; ".join(slow_reasons) if slow_reasons else "fast-path eligibility satisfied",
            "validator_identity": _text(
                validation.get(
                    "validator_identity",
                    "local-quorum-aggregator"
                    if validation_receipt["verification_mode"] == SELF_VERIFICATION_MODE
                    else "external-validator",
                ),
                "validator_identity",
            ),
            "validator_independence": _text(
                validation.get(
                    "independence_basis",
                    "same-operator multi-instance self-verification"
                    if validation_receipt["verification_mode"] == SELF_VERIFICATION_MODE
                    else "caller-supplied independent validation",
                ),
                "independence_basis",
            ),
                "human_disposition_ref": intuition_record["record_id"]
                if human_disposition is not None
                else None,
        }
    )
    unknown_record = None
    if disposition == "unknown":
        unknown_record = _record(
            {
                "schema": "justai.unknown.v1",
                "record_id": "unknown-1",
                "created_at": decision["created_at"],
                **_meta(
                    state_slice=state_slice,
                    proposal_digest=proposal_digest,
                    claim_ceiling=claim_ceiling,
                    provenance=provenance_data,
                    disposition="unknown",
                    evidence_refs=[promotion["record_id"], *alternative_refs],
                ),
                "unknown_reason": "; ".join(slow_reasons) or "human disposition absent",
                "missing_evidence": [f"validation:{validation_id}"],
                "next_review_condition": (
                    "local quorum packet and human reviewer provide new evidence"
                    if validation_receipt["verification_mode"] == SELF_VERIFICATION_MODE
                    else "independent validator and human reviewer provide new evidence"
                ),
                "preserved_alternatives": alternative_refs,
                "owner": "human-review-queue",
            }
        )
    evidence_manifest = _record(
        {
            "schema": "justai.evidence_manifest.v1",
            "record_id": "evidence-manifest-1",
            "created_at": decision["created_at"],
            **_meta(
                state_slice="justai-custody-v1",
                proposal_digest=proposal_digest,
                claim_ceiling=claim_ceiling,
                provenance=provenance_data,
                disposition="reviewed",
                evidence_refs=[decision["record_id"], promotion["record_id"]],
            ),
            "artifact_refs": [decision["record_id"], serendipity["record_id"], intuition_record["record_id"]],
            "source_commit_refs": [item["source_commit"] for item in provenance_data],
            "custody_root_ref": "externally-supplied-custody-root",
            "integrity_status": "digest-bound-local-records",
            "excluded_artifacts": ["models", "weights", "raw-traces", "credentials", "external-corpora"],
        }
    )
    integration_records = [decision, serendipity, intuition_record]
    if dissent_record is not None:
        integration_records.append(dissent_record)
    integration_records.extend([promotion, evidence_manifest])
    if unknown_record is not None:
        integration_records.append(unknown_record)
    integration = _record(
        {
            "schema": "justai.integration_result.v1",
            "record_id": "integration-1",
            "created_at": decision["created_at"],
            **_meta(
                state_slice=GOVERNED_STATE_SLICE,
                proposal_digest=proposal_digest,
                claim_ceiling=claim_ceiling,
                provenance=provenance_data,
                disposition="retrospective",
                evidence_refs=[record["record_id"] for record in integration_records],
            ),
            "phase_policy": "model-proposal-only; external gates remain closed",
            "math": "bounded algorithmic proposal and deterministic path classification",
            "linguistics": "intuition and dissent remain falsifiable records",
            "evidence": "local engineering evidence only; independent authority absent",
            "observed_ids": [record["record_id"] for record in integration_records],
            "required_digest_refs": [
                proposal_digest,
                validation_receipt["receipt_digest"],
                *[item["source_digest"] for item in provenance_data],
            ],
            "nonclaims": [
                "not independent review",
                "not scientific evidence",
                "not production readiness",
                "not authorization",
                "not profitability",
                "not hosted enforcement",
            ],
            "external_seal_ref": None,
        }
    )
    integration_records.append(integration)
    metrics = _metric_dimensions(
        proposal=proposal,
        validation=validation,
        disposition=disposition,
        slow_reasons=slow_reasons,
        budgets=budgets,
        proposal_count_used=proposal_count_used,
        recovery_attempts_used=recovery_attempts_used,
        verification_mode=validation_receipt["verification_mode"],
    )
    retrospective = {
        "later_outcome": _optional_text(intuition.get("later_outcome"), "intuition.later_outcome"),
        "intuition_added_value": _optional_bool(
            intuition.get("intuition_added_value"), "intuition.intuition_added_value"
        ),
        "disposition_reviewed": disposition,
        "human_disposition_recorded": human_disposition is not None,
        "validation_receipt": copy.deepcopy(validation_receipt),
        "alternatives": copy.deepcopy(candidate_records),
    }
    return GovernedLoopResult(
        path=path,
        disposition=disposition,
        records=tuple(integration_records),
        metrics=metrics,
        slow_reasons=tuple(slow_reasons),
        retrospective=retrospective,
    )
