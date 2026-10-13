# HyperAI Synthetic Workload-to-Receipt Trace Packet

Status: accepted local implementation slice under the user's direction to
follow the Compute Bazaar Rust port subplan. This packet freezes one local,
synthetic vertical path; it grants no authority to contact a provider, execute
a workload, move funds, publish evidence, or deploy.

State slice: `hyperai.bazaar.workload_trace_synthetic.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Contract owner: [`hyperai-market`](../crates/hyperai-market/src/lib.rs).
Baseline: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing dirty
and untracked worktree. Preserve unrelated changes.

## Decision for this slice

Use `hyperai-market` as the owner of buyer workload identity for this
compute-procurement path, following the interface plan's recommendation. This
local choice does not make `hyperai-market` a general identity owner for
non-market HyperAI tasks. The application facade owns only a read-only
projection over references returned by their respective contract owners.

Keep every existing V1 request, quote, deal, fulfillment, evaluation, and
receipt schema unchanged. Add versioned companion records:

- `WorkloadIntentV1`: schema version, opaque workload ID, positive revision,
  optional previous-intent digest, specification digest, acceptance-contract
  digest, data-handling-policy digest, ordered source-owned data references,
  optional deadline, and creation timestamp.
- `WorkloadRequestBindingV1`: workload ID, intent digest, and digest of the
  exact existing `BuyerRequestV1`.
- `WorkloadTraceV1`: the intent and binding plus source-labelled references
  to the existing request, selected offer, quote preview, approved deal,
  fulfillment, Eval suite catalog, verifier decision, mock disposition, and
  local market receipt. Eval is marked `advisory_only_not_run_for_this_workload`
  because the fixture does not execute a job. Data, Fleet, and Trade
  references use owner-supplied identities and digests; absent Data stays
  `not_seeded` or `not_requested`. Inference and Outcomes remain unlinked
  without a caller-selected complete owner record.

Intent and reference records carry metadata and digests only. They contain no
prompt, private input or output, credentials, raw trace, provider payload, or
copied owner record. Hash the canonical intent body with the existing
`hyperai-foundation` RFC 8785 canonical JSON and the domain
`hyperai.compute-market.workload-intent.v1`. The binding verifies the existing
request digest under its current market domain. A revised intent creates a
new immutable revision; a changed intent cannot reuse an approval or receipt.
Do not add identity fields to an existing V1 record.

## Implementation scope

Compose one deterministic local fixture: buyer intent and optional verified
Data input reference, existing eligible synthetic offer and advisory-only
market context, existing request and quote preview, digest-bound approval,
mock fulfillment, Eval suite catalog context, mock verifier/disposition, and
final receipt. Eval remains explicitly unrun for this workload. Fleet
simulation and Trade remain separate workspaces and appear only as
owner-labelled context; they do not establish execution, price, completion, or
payment authority. HyperIntel and Outcomes are linked only when their existing
local contracts return a complete caller-selected record; this slice creates
no new outcome or inference claim.

Expose the same read-only trace through `GET /api/v1/market/workload-trace`,
the browser, CLI, TUI, and a fixed read-only ACP/MCP tool. Preserve the
existing ten-tool ACP/MCP V1 contract and advertise the added trace tool in
V2. ACP receives only the bounded projection, never a write or confirmation
capability. This first version is deterministic and local; it does not create
a persistent second ledger.

Exact files:

- `crates/hyperai-market/src/lib.rs`: intent, source-reference, request
  binding, trace-reference contracts, digest construction, validation, and
  synthetic fixture.
- `crates/hyperai-application/src/lib.rs`: read-only composition from
  owner-returned records.
- `crates/hyperai-local-service/src/server.rs`: owner-authenticated trace
  route, scoped read-only alias, and explicit route policy.
- `crates/outcomes-cli/src/lib.rs`: pretty-printed JSON trace command.
- `crates/hyperai-tui/src/lib.rs`: trace view using the shared service result.
- `crates/hyperai-tui/tests/terminal_restore.rs`: current V2 toolset label in
  the TUI session view fixture.
- `apps/hyperai-interface/app.js` and `apps/hyperai-interface/app.css`:
  source-grouped trace view in the existing Deals journey.
- `crates/hyperai-mcp/src/lib.rs`, `crates/hyperai-acp/src/lib.rs`,
  `crates/hyperai-acp/src/tools.rs`, `crates/hyperai-acp/src/session.rs`,
  `crates/hyperai-local-service/tests/agent_api.rs`,
  `crates/hyperai-acp/tests/synthetic_peer.rs`,
  `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`, and
  `crates/outcomes-cli/tests/cli_compat.rs`: one read-only ACP/MCP tool,
  additive V2 fixed-tool negotiation, preserved V1 tool names, and packaged
  synthetic integration.
- This packet, `docs/compute-bazaar-rust-port-subplan.md`,
  `docs/bazaar-hyperai-interface-plan.md`, and `docs/state-slices.md`.

## Allowed effects and stop conditions

Allowed: construct deterministic HyperAI synthetic records; read owner
contracts through the local facade; exercise authenticated IPv4-loopback
requests and packaged clients in disposable homes; build local binaries and
run the proof gates; retain only necessary source, fixture, command, and
output digests; remove disposable homes and outputs after the run.

No non-loopback network, provider, seller, remote Fleet, external agent,
model, wallet, chain, reviewer, or non-synthetic data may be used. Do not
change existing V1 schemas or outcome/HIP-4 records. Do not import Bazaar
code, schemas, fixtures, datasets, assets, generated output, or dependencies.
Stop if a private payload must enter a record, a V1 migration is needed, or
the slice requires external authority.

## Proof gates and claim ceiling

- Validate intent invariants, revision linkage, reference ordering and
  uniqueness, digest stability, and tamper rejection.
- Verify the binding against the unchanged buyer-request digest; reject
  mismatched intent, request, quote, approval, or receipt links.
- Compare service, browser, CLI, TUI, and ACP projections for identical IDs,
  digests, source owners, status labels, claim ceilings, and nonclaims.
- Exercise missing, stale, conflicting, denied, failed, and inconclusive
  references without filling unknowns or changing linked records.
- Confirm ACP cannot approve, complete, or settle and its scoped capability
  cannot access owner-only routes.
- Run focused checks, `corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, browser syntax/tests, and the packaged
  disposable integration; report each scope and assurance level.

