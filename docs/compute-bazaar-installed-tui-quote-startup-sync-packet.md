# Installed TUI Quote Lifecycle Startup-Sync Continuation

State slice: `hyperai.bazaar.desk_runtime.installed_tui_quote_lifecycle_startup_sync.v1`.
Parent: [installed TUI quote lifecycle packet](compute-bazaar-installed-tui-quote-lifecycle-packet.md),
[Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md), and
[Rust port subplan](compute-bazaar-rust-port-subplan.md).

Status: closed `INCONCLUSIVE`; its one authorized local run is consumed. The
parent packet's earlier failed attempt remains `INCONCLUSIVE` and must not be
repeated.

## Purpose and boundary

Repeat the installed TUI quote-lifecycle qualification under a fresh identity,
using a render handshake before keyboard input. The TUI enters alternate
screen before its first draw; therefore seeing the alternate-screen escape
sequence is not proof that the initial workspace has rendered. The harness
will wait for `Workspace: Overview`, allow 300 ms for input polling to settle,
then send `j`, wait for `Workspace: Market / buyer supply`, and send Enter,
`a`, and `c` one at a time after each rendered-stage marker.

This changes test synchronization only. The installed TUI, service, API,
package, permissions, and domain behavior remain read-only. The test uses a
disposable home and loopback port `0`; external connections and real effects
remain closed.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`; all
pre-existing dirty and untracked work is preserved. The NUL-delimited Git
status SHA-256, after recording the parent packet's failed attempt, is
`9262db456b0040acccfa46b6712b04750fa7d9381cb4d17d67b2ef12d3e12f9f`.

| File | SHA-256 | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `e46e691f187655dbbd88e09ac8ebb07c5aa3d263c10ec5ff72fb406e8583efd7` | Writable PTY harness |
| `crates/outcomes-cli/tests/cli_compat.rs` | `e6a1660e8ee8a1101c6c23ed36bf5253bdf5548a42525b64ac39279644cd4618` | Writable installed-binary integration |
| `crates/hyperai-tui/src/lib.rs` | `346b9c1f68d1bbc8cba70e9e1e49e7ad62a6d2d2d0288cbd92a2d2bac9deb203` | Read-only TUI behavior |
| `crates/outcomes-cli/src/lib.rs` | `9367ccb2e2c1ac1d5d968e32d8e932938f97c8d82c6bee5903aed88db919354c` | Read-only CLI behavior |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` | Read-only installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Read-only locked dependencies |
| Parent failed packet | `b22487aa0a240e07ffb25c463ac1fe1b2cc079343c6ddec368af4848aaec4d4b` | Read-only failure record |
| `docs/state-slices.md` | `a7416a2fafaa6ddde0a15d1583467b0ddb0566f1374fd6dce13e849a8eb3ec12` | State-record baseline |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Parent-plan baseline |
| `docs/bazaar-hyperai-interface-plan.md` | `44098e988424ffe91009a12667bdb88721c099339720735201b6ea627eb9660e` | Product-plan baseline |

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: add the Overview-frame handshake
  and settle window before the existing opt-in quote flow.
- `crates/outcomes-cli/tests/cli_compat.rs`: retain invocation and bounded
  report assertions for the installed TUI flow.
- This packet and `docs/state-slices.md`.
- `docs/compute-bazaar-installed-tui-quote-lifecycle-packet.md`: record the
  failed first attempt and link this continuation.
- `docs/compute-bazaar-rust-port-subplan.md` and
  `docs/bazaar-hyperai-interface-plan.md`: record qualification only if every
  gate passes.

Read-only inputs: installed TUI/CLI, service routes, installer, manifests,
lockfile, imported products, and ignored Bazaar reference.

## Allowed effects and stop conditions

Allowed: one exact locked, offline ignored source-install integration; its
temporary installation/home, local IPv4 loopback service on port `0`, and
PTY; an in-memory Python syntax check; `corepack pnpm run lint:fast`; pinned
formatting; `git diff --check`; and scoped whitespace checks. The report may
retain stage booleans, digest-presence booleans, transcript byte count, and
transcript digest only; no raw screen contents.

Stop before the one-run integration if any frozen input drifts, if the service
targets a port other than `0`, or if the installed TUI cannot first render
Overview. Do not contact port `4317`, any external origin, provider, seller,
registry, agent, chain, or wallet. Do not change product code, contracts,
dependencies, or package contents. If this continuation fails or is
inconclusive, preserve the result and require another fresh packet.

## Proof gates

1. Observe the initial Overview frame before sending input; observe the Market
   frame after `j` and the 300 ms settle window.
2. Enter, `a`, and `c` render preview, approval, and mock receipt in order.
3. The completion display includes a receipt digest and
   `local_synthetic_mock_fulfillment_verifier_and_settlement_only`.
4. The existing PTY resize, Ctrl+C, terminal restoration, service-stop, and
   disposable-home cleanup checks pass in the same run.
5. The enclosing installed integration, `lint:fast`, formatting, and diff
   hygiene pass.

Claim ceiling: installed local synthetic TUI quote interaction on the tested
host only. This does not establish real seller contact, computation,
correctness, payment, settlement, native cross-platform support, broad
accessibility, independent review, or production readiness. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Publish boundary: local
code and documentation only.

## One-run execution — 2026-10-02

The exact locked offline ignored source-install integration exited 101 with
`0 passed, 1 failed`. It failed before navigation because the harness did not
observe `Workspace: Overview` within five seconds. It reached alternate-screen
and raw-mode setup, then stopped before quote actions, resize, or Ctrl+C. The
raw PTY transcript was not retained. The harness ran its service-stop
`finally` path; the disposable home was removed. This result is
`INCONCLUSIVE` for the installed quote lifecycle and does not indicate a TUI
product failure.

The sole allowance for this packet is consumed. Do not repeat its frozen
integration. Source inspection found that the alternate-screen reader can
consume a PTY chunk before the quote-flow helper snapshots `len(transcript)`;
the initial Overview marker may already be in the retained bytes. That cause
is plausible but unconfirmed because the raw transcript is intentionally not
kept. A fresh packet must test this marker-window hypothesis and preserve the
same no-raw-transcript rule. See the
[marker-window continuation](compute-bazaar-installed-tui-marker-window-packet.md).
