# HyperAI Interface Headless Browser Layout Packet

State slice: `hyperai.bazaar.interface.headless_browser_layout.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md), [Phase 1
contract packet](compute-bazaar-phase-1-contract-packet.md), and [Rust port
subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

The earlier browser attempt under
`hyperai.bazaar.workload_trace_browser_cdp_websocket.v1` was inconclusive and
requires a newly named slice before another browser attempt or controller
change. This packet authorizes a headless Chromium layout and navigation audit
using a small Node DevTools Protocol client instead of Puppeteer. It exercises
the current HyperAI-authored interface only; it imports no Bazaar code, data,
fixtures, or assets.

## Exact files

- `scripts/verify-interface-browser.mjs`: dependency-free local runner for a
  disposable interface service, headless Chrome session, fixed route/viewport
  checks, request-origin and method controls, and temporary screenshot output.
- `package.json`: expose the optional browser audit without adding it to root
  lint, which must remain usable when Chrome is unavailable.
- `apps/hyperai-interface/index.html`: add a self-contained SVG data-URI
  favicon so the browser does not fall back to a missing `/favicon.ico` route.
- `apps/hyperai-interface/app.js`: move route-change focus from the full page
  region to the new workspace heading, so the existing focus indicator marks
  the user's destination without outlining the entire page.
- `docs/compute-bazaar-interface-browser-packet.md`: scope and evidence.
- `docs/state-slices.md`: named slice, custody, assurance, and claim ceiling.
- `docs/bazaar-hyperai-interface-plan.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: record the result and remaining
  product-usability gaps.

## Allowed effects and controls

- Build `hyperai-ui` with the pinned toolchain and locked offline dependencies.
- Run one `hyperai-ui` process against a newly created empty synthetic home on
  an ephemeral IPv4 loopback port. Never connect to the existing user service
  on port 4317 or inspect its home.
- Launch the installed Google Chrome binary in headless mode with a fresh
  temporary profile and DevTools bound to IPv4 loopback. Disable background
  networking, synchronization, component updates, and non-local name
  resolution. The runner must abort if a browser request is not same-origin
  loopback or if a request method is not `GET`/`HEAD`.
- Visit only the eleven fixed workspace routes: Overview, Market, Deals, Data,
  Inference, Fleet, Eval, Trade, Evidence, Agents & Access, and Activity. Do
  not press workflow, import, save, approval, evaluation, Trade, agent, or
  service-action controls. The only source corrections allowed by this packet
  are the inline local favicon in `index.html` and the narrow route-focus
  target in `app.js`; neither adds a request route, dependency, or domain
  interaction.
- Check the empty synthetic state at 1440x1000, 720x500 (the CSS viewport
  equivalent of 200% zoom from 1440x1000), and 390x844. Record route heading,
  active navigation, content width, document overflow, unnamed interactive
  controls, and browser console/runtime errors. Retain screenshots only in a
  caller-selected temporary directory for immediate visual inspection.
- Stop only the service and Chrome processes started by this runner. Remove
  its temporary home, browser profile, logs, and screenshots after inspection.

No external network, provider, seller, registry, remote worker, user payload,
existing local home, payment, chain, publication, deployment, or persistent
synthetic record is in scope. Stop before any unexpected API write, external
request, non-loopback listener/socket, service collision, or unowned process.

## Proof gates and claim ceiling

- Confirm the live service listener and DevTools listener bind only to
  `127.0.0.1`, and all observed page requests use same-origin loopback and
  `GET`/`HEAD` methods.
- Check each fixed route at all three viewports for a single page heading,
  matching active navigation, no document-level horizontal overflow, no
  unnamed enabled buttons/links/form controls, a focused heading after route
  selection, and no browser runtime error or missing favicon request. The
  focus indicator must follow the heading rather than outline the full page.
- Visually inspect a small representative screenshot set before deleting it.
- Verify the service, browser, temporary home, profile, and screenshots are
  cleaned up and the existing listener on port 4317 is unchanged.
- Run the optional browser audit, root `corepack pnpm run lint`, pinned Rust
  formatting, import inventory, and scoped documentation/link/whitespace
  checks. Report the JustAI inventory mismatch separately if still present.

## Results (2026-09-29)

The exact current worktree passed `corepack pnpm run verify:interface:browser`
on macOS 26.6.1 (build 25G76, arm64), with Rust/Cargo 1.93.1, Node.js
22.23.2, pnpm 11.7.0, Python 3.14.5, and Google Chrome 154.0.8037.58. The
runner built `hyperai-ui` using `--locked --offline`, started it with a new
empty synthetic home on an ephemeral IPv4 loopback port, and started Chrome
with a fresh profile and DevTools on IPv4 loopback.

All 33 route/viewport checks passed: eleven routes at 1440x1000, 720x500, and
390x844 CSS pixels. Route headings and active navigation matched; route
selection focused the visible `workspace-page-title`; enabled controls had
accessible names; the mobile menu closed after selection; and document width
matched scroll width for every check. The browser made 17 observed requests,
all `GET`, to the same-origin loopback page or the authored inline SVG icon.
There were no runtime, console, or browser-log errors and no `/favicon.ico`
fallback request. The representative desktop Overview, 720x500 Data, and
mobile Deals screenshots from the successful visual run were inspected and
deleted. A leftover runner-created directory from an earlier invocation held
only those three expected image names and was also removed. The inline favicon
fixed the missing-resource fallback found during the earlier attempt. Moving
route focus to the heading removed the oversized focus outline around the page
region.

