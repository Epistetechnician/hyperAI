# HyperAI Synthetic HyperIntel Browser Round-Trip Packet

Status: locally qualified under
`hyperai.bazaar.interface.synthetic_hyperintel_browser_roundtrip.v1`.

Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md),
[Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md),
[Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md), and
[authenticated service packet](compute-bazaar-hyperintel-service-roundtrip-packet.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve all unrelated work.

## Purpose

The Inference UI already reads caller-selected files, posts to the
session-authenticated reconciliation route, renders a bounded report, and
projects a report card into session Activity. The API round-trip slice tests
the service contract; this slice drives the actual browser from file inputs
through the real local service and back to the Activity card.

The browser runner receives one deterministic synthetic request over stdin
from an ignored local integration test. It places the two strings in browser
`File` objects using the File API, submits the existing form, follows the
report into Activity, verifies the linked digest, and confirms the stream
canary is absent from page content and browser storage. No lineage file is
written to disk. The File API exercises the application's file-content path;
it does not claim to qualify a native operating-system file dialog.

## Exact files

- `scripts/verify-interface-browser.mjs`: add an opt-in
  `--reconciliation-stdin` scenario; permit one POST only to the exact local
  reconciliation route in this mode; retain the existing route, motion,
  contrast, external-network, and cleanup checks.
- `crates/hyperai-local-service/tests/hyperintel_reconciliation_browser.rs`:
  add an ignored test that builds the existing synthetic input in memory and
  pipes it to the browser runner without writing lineage or trust files.
- `crates/hyperintel-adapter/tests/support/reconciliation.rs`: read-only use
  of the existing test helper; do not change it.
- `docs/compute-bazaar-hyperintel-browser-roundtrip-packet.md` and
  `docs/state-slices.md`: scope and evidence.
- `docs/bazaar-hyperai-interface-plan.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: browser result and claim limits.

If the runner needs another source file, production route, or dependency,
amend this packet before proceeding.

## Allowed effects and controls

- Build only the existing local `hyperai-ui` binary offline. Run the browser
  on a headless local Chrome or Chromium process with remote debugging bound
  to IPv4 loopback.
- Start the service only on the runner's reserved ephemeral IPv4 loopback
  port, with a disposable home and browser profile. Let the runner remove both
  in its cleanup path. Do not access or modify `127.0.0.1:4317`.
- Send only the synthetic execution input and public trust registry from the
  Rust test to the Node runner through stdin. Do not put payload bytes in
  command arguments, logs, screenshots, browser storage, or repository files.
- Permit normal same-origin GET/HEAD requests and exactly one POST to
  `/api/v1/inference/reconcile-execution`. Block or fail on any other external
  request or mutating route.
- Confirm that the completed report is visible in Inference, then open
  Activity and verify the `HYPERINTEL` source label, local reconciliation
  status, exact report digest, claim limit, and return link to Inference. Verify
  the stream canary is absent from visible text, DOM markup, localStorage, and
  sessionStorage. Check narrow-layout horizontal overflow.
- No provider, seller, registry, model, non-loopback network, paid execution,
  Fleet operation, Trade action, custody, wallet, chain, reviewer, publication,
  release, or deployment is in scope.

## Proof gates

1. The source-owned synthetic builder produces a complete input within the
   adapter's combined-input limit and writes no lineage file.
2. The browser selects in-memory `File` objects, submits exactly one request
   to the owner route, and receives the report from the real service.
3. The Inference report shows `COMPLETED`, its report digest, and no execution
   stream; the response grants no authority.
4. Activity contains a source-linked report card with the same digest and a
   working route back to Inference. The stream canary appears in neither the
   rendered DOM nor browser storage.
5. The populated Inference and Activity views fit the tested mobile viewport;
   the browser sends no non-loopback or unapproved mutating request; all
   runner-owned processes and temporary roots are removed.
6. The offline `hyperai-ui` build, ignored browser integration test, runner
   syntax check, root lint, and scoped diff/Markdown hygiene pass. Report the
   standalone import check and preserved JustAI mismatch separately.

## Qualification result (2026-09-29)

The offline UI build, ignored browser integration test (1 passed), runner
syntax check, pinned formatting, strict Clippy, contract suite, and full root
lint passed. The browser reported one `COMPLETED` identity-only report, an
exact digest match in Activity, a working return link, no stream canary in DOM
or browser storage, and no horizontal overflow at 390 px. Its request monitor
allowed exactly one same-origin reconciliation POST. The test runner removed
its disposable service, browser profile, and temporary home.

The import verifier separately passed the pinned Simple Compute Market and
HyperIntel inventories, and still reports the preserved JustAI inventory
mismatch (177 files observed versus 176 recorded). This result does not
broaden the claim ceiling below.

## Current-tree revalidation (2026-09-30)

At HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, the offline
`hyperai-ui` build passed and the ignored integration test
`browser_reconciles_synthetic_lineage_and_links_activity_report` passed 1/1
with Google Chrome `154.0.8037.92`. The test retained its assertions for one
owner-route reconciliation POST, `COMPLETED`/no-authority status, lowercase
report digest equality between Inference and Activity, working return link,
canary absence from the DOM and browser storage, and the 390 px viewport.

The first invocation stopped before app startup because
`/opt/homebrew/bin/chromium` referenced a removed `/Applications/Chromium.app`.
The passing invocation set `CHROME_BIN` to the installed Google Chrome
executable directly. This was a launcher-path failure, not an application
result. The runner's disposable browser and service cleanup completed; port
`127.0.0.1:4317` had no listener before or after and was not contacted.

The exact passing test command was:

```sh
CARGO_NET_OFFLINE=true CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
  sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p hyperai-local-service --test hyperintel_reconciliation_browser -- \
  --ignored --exact browser_reconciles_synthetic_lineage_and_links_activity_report --nocapture
```

The offline `hyperai-ui` build passed using
`cargo build --locked --offline -p outcomes-cli --bin hyperai-ui` under the
pinned toolchain. The runner, ignored test source, and resulting binary
SHA-256 values were respectively
`737e2653ed66fb167404d0cdc68212a88ae4b0b43f41e87651896cf8cf3077f5`,
`873f76a68a55c8da44ceddede7509775840f337bc76cc96cd460dea2664e3b89`, and
`7feb0eb34de13b80204c79a3d0b6028a83b3de58e82bbd43926423ce54aa4ea9`.
The runner syntax and root lint gates were already passing for this runner
source hash in the 2026-09-30 Data browser qualification; they were not rerun
as part of this round-trip revalidation. The direct Markdown whitespace and
packet-anchor checks passed for this documentation update.

Assurance remains `INTERNAL_PASS` for the exact same-host synthetic browser
journey. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. The revalidation changes no production route, application
behavior, provider access, or claim ceiling.

## Claim ceiling and next gates

Maximum claim: `INTERNAL_PASS` for the exact same-host headless-browser flow
using synthetic records, in-memory File API objects, one loopback service, and
one desktop/mobile browser implementation. This is not native file-picker,
assistive-technology, cross-browser, cross-platform, separate-host, source
truth, correct-computation, payment, settlement, independent-review, or
production evidence.

Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Input
bytes exist only in process memory; the service home and browser profile are
disposable and removed. Future native platform and full accessibility gates
remain open. Publish boundary: local test and runner source plus documentation
only; no commit, PR, external circulation, release, or deployment is
authorized.
