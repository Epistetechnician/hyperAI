# Deployment

The tested profile is a single-process local CLI using SQLite-compatible
migrations and a filesystem content-addressed store. It is not a production
deployment claim.

The controlled local beta is governed by the immutable mock-only profile in
`config/local-beta-profile-v3.json` and the two-person operating procedure in
`docs/outcomes/local-beta-runbook.md`. It permits only serial, trusted-operator,
single-host runs of the seven recorded scenarios plus the file-backed SQLite
lost-receipt recovery check. That check covers an application restart with a
retained mock authority; it does not establish backend-process or host-crash
durability. The beta also records and read-only verifies sanitized durable
denial audits for all protected local API routes, but the development role
header is not production authentication or IAM. The beta does not authorize
live writes, hosted users, arbitrary
unreviewed validators, testnet, or mainnet.

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

The implemented API evaluates cached fixed-schema dependency state on each
request. Its default composition is unconfigured and returns not-ready. A
deployment must inject non-blocking snapshot and clock providers; real database,
object-store, signer, and source probes run outside request handlers.
Protected deployments must also inject a durable authorization-denial audit
sink. Compatibility constructors use a non-durable sink and do not establish
the composed beta audit guarantee.

`.env.example` contains placeholders only. Private keys must never be passed on
the CLI, stored in environment files, or logged. Mainnet write configuration has
no supported code path.

The migration contract can be rerun against an operator-supplied disposable
database without changing the default hermetic gate:

```sh
HSAI_TEST_POSTGRES_URL=postgres://... pnpm run verify:infra-postgres
```

The Docker infrastructure gate requires an already-present immutable image and
never pulls it. It runs the effect-free control-plane probe plus local verifier
PASS/FAIL, timeout, output-flood cleanup, and two-replay mock-settlement
acceptance:

```sh
HSAI_TEST_OCI_IMAGE_REPOSITORY=alpine \
HSAI_TEST_OCI_IMAGE_DIGEST=sha256:... \
HSAI_TEST_OCI_PLATFORM=linux/arm64 \
pnpm run verify:infra-oci
```

Passing this gate establishes the tested local verifier and claim-bound Docker
environment contract, not production hostile-code sandbox qualification.
