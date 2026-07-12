# Limitations and claim ceiling

Implemented and tested locally does not mean production-ready.

- The local Docker candidate verifier is tested with a preloaded digest-pinned
  Alpine image for controls, exact inputs, PASS/FAIL, timeout, output flood,
  cleanup, evidence, finality, and mock reconciliation. The Docker host and
  daemon remain trusted; this is not adversarial production sandbox
  qualification.
- The Docker image, explicit platform, argv, exact `LANG`/`TZ` values, fixed
  mounts, and isolation/resource controls are cryptographically bound to the
  frozen claim environment before Docker is invoked. This prevents
  operator-selected environment drift but does not establish cross-host
  determinism or qualify the image, daemon, kernel, rootless/user-namespace
  configuration, or seccomp/AppArmor policy for hostile production code.
- SQLite transaction/concurrency behavior and PostgreSQL 16 migrations are
  tested. Production HA, backup/restore, and rolling-upgrade behavior are not.
- Settlement crash recovery is joined across a file-backed SQLite restart and a
  retained in-memory mock authority. This models application restart only; it
  does not prove mock/backend process durability, host-crash recovery, live
  network status lookup, or distributed exactly-once delivery.
- Counterexample correction is tested locally with append-only SQLite history,
  PostgreSQL migration contracts, portable evidence, and mock settlement. The
  composed demo does not expose a production dispute endpoint, authenticate a
  live dispute authority, or perform a testnet/mainnet correction.
- External corpus profiles and regular-file materialization from already-pinned
  local checkouts are tested. Special objects are inspected inertly and block
  readiness when unresolved. Reviewed overlays compile sanitized local
  inventories into deterministic evaluation packages, but they do not establish
  runnable runtime/verifier correctness or upstream item permissions. The build
  does not clone upstream repositories, retrieve submodule or LFS objects,
  extract archives, or materialize OCI images.
- External challenge sources do not attest live completeness. Only the hermetic
  local fixture reaches `SETTLEMENT_READY`.
- Incomplete observation and divergent replay refusal are composed with local
  injected fixtures. They do not establish behavior under a real distributed
  source outage or cross-host nondeterministic validator deployment.
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
- API dependency readiness is evaluated from injected cached fixed-schema state;
  stale, unavailable, unknown, degraded, and clock-regressed state fails closed.
  Real PostgreSQL, object-store, signer, outbox, and source probe producers are
  deployment work; the development role header is not production IAM.
