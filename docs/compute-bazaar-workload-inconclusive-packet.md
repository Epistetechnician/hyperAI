# HyperAI Synthetic Inconclusive Workload-Trace Packet

Status: implemented and locally browser-checked under the user's direction to
follow the Compute Bazaar Rust port subplan. This packet adds one fixed
read-only preview of a mock inconclusive-verdict/dispute-hold trace.

State slice: `hyperai.bazaar.workload_trace_inconclusive_served.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Market owner: [`hyperai-market`](../crates/hyperai-market/src/lib.rs).
Baseline: `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus the existing dirty
and untracked worktree. Preserve unrelated changes.

## Contract decision

Expose a deterministic alternate projection by applying the existing
`CompletionVerdict::Inconclusive` to the existing synthetic approved deal and
fulfillment through `mock_verifier_decision` and `MockSettlementLedger`.
Verify the recomputed `LocalMarketReceiptV1` digest before composing the
`WorkloadTraceV1`. Its market references must expose
`mock_verdict_inconclusive` and `mock_disposition_hold_for_dispute`, preserving
the existing reference claim ceilings that the verdict is local fixture data,
not independent correctness evidence, and the disposition establishes no
payment or entitlement. The enclosing trace has a separate state-slice name
and claim ceiling so consumers can distinguish this preview from the default
successful synthetic trace.

The operation is in memory and repeats deterministically. It does not create
a new durable record, dispute, verifier, execution, or settlement mechanism.
The owner route is
`GET /api/v1/market/workload-trace/inconclusive-preview`; it accepts no body or
caller-selected values. The normal success route and seller-failure preview
remain unchanged. The preview endpoint is not added to ACP, CLI, or TUI in this
slice.

## Exact files

- `crates/hyperai-application/src/lib.rs`: apply the existing mock contracts,
  verify the resulting receipt, create the state-slice-bound trace, and add a
  focused contract test.
- `crates/hyperai-local-service/src/server.rs`: add the owner-only preview
  route and include it in scoped-capability route-exclusion coverage.
- `crates/hyperai-local-service/tests/agent_api.rs`: check unauthenticated
  denial, all three owner projections, and both outcome and claim labels.
- `apps/hyperai-interface/app.js`: select the success, seller-failure, or
  inconclusive preview endpoint and move focus to the rendered result.
- `apps/hyperai-interface/app.css`: keep the three preview controls wrapped
  and usable at narrow viewport widths.
- `apps/hyperai-interface/workload-trace.js` and
  `apps/hyperai-interface/workload-trace.test.mjs`: display the correct title,
  preview control, mock outcome, and nonclaims.
- This packet, `docs/compute-bazaar-workload-trace-packet.md`,
  `docs/bazaar-hyperai-interface-plan.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and `docs/state-slices.md`:
  record the contract, proof, claim ceiling, and remaining gates.

## Allowed effects and stop conditions

Allowed: reconstruct one HyperAI-authored synthetic fixture in memory; apply
the existing deterministic mock-verifier and mock-ledger contracts; serve the
result through the authenticated local HyperAI service; select it in the
existing browser interface; run focused and root-local validation; and
inspect the served result using a disposable home on IPv4 loopback.

No external network, provider, seller, agent, model, verifier, wallet, chain,
or source checkout is contacted. The operation writes no application state.
Do not modify market verifier, fulfillment, settlement, or receipt schemas.
Stop if the preview requires caller-provided data, a durable disposition, or
any interpretation that implies actual verification, dispute, or payment.

## Proof gates and claim ceiling

- The application test verifies the modified local receipt, trace digest,
  state-slice claim pairing, inconclusive verdict, dispute-hold status, and
  existing no-correctness/no-payment claim limits.
- The service test verifies that unauthenticated access is denied; the owner
  receives the exact projection; the existing success and seller-failure
  projections remain intact; and the scoped ACP capability cannot access the
  route.
- Browser tests cover the three fixed preview controls, clear titles and
  claims, refresh, return to success, and no stale content after a failed
  request. A served local-browser walkthrough confirms the exact route,
  visible status/claims, focus target, and no console errors.
- Run focused Rust and Node checks, `corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, and `git diff --check`. Stop the
  disposable service and remove its home.

The maximum claim is an internally consistent fixed synthetic trace whose
local mock verifier is inconclusive and whose in-memory mock disposition is
hold-for-dispute. It proves no actual verifier decision, computation,
dispute, funds hold, payment entitlement, independent review, accessibility
acceptance, cross-browser behavior, or production readiness. Reviewer status
must remain `NOT_REQUESTED` unless a separate review is authorized;
reproducibility remains `NOT_REPRODUCED` unless a fresh second environment is
used. Publish boundary: local source and documentation only, without commit,
PR, release, external circulation, or deployment.

## Implementation and validation — 2026-09-28

The application composes the alternate trace through the existing mock
verifier and in-memory settlement ledger, verifies the receipt digest, and
labels it with this slice's claim ceiling. The owner-only route returns the
projection and remains outside the scoped ACP capability. The served browser
renders the “Inconclusive verifier preview” title, “Synthetic mock outcome”
label, `mock verdict inconclusive`, and `mock disposition hold for dispute`,
alongside the explanation that this is not an actual verifier decision,
dispute, funds hold, or payment authorization. Activating the control focuses
the result heading; returning to success restores the default verified mock
trace. The success and seller-failure projections are covered by the service
test.

Responsive browser measurements at 390 by 844 showed all three action buttons
stacked within the 390 px document width. At 1280 by 900 the buttons appeared
in one row within the 1280 px document width. The browser console reported no
errors. The temporary service reported `stopped`, its home was removed, and
the existing local service on port 4317 remained listening.

Validation passed:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-application -p hyperai-local-service` — application, service, API, and route-policy suites, including owner denial and unchanged success/failure projections.
- `node --test apps/hyperai-interface/workload-trace.test.mjs` — 9 renderer tests.
- `sh scripts/with-pinned-rust.sh cargo build --locked --offline -p outcomes-cli --bin hyperai` — local service binary used for the served walkthrough.
- `corepack pnpm run lint` — root formatting, strict Clippy, contract, workspace, and doc-test gates.
- `python3 scripts/verify_imports.py` — all recorded pinned import inventories.
- `git diff --check` — passed after the evidence record was added.

Served artifact SHA-256 values: `target/debug/hyperai`
`f8f7fa8c174150dfd22501cce0bd4bede747bb94a4ac614e5d9dbcc22ae63c23`;
`app.js` `c5b772644f3be5a4ae7618847ceef1c8ebbdeaa26cc9719c7d42cefd5fb148f8`;
`app.css` `2f74dacadd9bac21b65af8d27ddcc95089f0f453ceed0bf206699c01b0a84da4`;
`workload-trace.js`
`546e53b9253f0751797c8d92dedc18bac74df69a5cbca0711c803e382dea0e4a`.

Assurance level: `INTERNAL_PASS` for the exact local synthetic application,
service, renderer, root validation, import, and served-browser checks above.
Reviewer status: `NOT_REQUESTED`. Reproducibility status:
`NOT_REPRODUCED`. Data and artifact custody: checked-out HyperAI source,
HyperAI-authored synthetic fixture records, and a removed disposable local
home; no Bazaar source or data was imported. Claim ceiling remains a fixed,
digest-verified local mock projection. Screen-reader use, cross-browser
behavior, other rendered negative references, duplicate disposition,
client-policy parity, actual execution, payment, independent review, and
production readiness remain unverified. External effects and publication
remain outside this slice.