The root `corepack pnpm run lint`, pinned `cargo fmt --all -- --check`, and
`git diff --check` passed. `python3 scripts/verify_imports.py` passed for
Simple Compute Market and HyperIntel; it still reports the recorded JustAI
inventory mismatch caused by the additional `products/justai/pnpm-lock.yaml`.
That unrelated snapshot was preserved without modification.

The page sources, browser runner, package script, build inputs, and built
binary used by this run are identified by SHA-256:

| File | SHA-256 |
| --- | --- |
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `a25c9d725d51e564a967cf4643bcbf64b91cabf75bc9e3db03f9d0b0bc1a1cc5` |
| `scripts/verify-interface-browser.mjs` | `4d0d56d1bbd77b33244f0f26d2d6d2c91cfe2ed44932cbf6fff80a89e3cae2a8` |
| `package.json` | `d99dcc90a2d41226f8b9abb5381e36b18065234c961c570b7b65e5d07a7c3ca3` |
| `Cargo.toml` | `78c6180d81c3c7dabff4de50b473bf04eaf7e7ac5ee82a967e7a39fa05797a6f` |
| `Cargo.lock` | `b3708a2e0577a3e3d4a07c9110b33ae0ff0676b7aaaf495cf4278ba46e7e6736` |
| `crates/outcomes-cli/Cargo.toml` | `8a0ccfcd67e8e232d655cb9b02104b75a04259df2a4b412df8a26a9d2e4fc144` |
| `crates/outcomes-cli/src/bin/hyperai-ui.rs` | `04da2c5f50f01c7a8b95bb4914725d0e9253018b55a9bdf520449df04f68be5d` |
| `target/debug/hyperai-ui` | `6793f917d6b1d448aab6b285afb110459ccb714d7e37dd5f0f6e09287693214a` |

Cleanup confirmed that the runner-owned service and Chrome processes exited,
its temporary home and browser profile were removed, and all runner-created
screenshot directories were deleted. The pre-existing service remained on
`127.0.0.1:4317` under PID 85094. No user data, external endpoint, provider,
seller, registry, or mutating workflow was used.

Assurance is `INTERNAL_PASS` for this same-host headless browser slice only.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Screen-reader and physical-keyboard acceptance, populated-state coverage,
contrast, browser/platform parity, clean second-environment reproduction,
independent review, and release or production authorization remain open.

Maximum claim: current-source, same-host headless Chrome layout, route, and
basic control-name checks for the fixed empty synthetic shell at the recorded
viewports. This is not OS screen-reader or physical keyboard acceptance,
contrast verification, populated-state coverage, cross-browser or
cross-platform qualification, policy parity, clean reproduction, independent
review, release readiness, or production authorization. Reviewer status is
`NOT_REQUESTED`; reproducibility remains `NOT_REPRODUCED` until another
environment runs the exact packet. No commit, PR, public release, external
circulation, or deployment is authorized.

### Current-source visual and reduced-motion rerun (2026-09-29)

The later invocation `corepack pnpm run verify:interface:browser --
--retain-screenshots` passed the same 33 route/viewport checks on the current
source. It also checked reduced-motion behavior across all 33 combinations
and 19,404 computed-style entries. All 17 observed requests were same-origin
loopback `GET`s. No unnamed enabled controls, horizontal overflow, runtime
errors, or console errors were found. The desktop Overview, mobile Deals, and
720x500 Data screenshots were visually inspected; the retained screenshots
and directory were then removed. The local service and browser processes
stopped, port 4317 remained owned by PID 85094, and port 4318 had no listener.

Current source and build identities:

| File | SHA-256 |
| --- | --- |
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `a25c9d725d51e564a967cf4643bcbf64b91cabf75bc9e3db03f9d0b0bc1a1cc5` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `scripts/verify-interface-browser.mjs` | `16f3e458754cc452776df35882973108c0a9840b53f3dda81bf032464316f42a` |
| `target/debug/hyperai-ui` | `35c4cd46a1df71ee791a592e14c2d12a17eb1d4679189af626fc9997cdc0ac4f` |

The inspected screenshot digests were desktop Overview
`33a08a40fab7c633520f693ae783291942863009c8375c132ca93fba9319a9e7`, mobile
Deals `4b2135dbecb461079ef6438de61bdc02fb36f97a378e73124a312827aafb2bbb`,
and 720x500 Data
`7d2e929397c9f0729cfb91c860f73cf1ea46b0cb684a01efea5657114015e6e0`.

Assurance remains `INTERNAL_PASS` for same-host checks of the empty local
shell. The rerun does not establish populated-workflow usability, screen-reader
or physical-keyboard acceptance, cross-browser/platform behavior,
client-policy parity, clean reproduction, independent review, or release
readiness. Reviewer status remains `NOT_REQUESTED`; reproducibility remains
`NOT_REPRODUCED`.
