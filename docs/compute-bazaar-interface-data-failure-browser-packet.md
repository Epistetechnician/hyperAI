# HyperAI Data Failure and Stale-Preview Browser Packet

State slice: `hyperai.bazaar.interface.data_failure_states_browser.v1`.
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md),
[populated Data browser workflow](compute-bazaar-interface-data-browser-workflow-packet.md),
[interface plan](bazaar-hyperai-interface-plan.md), and
[Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

## Purpose

Add an opt-in, runner-only browser qualification for failed model and view
previews and for clearing preview confirmations after a query changes. It
uses only the existing HyperAI UI and local Data API with the fixed synthetic
snapshot. It does not change application behavior, Phase 2 contracts, or the
existing populated-workflow command.

## Exact files

- `scripts/verify-interface-browser.mjs`: add the isolated
  `--data-failure-workflow` and its fixed request/status assertions.
- `package.json`: add `verify:interface:data-failures`; add no dependency and
  do not include browser execution in root lint.
- `docs/compute-bazaar-interface-data-failure-browser-packet.md`: this packet
  and its execution evidence.
- `docs/compute-bazaar-phase-2-data-packet.md`: append only this browser
  qualification result and its limits.
- `docs/compute-bazaar-rust-port-subplan.md`: update only the Phase 2 local
  interface qualification summary.
- `docs/bazaar-hyperai-interface-plan.md`: update only the critical-path
  evidence for failed previews and stale-preview clearing.
- `docs/state-slices.md`: authorization and result record.

## Frozen browser workflow

1. Start the existing local interface binary on an ephemeral IPv4 loopback
   port with a new disposable HyperAI home and seed its fixed synthetic data.
2. Run the fixed valid query and confirm the result and source digest are
   present.
3. Save the exact synthetic JSON bytes in runner memory, append one byte to
   that file, and submit a valid model preview. The local API must reject the
   changed source with HTTP 409; the UI must show and focus an accessible
   failure status, retain the model name, and show no confirmation control.
   Restore the exact original bytes in a `finally` path before continuing.
4. Preview a valid model. Run a different valid query before confirming it;
   assert the old model confirmation and preview status are cleared. Preview
   the model again and explicitly save it, checking its definition digest and
   inventory row.
5. Submit a view preview with the fixed malformed column `not a column`. The
   local API must reject it with HTTP 422; the UI must preserve the edited
   fields, focus an accessible failure status, and show no confirmation.
6. Preview a valid view. Run another valid query before confirming it; assert
   the view confirmation and preview status are cleared. Preview the valid
   view again and explicitly save it, checking its definition digest and
   inventory row.
7. Enforce the exact same-origin POST counts below; all other requests are
   same-origin loopback `GET` or `HEAD`, except the self-contained inline SVG.

| Route | Count | Expected result |
|---|---:|---|
| `/api/v1/data/synthetic/seed` | 1 | 200 |
| `/api/v1/data/query` | 3 | 200 each |
| `/api/v1/data/models/preview` | 3 | one 409, two 200 |
| `/api/v1/data/models/save` | 1 | 200 |
| `/api/v1/data/views/preview` | 3 | one 422, two 200 |
| `/api/v1/data/views/save` | 1 | 200 |

## Allowed effects and stop conditions

Build the pinned `hyperai-ui` binary with locked offline dependencies. Start
one runner-owned service and one headless Chrome instance with fresh
temporary home/profile directories and loopback listeners. The service may
write only the fixed synthetic sample and local saved model/view definitions
inside that temporary home. The runner may append one byte to the runner-owned
synthetic JSON file to cause a real integrity conflict, but it must retain the
original bytes in memory and restore them in a `finally` path before any
subsequent workflow action. The test fails if restoration fails.

Stop on an unexpected method, path, status, external request, browser error,
listener collision, unrestored fixture, missing failure focus, retained stale
confirmation, or partial/ambiguous save. Do not widen the allowlist to make a
failure pass and do not edit UI or service behavior under this slice. If the
observed contract differs from the packet, record the result and define a
separate slice before changing product code.

No existing local service/home (including port `4317`), upstream source,
non-synthetic data, SCM capture, HyperIntel, JustAI, provider, seller, remote
worker, credential, model execution, Fleet action, paid evaluation, payment,
escrow, chain write, federation, publication, release, commit, or deployment is
in scope.

## Proof gates and claim ceiling

The opt-in browser command must assert all six fixed POST route counts and the
exact status distribution; query changes must clear prior model/view preview
confirmations; both failed previews must preserve their editable fields, show
polite accessible error status, and focus that status; valid retries and
separate explicit model/view saves must complete with 64-character definition
digests. The runner must report no unexpected browser request, runtime error,
console error, or favicon fallback, and must clean up its service, browser,
home, profile, logs, and any retained screenshot after visual inspection.
The pre-existing listener on `127.0.0.1:4317` must remain unchanged.

Assurance can be `INTERNAL_PASS` only for the exact same-host headless-Chrome
run, local service, disposable home, fixed synthetic fixture, and checked UI
states. Reviewer status is `NOT_REQUESTED`; reproduction begins as
`NOT_REPRODUCED`. This does not establish screen-reader or physical-keyboard
acceptance, cross-browser/native-platform behavior, broad client-policy parity,
source rights, full Phase 2 acceptance, clean second-host reproduction,
independent review, release readiness, or production authorization. Phase 2
remains `NOT_ASSIGNED`. No commit, pull request, external circulation, or
deployment is authorized.

## Execution result (2026-09-30)

The command `CARGO_NET_OFFLINE=true corepack pnpm run
verify:interface:data-failures -- --retain-screenshots` passed on macOS 26.6.1
arm64 with Rust/Cargo 1.93.1, Node.js 22.23.2, pnpm 11.7.0, and Google Chrome
154.0.8037.58. The runner built the pinned `hyperai-ui` binary, started a fresh
temporary home and service on IPv4 loopback, and exercised all 33 route and
viewport checks plus 33 reduced-motion checks. It observed 43 browser
requests, including exactly 12 Data POSTs: one synthetic seed, three queries,
three model previews, one model save, three view previews, and one view save.
The response distribution matched the packet: model previews returned 409,
200, 200; view previews returned 422, 200, 200; all other Data routes
returned 200. No unexpected request or browser runtime/console error occurred.

The changed synthetic source bytes produced the expected model-preview
integrity conflict. The UI retained the model name, exposed a polite focused
failure status, and offered no confirmation. The runner restored the exact
original snapshot bytes before continuing. The malformed view column produced
the expected validation response while retaining the edited fields and focus.
For both forms, a new valid query removed the stale preview confirmation and
status; each definition then previewed and saved successfully with its digest
listed in the local inventory. The UI presents generic contract-error text;
the runner verifies exact HTTP codes from the response ledger.

One initial runner attempt incorrectly required the displayed text to contain
the HTTP status. It stopped after confirming the model failure state because
the UI intentionally omits that code. The assertion was corrected to use the
captured response status, and the complete rerun passed. No product code or
contract changed.

The visually inspected Data screenshot was removed with its temporary
directory. Runner-owned service, Chrome, home, and profile were cleaned up.
The existing `127.0.0.1:4317` listener remained owned by PID 85094 before and
after the run and was not contacted. The root
`CARGO_NET_OFFLINE=true corepack pnpm run lint` gate passed, including
`lint:fast`, `verify:contracts`, and `verify:full`; environment-gated Docker,
PostgreSQL, and staged-sidecar tests were ignored. `node --check` and package
JSON parsing passed. Simple Compute Market and HyperIntel import inventories
matched (2,189 and 99 files); the unchanged JustAI snapshot still reports 177
observed files versus 176 recorded and the import verifier exits 1. No source
snapshot changed.

SHA-256 identities for the executed browser bytes:

| File | SHA-256 |
|---|---|
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `scripts/verify-interface-browser.mjs` | `2c87e49a3ed6d1378d52ddd4504e4dcb19f3e239e69543bdd53cd927b37ee571` |
| `package.json` | `d5929a6cc9b516504ea43dc0ea4db7b5ddb9f8109556d83da49a352700ef23cc` |
| `target/debug/hyperai-ui` | `ff937fb648207e7f51cad29494faaf56d11bf12f32f949dd2757ba798ba4dd3e` |

The worktree is based at `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on
`main`; the implementation and packet remain uncommitted alongside existing
unrelated dirty work. Assurance is `INTERNAL_PASS` for this exact local
synthetic browser slice. Reviewer status is `NOT_REQUESTED` and reproducibility
is `NOT_REPRODUCED`. The next blocked gates are full keyboard and assistive
technology acceptance, cross-browser/platform behavior, broader client-policy
parity, clean second-host reproduction, source rights, independent review,
and overall Phase 2 qualification. Phase 2 remains `NOT_ASSIGNED`.
