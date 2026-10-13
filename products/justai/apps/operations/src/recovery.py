"""Replay-safe local recovery and explicit manual fallback."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import copy
import hashlib
import json
from typing import Any, Mapping

from apps.discovery.src.loop import canonical_json


class RecoveryError(ValueError):
    """Raised when recovery would replay, fork, or mutate an unknown baseline."""


@dataclass(frozen=True)
class RecoveryReceipt:
    operation_id: str
    attempt_id: str
    state_slice: str
    input_digest: str
    output_digest: str
    previous_receipt_digest: str
    receipt_digest: str


class ReplaySafeJournal:
    """Append-only journal that rejects duplicate attempts and broken chains."""

    def __init__(self, state_slice: str = "justai-operations-v1") -> None:
        if state_slice != "justai-operations-v1":
            raise RecoveryError("recovery journal has an unsupported state slice")
        self.state_slice = state_slice
        self._receipts: list[RecoveryReceipt] = []
        self._attempts: set[str] = set()

    @property
    def receipts(self) -> tuple[RecoveryReceipt, ...]:
        return tuple(self._receipts)

    def append(
        self,
        operation_id: str,
        attempt_id: str,
        payload: Mapping[str, Any],
        output: Mapping[str, Any] | None = None,
    ) -> RecoveryReceipt:
        if not operation_id.strip() or not attempt_id.strip():
            raise RecoveryError("operation and attempt ids must be non-empty")
        if attempt_id in self._attempts:
            raise RecoveryError("replayed attempt")
        input_digest = hashlib.sha256(canonical_json(payload)).hexdigest()
        output_value = output if output is not None else {
            "operation_id": operation_id,
            "attempt_id": attempt_id,
            "payload": payload,
        }
        output_digest = hashlib.sha256(canonical_json(output_value)).hexdigest()
        previous = self._receipts[-1].receipt_digest if self._receipts else "0" * 64
        body = {
            "operation_id": operation_id,
            "attempt_id": attempt_id,
            "state_slice": self.state_slice,
            "input_digest": input_digest,
            "output_digest": output_digest,
            "previous_receipt_digest": previous,
        }
        receipt_digest = hashlib.sha256(canonical_json(body)).hexdigest()
        receipt = RecoveryReceipt(receipt_digest=receipt_digest, **body)
        self._receipts.append(receipt)
        self._attempts.add(attempt_id)
        return receipt

    def snapshot(self) -> bytes:
        """Export the journal for restart-safe replay checks."""

        return canonical_json(
            {
                "schema": "justai.recovery_journal.v1",
                "state_slice": self.state_slice,
                "receipts": [asdict(receipt) for receipt in self._receipts],
            }
        ) + b"\n"

    @classmethod
    def from_snapshot(cls, payload: bytes) -> "ReplaySafeJournal":
        def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
            result: dict[str, Any] = {}
            for key, value in pairs:
                if key in result:
                    raise RecoveryError("duplicate recovery snapshot key")
                result[key] = value
            return result

        try:
            document = json.loads(payload.decode("utf-8"), object_pairs_hook=unique_object)
        except (RecoveryError, UnicodeDecodeError, json.JSONDecodeError) as error:
            raise RecoveryError("invalid recovery journal snapshot") from error
        if not isinstance(document, dict) or document.get("schema") != "justai.recovery_journal.v1":
            raise RecoveryError("invalid recovery journal schema")
        state_slice = document.get("state_slice")
        receipts = document.get("receipts")
        if not isinstance(state_slice, str) or not state_slice.strip() or not isinstance(receipts, list):
            raise RecoveryError("invalid recovery journal snapshot shape")
        journal = cls(state_slice)
        fields = {
            "operation_id", "attempt_id", "state_slice", "input_digest",
            "output_digest", "previous_receipt_digest", "receipt_digest",
        }
        for raw in receipts:
            if not isinstance(raw, dict) or set(raw) != fields:
                raise RecoveryError("invalid recovery receipt shape")
            if any(not isinstance(raw[field], str) for field in fields):
                raise RecoveryError("invalid recovery receipt values")
            try:
                receipt = RecoveryReceipt(**raw)
            except TypeError as error:
                raise RecoveryError("invalid recovery receipt values") from error
            if receipt.attempt_id in journal._attempts:
                raise RecoveryError("replayed attempt")
            journal._receipts.append(receipt)
            journal._attempts.add(receipt.attempt_id)
        journal.verify()
        return journal

    def verify(self) -> None:
        previous = "0" * 64
        for receipt in self._receipts:
            if (
                receipt.state_slice != self.state_slice
                or not receipt.operation_id.strip()
                or not receipt.attempt_id.strip()
                or not all(
                    _is_digest(value)
                    for value in (
                        receipt.input_digest,
                        receipt.output_digest,
                        receipt.previous_receipt_digest,
                        receipt.receipt_digest,
                    )
                )
                or receipt.previous_receipt_digest != previous
            ):
                raise RecoveryError("broken recovery journal chain")
            body = {
                "operation_id": receipt.operation_id,
                "attempt_id": receipt.attempt_id,
                "state_slice": receipt.state_slice,
                "input_digest": receipt.input_digest,
                "output_digest": receipt.output_digest,
                "previous_receipt_digest": receipt.previous_receipt_digest,
            }
            if hashlib.sha256(canonical_json(body)).hexdigest() != receipt.receipt_digest:
                raise RecoveryError("tampered recovery receipt")
            previous = receipt.receipt_digest


def manual_fallback(reason: str, preserved_alternatives: list[str]) -> dict[str, Any]:
    if not isinstance(reason, str) or not reason.strip() or not isinstance(preserved_alternatives, list) or not preserved_alternatives or any(
        not isinstance(value, str) or not value.strip() for value in preserved_alternatives
    ):
        raise RecoveryError("manual fallback requires a reason and alternatives")
    return {
        "state_slice": "justai-operations-v1",
        "disposition": "quarantine",
        "manual_required": True,
        "reason": reason,
        "preserved_alternatives": list(preserved_alternatives),
        "external_execution": "closed",
    }


def rollback_to_known_baseline(
    target_receipt_digest: str,
    known_baselines: Mapping[str, Mapping[str, Any]],
    journal: ReplaySafeJournal,
) -> dict[str, Any]:
    """Select an exact known baseline or fail closed without changing state."""

    journal.verify()
    if not any(receipt.receipt_digest == target_receipt_digest for receipt in journal.receipts):
        raise RecoveryError("rollback receipt is not present in the verified journal")
    entry = known_baselines.get(target_receipt_digest)
    if entry is None:
        raise RecoveryError("rollback baseline is unknown; manual fallback required")
    if entry.get("receipt_digest") != target_receipt_digest:
        raise RecoveryError("rollback baseline receipt binding is invalid")
    baseline = entry.get("baseline")
    baseline_digest = entry.get("baseline_digest")
    if not isinstance(baseline, Mapping) or not isinstance(baseline_digest, str):
        raise RecoveryError("rollback baseline binding is incomplete")
    actual_digest = hashlib.sha256(canonical_json(baseline)).hexdigest()
    if actual_digest != baseline_digest:
        raise RecoveryError("rollback baseline content is tampered")
    return copy.deepcopy(dict(baseline))


def bind_known_baseline(receipt_digest: str, baseline: Mapping[str, Any]) -> dict[str, Any]:
    """Create a digest-bound baseline entry for an independently known receipt."""

    if not _is_digest(receipt_digest) or not isinstance(baseline, Mapping):
        raise RecoveryError("baseline binding requires a receipt digest and object")
    baseline_copy = copy.deepcopy(dict(baseline))
    return {
        "receipt_digest": receipt_digest,
        "baseline_digest": hashlib.sha256(canonical_json(baseline_copy)).hexdigest(),
        "baseline": baseline_copy,
    }


def _is_digest(value: str) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )
