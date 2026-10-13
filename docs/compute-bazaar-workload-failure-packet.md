# HyperAI Synthetic Seller-Failure Trace Packet

Status: accepted local synthetic implementation slice under the user's
direction to follow the Compute Bazaar Rust port subplan. It adds a
seller-attributed failure record without changing the existing V1 completion,
verifier, settlement, or Workload Trace schemas.

State slice: `hyperai.bazaar.workload_trace_seller_failure.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Market owner: [`hyperai-market`](../crates/hyperai-market/src/lib.rs).
Baseline: `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus the existing dirty
and untracked worktree. Preserve unrelated changes.

## Contract decision

Add `FulfillmentFailureReceiptV1`, a seller-signed, deal-bound statement that
one identified fulfillment attempt failed. Its closed failure-code set is
`resource_unavailable`, `execution_error`, and `deadline_exceeded`. The record
contains the approved-deal digest, seller identity and public key, bounded
attempt ID, start and failure timestamps, failure code, and signature. It has
no free-text reason, private payload, output digest, charge, refund, verifier
decision, or settlement field.

Validation checks the approved deal, exact seller identity/key, deal digest,
attempt-ID grammar, attempt start at or after buyer approval, failure time at
or after attempt start, and the seller signature over a domain-separated
canonical payload. The failure code and timestamps are seller-attributed
claims; the signature does not prove that work ran, why it stopped, what was
consumed, or whether the buyer owes money.

The Workload Trace may link this record as `fulfillment_failed`, with a claim
ceiling of seller-attributed failure reporting only. That trace contains the
request, selected offer, seller quote, approved deal, and failure report; it
does not contain a completion receipt, verifier decision, mock disposition,
or completed market receipt. Existing `FulfillmentReceiptV1` remains the
seller-signed completion and metering record. Refund or dispute behavior is
not inferred from a failure report.

The local failure builder checks the exact quote-preview and approved-deal
digests before creating one deterministic synthetic report. The application
composer is test-only in this slice. The existing service route and product
UI remain on the fixed verified fixture; no scenario selector or new route is
added.

## Exact files

- `crates/hyperai-market/src/lib.rs`: additive failure record, closed reason
  enum, builder and verifier, fixed local fixture, and signature/lifecycle
  tests. Existing V1 records and settlement mappings stay unchanged.
- `crates/hyperai-application/src/lib.rs`: test-only failure trace composer,
  accepted trace-slice/claim pairing, and checks that completion-only records
  are absent from the failure projection.
- `apps/hyperai-interface/workload-trace.test.mjs`: renderer assertion for
  the failure status and seller-attribution claim ceiling.
- `crates/hyperai-tui/src/lib.rs` and
  `crates/hyperai-tui/tests/terminal_restore.rs`: render the bounded seller
  failure claim and test it with the failure status at wide and narrow sizes.
