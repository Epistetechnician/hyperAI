# HyperAI Data Full Keyboard Journey Packet v2

Status: completed local keyboard workflow; `INTERNAL_PASS` for this exact
same-host headless-Chrome path. Overall Phase 2 remains `NOT_ASSIGNED`.

State slice: `hyperai.bazaar.interface.data_full_keyboard_journey.v2`.
Parent: [Phase 2 Data contract packet](compute-bazaar-phase-2-data-packet.md).
Product direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Prior result: [v1 packet](compute-bazaar-interface-data-full-keyboard-packet.md)
records `INCONCLUSIVE`; its identical command will not be repeated.
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve every pre-existing path.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and exact delta

Exercise the complete keyboard-only local Data workflow from the initial page
through a digest-bound saved view. V1 demonstrated that its generic select-all
sequence did not replace a prefilled SQL textarea. This frozen continuation
uses `Home`, then `Shift+End`, verifies that the whole single-line value is
selected, and only then types the fixed SQL. All other controls continue to
use keyboard navigation and activation. This is a test-harness correction;
v1 did not establish a product defect. No application, API, storage, or data
contract change is authorized by this slice.

## Frozen journey and request boundary

Build the pinned `hyperai-ui` binary offline, then start a runner-owned service
on an ephemeral IPv4 loopback port with a fresh temporary home and Chrome
profile. After the script's read-only route/viewport preflight, reload the
initial workspace page and use only CDP keyboard input to:

1. Navigate to Data, create the fixed HyperAI synthetic snapshot, focus the
   SQL textarea, select its full one-line query using Home plus Shift+End, and
   execute the bounded query.
2. Reach and inspect the result table by keyboard; require its accessible name,
   visible keyboard focus, three rows, and snapshot digest.
3. Preview and explicitly save one fixed synthetic query model by keyboard.
4. Enter a view name, keyboard-navigate the presentation selector, reject one
   invalid column selection, correct the field, preview, and explicitly save
   the view by keyboard.
5. Require accessible focused status after validation, retained editable
   values, no confirmation before a valid digest-bound preview, and saved
   definition digests in the inventory.

Only same-origin GET/HEAD requests and these POSTs are allowed:

| Route | Count | Statuses |
|---|---:|---|
| `/api/v1/data/synthetic/seed` | 1 | `200` |
| `/api/v1/data/query` | 1 | `200` |
| `/api/v1/data/models/preview` | 1 | `200` |
| `/api/v1/data/models/save` | 1 | `200` |
| `/api/v1/data/views/preview` | 2 | `422`, `200` |
| `/api/v1/data/views/save` | 1 | `200` |

No DOM `.click()`, pointer event, direct API setup, external request, or
non-synthetic source is allowed in the measured journey. DOM inspection is
read-only. The earlier route/viewport audit uses script clicks for GET-only
navigation; the measured workflow reloads before beginning. Do not contact the
existing listener at `127.0.0.1:4317`.

## Exact files and permitted effects

- `scripts/verify-interface-browser.mjs`: add a textarea-specific keyboard
  selection helper; assert selection start/end before typing; retain exact
  action, focus, accessible-name, digest, route/status, and cleanup checks.
- `package.json`: add `verify:interface:data-full-keyboard`; no dependency or
  root-lint composition change.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record the frozen result and limits.

Application source is read-only in v2. No import tree, manifest, lockfile,
runtime API, or service is changed. The pre-implementation hashes are:

| File | SHA-256 |
|---|---|
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `scripts/verify-interface-browser.mjs` | `1d60debc0c8079a39041952075c023a58045a9d5c301145a505e5843f54a4701` |
| `package.json` | `1493eee78a241a90abe982a183d614e3a54429e28b3cb1f79168788230ad0f6d` |

Toolchain freeze: Rust/Cargo `1.93.1`, Node.js `22.23.2`, pnpm `11.7.0`, and
Google Chrome `154.0.8037.58` on macOS arm64. Use
`CARGO_NET_OFFLINE=true corepack pnpm run
verify:interface:data-full-keyboard`, followed by the packet's syntax,
root-lint, import, and documentation checks. The root test command must be
`corepack pnpm run lint`; nested workspaces stay out of scope.

## Stop conditions and claim ceiling

