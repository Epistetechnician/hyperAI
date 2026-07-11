# Limitations and claim ceiling

Implemented and tested locally does not mean production-ready.

- The scripted runner and tested Docker control-plane probe do not yet constitute
  the production candidate execution/output-normalization backend.
- SQLite transaction/concurrency behavior and PostgreSQL 16 migrations are
  tested. Production HA, backup/restore, and rolling-upgrade behavior are not.
- External corpus profiles and regular-file materialization from already-pinned
  local checkouts are tested. The build does not clone upstream repositories or
  resolve submodules, LFS objects, nested archives, or OCI images.
- External challenge sources do not attest live completeness. Only the hermetic
  local fixture reaches `SETTLEMENT_READY`.
- Live HIP-4 reading is optional and not a CI dependency. Public endpoint
  availability is not an SLA.
- There is no verified testnet resolution publisher. Manual handoff and unsigned
  intent generation are not settlement.
- Mainnet mutation and trading are structurally disabled.
- API/CLI mock settlement controls and body limits are tested locally; production IAM,
  rate limiting, TLS termination, HA, telemetry exporters, KMS/HSM integration,
  S3 retention controls, and disaster-recovery automation remain future work.
