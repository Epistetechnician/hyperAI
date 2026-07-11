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

The migration contract can be rerun against an operator-supplied disposable
database without changing the default hermetic gate:

```sh
HSAI_TEST_POSTGRES_URL=postgres://... pnpm run verify:infra-postgres
```

The Docker control-plane probe requires an already-present immutable image and
does not fetch, mount, network, or execute candidate content:

```sh
HSAI_TEST_OCI_IMAGE_REPOSITORY=alpine \
HSAI_TEST_OCI_IMAGE_DIGEST=sha256:... \
pnpm run verify:infra-oci
```

Passing this probe establishes daemon control enforcement, not production
candidate-verifier readiness.
