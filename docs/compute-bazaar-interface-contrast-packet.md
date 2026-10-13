# HyperAI Interface Solid-Surface Text Contrast Packet

State slice: `hyperai.bazaar.interface.solid_text_contrast.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust port subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

The interface plan names contrast review as an open design acceptance item.
This slice measures rendered text contrast for the fixed, empty HyperAI shell
on its eleven routes, then permits narrow foreground-color corrections in
`app.css` for text on computed solid-color backgrounds. The thresholds follow
WCAG 2.2 Success Criterion 1.4.3: 4.5:1 for ordinary text and 3:1 for large
text ([W3C specification](https://www.w3.org/TR/wcag/#contrast-minimum)).

This is not a general accessibility certification. It does not measure text
over gradients, images, or other complex backgrounds; non-text component or
focus-indicator contrast; inactive controls; hover states; error/populated
states; assistive-technology behavior; or link differentiation by color.

## Exact files

- `apps/hyperai-interface/app.css`: if the audit finds failures, adjust only
  foreground colors of text on solid-color surfaces. Do not change layout,
  type scale, component state, or domain text in this slice.
- `scripts/verify-interface-browser.mjs`: add an opt-in contrast mode that
  audits rendered direct text nodes using computed styles and the WCAG sRGB
  relative-luminance formula. Preserve the normal route/layout audit behavior.
- `package.json`: expose the optional contrast mode without adding Chrome as a
  requirement of root lint or introducing dependencies.
- `docs/compute-bazaar-interface-contrast-packet.md`: exact method, results,
  digests, and limitations.
- `docs/state-slices.md`, `docs/bazaar-hyperai-interface-plan.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`: scope and bounded result.

## Allowed effects and controls

- Build `hyperai-ui` with the pinned Rust toolchain and locked offline
  dependencies. Run it once with a new empty synthetic home on an ephemeral
  IPv4 loopback port. Never connect to or inspect the existing service on
  `127.0.0.1:4317` or its home.
- Launch the installed Google Chrome binary headlessly with a fresh temporary
  profile and DevTools bound to IPv4 loopback. Keep the existing network
  restrictions: same-origin loopback requests and the authored inline SVG
  favicon only; page methods must be `GET` or `HEAD`.
- Visit the eleven fixed routes at 1440x1000, 720x500, and 390x844 CSS pixels.
  Do not invoke workflow, import, save, approval, evaluation, Trade, agent,
  provider, or service-action controls.
- Inspect visible direct text nodes on solid backgrounds. Composite computed
  RGBA backgrounds through their ancestors over the page canvas. Classify text
  with a computed background image, non-normal blend mode, or inherited
  opacity as `review_required`; do not count it as an automated pass. Exclude
  hidden, `aria-hidden`, disabled-control, and logo text from the text gate.
- Use 4.5:1 for text below 24 CSS pixels unless bold at or above 18.67 CSS
  pixels, and 3:1 for large text. Compare the unrounded ratio to the threshold.
  Report sample counts, minimum measured ratio, failures, and skipped
  review-required samples.
- Representative screenshots may be retained temporarily for visual
  inspection, then deleted. Stop on an unexpected endpoint, mutation, process,
  or listener. Remove only runner-owned service/browser processes and files.

No source network, external application, user payload, registry, seller,
provider, remote worker, payment, chain, publication, or deployment is in
scope. No dependency or copied Bazaar source, schema, fixture, dataset, or
asset may be introduced.

## Proof gates and claim ceiling

- Confirm both listeners use IPv4 loopback and requests stay within the
  permitted read-only request boundary.
- For all 33 route/viewport combinations, verify the normal route, heading,
  focus, and no-overflow checks still pass, then measure every eligible
  visible text sample on an audited solid background.
- Pass only when every eligible ordinary-text ratio is at least 4.5:1 and
  every eligible large-text ratio is at least 3:1; list unmeasured complex
  backgrounds separately.
- Inspect the representative screenshots, remove all temporary state, and
  verify that the existing port 4317 listener is unchanged.
- Run the optional contrast audit, root `corepack pnpm run lint`, pinned Rust
  formatting, local documentation-link checks, and whitespace checks. Report
  the preserved JustAI import-inventory mismatch separately.

## Results (2026-09-29)

The expected red baseline found 944 failing sample occurrences across the 33
route/viewport combinations, with a minimum ratio of 2.0687:1 on the breadcrumb
divider and current route label. Foreground-only corrections in `app.css`
raised the measured solid-surface text to the passing range while retaining the
existing layout and type scale. Representative desktop, zoom-equivalent, and
mobile screenshots were visually inspected and deleted.

Final assurance is `INTERNAL_PASS` for the exact same-host empty-shell browser
audit. Google Chrome 154.0.8037.58 visited all 11 routes at 1440x1000,
720x500, and 390x844 CSS pixels. All 33 route/layout/focus checks passed; the
browser observed 17 `GET` requests, no nonlocal request, no state-changing
method, no console/runtime error, and no horizontal overflow. The text gate
checked 2,350 visible direct-text samples on computed solid-color backgrounds:
zero failed the 4.5:1 ordinary-text or 3:1 large-text threshold. The lowest
measured ratio was 4.6663:1 for source-reference code text against white. A
further 201 visible samples were classified `review_required` on
gradient/image backgrounds; they are not automated passes.

The exact source and built UI digests were:

- `apps/hyperai-interface/app.css`: `2b56aec78787e8cfc6a29dda5133a6df6ce9d5cf19abc0989630829e4e3f5b72`
- `scripts/verify-interface-browser.mjs`: `9dad2136f7491f42600c9cd6153a7db4c0f2bfb26a05529bd84c036c8370a25a`
- `package.json`: `40ae416cc52f626aed986b45d5246768ae8b7e78dad8ca2d799fd25470e1d999`
- `target/debug/hyperai-ui`: `5c32e893bf49ace6aec3b6830fab89cf88ab1febc13dda20d042510eb7eaa318`

Local root `corepack pnpm run lint` passed, including interface and installer
checks, pinned Rust formatting and Clippy, workspace contract tests, and the
full all-features Rust test suite. `git diff --check` passed. The import scan
passed the pinned Simple Compute Market and HyperIntel snapshots and retained
the known JustAI inventory mismatch (`products/justai/pnpm-lock.yaml`). The
browser used a disposable synthetic home, profile, and temporary screenshots;
all runner-owned temporary state and processes were removed. The pre-existing
service on `127.0.0.1:4317` was preserved. Reviewer status is
`NOT_REQUESTED`; reproducibility status is `NOT_REPRODUCED`. Data and artifact
custody is limited to checked-out source, the locally built UI binary, and
temporary local screenshots that were deleted after inspection.

The run environment was macOS 26.6.1 arm64 with pinned Rust/Cargo 1.93.1.

The result does not cover complex-background text, non-text contrast, hover or
inactive-control states, populated/error states, assistive technology,
physical keyboard operation, or other browsers/platforms. The next blocked
gate for a broader accessibility claim is manual review of complex-background
and non-text states, followed by the separately scoped keyboard, screen-reader,
and platform checks. No independent review or production authority is implied.

Maximum claim: same-host headless Chrome measurements for visible direct text
on computed solid-color backgrounds in the fixed empty shell, at the recorded
routes and viewports. This is not full WCAG conformance, a manual contrast
review of gradients or images, non-text contrast acceptance, populated-state
coverage, browser/platform parity, clean reproduction, independent review,
release readiness, or production authorization. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. No commit, PR, public
release, external circulation, or deployment is authorized.

### Current-source contrast rerun (2026-09-29)

`corepack pnpm run verify:interface:contrast` passed all 33 route/viewport
checks on the current source. Reduced-motion checks passed for 33 combinations
and 19,404 computed-style entries. The browser observed 28 same-origin loopback
`GET` requests, with no horizontal overflow, unnamed enabled control, runtime
error, or console error. On computed solid-color backgrounds, 2,350 text
samples had zero threshold failures; the minimum was 4.6663:1. Another 201
samples over gradients or images remain `review_required` and are not counted
as passes. The complete audit status is `passed-with-review-required`.

Current source and build identities:

| File | SHA-256 |
| --- | --- |
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `a25c9d725d51e564a967cf4643bcbf64b91cabf75bc9e3db03f9d0b0bc1a1cc5` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `scripts/verify-interface-browser.mjs` | `16f3e458754cc452776df35882973108c0a9840b53f3dda81bf032464316f42a` |
| `target/debug/hyperai-ui` | `35c4cd46a1df71ee791a592e14c2d12a17eb1d4679189af626fc9997cdc0ac4f` |

This current-source rerun does not expand the claim ceiling. Complex-background
and non-text contrast, populated and error states, assistive technology,
physical keyboard operation, browser/platform parity, clean reproduction,
independent review, and release authorization remain open. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
