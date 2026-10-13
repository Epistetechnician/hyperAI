# HyperAI compact viewport navigation packet

State slice: `hyperai.bazaar.interface.compact_viewport_navigation.v1`.

Status: local browser-presentation implementation completed under
`global-experiment-assurance-lifecycle-v1`.

## Objective

Make the existing HyperAI navigation usable when the browser is shorter than
the full navigation stack and when mobile browser chrome changes the visible
viewport. Keep the existing visual language, page routes, navigation labels,
content, connection state, and authority behavior intact.

## Task list

1. Preserve the baseline and worktree; do not change runtime or content.
2. Give the shell and navigation a dynamic viewport height with a legacy
   viewport fallback and safe-area spacing.
3. Let the fixed mobile navigation drawer scroll independently when its
   contents exceed the visible height.
4. Review the exact diff, run whitespace validation, and record the bounded
   result and remaining browser qualification gate.

## Baseline

Repository: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the preserved dirty and untracked worktree. The following hashes bind the
source and records inspected before this packet:

- `apps/hyperai-interface/app.css`:
  `769feaaf75333e99b3324fc683b6fe5967ebcb38892f870a8dc3499b478dddc3`
- `docs/bazaar-hyperai-interface-plan.md`:
  `ca4725ba0eff45d58f1e9c83966da29a23c118ef993de63804dfecbff08a7e84`
- `docs/state-slices.md`:
  `0b7cdabe1f0ec1127d57233682a8d303f912205a7541eaa77b91d80603a5c31a`

The observed CSS sets the shell to `100vh`, forces the sidebar to a 680-pixel
minimum height, and gives the mobile drawer no independent overflow region.
The drawer already has keyboard-aware open/close state in `app.js`; that code
is read-only for this slice.

## Exact scope

Writable files:

- `apps/hyperai-interface/app.css`
- `docs/compute-bazaar-interface-compact-viewport-navigation-packet.md`
- `docs/state-slices.md`
- `docs/bazaar-hyperai-interface-plan.md`

Allowed effects: CSS-only viewport and navigation layout refinements for the
existing shell. Support `100dvh` with a `100vh` fallback, avoid forcing a
minimum height larger than the viewport, provide safe-area-aware drawer
padding, and permit the sidebar contents to scroll inside its bounded height.
Document the exact behavior and limits in the product plan and state-slice
record.

Non-goals: no HTML, JavaScript, route, data, schema, API, service, storage,
navigation-label, interaction, policy, or domain changes; no new package or
asset; no upstream Bazaar source, data, fixture, schema, or asset import; no
live connector, provider, seller, agent, custody, settlement, external write,
publication, commit, PR, or deployment.

## Proof gates

- Confirm the stylesheet remains the only product source changed.
- Confirm the shell and drawer use a supported viewport unit, the fixed drawer
  can scroll independently, and safe-area padding does not remove the legacy
  viewport fallback.
- Review the final scoped diff and run `git diff --check`.
- Do not infer browser rendering, mobile Safari behavior, keyboard or screen
  reader accessibility, or cross-platform support from CSS inspection.

Claim ceiling: at most `INTERNAL_PASS` for the scoped CSS/source and whitespace
review. Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`.
Data and artifact custody: no external or user data; changes remain in this
local worktree. Known limitations: no browser session, physical mobile device,
screen reader, or alternate platform is qualified by this packet. Next gate:
browser/WebView qualification for compact height, safe areas, responsive
behavior, and assistive technology within the separately scoped interface
qualification work. Publish boundary: local source and Markdown only.

## Execution result (2026-10-01)

The shared shell and desktop/mobile sidebar now use `100dvh` after the
`100vh` fallback. The fixed minimum height was removed, the sidebar has
independent vertical scrolling with contained scroll chaining, and padding
honors the top, bottom, left, and right safe-area insets. Existing navigation
markup and `app.js` were not changed.

Scoped source inspection confirmed those declarations and the unchanged
reduced-motion rule. `git diff --check` and whitespace inspection of the
untracked scoped files emitted no diagnostics. No automated or browser tests
were run. Assurance is `INTERNAL_PASS` for the CSS/source and whitespace
review only; reviewer status is `NOT_REQUESTED`, and reproducibility is
`NOT_REPRODUCED`. Browser rendering, mobile device behavior, and assistive
technology remain unqualified.
