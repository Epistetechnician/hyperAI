# HyperAI Market Conflict Accessible-Name Packet

Status: closed `INTERNAL_PASS` for the exact local synthetic path.

State slice: `hyperai.bazaar.interface.market_conflict_accessible_name.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust port subplan](compute-bazaar-rust-port-subplan.md). Predecessor:
[focus-restoration packet](compute-bazaar-market-conflict-focus-restore-packet.md).

## Goal

The focus-restoration browser run confirmed the stale quote error status is
visible, focused, and visibly focused, but Chromium computed an empty
accessible name for its `role="status"` node. Add an explicit accessible name
matching the existing fixed visible message. Keep the error body private and
the no-action boundary unchanged.

WAI-ARIA 1.2 permits author naming for `status` and gives it implicit polite,
atomic live-region behavior ([status role](https://www.w3.org/TR/wai-aria/#status)).
The explicit label will match the already displayed, normalized UI message.

## Exact scope

Writable application files:

- `apps/hyperai-interface/market-supply-comparison.js`: set the status
  accessible name from the already normalized and escaped UI error string.
- `apps/hyperai-interface/market-supply-comparison.test.mjs`: assert the
  explicit accessible name matches the displayed fixed recovery text.

Writable evidence files: this packet, `docs/state-slices.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`.

Read-only: application controller, `scripts/verify-interface-browser.mjs`,
service/API/market code, package manifests, lockfiles, imported workspaces,
fixtures, and all source repositories. The existing browser runner already
asserts the exact computed status role/name, focus, `:focus-visible`, visible
outline, visible message, and no approval/completion UI.

## Allowed effects and stop conditions

Run focused interface tests, root `lint:fast`, root `verify:contracts`,
`python3 scripts/verify_imports.py`, JavaScript syntax, and diff checks. Then
run exactly once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:stale-quote-browser
```

Use the existing local synthetic runner with a fresh disposable service home
and Chrome profile, kernel-assigned IPv4 loopback ports, and same-origin
requests only. Allow the comparison read and exactly one stale
`POST /api/v1/market/supply-quote-preview`. The runner retains only validated
HTTP status and error code from a response clone. Approval, completion, all
other writes, external origins, providers, models, Fleet, wallets, chains, and
Bazaar source/data are closed.

Freeze the source hashes, complete Git-status byte count/digest, runtime
versions, and exact command before the browser run. If any input drifts or a
proof gate or cleanup check fails, close this packet without retry.

## Proof gates and claim ceiling

- Unit tests assert the status accessible label equals its visible text.
- The browser observes HTTP 409 and
  `MARKET_SUPPLY_COMPARISON_CHANGED`, with the exact fixed recovery text as
  both visible text and Chromium's computed accessible name for role `status`.
- Focus remains on the status, `:focus-visible` is true, and a visible outline
  is computed; the selected route is Market and quote-preview/approval/
  completion UI is absent.
- Aggregate requests contain exactly one comparison GET and one stale quote
  preview POST, with zero approval, completion, or other writes; the existing
  route, viewport, reduced-motion, browser-error, and cleanup checks pass.

Maximum claim: `INTERNAL_PASS` for this same-host synthetic accessible-name
path only. It does not qualify screen readers, user comprehension, other
browsers, platform/client accessibility, live supply, seller intent,
execution, payment, source rights, independent review, Phase 2 acceptance, or
production readiness. Reviewer status starts `NOT_REQUESTED`; reproduction
starts `NOT_REPRODUCED`. Custody is local source, fixed synthetic input, and
disposable browser/service state. Phase 2 remains `NOT_ASSIGNED`; live Fleet,
paid Eval, financial Trade, external actions, settlement, publication, and
deployment remain closed. Publish boundary: local source/docs only; no
commit, PR, release, or deployment.

## Execution record

Pre-run freeze: `2026-10-02 00:16:05 UTC`. Focused tests, JavaScript syntax,
and diff hygiene passed. Root `lint:fast` and `verify:contracts` passed (one
configured database-backed contract remained ignored). Import verification
passed for all three pinned source workspaces.

| Frozen input | Value |
|---|---|
| `apps/hyperai-interface/market-supply-comparison.js` | `0042f7eb6bf11447fc1a29317741edb98b489f1e9320b37d859a7c2571a29685` |
| `apps/hyperai-interface/market-supply-comparison.test.mjs` | `6e593ea37ca503491b5c9213feb2b0e68b6a6e7de1005c3fbb500f51984d5b4f` |
| Read-only `scripts/verify-interface-browser.mjs` | `d2e50c8f9f44fbc1afbc1f16a0a9f40c0224ace51e7488ae3ec9317f252f5f7e` |
| Read-only `apps/hyperai-interface/error-envelope.js` | `16bd46b87e8ba899779f7c51dc9a477feebcb16277fcac3540579cc4cea21874` |
| Read-only `package.json` | `0884806933b247d01ebce726b08536c5589cb89f3d50d61833494dc52a0386a0` |
| Full Git status | 2,674 lines / 230,075 bytes, SHA-256 `a27fd7c9f81918118b14db992f3fa4357e75b61c4b87b460e07c871c6f421935` |
| Runtime | Node.js `v22.23.2`; pnpm `11.7.0`; Rust `1.93.1`; Google Chrome `154.0.8037.92` |
| Browser command | `CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:stale-quote-browser` |

The frozen source set was used for the one permitted browser command. No
source edits occurred after the freeze.

Outcome: assurance `INTERNAL_PASS`; reviewer `NOT_REQUESTED`; reproducibility
`NOT_REPRODUCED`. The renderer/test hashes remained unchanged, and Git status
remained 2,674 lines / 230,075 bytes, SHA-256
`a27fd7c9f81918118b14db992f3fa4357e75b61c4b87b460e07c871c6f421935`.

The one run passed on Google Chrome `154.0.8037.92`. Chromium exposed the exact
fixed recovery copy as the computed name of role `status`; the visible text
matched, focus remained on the status, `:focus-visible` was true, and a solid
3px outline was computed. Market remained selected with quote-preview,
approval, and completion UI absent. The request report contained one
`GET /api/v1/market/supply-comparison` (200), one stale
`POST /api/v1/market/supply-quote-preview` (409,
`MARKET_SUPPLY_COMPARISON_CHANGED`), zero approval/completion requests, and
zero other writes. The run reported 33 route checks and passed reduced-motion
checks across the three viewports. Cleanup was conclusive: the browser process
group exited, the service exited on SIGTERM, both ephemeral listeners closed,
and temporary browser/service state was removed.

Data and artifact custody stayed with local source, fixed synthetic inputs,
and disposable local service/browser state. No Bazaar source/data, external
service, or real user payload was accessed. Screen readers, user comprehension,
other browsers/platforms/clients, live supply, execution, payment, source
rights, independent review, Phase 2, and production remain unqualified. The
next authority gates are source/data rights before reuse, separate
native-platform qualification, and distinct approval for live providers,
paid work, settlement, publication, or deployment.
