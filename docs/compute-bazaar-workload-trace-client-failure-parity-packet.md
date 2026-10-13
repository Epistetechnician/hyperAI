# Workload Trace Client Failure-State Parity Packet

Status: implemented and locally validated bounded synthetic test slice under
the user's request to follow the Compute Bazaar Rust port subplan.

State slice: `hyperai.bazaar.workload_trace_client_failure_parity.v1`.
Parent plan: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related work: [baseline client parity packet](compute-bazaar-workload-trace-client-parity-packet.md)
and [owner-backed Data read packet](compute-bazaar-workload-trace-data-read-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

## Frozen contract

Extend the existing disposable local-service parity checks to exercise the
actual owner-backed Data `conflicting` and `missing` states after seeding one
HyperAI-authored synthetic snapshot. For each state, the authenticated owner
route, CLI JSON trace, and MCP structured trace must match exactly. The
digest-free Data reference and empty Data intent-reference list must remain
consistent. The TUI must render each state and its no-source-digest label
while preserving the existing synthetic, no-correctness, and no-payment
claim text.

Use this deterministic order: seed and verify `bound`, alter the synthetic
observation bytes and verify `conflicting`, then remove the observation file
and verify `missing`. Do not recreate the snapshot after mutation. Do not
change product behavior, service routes, authorization, response schemas, ACP
capabilities, MCP tool definitions, or browser assets.

## Exact files and allowed effects

- `crates/outcomes-cli/tests/cli_compat.rs`: compare CLI JSON and the owner
  route for the changed and removed synthetic observation states.
- `crates/hyperai-tui/tests/terminal_restore.rs`: assert the TUI renders both
  owner-backed failure states without a Data digest.
- `crates/hyperai-mcp/src/lib.rs`: assert MCP structured equality with the
  owner route for `conflicting` and `missing` states.
- This packet, `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/state-slices.md`: record scope, evidence, and open gates.

Allowed effects: temporary service homes, the existing preview/seed flow,
mutation and removal of only its disposable synthetic observation file,
loopback requests to test-owned services on port `0`, focused tests, and root
validation. Preserve the existing service on `127.0.0.1:4317`. No Bazaar
source/data, user data, provider, seller, federation, paid execution, remote
Fleet, third-party agent/model, external write, chain activity, publication,
or deployment is in scope.

## Proof gates and claim ceiling

- Run the focused CLI, TUI, and MCP Rust tests, then
  `corepack pnpm run lint`.
- Run pinned import verification, Rust formatting, `git diff --check`, and
  exact-file trailing-whitespace checks.
- Confirm disposable temporary homes and service processes are cleaned up.
- Compare JSON/structured traces, including the Data intent references and
  digest-free owner reference, rather than checking only rendered labels.

Maximum claim: exact local owner-route, CLI, and MCP trace parity for the
synthetic `conflicting` and `missing` states, plus TUI rendering of those
states and the existing claim ceiling. This does not establish denied-state
parity, complete browser/CLI/TUI/ACP policy parity, broad accessibility,
platform support, live data, correct computation, execution, payment
entitlement, independent review, reproducibility, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Custody is checked-out HyperAI source and disposable HyperAI-authored
synthetic data. Publish boundary: local source and docs; no commit, PR,
release, external circulation, or deployment.

## Recorded validation

On 2026-09-28, the focused CLI, TUI, and MCP tests each passed 1/1 using pinned
Rust 1.93.1 and locked offline dependencies. The full `corepack pnpm run lint`
passed after the final code edit, including interface checks, formatting,
Clippy with warnings denied, contract tests, full all-feature workspace tests,
and doc tests. The focused tests were repeated after that root gate.
`python3 scripts/verify_imports.py`, `git diff --check`, and exact-file
trailing-whitespace checks passed; formatting is also checked by the root
gate. The tested source revisions and digests are listed in the associated
state-slice record.

Tests used disposable temporary homes, synthetic bytes, and test-owned
loopback services on port `0`; their services and data were cleaned up. A
post-test listener check confirmed the pre-existing service on
`127.0.0.1:4317` was still running. This is `INTERNAL_PASS` for the exact
states and assertions above. Reviewer status remains `NOT_REQUESTED`, and
reproducibility remains `NOT_REPRODUCED`.
