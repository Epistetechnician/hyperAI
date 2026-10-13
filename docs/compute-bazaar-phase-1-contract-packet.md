# HyperAI Compute Desk: Phase 1 Contract Packet

Status: Phase 1 contract packet for the accepted local implementation slice.
The packet itself grants no code, dependency, data, runtime, release, or
external-effect authority; the accepted state-slice record is the authority.

State slice: `hyperai.bazaar_rust_port_phase1_packet.v1`.
Proposed implementation slice: `hyperai.bazaar.desk_runtime.v1` (not
authorized by this packet).
Parent: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Product direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Market contract: [compute-market refactor plan](compute-market-refactor-plan.md).
Source pins and rights record: [provenance](provenance.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`; current
dirty and untracked work is preserved. Initial packet facts were checked on
2026-09-24. OS lifecycle assumptions and validation status were refreshed on
2026-09-25 below; the dirty working tree still has no frozen content digest.

## Decision freeze and handoff (2026-10-01)

Phase 1 decisions are fixed in the contract sections below. The local
implementation already exists under the separately accepted
`hyperai.bazaar.desk_runtime.v1` state slice; this packet neither reopens that
implementation nor grants runtime or release authority.

- **Service/API:** one existing Axum service, HTTP/1.1 JSON at `/api/v1`,
  IPv4 loopback only (`127.0.0.1:4317` by default; explicit port override and
  port 0 for tests). Preserve existing route paths, methods, response shapes,
  and schema identifiers; bundled clients use the versioned session/bearer
  auth contract.
- **Storage:** per-user HyperAI home with schema-v1 SQLite metadata only;
  existing market, outcomes, HyperIntel, JustAI, and SCM records remain in
  their owning stores. Offline metadata backup/restore uses the explicit
  basename, digest, safety-snapshot, and failure-atomicity contract below.
- **Proposed platform matrix:** macOS 14+ on Apple Silicon and Intel, Windows
  11 25H2+ x86_64, and Ubuntu 24.04+ x86_64. These are target decisions, not
  support claims; each native OS/architecture still needs its own acceptance
  run. Emulation and cross-compilation do not close that gate.
- **Dependencies:** Rust 1.93.1; Ratatui 0.30.2 with defaults disabled and its
  Crossterm backend, direct Crossterm 0.29.0; official ACP Rust SDK 2.2.0 with
  stable protocol v1 and locked schema 1.9.1. ACP is a later read-only
  adapter; Phase 1 launches no ACP process.
- **Compatibility:** `hyperai` is primary, `hsai` remains an alias, and
  `hyperai-ui` remains the compatibility service launcher. Preserve existing
  `market`/`outcomes` CLI behavior and bundled API contracts; use explicit
  `--bin` selection and `corepack pnpm` with the pinned Rust wrapper.
- **Data and reuse:** HyperAI-authored synthetic data is allowed. SCM's
  imported MIT source remains in its nested workspace, while rights to
  caller-supplied captures remain separate. HyperIntel use is limited to its
  pinned committed offline adapter. JustAI runtime use is excluded. Bazaar
  code/data/assets and automated feeds remain excluded until written rights
  cover exact materials and use.
- **Separate authority gates:** live Fleet operations, paid/provider Eval,
  financial Trade, seller/provider calls, network ingestion, escrow,
  settlement, publication, and production are outside this phase and require
  their own packet and accepted state slice.

The next planned Data work already has its own local synthetic contract
packet. Overall Phase 2 remains `NOT_ASSIGNED` while worker process isolation,
platform enforcement, and full qualification are unresolved. The worker IPC
record is planning-only; the prior macOS Virtualization attempts are closed,
and another VM start requires a fresh exact packet and explicit authorization.
Do not treat the Phase 1 contract freeze or the existing Phase 2 design as
authorization to implement that worker.

Current import-inventory status (2026-10-01): under
`hyperai.bazaar.justai_import_generated_lock_reconciliation.v1`, the generated
`products/justai/pnpm-lock.yaml` absent from the selected JustAI source commit
and checkout was removed from the imported snapshot. The recorded 176-file
JustAI digest now matches, and `python3 scripts/verify_imports.py` passes for
Simple Compute Market, HyperIntel, and JustAI. This is local inventory
evidence only; the imported source's redistribution rights remain unverified.
See the [reconciliation state slice](state-slices.md#hyperai-bazaar-justai_import_generated_lock_reconciliationv1).

Current-source follow-up (2026-09-27): the root `corepack pnpm run lint`,
`corepack pnpm run verify:monorepo`, and browser-JavaScript syntax check passed.
A disposable source install then exercised initialization, local service start
and connection, synthetic Market/Data/Fleet/Eval/Trade CLI paths, and the
installed TUI's pseudo-terminal startup and cleanup. These current-host results
do not close the declared native platform matrix, clean reproduction, or
broader client accessibility/parity gates; exact scope and limitations are in
the [`desk_runtime` state-slice record](state-slices.md#hyperai-bazaar-desk_runtimev1)
and [`ACP state-slice record`](state-slices.md#hyperai-bazaar-acpv1).

Install-to-ACP follow-up (2026-09-27): `corepack pnpm run install:local --
--root <disposable-root> --offline` installs the pinned source-built
`hyperai`, `hsai`, `hyperai-ui`, and `hyperai-mcp` binaries together. The
ignored source-install integration passed on macOS 26.6.1 arm64 with pinned
Rust/Cargo 1.93.1. It starts the installed service, connects the existing
HyperAI-authored synthetic ACP peer through the installed sibling MCP helper,
checks the fixed ten-tool surface and Fleet preview digest parity, confirms
capability revocation and shutdown, and verifies that the fixture peer is not
installed. This is `INTERNAL_PASS` for that local install path only. Native
platform coverage, third-party ACP behavior, clean reproduction of the current
source, package signing, and release qualification remain open. Exact commands,
scope, custody, and claim limits are in the
[local install ACP bundle state slice](state-slices.md#hyperai-bazaar-local_install_acp_bundlev1).

Current-source verification (2026-09-28): `corepack pnpm run lint` passed
workspace formatting, strict all-target/all-feature Clippy, contract tests,
the full root workspace suite, and doc tests. Environment-gated PostgreSQL
and Docker tests and two CLI tests requiring separately staged binaries remain
ignored by that root command. The locked offline source-install-to-ACP
integration also passed after its test-only peer was corrected to expect the
current separate mock-verdict and mock-disposition trace statuses. The first
rerun had exposed that stale fixture expectation; the service/API trace
contract did not change. This remains `INTERNAL_PASS` for the exact local
synthetic install path, not platform-matrix or release qualification.

Installed public CLI lifecycle follow-up (2026-09-29): the repeatable offline
source-install integration now invokes `--version`, `init`, `doctor`,
`service start`, `service status`, `connect`, and `service stop` on the
installed `hyperai` binary before running the existing synthetic ACP/MCP
integration. The exact test, root lint, pinned Rust formatting, and diff check
passed on macOS 26.6.1 arm64 with Rust 1.93.1. This is `INTERNAL_PASS` for the
local installed lifecycle and ACP path only; no separate-host reproduction or
native-platform qualification is established. Simple Compute Market and
HyperIntel import verification passed, while JustAI inventory verification
remains mismatched by an extra imported `products/justai/pnpm-lock.yaml` whose
origin is unresolved. See the [installed CLI lifecycle packet](compute-bazaar-installed-cli-lifecycle-packet.md)
and [state-slice record](state-slices.md#hyperai-bazaar-desk_runtime_installed_cli_lifecyclev1).

Installed compatibility-entrypoint follow-up (2026-09-29): the same offline
source-install integration now proves installed `hsai` returns the same
version and synthetic market-demo JSON as `hyperai`, and installed
`hyperai-ui` starts the shared local service that installed `hyperai` then
connects to, inspects, and gracefully stops. The exact test, root lint,
pinned Rust formatting, and scoped hygiene passed on macOS 26.6.1 arm64 with
Rust 1.93.1. This is `INTERNAL_PASS` for those installed compatibility paths
on that host only. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Native platform, accessibility, signing, and release gates
remain open; the separate JustAI import mismatch remains unresolved. See the
[compatibility entrypoints packet](compute-bazaar-installed-compat-entrypoints-packet.md)
and [state-slice record](state-slices.md#hyperai-bazaar-desk_runtime_installed_compat_entrypointsv1).

Installed TUI PTY follow-up (2026-09-29): the exact ignored offline
source-install integration now runs the installed `hyperai desk` binary in a
POSIX pseudo-terminal, sends Ctrl+C, and verifies raw-mode/alternate-screen
entry, clean exit, terminal and cursor restoration, and temporary-home
cleanup. The exact integration, root lint, pinned Rust formatting, and diff
hygiene passed on macOS 26.6.1 arm64 with Rust/Cargo 1.93.1 and Python 3.14.5.
This is `INTERNAL_PASS` for this installed POSIX TUI path only. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Physical
keyboard, assistive technology, native Linux/Windows/Intel macOS,
signing, and release qualification remain open. The unrelated JustAI import
mismatch remains unresolved. See the [installed TUI PTY packet](compute-bazaar-installed-tui-pty-packet.md)
and [state-slice record](state-slices.md#hyperai-bazaar-desk_runtime_installed_tui_ptyv1).

Installed TUI resize qualification (2026-09-29): under
`hyperai.bazaar.desk_runtime.installed_tui_resize.v1`, the exact locked,
offline source-install integration passed on macOS 26.6.1 arm64 with pinned
Rust/Cargo 1.93.1 and Python 3.14.5. The preliminary one-key probe only
addressed row 4 at both sizes and is superseded. In the geometry-strengthened
run, eleven navigation events at 48x10 kept redraw cursor rows at or below row
9; one wrap event at 100x30 exposed rows through row 28. The installed binary
exited 0, restored termios/cursor/alternate screen, and its temporary home
and test service were removed. Root lint passed after the follow-up.
Assurance is `INTERNAL_PASS` for this synthetic macOS arm64 PTY path only;
reviewer status is `NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`.
Physical keyboard, assistive technology, populated-state, native platform,
release, and production gates remain open. The JustAI import mismatch is
preserved. See the [installed resize packet](compute-bazaar-installed-tui-resize-packet.md)
and [state-slice record](state-slices.md#hyperai-bazaar-desk_runtime_installed_tui_resizev1).

Local installer transaction follow-up (2026-09-29): the source installer now
builds and validates the complete four-binary bundle in a sibling staging
root, then promotes it with restoration of the previous install on promotion
failure. Three fake-Cargo tests passed for initial install, upgrade, staged
package failure, promotion rollback, failed-rollback backup retention, and
synthetic data-home preservation. The exact locked offline installed-binary
integration and `corepack pnpm run lint` passed on macOS 26.6.1 arm64 with
pinned Rust/Cargo 1.93.1. This is `INTERNAL_PASS` for these local paths only;
reviewer status is `NOT_REQUESTED` and reproduction is `NOT_REPRODUCED`.
Native Linux/Windows/Intel macOS, crash recovery, signing, and release gates
remain open. The unresolved JustAI extra-lockfile inventory mismatch remains
preserved. See the [installer transaction packet](compute-bazaar-local-installer-transaction-packet.md)
and [state-slice record](state-slices.md).

### Windows MSVC workspace cross-check preflight (2026-09-28)

State slices: `hyperai.bazaar.desk_runtime.v1` and
`hyperai.bazaar.acp.v1`. On Darwin arm64 with pinned Rust 1.93.1, the command

```sh
sh scripts/with-pinned-rust.sh cargo check --locked --offline --workspace \
  --all-targets --all-features --target x86_64-pc-windows-msvc
```

did not reach compilation of HyperAI workspace crates. The `zstd-sys` and
`stacker` C build steps failed because the host has no Windows SDK/CRT headers
(`stdlib.h`, `string.h`, and `windows.h`) or MSVC `lib.exe`; `VCINSTALLDIR`,
`WindowsSdkDir`, and the target tool overrides are absent. This is
`INCONCLUSIVE` as a cross-build preflight, not a HyperAI code failure or a
Windows compile pass. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. No toolchain was installed and no source file was changed.
The next gate is a configured Windows/MSVC build environment; native Windows
runtime qualification remains separately required.

### x86_64 Apple workspace check under Rosetta (2026-09-28)

State slices: `hyperai.bazaar.desk_runtime.v1` and
`hyperai.bazaar.acp.v1`. On the Darwin arm64 host with pinned Rust 1.93.1,
`arch -x86_64 /usr/bin/uname -m` returned `x86_64`, confirming Rosetta is
available. The locked, offline workspace check completed successfully for
`x86_64-apple-darwin` with all targets and features:

```sh
sh scripts/with-pinned-rust.sh cargo check --locked --offline --workspace \
  --all-targets --all-features --target x86_64-apple-darwin
```

The matching locked, offline workspace test command also completed successfully
with all features; seven opt-in tests were ignored by their existing
environment/staging gates:

```sh
sh scripts/with-pinned-rust.sh cargo test --locked --offline --workspace \
  --all-features --target x86_64-apple-darwin
```

This is same-host x86_64 execution through Rosetta against the current dirty
working tree. No frozen source manifest or clean second environment was used,
so reproduction remains `NOT_REPRODUCED`; no overall assurance label is
assigned. This does not establish native Intel hardware support, native
Windows support, independent review, or release readiness. Reviewer status is
`NOT_REQUESTED`. The tests used local synthetic fixtures and created no
external connections or effects.

### Current-source focused x86_64 package run under Rosetta (2026-10-02)

Under `hyperai.bazaar.desk_runtime.v1`, the pinned locked/offline command

```sh
arch -x86_64 sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  --target x86_64-apple-darwin -p outcomes-cli -p hyperai-local-service \
  -p hyperai-tui --all-features
```

passed 137 tests across the local-service, Data/API/storage, TUI, and CLI
suites: 0 failed, with 3 existing staged-binary or browser tests ignored.
`file` confirmed the service, CLI, and TUI test binaries are Mach-O x86_64.
The Rust source manifest contained 185 files and 3,715,759 bytes, SHA-256
`146ba276c86771b146174e665e2dd255f5d278eacdb04269d9b955d22dc2b313`.
Assurance is `INTERNAL_PASS` for this exact same-host Rosetta test scope;
reviewer status is `NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`.
Tests used local synthetic data and disposable test state, with no external
connections or writes. Rosetta is supplemental architecture evidence only; it
does not establish native Intel behavior, the macOS 14 floor, native
Windows/Ubuntu support, independent review, or release readiness. See the
[state-slice evidence](state-slices.md#hyperai-bazaar-desk_runtimev1).

SQLite page-budget failure follow-up (2026-09-28): under
`hyperai.bazaar.desk_runtime.v1`, the focused pinned-toolchain command
`sh scripts/with-pinned-rust.sh cargo test --locked --offline -p
hyperai-local-service --test storage sqlite_full_rejects_a_write_without_partial_metadata`
passed. The test caps SQLite `max_page_count` at the current page count, then
confirms a metadata insert fails with SQLite result code 13 (`SQLITE_FULL`),
leaves no partial row, and preserves schema version 1. The full root
`corepack pnpm run lint` also passed on this working tree, including this test.
This is `INTERNAL_PASS` for SQLite's page-budget refusal only; it does not
simulate physical filesystem exhaustion/`ENOSPC` or establish storage behavior
on other operating systems. Reviewer status is `NOT_REQUESTED`, reproducibility
for that standalone one-test command was `NOT_REPRODUCED`, and the database was
disposable synthetic test data. That test is included in the later focused
storage-suite reproduction recorded below. Native host-filesystem failure
behavior and the supported-platform storage matrix remain open; a bounded
Linux `tmpfs` `ENOSPC` check is recorded later in this packet.

Manual metadata backup/restore follow-up (2026-09-28): the accepted schema-v1
recovery contract is implemented under the local desk-runtime slice. The
storage backup command reports a consistent VACUUM INTO snapshot's safe
basename, byte count, and SHA-256. Restore validates a same-home generated
backup, refuses unsupported or corrupt layouts, holds the service and
initialization locks, writes through a SQLite transaction, and retains a
pre-restore safety snapshot. The CLI uses stable error codes and sanitized
messages; no REST route or metadata values are exposed.

On macOS arm64 with Rust 1.93.1, the storage integration suite passed 9/9
cases, the CLI compatibility suite passed 12 with 2 existing staged-binary
cases ignored, and a forced mid-copy failure test confirmed rollback and
recovery-basename reporting. The root lint passed workspace formatting,
strict Clippy, contract tests, the full root suite, and doc tests. This is
INTERNAL_PASS for the current macOS working tree. A clean, network-disabled
`linux/amd64` container on the Apple Silicon host also passed the focused
storage (9), CLI compatibility (12), and TUI terminal-restoration (7) suites
with Rust 1.93.1 and the locked offline dependency set. This is
`REPRODUCED_LOCAL` for those exact commands and the 273-file source inventory
SHA-256 `ef8cf5c78e8ea0060470eb4b7670d7a8e906db3b41681eab31640d0ce8322228`
only; it is same-owner emulation, not native Linux qualification or
independent review. The CLI suite has 2 pre-existing staged-binary tests
ignored. Reviewer status is `NOT_REQUESTED`. Test databases and snapshots used
synthetic values in disposable temporary homes. Physical filesystem
exhaustion, native Windows, Ubuntu and Intel macOS behavior, and the complete
declared target matrix remain open. Exact image, lockfile, output digests,
failed broader link attempt, custody, and claim limits are in the
[`desk_runtime` state-slice record](state-slices.md#hyperai-bazaar-desk_runtimev1).

Current-source PTY Ctrl+C follow-up (2026-09-28):
`corepack pnpm run verify:tui-ctrl-c` passed on macOS 26.6.1 arm64 with Rust
1.93.1 and Python 3.14.5. The command builds the local `hyperai` binary
offline, then uses the retained standard-library POSIX PTY harness at
`scripts/verify_hyperai_tui_ctrl_c.py` (SHA-256
`90ecf8173efcb971b2793f027537048cfe8450bd4f5ac0bbb092870212b811dd`). The
harness initialized a disposable home, started the loopback service on port
0, entered raw mode and the alternate screen, sent Ctrl+C (`0x03`), and
observed exit status 0, baseline termios restoration, alternate-screen exit,
cursor restoration, service shutdown, and temporary-home removal. The
2,286-byte transcript SHA-256 is
`6e3da0669681465b63b74e73a388cc1be4bf859cf74fb6e3c18684a2e15d8077`; the
rebuilt `target/debug/hyperai` SHA-256 is
`9ccb145fa9732e9fb71a8c4b20ae1a47253cc6bd21c1431e35df4b067ede3304`.
After adding the retained harness and package command,
`corepack pnpm run lint` passed formatting, strict all-target/all-feature
Clippy, contract tests, and the full root workspace and doc-test suites;
environment-gated external infrastructure checks remain ignored.
This is `INTERNAL_PASS` for that current-source macOS arm64 PTY path only.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED` because
the retained harness has not been run in a clean second environment. It does
not establish physical-key input, signal-driven interruption, live-session
panic recovery, or another platform's behavior.

Current-source live-session panic restoration follow-up (2026-10-01):
`corepack pnpm run verify:tui-panic` passed on macOS arm64 with Rust 1.93.1
and Python 3.14.5. A default-off `tui-panic-test-support` feature enables a
debug-build-only hook in the actual `hyperai desk` loop; it waits after the
first rendered frame for the PTY harness to send `p`, then unwinds. The test
observed raw mode and alternate screen before the key, exit status 101, exact
baseline termios restoration, alternate-screen exit, and cursor restoration.
The ephemeral loopback service and temporary home were removed. The panic
binary SHA-256 is
`ec89e369e5afd2bb64da4974e0baea6e3b80abd7020612eee6e3b48527bb18f4`; the
2,360-byte PTY transcript SHA-256 is
`ea3175e83366440b29c325a91e2f19fb0383eaa6100a415cd7c634eeaae7396f`.

The default-feature `corepack pnpm run verify:tui-ctrl-c` still passed, with
resize redraws bounded at rows 9/10 and 28/30, clean Ctrl+C exit, exact
termios restoration, screen/cursor cleanup, and temporary-home removal. The
focused `hyperai-tui` terminal suite passed 10/10, formatting passed, and
strict Clippy passed for `hyperai-tui` and `outcomes-cli` with the test-support
feature. Assurance is `INTERNAL_PASS` for these local macOS arm64 PTY paths;
reviewer status is `NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`.
This does not qualify panic-abort, process kill, physical keyboard, another
OS/architecture, accessibility, or the Phase 1 platform matrix. The initial
PTY-script indentation error and first Clippy lint failure were corrected and
are retained in the state-slice record. See
[`hyperai.bazaar.desk_tui.live_panic_cleanup.v1`](state-slices.md#hyperai-bazaar-desk_tui-live_panic_cleanupv1).

## Purpose and product boundary

HyperAI is the single buyer and operator interface for compute discovery,
market context, procurement, inference lineage, resource operations,
evaluation, research, and evidence. “Connect everything” means a coherent
navigation and action-preview experience over separately owned contracts. It
does not mean merging the products' databases, trust roots, execution
authority, settlement rules, or claim semantics.

Phase 1 establishes the local service and terminal foundation that later
workspaces can use. It connects the existing browser shell, CLI, and TUI to
one Rust application facade. It does not build the Data lake, ingest market
feeds, execute workloads, run evaluations, create Trade positions, or project
records into JustAI. Those remain later, separately gated work.

This packet sets exact technical defaults for that first implementation
slice. They are proposed product contracts, not approvals to reuse protected
source or data and not evidence that the runtime works.

## Phase 1 scope

### Included

- One Rust-owned local service based on the existing Axum host and
  `hyperai-application` facade.
- `hyperai init`, `hyperai doctor`, `hyperai service start|status|stop`, and
  `hyperai connect|desk`, with a foreground recovery/development mode.
- A versioned HTTP/JSON API contract, local-client authentication, bounded
  request handling, pagination rules, and the contract for future cancellable
  jobs.
- A versioned per-user HyperAI home, restrictive permissions, a local SQLite
  metadata store, migration refusal, and backup/recovery behavior.
- A Rust TUI foundation isolated from domain crates.
- Explicit supported target triples and an OS lifecycle test matrix.
- Stable CLI, HTTP, browser-shell, Rust-toolchain, and workspace compatibility
  requirements.
- Exact dependency selections for the TUI and the future stable-v1 ACP client.
- A reproducible source-build and local install/lifecycle exercise against a
  temporary user home. This is not a signed public installer or release.

### Excluded

- Any provider, seller, registry, market-data, model, Fleet, chain, broker,
  wallet, remote worker, or public-network connection.
- Copying Bazaar code, schemas, SQL, models, datasets, benchmark material,
  fixtures, generated output, or visual assets into HyperAI or its build graph.
- Paid or remote evaluation; SSH, allocation launch, workload start, or
  termination; external evidence publication; escrow or settlement; financial
  Trade orders; or JustAI runtime integration.
- General-purpose ACP tool execution. Phase 1 freezes the SDK and protocol
  choice only; the ACP client is a later slice and starts read-only.
- Migration of existing market receipt journals, outcome records, HyperIntel
  stores, or nested product data into the new local metadata database.
- Installing background services at login, remote binding, auto-update,
  public release, or production deployment.

## Contract decisions

| Area | Phase 1 decision | Compatibility and boundary |
|---|---|---|
| Product composition | Keep one HyperAI shell and one Rust application facade. Add module-owned ports behind the facade; use adapters at product boundaries. | The UI reports source, freshness, authority, and claim limits. It is not a new source of truth for listings, quotes, execution, outcomes, custody, or funds. |
| Service | Evolve the existing Axum host; do not create a second daemon. Default to `127.0.0.1:4317`; support an explicit port override and port `0` for tests. Never bind `0.0.0.0`, IPv6 wildcard, or a LAN address in this slice. | `hyperai-ui` remains a compatibility foreground launcher for the same service until separately deprecated. No OS auto-start integration. |
| Service ownership | One instance per HyperAI home, guarded by an OS process lock and a versioned runtime record. Readiness is a live probe tied to the process instance, not a PID-file check. Stop is graceful first; crash recovery rotates the session capability before serving again. | A stale record never authorizes a second writer or proves a process is live. `doctor` may report unknown instead of guessing. |
| API transport | HTTP/1.1 JSON REST at `/api/v1`, served from the same loopback origin as the browser assets. Keep Axum and Tokio already in the root workspace. | No GraphQL, gRPC, WebSocket, or remote listener in Phase 1. Add routes and fields only; preserve existing route paths, methods, response shapes, and schema identifiers. |
| API errors | Preserve `hyperai.interface.error.v1` with `schema_version`, `status`, `code`, and generic `message`. Common mappings are `400 INVALID_JSON`, `401 UNAUTHENTICATED`, `403 FORBIDDEN`, `404 NOT_FOUND`, `409 CONFLICT`, `413 BODY_TOO_LARGE`, `422 INVALID_REQUEST`, `429 RATE_LIMITED`, and `500 INTERNAL_ERROR`; keep existing business codes stable. Add correlation through an `X-Request-Id` response header, not a required error-body field. | Accept client request IDs only when they match `[A-Za-z0-9_-]{1,64}`; otherwise return `400 INVALID_REQUEST` and a fresh response ID. Do not return filesystem paths, SQL text, provider details, secrets, prompts, or stack traces. Keep current route response bodies readable by existing clients. |
| API limits | Retain the existing 1,048,576-byte global JSON request cap; allow lower per-route caps. Require JSON content type, reject malformed or unknown schema versions with a typed client error, and impose a 30-second ceiling on synchronous API work. | A future operation that can exceed the synchronous ceiling must use a named async job contract; do not silently hold an HTTP request open. |
| Pagination | New collection routes use deterministic keyset pagination: default 50 rows, maximum 100, opaque cursor bound to the canonical filters, sort order, and source snapshot/release digest or local database revision. | No offset pagination. Invalid, mismatched, or expired cursors fail with `400 INVALID_REQUEST` or `409 CONFLICT` when the bound snapshot changed. Existing non-paginated routes stay unchanged. |
| Cancellation | Phase 1 defines, but does not expose an empty generic job engine. The first async feature adds `POST /api/v1/jobs`, `GET /api/v1/jobs/{id}`, and `POST /api/v1/jobs/{id}/cancel`; creation requires an idempotency key. Cancellation is cooperative and returns the actual terminal state (`cancelled`, `completed`, or `failed`), never a promise that completed effects were undone. | No server event stream in Phase 1. Clients poll bounded job status when a later feature introduces jobs. A client disconnect is not reported as successful cancellation. |
| Browser and local-client auth | Keep the service loopback-only and authenticate every API operation except a minimal, non-sensitive liveness/status response. Browser sessions use an HttpOnly, SameSite cookie, exact Host/Origin checks, and a same-origin CSRF token for state-changing requests. CLI/TUI clients read a random per-instance bearer capability from an owner-only runtime file. | No wildcard CORS, token in URL/query, argv, environment, logs, or browser local storage. On Unix use `0700` directories and `0600` files; on Windows remove broad inherited ACLs and grant the current user. The same-user malware boundary is not solved by loopback auth. |
| Local home | Resolve `--home` first, then `HYPERAI_HOME`, then an OS-standard per-user home: macOS `~/Library/Application Support/HyperAI`; Windows `%LOCALAPPDATA%\HyperAI`; Linux `$XDG_DATA_HOME/hyperai` or `~/.local/share/hyperai`. | All paths stay under that home. No credential discovery, network request, silent overwrite, or migration of another product's files during `init`. |
| Local metadata | Use the existing root SQLx dependency at `0.8.6` with SQLite for service metadata and schema version only. Keep a versioned `config.json`, `metadata/desk.sqlite3`, `runtime/`, `logs/`, and `backups/` under the home. Use a single service writer, SQLite WAL, a five-second busy timeout, atomic config writes, and a consistent SQLite backup before migration. | No private job input/output, prompt, trace, credentials, scorer, holdout, market source payload, outcome record, or provider receipt in this store. Unknown newer schema versions fail closed. Existing domain journals and nested product stores remain where they are. |
| Local metadata backup and restore | Add offline CLI commands `hyperai storage backup` and `hyperai storage restore --backup-name NAME`; snapshots contain only the initialized schema-v1 `desk_meta` store and live under this home’s protected `backups/` directory. Backup uses SQLite `VACUUM INTO` for a consistent snapshot and returns a versioned JSON receipt with its basename and SHA-256. | Backup does not require the service to stop. Restore requires exclusive service-home ownership and the initialization lock, accepts only an allowed basename from this home, verifies file type, schema, exact table shape, and SQLite integrity, saves a pre-restore snapshot, then replaces metadata rows in one SQLite transaction. A failed restore leaves the live rows unchanged. Migration snapshots remain migration-recovery artifacts and are not accepted by the manual restore command. These are local CLI operations, not HTTP endpoints. |
| TUI | Add an isolated `hyperai-tui` crate that owns terminal rendering/input and calls typed application/service APIs; it must not contain business rules. Pin `ratatui = { version = "=0.30.2", default-features = false, features = ["crossterm"] }` and direct `crossterm = "=0.29.0"`. Ratatui 0.30.2's `crossterm` feature selects its Crossterm 0.29 backend. | Keep one Crossterm version in the dependency graph and one event reader for the terminal queue. Restore raw mode, alternate screen, and cursor on normal exit, panic, interrupt, and startup failure. Piped/non-TTY commands remain plain text or machine output. |
| Directory mapping | Pin `directories = "=6.0.0"` for known-folder/XDG path resolution; keep explicit `--home` precedence. | Do not silently change the data root when upgrading the helper crate. |
| ACP SDK | For the later ACP slice, use the official Rust SDK crate `agent-client-protocol = "=2.2.0"`, stable protocol v1 only (locked schema crate `1.9.1`); do not enable `unstable_protocol_v2` or the umbrella `unstable` feature. Keep ACP in a future `hyperai-acp` adapter, not the domain facade. | The root toolchain stays Rust `1.93.1`; pin through `Cargo.lock` and `--locked`. No ACP process is launched in Phase 1. |
| Rust command compatibility | Add a real `hyperai` binary target as the primary executable. Keep `hsai` as an alias to the same command implementation. Keep `hyperai-ui` as a compatibility service launcher. Set `default-run = "hyperai"`, but scripts still select their binary explicitly. | Preserve existing `market` and `outcomes` command names, flags, exit-code meanings, JSON shapes, stdout machine output, and stderr diagnostics. |
| JavaScript entrypoints | Make the `hyperai` script invoke `cargo run -p outcomes-cli --bin hyperai --`; make `hsai` invoke `--bin hsai`; retain `interface:local` invoking `--bin hyperai-ui`. | Use `corepack pnpm` and `scripts/with-pinned-rust.sh`. The current unqualified `cargo run -p outcomes-cli` scripts must not be treated as a compatibility guarantee while the package has multiple binaries and no `default-run`. |
| Nested workspaces | Root Cargo owns the existing HyperAI outcome-protocol crates and HIP-4 records, `hyperai-market`, its application facade, service/UI composition, and later independently authored modules. Keep HyperIntel, JustAI, and Simple Compute Market on their pinned nested build graphs. | No dependency direction from outcomes to market vocabulary; no flattening of product persistence, test fixtures, or authority. |

### Manual metadata backup and restore contract

The following maintenance commands operate only on the selected HyperAI
home's `metadata/desk.sqlite3`. `--home` keeps its existing precedence over
`HYPERAI_HOME` and the platform default:

```text
hyperai storage backup [--home PATH]
hyperai storage restore --backup-name NAME [--home PATH]
```

`backup` requires an initialized schema-v1 home. It takes a consistent SQLite
snapshot into `backups/` without stopping the local service and never replaces
an existing file. Generated names are
`desk-manual-<unix-nanoseconds>-<pid>-<sequence>.sqlite3`. Its stdout JSON
schema is `hyperai.local-storage-backup.v1`, with `status: "created"`,
`backup_name`, `database_schema_version: 1`, `file_bytes`, and
`backup_sha256`. It does not emit the home path, database rows, keys, or
values.

`restore` requires an initialized schema-v1 home and an idle service home. It
holds the existing service lock and initialization lock through validation
and restore, so another service cannot start during the operation. `NAME`
must be a single generated manual-backup or pre-restore basename in this
home's `backups/`; absolute paths, separators, traversal, symlinks, and other
filenames are rejected. The selected database must be a regular file with
`user_version = 1`, an `integrity_check` result of `ok`, and exactly the
expected `desk_meta(key TEXT PRIMARY KEY, value TEXT NOT NULL)` table. Older
pre-migration snapshots and unknown or malformed schemas are refused.

Before changing metadata, restore creates a consistent safety snapshot named
`desk-before-restore-<unix-nanoseconds>-<pid>-<sequence>.sqlite3`. It replaces
only the `desk_meta` rows in a transaction; the current schema and
`user_version` remain 1. A failure before commit leaves the current rows
unchanged, and the safety snapshot is retained if it was already created.
Success stdout uses schema `hyperai.local-storage-restore.v1` and reports
`status: "restored"`, `selected_backup_name`, `selected_backup_sha256`,
`safety_backup_name`, `safety_backup_sha256`, and `metadata_row_count`. It
never returns metadata keys or values.

CLI failures retain the existing `{"error_code", "message"}` shape and use
stable codes and existing exit-code classes: `HOME_NOT_INITIALIZED`,
`INVALID_BACKUP_NAME`, and `BACKUP_NOT_FOUND` exit 2; `HOME_BUSY` exits
3; `UNSUPPORTED_SCHEMA` and `INVALID_BACKUP` exit 4; and
`STORAGE_OPERATION_FAILED` exits 6. Messages never include local paths, SQL
text, or database values; a post-snapshot restore failure includes the safe
recovery basename. Backup or safety-snapshot creation
failure must not mutate current metadata. Restore must validate the source and
successfully create the safety snapshot before starting its transaction. No
REST route or additional daemon capability is introduced.

### API details

The existing service is an Axum binary that binds IPv4 loopback on port 4317,
has a 1 MiB JSON body cap, serves same-origin static assets, and exposes
versioned status, synthetic market, receipt-verification, SCM-capture, and
HyperIntel verification/reconciliation routes. It currently has response
security headers but no session/token authentication, Host/Origin validation,
or CORS policy. Phase 1 must add the authentication contract before adding
new service capabilities.

The API uses JSON request and response bodies. Existing success payloads and
the existing error body are preserved. New response types include their own
`schema_version`. `X-Request-Id` is generated by the service when absent and
validated when supplied; it is an opaque diagnostic identifier, not an
authorization token. Reject unknown hostnames and unexpected origins. Keep
the status probe minimal and free of home paths, user identity, secrets, and
record contents.

Compatibility preserves current CLI arguments/output and current API route
methods, paths, request/response bodies, and schema identifiers. The current
local host has no authentication and no released third-party API contract;
Phase 1 therefore intentionally tightens access control. The bundled browser
shell, CLI, TUI, and any documented local client must migrate to the session
or bearer contract in the same implementation slice. Do not describe
unauthenticated direct callers as backward-compatible.

When the browser shell loads, the service creates a short-lived session tied
to that service instance, sets the HttpOnly cookie, and exposes a separate
CSRF value only to the same-origin shell in the bootstrap response. The shell
keeps the CSRF value in memory and sends it as a header for state-changing
requests. Rotate both values on service restart and clear them on clean stop.
The local CLI/TUI bearer file is readable only by the current OS account and
is never passed as a command argument or environment variable.

For future async work, `POST /api/v1/jobs` accepts a typed versioned request
and `Idempotency-Key`; identical retries return the original job reference,
while a reused key with different canonical request bytes returns conflict.
Job status exposes bounded progress and an explicit terminal state. A cancel
request is idempotent; a completed job returns `completed`, and no response
claims that a completed external effect has been rolled back. The routes are
not enabled until a named Data, Fleet, or Eval feature supplies a bounded job
type and its own authority rules.

## Supported build targets

These are proposed Phase 1 core-desk support targets, not tested claims. They
cover the local service, CLI, and TUI; an optional adapter may have a narrower
platform matrix. The release matrix must test every target before calling it
supported:

| Proposed runtime floor | Architectures and Rust targets | Terminal/runtime notes |
|---|---|---|
| macOS 14 Sonoma or later | Apple Silicon `aarch64-apple-darwin`; Intel `x86_64-apple-darwin` | Run service, cookie/session checks, SQLite lifecycle, and TUI restoration on both architectures. |
| Windows 11 25H2 or later on a Microsoft-serviced General Availability release | `x86_64-pc-windows-msvc` | Exercise current-user ACLs, child-process start/stop/crash recovery, console resize/interrupt, and terminal restoration. Recheck the supported-version floor at each release. |
| Ubuntu 24.04 LTS or later | `x86_64-unknown-linux-gnu`, glibc 2.39 baseline | Exercise XDG path precedence, Unix modes, process locking, SQLite backup/recovery, and terminal restoration. |

Windows ARM64, Linux ARM64, BSD, mobile, browser-only service hosting, and
other terminals/operating systems remain outside the first support promise.
ACP uses Unix process groups and has an optional Windows Job Object
implementation behind the default-off `windows-acp` feature. Locked MSVC
all-target checks and strict Clippy pass with that feature both disabled and
enabled, but this is compile/static evidence only. Windows builds with the
feature disabled and other unsupported targets fail closed with
`ACP_PLATFORM_UNSUPPORTED`. Windows desk support does not enable ACP sessions:
native Windows connect/disconnect, failed-handshake, cancellation, and
descendant-cleanup tests must pass before product configuration may enable the
feature. See the [Phase 8 ACP platform scope](compute-bazaar-phase-8-acp-packet.md#platform-scope).
The current checkout host is macOS 26.6.1 arm64; that establishes neither
Intel macOS nor Windows/Linux compatibility. Refreshed on 2026-09-25 against
the linked vendor records: Windows 11 25H2 remains a General Availability
release, with Home/Pro updates listed through 2027-10-12; 26H1 is scoped to new
devices, so the target floor remains a Microsoft-serviced 25H2-or-later GA
release. Ubuntu 24.04 LTS remains in standard security maintenance through May
2029, and the “24.04 LTS or later” floor accommodates newer LTS releases.
Recheck the OS floors at each release. See [Microsoft's Windows 11 release
information](https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information)
and [Ubuntu's 24.04 LTS lifecycle](https://ubuntu.com/about/release-cycle?product=ubuntu&release=ubuntu&version=24.04+LTS).

Current platform-floor check (2026-10-01): Microsoft now lists Windows 11
26H2 as generally available from 2026-09-29, with Home/Pro updates through
2028-10-10; 25H2 remains generally available with Home/Pro updates through
2027-10-12. The proposed 25H2-or-later floor remains viable; 24H2 is near its
Home/Pro end-of-updates date on 2026-10-13. Canonical lists Ubuntu 24.04 LTS
standard support through May 2029 and 26.04 LTS through May 2031, so the
24.04-or-later floor remains within standard maintenance. Apple's Xcode 26.1
matrix lists macOS 11–26.1 deployment targets but requires macOS 15.6–26.x to
run that Xcode version. Thus the proposed macOS 14 runtime floor is within the
listed deployment range, while its native build/test host matrix is a separate
decision. These vendor/toolchain facts do not qualify HyperAI on any target or
establish Apple's OS security-support window. Keep all rows proposed until the
native service, storage, CLI, and TUI gates pass on each target. See [Microsoft's current release table](https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information), [Canonical's release lifecycle](https://ubuntu.com/about), and [Apple's Xcode system requirements](https://developer.apple.com/xcode/system-requirements/).

Build-host qualification is separate from the runtime target matrix. Each
declared target needs a compatible pinned Rust toolchain and native SDK/linker
environment recorded in its test packet; a target triple or successful
cross-compile does not establish native lifecycle behavior.

### Required native build and runtime matrix

Before a target is described as supported, build it with the pinned toolchain
and run the service, storage, CLI, and TUI acceptance gates on a native host of
the same OS and architecture. Cover both the proposed minimum runtime version
and the latest generally available release that still supports that
architecture; if they coincide, one cell is sufficient. Rosetta, an emulated
container, or a cross-compile may provide supplemental evidence but does not
substitute for the native run. Record the exact OS build, hardware architecture,
Rust toolchain, SDK/linker versions, command, and result for each cell.

| Rust target | Required host | Runtime version coverage | Current evidence |
|---|---|---|---|
| `aarch64-apple-darwin` | Native Apple Silicon Mac with the pinned Rust toolchain and Apple SDK/linker | macOS 14 minimum and latest GA macOS release supporting Apple Silicon | Same-host arm64 qualification exists on macOS 26.6.1; macOS 14-floor coverage remains open. |
| `x86_64-apple-darwin` | Native Intel Mac with the pinned Rust toolchain and Apple SDK/linker | macOS 14 minimum and latest GA macOS release supporting Intel | Rosetta workspace checks on Apple Silicon are supplemental; native Intel coverage remains open. |
| `x86_64-pc-windows-msvc` | Native x64 Windows host with the MSVC toolchain and Windows SDK | Windows 11 25H2 minimum and latest serviced GA version (26H2 as of 2026-10-01) | MSVC cross-checks did not build the service/TUI/CLI graph; native build and runtime coverage remain open. |
| `x86_64-unknown-linux-gnu` | Native x86_64 Ubuntu host with the pinned Rust toolchain and glibc 2.39 baseline | Ubuntu 24.04 minimum and latest GA LTS (26.04 as of 2026-10-01) | Container/emulated Linux checks are supplemental; native Ubuntu coverage remains open. |

At each release, recheck the latest serviced operating-system release and
retire any floor that is no longer serviced before publishing a support claim.
This matrix defines the evidence needed for a support claim; it does not
authorize a public release.

## Source, data, and reuse decisions

| Source | Phase 1 decision | Later condition |
|---|---|---|
| HyperAI-authored synthetic data | Allowed for local fixtures and UI demonstration. Identify it as synthetic and deterministic. | It never establishes seller availability, correct computation, payment entitlement, or production performance. |
| Simple Compute Market | Keep the already imported MIT-licensed source in its nested workspace. Do not alter its build graph or call its hosted registry/storefront in Phase 1. | MIT covers source-code reuse subject to its terms; seller listing contents, signatures, and capture rights/retention are separate. Any adapter needs a named source, caller-supplied bytes, provenance, freshness, and retention contract. |
| The Compute Bazaar | Keep `references/the-compute-bazaar-main/` ignored and read-only. No copied code, schemas, SQL, model/view definitions, data, benchmarks, fixtures, generated output, assets, or package dependency. | No Bazaar-derived ingestion or redistribution until a rights holder grants written code and data rights for the exact assets and use. Independently authored behavior may follow high-level documented concepts without copying protected expression. |
| HyperIntel | Use only the committed source snapshot recorded in provenance through the existing offline adapter. Do not copy uncommitted source from the separate HyperIntel checkout. No provider call or inference route in Phase 1. | Any additional source revision, execution, provider access, or data transfer needs its own HyperIntel-owned slice and adapter contract. |
| JustAI | No runtime call, code/data import, custody operation, paid execution, or public projection in Phase 1. Preserve its independent charter, status, and evidence/custody boundaries. | A one-way metadata projection needs a JustAI owner decision that the exact use fits its charter and roadmap, plus separate disclosure, retention, revocation, and evidence gates. |
| Market/provider feeds | No automated source is approved for ingestion in this packet. | Before an adapter, record source owner, license/terms, permitted fields, classification, authentication, freshness, revision semantics, retention/deletion, provenance, schema, network scope, and an explicit source-specific state slice. |

The exact provenance pins are SCM `a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`,
Bazaar `d1809e5cfa843c087ebc4de9f00561bf0be67d55`, HyperIntel
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`, and JustAI
`9a0e56536b968cbb2cb7173e142068b53c4f9fe3`. The linked provenance record
remains authoritative for snapshot composition and file-tree digests.

No data is fetched during initialization, service start, `doctor`, or TUI
startup. An offline user-selected capture stays under its existing adapter
contract and is not copied into the new metadata database.

## Proposed implementation file map

The accepted code state slice is `hyperai.bazaar.desk_runtime.v1`. Its first
patch set is limited to these files:

| File | Phase 1 responsibility |
|---|---|
| `Cargo.toml`, `Cargo.lock` | Register the two new root-owned crates and lock the selected Phase 1 dependencies. Use the already-locked `sha2` and `futures-util` workspace dependencies in `hyperai-local-service` for backup-file digests and bounded row streaming. Do not add the ACP SDK until its Phase 8 implementation. |
| `crates/hyperai-local-service/Cargo.toml`, `src/lib.rs`, `src/auth.rs`, `src/lifecycle.rs`, `src/storage.rs`, `src/server.rs` | New service boundary for the existing Axum router, local lifecycle, versioned config/SQLite metadata, loopback auth/session, request bounds, and readiness. `storage.rs` also owns offline schema-v1 metadata backup/restore; no HTTP route is added. It composes `hyperai-application` and owns no market or outcome domain rules. |
| `crates/hyperai-tui/Cargo.toml`, `src/lib.rs` | New terminal client crate using the selected Ratatui/Crossterm versions. It connects to the service API and contains no domain policy. |
| `crates/outcomes-cli/Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/bin/hsai.rs`, `src/bin/hyperai-ui.rs` | Make `hyperai` the primary target, retain an `hsai` alias, keep `hyperai-ui` as a compatibility launcher, and add the offline `storage backup|restore` commands with stable JSON and sanitized errors. |
| `apps/hyperai-interface/index.html`, `apps/hyperai-interface/app.js` | Bootstrap the browser session and CSRF header without persisting credentials in browser storage; preserve current UI routes and synthetic workflows. |
| `crates/hyperai-local-service/tests/service_lifecycle.rs`, `api_contract.rs`, `storage.rs` | Exercise the local service, compatibility, auth, bounds, migration, schema-v1 backup/restore, failure atomicity, and recovery contracts with synthetic data and temporary homes. |
| `crates/hyperai-tui/tests/terminal_restore.rs` | Exercise terminal cleanup and non-TTY behavior through a controllable backend. |
| `scripts/verify_hyperai_tui_ctrl_c.py` | Reproduce the POSIX local-service/TUI Ctrl+C PTY lifecycle using only Python's standard library, a disposable home, and an already built `hyperai` binary. It performs no network access beyond IPv4 loopback. |
| `crates/outcomes-cli/tests/cli_compat.rs` | Capture the `hyperai` and `hsai` command/flag/output compatibility contract, including storage backup/restore reports and failures. |
| `package.json` | Give each script an explicit `--bin` target, retain `interface:local`, and expose the bounded `verify:tui-ctrl-c` check. |
| `README.md`, `docs/state-slices.md`, `docs/compute-bazaar-phase-1-contract-packet.md`, `docs/compute-bazaar-rust-port-subplan.md` | Document launch/connection behavior, record exact state-slice authority and gates, and report only evidence actually produced. |

This map excludes edits to `products/hyperintel/`, `products/justai/`,
`products/simple-compute-market/`, `references/the-compute-bazaar-main/`,
existing market/outcome domain contracts, import manifests, and source/data
artifacts. If implementation needs another file, stop and amend the accepted
slice before touching it.

## Phase 1 acceptance gates

The future implementation slice can claim only local synthetic behavior after
all applicable gates below pass. Passing these gates alone is not independent
review or production authorization.

1. **Build and compatibility:** use the pinned Rust toolchain and locked
   dependencies; build every declared Rust target; prove `hyperai`, `hsai`,
   and `hyperai-ui` launch the expected entrypoints; capture golden CLI output,
   flags, exit codes, and current HTTP route schemas before changing code.
2. **Service lifecycle:** repeat start/status/stop, foreground start, duplicate
   start, occupied-port refusal, forced crash, stale runtime-record recovery,
   restart token rotation, and graceful shutdown with a temporary home.
3. **Network boundary:** assert listeners bind only to IPv4 loopback; reject
   unexpected Host/Origin, missing auth on protected routes, bad CSRF, and
   wildcard CORS behavior; ensure no provider or public-network request occurs.
4. **API contracts:** retain current route methods and bodies; verify stable
   error codes, request IDs, 1 MiB cap, per-route lower caps, 30-second
   deadline, cursor mismatch/expiry, idempotency conflict semantics, and
   cancellation terminal-state rules before a job-using module enables jobs.
5. **Storage:** test first init, repeated init without overwrite, custom-home
   precedence, owner permissions/ACLs, path traversal and symlink refusal,
   SQLite migration, manual backup digest and consistent contents, restore
   round-trip and pre-restore safety snapshot, rejection of traversal,
   symlink, corrupt, wrong-shape, old, and newer-schema backups, and atomic
   no-change behavior on every rejected or failed restore. Verify backup and
   restore lock behavior, disk-full failure, and process-lock recovery. Verify
   no existing market, outcomes, HyperIntel, JustAI, or SCM data is moved or
   rewritten.
6. **TUI:** verify the locked graph uses only Crossterm 0.29 with Ratatui's
   matching backend and one terminal event reader. Test narrow terminal layout,
   UTF-8, resize, non-TTY behavior, startup failure, panic, Ctrl-C, normal
   exit, and restoration of raw mode, alternate screen, and cursor on each
   supported OS.
7. **Platform matrix:** satisfy the [required native build and runtime
   matrix](#required-native-build-and-runtime-matrix) at the minimum and latest
   serviced OS versions for each declared architecture. On each native cell,
   run service lifecycle, auth/path, CLI compatibility, SQLite backup/restore,
   and TUI restoration gates. Rosetta, emulated containers, and cross-compiles
   are supplemental only. An untested cell is unsupported, not inferred from
   another OS or architecture.
8. **Install boundary:** source-build into a temporary location and exercise
   local upgrade/rollback against disposable data. No signed or public package
   is produced by this packet.

Report results separately by root workspace and each nested workspace. For
any future evidence packet record exact revision, state slice, assurance
level, reviewer status, reproduction status, data/artifact custody, known
limitations, claim ceiling, and next authority gate. This planning packet has
no runtime assurance label.

## Phase 1 readiness audit — 2026-09-28

The Phase 1 contract decisions are frozen in this packet. The local runtime
was implemented under the separate accepted
`hyperai.bazaar.desk_runtime.v1` state slice; this section summarizes its
current evidence and remaining exit gates. It assigns no overall assurance
level and does not convert proposed build targets into supported platforms.

| Area | Current evidence | Still required for the Phase 1 exit claim |
|---|---|---|
| Service, API, and compatibility | The current root lint and focused service/CLI suites pass for the recorded working tree. Local lifecycle, loopback authentication, browser session/CSRF, stable error envelopes, and command aliases are implemented. | Run the complete lifecycle, API, and compatibility matrix on each declared native target. Async-job idempotency and cancellation remain dormant contracts until a module introduces a typed job; they are not implemented as a generic Phase 1 job engine. |
| Storage and recovery | Schema-v1 metadata backup/restore tests pass on macOS arm64; the focused storage, CLI, and TUI suites also pass in a clean, network-disabled Linux/amd64 container. A bounded Linux `tmpfs` exhaustion and recovery check is recorded separately. | Native Windows ACL and native Linux filesystem behavior remain unqualified. Emulated `tmpfs` evidence does not establish native filesystem exhaustion behavior. |
| CLI, TUI, and install | CLI compatibility and local source-install/service/ACP integration checks pass on macOS arm64. PTY key-byte Ctrl+C, resize redraw, and one live-session panic-unwind cleanup path have local macOS arm64 evidence; the named Linux-container suite covers its recorded terminal-restoration tests. | Physical-keyboard Ctrl+C, panic-abort and forced termination, and the native terminal path on every declared target remain unqualified. Install/upgrade/rollback still require the declared native matrix. No signed installer or public release is covered. |
| Platform matrix | Native macOS arm64 has focused local evidence. Linux/amd64 and Intel-target artifacts have same-owner emulated/container evidence for the exact recorded suites. Four Rust-only crates passed an MSVC-target compile check. | Native Ubuntu x86_64, native Intel macOS, and native Windows remain unqualified. The MSVC check does not compile the service/TUI/CLI dependency graph or establish Windows runtime behavior. Until each row passes its specified native gates, describe the table above as a proposed support matrix, not product support. |
| Data sources and rights | HyperAI-authored synthetic data is allowed for local fixtures. SCM source code remains under its pinned MIT workspace; rights to caller-supplied capture contents are separate. HyperIntel use remains limited to its pinned committed offline adapter. JustAI runtime integration is excluded. | No automated feed is approved. Compute Bazaar code and data remain excluded pending written rights for exact assets and use. Any new source adapter needs a source-specific contract, rights, classification, freshness, retention, and state slice. |

Phase 1 therefore has an implemented local foundation with bounded local
validation, but it has not passed the full acceptance matrix or release gate.
Continue other authorized local synthetic phases without implying platform
support; keep any native-platform support claim closed until the missing native
runs are available.

### Current-source qualification follow-up — 2026-10-01

On macOS 26.6.1 arm64 with the pinned Rust/Cargo 1.93.1 toolchain, the current
dirty-worktree command `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited
0. The interface suite passed 17/17, installer checks passed 3/3, and pinned
formatting, strict all-target/all-feature Clippy, root contract tests, all
workspace feature tests, and doc tests passed. The root gate's Docker/Postgres,
staged installer/sidecar, and browser checks remained environment-gated and
ignored; they are not counted as passes. The root and nested-workspace record
is [`hyperai.bazaar.monorepo_workspace_validation.v1`](state-slices.md#hyperai-bazaar-monorepo_workspace_validationv1).

This is `INTERNAL_PASS` for the exact local root command and its synthetic
scope, with reviewer status `NOT_REQUESTED` and reproducibility
`NOT_REPRODUCED`. It adds no new native-platform evidence and does not close
the declared macOS Intel, Windows, or native Ubuntu matrix; full install
upgrade/rollback coverage; physical-keyboard, forced-termination, or
cross-platform TUI behavior; clean second-host reproduction; Bazaar source or
data rights; independent review; or release readiness. Test fixtures remained
local and synthetic; no provider, registry, ACP agent, remote fleet, chain, or
public service was contacted. No test-output artifact was retained. The
working tree remains dirty and has no frozen source digest, so this result is
bound to the named command, host, toolchain, baseline revision, and state slice,
not to a redistributable build.

### Focused native Apple Silicon Phase 1 run (2026-10-01)

On native `aarch64-apple-darwin`, macOS 26.6.1 build 25G76 (Darwin 25.6.0),
the pinned Rust/Cargo 1.93.1 Phase 1 suites passed against baseline
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the dirty working tree. The
279-file source/harness inventory SHA-256 is
`a45ee047570ca7e27474fa494257c64b90d4559e3a00b8d7144b04ea80fc88ad`; the
`Cargo.lock` SHA-256 is
`84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a`. Each
inventory record is sorted relative-path UTF-8, NUL, raw SHA-256 of file
bytes, and LF. Inputs are root `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `package.json`, `pnpm-lock.yaml`, `crates/`,
`apps/hyperai-interface/`, `config/`, `schemas/`, `fixtures/`,
`products/hyperintel/`, `scripts/with-pinned-rust.sh`, and
`scripts/verify_hyperai_tui_ctrl_c.py`; `target/`, `node_modules/`, `.git/`,
`.venv/`, and `__pycache__/` are excluded.

The locked offline service command covering `api_contract`,
`service_lifecycle`, and `storage` passed 11 tests. The CLI compatibility
suite passed 14 tests with 2 staged-binary/sidecar cases ignored. The TUI
`terminal_restore` suite passed 10 tests. `CARGO_NET_OFFLINE=true corepack pnpm
run verify:tui-ctrl-c` passed PTY resize at 48x10 and 100x30, Ctrl+C exit,
alternate-screen/cursor restoration, and raw terminal restoration; its test
binary SHA-256 was
`843878c034c04210d33149f50d4c24c0915590b57be08e7a0f1eba41fcf4a4d1` and PTY
transcript SHA-256 was
`75dc388b53c6948f9892207c601503a2c45e2c389aa5befe219d19307e9f0f5e`. The
`verify:tui-panic` run observed the expected panic exit 101 and terminal
cleanup; its test binary SHA-256 was
`5e276b39166fcc75334a1c719fd00877fac1df16bdeac8b435f04f64e68680d6` and PTY
transcript SHA-256 was
`739eec4567b47ee6a9f72b75b0f1dc45c2916cc3940d3b4569ba893fda01552b`. Both
runner invocations reported removal of their disposable homes. The post-run
`python3 scripts/verify_imports.py` check passed for the pinned Simple Compute
Market, HyperIntel, and JustAI snapshots. No provider, registry, external ACP
agent, remote fleet, chain, or public service was contacted.

Exact commands were:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline --jobs 1 -p hyperai-local-service --test service_lifecycle --test api_contract --test storage`
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline --jobs 1 -p outcomes-cli --test cli_compat`
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline --jobs 1 -p hyperai-tui --test terminal_restore`
- `CARGO_NET_OFFLINE=true corepack pnpm run verify:tui-ctrl-c`
- `CARGO_NET_OFFLINE=true corepack pnpm run verify:tui-panic`
- `python3 scripts/verify_imports.py`

Assurance is `INTERNAL_PASS` for these exact local synthetic and loopback
checks; reviewer status is `NOT_REQUESTED`, and reproduction is
`NOT_REPRODUCED`. The staged sidecar CLI cases remain ignored. This adds
current-host native Apple Silicon evidence only: it does not cover macOS 14,
native Intel macOS, native Windows, native Ubuntu, or establish the complete
Phase 1 matrix, accessibility, independent review, or release readiness.
Temporary test state was removed and no raw PTY transcript was retained. See
the [`desk_runtime` state slice](state-slices.md#hyperai-bazaar-desk_runtimev1).

## Implementation snapshot

The accepted code slice is `hyperai.bazaar.desk_runtime.v1`, based on working
tree `main` at `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the uncommitted
Phase 1 changes recorded in [state slices](state-slices.md). The implementation
adds one local service and storage home, CLI aliases and lifecycle commands,
browser-session bootstrap, and a service-backed terminal desk. The Bazaar
checkout remains ignored and reference-only; no upstream Bazaar code or data
was copied, and no external service was called.

Historical validation reported on 2026-09-24 for the then-current macOS arm64
working-tree snapshot passed for `hyperai-local-service`, `hyperai-tui`, and
`outcomes-cli`: pinned-toolchain
`cargo check --locked --offline`, tests, and Clippy with warnings denied for
every target/feature in those crates. Root `corepack pnpm run lint` was
reported passing at that snapshot, including formatting, workspace Clippy,
contract tests, and the full workspace test suite; environment-gated
PostgreSQL and Docker tests were skipped. That historical result is not bound
to a frozen working-tree digest and does not establish the latest dirty-tree
result. The full root lint status recorded on 2026-09-25 failed in
`outcomes-domain::fixtures::tests::contract_generated_assets_match_typed_sources`
at `crates/outcomes-domain/src/fixtures.rs:251`; `verify:full` was not run at
that time. A 2026-09-26 follow-up under
[`hyperai.outcomes.schema_workspace_feature_parity.v1`](state-slices.md#hyperaioutcomesschema_workspace_feature_parityv1)
strictly parses and RFC 8785-compares generated schemas and noncanonical pretty
claim JSON to tolerate workspace-wide `serde_json/preserve_order` feature
unification; the canonical claim fixture remains byte-exact. The standalone
fixture test, root `verify:contracts`, and full root `lint` now pass on the
current working tree. Opt-in PostgreSQL, Docker, and ACP sidecar tests remain
ignored without their selected local dependencies. The
JavaScript shell passed `node --check`, and the `hyperai` package
script successfully printed command help. The tests exercise
local service lifecycle and session rotation; status, auth, CSRF, Host/Origin,
request-ID, JSON-size, and response-schema behavior; local storage modes,
backup and newer-schema refusal; CLI alias JSON; and TUI cleanup callback and
narrow UTF-8 layout. Additional CLI checks passed for duplicate start,
occupied-port refusal with capability cleanup, forced-process stale-record
recovery with bearer rotation, and explicit-home versus `HYPERAI_HOME`
precedence. A disposable `cargo install --debug --locked --offline` exercise
installed `hyperai`, `hsai`, and `hyperai-ui`; ran init, doctor, service start,
connect, desk, and stop; performed a same-revision forced reinstall and
restored the prior binaries; and confirmed local-home file digests were stable.
The TUI ran through a pseudo-terminal; pressing `q` restored the alternate
screen, cursor visibility, and terminal mode flags. Cargo warned that locked
transitive packages `chacha20 0.10.1` and `spin 0.9.8` are yanked; the offline
install succeeded. That warning was resolved on 2026-09-26 by the narrowly
scoped offline lock update recorded in
[`hyperai.bazaar_rust_port_dependency_yank_review.v1`](state-slices.md#hyperai-bazaar-rust-port-dependency-yank-reviewv1):
the current lockfile selects `chacha20 0.10.2` and `spin 0.9.9`, and the
focused local check passed. This resolves the two current lock entries only;
it does not complete the Phase 1 platform matrix, clean reproduction, or
release qualification. The original install result remains historical evidence
for the earlier lockfile. A local browser smoke against a disposable loopback
home also passed: the shell bootstrapped a browser session, completed the
synthetic market lifecycle over
the authenticated API, and rendered Market, Data, Fleet, and Eval with their
source and authority labels. Quote approval/completion and caller-supplied
registry-capture browser flows remain unqualified.

On 2026-09-25, this command was attempted from the macOS arm64 host:
`cargo check --locked --offline --target x86_64-unknown-linux-gnu --all-targets -p hyperai-local-service -p hyperai-tui -p outcomes-cli`.
It stopped in the `zstd-sys` build script because `x86_64-linux-gnu-gcc` is not
installed; that cross-target attempt produced no Linux build or runtime result.
Native Ubuntu x86_64, native Intel macOS, and Windows remain unqualified.

A supplemental, network-disabled Debian 12 ARM64 container probe was run with
the pre-cached official `rust:1.93.1-bookworm` image. The same three package
all-target suites passed 67 tests with Rust 1.93.1 and Cargo limited to one
build job after the default-parallel attempt hit linker memory pressure. This
is additional local Unix portability evidence only: Linux ARM64 is outside
the proposed release matrix, the declared Ubuntu x86_64 target remains
unqualified, and the container run did not retain a command-output digest or
establish `REPRODUCED_LOCAL`. Exact build/test inputs and the image identity
are recorded in the [Phase 1 state-slice entry](state-slices.md).

A supplemental Linux x86_64 probe on 2026-09-25 used the official
`rust:1.93.1-bookworm` image under Docker Desktop Rosetta emulation on Apple
Silicon. The image config ID was
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`
and the pulled manifest digest was
`sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The Debian 12 `x86_64-unknown-linux-gnu` run used Rust 1.93.1, disabled
networking, read-only source and Cargo-cache mounts, a fresh target directory,
and one Cargo job. The same three `--all-targets` package suites passed 61
tests: 25 local-service, 10 TUI, and 26 CLI. This is emulated Debian
portability evidence, not qualification of native Ubuntu 24.04 or the full
release matrix. Its command-output SHA-256 is
`ad8db9ae99eb4d58fc6be5a763ba52d6e1be33b39fd6e70287821c5a317ee45a`; the
post-run source/dependency inventory is recorded in the
[Phase 1 state-slice entry](state-slices.md).

An additional `x86_64-apple-darwin` run on 2026-09-25 used Rust 1.93.1 and
Xcode's macOS 27.0 SDK. The x86_64 test binaries ran under Rosetta on Apple
Silicon. The exact command
`cargo test --locked --offline --jobs 1 --target x86_64-apple-darwin --all-targets
-p hyperai-local-service -p hyperai-tui -p outcomes-cli` passed 71 tests: 25
local-service, 10 TUI, and 36 CLI. Its command-output SHA-256 is
`4b4a6d32a2e73cb42152e91051c2855aa14b2d28c93e012a5845af9e265fb52a`. This
exercises Intel-target artifacts on macOS; native Intel hardware qualification
remains open.

A supplemental Ubuntu 24.04.5 x86_64 container run on 2026-09-25 used the
locally derived test image
`sha256:d585a391439094f577f80bfdd4335b2d0947de24376e835644e2278e13e4fc34`,
with Rust 1.93.1 and Cargo 1.93.1. Under Docker Desktop Rosetta emulation on
Apple Silicon, networking disabled, read-only source and Cargo-cache mounts,
a fresh target directory, and one Cargo job, the exact command
`cargo test --locked --offline --jobs 1 --all-targets -p hyperai-local-service
-p hyperai-tui -p outcomes-cli` passed 67 tests: 24 local-service, 7 TUI, and
36 CLI. The output SHA-256 is
`196212505ac1254399dfc4b33f5b1165bbd9ce2478ab3b3ca33dc78890b3ffac`.
The build/test-input inventory SHA-256 is
`b27ff2ae29152f616793348063ac24f732ca81f0364c4f5f58fc0d897087e198`; image
base identities, runtime details, and inventory method are recorded in the
[Phase 1 state-slice entry](state-slices.md). This exercises Ubuntu userspace
and x86_64 binaries in an emulated container, not native Ubuntu hardware.
Native Ubuntu x86_64, native Intel macOS, and Windows qualification remain
open.

A native macOS 26.6.1 arm64 run on 2026-09-25 used the pinned Rust 1.93.1
toolchain and Cargo 1.93.1. The same three package suites passed 67 tests: 24
local-service, 7 TUI, and 36 CLI. Its command-output SHA-256 is
`1c64a6d40720fbfe2d561f72972ed27c5bd459c399dd4a482ac8a48b53076519`.
The source-input inventory digest matches the 270-file inventory recorded in
the [Phase 1 state-slice entry](state-slices.md). The run used the existing
host Cargo target directory, so it is not a clean reproduction. It qualifies
only the native macOS arm64 package-test scope; native Ubuntu, native Intel
macOS, and Windows qualification remain open.

On 2026-09-25, a macOS arm64 pseudo-terminal run started the local service in a
fresh temporary home, launched `target/debug/hyperai desk` with `--home`
pointing to that home, waited for the workspace to render, and sent the Ctrl+C
key event (`0x03`). Desk exited with
code 0, restored the exact pre-run PTY termios state, left the alternate screen,
and showed the cursor. The tested binary SHA-256 was
`c1d9768857d81b5edf0323d7b7d62d414e3278787112b2356c8cbc63e12daff5`. This
qualifies the PTY Ctrl+C key-event path only; a Ctrl+C generated by a physical
keyboard and a panic during a live TUI session remain unqualified.

No overall assurance label is assigned. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `REPRODUCED_LOCAL` for the exact three-package offline test
command across native macOS arm64 and a clean Ubuntu 24.04.5 container; the
full platform matrix remains unreproduced, and same-owner reproduction is not
independent review. Data/artifact custody: source remains in the working tree,
and test/install homes and command logs are temporary. Known limitations:
quote approval/completion and caller-supplied registry-capture browser flows,
physical-keyboard Ctrl+C and panic recovery during a live TUI session, Windows
ACL behavior, native Ubuntu 24.04 x86_64, native Intel macOS, and the full
declared platform matrix have not been qualified. The panic check
exercises the cleanup guard during unwind, not a panic in a live terminal
session. Claim ceiling: the local
synthetic behavior covered by those focused checks on this host. The next
engineering gate is full Phase 1 qualification per supported target. Root
workspace validation does not replace standalone HyperIntel, JustAI, or Simple
Compute Market validation; those workspaces were not changed or run in this
slice. Provider,
remote Fleet, paid Eval, financial Trade, publication, and production still
require separate state slices and authority.

## Handoff after Phase 1

After the shared service contract is implemented and qualified locally:

1. Data may add source-labelled metadata and a local query store only after
   its per-source rights and retention packet is complete. Bazaar data stays
   excluded while its rights remain unresolved.
2. Market adapters remain read-only until their own seller/registry and buyer
   consent contracts are authorized. Quotes and fulfillment remain seller
   owned; observations never authorize settlement.
3. Fleet begins with records and preflight. Launch, SSH, metering, termination,
   or live operator access each require a distinct operation slice.
4. Eval starts with frozen, local synthetic or explicitly cleared suites.
   Paid/provider evaluation is a separate authority gate with protected
   holdouts and scorer custody.
5. Trade begins as research and paper scenarios over versioned data. Price
   observations are advisory; financial orders, perps, oracle publication,
   escrow, and settlement require a separate product plan and authority.
6. ACP uses the selected official stable-v1 SDK in its own adapter, with
   read-only capabilities first. It inherits service policy and cannot approve
   its own actions.
7. JustAI, ESN-JustNet, external publication, testnet writes, and production
   deployment remain separate owner-authorized integrations.

The accepted state-slice record in [`state-slices.md`](state-slices.md)
authorizes only the local Phase 1 implementation described here. The service,
storage, API, TUI, and compatibility choices above are the implementation
contract; provider access, live Fleet operations, paid evaluation, financial
Trade, public publication, and production deployment remain closed.

## Technical references

- [Ratatui feature flags](https://ratatui.rs/installation/feature-flags/),
  [installation](https://ratatui.rs/installation/), and
  [backend guidance](https://ratatui.rs/concepts/backends/): feature-to-version
  mapping, selected Crossterm backend, and compatible terminal dependency.
- [Official ACP Rust SDK 2.2.0 API](https://docs.rs/agent-client-protocol/2.2.0/agent_client_protocol/)
  and [ACP v1 initialization](https://agentclientprotocol.com/protocol/v1/initialization):
  selected stable protocol and capability-negotiation surface. The SDK keeps
  stable v1 on its default builder and exposes draft v2 through the explicit
  `.v2()` API/feature path.
- [`directories` 6.0.0 documentation](https://docs.rs/directories/6.0.0/directories/):
  platform-specific user-directory resolution.

### Windows MSVC target cross-check — 2026-09-28

The pinned Rust 1.93.1 `x86_64-pc-windows-msvc` standard library was installed
on the macOS arm64 host. A Windows-target check of
`hyperai-local-service`, `hyperai-tui`, and `outcomes-cli` stopped in the
`zstd-sys` C build because this host had no Windows SDK/UCRT headers. A wider
selection including Trade reaches the same dependency through
`hyperai-data`/DataFusion. The exact failure and scope are in the
[`desk_runtime` state-slice record](state-slices.md#hyperai-bazaar-desk_runtimev1).

The narrower offline command type-checked all targets for
`hyperai-foundation`, `hyperai-market`, `hyperai-fleet`, and `hyperai-eval`
against the MSVC target. This is compile-only evidence: no linking, Windows
execution, ACL behavior, service lifecycle, or terminal restoration was
tested. It is `INTERNAL_PASS` for those four crates only, not a Windows support
claim. The Windows row of the declared target matrix remains open pending a
native Windows run. Rust documents non-Windows MSVC cross-compilation as
unsupported, and Microsoft's SDK supplies the required C headers and libraries
([Rust target notes](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html),
[Windows SDK requirements](https://learn.microsoft.com/en-us/windows/win32/learnwin32/prepare-your-development-environment)).
The [`cargo-xwin` cross-build helper](https://github.com/rust-cross/cargo-xwin)
can supply the MSVC CRT/SDK for a separate cross-check, but its documented
workflow has a Microsoft SDK license condition; that workflow was not used.

### Physical tmpfs exhaustion check — 2026-09-28

The focused ignored test
`physical_tmpfs_exhaustion_rejects_metadata_write_without_partial_metadata`
now fills an exact size-limited Linux `tmpfs` until the kernel returns
`ENOSPC`, then checks the metadata write, sentinel value, and schema version.
It passed twice in fresh `--network none` Linux/amd64 containers with a 32 MiB
`tmpfs`; the second container reused the compiled target. Both runs reported
33,492,992 filler bytes and SQLite result code 13 (`SQLITE_FULL`), with no
partial metadata row and schema version 1 preserved.

The source inventory was 2,698 files / 32,685,179 bytes, SHA-256
`de44b47a1e511d55f68590cf79493e426f58e4e9dc854e06b1883cd69dc750f0`. The test
binary SHA-256 was
`7007c6391e8c537e3b424aa128c0a7b8db2cdff872b3022281acf22f082b5815`. The
container image was
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`
(`rust:1.93.1-bookworm`) with Rust 1.93.1. The Docker VM ran this amd64 image
under Apple Silicon emulation. Root `corepack pnpm run lint`, pinned Rust
formatting, `python3 scripts/verify_imports.py`, and `git diff --check` passed.

Assurance is `INTERNAL_PASS` for the exact Linux `tmpfs` failure path and root
gates. Reviewer status is `NOT_REQUESTED`; reproduction status is
`NOT_REPRODUCED` because both runs used the same host and the second reused
its target. The source manifest and 4.1 GiB temporary build target were removed
after their digests were recorded. This does not test a host disk, another
operating system, native Ubuntu, power-loss recovery, or production disk
capacity. Exact slice and custody details are in
[`hyperai.bazaar.desk_runtime.physical_enospc.v1`](state-slices.md#hyperai-bazaar-desk_runtime_physical_enospcv1).

### Locked TUI and ACP dependency graph audit (2026-09-30)

State slices: `hyperai.bazaar_rust_port_phase1_packet.v1`,
`hyperai.bazaar.desk_runtime.v1`, and `hyperai.bazaar.acp.v1`. This is a
read-only contract audit; it changes no Rust dependency, feature, or toolchain.

The pinned offline `cargo tree --locked --offline` graph matches the packet:

- `hyperai-tui` uses `ratatui 0.30.2` with default features disabled and its
  `crossterm` feature. The direct and Ratatui backend dependencies resolve to
  one `crossterm 0.29.0`; `ratatui-crossterm 0.1.2` uses its
  `crossterm_0_29` feature.
- `hyperai-acp` uses `agent-client-protocol 2.2.0` with only its empty default
  feature set. `Cargo.lock` binds `agent-client-protocol-schema 1.9.1`. Neither
  `unstable_protocol_v2` nor the umbrella `unstable` feature is enabled.
- `hyperai-mcp` resolves the same ACP SDK through its direct dependency and
  `agent-client-protocol-rmcp 3.1.1`; it does not enable draft protocol
  features.

The exact commands were `cargo tree --locked --offline -p hyperai-tui
--no-dedupe -e features -i ratatui`, the corresponding `-i crossterm` query,
the `hyperai-acp` inverse query for `agent-client-protocol`, and the two
`hyperai-mcp` inverse queries for `agent-client-protocol-rmcp` and
`agent-client-protocol`, all run through `scripts/with-pinned-rust.sh`. The
working-tree `Cargo.lock` SHA-256 is
`84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a`;
TUI, ACP, and MCP manifest SHA-256 values are
`45dd71ef6afe986dd73f4cf6ebb4298d30d7babc3926bbb1165a877af571cc1c`,
`86db2fa85795ffcf7d89876526f8eb21948353fa5bdd48bc490aad14c75a726c`, and
`6141d8f52750fab10aabdd33347a82176f3b238c6fbe0707947682046ea304bf`,
respectively. This is `INTERNAL_PASS` for locked dependency selection and
feature resolution at revision `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`
plus its dirty worktree. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. It does not qualify native Windows/Linux behavior, terminal
accessibility, or release support; those Phase 1 gates remain open. No
external data or runtime artifact was accessed or created.

### Current-source locked dependency graph recheck — 2026-10-02

State slice: [`hyperai.bazaar.phase1_dependency_graph_recheck.v1`](state-slices.md#hyperai-bazaar-phase1_dependency_graph_recheckv1).
The current `Cargo.lock` and TUI manifest have changed since the 2026-09-30
graph audit, so the five pinned offline inverse-tree queries were rerun against
the current worktree. All exited 0. The selected graph still resolves
`ratatui 0.30.2`, one `crossterm 0.29.0`, `agent-client-protocol 2.2.0`,
`agent-client-protocol-schema 1.9.1`, and `agent-client-protocol-rmcp 3.1.1`.
The ACP SDK resolves only its default feature; no draft protocol-v2 feature is
enabled. MCP and ACP converge on the same SDK version. Current input hashes
and exact commands are recorded in the state slice.

This is `INTERNAL_PASS` for current locked dependency selection and feature
resolution only. It does not qualify native Windows/Linux behavior, terminal
accessibility, ACP peer interoperability, or release support. No dependency,
feature, source, or lockfile was changed by this audit.

### Interactive TUI Market history qualification — 2026-10-02

State slice: [`hyperai.bazaar.desk_runtime.tui_market_history_interactive_pagination.v1`](state-slices.md#hyperai-bazaar-desk_runtime_tui_market_history_interactive_paginationv1).
The installed TUI traversed two authenticated, service-generated synthetic
history pages on macOS arm64. The test-owned loopback proxy constrained each
request to one row while preserving the existing TUI page size, record bytes,
service cursor, authentication, and page digests. Pressing `n` displayed the
second record and terminal state; another `n` made no third request. Terminal
restoration and disposable service/home/proxy cleanup passed.

`corepack pnpm run verify:tui-market-history-pagination`, the existing
`verify:tui-market-history` and `verify:tui-ctrl-c`, root `lint:fast`,
`python3 scripts/verify_imports.py`, `git diff --check`, and scoped whitespace
checks passed. The initial pagination attempt stopped before PTY launch because
the synthetic preview POST omitted the JSON content type; the harness was
corrected to match the existing service contract before the passing run. This
is `INTERNAL_PASS` for the exact local POSIX TUI path only; reviewer status is
`NOT_REQUESTED` and reproduction is `NOT_REPRODUCED`. A Python 3.14
fork/thread deprecation warning was emitted but did not prevent cleanup. This
does not change the Phase 1 support matrix: native macOS 14, native Intel,
Windows, Ubuntu, broad accessibility, independent review, and release gates
remain open. Exact evidence and custody are in the
[pagination packet](compute-bazaar-installed-tui-market-history-pagination-packet.md).

### Physical tmpfs restore exhaustion check — 2026-09-28

The focused ignored test
`physical_tmpfs_full_restore_rolls_back_and_retains_safety_snapshot` created a
schema-v1 store, wrote an 8,540,160-byte synthetic manual backup, and forced
actual `ENOSPC` while restoring it on an exact 32 MiB Linux `tmpfs`. The API
returned `StorageError::RestoreFailed` with a safe safety-snapshot basename and
`outcome_unknown=true`. After releasing the filler and reopening the database,
the pre-restore one-row value, schema version 1, selected manual-backup bytes,
and safety snapshot contents were all intact. This records persisted-state
preservation after recovery; it does not relabel the API's error-boundary
uncertainty as a confirmed rollback.

The focused cargo command, run from the repository root, was:

```sh
HYPERAI_PHYSICAL_FULL_TEST_ROOT=/mnt/enospc cargo test --locked --offline \
  -p hyperai-local-service --test storage \
  physical_tmpfs_full_restore_rolls_back_and_retains_safety_snapshot \
  -- --ignored --exact --nocapture
```

It passed twice in fresh `docker run --rm --pull=never --platform linux/amd64
--network none` containers using `rust:1.93.1-bookworm`, image
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
and Rust 1.93.1. Each container mounted `/workspace` and the Cargo registry
read-only, used a disposable host target, and mounted
`--tmpfs /mnt/enospc:rw,nosuid,nodev,size=32m,mode=0700`. A fresh host target
was created with `mktemp -d /tmp/hyperai-restore-enospc-final-target.XXXXXX`.
The first run compiled offline from that empty target; the second container
reused it. The exact container command was:

```sh
docker run --rm --pull=never --platform linux/amd64 --network none \
  --mount type=bind,src="$PWD",dst=/workspace,readonly \
  --mount type=bind,src="$HOME/.cargo/registry",dst=/usr/local/cargo/registry,readonly \
  --mount type=bind,src="$TARGET_ROOT",dst=/target \
  --tmpfs /mnt/enospc:rw,nosuid,nodev,size=32m,mode=0700 \
  --workdir /workspace \
  --env CARGO_HOME=/usr/local/cargo --env CARGO_TARGET_DIR=/target \
  --env RUSTC=/usr/local/rustup/toolchains/1.93.1-x86_64-unknown-linux-gnu/bin/rustc \
  --env RUSTDOC=/usr/local/rustup/toolchains/1.93.1-x86_64-unknown-linux-gnu/bin/rustdoc \
  --env PATH=/usr/local/rustup/toolchains/1.93.1-x86_64-unknown-linux-gnu/bin:/usr/local/bin:/usr/bin:/bin \
  rust:1.93.1-bookworm sh -c 'set -eu; HYPERAI_PHYSICAL_FULL_TEST_ROOT=/mnt/enospc cargo test --locked --offline -p hyperai-local-service --test storage physical_tmpfs_full_restore_rolls_back_and_retains_safety_snapshot -- --ignored --exact --nocapture'
```

Both runs reported `filler_bytes=24997888`,
`reserve_bytes=2097152`, `backup_bytes=8540160`, `outcome_unknown=true`,
`live_rows=1`, `live_value=pre-restore`, and `safety_snapshot=ok`.

The baseline is revision `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`
plus the working-tree source manifest covering root `Cargo.toml`,
`Cargo.lock`, `.cargo/`, and all regular files under `crates/`. The manifest
has 107 files / 2,100,272 bytes, SHA-256
`13c34621099542dbf212ec71e3e3f526d483d0b18ab813e0638b2321a18649a3`; each
sorted record hashes its relative path, a NUL byte, the file SHA-256, and a
newline. The test source SHA-256 is
`78cef9909fc0ad6e4d604a757c7cad327b73885dabb7a5841c6aaa3a04bfaac0`;
`Cargo.lock` SHA-256 is
`b3708a2e0577a3e3d4a07c9110b33ae0ff0676b7aaaf495cf4278ba46e7e6736`; the
test binary SHA-256 is
`8a8334507d097158c721912c50042820fcd41c809a513f96449a39215856de47`; the
first and second passing command-output SHA-256 values are
`b5f18e3bed474f74359e661b2a2027da491957ecffc17a25a8db6f9662c9bd09` and
`0a4f25aab2f0fa694d346b15e1171f9e8bc9be9537f2cdaf278d26a41b91432a`.

Assurance is `INTERNAL_PASS` for persisted-state preservation after recovery
on this emulated Linux tmpfs path. Reviewer status is `NOT_REQUESTED` and
reproducibility is `NOT_REPRODUCED` because both runs used this host and the
second reused its target. Synthetic rows, backup, safety snapshot, and filler
were confined to the bounded `tmpfs`; the source and Cargo registry were
read-only, and no Bazaar data was used. The temporary build target and captured output were
removed after digest capture. Root `corepack pnpm run lint`, pinned Rust
formatting, `python3 scripts/verify_imports.py`, and targeted documentation
whitespace checks passed. Native Linux, other filesystems and operating
systems, power-loss recovery, independent review, and production readiness are
not established. See
[`hyperai.bazaar.desk_runtime.restore_enospc_rollback.v1`](state-slices.md).

### Physical tmpfs recovery check — 2026-09-28

The same opt-in test now closes the filler file, removes it after the failed
metadata write, and verifies that the store accepts and returns a new metadata
value. It also rechecks that the rejected key is absent, the earlier sentinel
is intact, and schema version 1 is unchanged. The exact ignored test passed
twice in fresh `--network none` Linux/amd64 containers with a 32 MiB `tmpfs`;
the second execution reused the first execution's compiled target. Both runs
reported 33,492,992 filler bytes,
SQLite code 13, and `recovery_write=ok`.

From the repository root, each container ran this exact test command:

```sh
HYPERAI_PHYSICAL_FULL_TEST_ROOT=/mnt/enospc cargo test --locked --offline \
  -p hyperai-local-service --test storage \
  physical_tmpfs_exhaustion_rejects_metadata_write_without_partial_metadata \
  -- --ignored --exact --nocapture
```

The first execution used a fresh host temporary target; the second reused that
target in a new container with a new `tmpfs`. Both used
`docker run --rm --pull=never --platform linux/amd64 --network none`, the
recorded `rust:1.93.1-bookworm` image, a read-only `/workspace` source mount, a
read-only Cargo registry mount, and
`--tmpfs /mnt/enospc:rw,nosuid,nodev,size=32m,mode=0700`. Inside the container,
`PATH`, `RUSTC`, and `RUSTDOC` selected the direct Rust 1.93.1 toolchain binaries
at `/usr/local/rustup/toolchains/1.93.1-x86_64-unknown-linux-gnu/bin` to avoid a
rustup metadata lookup. `Cargo.lock` SHA-256 is
`b3708a2e0577a3e3d4a07c9110b33ae0ff0676b7aaaf495cf4278ba46e7e6736`.

The test source SHA-256 is
`7c56ae9cfcc3cd864caea0928b6563ea3c3d2e2324d92218a292e68d1b704ae7`; the test
binary SHA-256 is
`a80e98177b7e65e55c0895aa882537df19f151fde9353d82c252c15e1999a0d7`. The
first and second captured command-output SHA-256 values are
`b56ebb6de53bb5b96ad3efc277b48ba923747f5bdb19d00072eb8f832f588959` and
`9b615b78cbad72a9481e22add4f99f870d0a8f459538452fe1ad8ed6698777ee`. Both
containers used image
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`
(`rust:1.93.1-bookworm`) and Rust 1.93.1. The source and Cargo registry were
mounted read-only; test artifacts went to a disposable target and runtime test
files went to the bounded `tmpfs`.

This is `INTERNAL_PASS` for post-`ENOSPC` metadata recovery on the emulated
Linux `tmpfs` path. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED` because both executions used this host and the second reused
the compiled target. It does not establish native Linux, other filesystems,
other operating systems, or power-loss recovery. The temporary target and
output files were removed after digest capture. See
[`hyperai.bazaar.desk_runtime.physical_enospc.v1`](state-slices.md#hyperai-bazaar-desk_runtime_physical_enospcv1).

## Shared-shell keyboard qualification status (2026-10-02)

The task-navigation keyboard qualification is not accepted yet. Its v1 run
failed before browser assertions because the child-process `PATH` omitted
`/usr/sbin/lsof`. A fresh packet corrected only `PATH`, preserved the same
keyboard assertions, and then timed out on the first Overview keyboard
transition while the Overview route, expected heading, and visible heading
focus were already present. Both one-run identities are consumed; no browser
assertion was weakened. The offline build and source syntax check passed, but
`lint:fast` and import verification did not run after the browser failure.
Keyboard acceptance remains `INCONCLUSIVE`; this does not change the Phase 1
service/API or platform decisions. See the [v1 packet](compute-bazaar-interface-task-navigation-keyboard-v1-packet.md),
[path-recovery packet](compute-bazaar-interface-task-navigation-keyboard-path-recovery-v1-packet.md),
and [state slices](state-slices.md#hyperai-bazaar-interface-task_navigation_keyboard_path_recoveryv1).

## Shared-shell keyboard qualification follow-up (2026-10-02)

The source diagnosis found that the earlier Overview wait predicate expected
the navigation label as the page H1. A separately named heading-contract
packet corrected only that runner mapping and its single same-host synthetic
Chrome workflow passed: all 11 routes, keyboard focus, mobile
open/select/Escape/focus restoration, and 300 same-origin GET requests with
zero writes. `lint:fast` and import verification also passed. This is
`INTERNAL_PASS` for that local flow only; it does not establish full
accessibility, another browser or platform, live integration, or production
readiness. The Phase 1 API/storage/platform decisions are unchanged, and
overall Phase 2 remains `NOT_ASSIGNED`. See the
[heading-contract packet](compute-bazaar-interface-task-navigation-heading-contract-v1-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-interface-task_navigation_keyboard_heading_contractv1).
