# Controlled local beta runbook

## Boundary

This beta is restricted to the immutable
`hermetic-trusted-operator-v1` profile in
`config/local-beta-profile.json`: one trusted operator, one local host, serial
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
   `live_writes: false`, `serial_concurrency: 1`, and exactly the seven approved
   scenarios.
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
pnpm run beta:local -- --run-id "${RUN_ID}"
git rev-parse HEAD > "${BETA_ROOT}/git-commit.txt"
rustc --version > "${BETA_ROOT}/rustc-version.txt"
cargo --version > "${BETA_ROOT}/cargo-version.txt"
pnpm --version > "${BETA_ROOT}/pnpm-version.txt"
```

The rehearsal itself enforces and copies the embedded profile, executes the
matrix serially, reloads all eight bundles from disk, verifies both dispute
views, performs the required exported-file tamper rejection, and writes
`beta-run-manifest.json`. The version files above are operator provenance added
only after the command has atomically claimed the fresh run directory.

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
- an authorization denial, secret exposure, artifact mutation, missing blob,
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

After review, generate a deterministic inventory from inside the run root:

```sh
cd "${BETA_ROOT}"
find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 shasum -a 256 > SHA256SUMS
shasum -a 256 -c SHA256SUMS
chmod -R a-w .
```

Copy the retained directory to its declared retention location, verify
`SHA256SUMS` again there, and record the retention period and deletion approvers
in the signoff. `PUBLIC` is the beta profile ceiling, but the operator and
reviewer must still inspect the inventory for credentials, paths, private
evidence, or personal data before sharing any artifact.

## Signoff and claim ceiling

Copy `docs/templates/local-beta-signoff.md` into the run directory. The operator
and independent reviewer must both sign the exact commit, profile digest,
inventory digest, scenario matrix, deviations, retention decision, and final
disposition. Permitted dispositions are `PASS`, `QUARANTINED`, and `INVALID`.

The only permitted claim is: controlled, trusted-operator, single-host local
beta of a hermetic verifiable-outcome protocol with offline-verifiable evidence
and idempotent in-memory mock settlement. It makes no live market creation or
settlement, production sandbox, production IAM, HA/DR, live-source completeness,
cross-host determinism, or exactly-once external delivery claim.
