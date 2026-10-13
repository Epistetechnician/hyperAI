# Installed HyperAI CLI Lifecycle Packet

State slice: `hyperai.bazaar.desk_runtime.installed_cli_lifecycle.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md)
and [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated work.

## Purpose

Make the installed-binary `install -> init -> doctor -> service start ->
connect/status -> service stop` path a repeatable integration check. Existing
tests exercise lifecycle commands using Cargo test binaries, and the
packaged ACP integration starts the installed host through its hidden
`__serve` entrypoint. This slice verifies the public lifecycle commands on
the installed `hyperai` binary itself.

## Exact files

- `crates/outcomes-cli/tests/cli_compat.rs`: extend the existing ignored
  offline source-install test with installed `--version`, `init`, `doctor`,
  `service start`, `service status`, `connect`, and `service stop` assertions.
  A cleanup guard retries stop on test failure or unwind.
- `docs/compute-bazaar-installed-cli-lifecycle-packet.md`: contract and proof.
- `docs/state-slices.md`: authority, evidence, and claim ceiling.
- `docs/compute-bazaar-phase-1-contract-packet.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: record the result and remaining
  platform gates.

## Allowed effects and boundaries

Run the existing local source installer with `--offline` into a disposable
temporary root. Invoke only those installed CLI commands against a disposable
HyperAI home, with the service bound to an OS-assigned loopback port (`--port
0`). Verify that `connect` and `service status` report the same ready service
instance, that `doctor` reports the initialized home and loopback service as
ready, and that `service stop` returns the home to `stopped`. Keep the
existing service on `127.0.0.1:4317` untouched. The existing synthetic ACP
integration then runs unchanged in its separate disposable home.

No provider, seller, registry, external agent, model, remote worker, user
data, network access, paid execution, live Fleet, financial Trade, wallet,
chain, publication, or deployment is in scope. No runtime behavior, API
contract, package contents, credentials, or source dependencies change.

## Proof gates and claim ceiling

- Run the exact ignored source-install test under pinned Rust 1.93.1 with
  locked offline dependencies; require the installed lifecycle and ACP checks
  to pass.
- Run root `corepack pnpm run lint`, pinned import verification,
  `cargo fmt --all -- --check`, and `git diff --check`; check exact-file
  whitespace and temporary service/home cleanup.
- Confirm the pre-existing service on port 4317 remains running.

Maximum claim: repeatable local installed-binary lifecycle behavior on the
tested macOS arm64 host. This does not establish native Windows/Linux/Intel
macOS behavior, installer signing, accessibility, release readiness,
independent review, or production authorization. Reviewer status is
`NOT_REQUESTED`; reproduction remains `NOT_REPRODUCED` until a separate
environment runs this exact test.

## Validation report — 2026-09-29

On Darwin arm64 / macOS 26.6.1 with pinned Rust 1.93.1, the exact ignored
offline install integration passed:

```sh
sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli \
  --test cli_compat source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

This run passed the installed public CLI lifecycle assertions and the existing
synthetic ACP/MCP integration. `corepack pnpm run lint`, pinned Rust formatting,
and `git diff --check` passed. The focused test source SHA-256 is
`3e5539f63a839f8c82a473fc12107da48e94984ee71f7326b34007651932deae`.
The user-owned `127.0.0.1:4317` listener remained PID 85094; after the test,
process inspection found only that existing service. Temporary installation
and homes use `tempfile::TempDir` and were removed when the test returned.

Assurance is `INTERNAL_PASS` for this installed-binary lifecycle and ACP path
on this host. Reviewer status remains `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED` because no separate environment ran this exact test. All test
inputs were synthetic and disposable; no provider, registry, seller, external
agent, paid operation, or network dependency was used.

The separate `python3 scripts/verify_imports.py` gate passed Simple Compute
Market and HyperIntel, but reports a JustAI snapshot mismatch: the imported
tree has 177 files while its source checkout at
`9a0e56536b968cbb2cb7173e142068b53c4f9fe3` has no tracked or present
`pnpm-lock.yaml`; the extra imported file is
`products/justai/pnpm-lock.yaml`. Its origin is unresolved and it was
preserved. Do not claim the complete import inventory is verified until this
is reconciled. Native Linux/Windows/Intel macOS behavior, accessibility,
installer signing, independent review, and release qualification remain open.

The current-source installed buyer quote lifecycle was subsequently qualified
under the separate [installed quote lifecycle packet](compute-bazaar-installed-quote-lifecycle-packet.md).
