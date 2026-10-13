# HyperAI Interface Reduced-Motion Qualification Packet

State slice: `hyperai.bazaar.interface.reduced_motion_routes.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md),
[headless browser layout packet](compute-bazaar-interface-browser-packet.md),
and [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

The interface already has a CSS `prefers-reduced-motion` rule, but the
headless browser runner does not emulate that preference and the rule does not
clear transition or animation delays. This continuation adds explicit delay
handling and browser evidence for the fixed empty shell. It does not change
routes, domain actions, service APIs, or source connections.

## Exact files

- `apps/hyperai-interface/app.css`: extend only the existing reduced-motion
  media rule to set transition and animation delays to zero.
- `scripts/verify-interface-browser.mjs`: add one CDP-emulated
  `prefers-reduced-motion: reduce` pass across all fixed routes and viewports.
- `docs/compute-bazaar-interface-reduced-motion-packet.md`: this scope and
  evidence record.
- `docs/state-slices.md`: authorization and final evidence.
- `docs/compute-bazaar-rust-port-subplan.md`: summarize this local qualification
  while retaining the remaining accessibility and platform gates.

## Allowed effects and controls

- Build `hyperai-ui` with the pinned toolchain and locked offline dependencies.
- Run one interface service on a newly allocated IPv4 loopback port with a new
  empty synthetic home. Never connect to the existing service on port 4317 or
  inspect its home.
- Launch headless Chrome with a fresh temporary profile, DevTools bound to
  IPv4 loopback, background networking disabled, and non-local name resolution
  refused. Keep the existing same-origin `GET`/`HEAD` request guard enabled.
- Emulate `prefers-reduced-motion: reduce` through CDP. Visit only Overview,
  Market, Deals, Data, Inference, Fleet, Eval, Trade, Evidence, Agents & Access,
  and Activity at 1440x1000, 720x500, and 390x844 CSS pixels.
- Do not press workflow, import, save, approval, evaluation, Trade, agent, or
  service-action controls. Inspect computed style only for rendered elements
  and their `::before`/`::after` pseudo-elements.
- Remove runner-owned processes, temporary home, Chrome profile, and logs.
  Preserve the existing listener on port 4317.

No external network, provider, seller, registry, remote worker, user payload,
payment, chain, publication, deployment, or persistent synthetic record is in
scope. Stop before any unexpected API write, external request, non-loopback
listener/socket, service collision, or unowned process.

## Proof gates and claim ceiling

- Confirm the emulated media query reports `prefers-reduced-motion: reduce`.
- For every rendered element and pseudo-element on every route and viewport,
  confirm transition and animation durations are no greater than 0.01 ms,
  their delays are zero, and computed scroll behavior is `auto`.
- Confirm all eleven route headings, active navigation, visible heading focus,
  accessible control names, and no-overflow behavior still pass under the
  emulated preference.
- Confirm all browser page requests remain same-origin loopback and use only
  `GET`/`HEAD`; record zero browser/runtime errors.
- Remove the runner's service, Chrome, profile, temporary home, and logs; verify
  the pre-existing service on port 4317 remains unchanged.
- Run the optional browser audit, root `corepack pnpm run lint`, pinned Rust
  formatting, import inventory, and scoped documentation/link/whitespace
  checks. Report any pre-existing import mismatch separately.

Maximum claim: same-host headless Chrome evidence for the fixed empty
HyperAI-authored interface shell under Chromium's emulated reduced-motion
preference at the recorded routes and viewports. This does not establish
physical OS preference handling, screen-reader behavior, populated-state
accessibility, cross-browser/platform behavior, client-policy parity, clean
second-environment reproduction, independent review, release readiness, or
production authorization. Reviewer status is `NOT_REQUESTED`.

## Results (2026-09-29)

The exact worktree passed `corepack pnpm run verify:interface:browser` on
macOS 26.6.1 arm64 with Google Chrome 154.0.8037.58, Node.js 22.23.2, pnpm
11.7.0, and pinned Rust 1.93.1. The runner built `hyperai-ui` with locked
offline dependencies, started the service and DevTools on ephemeral IPv4
loopback ports, and used a fresh synthetic home and Chrome profile.

All 33 reduced-motion route/viewport cases passed over the eleven workspaces
at 1440x1000, 720x500, and 390x844 CSS pixels. Chromium reported
`prefers-reduced-motion: reduce` for every case. The runner inspected 19,404
element/pseudo-element style entries. The maximum transition and animation
durations were 0.00001 seconds (0.01 ms); maximum delays were zero; and
computed scroll behavior was `auto`. All 33 normal-mode route checks also
passed with the same single headings, active navigation, visible heading
focus, accessible control names, and no document overflow. The browser
observed 17 same-origin `GET` requests and no runtime or console errors.

`corepack pnpm run lint` passed. Root `lint:fast`, `verify:contracts`, and
`verify:full` completed successfully, including JavaScript checks, strict Rust
Clippy, contract tests, and the full workspace test suite. Opt-in PostgreSQL,
Docker, and staged-binary tests were ignored because their configured local
services/images/binaries were not selected. `python3 scripts/verify_imports.py`
passed Simple Compute Market and HyperIntel; it reported the existing JustAI
177-file inventory mismatch at revision
`9a0e56536b968cbb2cb7173e142068b53c4f9fe3`. That snapshot was preserved.

The runner-owned service and Chrome processes exited. Its disposable home,
Chrome profile, logs, and runtime directory were removed. The pre-existing
service remained on `127.0.0.1:4317` under PID 85094. No screenshot or persistent
synthetic record was created.

Source SHA-256 values:

| File | SHA-256 |
| --- | --- |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `scripts/verify-interface-browser.mjs` | `16f3e458754cc452776df35882973108c0a9840b53f3dda81bf032464316f42a` |

Assurance is `INTERNAL_PASS` for this same-host headless Chrome slice only.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Physical OS preference handling, screen-reader and physical-keyboard
acceptance, populated-state accessibility, cross-browser/platform behavior,
clean second-environment reproduction, independent review, and release or
production authorization remain open. There is no Bazaar code/data reuse or
external integration in this slice.
