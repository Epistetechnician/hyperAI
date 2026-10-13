# Workload Trace Seller-Failure Client Parity Packet

Status: accepted local synthetic implementation slice under the user's
direction to continue the Compute Bazaar Rust port subplan.

State slice: `hyperai.bazaar.workload_trace_seller_failure_client_parity.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Existing contract: [seller-failure packet](compute-bazaar-workload-failure-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing
dirty and untracked worktree. Preserve unrelated changes and the user-owned
service on port 4317.

## Objective

Expose the existing fixed synthetic seller-failure projection consistently
through the local CLI, TUI, and ACP/MCP clients. Do not create a new failure
contract, accept caller-supplied failure details, perform a workload, or
change the browser's existing preview. The projection remains a seller-signed
synthetic report with no claim about real execution, cause, metering, refund,
or payment entitlement.

## Compatibility and capability decisions

- Keep `hyperai market trace`'s default output and `--data-root` behavior
  byte-for-byte compatible. Add opt-in `--seller-failure-preview` to return
  the existing application-owned fixed failure projection.
- Keep the TUI's startup/default Workload Trace unchanged. Add visible `f`
  (fixed seller-failure preview) and `r` (return to the default trace)
  controls that fetch the existing owner routes. Show an explicit synthetic
  label and claim ceiling.
- Keep ACP protocol v1 and the existing `MCP_TOOL_NAMES_V1` and
  `MCP_TOOL_NAMES_V2` arrays unchanged. Define V3 as the existing V2 tools
  with one appended read-only tool:
  `hyperai_market_workload_trace_seller_failure_preview`.
- Add the exact scoped route
  `/api/v1/agent/tools/market/workload-trace/failure-preview`, implemented by
  the existing fixed failure-preview handler. Its only capability is GET.
  The owner-only route remains unchanged; every other owner route and every
  write method remain denied to the scoped ACP capability.
- The MCP V3 tool calls only that route and has empty input. Do not pass
  private values, free text, paths, seller IDs, deal IDs, or credentials.

## Exact files

- `crates/outcomes-cli/src/lib.rs` and
  `crates/outcomes-cli/tests/cli_compat.rs`: additive opt-in flag and exact
  comparison with the existing authenticated route; default success output
  remains unchanged.
- `crates/hyperai-tui/src/lib.rs` and
  `crates/hyperai-tui/tests/terminal_restore.rs`: explicit fixed-preview
  controls, service fetch, claim rendering, and narrow/wide terminal checks.
- `crates/hyperai-acp/src/lib.rs` and `crates/hyperai-acp/src/session.rs`:
  immutable V1/V2 tool lists, additive V3 list, and ACP profile/tool
  permission snapshot.
- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`: advertise and exercise
  the exact V3 tool list in the local protocol fixture.
- `crates/hyperai-acp/tests/synthetic_peer.rs`: assert the ACP permission
  snapshot advertises the V3 toolset while keeping protocol v1.
- `crates/hyperai-local-service/src/server.rs` and
  `crates/hyperai-local-service/tests/agent_api.rs`: one fixed GET alias for
  the scoped agent capability, preserving all existing owner-route denials.
- `crates/hyperai-mcp/src/lib.rs`: append the empty-input V3 tool and compare
  its result with the owner-authenticated failure-preview route.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-workload-failure-packet.md`,
  `docs/compute-bazaar-phase-8-acp-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record the contract, compatibility,
  evidence, and limits.

No browser, imported product snapshot, Bazaar reference, Cargo dependency,
database schema, persisted record, or generic execution route is changed.

## Allowed effects and non-goals

Allowed effects: read the one deterministic HyperAI synthetic failure
projection; send local loopback GET requests from disposable tests on port 0;
render it in the TUI; expose it as one fixed read-only MCP tool; and run
focused local and root validation. Use only temporary test homes and synthetic
fixtures. Clean up test services and homes. Preserve the existing listener on
127.0.0.1:4317.

Non-goals: real seller/provider calls, running or retrying a workload,
credential access, arbitrary agent-selected routes, caller-supplied failure
details, new failure or settlement records, refund/dispute decisions, data
source changes, paid execution, live Fleet, financial Trade, public-network
access, publication, deployment, or changes to the browser preview.

## Proof gates

1. Focused CLI test: the opt-in CLI JSON equals the existing owner route's
   failure-preview JSON; the default CLI output still equals the success route.
2. Focused TUI test: `f` and `r` select only the two fixed local routes; the
   seller-reported status, no-real-execution limit, and no-payment limit are
   visible at wide and narrow terminal sizes.
3. Service authorization test: the scoped capability can GET only the new
   agent alias for this projection; the owner route, unrelated owner routes,
   and all write methods remain unavailable to it. The normal owner bearer can
   still access the existing owner preview.
4. MCP integration test: the new V3 tool returns the exact same failure trace
   as the owner route. The ACP profile and fixture peer advertise the V3 list.
   V1 and V2 tool arrays remain unchanged; V3 has the V2 list as its exact
   prefix and one appended tool.
5. Run the focused Rust tests, `cargo fmt --all -- --check`,
   `python3 scripts/verify_imports.py`, root `corepack pnpm run lint`, and
   `git diff --check`. Do not touch or stop the existing service on port 4317.

## Claim ceiling and reporting

Claim ceiling: local cross-client projection consistency for one fixed
HyperAI-authored synthetic seller-failure record. This does not prove a real
failure, execution or non-execution, cause, metering, correctness, refund,
payment entitlement, third-party agent behavior, independent review, native
platform support, or production readiness.

Validation result (2026-09-28): `INTERNAL_PASS` for the exact local synthetic
client-parity slice on HyperAI `main` at HEAD
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the uncommitted scoped work.
Rust/Cargo 1.93.1 focused tests passed for MCP (8), local-service authorization
(1) and agent API (10), CLI parity (1), and the TUI package (6 unit, 10
terminal). TUI coverage includes both fixed routes, stale-projection clearing,
and three seller-failure render cases.
The explicitly run ignored packaged ACP/MCP integration passed (1): it
advertised the exact V3 tool list, called the fixed failure-preview tool, and
confirmed capability revocation and sidecar shutdown. The CLI parity test
compared default and opt-in JSON with their service projections. Root
`corepack pnpm run lint` passed interface checks, formatting, Clippy, contract
tests, and the full all-features workspace suite. `python3
scripts/verify_imports.py`, pinned `cargo fmt --all -- --check`, and
`git diff --check` passed.

Reviewer status: `NOT_REQUESTED`; reproducibility: `NOT_REPRODUCED`. Custody is
the checked-out HyperAI source, import manifests, and disposable synthetic
homes/fixtures. Test services used port 0; the existing service on port 4317
remained running. The claim ceiling is local consistency for one fixed
HyperAI-authored synthetic seller-failure projection. No actual execution or
failure, correctness, metering, refund, payment entitlement, third-party
behavior, native platform support, or production behavior is established.
Accessibility, keyboard interaction, failed-preview transitions, and
cross-browser/client-policy parity remain open. Provider calls, federation,
paid execution, live Fleet, financial Trade, publication, and deployment
remain behind their own rights, owner, review, and operator-authorization
gates. Publish boundary is local source and documentation only; no commit,
PR, release, external circulation, or deployment.
