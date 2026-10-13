# HyperAI Synthetic Data-Status Preview Packet

Status: authorized local synthetic implementation slice under the user's
direction to follow the Compute Bazaar Rust port subplan. It adds explicit
browser previews for three unresolved Data reference statuses.

State slice: `hyperai.bazaar.workload_trace_data_status_served.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Workload contract: [`hyperai-market`](../crates/hyperai-market/src/lib.rs).
Baseline: `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus the existing dirty
and untracked worktree. Preserve unrelated changes.

## Contract decision

Serve the fixed statuses `missing`, `conflicting`, and `denied` through one
owner-authenticated read-only route:
`GET /api/v1/market/workload-trace/data-status-preview/{status}`. Only those
three path values are accepted. Any other value returns a bounded 400 error.
The preview constructs an in-memory HyperAI-authored synthetic intent that
contains one fixed expected Data reference and a matching request binding. The
trace's Data reference carries the selected status and no source-observed
digest, then the application validates and seals the complete trace digest.

These are fixed presentation and contract fixtures. They do not query the
Data catalog, inspect manifests or files, simulate an authorization decision,
write storage, or establish that a real source is missing, conflicting, or
denied. The intent's fixed digest is an expected synthetic value, not evidence
that Data supplied matching bytes. The owner route remains outside ACP, CLI,
and TUI in this slice; the regular workload-trace route and previews remain
unchanged.

## Exact files

- `crates/hyperai-application/src/lib.rs`: build each bounded status fixture
  using the existing intent, binding, trace-validation, and digest contracts.
- `crates/hyperai-local-service/src/server.rs`: add the path-parameter route,
  strict status mapping, bounded invalid-status error, and owner/ACP policy.
- `crates/hyperai-local-service/tests/agent_api.rs`: cover all three statuses,
  invalid status, unauthenticated denial, and scoped-ACP denial.
- `apps/hyperai-interface/app.js`: map the three preview controls to the
  fixed status route and focus the rendered result.
- `apps/hyperai-interface/workload-trace.js` and
  `apps/hyperai-interface/workload-trace.test.mjs`: render status-specific
  titles, nonclaims, collapsed scenario controls, and stale-content-free
  retry states.
- `apps/hyperai-interface/app.css`: keep the grouped controls legible at
  desktop and narrow viewports.
- This packet, `docs/compute-bazaar-workload-trace-packet.md`,
  `docs/bazaar-hyperai-interface-plan.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and `docs/state-slices.md`:
  record the contract, evidence, claim ceiling, and remaining gates.

## Allowed effects and stop conditions

Allowed: construct only the fixed synthetic intent and status references in
memory; return them from the authenticated HyperAI local service; select the
three previews in the existing browser interface; and run local Rust,
renderer, served-browser, root, import, and whitespace checks against a
disposable loopback service home.

No filesystem or Data catalog read/write is used by this preview path. Do not
change market, Data, query, manifest, or storage schemas; do not seed or
corrupt a Data source to obtain a preview. No provider, seller, agent, model,
network source, reviewer, wallet, chain, or external service is contacted.
Stop if representing a scenario requires reading or altering source-owned
Data, or if a preview could be mistaken for a real Data result.

## Proof gates and claim ceiling

- For each status, the application test verifies the same expected Data
  identity is committed by the synthetic intent and request binding, the
  trace's Data reference has no source digest, the status is exact, and the
  complete trace digest validates.
- The service test verifies owner access, 401 denial without owner auth, 400
  for an unsupported status, and denial through the scoped ACP capability.
- Renderer tests verify explicit synthetic-only language, labels, focus
  actions, all three statuses, and no stale trace markup on a failed request.
- A served local-browser walkthrough verifies the exact route, result title,
  status and nonclaims, return to success, narrow and desktop layout, and no
  browser console errors.
- Run focused Rust and Node checks, `corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, and `git diff --check`; stop the
  disposable service and remove its home.

Maximum claim: the local interface can display three fixed, digest-bound
synthetic unresolved-reference states. It establishes no actual Data
availability, source conflict, access denial, policy enforcement, storage
behavior, workload input, execution, correctness, payment entitlement,
independent review, full accessibility, cross-browser behavior, or production
readiness. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED` unless a clean second environment is used. Publish boundary:
local source and docs only, without commit, PR, release, external circulation,
or deployment.

## Validation record — 2026-09-28

`sh scripts/with-pinned-rust.sh cargo test --locked --offline -p
hyperai-application -p hyperai-local-service` passed, including the focused
application and owner-route cases. `node --check` passed for the interface
modules and `node --test apps/hyperai-interface/workload-trace.test.mjs`
passed all 10 renderer tests. The root `corepack pnpm run lint` gate passed
the fast, contracts, and full workspace gates; the built `hyperai` service
binary also compiled successfully. `python3 scripts/verify_imports.py`
verified the recorded Simple Compute Market, HyperIntel, and JustAI snapshots.

The served browser walkthrough displayed all three statuses, the fixed
synthetic-fixture label, no-source-read disclosure, status-specific
nonclaims, and focused result heading. Returning to the successful trace
restored its ordinary unseeded Data state without stale preview content.
At 390 by 844, the document remained 390 pixels wide and the three scenario
buttons each fit within the 324-pixel content column. At 1280 by 900, the
document remained 1280 pixels wide and all three scenario controls fit on one
row. Browser error logs were empty. The disposable service was stopped, its
home was removed, and the pre-existing listener on `127.0.0.1:4317` remained
running.

Source SHA-256: application
`5f92a12afe27e13c6672a492b4c4c9d26ec1ddc62b8c8eb784d3c73883f7361c`, local
service `b22d25beba262f5253d702ca824103d5b12d5993262328ea33aa9f373d2bc1e0`,
browser controller `12afd31a0ea2f88e1d644accef45b02005a5bcbba7f555badbdf45dec26102b1`,
renderer `b00062c2f517d55c5e5ac672a0e3a0bb3c020609390350e76ae0a19f2fba56f8`,
styles `d1cef12d10879e41d0020ddf153d5bc33248754486c0ba61a3235ea163af8f2a`,
and served binary
`dc3286da271c3c48d2ba4d74c28bc6804a2e7ddac7aeb7c4c5528df052ea27f1`.

Assurance: `INTERNAL_PASS` for the fixed synthetic contract, focused and root
checks, and the exact served-browser path above. Reviewer status:
`NOT_REQUESTED`. Reproducibility status: `NOT_REPRODUCED`. Custody is limited
to checked-out HyperAI source, its fixed synthetic records, and the removed
disposable loopback home; no source-owned Data was read and no Bazaar source
or data was imported. Duplicate disposition, actual Data reads, failed
fulfillment, full accessibility, cross-browser behavior, broader
client-policy parity, independent review, and production readiness remain
open. No external effect occurred.
