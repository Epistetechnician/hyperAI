# HyperAI Data Full Keyboard Journey Packet

Status: the initial v1 attempt stopped at its SQL keyboard-edit assertion;
overall slice result is `INCONCLUSIVE`, with no acceptance claim.

State slice: `hyperai.bazaar.interface.data_full_keyboard_journey.v1`.
Parent: [Phase 2 Data contract packet](compute-bazaar-phase-2-data-packet.md).
Product direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Preserve every pre-existing path.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose

Qualify the complete local Data journey using keyboard input from the initial
workspace page through a digest-bound saved view. This extends the existing
focused keyboard correction check, which began after the Data route and its
model prerequisite had been prepared with browser-script actions.

This slice may fix only keyboard focus, keyboard-operable controls, accessible
labels/status, or visible focus treatment demonstrated to fail in the frozen
journey. It must preserve the existing API and data contracts. No new product
dependency is allowed.

## Frozen journey

1. Build the pinned `hyperai-ui` binary with locked offline dependencies. Start
   the runner-owned service on an ephemeral loopback port with a fresh
   temporary HyperAI home, and start headless Chrome with a fresh profile.
2. Use CDP keyboard events to tab to the Data navigation control and activate
   it with Enter. Use keyboard input to create the fixed HyperAI synthetic
   snapshot.
3. Tab to the SQL field, enter the fixed three-column query, reach the query
   submit button with Tab, and activate it with Enter. Require three rows from
   the expected snapshot and verify the result table region is named and
   reachable with keyboard focus.
4. Use Tab, text input, and Enter to preview and explicitly save a fixed
   synthetic query model. Require a digest-bound preview before the save
   control appears, then require the saved definition digest in the inventory.
5. Use keyboard input to name a view, select its presentation, and edit its
   columns. Submit one invalid column selection; require an accessible focused
   error, retained values, and no confirmation control. Correct the columns,
   preview again, and explicitly save by keyboard. Require a digest-bound
   preview and the saved view digest in the inventory.
6. Assert that no workflow action used DOM `.click()`, pointer input, or a
   direct API setup helper. Read-only DOM inspection is allowed. Permit only
   same-origin requests to the runner-owned IPv4 loopback service and the
   frozen API writes below. Clean the temporary service, home, browser
   profile, processes, and output.

The workflow must record which controls received keyboard focus, whether the
focused control matched `:focus-visible`, the action order, result row and
snapshot digests, saved model/view digests, API methods/statuses, browser
errors, and cleanup results. This synthetic CDP path does not prove physical
keyboard behavior or operation by an OS screen reader.

## Request boundary

The only permitted POSTs are:

| Route | Count | Statuses |
|---|---:|---|
| `/api/v1/data/synthetic/seed` | 1 | `200` |
| `/api/v1/data/query` | 1 | `200` |
| `/api/v1/data/models/preview` | 1 | `200` |
| `/api/v1/data/models/save` | 1 | `200` |
| `/api/v1/data/views/preview` | 2 | `422`, `200` |
| `/api/v1/data/views/save` | 1 | `200` |

GET and HEAD requests may target only the runner-owned loopback service. Reject
non-loopback requests, any other mutation, unexpected API status, process or
port collision, ambiguous save result, lost focus or field value, missing
accessible name/status, stale digest, or dirty runner-owned state. Do not
contact the existing service at `127.0.0.1:4317`.

## Exact files and allowed effects

- `apps/hyperai-interface/app.js` and `app.css`: only the minimum keyboard,
  label/status, and focus-treatment fixes exposed by this frozen journey; no
  API, storage, or data-contract changes.
- `scripts/verify-interface-browser.mjs`: add the opt-in full keyboard journey
  and exact keyboard, digest, route, status, and cleanup assertions.
- `package.json`: add `verify:interface:data-full-keyboard`; add no dependency
  and leave root lint composition unchanged.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record the freeze and observed
  result only.

Allowed runtime effects are building the pinned local binary, starting the
isolated service and Chrome profile, creating one fixed synthetic snapshot,
issuing the requests in the table, saving one model and one view in the
disposable home, and creating/removing temporary local output. The captured
source versions at freeze are:

| File | SHA-256 |
|---|---|
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `scripts/verify-interface-browser.mjs` | `50355928293c5ecc26ed7ce873bf35a66a529a0c70a4b63bf5f32279999da2a3` |
| `package.json` | `105ffcf27c4eda872611935fb8894d0db13859837b97e7286d9c624671e980a1` |

Toolchain freeze: Rust/Cargo `1.93.1`, Node.js `22.23.2`, pnpm `11.7.0`, and
Google Chrome `154.0.8037.58` on macOS arm64. Source build and package work
must remain offline. The pre-existing service PID 85094 on
`127.0.0.1:4317` is outside this slice and must remain untouched.

## Proof gates

- The route change, seed, query, model preview/save, failed/corrected view
  preview, and view save are all activated through keyboard events.
- Keyboard focus and `:focus-visible` are asserted for the main controls;
  the results region has a usable accessible name; form labels and status
  announcements survive failure and recovery.
- Exact API counts and statuses match the request table; no nonlocal request,
  runtime exception, console error, or unexpected browser log occurs.
- `node --check scripts/verify-interface-browser.mjs`,
  `CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:data-full-keyboard`,
  `CARGO_NET_OFFLINE=true corepack pnpm run lint`, pinned import checks, and
  scoped documentation hygiene pass.
- Every runner-owned process and temporary path is removed, and PID 85094
  remains unchanged. Record the final source digests, command output digest,
  tool versions, and literal result.

Stop on any failed gate and retain its literal result. Do not repeat an
ambiguous external write; this slice has no external write path.

## Non-goals and claim ceiling

No non-synthetic input, upstream source, external connector, browser network,
provider or seller call, model execution, Fleet action, paid evaluation,
payment, escrow, chain write, publication, commit, release, deployment, or
independent review is included. No Bazaar code or data is copied. The
maximum claim is `INTERNAL_PASS` for this exact same-host headless-Chrome CDP
keyboard path over the fixed synthetic sample. Reviewer status is
`NOT_REQUESTED`; reproduction status is `NOT_REPRODUCED`. This does not
establish hardware keyboard acceptance, full OS screen-reader operation,
cross-browser/native-platform parity, overall Phase 2 acceptance, source
rights, or production readiness.

Publish boundary is this packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-2-data-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`. No commit or external circulation is
authorized.

## Initial v1 execution result (2026-09-30): `INCONCLUSIVE`

The opt-in browser run reached the Data workspace, created the synthetic
snapshot, and focused the SQL textarea by keyboard. Its select-all helper did
not replace the textarea contents; the resulting value contained the original
query followed by the same query again, and the assertion stopped before query
submission, model/view writes, or any further workflow action. No query or
definition save was issued. This is a test-harness interaction failure and
does not establish a product keyboard defect or a Data API result. The failing
command was
`CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:data-full-keyboard`.

The runner's `finally` cleanup was checked after exit: no
`hyperai-interface-browser-*` temporary root or runner-owned service/Chrome
process remained, and the pre-existing PID 85094 listener on
`127.0.0.1:4317` remained unchanged. The run stopped on the first failure and
was not repeated with the same inputs. Any continuation uses a fresh named
state slice and packet.
