# Workload Trace Denied-State Client Parity Packet

Status: bounded local synthetic test continuation under the user's request to
follow the Compute Bazaar Rust port subplan.

State slice: `hyperai.bazaar.workload_trace_client_denied_parity.v1`.
Parent plan: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related work: [failure-state parity packet](compute-bazaar-workload-trace-client-failure-parity-packet.md)
and [owner-backed Data read packet](compute-bazaar-workload-trace-data-read-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

## Frozen contract

On Unix, seed one disposable synthetic observation snapshot and temporarily
remove read permission from only its JSON observation file. Capture a direct
read result while permission is restricted. If and only if that probe returns
`PermissionDenied`, require the authenticated owner route, CLI JSON, and MCP
structured trace to agree on `denied`, no Data digest, and an empty Data
intent-reference list; require the TUI to render `denied`, no source digest,
and the existing claim limits. Restore the original permission before leaving
the test. If the operating system still permits the direct read, check that
all clients consistently show the actual returned state, record that
permission denial was not exercised, and make no `denied` parity claim.

No product behavior, route, schema, authentication, ACP capability, MCP tool,
or browser asset changes are allowed.

## Exact files and allowed effects

- `crates/outcomes-cli/tests/cli_compat.rs`: compare owner and CLI projections
  for the permission-limited synthetic snapshot.
- `crates/hyperai-tui/tests/terminal_restore.rs`: render the actual owner trace
  status and claim ceiling under the same permission condition.
- `crates/hyperai-mcp/src/lib.rs`: compare MCP and owner structured traces.
- This packet, `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/state-slices.md`: record scope, observed platform result, and open
  gates.

Allowed effects: disposable service homes; the existing preview/seed flow;
temporary permission changes on its synthetic JSON file, restored before
cleanup; loopback requests to test-owned services on port `0`; focused/root
validation. Preserve the user-owned service on `127.0.0.1:4317`. No Bazaar
source/data, real-user data, networked provider, seller, federation, paid
execution, remote Fleet, third-party agent/model, external write, chain,
publication, or deployment.

## Proof gates and claim ceiling

- Run the focused CLI, TUI, and MCP Rust tests with output visible to capture
  whether the direct permission probe actually denied access.
- Run `corepack pnpm run lint`, pinned import verification, Rust formatting,
  diff hygiene, and exact-file whitespace checks.
- Verify the test-owned service and temporary home are cleaned up and the
  existing listener on port `4317` remains running.

Maximum claim: if the direct read returns `PermissionDenied`, exact local
owner-route/CLI/MCP parity and TUI rendering for the actual filesystem-denied
synthetic state on this host. Otherwise, the result is `NOT_EXERCISED` for
denied-state parity. Neither result establishes permission-policy parity on
other platforms, browser parity, complete UI/CLI/TUI/ACP parity, accessibility,
live data, execution, payment entitlement, independent review, clean
reproduction, or production readiness. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. Custody is checked-out HyperAI source and
disposable HyperAI-authored data. Publish boundary: local source and docs; no
commit, PR, release, external circulation, or deployment.

## Recorded validation (2026-09-28)

On this macOS Unix host, all three focused CLI, TUI, and MCP tests passed 1/1
with visible `PermissionDenied` direct-read probes. The `hyperai-tui` test was
split into state-specific helpers after the first full root run found its
line-count limit; the focused TUI case and final root gate passed after that
refactor. `corepack pnpm run lint` passed, including interface checks, Rust
formatting, Clippy, contract tests, full all-feature workspace tests, and doc
tests. Pinned import verification passed for Simple Compute Market revision
`a1c46d20b1b2f1aabe53e1b052921a87cf9d7360` (2,189 files; SHA-256
`afe6d9363a24fa627d9530240c190b6e68cdd1b7d6425d54d0b7547d829207ab`),
HyperIntel revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc` (99 files;
SHA-256 `62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`),
and JustAI revision `9a0e56536b968cbb2cb7173e142068b53c4f9fe3` (176 files;
SHA-256 `e853b7319aa9c1e1f4a49d5717b48e634ac2d39cd551621aa0324c0708e290c4`).
`git diff --check` and exact-file trailing-whitespace checks passed. Test-owned
homes and files were temporary and cleaned up; the existing service on
`127.0.0.1:4317` remained listening.

Final source SHA-256 digests:

- `crates/outcomes-cli/tests/cli_compat.rs`:
  `dd34eaee0b945fbe1e9f7b557a6c92d80f15011ae115044362a34133ed7179b2`
- `crates/hyperai-tui/tests/terminal_restore.rs`:
  `3f9c7c27fbcc5f38d9bf4bec3a8deac75611c4ae03d5ce3606badc9aa22b1c3d`
- `crates/hyperai-mcp/src/lib.rs`:
  `12645ee9608781cd94ffa88fc80be1929466831df9fdf83f2ecb337154e13738`

Assurance is `INTERNAL_PASS` for the stated Unix host, temporary synthetic
filesystem condition, owner route, and CLI/TUI/MCP projections. Reviewer
status remains `NOT_REQUESTED`; separate-host reproducibility is
`NOT_REPRODUCED`.
