# HyperAI Market Conflict Recovery Copy Packet

Status: completed local synthetic UI and browser-harness continuation.

State slice: `hyperai.bazaar.interface.market_conflict_recovery_copy.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust port subplan](compute-bazaar-rust-port-subplan.md). Predecessor:
[stale supply quote browser packet](compute-bazaar-interface-stale-quote-browser-packet.md).

## Goal

Replace the generic message for the known
`MARKET_SUPPLY_COMPARISON_CHANGED` response with this fixed recovery message:

> This supply comparison is out of date. Reload this page to get a fresh comparison before requesting a quote.

Keep the generic message for unknown, malformed, unauthenticated, and
unmapped error codes. Never render service-supplied error message text. Extend
the browser harness to record only the stale route's HTTP status and validated
versioned error code from a cloned local response, then assert the exact
user-facing message. No API or action behavior changes.

## Frozen baseline

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. Before this packet,
`git status --porcelain=v1 --untracked-files=all` was 2,671 lines / 229,884
bytes, SHA-256
`bc73afd98c6e1850433dfbbb293b4904ca618f7df6e8d8dc6a03ee18ac5f063d`.

| File | SHA-256 before this slice |
|---|---|
| `apps/hyperai-interface/error-envelope.js` | `52b74c23fdffe6971a0caa6584870387e77cabe289f61e20985cc59cb322d3ad` |
| `apps/hyperai-interface/error-envelope.test.mjs` | `022407b225dd0fed1eade0cc55b9ed7dfd27119d17eb586d5db43d5c988fede4` |
| `scripts/verify-interface-browser.mjs` | `7ed53daf1b89c0561cb6010c1eafdd2121ee4843974846787ec65276d3b28263` |
| `package.json` (read-only) | `0884806933b247d01ebce726b08536c5589cb89f3d50d61833494dc52a0386a0` |
| `apps/hyperai-interface/app.js` (read-only) | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `crates/hyperai-local-service/src/server.rs` (read-only) | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |

Runtime: Node.js `v22.23.2`, pnpm `11.7.0`, pinned Rust/Cargo `1.93.1`, and
Google Chrome `154.0.8037.92`.

## Post-edit pre-run freeze — 2026-10-01

Full Git status is 2,672 lines / 229,947 bytes, SHA-256
`ee3d17024204b5a465dee9a5623d09c682e278b3f917339e69e451fe5f6707b6`.

| File | SHA-256 for this run |
|---|---|
| `apps/hyperai-interface/error-envelope.js` | `16bd46b87e8ba899779f7c51dc9a477feebcb16277fcac3540579cc4cea21874` |
| `apps/hyperai-interface/error-envelope.test.mjs` | `56473545e77df64401a88a181f32382515f110b93b1c8628d8f26174ffac35ce` |
| `scripts/verify-interface-browser.mjs` | `29b742d6498cb08badbb747a5707081ae758b5d89bded5da2aefcb9161fc6483` |
| `apps/hyperai-interface/app.js` (read-only) | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `package.json` (read-only) | `0884806933b247d01ebce726b08536c5589cb89f3d50d61833494dc52a0386a0` |
| `crates/hyperai-local-service/src/server.rs` (read-only) | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |

Preflight passed: JavaScript syntax checks; focused error-envelope tests
(4/4); root `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` (34 interface
tests, 3 installer tests, formatting, and strict Clippy); root
`CARGO_NET_OFFLINE=true corepack pnpm run verify:contracts` (selected contract
tests passed; the configured Postgres test remains ignored); all three pinned
imports via `python3 scripts/verify_imports.py`; and `git diff --check`.

## Exact writable files

- `apps/hyperai-interface/error-envelope.js`: add one fixed code-to-copy
  mapping; preserve the generic fallback and existing status/code validation.
- `apps/hyperai-interface/error-envelope.test.mjs`: test exact mapping,
  generic fallback, and rejection of body-supplied text.
- `scripts/verify-interface-browser.mjs`: capture only HTTP status and the
  error envelope code from a response clone in stale mode; assert exact copy.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record scope and result.

All other application, service, market, API, workspace, manifest, dependency,
import, fixture, and upstream files are read-only. The user-visible change is
limited to the one fixed conflict code; no action authority is added.

## Allowed effects and stop conditions

Run `node --check` on the changed JavaScript, root `lint:fast`, root
`verify:contracts`, `python3 scripts/verify_imports.py`, and `git diff --check`.
Then run exactly once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:stale-quote-browser
```

