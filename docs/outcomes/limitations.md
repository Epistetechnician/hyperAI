# Limitations and claim ceiling

Implemented and tested locally does not mean production-ready.

- The scripted runner and OCI policy model do not constitute a production
  hostile-code sandbox.
- PostgreSQL migration SQL exists, but this run did not execute it against a
  live PostgreSQL server. SQLite transaction/concurrency behavior is tested.
- External corpus work is profile/pin/conformance metadata. No upstream bytes,
  submodules, LFS objects, nested archives, containers, or live submissions are
  materialized.
- External challenge sources do not attest live completeness. Only the hermetic
  local fixture reaches `SETTLEMENT_READY`.
- Live HIP-4 reading is optional and not a CI dependency. Public endpoint
  availability is not an SLA.
- There is no verified testnet resolution publisher. Manual handoff and unsigned
  intent generation are not settlement.
- Mainnet mutation and trading are structurally disabled.
- API operator auth and body-limit behavior are tested locally; production IAM,
  rate limiting, TLS termination, HA, telemetry exporters, KMS/HSM integration,
  S3 retention controls, and disaster-recovery automation remain future work.
