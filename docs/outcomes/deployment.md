# Deployment

The tested profile is a single-process local CLI using SQLite-compatible
migrations and a filesystem content-addressed store. It is not a production
deployment claim.

Production topology requires the API and worker as separate non-root services,
PostgreSQL with transactional outbox, S3-compatible immutable/versioned object
storage, a real hostile-code isolation backend, external KMS/HSM signer
references, operator identity enforcement, and a telemetry backend. Run schema
migrations as a distinct release job before new workers. Back up PostgreSQL and
the evidence bucket together and verify sampled bundle digests after restore.

Liveness means the process loop responds. Readiness additionally requires the
database schema, outbox, evidence store, signer registry, and required source
adapters. An external-source outage must make dependent work not-ready without
making existing immutable public evidence unavailable.

`.env.example` contains placeholders only. Private keys must never be passed on
the CLI, stored in environment files, or logged. Mainnet write configuration has
no supported code path.