- `docs/compute-bazaar-workload-failure-packet.md`,
  `docs/compute-bazaar-workload-trace-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and `docs/state-slices.md`:
  decisions, evidence, and limits.

## Allowed effects and stop conditions

Allowed: construct deterministic HyperAI-owned synthetic records in memory;
sign and verify them with fixed fixture keys; compose and verify a local
failure trace in application tests; render fixed JSON in Node and TUI tests;
run focused and root-local validation; retain only the named evidence and
remove temporary output.

Do not change existing V1 schemas or existing success behavior. Do not add a
service route, public failure scenario control, persistence, retry engine,
refund mapping, provider/seller call, live workload, external data, wallet,
chain, funds movement, Bazaar code/data/assets, publication, or deployment.
Stop if representing the case requires private payload bytes, actual execution,
or an entitlement decision.

## Proof gates and claim ceiling

### Implementation and validation evidence (2026-09-28)

The bounded implementation adds a signed `FulfillmentFailureReceiptV1`, a
failure-only application trace projection, and seller-attribution claims in
the static browser renderer and TUI. It leaves existing V1 success records,
the service route, and the served-browser fixture unchanged. The failure
record binds to the approved synthetic deal; its trace omits completion,
verifier, disposition, and completed-market-receipt records.

Focused market, application, and TUI tests passed (15, 10, and 14 tests,
respectively); the dependency-free interface renderer suite passed 10 tests.
Pinned Rust formatting and `python3 scripts/verify_imports.py` passed, as did
`git diff --check` and root `corepack pnpm run lint`. The root gate included
strict Clippy, contract tests, workspace tests, and doc tests; environment-
gated Docker, PostgreSQL, ACP, and sidecar cases remained ignored.

Assurance is `INTERNAL_PASS` for these local synthetic contracts and static
projections only. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Custody remains the checked-out HyperAI source and fixed
HyperAI-authored records. The service and served-browser failure path,
accessibility, client-policy parity, actual fulfillment, and all refund,
correctness, payment, independent-review, and production claims remain open
or outside this slice.

- Valid reports verify; wrong signer/deal, changed signed bytes, invalid
  attempt ID, and invalid timestamp order fail closed.
- The application trace binds the failure report to the existing synthetic
  approved deal and contains no completion receipt, verifier decision,
  disposition, or completed market receipt.
- The trace verifier accepts only the exact success/failure state-slice and
  claim-ceiling pairing and preserves the trace digest.
- Browser renderer and TUI label the failure report and retain the explicit
  seller-attribution/no-payment claim limit.
- Run focused market/application/TUI/interface checks, root
  `corepack pnpm run lint`, `python3 scripts/verify_imports.py`, and
  `git diff --check`.

Claim ceiling: an internally consistent, synthetic seller-signed failure
statement linked to one local approved-deal fixture. This does not prove an
actual failure, execution, non-execution, cause, metering, correctness,
refund, payment entitlement, independent verification, or production behavior.
Reviewer status: `NOT_REQUESTED`. Reproducibility status:
`NOT_REPRODUCED`. Custody: checked-out HyperAI source and fixed
HyperAI-authored synthetic records only. Publish boundary: local source and
documentation only; no commit, PR, publication, external circulation, or
deployment.

## Served preview follow-up contract

This is a separate local-only slice,
`hyperai.bazaar.workload_trace_seller_failure_served.v1`. Preserve
`GET /api/v1/market/workload-trace` as the existing successful synthetic
trace. Add the owner-authenticated read-only route
`GET /api/v1/market/workload-trace/failure-preview`, which builds one
deterministic seller-signed failure record against the existing approved
synthetic deal and returns the existing `WorkloadTraceV1` envelope with the
seller-failure state-slice and claim ceiling. It writes no state and accepts
no caller-supplied scenario, deal, seller, or failure details.

The browser offers an explicit switch between the successful trace and the
fixed failure preview. It labels the latter as a synthetic seller-reported
example, states that HyperAI did not execute a workload to produce it, and
does not imply that a real workload did or did not run. A failed request
clears the previous projection. The new route is absent from the scoped ACP
capability allowlist; the existing ACP workload-trace tool continues to
return the successful trace. CLI and TUI behavior is unchanged in this slice.

The service test must verify both authenticated projections, preserve the
existing success payload, reject unauthenticated and scoped-ACP access to the
new route, and confirm that the failure projection omits all completion-only
records. The renderer test must cover the seller attribution, execution and
payment limits, and return-to-success control. This preview proves only the
local route and browser projection for fixed HyperAI-authored synthetic data.

## Served preview implementation evidence — 2026-09-28

The application facade now returns the fixed verified failure projection.
The local service exposes it through the owner-authenticated route; the
existing success route and its payload remain unchanged. Focused Rust tests
passed for the facade, service authorization, success/failure projections,
and exclusion of completion-only records. The scoped ACP capability remains
denied access to every owner route, including the new route. The renderer
suite passed 11 tests.

In a disposable local service and browser session, the served Deals view
loaded success, displayed the seller-reported failure preview, then returned
to success. The result heading received focus and browser console errors were
empty. The rendered claims state that HyperAI did not execute a workload to
produce the preview and that the preview does not establish whether real work
ran or any refund or payment entitlement. The disposable service was stopped
and its home removed; the pre-existing service on port 4317 was left running.

`corepack pnpm run lint`, pinned Rust formatting, JavaScript syntax checks,
`python3 scripts/verify_imports.py`, and `git diff --check` passed. Assurance
is `INTERNAL_PASS` for this exact local synthetic route and browser path.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Custody is checked-out HyperAI source and HyperAI-authored synthetic data.
Accessibility acceptance, cross-browser behavior, broader client-policy
parity, actual execution, correctness, entitlement, independent review, and
production behavior remain unproven. No external system, provider, or Bazaar
source/data was used.

Client parity follow-up (2026-09-28): the existing fixed projection is now
available through an opt-in CLI flag, TUI `f`/`r` controls, and an append-only
MCP V3 tool over one exact scoped GET alias. The successful trace and the
owner-only route remain unchanged. Focused tests and the packaged ACP/MCP
integration pass for this local synthetic scope; detailed evidence, claim
limits, and the remaining accessibility and policy-parity work are recorded
in the [client-parity packet](compute-bazaar-workload-trace-seller-failure-clients-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-workload_trace_seller_failure_client_parityv1).
