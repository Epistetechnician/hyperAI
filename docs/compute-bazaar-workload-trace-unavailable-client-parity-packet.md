# Workload Trace Unavailable-State Client Parity Packet

Status: local synthetic test work complete. The exact projection checks pass;
overall assurance remains `INCONCLUSIVE` under the global overlay because
manifest-path fuzzing and a bounded red-team review were outside this slice.

State slice: `hyperai.bazaar.workload_trace_unavailable_client_parity.v1`.
Parent: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related contracts: [owner-backed Data read](compute-bazaar-workload-trace-data-read-packet.md),
[baseline client projection parity](compute-bazaar-workload-trace-client-parity-packet.md),
and [Phase 8 ACP](compute-bazaar-phase-8-acp-packet.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

Frozen source SHA-256 before this slice:

- `crates/outcomes-cli/tests/cli_compat.rs`:
  `8909939a0d92b56b015f3b26f20fa93151449e60aee946427808c3722a4a60f2`
- `crates/hyperai-tui/tests/terminal_restore.rs`:
  `2e6c2524b7470810d30289f1ffe989469f2cd7c4c8c582fd22d97f5cf08fc249`
- `crates/hyperai-mcp/src/lib.rs`:
  `ac7da19dcda0f252fbd91329d18f67dd00cbea458f8d08ce0627ca6204bc5633`
- `docs/compute-bazaar-rust-port-subplan.md`:
  `c4f04a22ca90e8f419cfab9fd1c255efa692d7404b5a710f4d4dc76b72d718f2`
- `docs/state-slices.md`:
  `78b3afaa282585890f5904951a7df0e53f8dcfaab92ab370807521cabbd40a84`

## Objective and contract

Prove that one owner-authenticated Workload Trace reports the Data owner's
actual `unavailable` status consistently through the service route, CLI JSON,
TUI, and MCP structured response. Use a disposable service home and create an
oversized `.json` entry under its synthetic Data `manifests/` directory. The
Data catalog's 16 KiB manifest ceiling classifies this bounded local
`LimitExceeded` result as `unavailable`.

For all four surfaces, require the Data reference to have status `unavailable`
and no digest, the buyer intent to have no Data references, and the workload
trace itself to retain its valid digest and existing synthetic claim ceiling.
The CLI JSON and MCP structured result must equal the owner-route response.
The TUI must render the unavailable label, no-source-digest state, and existing
synthetic/no-correctness/no-payment claims. No fixture name, manifest bytes,
filesystem path, or raw error may appear in the user-facing projection.

Keep all route paths, methods, authentication, WorkloadTraceV1 and MCP schemas,
ACP capability, allowlist, product behavior, and browser source unchanged. The
existing not-seeded, bound, denied, missing, conflicting, seller-failure, and
inconclusive assertions remain intact.

## Exact writable files and effects

- `crates/outcomes-cli/tests/cli_compat.rs`: add the unavailable owner-route
  and CLI equality case to the existing Workload Trace integration test.
- `crates/hyperai-tui/tests/terminal_restore.rs`: add unavailable-state
  rendering to the existing owner-projection test using the same temporary
  service home.
- `crates/hyperai-mcp/src/lib.rs`: compare the unavailable MCP response with
  the owner-route response in the existing disposable-service test.
- This packet, `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/state-slices.md`: record exact scope, evidence, and open gates.

Allowed effects: temporary service homes; HyperAI-authored synthetic data and
a generated oversized negative-test manifest inside those homes; local
loopback service instances on port `0`; focused tests; root validation; and
removal of test-owned homes/processes. Preserve the user-owned service on
`127.0.0.1:4317`.

Non-goals: changing Data status mapping or product contracts; reading or
copying Bazaar source/data; using real-user data; calling a provider, seller,
registry, agent, model, remote Fleet, or public network; paid execution,
custody, wallet/chain activity, external review submission, publication,
testnet writes, commits, PRs, or deployment.

## Proof gates and claim ceiling

- The focused CLI test proves exact owner-route/CLI JSON equality and the
  digest-free `unavailable` reference.
- The focused TUI test verifies the displayed status and claims at wide and
  narrow test sizes.
- The focused MCP test proves exact owner-route/MCP structured equality and
  absence of raw fixture details.
- Run the three focused Rust tests, `corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, pinned Rust formatting, and
  `git diff --check`.
- Verify test-owned services and homes are cleaned up and port `4317` remains
  untouched.

Maximum claim: exact local route, CLI, TUI, and MCP projection behavior for one
synthetic manifest-limit condition on the tested host. This does not establish
complete UI/CLI/TUI/ACP policy parity, broad accessibility, platform support,
DataFusion process isolation, live data, market truth, execution, payment
entitlement, independent review, clean reproduction, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Custody is checked-out HyperAI source and disposable HyperAI-authored test data.
Publish boundary: local source and docs; no commit, PR, release, external
circulation, or deployment.

## Execution record (2026-10-01)

Focused outcomes:

- CLI: `market_trace_cli_matches_the_authenticated_service_projection` passed.
- TUI: `workload_trace_tui_renders_the_authenticated_service_projection`
  passed, including the unavailable label, no-source-digest state, and existing
  synthetic claims at wide and narrow sizes.
- MCP: `workload_trace_mcp_matches_changed_data_owner_route` passed.
- Root: `CARGO_NET_OFFLINE=true corepack pnpm run lint` passed, including
  interface checks, installer tests, Rust formatting, strict Clippy, workspace
  tests, and doctests. Tests annotated with external build prerequisites remain
  ignored by their existing conditions.
- Import verification, pinned Rust formatting, and `git diff --check` passed.
- Focused services used port `0`; the test-owned temporary homes and processes
  were cleaned up. Port `127.0.0.1:4317` was not used.

Final source SHA-256:

- `crates/outcomes-cli/tests/cli_compat.rs`:
  `924e929e375f23c93f27f88802ea3211ededc5ca92a646a6a688dd281efad4ff`
- `crates/hyperai-tui/tests/terminal_restore.rs`:
  `10154a84953a6a7c105e2537d95cacf1a886c86020592d840e83ac6c0c9705f7`
- `crates/hyperai-mcp/src/lib.rs`:
  `e5da96be4c3c161ece1299353ebd99e817c87bf6901d4048fbdeb513d7ffc40b`

Local deterministic proof gates passed, but the assurance level is
`INCONCLUSIVE`: this packet did not run fuzzing against the manifest-reading
path or a bounded red-team review. No claim is made for complete policy parity,
independent acceptance, or clean reproduction.
