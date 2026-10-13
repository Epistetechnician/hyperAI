# HyperAI Data Full Keyboard Accessible-Name Packet v3

Status: frozen local qualification continuation. No implementation changes are
authorized outside this state slice.

State slice: `hyperai.bazaar.interface.data_full_keyboard_ax_names.v1`.
Parent: [Phase 2 Data contract packet](compute-bazaar-phase-2-data-packet.md).
Product direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Preceding run: [full keyboard v2 packet](compute-bazaar-interface-data-full-keyboard-v2-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
current dirty and untracked worktree. Preserve all pre-existing state.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and exact delta

The v2 browser workflow passed its synthetic Data path, but its focus helper
treated nonempty label `textContent` as an accessible-name check. That text can
include a control's current value or every option in a select. This continuation
replaces that proxy with the accessible role and computed name returned by the
pinned Chromium Accessibility tree for each focused control. The protocol
method is [`Accessibility.getPartialAXTree`](https://chromedevtools.github.io/devtools-protocol/tot/Accessibility/#method-getPartialAXTree).
This is browser accessibility-tree evidence, not an OS screen-reader test.

Application source and runtime/API behavior are read-only. The harness also
retains its visible-focus check, keyboard-only actions, form/error/digest
assertions, and exact same-origin request policy.

## Frozen accessible-name expectations

During the same keyboard workflow, require Chromium's computed role and name
to match every entry below. For editable text fields, inspect the accessible
node after keyboard focus is established and before changing the value.

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

The AX node must be present, not ignored, and match both frozen fields exactly.
Keep the existing `role=status`, live announcement, focus, retained-invalid-
input, visible-focus, digest, and saved-inventory assertions.

## Exact files and permitted effects

- `scripts/verify-interface-browser.mjs`: add a new opt-in mode; enable the CDP
  DOM and Accessibility domains; read the target node with `DOM.querySelector`
  and `Accessibility.getPartialAXTree`; replace `textContent`-derived focus
  names with the computed role/name; assert the frozen expectations in the full
  keyboard journey. Do not change the application.
- `package.json`: expose `verify:interface:data-full-keyboard-ax` without adding
  dependencies or including browser work in root lint.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record execution results and limits.

Application files are read-only. Their frozen SHA-256 values are:

| File | SHA-256 |
|---|---|
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |

Harness baseline SHA-256:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `d013b8dc9ed14ce0fdfe308ff53a5d90fc96116f96f64c3a24702ea8fc818dce` |
| `package.json` | `1493eee78a241a90abe982a183d614e3a54429e28b3cb1f79168788230ad0f6d` |

Toolchain freeze: macOS arm64, pinned Rust/Cargo `1.93.1`, Node.js `22.23.2`,
pnpm `11.7.0`, and Google Chrome `154.0.8037.58`. The system Rust outside
`scripts/with-pinned-rust.sh` is not used.

Run once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:data-full-keyboard-ax
```

The command uses the existing runner's read-only 33 route/viewport and 33
reduced-motion checks, then reloads the page before the measured workflow. The
workflow uses CDP keyboard events to create the fixed HyperAI synthetic
snapshot; query it; inspect and focus the named result region; preview and save
one model; test invalid view columns; correct, preview, and explicitly save one
view. DOM and AX reads are inspection only. No DOM click, direct API setup, or
non-synthetic data is allowed in the measured journey.

The only allowed POSTs remain the v2 packet's exact route/count/status set:
synthetic seed `1 x 200`, query `1 x 200`, model preview/save `1 x 200` each,
view preview `2 x (422, 200)`, and view save `1 x 200`. Other requests must be
same-origin `GET`/`HEAD`. Do not connect to the existing service on
`127.0.0.1:4317`; use a fresh temporary home/profile and ephemeral loopback
ports, then remove all runner-owned state.

## Stop conditions, gates, and claim ceiling

Stop and preserve the literal result on missing/ignored AX nodes, any role or
name mismatch, keyboard/focus failure, unexpected request/status, browser
error, port collision, save ambiguity, or failed cleanup. Do not repeat the
same frozen command under this identity. If the app fails an expectation,
record the exact failure and define a new implementation slice before changing
app source.

Proof gates: exact frozen control roles/names; visible keyboard focus on
required controls; the existing full query/model/view journey; unchanged Data
status, input-retention and digest-confirmation semantics; exact POST counts and
statuses; successful runner cleanup; Node syntax and package JSON checks;
`git diff --check` and scoped Markdown-link hygiene; root `corepack pnpm run
lint`; and import inventory with any pre-existing mismatch reported unchanged.

Maximum claim: `INTERNAL_PASS` for exact Chromium AX role/name computation in
the listed synthetic Data workflow on this same host, plus the enumerated
keyboard and focus behavior. Reviewer status is `NOT_REQUESTED` and
reproducibility is `NOT_REPRODUCED`. This does not establish VoiceOver or
another OS screen reader, physical-keyboard hardware, another browser/platform,
full Phase 2 acceptance, independent review, source rights, or production
readiness. No provider, upstream repository, external origin, live Fleet,
paid Eval, settlement, release, publication, or deployment is in scope.

## Execution result (2026-09-30)

The one-shot command exited 1 after the synthetic snapshot, query, and result
region checks. At the query-model name field, Chromium returned role `textbox`
and accessible name `MODEL NAME`; the frozen expectation was `Model name`.
The node was present and not ignored. The application CSS deliberately applies
`text-transform: uppercase` to `.data-field > span` labels, so this is a
rendered-name capitalization difference, not evidence that the field has no
accessible name. Per the stop condition, the runner did not type the model
name, preview or save a model, or reach the view workflow. Do not repeat this
v3 command.

Gate status is `INCONCLUSIVE` for the complete workflow; the exact role/name
mismatch is retained above. Reviewer status is `NOT_REQUESTED` and
reproducibility is `NOT_REPRODUCED`. Runner-owned temporary roots were removed;
the existing listener PID 85094 on `127.0.0.1:4317` was preserved and not
contacted. No browser artifact or output digest was retained. A new named
continuation is required to compare the semantic name while accounting for
presentation-only casing.
