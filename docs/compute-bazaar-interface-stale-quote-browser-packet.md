# HyperAI Stale Supply Quote Browser Packet

Status: completed local synthetic browser qualification slice.

State slice: `hyperai.bazaar.interface.stale_supply_quote_browser.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust port subplan](compute-bazaar-rust-port-subplan.md). This qualifies one
negative path after the
[synthetic supply-to-receipt browser workflow](compute-bazaar-supply-quote-lifecycle-browser-packet.md).

## Goal

Show that a stale buyer supply-comparison digest fails closed before quote
approval. The browser remains on the Market page, receives the versioned
conflict, clears quote-preview state, and exposes a live status. No product
behavior changes are in scope.

## Frozen baseline

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the full pre-existing dirty and untracked worktree. Before this packet,
`git status --porcelain=v1 --untracked-files=all` was 2,670 lines / 229,821
bytes, SHA-256
`767954cdc25bf296475ef9c836305b029849dd4861545283b906027a7ad94ed5`.

| File | SHA-256 before this slice |
|---|---|
| `package.json` | `9b5096417011f6e15a53b5e2a028fa13ff10ad5a00041aa29a3cf1a2f0d7658a` |
| `scripts/verify-interface-browser.mjs` | `67d69e4b8f66c9f0431701cdbe41d9bf45c6a97c6b53aa228da95666b879121c` |
| `apps/hyperai-interface/app.js` (read-only) | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `crates/hyperai-local-service/src/server.rs` (read-only) | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `crates/hyperai-market/src/lib.rs` (read-only) | `16b07ec36107d29182020abecd23b8cd23862e3c3203914790877409a65a44f8` |

Runtime: Node.js `v22.23.2`, pnpm `11.7.0`, pinned Rust/Cargo `1.93.1`, and
Google Chrome `154.0.8037.92`.

## Post-edit pre-run freeze — 2026-10-01

Full Git status is 2,671 lines / 229,884 bytes, SHA-256
`bc73afd98c6e1850433dfbbb293b4904ca618f7df6e8d8dc6a03ee18ac5f063d`.

| File | SHA-256 for this run |
|---|---|
| `package.json` | `0884806933b247d01ebce726b08536c5589cb89f3d50d61833494dc52a0386a0` |
| `scripts/verify-interface-browser.mjs` | `7ed53daf1b89c0561cb6010c1eafdd2121ee4843974846787ec65276d3b28263` |
| `apps/hyperai-interface/app.js` (read-only) | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `crates/hyperai-local-service/src/server.rs` (read-only) | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `crates/hyperai-market/src/lib.rs` (read-only) | `16b07ec36107d29182020abecd23b8cd23862e3c3203914790877409a65a44f8` |

Preflight passed: `node --check scripts/verify-interface-browser.mjs`,
`git diff --check`, package JSON parsing, root `CARGO_NET_OFFLINE=true
corepack pnpm run lint:fast` (33 interface tests, 3 installer tests, pinned
formatting, and strict all-target/all-feature Clippy), root
`CARGO_NET_OFFLINE=true corepack pnpm run verify:contracts` (all selected
contract tests passed; one configured Postgres test remains ignored), and
`python3 scripts/verify_imports.py` for the pinned Simple Compute Market,
HyperIntel, and JustAI trees. The browser command below is the only run under
this packet.

## Exact writable files

- `scripts/verify-interface-browser.mjs`: add a dedicated stale-digest mode.
- `package.json`: register its isolated command.
- This packet and `docs/state-slices.md`: record scope and execution evidence.
- `docs/compute-bazaar-rust-port-subplan.md` and
  `docs/bazaar-hyperai-interface-plan.md`: update current evidence after the
  exact run.

Product application, API and market code, imported trees, manifests, lockfiles,
dependencies, source fixtures, and runtime behavior are read-only.

## Allowed effects and stop conditions

Run JavaScript syntax, root `lint:fast`, root contract verification, import
verification, formatting, and diff checks. Build the existing local browser
service using the pinned offline toolchain. Then execute exactly once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:stale-quote-browser
```