Stop on incorrect textarea selection, a focus/name/indicator failure, lost
field values, stale/missing digest, unexpected request/status, browser error,
process or port collision, ambiguous save, or failed cleanup. Preserve the
literal result and do not repeat the same frozen command under v2.

Allowed runtime effects are limited to the pinned offline build, isolated
ephemeral loopback service and browser, one fixed synthetic seed/query/model/
view sequence, and disposable local output. No provider, seller, upstream
repository, external data, model execution, Fleet operation, paid evaluation,
payment, settlement, chain write, publication, release, commit, or deployment
is included.

Maximum claim: `INTERNAL_PASS` for this exact same-host headless-Chrome CDP
keyboard workflow. Reviewer status is `NOT_REQUESTED`; reproduction starts
`NOT_REPRODUCED`. It does not establish hardware keyboard use, OS screen-reader
operation, cross-browser/native-platform behavior, overall Phase 2 acceptance,
rights, independent review, or production readiness. No commit or external
circulation is authorized.

## Execution record (2026-09-30)

`CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:data-full-keyboard`
exited 0 on the frozen macOS arm64 / Rust 1.93.1 / Node 22.23.2 / pnpm 11.7.0
host with Google Chrome 154.0.8037.58. All 33 route/viewport checks and 33
reduced-motion checks passed. The measured CDP-only journey created the fixed
synthetic snapshot, selected and ran the SQL query, inspected its focused
three-row result, previewed and saved the query model, navigated the view
presentation control, retained the invalid column after the expected 422,
then corrected, previewed, and explicitly saved the view. Every required
control showed a focus indicator; the result region had its required accessible
name. The run recorded 55 same-origin requests and only the frozen GET/HEAD
reads and Data POST route counts/statuses. It contacted no external origin.

The fixed synthetic result used dataset
`hyperai.synthetic_market_observations_json.v1` and snapshot digest
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`. The
saved model digest was
`6989aa6a4daff1242e678b6cc133527e25c3c95f6593aebfd9972210f01ffc6e`; the
saved view digest was
`189e0266818318ecd99cafaa1a0613610b8ba4a79b3cc41dfbad4beb89020adc`.
No application code changed in v2. Post-run SHA-256 values are recorded for
the test runner and package manifest; the app JavaScript and CSS remain at
their frozen digests above.

The runner removed its temporary home/profile and stopped its owned service
and Chrome process. No runner temporary root or owned process remained;
pre-existing PID 85094 on `127.0.0.1:4317` was unchanged and not contacted.
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 with the v2 runner and
execution record present. It passed interface and installer checks, pinned
formatting, strict all-target Clippy, contract tests, all-feature workspace
tests, and doc tests; environment-gated tests remained ignored. Runner syntax,
package JSON parsing, and `git diff --check` also passed. Post-run SHA-256:
`scripts/verify-interface-browser.mjs`
`d013b8dc9ed14ce0fdfe308ff53a5d90fc96116f96f64c3a24702ea8fc818dce`,
`package.json`
`1493eee78a241a90abe982a183d614e3a54429e28b3cb1f79168788230ad0f6d`,
`apps/hyperai-interface/app.js`
`f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd`, and
`apps/hyperai-interface/app.css`
`5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046`.

`corepack pnpm run verify:imports` matched Simple Compute Market: 2,189 files,
SHA-256 `afe6d9363a24fa627d9530240c190b6e68cdd1b7d6425d54d0b7547d829207ab`,
revision `a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`; and HyperIntel: 99 files,
SHA-256 `62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`,
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. It exited 1 for the
pre-existing JustAI inventory mismatch: 177 files observed versus 176
recorded, SHA-256 `08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`,
revision `9a0e56536b968cbb2cb7173e142068b53c4f9fe3`. This state was not changed.
The final text-only additions recording these gate results were checked with
`git diff --check` and local Markdown-link validation.

Assurance is `INTERNAL_PASS` for this exact synthetic same-host headless-Chrome
workflow. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. The earlier v1 select-all failure remains recorded and was
not repeated under its frozen identity. No conclusion is made about physical
keyboard hardware, OS screen-reader behavior, cross-browser/native-platform
behavior, full Phase 2 acceptance, source rights, independent review, or
production readiness.
