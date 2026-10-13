# Claim ceilings

State slice: `justai-contracts-v1`.

Claim ceilings are ordered and enforced independently from disposition:

```text
source-description-only
  < local-proposal
  < local-replay
  < engineering-evidence
  < independent-review-pending
  < scientific-evidence
  < production-authorized
```

This ordering is a validation limit, not a promise that a later level is available. JustAI v1 produces local engineering records only. Scientific evidence, production readiness, hosted enforcement, profitability, and authorization require separate gates and cannot be inferred from a green test or local replay.
