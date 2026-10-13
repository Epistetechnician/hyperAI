# Installed HyperAI TUI PTY Packet

State slice: `hyperai.bazaar.desk_runtime.installed_tui_pty.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md)
and [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

Exercise `hyperai desk` from the installed source bundle in a real POSIX PTY.
The existing standard-library harness accepts `--binary`; the ignored source
install integration will pass it the installed `hyperai` executable and
assert alternate-screen/raw-mode entry, Ctrl+C exit, terminal restoration,
cursor/alternate-screen restoration, and disposable-home cleanup.

## Exact files

- `crates/outcomes-cli/tests/cli_compat.rs`: invoke the existing PTY harness
  against the installed `hyperai` binary and validate its fixed JSON report.
- `docs/compute-bazaar-installed-tui-pty-packet.md`: scope and evidence.
- `docs/state-slices.md`: accepted authority, assurance, custody, and claim
  ceiling.
- `docs/compute-bazaar-phase-1-contract-packet.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: report the result and remaining
  platform gates.

## Allowed effects and boundaries

Run the existing source installer with `--offline` into a disposable root.
Invoke `python3 scripts/verify_hyperai_tui_ctrl_c.py --binary <installed-hyperai>`
on POSIX only. The harness creates one temporary HyperAI home, starts the
installed service on loopback with port `0`, enters the installed TUI in an
80x24 pseudo-terminal, sends Ctrl+C, checks terminal restoration, stops the
service, and deletes the temporary home. Keep the existing service on
`127.0.0.1:4317` untouched.

No provider, seller, registry, external agent, model, remote worker, user
data, non-loopback network, paid execution, live Fleet, financial Trade,
wallet, chain, publication, or deployment is in scope. No runtime behavior,
package contents, API contract, source dependency, credential, or build graph
changes.

## Proof gates and claim ceiling

- Run the exact ignored source-install integration under pinned Rust 1.93.1
  with locked offline dependencies; require all PTY assertions and existing
  installed lifecycle/ACP assertions to pass.
- Run root `corepack pnpm run lint`, pinned Rust formatting, `git diff
  --check`, and scoped whitespace checks. Run the pinned import verifier and
  record each product result separately; preserve unrelated mismatches.
- Confirm the harness reports temporary-home removal, no test-owned service
  remains, and port 4317 remains owned by the existing service.

Maximum claim: installed TUI startup and terminal restoration on the tested
POSIX host. This does not qualify physical-key input, resize behavior, assistive
technology, native Linux or Windows, Intel macOS, installer signing, release
readiness, independent review, or production authorization. Reviewer status
is `NOT_REQUESTED`; reproduction remains `NOT_REPRODUCED` until a separate
environment runs the exact test.

## Validation report (2026-09-29)

The exact ignored offline source-install integration passed:

```sh
sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli \
  --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

Environment: macOS 26.6.1 arm64, Rust/Cargo 1.93.1, Python 3.14.5. The
installed TUI entered raw mode and the alternate screen, handled Ctrl+C with
exit status 0, restored termios/cursor/alternate-screen state, and removed its
temporary home. The test also retained its installed CLI, shared-service, and
synthetic ACP/MCP checks. It validated that emitted transcript and installed
binary SHA-256 values had 64 characters but did not retain the values.

`corepack pnpm run lint`, pinned Rust formatting, and `git diff --check`
passed. The pinned import verifier passed Simple Compute Market and HyperIntel;
JustAI remains mismatched because its imported tree contains the unresolved
extra `products/justai/pnpm-lock.yaml`. No files were staged or committed. The
test-owned service was removed and the pre-existing `127.0.0.1:4317` listener
remained running. Assurance is `INTERNAL_PASS` for the stated installed POSIX
path; reviewer status is `NOT_REQUESTED`, and reproducibility is
`NOT_REPRODUCED`.
