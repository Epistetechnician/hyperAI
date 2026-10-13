# Installed TUI Buyer Quote Lifecycle Packet

State slice: `hyperai.bazaar.desk_runtime.installed_tui_quote_lifecycle.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md),
[installed quote lifecycle packet](compute-bazaar-installed-quote-lifecycle-packet.md),
and [Rust port subplan](compute-bazaar-rust-port-subplan.md).

Status: accepted local qualification slice; implementation is pending.

## Purpose and boundary

Exercise the source-installed `hyperai desk` TUI through the existing synthetic
buyer flow: select Market, request a quote preview, approve that preview, and
request mock completion. Observe the rendered preview, approval, receipt, and
effect boundary through the PTY transcript without retaining the transcript
or changing product behavior.

The existing PTY harness starts the service on IPv4 loopback port `0`, creates
a disposable home, and checks terminal restoration and resizing. The new
optional flow uses only that service and fixed HyperAI synthetic supply. The
existing port-4317 service is never targeted. No remote provider, seller,
registry, agent, database, chain, or external endpoint is contacted.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`; the
pre-existing dirty and untracked work is preserved. The NUL-delimited Git
status SHA-256 at freeze is
`dd99044734877c2896f81018a34de09bac2b7b8ce6b586f24ab0ebcd6f55e04f`.

| File | SHA-256 | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `efeb7b8765f4de75d45de56bece597b50b2044684be0efd61ce5eb22e1b3c613` | Writable PTY harness |
| `crates/outcomes-cli/tests/cli_compat.rs` | `b2c2fed0bd56172cee276c28a7e6c6d2382ffee861f2ea47e77c05928f3ae262` | Writable installed-binary integration |
| `crates/hyperai-tui/src/lib.rs` | `346b9c1f68d1bbc8cba70e9e1e49e7ad62a6d2d2d0288cbd92a2d2bac9deb203` | Read-only TUI behavior |
| `crates/outcomes-cli/src/lib.rs` | `9367ccb2e2c1ac1d5d968e32d8e932938f97c8d82c6bee5903aed88db919354c` | Read-only command behavior |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` | Read-only installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Read-only locked dependencies |
| `docs/state-slices.md` | `bf0dc4583ae8e9681ff6eb563cec76393b3c37af9b0e6001f23477cdcade1c9a` | State-record baseline |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Parent-plan baseline |
| `docs/bazaar-hyperai-interface-plan.md` | `44098e988424ffe91009a12667bdb88721c099339720735201b6ea627eb9660e` | Product-plan baseline |

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: add an opt-in installed-TUI flow,
  bounded marker waits, and boolean evidence in the JSON report. Retain only
  the existing transcript byte count and digest; never emit raw terminal
  output.
- `crates/outcomes-cli/tests/cli_compat.rs`: enable the flow in the existing
  offline source-install integration and assert its report.
- This packet and `docs/state-slices.md`.
- `docs/compute-bazaar-rust-port-subplan.md` and
  `docs/bazaar-hyperai-interface-plan.md`: record the result only after the
  proof gates pass.

Read-only inputs: the installed TUI and CLI behavior, local service routes,
installer, workspace manifests, lockfile, imported products, and ignored
Bazaar reference.

## Allowed effects and limits

Allowed effects are the exact ignored Unix source-install integration using
pinned Rust and locked offline dependencies; its disposable install, service,
home, and PTY; an in-memory Python syntax check for the changed harness;
`corepack pnpm run lint:fast`; pinned formatting; `git diff --check`; and scoped
whitespace inspection. The service binds only to IPv4 loopback port `0`. The
harness retains bounded result flags and hashes, not screen contents.

Do not change the TUI, service, API, domain contracts, permissions, package
contents, dependencies, or product behavior. Do not change or read the
user-owned HyperAI home. Do not contact external systems or enable providers,
sellers, live Fleet, paid Eval, financial Trade, wallets, chains, publication,
or deployment. Bazaar code, data, schemas, fixtures, and assets remain
excluded.

## Proof gates

1. The installed TUI opens in a disposable PTY and navigates to the Market
   workspace.
2. `Enter`, `a`, then `c` expose the synthetic quote-preview, approval, and
   mock-receipt stages in order.
3. The completion display includes a receipt digest and the fixed
   `local_synthetic_mock_fulfillment_verifier_and_settlement_only` boundary.
4. The PTY still proves its existing resize, Ctrl+C, raw-mode, alternate-screen,
   cursor-restoration, service-shutdown, and temporary-home cleanup checks.
5. The enclosing ignored offline install integration passes, followed by
   `lint:fast` and diff hygiene.

Claim ceiling: installed HyperAI TUI display and interaction for one fixed
local synthetic quote lifecycle on the tested host. This does not establish
seller contact, workload execution, correct computation, payment, settlement,
cross-platform support, broad accessibility, independent review, or production
readiness. Reviewer status is `NOT_REQUESTED`; reproducibility begins as
`NOT_REPRODUCED`. Publish boundary: local code and documentation only; no
staging, commit, PR, release, external publication, or deployment.

## Execution attempt — 2026-10-02

The exact locked offline ignored source-install integration exited 101 with
`0 passed, 1 failed`. The installed TUI PTY run did not observe
`Workspace: Market / buyer supply` within five seconds after sending `j`, so
it stopped before preview, approval, receipt, resize, or Ctrl+C assertions.
The TUI had entered the alternate screen and raw mode. The test output did not
retain the raw transcript or a successful-run JSON report. The Python harness
ran its `finally` service-stop path; a read-only scan found no remaining
`hyperai-ctrlc-*` temporary directory. This attempt is `INCONCLUSIVE` for the
installed TUI workflow and proves none of the new quote-lifecycle gates.

The source order shows that the terminal enters alternate-screen mode before
its first draw and event-poll iteration. The harness reacted to the alternate
screen sequence without first observing the initial Overview draw. The next
attempt must use a fresh packet and synchronize on that first rendered screen
before sending navigation input; this packet's one attempt is consumed and
must not be repeated. The new one-run continuation is the
[startup-sync packet](compute-bazaar-installed-tui-quote-startup-sync-packet.md).
