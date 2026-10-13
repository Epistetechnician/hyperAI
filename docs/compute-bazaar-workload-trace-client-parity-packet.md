# Workload Trace Client Projection Parity Packet

Status: authorized local synthetic continuation under the user's request to
follow the Compute Bazaar Rust port subplan.

State slice: `hyperai.bazaar.workload_trace_client_projection_parity.v1`.
Parent plan: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related contracts: [Phase 8 ACP packet](compute-bazaar-phase-8-acp-packet.md)
and [owner-backed Data read packet](compute-bazaar-workload-trace-data-read-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`, plus the
existing dirty and untracked worktree. Preserve unrelated work.

## Frozen contract

Use one disposable HyperAI service home and its existing owner-authenticated
`GET /api/v1/market/workload-trace` route to compare the same local projection
through the CLI, TUI, and read-only MCP adapter. Cover `not_seeded` before
creating a snapshot and `bound` after the service's existing synthetic preview
and seed flow. For `not_seeded`, the trace has no Data intent reference and no
Data digest. For `bound`, the intent and Data trace reference carry the exact
same verified digest. The MCP adapter returns the same structured trace as the
owner route; the TUI renders the bound status and a shortened verified digest;
the CLI's JSON trace equals the owner-route response for both states.

Retain the existing conflicting-data MCP parity assertion. Do not change the
route, method, authentication, WorkloadTraceV1 or MCP schema, ACP capability,
tool list, product behavior, or browser source. The earlier served-browser
evidence remains in the [Data read state slice](state-slices.md#hyperai-bazaar-workload_trace_data_read_servedv1);
this slice adds test evidence at the CLI, TUI, and MCP seams.

## Exact files and allowed effects

- `crates/outcomes-cli/tests/cli_compat.rs`: extend the disposable service
  projection test to compare `not_seeded` and `bound` service traces with the
  CLI JSON output.
- `crates/hyperai-tui/tests/terminal_restore.rs`: render the owner route's
  `bound` projection and verify status, digest display, and claim text.
- `crates/hyperai-mcp/src/lib.rs`: verify the existing MCP route returns the
  exact `bound` trace before the existing local corruption/conflict case.
- This packet, `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/state-slices.md`: record scope, evidence, and remaining gates.

Allowed effects: start and stop loopback service instances on port `0`; read
their temporary owner capability within tests; create synthetic Data only by
the existing preview/seed contract inside a disposable home; and run focused
local tests plus root validation. No user home or service data is read or
modified. The only network path is the test-owned loopback service.

Stop if this requires Bazaar code/data, provider or seller calls, federation,
real-user data, paid execution, remote Fleet, a third-party agent or model,
custody, wallet/chain activity, reviewer submission, public publication,
testnet write, or deployment. Those actions retain their own slices and
authority.

## Proof gates and claim ceiling

- The CLI integration asserts exact JSON equality with the owner route for
  `not_seeded` and `bound`, including empty-versus-bound Data intent references.
- The TUI integration verifies it displays `bound`, the verified digest
  prefix, and the existing synthetic-only claim ceiling from the owner trace.
- The MCP integration asserts structured equality with the owner route for
  `bound`, while retaining the digest-free `conflicting` route parity check.
- Run the three focused Rust tests, `corepack pnpm run lint`, pinned import
  verification, formatting, and `git diff --check`.
- Clean all test-owned homes/processes. Preserve the user's service on port
  4317.

Maximum claim: these exact CLI/TUI/MCP paths consume the same owner trace for
one local synthetic dataset in `not_seeded` and `bound` states; MCP also keeps
the existing `conflicting` parity case. This does not establish complete
UI/CLI/TUI/ACP permission-policy parity, broad accessibility, platform
support, live data, source freshness, market truth, correct computation,
execution, payment entitlement, independent review, reproducibility, or
production readiness. Assurance is `INTERNAL_PASS` only after these exact
local gates pass. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Custody is checked-out HyperAI source and removed
HyperAI-authored test fixtures. Publish boundary: local source and docs; no
commit, PR, release, external circulation, or deployment.

## Recorded validation

On 2026-09-28, the three focused Rust tests each passed 1/1 with pinned Rust
1.93.1 and locked offline dependencies. `corepack pnpm run lint` passed,
including interface checks, formatting, Clippy with warnings denied, workspace
contract tests, and workspace all-feature tests. `python3
scripts/verify_imports.py`, pinned `cargo fmt --all -- --check`, and
`git diff --check` passed. The full command was run after the final code edit;
the three named focused tests were repeated afterward. The focused test homes
and loopback services were disposable and removed/stopped by test cleanup. No
user-owned service was changed.

Source SHA-256:

- `crates/outcomes-cli/tests/cli_compat.rs`:
  `05fd6f88daca731acf0b65326c806a4367b5a3fd3b67e2053b512942de342d13`
- `crates/hyperai-tui/tests/terminal_restore.rs`:
  `e09a8da0540642c4caa866b83dda9a75bc93c515219394e3d794b54097c5cd1b`
- `crates/hyperai-mcp/src/lib.rs`:
  `28339d1afb79ef950251d2114d1e3e3ac82b3d2355e562cc73201537c0b2c1c7`

The pinned import verifier confirmed the recorded Simple Compute Market,
HyperIntel, and JustAI source revisions and digests. Bazaar remains an ignored
local-only reference; no Bazaar source or data was read by these tests or
copied into the tracked integration.
