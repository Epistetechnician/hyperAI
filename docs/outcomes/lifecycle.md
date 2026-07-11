# Outcome lifecycle

Execution outcome, claim truth, resolution finality, and settlement lifecycle
are separate typed state machines.

```mermaid
stateDiagram-v2
  [*] --> UNRESOLVED
  UNRESOLVED --> TRUE: eligible PASS and quorum
  UNRESOLVED --> FALSE: deadline grace completeness and quorum
  NONE --> PROVISIONAL: resolution proposal
  PROVISIONAL --> DISPUTED: blocking dispute
  DISPUTED --> PROVISIONAL: append only decision
  PROVISIONAL --> FINAL: window closed and unpaused
```

`FAIL` rejects one candidate; it does not make an existential claim false.
`INCONCLUSIVE` and `ERROR` never map directly to `TRUE` or `FALSE`. A negative
resolution requires deadline and grace expiry, complete finalized observation,
no eligible PASS, and terminal processing for every pending eligible candidate.

A final resolution is immutable. A correction is a separately signed,
non-settling superseding record. Active disputes, pauses, stale bindings,
missing signatures, incomplete observation, and unsupported backend capability
block settlement readiness.

Settlement progresses through `UNBOUND`, `BOUND`, `NOT_READY`, `READY`,
`INTENT_BUILT`, `SUBMITTED`, and `CONFIRMED`, with explicit `PAUSED` and
`RECONCILIATION_FAILED` states. `CONFIRMED` is terminal. Unpausing returns to
`NOT_READY` so every readiness condition is re-evaluated.