## Validation record — 2026-09-27

- `corepack pnpm run lint`: passed `lint:fast` (format and workspace Clippy),
  `verify:contracts`, and `verify:full` (workspace tests and doc tests).
- `python3 scripts/verify_imports.py`: passed all three pinned import
  inventories; no imported snapshot changed.
- `sh scripts/with-pinned-rust.sh cargo build --locked --offline --workspace
  --bins`: passed.
- The packaged ignored test
  `service_issues_scoped_capability_to_synthetic_acp_profile` passed, including
  the scoped capability read and owner-route denial checks.
- The packaged ignored test
  `source_install_bundles_mcp_sidecar_and_connects_synthetic_peer` passed.
- `node --check apps/hyperai-interface/app.js` and
  `node --test apps/hyperai-interface/error-envelope.test.mjs` passed (3
  tests). These checks do not exercise rendered workload-trace behavior in a
  browser.
- `git diff --check` passed.

Assurance level: `INTERNAL_PASS` for the exact local synthetic contract,
workspace validation, CLI/TUI/service tests, and packaged ACP/MCP scenarios
listed above. Reviewer status: `NOT_REQUESTED`. Reproducibility status:
`NOT_REPRODUCED`. Data and artifact custody: checked-out HyperAI source,
HyperAI-authored synthetic fixtures, and disposable local integration homes;
no Bazaar source or data was imported. At this base-slice validation snapshot,
the rendered browser trace and cross-browser behavior were unverified; the
separately scoped browser-binding follow-up below now covers a narrow local
browser path. Next blocked authority gate: any
provider call, federation, paid execution, chain write, public release, or
production operation needs its own named slice and applicable authorization.

Maximum claim: a digest-bound local synthetic workload-to-receipt projection
whose identity and status are consistent in the exact exercised service,
CLI, TUI, ACP, and MCP paths. This base slice does not claim rendered browser
behavior; the browser-binding follow-up below is limited to its exact tested
local seed, binding, failure, and viewport path. It does not
prove real availability, execution, correct computation, seller identity,
independent verification, payment entitlement, broad client parity,
production readiness, or independent review. Bazaar rights,
native-platform acceptance, full assistive accessibility, broader
visual/browser behavior, and clean second-host reproduction remain open.
Publish boundary: local source and docs only; no commit, pull request, public
release, external circulation, or deployment.

## Browser Data-binding follow-up — 2026-09-27

State slice: `hyperai.bazaar.workload_trace_browser_binding.v1`. On the rebuilt
local interface, the Data workspace exposed the explicit synthetic seed
action. One activation created the fixed HyperAI-owned sample; opening Deals
without reloading showed the verified Data reference and its manifest digest
in the workload trace. After one byte was appended to that disposable JSON
fixture, Refresh cleared the previous trace and showed the generic read-only
route error. At 1280 by 900, the source cards used the intended desktop layout;
at 390 by 844, all five cards fit without document or body overflow and the
refresh text remained legible. The exact build, dimensions, measured widths,
custody, and open gates are recorded in the
[`hyperai.bazaar.workload_trace_browser_binding.v1` state slice](state-slices.md#hyperai-bazaar-workload_trace_browser_bindingv1).

This `INTERNAL_PASS` does not qualify cross-browser behavior, full
accessibility, failed-fulfillment rendering, any non-synthetic source,
independent review, or production readiness. Later fixed-preview follow-ups
below cover missing, conflicting, and denied status displays and the
inconclusive mock-outcome display; they do not perform source reads or prove
real-world conditions.

## Trace-link consistency follow-up — 2026-09-28

Under `hyperai.bazaar.workload_trace_synthetic.v1`, the application verifier
now requires each buyer-intent data reference to have a source-labelled trace
reference with the same owner, schema, and record ID. A `bound` or `stale`
reference must carry the intent's exact digest. `missing`, `conflicting`,
`denied`, and `unavailable` references may remain explicit without claiming a
source digest. A HyperAI Data reference marked `bound` without a matching
intent reference is rejected. No route or record schema changed.

The focused application tests cover explicit unresolved states, stale and
bound digest matches, missing links, and conflicting digest claims. The
inconclusive-verifier follow-up below now checks that its actual verdict and
mock disposition remain visible in the application trace. At this point in
the trace-link follow-up, the rendered browser had not yet exposed these
scenarios. The later fixed-preview follow-ups below cover missing,
conflicting, denied, and mock-inconclusive/dispute-hold presentation only;
failed fulfillment and actual source-state behavior remain open.

Assurance: `INTERNAL_PASS` for the application-level link-consistency checks
and the stated local root gates. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody: checked-out HyperAI source and
HyperAI-authored synthetic metadata only. No Bazaar source/data or external
system was used. Claim ceiling remains digest-bound local synthetic
projection integrity. Publish boundary remains local source and docs only;
no commit, PR, release, external circulation, or deployment.

## Mock verdict and disposition visibility follow-up — 2026-09-28

Under `hyperai.bazaar.workload_trace_synthetic.v1`, the application projection
now carries the receipt's specific mock verdict (`verified`, `rejected`, or
`inconclusive`) and mock disposition (`release_to_seller`, `refund_to_buyer`,
or `hold_for_dispute`) in the corresponding reference statuses. Their claim
ceilings still state that the verdict is not independent correctness evidence
and the disposition is not payment or entitlement. No public record schema,
route, or settlement behavior changed.