The command uses the existing local interface binary, fresh disposable service
home and Chrome profile, kernel-assigned IPv4 loopback ports, fixed synthetic
comparison, and one zero-digest `POST /api/v1/market/supply-quote-preview`.
Only same-origin reads and that single stale preview request are allowed. The
runner may retain the response status and validated `code` field only; it must
not retain or emit response body text. No quote approval, completion, seller,
provider, registry, external origin, model, Fleet, wallet, chain, or funds
action is allowed. Use no Bazaar source or data.

Before the browser run, freeze edited-source hashes, full Git-status byte
count/digest, browser version, and exact command. Stop without retry on source
or state drift, unexpected status/code/message, any raw service message in the
UI or output, a write outside the allowlist, browser error, or ambiguous
cleanup.

## Proof gates and claim ceiling

- The fixed unit test maps only
  `MARKET_SUPPLY_COMPARISON_CHANGED` to the exact copy above.
- Unknown and other valid codes still use the generic fallback; malformed
  envelopes remain unknown; body-supplied `message` fields never appear.
- The browser observes HTTP 409 and
  `MARKET_SUPPLY_COMPARISON_CHANGED` from the response clone.
- The Market error is the exact fixed text in a polite live status region; the
  route remains Market, quote-preview details are absent, and no approval,
  completion, or other write is issued.
- Existing 33-route, three-viewport, reduced-motion, and contained-layout
  checks pass. Browser/service processes, process group, listeners, and
  temporary home/profile are cleaned conclusively.

Maximum claim: `INTERNAL_PASS` for this local synthetic stale-comparison
recovery message, fixed envelope mapping, and browser response binding. This
does not establish user comprehension, general accessibility, other clients,
native-platform support, live supply, seller intent, execution, payment,
independent review, source rights, Phase 2 acceptance, or production
readiness. Reviewer status starts `NOT_REQUESTED`; reproduction starts
`NOT_REPRODUCED`; custody is limited to local source, fixed synthetic input,
and disposable browser/service state. Phase 2 remains `NOT_ASSIGNED`; live
Fleet, paid Eval, financial Trade, provider access, settlement, publication,
and deployment remain closed. Publish boundary: local source and
documentation only; no commit, PR, release, or deployment.

## Execution record — 2026-10-01: `INTERNAL_PASS`

The exact frozen command exited 0 on Google Chrome `154.0.8037.92`. The
stale request used the all-zero comparison digest and returned HTTP 409 with
`MARKET_SUPPLY_COMPARISON_CHANGED`, captured from a cloned response as the
only retained error-body field. The page stayed on Market and announced:

> This supply comparison is out of date. Reload this page to get a fresh comparison before requesting a quote.

The same exact text appeared in the polite live status region. Quote-preview
details were cleared; no approval, completion, or other non-read request was
made. Exactly one comparison `GET` returned HTTP 200 and one quote-preview
`POST` returned HTTP 409. The runner emitted only the validated code and
status, not the service message or response body.

All 33 workspace routes passed at 1440x1000, 720x500, and 390x844 with no
document overflow. All 33 reduced-motion checks passed across 23,760 motion
entries. Chrome exited 0 with no live process-group members; the service
stopped after `SIGTERM`; both ephemeral listeners closed; and the temporary
home/profile was removed.

The root focused error-envelope tests passed 4/4, `lint:fast` passed with 34
interface tests and 3 installer tests plus pinned formatting/strict Clippy,
`verify:contracts` exited 0 (one configured Postgres test remains ignored),
all three imported-source pins passed, and `git diff --check` passed. Post-run
source hashes matched the frozen values. Git status remained 2,672 lines /
229,947 bytes, SHA-256
`ee3d17024204b5a465dee9a5623d09c682e278b3f917339e69e451fe5f6707b6`.

Assurance is `INTERNAL_PASS` for this same-host synthetic conflict mapping and
browser recovery path. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. The exact instruction to reload was verified in UI, but user
comprehension was not evaluated. Native platforms, other clients, broader
accessibility, source rights, independent review, Phase 2 acceptance, and
production readiness remain unverified.

For audit context, the one browser command was:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:stale-quote-browser
```
