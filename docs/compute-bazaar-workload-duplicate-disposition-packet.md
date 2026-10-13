# HyperAI Duplicate Mock-Disposition Preview Packet

Status: authorized local synthetic implementation slice under the user's
direction to follow the Compute Bazaar Rust port subplan. It adds a browser
preview of the existing market ledger's duplicate-settlement replay refusal.

State slice: `hyperai.bazaar.workload_trace_duplicate_disposition_served.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Contract owner: [`hyperai-market`](../crates/hyperai-market/src/lib.rs).
Baseline: `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus the existing dirty
and untracked worktree. Preserve unrelated changes.

## Contract decision

Use the existing `MockSettlementLedger::settle` behavior without changing its
contract: the first settlement for a deal returns one mock disposition; a
second attempt on the same deal returns `MarketError::Replay`. The application
preview composes the fixed HyperAI synthetic market receipt, repeats the
already-accepted settlement against a fresh in-memory ledger, confirms the
first result equals the receipt's existing mock disposition, then performs an
exact retry and exposes only the replay refusal. It does not construct or
claim a second settlement receipt.

Add one bounded owner-authenticated loopback route:
`GET /api/v1/market/workload-trace/duplicate-disposition-preview`. It accepts
no parameters and reads no Data source, filesystem record, journal, or network
source. It remains outside the scoped ACP, CLI, and TUI surfaces in this
slice. The browser presents the existing first mock disposition beside the
fixed retry refusal and states that there is no second receipt, durable
ledger write, payment, or funds movement.

## Exact files

- `crates/hyperai-application/src/lib.rs`: construct and verify the fixed
  duplicate-attempt projection using the existing in-memory market contract.
- `crates/hyperai-local-service/src/server.rs`: add the owner-only route and
  keep it outside the scoped ACP capability.
- `crates/hyperai-local-service/tests/agent_api.rs`: cover the response,
  owner authentication, and scoped-ACP denial.
- `apps/hyperai-interface/app.js`: route the fixed preview control and retain
  the existing return-to-success behavior.
- `apps/hyperai-interface/workload-trace.js` and
  `apps/hyperai-interface/workload-trace.test.mjs`: render the first
  disposition, replay refusal, explicit no-effect claims, and safe fallback.
- `apps/hyperai-interface/app.css`: keep the additional scenario control
  usable at narrow and desktop widths.
- This packet, `docs/compute-bazaar-workload-trace-packet.md`,
  `docs/bazaar-hyperai-interface-plan.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and `docs/state-slices.md`:
  record the contract, browser evidence, assurance, claim ceiling, and gates.

## Allowed effects and stop conditions

Allowed: build the exact HyperAI-authored synthetic workload trace in memory;
exercise the owner-authenticated loopback route; select the fixed preview in
the existing interface; and run local Rust, Node, served-browser, root,
import, and whitespace checks against a disposable service home.

No persistent market journal, file, Data source, external system, provider,
seller, verifier, wallet, chain, or reviewer is contacted or changed. No
second settlement receipt is created. Stop if the duplicate scenario needs a
contract/schema change, persistence, source read, or external authority.

## Proof gates and claim ceiling

- Application tests prove the first in-memory settlement equals the
  synthetic receipt's existing disposition and the same-deal retry returns
  exactly `MarketError::Replay`.
- The trace retains exactly one disposition reference and adds one explicit
  synthetic retry-refusal reference with no digest or second receipt claim.
- Service tests verify owner access, 401 denial without owner authentication,
  and denial through the scoped ACP capability.
- Renderer tests verify first disposition, retry refusal, no second receipt
  or funds movement, return-to-success, and stale-content-free error fallback.
- The served-browser walkthrough checks the exact route and wording,
  keyboard focus, return-to-success, responsive layout, and empty console
  error log.
- Run focused Rust and Node checks, `corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, and `git diff --check`; stop the
  disposable service and remove its home.

Maximum claim: the local interface can display one existing mock disposition
and the existing contract's fixed same-deal retry refusal. It establishes no
idempotent durable settlement, payment, funds movement, execution,
computation correctness, independent review, full accessibility,
cross-browser behavior, or production readiness. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED` unless a clean second
environment is used. Publish boundary: local source and docs only, without
commit, PR, release, external circulation, or deployment.

## Validation record — 2026-09-28

Focused `cargo test --locked --offline -p hyperai-application -p
hyperai-local-service` passed: 13 application tests, 9 service unit tests,
9 owner/agent API tests, and the remaining package integration groups. The
new application test confirms the initial same-deal ledger result equals the
receipt's disposition, the repeated call returns `MarketError::Replay`, the
trace has one receipt, and an altered retry status is rejected. The service
test confirms the owner route and unchanged default trace; the owner-route
policy test confirms scoped ACP cannot access it. `node --check` passed for
the browser modules and the workload-trace renderer suite passed all 11 tests.
The root `corepack pnpm run lint` passed `lint:fast`, contract checks, and the
full workspace/all-features test suite. `python3 scripts/verify_imports.py`
verified all recorded inventories, and the `hyperai` binary built offline.

The served browser showed the existing `mock disposition release to seller`
and `mock settlement replay refused`, with exactly one local receipt
reference. Its claim text stated there was no second settlement receipt,
durable ledger write, or funds movement. The result heading received focus;
returning to success restored `One workload, source-owned records` without
the replay reference. At 390 by 844, document and body widths were 390 pixels
and all buttons fit within the 324-pixel panel content width. At 1280 by 900,
document and body widths were 1280 pixels; all scenario buttons fit the page
without horizontal overflow. The browser error log was empty. The disposable
service was stopped, its home was removed, the browser tab was closed, and
the pre-existing listener on `127.0.0.1:4317` remained active.

SHA-256: binary `af67e7f29c0dec34c625beec68ac2f30c43d9fa56e7e62a5ff8299d52744f802`;
application `e6fef9565a264979fbf75c5d2e62143f686b6d30abb7495a5bc9bec0deb2e59d`;
service `8931104fed50bee6ffd81e913b50136cc7ff1d21efad03601e96ce8a7e742e7d`;
service test `f0c95e5ad8d7d49e44efb1b145098f238384155534ae152261b382d1e70fd9c2`;
interface controller `5b20ea76e0f8d31fece324a2139aba3d7e932360c63ad7fed2262cebaa5b4e56`;
renderer `5020b41458db60055724860bcb9121784aea5a82e1f433cbc32f5f9df55e58db`;
renderer tests `540ce6f5bca31ebd6de51a9439f8763943b489c0ce95cb1a832758ab9c0a0909`;
styles `815ae0c55a3b1be62f0ec00db884ee568256646f83ae18e13a43900e871c6a8a`.

Assurance: `INTERNAL_PASS` for the exact synthetic contract, focused tests,
root checks, and served-browser path. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody is checked-out HyperAI source,
HyperAI-authored fixed records, and a removed disposable service home; no
Bazaar source/data, source-owned Data, or external system was used. Actual
owner-backed Data reads, failed fulfillment, broader client-policy parity,
full accessibility, cross-browser behavior, independent review, and
production readiness remain open.
