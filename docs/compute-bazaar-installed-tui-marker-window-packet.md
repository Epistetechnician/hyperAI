# Installed TUI Quote First-Frame Marker Window Continuation

State slice: `hyperai.bazaar.desk_runtime.installed_tui_marker_window.v1`.
Parent packets: [installed TUI quote lifecycle](compute-bazaar-installed-tui-quote-lifecycle-packet.md)
and [startup synchronization](compute-bazaar-installed-tui-quote-startup-sync-packet.md).

Status: closed `INCONCLUSIVE`; its one installed-binary integration attempt is
consumed. The two prior quote-lifecycle packets are also closed
`INCONCLUSIVE`; none may be rerun.

## Purpose and bounded diagnosis

Determine whether the startup-sync failure was caused by the harness waiting
only on PTY bytes received after an offset that was set after the alternate
screen bytes had already been read. The parent run did not retain raw output,
so this is a source-supported hypothesis, not a confirmed explanation.

The installed TUI, CLI, service, API, permissions, and domains remain
read-only. The harness will record whether `Workspace: Overview` was already
present in its accumulated transcript before the initial marker wait, then
search from byte zero. It will retain only that boolean, existing stage
booleans, byte count, and transcript digest. It will never print or persist
terminal text.

## Evidence and fix selection

Local source evidence: `run_desk` drains one PTY read and exits its startup
loop as soon as the accumulated transcript contains the alternate-screen
sequence. `exercise_market_quote_lifecycle` then sets its marker-search offset
to the current transcript length before waiting for the first Overview marker.
The TUI draws its first frame before entering Crossterm's 100 ms event-poll
loop. Therefore the alternate-screen sequence and first Overview frame can be
present in the same already-read PTY chunk, placing Overview before the saved
offset. The raw bytes were not retained in either failed attempt, so whether
that occurred in the failed run remains unknown.

Candidate fixes considered:

1. **Search the already accumulated startup bytes from offset zero.** This
   addresses the exact read-order defect in the harness, requires no product
   change or new dependency, and allows a boolean to show whether the marker
   was already present. Selected.
2. **Replace `openpty` plus `fork` with a controlling-terminal session** such
   as Python's `pty.fork`. This is a valid PTY setup to consider if input or
   terminal ownership remains suspect. The present run entered raw mode and
   the alternate screen, so it does not explain a missing marker at the
   saved transcript offset.
3. **Interpret output with a VT screen emulator.** Ratatui emits buffer diffs
   and only writes changed cells, so a virtual screen is more robust than
   searching raw history for later screen states. It adds a parser/dependency
   and exceeds the observed first-frame bug; reserve it for a fresh packet if
   post-key markers still fail after the offset repair.
4. **Increase the fixed sleep before input.** Crossterm's event poll waits for
   event availability, and PTY I/O is asynchronous; elapsed time alone cannot
   recover bytes already consumed before the marker offset. Rejected as the
   primary fix.
5. **Add TUI test-support reporting from Ratatui's completed frame.** This
   could verify a rendered buffer but changes the product-side test seam and
   would not by itself prove installed PTY keyboard behavior. Rejected for
   this harness-only slice.

