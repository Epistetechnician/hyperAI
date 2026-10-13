# Installed ACP Workload Trace Conflict Contract Packet

Status: implemented and locally validated on 2026-09-28 while running
`hyperai.bazaar.workload_trace_acp_denied_parity.v1`.

State slice: `hyperai.bazaar.acp_fixture_data_conflict_contract.v1`.
Parent evidence path: the ignored local source-install integration in
[`cli_compat.rs`](../crates/outcomes-cli/tests/cli_compat.rs).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

## Finding and frozen contract

The installed integration already changes the fixed synthetic JSON snapshot
and checks the resulting Workload Trace through the owner service and the
installed MCP sidecar. Its old test-only expectation treated a data-integrity
status as an HTTP 409 operation error. The current application contract
classifies local Data read failures inside a valid Workload Trace: the route
returns HTTP 200 and the Data reference reports `conflicting`, with no Data
digest and no buyer-intent Data reference. This is confirmed by the existing
application and service tests. Keep the service, trace schema, status mapping,
authentication, ACP capability, and MCP tool behavior unchanged.

Update only the test-only assertions and synthetic peer to compare the owner
trace with the installed MCP result, require `conflicting` with no digest or
intent Data reference, and emit a bounded fixed summary. This preserves a
meaningful conflict control after the new denied-state check.

## Exact files and allowed effects

- `crates/outcomes-cli/tests/cli_compat.rs`: assert the current owner-route
  conflict representation and record `conflicting` in the temporary fixture
  report.
- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`: validate the same fixed
  conflict result through the installed `hyperai-mcp` sidecar and emit a
  bounded synthetic prompt summary.
- This packet, `docs/compute-bazaar-workload-trace-acp-denied-parity-packet.md`,
  and `docs/state-slices.md`: record the finding, scope, and evidence.

Allowed effects: locked offline build and install under the integration's
disposable root; one disposable local service/home; one fixed HyperAI-authored
synthetic snapshot whose bytes are changed only by the existing test; local
loopback calls from the synthetic ACP peer and installed sidecar; and focused
validation. Preserve the user-owned service on `127.0.0.1:4317`. No external
network, model, third-party agent, provider, paid execution, live Fleet,
chain, publication, or deployment.

## Proof gates and claim ceiling

- Build workspace binaries with the pinned Rust toolchain and locked offline
  dependencies.
- Run only the exact ignored source-install ACP integration.
- Require owner and installed MCP Workload Trace values to agree on
  `conflicting`, no Data digest, and no intent Data reference; ensure no
  capability or source bytes appear in fixture output.
- Run root lint, import verification, diff and whitespace checks, and verify
  service/process cleanup while preserving the existing port-4317 listener.

Maximum claim: the test-only installed path checks one local synthetic data
integrity conflict on the tested Unix host. This does not establish product
error behavior beyond the already-existing Workload Trace contract, other
platform behavior, third-party compatibility, broader client parity,
independent review, or production readiness. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Custody is local source
and disposable synthetic test data. Publish boundary is local source and
documentation; no commit, PR, release, external circulation, or deployment.

## Local execution record — 2026-09-28

At HyperAI revision `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the
preserved dirty/untracked worktree, the exact ignored offline source-install
integration passed. The owner route and scoped ACP route returned the
versioned Workload Trace with Data status `conflicting`; the installed MCP
sidecar returned the identical parsed trace. The assertions confirmed a
trace digest but no Data snapshot digest, no intent Data reference, and no
synthetic file path or scoped capability in the fixture output. No product
code or route behavior changed.

The same locked offline build and integration described in the [denied-state
execution record](compute-bazaar-workload-trace-acp-denied-parity-packet.md#local-execution-record--2026-09-28)
also passed, as did `corepack pnpm run lint`, pinned import verification,
`git diff --check`, and exact-file trailing-whitespace checks. The temporary
home/install root and owned processes were cleaned up; the pre-existing
port-4317 service remained PID 85094.