The focused application test constructs a valid synthetic receipt with an
inconclusive mock verifier decision, applies the existing mock-ledger mapping,
verifies the receipt digest, composes and verifies the complete trace, and
checks for `mock_verdict_inconclusive` plus
`mock_disposition_hold_for_dispute`. This proves application-level projection
of that existing owner-contract state only. It does not exercise the rendered
browser, other clients, a real verifier, or funds movement.

The CLI compatibility case also confirms the standard synthetic receipt is
exposed with its verified verdict and mock release disposition while Trade
remains labelled advisory market data. The root `corepack pnpm run lint` gate
passed format, Clippy, contract tests, workspace tests, and doc tests;
`python3 scripts/verify_imports.py` also passed all three pinned import
inventories.

Assurance: `INTERNAL_PASS` for this local synthetic application contract and
the focused workload-trace test. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody: checked-out HyperAI source and
HyperAI-authored synthetic records only; no Bazaar source/data or external
system was used. Claim ceiling remains digest-bound local synthetic
projection integrity. Browser rendering of negative outcomes and broader
client parity remain open. Publish boundary remains local source and docs
only; no commit, PR, release, external circulation, or deployment.

## TUI status readability follow-up — 2026-09-28

Under the same synthetic workload-trace slice, the TUI now renders
underscore-delimited statuses as readable phrases, matching the browser's
display convention. It breaks the final trace disclaimer into short lines so
the no-execution, no-funds, and no-correctness/payment boundaries remain
visible in a narrow terminal.

The terminal-render test injects only a HyperAI-authored synthetic trace with
an inconclusive mock verdict and dispute hold. It checks the humanized labels,
the absent disposition digest, and claim limits at 100 by 32 and 56 by 20
cells. This tests TUI rendering of supplied records; the actual receipt to
trace behavior is separately covered by the application test above. It does
not prove every terminal size or a live source connection.

Assurance: `INTERNAL_PASS` for this bounded TUI presentation test. Reviewer
status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Custody:
checked-out HyperAI source and a synthetic in-memory JSON value; no Bazaar
source/data or external system was used. Claim ceiling remains readable local
presentation of the existing synthetic trace. Broader terminal accessibility,
all-size behavior, full client parity, external sources, and publication
remain open.

## Clean Linux TUI package reproduction — 2026-09-28

The pinned Rust `1.93.1` TUI package suite was run from a fresh source snapshot
in a network-disabled `linux/amd64` container. The sorted JSONL source manifest
contains 267 files and 3,751,638 bytes, with SHA-256
`5a401d3374ef7211ee35c0956e193a677b059e2e8c3c474d95426f4e407c2623`. It
includes the current browser assets, root manifests, all Rust workspace
crates, build inputs, and the pinned HyperIntel snapshot. The verified image
ID is `sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`
(`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`).

`cargo test --locked --offline --jobs 4 -p hyperai-tui --all-targets
--all-features` passed 4 unit tests and 7 terminal integration tests, with 0
failures and 0 ignored. This includes
`workload_trace_workspace_humanizes_mock_outcomes_and_keeps_claim_limits`.
The pinned Rust compiler reported `1.93.1 (01f6ddf75 2026-02-11)` for
`x86_64-unknown-linux-gnu`. The full test-output SHA-256 is
`e4fcc1635924227b0e2dfbf47983940084fbed2ee6a5893e8a5d461c23e7c1e8`.
`python3 scripts/verify_imports.py` passed before and after the run. The source
snapshot and Cargo cache were mounted read-only; the target directory was
fresh; the temporary snapshot, target, manifest, and log were removed after
hashing.

Assurance is `REPRODUCED_LOCAL` for this TUI package command and exact source
manifest only. Docker Desktop emulated Linux/amd64 on Apple Silicon; this does
not qualify native Linux, a separate host, other operating systems, broad
terminal accessibility, full client-policy parity, independent review, or
release readiness. Reviewer status is `NOT_REQUESTED`. Custody is limited to
the manifest-bound HyperAI source, the verified HyperIntel import, read-only
Cargo cache, and synthetic in-memory test values. Claim ceiling remains TUI
rendering and terminal-restoration behavior for the package suite. The failed
container bootstrap attempts ended before compilation and are not test
failures. No source tree, external service, or upstream Bazaar code/data was
used.

## Workload-trace render contract follow-up — 2026-09-28

Under `hyperai.bazaar.workload_trace_view_contract.v1`, the dependency-free
workload-trace renderer was extracted from the application bootstrap and
loaded before `app.js`. Status, link-basis, and claim-ceiling codes are
humanized; a trace with no references explicitly says availability and
outcomes remain unknown; missing source digests remain absent; and a failed
load returns an error view without stale reference markup. The fixed
inconclusive verdict, dispute hold, and claim limits remain visible. Source
controlled text is escaped. No route, API, schema, or product action changed.

`corepack pnpm run verify:interface` passed 9 Node tests across the
error-envelope and workload-trace suites. `corepack pnpm run lint` passed the
root fast, contract, and full gates. `python3 scripts/verify_imports.py` passed
all three pinned inventories, and `git diff --check` passed.

