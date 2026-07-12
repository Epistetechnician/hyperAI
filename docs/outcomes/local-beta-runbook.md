# Controlled local beta runbook

## Boundary

This beta is restricted to the immutable
`hermetic-trusted-operator-v3` profile in
`config/local-beta-profile-v3.json`: one trusted operator, one local host, serial
execution, recorded fixtures, public artifacts, and the in-memory mock HIP-4
backend. Live writes are forbidden. Testnet and mainnet are outside this beta.
The profile does not authorize arbitrary unreviewed validators, hosted users,
or production claims.

## Launch criteria

Both the operator and an independent reviewer must confirm all of the following
before the first scenario starts:

1. The checked-out commit is recorded and `git status --short` is empty.
2. The pinned Rust toolchain and pnpm version are installed.
3. The profile has `network: MOCK`, `backend: mock-hip4`,
   `live_writes: false`, `serial_concurrency: 1`, exactly the seven approved
   scenarios, and the ordered `sqlite-settlement-recovery` and
   `authorization-denial` operational checks.
4. No private key, signing material, credential, token, or populated `.env`
   file is present in the run directory, command line, captured environment, or
   logs.
5. A fresh, unique artifact root has enough free space. A prior run directory
   must never be reused or overwritten.
6. The root gate passes. Infrastructure gates are optional and may run only
   against deliberately provisioned local dependencies.
7. The operator and reviewer have agreed on the run identifier, retention
   location, disk budget, incident owner, and stop conditions.

## Preflight and execution

Run from the repository root. Replace the example run identifier with a unique
UTC identifier. Do not run scenarios concurrently.

```sh
pnpm install --ignore-scripts --frozen-lockfile
pnpm run lint
git status --short
RUN_ID=20260712T120000Z
BETA_PARENT="artifacts/local-beta"
BETA_ROOT="${BETA_PARENT}/${RUN_ID}"
pnpm run beta:local --run-id "${RUN_ID}"
pnpm run beta:verify --run-id "${RUN_ID}"
```

The rehearsal itself requires a clean source tree, records bounded Git/tool and
allowlisted-input provenance, enforces and copies the embedded profile, executes
the matrix serially, reloads all eight bundles, verifies both dispute views,
performs the required exported-file tamper rejection, exercises lost-receipt
recovery across a file-backed SQLite application restart, and sends real
unauthorized requests to all five protected local API routes. It writes manifest
v4. Both operational verifiers reopen retained databases read-only and reject
SQLite WAL or shared-memory sidecars.
It then creates and immediately verifies `beta-artifact-inventory.json`. PASS is
not returned for an unsealed run. `beta:finalize` exists only to recover an
interrupted run that wrote a complete PASS manifest before inventory creation.

When explicitly provisioned, the optional local infrastructure gates are:

```sh
HSAI_TEST_POSTGRES_URL=postgres://... pnpm run verify:infra-postgres
HSAI_TEST_OCI_IMAGE_REPOSITORY=alpine HSAI_TEST_OCI_IMAGE_DIGEST=sha256:... HSAI_TEST_OCI_PLATFORM=linux/arm64 pnpm run verify:infra-oci
```

The OCI gate never pulls an image. Passing either optional gate does not raise
the beta claim ceiling.

## Expected scenario matrix

| Scenario | Truth/finality | Mock effects | Required disposition |
|---|---|---:|---|
| `true` | `TRUE/FINAL` | 1 | Reconciliation `CONFIRMED` |
| `false` | `FALSE/FINAL` | 0 | No publication |
| `invalid-then-valid` | `TRUE/FINAL` | 1 | FAIL retained, later PASS qualifies, reconciliation `CONFIRMED` |
| `disputed-false-counterexample-true` | `TRUE/FINAL` | 1 | Original FALSE retained, accepted decision and supersession retained, reconciliation `CONFIRMED` |
| `incomplete-observation` | `UNRESOLVED/NONE` | 0 | `ObservationIncomplete` blocks resolution and settlement |
| `divergent-replays` | `UNRESOLVED/NONE` | 0 | `ReplayDivergence` blocks resolution and settlement |
| `runner-error` | `UNRESOLVED/NONE` | 0 | Runner error creates no resolution or effect |

Every listed evidence bundle must pass standalone verification. Any mismatch is
a failed run, not an exception to approve during execution.