Reference behavior: [Ratatui rendering and buffer diffing](https://docs.rs/ratatui/latest/ratatui/struct.Terminal.html#rendering-pipeline),
[Crossterm event polling](https://docs.rs/crossterm/latest/crossterm/event/index.html),
[Python PTY utilities](https://docs.python.org/3/library/pty.html), and
[PTY master/slave behavior](https://www.man7.org/linux/man-pages/man7/pty.7.html).
These references inform the test design; they do not establish what the
closed attempts emitted.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved. The NUL-delimited Git
status SHA-256 after recording this packet and state slice, and before
changing the harness, is
`faf13fedabe6e18ba456ebc4147a21c3cccd56312076eb82e44a7975212e1e4c`.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `bd71ecfeca5bee2d9dc555c562ccf77d2234640b0ffbc717af8f29a1e7e505ec` | Writable harness |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only integration |
| `crates/hyperai-tui/src/lib.rs` | `346b9c1f68d1bbc8cba70e9e1e49e7ad62a6d2d2d0288cbd92a2d2bac9deb203` | Read-only TUI |
| `crates/outcomes-cli/src/lib.rs` | `9367ccb2e2c1ac1d5d968e32d8e932938f97c8d82c6bee5903aed88db919354c` | Read-only command |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53ceebd44a7b9ddf49b3` | Read-only installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline build |
| `docs/compute-bazaar-installed-tui-quote-lifecycle-packet.md` | `5173a5cb67e2575c5c758ccf29c53456e84d2b71cf195d203271c2795b1388d0` | Read-only first failure |
| `docs/compute-bazaar-installed-tui-quote-startup-sync-packet.md` | `cc6e63ef0873864244bd97ffc6e3046d5e6daaa61bf494654cde407c082598a9` | Closed packet baseline |
| `docs/compute-bazaar-phase-1-contract-packet.md` | `bae05fbeaed2431322aba4207fe4f23867c485158b38d7c5fdbdd40e74263993` | Read-only Phase 1 contract |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only parent plan |
| `docs/bazaar-hyperai-interface-plan.md` | `44098e988424ffe91009a12667bdb88721c099339720735201b6ea627eb9660e` | Read-only product plan |

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: search the initial Overview marker
  from byte zero and report whether it was already present before the wait.
- This packet and `docs/state-slices.md`: scope, evidence, and final result.

The two earlier packets are closed in their own records. Do not change the
TUI, CLI integration, service, API, dependencies, package contents, domain
contracts, or authorization behavior under this slice.

## Allowed effects and stop conditions

Allowed: an in-memory Python syntax check; one exact locked offline ignored
source-install integration on the existing Darwin arm64 host; its disposable
installation, home, IPv4 loopback port `0` service, and PTY; root
`corepack pnpm run lint:fast`; pinned Rust formatting; `git diff --check`; and
scoped cleanup/whitespace checks. The integration is:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

Before the run, verify the frozen source hashes and intended disposable port.
Stop if any read-only input drifted, if the service would target port `4317`,
or if a temporary path resolves inside the user-owned HyperAI home. Do not
contact providers, sellers, registries, agents, remote workers, chains,
wallets, or non-loopback endpoints. Do not run the integration a second time
under this packet. If it fails or is inconclusive, preserve that result and
create a fresh packet before another integration attempt.

## Proof gates and claim ceiling

1. The report records `overview_marker_preobserved` and observes Overview
   before sending `j`.
2. The Market heading follows `j`; preview, approval, and receipt follow Enter,
   `a`, and `c` in order.
3. The receipt digest and fixed synthetic effect-boundary identifier are
   visible in the retained boolean evidence.
4. Existing PTY resize, Ctrl+C, terminal restoration, service shutdown, and
   temporary-home cleanup assertions pass.
5. The enclosing installed integration, `lint:fast`, pinned formatting, and
   diff hygiene pass.

The maximum claim is the installed local synthetic TUI quote flow and marker
observation on this host. It does not establish real fulfillment, computation
correctness, payment, settlement, native cross-platform support, broad
accessibility, independent review, or production readiness. Reviewer status
is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Publish boundary: local
code and documentation only; no staging, commit, PR, release, publication, or
deployment.

## One-run result — 2026-10-02

The exact locked offline ignored source-install integration exited 101 with
`0 passed, 1 failed`: after alternate-screen/raw-mode entry, the harness did
not observe `Workspace: Overview` within five seconds, even when searching the
full accumulated transcript from byte zero. It stopped before quote keys,
resize, or Ctrl+C assertions. Because the marker wait raised before returning
its JSON report, the new `overview_marker_preobserved` flag was not emitted.
No raw transcript was retained. The Python child cleanup ran; the Rust test's
disposable home was removed. Post-run checks found no test-owned
`hyperai-ctrlc-*` directory and no matching HyperAI/outcomes CLI process.

The single allowance is consumed. The full-buffer-offset hypothesis does not
explain this result by itself. The raw marker may be split by terminal control
sequences, or the test may need a semantic render signal; the failed run does
not distinguish those cases. Do not rerun this packet. A fresh packet must
select a new diagnostic from the TUI render implementation and Ratatui's
screen-testing facilities.

`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed: interface checks
34/34, installer checks 3/3, pinned formatting, and strict workspace Clippy.
Python syntax, pinned formatting, and scoped trailing-whitespace checks also
passed before the integration. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. Assurance is `INCONCLUSIVE` for the
installed TUI quote interaction.
