# HyperAI Data Keyboard View Browser Packet

Status: completed test-harness-only local browser slice; `INTERNAL_PASS` for
the exact synthetic workflow below.

State slice: [`hyperai.bazaar.interface.data_keyboard_view_browser.v1`](state-slices.md#hyperai-bazaar-interface-data_keyboard_view_browserv1).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve every pre-existing path.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose

Qualify the populated Data view correction path after setup has produced a
synthetic query result and a saved synthetic model. Browser keyboard events
must edit an invalid column selection, submit the preview, preserve the field
and focus the accessible failure status, correct the field, review a
digest-bound preview, and explicitly save the view.

This is a local interaction qualification only. It changes no interface,
application, or service behavior and does not establish complete accessibility.

## Fixed workflow

1. Build the pinned `hyperai-ui` binary with locked offline dependencies.
2. Start the runner-owned service and headless Chrome on ephemeral IPv4
   loopback ports, with fresh temporary home and browser-profile directories.
3. Seed the fixed HyperAI synthetic sample, run the fixed three-column query,
   and create/save the fixed synthetic model using the existing runner setup
   helpers.
4. After the Data view form renders, navigate to its columns field with Tab,
   replace its value with `not a column`, navigate to Preview view by Tab, and
   activate it with Enter.
5. Require the failure status to have `role=status`, `aria-live=polite`,
   `aria-atomic=true`, and focus. Require the invalid value and view name to
   remain editable and no confirmation control to appear.
6. Use Tab to return to the columns field, replace it with
   `gpu_model, asking_usd_micros_per_hour`, Tab to Preview view, and activate
   with Enter.
7. Require a valid digest-bound preview and visible Confirm view save control.
   Reach that control with Tab, activate with Enter, and require a saved local
   view whose definition digest appears in the inventory.
8. Require only the POST routes/statuses below; clean the temporary service,
   home, Chrome profile, browser process, and output.

The field edit, failed and corrected preview submissions, and save
confirmation in steps 4–7 must use CDP keyboard input events. The runner may
use its existing controlled setup helpers before the view form is rendered.

## Request boundary

Only same-origin requests to the runner-owned IPv4 loopback service are
allowed. The exact expected Data POSTs are:

| Route | Count | Statuses |
|---|---:|---|
| `/api/v1/data/synthetic/seed` | 1 | `200` |
| `/api/v1/data/query` | 1 | `200` |
| `/api/v1/data/models/preview` | 1 | `200` |
| `/api/v1/data/models/save` | 1 | `200` |
| `/api/v1/data/views/preview` | 2 | `422`, `200` |
| `/api/v1/data/views/save` | 1 | `200` |

No other POST method or path is allowed. The expected `422` is the deliberate
invalid view columns preview. Browser requests to non-loopback destinations,
service collisions, changed existing listeners, unexpected statuses, or
ambiguous save outcomes stop the run.

## Exact files and allowed effects

- `scripts/verify-interface-browser.mjs`: add an opt-in workflow, keyboard
  event helpers, exact route/status assertions, and result assertions.
- `package.json`: add `verify:interface:data-keyboard`; keep root lint unchanged
  and add no dependency.
- This packet: freeze the scope and retain the result.
- `docs/compute-bazaar-phase-2-data-packet.md`: append this result and limits.
- `docs/compute-bazaar-rust-port-subplan.md`: update only the Phase 2 browser
  accessibility evidence.
- `docs/bazaar-hyperai-interface-plan.md`: update only the matching Data
  keyboard evidence and claim limit.
- `docs/state-slices.md`: record the authorization and exact result.

Allowed effects are limited to building the pinned local binary, starting the
isolated loopback service and Chrome profile, creating the fixed synthetic
sample, issuing the requests in the table, saving one model and one view in the
disposable home, and creating/removing temporary local output. Do not contact
the existing service on `127.0.0.1:4317`.

## Non-goals and claim ceiling

No product code change, external or non-synthetic data, upstream Bazaar/SCM/
HyperIntel/JustAI access, provider or seller call, federation, credentials,
model execution, Fleet operation, paid evaluation, payment, escrow, chain
write, publication, release, commit, or deployment is included.

`INTERNAL_PASS` may cover only this same-host headless-Chrome keyboard path
over the fixed synthetic sample and disposable home. It does not establish
complete Data-workspace keyboard usability, OS screen-reader operation,
browser/platform parity, overall Phase 2 acceptance, source rights, clean
second-host reproduction, independent review, or production readiness.
Reviewer status starts `NOT_REQUESTED`; reproducibility starts
`NOT_REPRODUCED`.

## Proof and stop conditions

Require the invalid field value and view name to remain after rejection; the
accessible status to receive focus; no confirmation before a successful
preview; a valid digest before confirmation; an explicit keyboard save; the
saved definition digest in the inventory; exact request counts and statuses;
cleanup of every runner-owned process and temporary path; and an unchanged
pre-existing listener. Run runner syntax, root lint, import-preservation, and
scoped documentation checks.

Stop on any lost focus or value, unexpected POST or nonlocal request, route or
status mismatch, stale or missing preview digest, ambiguous write, service
collision, or changed existing listener. Do not broaden the allowlist or edit
application behavior under this slice. Publish boundary is the seven files
above and transient local output; no commit or external circulation is
authorized.

## Execution record

On 2026-09-30, `CARGO_NET_OFFLINE=true corepack pnpm run
verify:interface:data-keyboard` passed on macOS 26.6.1 arm64 with Rust/Cargo
1.93.1, Node.js 22.23.2, pnpm 11.7.0, and Chrome 154.0.8037.58. It made 38
browser requests and exactly the seven frozen Data POSTs. The deliberate
invalid preview returned 422; the corrected preview returned 200; all other
Data POSTs returned their frozen 200 status. Keyboard input retained the invalid
value, focused the accessible failure status, previewed the corrected columns,
and saved the view with its definition digest in the inventory. The
prerequisite saved model digest was
`3b5ab33d8bc811ca964a1ac587d4daa9e97cde359b5ae01040691ea6116e7458`; the view
preview digest was
`90ff5e09bf2ea0d523357f5af4ff387c24dd04480a9d62476ad46995ccf6d368`; and the
saved view digest was
`a5fdf500a1dd46fdba06eaac4e4692d6e50fe1e402d5babffb36c48c25399926`.
The runner SHA-256 is
`50355928293c5ecc26ed7ce873bf35a66a529a0c70a4b63bf5f32279999da2a3`,
`package.json` SHA-256 is
`105ffcf27c4eda872611935fb8894d0db13859837b97e7286d9c624671e980a1`, and
the built `target/debug/hyperai-ui` SHA-256 is
`ff937fb648207e7f51cad29494faaf56d11bf12f32f949dd2757ba798ba4dd3e`.

`node --check scripts/verify-interface-browser.mjs`, package JSON parsing, and
`git diff --check` passed. `CARGO_NET_OFFLINE=true corepack pnpm run lint`
exited 0, including `lint:fast`, `verify:contracts`, and `verify:full`; declared
environment-gated tests remained ignored. Pinned Simple Compute Market and
HyperIntel imports passed. The preserved JustAI inventory still reports 177
files observed versus 176 recorded; no imported snapshot was changed. The
pre-existing listener on `127.0.0.1:4317` remained PID 85094. The disposable
home, browser profile, service, Chrome process, and temporary output were
cleaned.

Assurance is `INTERNAL_PASS` for this same-host headless-Chrome keyboard path
only. Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
This does not establish screen-reader or physical-keyboard acceptance,
cross-browser/platform behavior, full Data-workspace keyboard usability,
overall Phase 2 acceptance, source rights, independent review, or production
readiness. Phase 2 remains `NOT_ASSIGNED`.
