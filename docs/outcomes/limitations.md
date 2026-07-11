# Limitations and claim ceiling

Implemented and tested locally does not mean production-ready.

- The local Docker candidate verifier is tested with a preloaded digest-pinned
  Alpine image for controls, exact inputs, PASS/FAIL, timeout, output flood,
  cleanup, evidence, finality, and mock reconciliation. The Docker host and
  daemon remain trusted; this is not adversarial production sandbox
  qualification.
- SQLite transaction/concurrency behavior and PostgreSQL 16 migrations are
  tested. Production HA, backup/restore, and rolling-upgrade behavior are not.
- External corpus profiles and regular-file materialization from already-pinned
  local checkouts are tested. Special objects are inspected inertly and block
  readiness when unresolved. The build does not clone upstream repositories,
  retrieve submodule or LFS objects, extract archives, or materialize OCI images.
- External challenge sources do not attest live completeness. Only the hermetic
  local fixture reaches `SETTLEMENT_READY`.
- Live HIP-4 reading is optional and not a CI dependency. Public endpoint
  availability is not an SLA.
- There is no verified testnet resolution publisher. Manual handoff and unsigned
  intent generation are not settlement.
- Mainnet mutation and trading are structurally disabled.
- API/CLI mock settlement controls, body limits, and deterministic fixed-cardinality
  OpenMetrics rendering are tested locally. Production metrics transport and
  aggregation, IAM, rate limiting, TLS termination, HA, tracing, alerting,
  KMS/HSM integration, S3 retention controls, and disaster-recovery automation
  remain future work.
