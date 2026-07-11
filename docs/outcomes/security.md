# Security

The repository-grounded security analysis is [the threat model](../threat-model.md).
Security boundaries are fail-closed: execution errors are not FALSE, incomplete
observation blocks negative finality, ambiguity blocks settlement, mainnet writes
are unavailable, and mock effects are idempotent and reconciled before retry.
