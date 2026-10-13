# HyperAI Interface Task Navigation Keyboard Packet v1

Status: consumed. Its one-shot command `FAILED` before browser assertions;
keyboard acceptance remains `INCONCLUSIVE`. Do not repeat this identity.
Product HTML, CSS, and JavaScript are read-only. Any product correction or
further execution requires a new state slice.

State slice: `hyperai.bazaar.interface.task_navigation_keyboard.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md).
Interface direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the pre-existing dirty and untracked worktree. Preserve it. Before this slice,
`git status --porcelain=v1 -z` SHA-256 was
`5dfab8730943d0b665d67ac98159766a0a546ed7b96361ae154ef078b88d26eb`; the
staged path list was empty.

## Purpose

The Work / Research / Operate route grouping was added after the earlier
all-route browser accessibility check. That check covered keyboard focus
traversal and one mobile navigation path, but did not bind its evidence to the
current grouped navigation. The existing route browser audit exercises routes
with DOM clicks. This packet qualifies current navigation using browser-level
Tab, Enter, and Escape events without changing a product asset or making a
domain request that writes state.

## Frozen inputs

| Input | SHA-256 |
|---|---|
| `apps/hyperai-interface/index.html` | `026c82dbc0ea75b115b9428adf918e6800564425f5c19f939c334694783d38f1` |
| `apps/hyperai-interface/app.js` | `9764756b31e215e545d88ed60aa91e2ea75a281ebe1fa4601413c88061cb2ad1` |
| `apps/hyperai-interface/app.css` | `ded42d5f92c5e96495a30199bae36b8d86d2eb54f717f201f7a857c7d26f5251` |
| `scripts/verify-interface-browser.mjs` | `2116adc5f864db69dfedfdc9c5c525bb224d8ce3a43af54519e358fc928ceb2e` |
| `package.json` | `59348851ace8cff9cd09bb4de04c09583a6d55ca369b138c176f9e34a2a82bff` |
| `Cargo.lock` | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `rust-toolchain.toml` | `7134cca955184c22a7374bda78be176b570e6d8b69ee92dc02d5efc3044212e8` |
| `scripts/with-pinned-rust.sh` | `d0bb19b6f621a31e10a7b46be852889a4f69bd0196ba7545abd9219b36f51171` |

Toolchain: macOS 26.6.1 arm64, Rust/Cargo 1.93.1, Node.js 22.23.2,
pnpm 11.7.0, and Google Chrome 154.0.8037.92. The browser runner reserves
fresh IPv4 loopback ports and uses a disposable home and Chrome profile.
The pre-existing service on `127.0.0.1:4317` is outside this packet.

## Scope and measured workflow

Writable files are `scripts/verify-interface-browser.mjs`, `package.json`,
this packet, `docs/state-slices.md`, `docs/compute-bazaar-phase-1-contract-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`. The three browser assets and all
service/domain contracts are read-only. Add no dependency, route, schema,
permission, or product action.

The one-shot runner mode must:

1. Use only CDP keyboard events to traverse and activate all 11 current route
   buttons: Overview, Market, Deals, Data, Inference, Fleet, Eval, Trade,
   Evidence, Agents & access, and Activity. For each route, require a visible
   keyboard focus indicator, computed Chromium role `button`, exact accessible
   name, successful Enter activation, the matching `aria-current="page"`,
   exactly one matching level-one workspace heading, and visible focus on that
   heading after navigation.
2. Preserve the navigation order by group: Work (Overview, Market, Deals),
   Research (Data, Eval, Trade), and Operate (Inference, Fleet, Evidence,
   Agents & access, Activity).
3. At 390 by 844 CSS pixels, use the keyboard to open the mobile navigation,
   verify the expanded state and focus on the current route, select Data, and
   verify the menu closes and focus moves to the Data heading. From a fresh
   page, open the menu again, press Escape, and verify it closes and restores
   visible focus to the navigation toggle.
4. Perform no DOM click, direct focus call, route-state mutation, form submit,
   or action activation in the measured workflow. Observe only same-origin
   `GET` and `HEAD` requests; require zero `POST` or other write requests.
5. Fail on a browser/runtime error, external request, unexpected HTTP method,
   route/name/focus mismatch, or cleanup failure. Stop and preserve the literal
   result; do not repeat this identity or weaken an expectation.

## Exact run and proof gates

Expose one root command named
`corepack pnpm run verify:interface:task-navigation-keyboard`. It runs the
script syntax check, builds the pinned `hyperai-ui` binary offline, and
executes the new workflow mode. Run this command once. Then run
`corepack pnpm run lint:fast`, `python3 scripts/verify_imports.py`, and
documentation/staging hygiene checks. Do not run `verify:full` under this
slice. Verify the frozen browser assets and Cargo/toolchain inputs still
match; only the named runner and package script change, with their resulting
hashes recorded. Verify that the runner-owned service and Chrome process group
exit, the reserved ports close, temporary state is removed, and no file is
staged.

## Claim ceiling

Target assurance is `INTERNAL_PASS` for the exact current-source, same-host
Chrome keyboard route and mobile menu workflow only. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. CDP key events do not
establish physical-keyboard, VoiceOver, other screen-reader, other-browser,
native-platform, full workflow accessibility, complete client-policy parity,
independent-review, source-rights, or production readiness. No external source,
registry, seller, provider, agent, live Fleet, paid Eval, financial Trade,
settlement, publication, or deployment is in scope.

Data and artifact custody is limited to HyperAI source, fixed local synthetic
empty states, and disposable browser/service state. Publish boundary: local
source and documentation only; no commit, PR, release, or deployment.

## Execution record (2026-10-02)

After isolated installation of the pinned toolchain, the offline `hyperai-ui`
build completed. The one-shot then exited 1 before browser assertions with
`Browser audit error before cleanup: spawnSync lsof ENOENT` and
`HyperAI interface browser audit failed: spawnSync lsof ENOENT`. The runner's
child-process `PATH` omitted `/usr/sbin`, where macOS provides `lsof`. This
execution consumed the v1 identity; it was not retried. No `lint:fast` or
import inventory follow-up ran. The pre-existing `target/debug/hyperai-ui`
binary was restored byte-for-byte with SHA-256
`48bd6d3bb40f76f561e86e6f1607e9f0efa022dadfe94f1f3e701727993ca362`. Process
inspection found no runner-owned service, browser, or active browser runner;
port 4317 was not listening. The only matching browser temp directory had
birth time 2026-10-01 08:19:33, predating this execution, and was preserved
because ownership is unresolved. The keyboard behavior itself is
`INCONCLUSIVE`, not a pass or product failure.

Any continuation must use a new identity. The authorized continuation is
[`task-navigation keyboard path recovery v1`](compute-bazaar-interface-task-navigation-keyboard-path-recovery-v1-packet.md),
which inherits all assertions and adds `/usr/sbin` to `PATH`; it does not
reopen this consumed packet.

Follow-up record (2026-10-02): source diagnosis found the first Overview wait
was impossible because the runner reused the navigation name `Overview` as the
expected H1, while the actual heading is `Compute, with context.`. The fresh
[`heading contract v1`](compute-bazaar-interface-task-navigation-heading-contract-v1-packet.md)
changed only that runner expectation and passed its one-shot workflow plus the
authorized fast lint and import checks. This later result does not change the
literal consumed v1 execution above or broaden its claim ceiling.
