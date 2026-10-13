# Installed ACP Workload Trace Denied-State Parity Packet

Status: implemented and locally validated on 2026-09-28 under the named
synthetic continuation slice.

State slice: `hyperai.bazaar.workload_trace_acp_denied_parity.v1`.
Parent: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related: [Phase 8 ACP packet](compute-bazaar-phase-8-acp-packet.md),
[installed ACP bundle record](state-slices.md#hyperai-bazaar-local_install_acp_bundlev1),
and [Workload Trace Data read packet](compute-bazaar-workload-trace-data-read-packet.md).
The ignored install test's existing conflicting-input control is corrected
under the separate
[`hyperai.bazaar.acp_fixture_data_conflict_contract.v1`](state-slices.md#hyperai-bazaar-acp_fixture_data_conflict_contractv1)
slice: the current contract returns HTTP 200 with Data status `conflicting`,
no Data digest, and no intent Data reference.
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

## Frozen contract

Extend the existing ignored local-install integration to carry one actual
filesystem-denied result through the installed HyperAI service, stable ACP v1
synthetic peer, and installed sibling `hyperai-mcp` sidecar. Seed only the
fixed HyperAI-authored synthetic JSON snapshot. Temporarily remove read
permission from that single file and probe a direct read. If and only if the
probe returns `PermissionDenied`, require the owner Workload Trace route and
the ACP peer's `hyperai_market_workload_trace` result to report the same
`denied` Data status, with no Data digest or buyer-intent Data reference. The
fixture must retain the existing claim ceiling and expose no path, bytes,
permission details, owner bearer, or scoped capability. Restore the original
file permissions before continuing to the existing conflicting-input check.

If direct access remains possible, report denied-state ACP coverage as
`NOT_EXERCISED`; keep the test's bound-state check and verify the existing
conflicting-input path against the current trace contract. Do not make a
denied-state claim in that case.

The installed tool allowlist, profile permissions, service routes, auth
contract, protocol version, and product behavior remain unchanged. Only the
test fixture learns the bounded expected `denied` result.

## Exact files and allowed effects

- `crates/outcomes-cli/tests/cli_compat.rs`: extend the ignored source-install
  integration with a permission probe, owner-route assertion, and the ACP
  connection/prompt/disconnect sequence for the actual denied state.
- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`: validate the fixed denied
  trace result returned through the installed MCP sidecar and emit a bounded
  synthetic prompt summary.
- This packet, `docs/compute-bazaar-phase-8-acp-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and `docs/state-slices.md`:
  record the exact scope, evidence, and remaining gates.

Allowed effects: locked offline source build and installation into an explicit
disposable root; one disposable HyperAI home and local service on IPv4
loopback; one fixed synthetic Data snapshot; temporary read-permission
restriction and restoration on its JSON file; the HyperAI-authored synthetic
ACP peer and installed `hyperai-mcp` sidecar; and local validation. Preserve
the user-owned service on `127.0.0.1:4317`. No external agent, provider,
model, seller, registry, network access beyond loopback, real-user data,
credentials, paid execution, Fleet operation, chain, publication, or release.

## Proof gates and claim ceiling

- Build the locked workspace binaries offline, then run the exact ignored
  `source_install_bundles_mcp_sidecar_and_connects_synthetic_peer` test.
- Capture the direct OS permission probe. On `PermissionDenied`, verify the
  owner route and the installed ACP/MCP trace result both expose `denied`, no
  Data digest, and no intent Data reference; verify the synthetic agent
  summary and disconnect revocation.
- Restore the file mode before the subsequent existing conflicting-input
  assertion; verify the fixture, sidecar, service, temporary home, and install
  root are cleaned up.
- Run `corepack pnpm run lint`, `python3 scripts/verify_imports.py`,
  `git diff --check`, and exact-file trailing-whitespace checks. Confirm the
  listener on `127.0.0.1:4317` remains owned by its pre-existing process.

Maximum claim: on the tested Unix host, one local synthetic filesystem denial
is represented through the installed HyperAI service, ACP v1 test client,
and the installed HyperAI MCP sidecar under the existing scoped capability.
This does not establish other-host or Windows behavior, third-party agent
compatibility, complete UI/CLI/TUI/ACP policy parity, accessibility, external
source behavior, execution, payment entitlement, independent review, or
production readiness. Assurance is `INTERNAL_PASS` only if the exact denial
probe and all listed gates pass; otherwise denied-state ACP assurance is
`NOT_EXERCISED`. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Custody is checked-out HyperAI source and temporary
HyperAI-authored synthetic artifacts. Publish boundary is local source and
docs; no commit, PR, release, external circulation, or deployment.

## Local execution record — 2026-09-28

Baseline revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, with the
pre-existing dirty/untracked worktree preserved. Host: Darwin arm64; pinned
`rustc 1.93.1`.

The locked offline workspace-bin build passed. The exact ignored
`source_install_bundles_mcp_sidecar_and_connects_synthetic_peer` integration
passed (1 passed, 0 failed). Its direct OS probe returned
`PermissionDenied`; the owner route, the scoped ACP route, and the installed
MCP sidecar agreed on `denied`. The peer compared the direct route result with
the parsed MCP result and checked that the trace contained no Data digest or
buyer-intent Data reference. The denied summary was emitted and disconnect
revocation passed. The test restored the sample's original mode before the
subsequent conflict check.

The existing conflict control exposed a stale test-fixture assumption: it
expected HTTP 409, while the current trace contract returns HTTP 200 and a
Data reference status of `conflicting`. That test-only expectation was
corrected under the separate
[`hyperai.bazaar.acp_fixture_data_conflict_contract.v1`](state-slices.md#hyperai-bazaar-acp_fixture_data_conflict_contractv1)
slice; service and product behavior did not change. The same integration then
passed the owner and MCP conflict checks.

`corepack pnpm run lint` passed with exit code 0. `python3
scripts/verify_imports.py` passed for the pinned Simple Compute Market,
HyperIntel, and JustAI snapshots. `git diff --check` and scoped trailing-
whitespace checks passed. The disposable home, install root, service, peer,
and MCP sidecar were cleaned up. The existing listener on
`127.0.0.1:4317` remained PID 85094; process inspection found no additional
test-owned HyperAI, ACP peer, or MCP process.

The tested source digests are:

- `crates/outcomes-cli/tests/cli_compat.rs`:
  `b2e571662de3ec10b787a08d60074913528d33bf57cd3c2b82f5261ae0970585`
- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`:
  `94834a26824c5169f3919a26a30959994016220aeeaee2682ae16fe98f5f0002`

These records establish `INTERNAL_PASS` only for the stated Unix host,
synthetic permission-denied condition, and installed owner/ACP/MCP route.
Native Windows and other Unix permission behavior, accessibility, complete
client parity, separate-host reproduction, third-party agents, independent
review, and production readiness remain open.