Assurance: `INTERNAL_PASS` for the HTML fragments, fixed synthetic cases, and
local root gates. Reviewer status: `NOT_REQUESTED`. Reproducibility status:
`NOT_REPRODUCED`. Custody: checked-out HyperAI source and fixed in-memory
HyperAI-authored JSON only; no Bazaar source/data or external system was used.
The test harness checks the production module and script ordering, but does
not run a browser or demonstrate these failure states in the served UI. The
next local gate is served-browser coverage and comparison with owner-route and
CLI/TUI/ACP behavior. Claim ceiling remains tested markup behavior for the
listed synthetic states; execution, correctness, payment, broad client parity,
accessibility, independent review, and production readiness are not established.
The publish boundary is local source and docs only.

## Headless browser DOM probe — 2026-09-28

An optional Chrome DOM probe was attempted under
`hyperai.bazaar.workload_trace_dom_contract.v1`. Chrome 153.0.8010.53 did not
reach DOM capture within the 15-second limit; stderr showed repeated
`CVDisplayLinkCreateWithCGDisplay` failures. It also logged a Google Cloud
Messaging registration response (`DEPRECATED_ENDPOINT`) despite
`--disable-background-networking`, followed by a network-service error. This
crossed the intended no-network boundary, so the test harness and package
script were removed and further attempts stopped. The browser loaded only a
temporary local page containing the HyperAI renderer and synthetic metadata;
no user page, service response, or product data was supplied. All observed
test processes and the exact temporary page/profile were removed.

