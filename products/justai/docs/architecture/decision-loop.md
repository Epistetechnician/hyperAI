# Governed decision loop

State slice: `justai-discovery-v1`.

The loop is a pure-data sequence:

```text
algorithmic proposal
  -> locked-budget alternatives and serendipity sample
  -> falsifiable human intuition record
  -> validation (external-independent or local-quorum)
  -> promote | Unknown | quarantine
  -> retrospective
```

The fast path requires confidence at least 80/100, a reversible action, accepted validation, and no custody/provenance ambiguity. Validation may be external-independent or a bounded local-quorum packet with three distinct instances and two distinct machine digests. Local quorum is never independent review and cannot raise the `local-replay` claim ceiling. Any disagreement, unexpected result, novel candidate, missing or stale evidence, irreversible action, human skill or safety concern, ambiguous rollback, or infrastructure uncertainty enters the slow path.

The slow path preserves all alternatives, records dissent when disagreement exists, requires an explicit human disposition for promotion, and defaults to `Unknown` when that disposition or required evidence is absent. `quarantine` is available when material needs isolation. There is no forced acceptance branch.

Intuition is evidence-bearing rather than mystical. It records a hypothesis, confidence, observed evidence, rejected alternatives, recommended action or restraint, reviewer identity, later outcome, and whether it added value. A missing later outcome is an unresolved review state, not a success.
