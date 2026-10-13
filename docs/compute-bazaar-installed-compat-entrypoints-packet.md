# Installed HyperAI Compatibility Entrypoints Packet

State slice: `hyperai.bazaar.desk_runtime.installed_compat_entrypoints.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md)
and [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

Verify the installed compatibility entrypoints, not just their presence in
the install directory. The ignored offline source-install integration will
prove that installed `hsai` keeps the same version and synthetic market-demo
JSON contract as `hyperai`, and that installed `hyperai-ui` can launch the
shared local service which installed `hyperai` then connects to and stops.

## Exact files

- `crates/outcomes-cli/tests/cli_compat.rs`: extend the existing ignored
  source-install integration with installed alias parity and compatibility
  launcher lifecycle checks. Use a disposable home, a selected loopback port
  other than 4317, and an RAII child-process guard.
- `docs/compute-bazaar-installed-compat-entrypoints-packet.md`: contract and
  evidence.
- `docs/state-slices.md`: accepted scope, assurance, custody, and claim ceiling.
- `docs/compute-bazaar-phase-1-contract-packet.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: report the Phase 1 result and
  remaining platform gates.

## Allowed effects and boundaries

Run the existing local installer with locked dependencies and `--offline`
into a disposable install root. Invoke installed `hyperai`, `hsai`, and
`hyperai-ui` only with the checks above. `hsai market demo` is the existing
fixed HyperAI-authored synthetic report and writes no artifacts. Launch
`hyperai-ui` against one disposable home on an OS-selected free IPv4 loopback
port; verify readiness, connect and status using the installed `hyperai`,
request graceful stop, and reap the child. Keep the existing service on
`127.0.0.1:4317` untouched.

No provider, seller, registry, external agent, model, remote worker, user data,
non-loopback network, paid execution, live Fleet, financial Trade, wallet,
chain, publication, or deployment is in scope. No runtime code, package
contents, API contract, source dependency, credential, or build graph changes.

## Proof gates and claim ceiling

- Run the exact ignored source-install integration under pinned Rust 1.93.1
  with locked offline dependencies. Require identical installed `hyperai` and
  `hsai` version output and demo JSON; require `hyperai-ui` readiness and
  successful installed-CLI connect/status/stop against its instance.
- Run root `corepack pnpm run lint`, pinned Rust formatting, `git diff
  --check`, and scoped whitespace checks. Run the pinned import verifier and
  record each product result independently; preserve unrelated source-tree
  mismatches.
- Confirm the compatibility child is gone, the disposable home is removed,
  and the pre-existing listener on port 4317 remains running.

Maximum claim: installed compatibility-entrypoint behavior on the tested
macOS arm64 host. This does not qualify native Windows, Linux, or Intel macOS
behavior, installer signing, accessibility, release readiness, independent
review, or production authorization. Reviewer status is `NOT_REQUESTED`;
reproduction is `NOT_REPRODUCED` until a separate environment runs the exact
test.

## Validation report — 2026-09-29

On Darwin arm64 / macOS 26.6.1 with pinned Rust 1.93.1, the exact ignored
locked offline source-install integration passed. Installed `hyperai` and
`hsai` returned identical `hyperai 0.1.0` version output and identical
`hyperai.compute-market.local-demo.v1` JSON. Installed `hyperai-ui` started
the shared service in a disposable home on a selected IPv4 loopback port;
installed `hyperai service status` and `connect` identified the same ready
instance, and `service stop` returned it to `stopped` with a successful child
exit.

The focused test source SHA-256 is
`a865eb25f5dc8ac1a69520c168ce0e7c65c517489cc04f009dc5853494a41a06`.
Root `corepack pnpm run lint`, pinned Rust formatting, and scoped whitespace
checks passed. After the test, process inspection found only the preserved
user-owned listener PID 85094 on `127.0.0.1:4317`; installation and home
directories are owned by `tempfile::TempDir` and were removed on return.

Assurance is `INTERNAL_PASS` for these installed compatibility entrypoints on
this host. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED` because no separate environment ran the exact test. All
inputs were HyperAI-authored synthetic output and disposable local state; no
provider, registry, seller, external agent, paid operation, or network
dependency was used.

The standalone import verifier passed Simple Compute Market and HyperIntel
but reports the unresolved JustAI extra-file mismatch documented in the
[installed lifecycle packet](compute-bazaar-installed-cli-lifecycle-packet.md).
The JustAI file was preserved. Native Linux/Windows/Intel macOS behavior,
installer signing, accessibility, independent review, and release
qualification remain open.
