# Installed HyperAI TUI Resize Packet

State slice: `hyperai.bazaar.desk_runtime.installed_tui_resize.v1`.
Parent: [installed TUI PTY packet](compute-bazaar-installed-tui-pty-packet.md)
and [Rust compute-bazaar subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

Extend the existing installed-binary POSIX PTY check to exercise a terminal
resize down and back up while `hyperai desk` is running. Verify that output
after each `TIOCSWINSZ` update contains a redraw bounded by the new terminal
height. This checks local rendering response only; PTY resize injection is
not physical terminal or accessibility acceptance.

## Exact files

- `scripts/verify_hyperai_tui_ctrl_c.py`: set the PTY dimensions while the
  installed TUI is active, capture resize redraws, and report the observed
  dimensions and cursor-row bounds.
- `crates/outcomes-cli/tests/cli_compat.rs`: assert both resize reports in the
  installed-binary integration.
- `docs/compute-bazaar-installed-tui-resize-packet.md`: scope and evidence.
- `docs/state-slices.md`: state slice, custody, assurance, and claim ceiling.
- `docs/compute-bazaar-phase-1-contract-packet.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: record result and remaining
  platform/usability gates.

## Allowed effects and controls

Run the existing locked, offline source-install integration with the
installed `hyperai` executable. It may create one temporary home, start its
service on IPv4 loopback port `0`, enter an 80x24 POSIX pseudo-terminal, resize
that PTY to fixed smaller and larger dimensions, send a `j` navigation
sequence to force visible redraws, send Ctrl+C, stop its own service, and
remove its temporary home. Keep the existing service on
`127.0.0.1:4317` untouched.

No provider, seller, registry, external agent, model, remote worker, user data,
non-loopback network, paid execution, live Fleet, financial Trade, wallet,
chain, publication, deployment, product runtime change, or dependency change
is in scope. The twelve `j` inputs only navigate local workspace views; they
do not invoke Data, Fleet, Eval, Trade, agent, or service actions.

## Proof gates and claim ceiling

- The installed TUI enters raw mode and the alternate screen before resize.
- The harness applies fixed PTY sizes `80x24 -> 48x10 -> 100x30` and confirms
  the PTY reports each requested size.
- At 48x10, send eleven `j` keys to traverse the complete twelve-item
  workspace list; cursor positions must stay within rows 1 through 10.
- At 100x30, send one `j` key to wrap from the last workspace to Overview.
  The redraw must expose navigation rows below row 10 and no cursor position
  may exceed row 30. This distinguishes expansion from a stale short viewport.
- Ctrl+C exits with status 0 and restores the initial terminal state, cursor,
  and alternate screen; the temporary home is removed and the test-owned
  service stops.
- Run the exact ignored installed-binary integration, root
  `corepack pnpm run lint`, pinned Rust formatting, `git diff --check`, and
  scoped Markdown hygiene. Preserve and report unrelated import-inventory
  mismatches.

Maximum claim: the installed TUI emitted height-bounded redraws after the two
specified synthetic PTY resize events on the tested POSIX host. This does not
qualify physical-key input, actual terminal emulator behavior, screen readers,
populated-state usability, native Linux/Windows/Intel macOS, independent
review, release readiness, or production authorization. Reviewer status is
`NOT_REQUESTED`; reproduction remains `NOT_REPRODUCED` until another
environment runs the exact test.

## Validation report

The geometry-strengthened locked/offline source-install integration passed on
macOS 26.6.1 arm64 with Rust/Cargo 1.93.1 and Python 3.14.5:

```sh
sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli \
  --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The one-key probe described in earlier notes is historical only and does not
qualify this geometry-strengthened run.

The exact integration passed 1/1. At 48x10, eleven navigation events produced
a 3,614-byte redraw with maximum cursor row 9. At 100x30, one wrap event
produced a 1,925-byte redraw with maximum cursor row 28. The installed binary
SHA-256 was
`85ae8c17a960ed781eb1d6b2e46d2b0c57109c1de79435f20c3c9a55fb7594fa`; the
final integration transcript SHA-256 was
`df19ee0f88489cb106bcc62f052763795b05333e6e7bcd35f3955d8d32620bc6`.
The harness SHA-256 is
`2a4d26b3857e2cbde46635f0a24666fbcb850eb981df86078bfad814f40466c0`, and the
integration-test source SHA-256 is
`20aa56550a5ea6213ba6d5880cac544f9b9021de54d57bb68581f475a77318ce`.

The full root `corepack pnpm run lint` passed after the geometry test and
test-only ACP isolation change. Pinned Rust formatting and `git diff --check`
passed. The import verifier passed Simple Compute Market (2,189 files) and
HyperIntel (99 files); JustAI remains mismatched by the preserved extra
`products/justai/pnpm-lock.yaml` relative to its recorded snapshot. No other
upstream source tree was changed.

The temporary home, PTY child, and test-owned service were removed. The
pre-existing service on `127.0.0.1:4317` remained owned by PID 85094. No
dataset, provider, outside endpoint, or user payload was used.

Assurance is `INTERNAL_PASS` for the exact installed macOS arm64 PTY resize and
restoration path. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Known limitations include synthetic PTY rather than physical
terminal input, no screen-reader or populated-state usability assessment, and
no native Linux, Windows, or Intel macOS qualification. Claim ceiling:
height-bounded redraw following the two specified resize events on this host.
The next gates are physical keyboard and assistive-technology acceptance,
native-platform qualification, and a clean second-environment reproduction.
Production/release authority remains closed.