Use a fresh disposable service home and browser profile with kernel-assigned
IPv4 loopback ports. Same-origin reads are allowed; the workflow submits
exactly one `POST` to `/api/v1/market/supply-quote-preview` with an all-zero
comparison digest; the request receives HTTP 409. The service maps the typed
`SupplyComparisonMismatch` to `MARKET_SUPPLY_COMPARISON_CHANGED`; the browser
runner retains response status but not the response body, so the on-wire error
code is source-verified rather than captured in this browser result. Approval,
completion, and every other write are denied by the runner. Do not contact the
existing port 4317 listener,
external origins, providers, sellers, registries, models, fleets, wallets, or
chains. Use no Bazaar source, data, schemas, fixtures, or assets.

Before the run, record the post-edit harness/package hashes, full Git-status
byte count/digest, browser version, and exact command. Stop without retry if
any frozen input drifts, the response differs from the expected 409 contract,
quote-preview state remains, a request is outside the allowlist, a browser
error occurs, or cleanup is ambiguous.

## Proof gates and claim ceiling

- Keyboard-navigate to Market and verify its visible heading and current
  comparison digest.
- Intercept only the one quote-preview request and replace its digest with
  64 zeroes; retain the originally displayed digest for comparison.
- Confirm one HTTP 409 response and verify in the frozen service source that
  `SupplyComparisonMismatch` maps to
  `MARKET_SUPPLY_COMPARISON_CHANGED`.
- Confirm an accessible live error status appears, the page remains on Market,
  quote-preview details are cleared, and no approval or completion request is
  made.
- Confirm there is exactly one non-read request and it is the stale quote
  preview; retain the existing route, viewport, and reduced-motion checks.
- Confirm browser/service exit, process-group emptiness, listener closure, and
  temporary-home/profile removal.

Maximum claim: `INTERNAL_PASS` for this exact same-host local synthetic stale
comparison-digest failure path. The generic error copy is evaluated only for
presence and live-region exposure; this packet does not claim user
comprehension. It does not establish other client behavior, general
accessibility, platform support, live supply, seller intent, execution,
payment, reuse rights, independent review, Phase 2 acceptance, or production
readiness. Reviewer status starts `NOT_REQUESTED`; reproduction starts
`NOT_REPRODUCED`. Artifacts remain local. Phase 2 stays `NOT_ASSIGNED`, and
live Fleet, paid Eval, financial Trade, external access, settlement,
publication, and deployment remain closed. Publish boundary: local source and
documentation only; no commit, PR, release, or deployment.

## Execution record — 2026-10-01: `INTERNAL_PASS`

The exact command exited 0 on Google Chrome `154.0.8037.92`. Keyboard
navigation opened Market and exposed the fixed comparison digest
`5036567a60199af6f6a9ea0fde9871854ce951b1d65ec0f1783ab483575ea5b6`.
The browser's displayed/client digest matched; the runner changed only the
outgoing preview request to the all-zero digest
`0000000000000000000000000000000000000000000000000000000000000000`. The
service returned HTTP 409. The frozen server source maps the typed comparison
mismatch to `MARKET_SUPPLY_COMPARISON_CHANGED`, but the browser result did not
retain or independently inspect the JSON response code.

The page stayed on Market and exposed the generic safe error in a
`role="status"`, `aria-live="polite"` element. Quote details, approval, and
completion were absent. Exactly one supply-comparison `GET` returned HTTP
200, one supply-quote-preview `POST` returned HTTP 409, and quote-approve,
quote-complete, and all other writes were absent. All 33 routes passed at
1440x1000, 720x500, and 390x844 with no document overflow. All 33
reduced-motion checks passed across 23,760 motion entries. The browser exited
0 with no live process-group members; the service stopped after `SIGTERM`,
both ephemeral listeners closed, and the temporary home/profile was removed.

Post-run package and runner hashes matched the frozen identities. Full Git
status remained 2,671 lines / 229,884 bytes, SHA-256
`bc73afd98c6e1850433dfbbb293b4904ca618f7df6e8d8dc6a03ee18ac5f063d`. The
three imported-source digests and `git diff --check` passed after the run.
Assurance is `INTERNAL_PASS` for this exact same-host synthetic fail-closed
browser path. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. The generic error copy is safely bounded but does not show
users what to do next; this run makes no user-comprehension claim. Other
clients, broader accessibility, native platforms, source rights, Phase 2
acceptance, independent review, and production readiness remain unverified.

The source/API error code is verified by code inspection in
`crates/hyperai-local-service/src/server.rs`; it is not an observed browser
response-body claim.

For audit context, the only run command was:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:stale-quote-browser
```
