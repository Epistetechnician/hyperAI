# HyperAI Data Full Keyboard Accessible-Name Packet v4

Status: frozen local qualification continuation. No implementation changes are
authorized outside this state slice.

State slice: `hyperai.bazaar.interface.data_full_keyboard_ax_semantic_names.v1`.
Parent: [Phase 2 Data contract packet](compute-bazaar-phase-2-data-packet.md).
Product direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Preceding stop: [v3 exact-name packet](compute-bazaar-interface-data-full-keyboard-ax-v3-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
current dirty and untracked worktree. Preserve all pre-existing state.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and frozen comparison

V3 stopped when Chromium returned the accessible name `MODEL NAME` for the
field labeled `Model name`. The app intentionally uppercases `.data-field >
span` through CSS. V4 keeps the browser-computed AX tree as the evidence source
and compares names using this exact normalization on both expected and actual
names:

1. Unicode NFC normalization.
2. Collapse each run of Unicode whitespace to one ASCII space and trim ends.
3. Convert ASCII `A` through `Z` to lowercase. Do not change other characters.

Roles remain exact and case-sensitive. Normalized names remain exact for every
word, space, and punctuation character. The raw browser role and name must be
retained in workflow output. This checks the computed browser name while
ignoring authored-vs-rendered letter casing; it is not an OS screen-reader
test.

## Frozen accessible-name expectations

| Control | Chromium role | Accessible name |
|---|---|---|
| Data navigation | `button` | `Data` |
| Synthetic snapshot action | `button` | `Create local synthetic snapshot` |
| SQL editor | `textbox` | `SQL` |
| Query submission | `button` | `Run bounded query` |
| Query results container | `region` | `Query results table` |
| Query model name | `textbox` | `Model name` |
| Model preview | `button` | `Preview model` |
| Model confirmation | `button` | `Confirm local save` |
| View name | `textbox` | `View name` |
| View presentation | `combobox` | `Presentation` |
| View columns | `textbox` | `Columns` |
| View preview, including corrected retry | `button` | `Preview view` |
| View confirmation | `button` | `Confirm view save` |

Each target AX node must be present and not ignored. Keep exact role checks,
visible keyboard-focus checks, `role=status`, live announcements,
retained-invalid-input, digest confirmation, and saved-inventory assertions.

## Exact files and permitted effects

- `scripts/verify-interface-browser.mjs`: add a new opt-in mode using the
  frozen normalization above, retaining raw AX names and the existing full
  keyboard journey. Keep the v3 exact-name mode unchanged. Do not change the
  application.
- `package.json`: expose
  `verify:interface:data-full-keyboard-ax-semantic`; add no dependencies and
  keep browser work outside root lint.
- Record v3 and v4 outcomes in this packet, the v3 packet,
  `docs/state-slices.md`, `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`.

Application files are read-only. Their frozen SHA-256 values are:

| File | SHA-256 |
|---|---|
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |

Harness baseline SHA-256:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `73f7efecbc6715dc05c738cea9022e85f5c392a6284a80ad4e2a2241527ccf43` |
| `package.json` | `f79260facd56c261ca71ad376cb4c72f3d32f2b4b72ce1807040ee3a0036081a` |

Toolchain freeze: macOS arm64, pinned Rust/Cargo `1.93.1`, Node.js `22.23.2`,
pnpm `11.7.0`, and Google Chrome `154.0.8037.58`. The system Rust outside
`scripts/with-pinned-rust.sh` is not used.

Run once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:data-full-keyboard-ax-semantic
```

The command runs the existing read-only 33 route/viewport and 33 reduced-motion
checks, then reloads before the measured journey. CDP keyboard events create a
fixed HyperAI synthetic snapshot, query it, focus the named result region,
preview and save one model, submit an invalid view column, correct it, preview
and explicitly save one view. CDP DOM/AX reads are inspection only. No DOM
click or direct API setup is allowed in the measured journey.

The only permitted POSTs are: synthetic seed `1 x 200`, query `1 x 200`, model
preview/save `1 x 200` each, view preview `2 x (422, 200)`, and view save
`1 x 200`. Other requests must be same-origin `GET`/`HEAD`. Do not connect to
`127.0.0.1:4317`; use a fresh temporary home/profile and ephemeral loopback
ports, then remove runner-owned state.

## Stop conditions, gates, and claim ceiling

Stop and preserve the literal result on absent or ignored AX nodes, any exact
role mismatch, any v4 normalized-name mismatch, keyboard/focus failure,
unexpected request/status, browser error, port collision, save ambiguity, or
failed cleanup. Do not repeat this frozen command or change normalization
under this identity. Any product correction needs its own state slice.

Proof gates: all frozen roles and normalized names; raw AX names present in
the output; visible focus and the full query/model/view journey; unchanged Data
status, input-retention and digest-confirmation semantics; exact POST counts and
statuses; successful cleanup; syntax and package JSON checks; `git diff
--check` and scoped Markdown-link hygiene; root `corepack pnpm run lint`; and
import inventory with any pre-existing mismatch reported unchanged.

Maximum claim: `INTERNAL_PASS` for the exact Chromium roles, normalized names,
and listed synthetic Data keyboard workflow on this same host. Reviewer status
is `NOT_REQUESTED` and reproducibility is `NOT_REPRODUCED`. This does not
establish VoiceOver or another OS screen reader, physical-keyboard hardware,
another browser/platform, complete Phase 2 acceptance, independent review,
source rights, or production readiness. No provider, upstream repository,
external origin, live Fleet, paid Eval, settlement, release, publication, or
deployment is in scope.

## Execution result (2026-09-30)

The frozen command exited 0 on macOS 26.6.1 arm64 with Node.js 22.23.2,
pnpm 11.7.0, pinned Rust/Cargo 1.93.1, and Chrome 154.0.8037.58. All 13
role/name expectation rows passed, including the corrected view-preview
repeat. Raw Chromium output included `MODEL NAME`, `VIEW NAME`, `PRESENTATION`,
and `COLUMNS`; role values matched exactly and the frozen normalization
matched the authored names.

All 33 route/viewport checks, 33 reduced-motion checks, and the full synthetic
keyboard path passed. The journey queried three rows, saved a digest-bound
model, retained invalid columns after HTTP 422, corrected them, then previewed
and saved the view. It observed 55 same-origin requests and exactly the frozen
seven Data POSTs; the six route/status groups matched the frozen expectations.
Synthetic dataset ID: `hyperai.synthetic_market_observations_json.v1`;
snapshot digest: `3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`;
model definition digest: `6989aa6a4daff1242e678b6cc133527e25c3c95f6593aebfd9972210f01ffc6e`;
view definition digest: `189e0266818318ecd99cafaa1a0613610b8ba4a79b3cc41dfbad4beb89020adc`.

Assurance is `INTERNAL_PASS` for this exact same-host Chromium workflow.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Temporary profiles and runner-owned processes were removed; no screenshot or
browser artifact was retained. PID 85094 on `127.0.0.1:4317` remained
untouched. The root `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0.
The HTML, JavaScript, and CSS hashes remained equal to the frozen baseline.
Runner syntax, package JSON, and `git diff --check` passed. Simple Compute
Market and HyperIntel import inventories matched their pins; JustAI remains
mismatched at 177 observed versus 176 recorded, with its tree untouched.
Post-run harness hashes are runner
`48e6bf7a09233663905340bada278bca4395476ba9b9113998e37a7311918d00` and
package `85fc2a382acd45ba044b93a8b90ef8aa555b913fe73612977afaa8b907084c13`.
The v3 stop remains `INCONCLUSIVE` and was not repeated. Phase 2 remains
`NOT_ASSIGNED`.
