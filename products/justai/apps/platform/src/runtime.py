"""Project isolation, reproducible export, and hosted non-authorization."""

from __future__ import annotations

from dataclasses import dataclass, field
import copy
from typing import Any, Mapping

from apps.discovery.src.loop import canonical_json, digest


class PlatformBoundaryError(ValueError):
    """Raised when a platform operation crosses a project or authority seam."""


@dataclass
class _ProjectState:
    project_id: str
    records: list[dict[str, Any]] = field(default_factory=list)


class ProjectStore:
    """In-memory project store with explicit project-scoped reads and writes."""

    def __init__(self) -> None:
        self._projects: dict[str, _ProjectState] = {}

    def create(self, project_id: str) -> None:
        """Create a project without exposing its mutable internal state."""

        if not project_id.strip() or project_id in self._projects:
            raise PlatformBoundaryError("project id must be unique and non-empty")
        state = _ProjectState(project_id=project_id)
        self._projects[project_id] = state

    def append(self, project_id: str, record: Mapping[str, Any]) -> None:
        state = self._projects.get(project_id)
        if state is None:
            raise PlatformBoundaryError("unknown project")
        state.records.append(copy.deepcopy(dict(record)))

    def read(self, project_id: str) -> list[dict[str, Any]]:
        state = self._projects.get(project_id)
        if state is None:
            raise PlatformBoundaryError("unknown project")
        return copy.deepcopy(state.records)

    def export(self, project_id: str) -> bytes:
        """Export aggregate metadata only; record payloads remain local to this store."""

        records = self.read(project_id)
        schema_counts: dict[str, int] = {}
        for record in records:
            schema = record.get("schema", "unknown")
            schema = schema if isinstance(schema, str) and schema.strip() else "unknown"
            schema_counts[schema] = schema_counts.get(schema, 0) + 1
        payload = {
            "schema": "justai.project_aggregate_export.v1",
            "project_id": project_id,
            "record_count": len(records),
            "schema_counts": schema_counts,
        }
        payload["export_digest"] = digest(payload)
        return canonical_json(payload) + b"\n"


@dataclass(frozen=True)
class HostedRuntime:
    """Synthetic display/runtime surface; it never authorizes execution."""

    name: str = "justai-hosted-synthetic-runtime"
    authorizes_external_execution: bool = False
    can_select_policy: bool = False
    can_select_validator: bool = False
    can_issue_release: bool = False

    def __post_init__(self) -> None:
        if any(
            type(value) is not bool or value
            for value in (
                self.authorizes_external_execution,
                self.can_select_policy,
                self.can_select_validator,
                self.can_issue_release,
            )
        ):
            raise PlatformBoundaryError("hosted runtime authorization capabilities are fixed closed")

    def execute(self, _request: Mapping[str, Any]) -> None:
        raise PlatformBoundaryError(
            "hosted runtime is non-authorizing; external execution remains closed"
        )

    def status(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "infrastructure_reachability": "not measured",
            "connector_health": "not measured",
            "authentication": "not measured",
            "application_correctness": "not asserted",
            "deployment_state": "not asserted",
            "authorizes_external_execution": self.authorizes_external_execution,
            "can_select_policy": self.can_select_policy,
            "can_select_validator": self.can_select_validator,
            "can_issue_release": self.can_issue_release,
        }