This result is `INCONCLUSIVE`; it establishes no browser DOM behavior. The
earlier Node HTML-fragment tests remain the available evidence. Resume browser
qualification only in a disposable runtime with network denied below Chrome
and bounded process-tree cleanup, under a fresh named state slice. Chrome's
documented `--timeout` option and Node's detached process-group behavior were
used while investigating the capture and cleanup bounds ([Chrome Headless
CLI](https://developer.chrome.com/docs/automation-and-testing/headless-cli),
[Node child processes](https://nodejs.org/api/child_process.html)).

## Isolated browser DOM follow-up — 2026-09-28

Under `hyperai.bazaar.workload_trace_browser_dom_private_ipc.v1`, the exact
production `workload-trace.js` renderer was parsed by Google Chrome
153.0.8010.53 on macOS 26.6.1 arm64. To let Chrome create its disposable
profile socket without enabling IP networking, the generated Seatbelt profile
denied all outbound, inbound, and bind operations, then allowed Unix-domain
operations only for paths under one private temporary root. The preflight
confirmed that loopback TCP and UDP for IPv4 and IPv6, IP binds, and a listening
Unix-domain socket outside the root returned `EPERM`; a temporary-root
Unix-domain socket worked. No external address, service, or network URL was
used.

The fixture exercised missing, conflicting, denied, stale, failed-fulfillment,
inconclusive-verifier, and dispute-hold references, missing digests, claim
ceilings, owner-list and disclosure semantics, an empty-reference state, the
error fallback without stale records, and escaped source-controlled markup.
The first scoped-profile attempt did not yield a DOM result because an inline
test assertion contained a literal HTML `</script>` terminator; the extracted
script's `node --check` caught it, and the corrected fixture passed syntax
checks before the successful browser run. This was a harness error, not a
renderer failure.
Chrome serialized the actual DOM with
`DOM_CHECK=PASS;ASSERTIONS=16;FAILED=` and exited within the 15-second process
deadline. Exact identities: renderer SHA-256
`a4eabaf71225b6a3e06766703fd8e26472485ef3c2a0d8f30f9cb5f7c3ef932f`, profile
SHA-256 `57c15221e5bfdf532359da6e501c7549133ab0880d14fb5870b8a048bc131522`,
fixture SHA-256
`70b36ca941668dd9a415f7950d9fdb907042ea9292320076e63ced6b905c4a3d`, and
18,035-byte DOM output SHA-256
`5617800ffefcd5a1672d278c643352ef207b0f7dfa554d5264d38ba0f4636282`. Chrome
also emitted macOS display/task-policy diagnostics and a GPU/network-service
restart after the assertions; these do not establish visual layout or service
connectivity. No probe-owned process remained.

The focused interface suite passed 9 tests, all three pinned import inventories
verified, the root `corepack pnpm run lint` gate passed, and `git diff --check`
passed. No application source changed in this follow-up; import verification
does not validate the ignored Bazaar reference.

Assurance: `INTERNAL_PASS` for the fixed renderer DOM assertions and tested
socket-policy preflight. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. Custody: checked-out HyperAI source and HyperAI-authored
synthetic metadata only; no Bazaar source or data was used. This closes only
HTML parsing and semantic attributes for these renderer fixtures. It does not
exercise the served interface, route/API failure behavior, visual layout,
accessibility tree, assistive technology, cross-browser behavior, full client
parity, independent review, or production readiness. Next local gate: compare
the served-browser trace behavior with owner-route and CLI/TUI/ACP contracts.

## Service, CLI, and TUI projection parity follow-up — 2026-09-28

Revision: HyperAI `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main` plus
the existing dirty and untracked worktree. State slice:
`hyperai.bazaar.workload_trace_synthetic.v1`.

Under `hyperai.bazaar.workload_trace_synthetic.v1`, the existing local-service
middleware test now asserts that the complete JSON from the scoped-agent trace
route equals the owner-authenticated trace route. A new CLI integration test
starts the HyperAI service on ephemeral IPv4 loopback in a disposable home,
reads the owner route, then runs `hyperai market trace --data-root` against the
same home and compares the complete JSON values. The test uses only the fixed
synthetic market fixture and an empty local Data root. No product route,
authorization rule, or record schema changed. A TUI integration test fetches
the authenticated owner route from a disposable service and passes the exact
response through the current terminal renderer, checking workload ID, digest,
mock verdict/disposition, and claim limits.

The focused service, CLI, and TUI parity tests passed (1 each). Root
`corepack pnpm run lint` passed `lint:fast`, `verify:contracts`, and
`verify:full`; `python3 scripts/verify_imports.py` passed all three pinned
inventories. The first root-gate attempt caught a formatting difference in
the new Rust test signature; that was corrected before the passing run.

Assurance: `INTERNAL_PASS` for owner/agent route JSON equality, complete CLI
JSON equality, and the listed TUI rendering assertions. Reviewer status:
`NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Custody: checked-out
HyperAI source, fixed HyperAI synthetic market records, and a disposable local
home; no Bazaar source or data was used. The TUI check covers the current route
response passed through the renderer, not the complete TUI startup lifecycle.
This does not prove that the browser or ACP/MCP display the same failures or
policy decisions, and it does not close the served-browser negative-state,
broader client-parity, accessibility, native-platform, independent-review, or
production gates. The authority gate for non-synthetic sources, live Fleet,
paid Eval, financial Trade, funds movement, and deployment remains separately
closed. The served-browser and MCP service-error continuation is recorded
below; broader ACP prompt parity remains open.

## Served renderer and changed-input error follow-up — 2026-09-28

State slices: `hyperai.bazaar.workload_trace_served_renderer.v1` and
`hyperai.bazaar.workload_trace_failure_parity.v1`.

The first served-browser attempt exposed an integration defect that the
renderer-only tests could not detect: `index.html` loaded
`/workload-trace.js`, but the local service did not serve that path. Selecting
Deals raised a `renderPanel` TypeError and left the Data page content on screen.
The local service now serves the existing renderer bytes at that same-origin
static path. `serves_the_workload_trace_renderer_at_the_interface_asset_path`
passed and asserts byte equality with the source and the JavaScript content
type; no API path, authorization rule, or contract changed.

On the rebuilt service in a disposable home, the browser created the fixed
synthetic Data snapshot through the existing seed action. Deals then rendered
the complete Workload Trace with the Data reference `bound` to manifest digest
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335` and trace
digest
`cdc3acad30fd4e00498d89c5ab04b1b695063eaaa434d0e83d3d381adc629364`. After
one space was appended to that disposable JSON snapshot, its 511-byte content
had SHA-256
`9af3212de61064dfb713f310ea9cb3f577a70e87e76021cadc91f4f3b82a28f6`. Refresh
returned the browser to the generic error panel and removed all old trace
reference markup. The served-page console had no errors after the repair.
The authenticated owner route returned HTTP 409, code `CONFLICT`, and the
generic service message without fixture details.

The focused MCP adapter test
`workload_trace_mcp_error_matches_changed_data_owner_route` passed against a
disposable local service with the same changed-input failure. It compared the
owner response to the MCP adapter result and confirmed `HTTP 409 (CONFLICT)`
with no fixture bytes or service message in the MCP error. This test uses a
local owner bearer for the HTTP client; scoped-agent access remains separately
covered by the route policy tests and does not establish ACP prompt-level
error parity.

`corepack pnpm run verify:interface` passed 9 tests. The root
`corepack pnpm run lint` passed interface, formatting, strict Clippy, contract,
workspace, and doc-test gates. Two staged-binary CLI cases and the
environment-gated PostgreSQL/Docker cases remained ignored. The local service
was stopped and the disposable home removed. `python3 scripts/verify_imports.py`
passed all three pinned inventories. After all evidence edits,
`git diff --check` and the exact-file whitespace/final-newline scan passed.

Assurance: `INTERNAL_PASS` for static renderer wiring, the exact served
Data-bound trace, changed-input browser fallback, owner-route error envelope,
and the MCP adapter's status/code mapping. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody: checked-out HyperAI source and one
fixed HyperAI synthetic snapshot in a disposable local home; no Bazaar source
or data was used. This does not establish full ACP prompt behavior, broad
cross-client policy parity, screen-reader/accessibility behavior,
cross-browser support, native-platform support, independent review, or
production readiness. Live Fleet, paid Eval, financial Trade, funds movement,
external projection, and deployment remain behind separate authority gates.

## Packaged ACP/MCP error propagation follow-up — 2026-09-28

State slice: `hyperai.bazaar.workload_trace_acp_error.v1`.

The exact ignored local source-install integration
`source_install_bundles_mcp_sidecar_and_connects_synthetic_peer` passed against
locked offline workspace binaries. It first connected the bundled synthetic
ACP peer and MCP sidecar to an unchanged preview-bound synthetic Data snapshot,
validated the full trace, and observed the fixed success prompt event. After
the test appended one space to the disposable Data JSON, the owner route
returned HTTP 409/`CONFLICT` with the generic service message. A second peer
session reached that same error through the scoped capability and packaged MCP
stdio protocol. The sidecar exposed only HTTP status and code, excluding the
service message, fixture identifiers, and capability value. The ACP prompt
event reported only `409:CONFLICT`. After disconnect, the capability and MCP
sidecar calls were denied; the Fleet simulation remained unchanged.

The full root `corepack pnpm run lint` passed interface checks, formatting,
strict Clippy, contract tests, workspace tests, and doc tests. Two staged-binary
CLI cases and environment-gated PostgreSQL/Docker cases remained ignored.
`python3 scripts/verify_imports.py` passed all three pinned inventories. The
disposable home and child processes were cleaned up.

Assurance: `INTERNAL_PASS` for this exact local installed synthetic ACP/MCP
error and prompt path. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. Custody: checked-out HyperAI source and fixed HyperAI-authored
synthetic Data in a disposable local home; no Bazaar source or data was used.
This does not establish model output, general ACP-client behavior, Windows
runtime support, accessibility, independent review, or production readiness.
Provider calls, paid execution, live Fleet, Trade execution, funds movement,
external projection, and deployment retain their separate closed authority
gates.

## Fresh-home unseeded Data parity follow-up — 2026-09-28

State slice: `hyperai.bazaar.workload_trace_synthetic.v1`.

With no Data snapshot seeded, the authenticated owner route returns an empty
`intent.body.data_references` array and a Data context record marked
`not_seeded` with no digest. The CLI test compares its complete JSON value
with the owner route for the same empty data root. The TUI consumes the
owner-route response and renders
`not seeded` and `no source digest`. A served browser run against a separate
fresh home displayed “not seeded” and “No separate source digest” in Deals;
the page had no console errors.

The focused service, CLI, and TUI tests passed. `corepack pnpm run lint` passed
the root interface, formatting, strict Clippy, contract, workspace, and
doc-test gates. `python3 scripts/verify_imports.py` passed all three pinned
inventories. The browser service was stopped, its disposable home was removed,
and its temporary tab was closed.

Assurance: `INTERNAL_PASS` for this exact fresh-home Data absence state across
the route, CLI, TUI renderer, and served browser. Reviewer status:
`NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Custody: current HyperAI
source and an empty disposable local home; no Bazaar source or data. The
result does not qualify actual missing/conflicting/denied Data source records,
failed fulfillment, inconclusive verification, duplicate disposition, full
accessibility, or general client-policy parity. No external connector or
authority gate changed.

## Mock verifier rejection and refund projection — 2026-09-28

State slices: `hyperai.bazaar.workload_trace_synthetic.v1` and
`hyperai.bazaar.workload_trace_view_contract.v1`.

The application test constructs a HyperAI-authored synthetic market receipt,
binds a `Rejected` local mock-verifier decision to its seller-signed completion
receipt, and applies the existing in-memory mock ledger. The projected trace
retains `mock_verdict_rejected` and `mock_disposition_refund_to_buyer`, with
claim ceilings stating that the verdict is not independent correctness
evidence and the disposition is not a payment or entitlement claim. The TUI
and dependency-free browser renderer tests preserve those labels and limits.

This does not represent failed fulfillment. The current V1 fulfillment record
asserts seller-reported completion and has no failure status; the scenario is
verifier rejection after a claimed completion. No route, schema, authorization,
runtime behavior, or product scenario control changed. The served browser route
still uses the fixed verified fixture.

The focused application and TUI tests passed under the repository-pinned Rust
1.93.1 toolchain, and the interface renderer suite passed 7 tests. The full
`corepack pnpm run lint` gate passed interface checks, formatting, strict
Clippy, contract tests, workspace tests, and doc tests; two staged ACP/MCP
sidecar tests and environment-gated PostgreSQL/Docker tests remained ignored.
`python3 scripts/verify_imports.py` passed all three pinned inventories, and
`git diff --check` passed.

Assurance: `INTERNAL_PASS` for the exact local synthetic verifier-rejection
projection and TUI/renderer labels. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody: checked-out HyperAI source and
HyperAI-authored in-memory synthetic records; no Bazaar source or data was
used. This does not prove fulfillment failure, real verification, correct
computation, execution, refund, or funds movement. The next design gate for an
actual failed-fulfillment claim is an owner-approved versioned market contract
and its own named state slice. No external action, publication, or deployment
is authorized.

Seller-attributed failure follow-up (2026-09-28): the separately named
`hyperai.bazaar.workload_trace_seller_failure.v1` slice adds a signed,
deal-bound failure record and a failure-only synthetic trace projection. The
focused market, application, TUI, and interface suites passed, as did root
`corepack pnpm run lint` and import verification. This is `INTERNAL_PASS` for
the local synthetic contract and static projections; reviewer status is
`NOT_REQUESTED` and reproducibility is `NOT_REPRODUCED`. The service and
served-browser route still use the successful fixture. Actual fulfillment,
payment or refund entitlement, independent review, accessibility, and client
policy parity remain unverified. See the [failure packet](compute-bazaar-workload-failure-packet.md)
and [state-slice record](state-slices.md#hyperai-bazaar-workload_trace_seller_failurev1).

### Served seller-failure preview (2026-09-28)

Under the separate `hyperai.bazaar.workload_trace_seller_failure_served.v1`
slice, the authenticated owner API serves a fixed seller-failure preview at
`GET /api/v1/market/workload-trace/failure-preview`. The existing
`GET /api/v1/market/workload-trace` remains the successful trace and the ACP
capability remains restricted to that original route. The browser can switch
between them; the failure view is labelled synthetic and seller-reported and
states that it does not establish whether a real workload ran or any refund
or payment entitlement. This is a read-only projection, with no execution or
funds movement.

Implementation and validation evidence (2026-09-28): the focused application
and local-service Rust suites passed, including the seller-failure facade
projection, owner authentication, unchanged success route, failure-record
links, completion-record exclusions, and scoped-ACP owner-route denial. The
dependency-free interface suite passed 11 tests. The served local browser
walkthrough loaded the successful trace, switched to the fixed failure
preview, and returned to the successful trace; focus moved to the result
heading and the browser reported no console errors. The failure preview
showed the synthetic seller-attributed failure and explicitly said HyperAI
did not execute work to create the preview, while making no claim about real
execution or payment entitlement. The disposable service was stopped and its
home removed; the pre-existing service on port 4317 was preserved.

Root `corepack pnpm run lint`, pinned Rust formatting, JavaScript syntax,
`python3 scripts/verify_imports.py`, and `git diff --check` passed. The import
check verified all three recorded source inventories without changing them.
The full local validation remains `INTERNAL_PASS` for this exact synthetic
route and served-browser path. Reviewer status: `NOT_REQUESTED`.
Reproducibility status: `NOT_REPRODUCED`. Custody: checked-out HyperAI source,
HyperAI-authored synthetic records, and a disposable local home; no Bazaar
source or data was imported. This does not establish complete accessibility,
cross-browser behavior, general client-policy parity, actual fulfillment,
correct computation, refund or payment entitlement, independent review, or
production readiness. No external system was contacted; the service used
only the removed disposable home.

The exact state-slice record is
[`hyperai.bazaar.workload_trace_seller_failure_served.v1`](state-slices.md#hyperai-bazaar-workload_trace_seller_failure_servedv1).

### Inconclusive mock-outcome preview (2026-09-28)

The separately authorized
`hyperai.bazaar.workload_trace_inconclusive_served.v1` slice adds a fixed
owner-authenticated route for an alternate trace composed through the existing
local mock verifier and mock ledger. It preserves the successful default
trace and seller-failure preview. The browser control, claim ceiling, and
proof gates are defined in the
[inconclusive preview packet](compute-bazaar-workload-inconclusive-packet.md).
This scenario does not represent an actual verifier, dispute, funds hold,
payment, or job outcome. The application, service, renderer, root validation,
import inventories, and served-browser proof gates passed for this exact
synthetic path. At 390 by 844 the controls stack within the viewport; at 1280
by 900 they fit in one row. The result heading receives focus, the browser
reported no console errors, the temporary service was stopped, and its home
was removed. Full evidence, artifact digests, assurance label, and remaining
limits are recorded in the [inconclusive preview packet](compute-bazaar-workload-inconclusive-packet.md)
and its [state slice](state-slices.md#hyperai-bazaar-workload_trace_inconclusive_servedv1).

### Fixed unresolved Data-status previews (2026-09-28)

Under `hyperai.bazaar.workload_trace_data_status_served.v1`, the authenticated
local route and existing Deals interface now expose fixed `missing`,
`conflicting`, and `denied` Data-reference previews. These previews compose
HyperAI-authored in-memory intent and request records; they perform no Data
catalog, manifest, or filesystem reads and do not assert a real missing
source, conflict, access denial, or policy decision. Focused application,
service, and renderer checks and the root `corepack pnpm run lint` gate
passed. `python3 scripts/verify_imports.py` verified all three recorded
inventories.

The served browser showed each status, the synthetic-fixture and no-source-
read wording, and the status-specific nonclaims, then returned to the ordinary
unseeded trace. The result heading received focus and browser error logs were
empty. At 390 by 844 the viewport had no horizontal overflow and all scenario
buttons fit the content column; at 1280 by 900 all three controls fit on one
row. The disposable service home was removed after stopping the service;
the pre-existing port 4317 listener remained running.

Assurance: `INTERNAL_PASS` for the stated fixed synthetic route and served
browser path. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. The source hashes, exact checks, custody, and open limits
are recorded in the [Data-status preview packet](compute-bazaar-workload-data-status-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-workload_trace_data_status_servedv1).
Actual Data reads and conditions, failed fulfillment, full accessibility,
cross-browser behavior, broad client-policy parity,
independent review, and production readiness remain open.

### Duplicate mock-disposition replay refusal (2026-09-28)

Under `hyperai.bazaar.workload_trace_duplicate_disposition_served.v1`, the
authenticated loopback route composes the existing successful synthetic
receipt and repeats its same-deal disposition call against one in-memory
`MockSettlementLedger`. The first result equals the receipt's existing
release-to-seller mock disposition; the second returns the market contract's
`Replay` error. The trace adds one digest-free replay-refusal reference and
retains exactly one receipt and one mock disposition. The market contract,
receipt schema, durable journal, and standard trace remain unchanged.

The served browser displayed the first disposition and retry refusal, stated
that there is no second receipt, durable ledger write, or funds movement,
focused the result heading, and returned to the normal trace without stale
content. At 390 by 844 the document remained 390 pixels wide and every trace
and scenario control fit within the viewport; at 1280 by 900 the document
remained 1280 pixels wide and the scenario controls fit without overflow.
Browser error logs were empty. The disposable service and browser tab were
closed and its home removed; the existing listener on port 4317 remained
running.

Focused application and service tests, all 11 workload-trace renderer tests,
the root `corepack pnpm run lint` stages, pinned import verification, the
local service build, and `git diff --check` passed. Assurance:
`INTERNAL_PASS` for this exact local synthetic route and browser path.
Reviewer status: `NOT_REQUESTED`; reproducibility: `NOT_REPRODUCED`. Source
and binary SHA-256 digests, custody, and detailed observations are recorded
in the [duplicate-disposition packet](compute-bazaar-workload-duplicate-disposition-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-workload_trace_duplicate_disposition_servedv1).
This does not establish durable idempotency, payment or funds movement,
correct computation, independent review, full accessibility,
cross-browser behavior, or production readiness. Actual owner-backed Data
reads, failed fulfillment, and broader client-policy parity remain open.

### Owner-backed Data read outcomes (2026-09-28)

The new `hyperai.bazaar.workload_trace_data_read_served.v1` slice connects the
existing Data owner's local catalog verification to the existing
owner-authenticated Workload Trace route. `bound` carries the exact verified
digest in both the intent and Data trace reference. Unresolved results carry
no digest and do not enter the buyer intent. The route, WorkloadTraceV1
schema, fixed no-read status previews, and ACP allowlist did not change.

The focused Data, application, and local-service Rust suites passed. The
served browser verified `not_seeded`, `bound`, `denied` from a permission-
limited file, `missing` after deleting the snapshot, `conflicting` after
changing its bytes, then `bound` again after restoring the exact original
sample. Each unresolved trace omitted a separate Data digest. The detail
disclosure showed the local read basis and status-specific claim ceiling. At
390 by 844 and 1280 by 900, document and body widths matched the viewport and
the console error log was empty. The disposable service/home was removed; the
user-owned service on port 4317 was preserved.

This is `INTERNAL_PASS` for these exact local synthetic paths. Reviewer
status: `NOT_REQUESTED`; reproducibility: `NOT_REPRODUCED`. Data/artifact
custody was limited to checked-out HyperAI source, HyperAI-authored synthetic
records, and the removed disposable home. No Bazaar source/data or external
source was used. This does not establish source freshness, market truth,
execution, correct computation, payment entitlement, independent review,
full accessibility, cross-browser behavior, or production readiness. The
next broader gates remain failed-fulfillment presentation, full accessibility
and policy parity, independent review if later required, and exact rights for
any upstream source/data use. See the [packet](compute-bazaar-workload-trace-data-read-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-workload_trace_data_read_servedv1).

### Static Data workspace connection metadata (2026-10-01)

Under `hyperai.bazaar.interface_status_data_workspace.v1`, the existing
static `/api/v1/status` inventory appends a `data` entry for the local
manifest-verified JSON/Parquet workspace and bounded read-only SQL path. The
Workload Trace maps `hyperai-data` references to that entry and keeps the
current Data reference status separate. The display explicitly labels the
inventory value as static metadata, not a live probe. The existing status
schema and prior connection entries remain unchanged.

The local-service status contract checks the schema, preserves the existing
ten inventory IDs in their prior order, and verifies the Data entry's owner,
state, and boundary. Renderer tests check the Data destination and static
label, plus explicit fallback when the inventory entry is absent. No Data
catalog, manifest, file, or service route is read by this inventory path.
With Node `v22.23.2`, pnpm `11.7.0`, and Rust/Cargo `1.93.1`, the focused
status contract passed 1/1, the interface suite passed 16/16, and
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 through formatting,
strict Clippy, contract tests, all-feature workspace tests, and doc tests.
Configured browser, PostgreSQL, Docker, and staged-sidecar cases remained
ignored. The exact source hashes and claim limits are in the
[state-slice record](state-slices.md#hyperai-bazaar-interface_status_data_workspacev1).

### Workload Trace semantic HTML follow-up (2026-10-01)

State slice:
[`hyperai.bazaar.workload_trace_semantic_markup.v1`](state-slices.md#hyperai-bazaar-workload_trace_semantic_markupv1).

The empty and populated trace panels use their visible heading to name the
section. Workload ID, intent revision, and trace digest use a definition list.
Reference statuses carry an explicit “Status” prefix in visually hidden text;
connection state and claim ceiling are ordinary text in the reading order.
The complete source and trace digests are included in visually hidden markup,
while sighted users keep the abbreviated digest display. The fixed renderer
suite checks these structures and that source-controlled text remains
escaped. Generic `span` and `code` elements no longer carry `aria-label`; see
the [W3C ARIA in HTML naming requirements](https://www.w3.org/TR/html-aria/).

`CARGO_NET_OFFLINE=true corepack pnpm run verify:interface` passed all 17
interface tests. `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` exited 0,
including interface tests, 3 installer tests, pinned formatting, and strict
all-target/all-feature Clippy. This is `INTERNAL_PASS` for semantic markup and
fixed renderer assertions only; reviewer status is `NOT_REQUESTED` and
reproduction is `NOT_REPRODUCED`. No browser or operating-system screen-reader
run was performed. The broader accessibility and cross-client parity gates
remain open.

### Workload Trace served semantics and keyboard follow-up (2026-10-01)

State slice:
[`hyperai.bazaar.workload_trace_digest_ax_exposure.v1`](state-slices.md#hyperai-bazaar-workload_trace_digest_ax_exposurev1).

The first served Chromium check found that the clipped full-digest span was
ignored in the accessibility tree. The renderer now exposes the full trace and
source digests as descriptions through `aria-describedby` references to hidden
text, while the abbreviated visual code stays `aria-hidden`. This follows the
[WAI-ARIA Authoring Practices guidance on descriptions](https://www.w3.org/WAI/ARIA/apg/practices/names-and-descriptions/),
which permits a referenced description to be hidden.

The dedicated `verify:interface:workload-trace-browser` mode uses an ephemeral
loopback service, fresh temporary home and Chrome profile, and only the Deals
route. It checks Chromium's computed named region, definition-list roles, full
trace/source digest descriptions, hidden visual abbreviations, and Tab/Enter
flows for seller-failure, inconclusive, and successful-return previews. The
2026-10-01 run on Google Chrome `154.0.8037.92` passed. It observed 20 local
GET requests overall, including exactly two `GET /api/v1/market/workload-trace`
requests and one each for `/failure-preview` and `/inconclusive-preview`; all
four trace responses were HTTP 200. All observed requests used GET and the
workflow recorded zero writes. All tested controls and restored headings had
visible keyboard focus.

`CARGO_NET_OFFLINE=true corepack pnpm run verify:interface` passed all 17
interface tests, and the dedicated browser workflow exited 0. This is
`INTERNAL_PASS` for the exact pinned local Chrome and synthetic preview flow.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. No OS
screen reader or other browser was tested, so this does not establish complete
accessibility or cross-browser behavior.