The operational recovery check must first choose `QueryExternalStatus` with one
intent and no receipt or reconciliation, then choose `Complete` after recovery
with one intent, one receipt, and one reconciliation. Exactly one mock submit
call and one external effect are permitted; recovery must not resubmit.

The authorization check must return the same fixed `403/FORBIDDEN` response for
all five protected routes, retain five exact sanitized denial records, add no
record for an authorized dependency read, and create zero protected effects.
The audit payload must contain no request body, supplied or expected role,
header, credential, path parameter, IP address, or raw error.

## Stop and quarantine

Stop immediately and do not start another scenario when any of these occurs:

- a command exits nonzero or the root gate, profile check, or evidence
  verification fails;
- a digest, claim identity, environment, image, platform, or binding differs
  from the recorded value;
- truth, finality, blocker, effect count, or reconciliation differs from the
  expected matrix;
- incomplete or stale observation supports FALSE, or divergent/error execution
  supports a resolution;
- readiness is stale, unknown, degraded, unavailable, clock-regressed, or HTTP
  503;
- an unexpected authorization denial outside the bounded denial check, secret
  exposure, artifact mutation, missing blob,
  disk exhaustion, or host instability occurs;
- any testnet/mainnet or other externally effective write becomes reachable;
- a settlement receipt is absent or ambiguous. Never blind-resubmit; preserve
  the intent and use authoritative lookup only;
- the same unexplained error occurs twice.

Freeze the affected run directory read-only, preserve process and host facts,
and copy `docs/templates/local-beta-incident.md` into the run directory. Record
the affected state slices, timeline, hashes, external-effect count, containment,
and recovery evidence. A source gap prohibits negative finality. Replay
divergence quarantines the verifier/environment tuple. Evidence loss blocks
publication and finalization until every restored digest verifies.

## Resume

Resume requires a completed incident record, identified cause, corrective
change or demonstrated environmental recovery, a fresh successful root gate,
and written approval from both operator and reviewer. Start a new run identifier
and artifact root. Never continue inside or modify a quarantined run. A resumed
run begins at preflight and executes the full seven-scenario matrix.

## Retention and integrity inventory

Retain the profile, commit and tool versions, exact command log and exit codes,
scenario reports, evidence bundles and referenced CAS blobs, observation and
evaluation records, eligibility records, resolutions, attestations, disputes,
decisions, supersession records, intents, receipts, reconciliations, gate logs,
incident records, and final signoff. Treat the database and evidence artifacts
as one recovery unit. Never retain private keys or a populated `.env` file.

The CLI-generated inventory is the integrity boundary. Verify the retained copy
from the repository root:

```sh
pnpm run beta:verify --run-id "${RUN_ID}"
```

Copy the sealed directory to its declared retention location and run the same
read-only verifier against that parent. Any added, removed, or modified file,
unsafe type, manifest/profile/provenance drift, evidence failure, or inventory
tamper rejects. The inventory proves exact retained bytes, not content safety;
the operator and reviewer must still inspect for credentials, private evidence,
or personal data before sharing.

## Signoff and claim ceiling

Keep completed signoff as detached sibling metadata outside the sealed run root,
for example `${BETA_ROOT}.signoff.md`; otherwise it would invalidate the exact
inventory. The operator and reviewer must both sign the exact commit, profile
digest, inventory digest, scenario matrix, deviations, retention decision, and
final disposition. Permitted dispositions are `PASS`, `QUARANTINED`, and
`INVALID`.

Runs `beta-001` and `beta-002` predate manifest v2 provenance and automatic
inventory sealing. They remain useful rehearsal evidence but are legacy,
unsealed runs and cannot be upgraded or rewritten in place. `beta-003` is a
sealed manifest-v2 run and remains verifiable, but predates the SQLite recovery
check. `beta-004` is a sealed manifest-v3 run with recovery and remains
verifiable, but predates durable denial auditing. Use a fresh manifest-v4 run
for the current beta contract.

The only permitted claim is: controlled, trusted-operator, single-host local
beta of a hermetic verifiable-outcome protocol with offline-verifiable evidence,
idempotent in-memory mock settlement, and file-backed SQLite recovery across an
application restart while the mock authority is retained, plus durable
sanitized denial evidence for protected local API routes. It makes no production
authentication, identity-provider, session-security, live market creation or
settlement, backend-process or host-crash durability, production sandbox,
production IAM, HA/DR, live-source completeness, cross-host determinism, or
distributed exactly-once delivery claim.
