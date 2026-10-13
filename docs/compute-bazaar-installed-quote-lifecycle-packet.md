# Installed Buyer Quote Lifecycle Packet

State slice: `hyperai.bazaar.desk_runtime.installed_quote_lifecycle.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md),
[installed CLI lifecycle packet](compute-bazaar-installed-cli-lifecycle-packet.md),
and [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
This extends the existing offline install integration to prove that the
installed executable carries the buyer quote lifecycle already qualified
against the local test binary.

## Purpose and boundary

Run the installed `hyperai` binary through compare, quote preview, explicit
buyer approval, and deterministic mock completion against the same disposable
local service used by the installed lifecycle check. Exercise stale preview
and deal digests and preserve their stable error codes. This closes an
installed-CLI packaging gap only; it adds no service, domain, or API behavior.

The source installer runs with `--offline` into a `TempDir`. The installed
service binds to an OS-assigned IPv4 loopback port (`--port 0`) and uses a
disposable HyperAI home. The existing user-owned service on port 4317 remains
untouched. The enclosing ignored integration continues to use its existing
fixed synthetic ACP/MCP peer in a separate temporary home.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the pre-existing dirty and untracked work preserved. The NUL-delimited Git
status SHA-256 at freeze is
`416006f11e89e73e4d6886f925b2dd3837d14721c556af6368ae9106f95a7ba0`.

| File | SHA-256 |
|---|---|
| `crates/outcomes-cli/tests/cli_compat.rs` | `3f0a86eb70eab4d9250e0aaa57e5679c66cdb6f7b6234f4abd870cba95754465` |
| `crates/outcomes-cli/src/lib.rs` (read-only) | `9367ccb2e2c1ac1d5d968e32d8e932938f97c8d82c6bee5903aed88db919354c` |
| `crates/hyperai-local-service/src/server.rs` (read-only) | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `scripts/install-hyperai-local.mjs` (read-only) | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` |
| `Cargo.lock` (read-only) | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `Cargo.toml` (read-only) | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `crates/outcomes-cli/Cargo.toml` (read-only) | `6b733497c27e1d59a525b1c1ac8ee2958567c230990a406462ad03a106c99648` |
| `docs/compute-bazaar-installed-cli-lifecycle-packet.md` | `8f6a062fe47e6f417779466e715e7fe87285fcce09584f40ac5fa4db77cc4599` |
| `docs/compute-bazaar-phase-1-contract-packet.md` | `bae05fbeaed2431322aba4207fe4f23867c485158b38d7c5fdbdd40e74263993` |
| `docs/compute-bazaar-rust-port-subplan.md` | `3c29118d9d57ce16b73f3f05366fe85f7965918febda1ed0e629b351eea8c356` |
| `docs/bazaar-hyperai-interface-plan.md` | `95499d28c92c102b8ea19f7e818a575124dd9016bd959fd8460285bd64146104` |
| `docs/state-slices.md` | `682ec9dbf74fa049fe7b3517ec067472de39e93e1f0d85ce510794e022e4d61a` |

## Exact writable files

- `crates/outcomes-cli/tests/cli_compat.rs`: reuse the quote command helpers
  with an explicit executable path and invoke them from the installed CLI
  lifecycle while its disposable service is ready.
- This packet and `docs/state-slices.md`.
- `docs/compute-bazaar-installed-cli-lifecycle-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: link the qualification result after
  the gates pass.

Read-only inputs are the existing CLI implementation, service routes,
installer, Rust manifests/lockfile, and installed-source lifecycle behavior.
No package contents, production source, API schema, dependency, permission, or
service behavior changes.

## Allowed effects and non-goals

Allowed effects are the exact ignored Unix source-install integration below,
which builds locked binaries offline, installs them under a disposable root,
and operates only on temporary homes and IPv4 loopback port 0; root
`lint:fast`; pinned formatting; `git diff --check`; and scoped whitespace
inspection. The test's existing compatibility-entrypoint, installed TUI PTY,
and synthetic ACP/MCP checks remain part of the frozen enclosing integration.

No provider, seller, registry, remote agent, real workload, paid execution,
live Fleet, financial Trade, wallet, chain, external endpoint, user home,
publication, or deployment is in scope. Do not run the full root gate known to
be blocked by the recorded storage/HomeBusy condition.

## Proof gates and claim ceiling

1. The installed binary runs `market compare` and passes the returned digest to
   `market quote-preview`.
2. A stale preview digest is rejected with `QUOTE_PREVIEW_CHANGED`; the exact
   nested quote-preview digest succeeds through `market quote-approve`.
3. A stale deal digest is rejected with `QUOTE_APPROVAL_CHANGED`; the returned
   deal digest succeeds through `market quote-complete`.
4. The completion binds the preview and deal digests to its report and receipt
   and retains the synthetic no-network/no-funds effect boundary.
5. The enclosing installed lifecycle stops its service, the source installer
   and homes remain disposable, and its existing ACP/MCP and compatibility
   checks pass.

Maximum claim: installed CLI buyer-quote lifecycle behavior on the tested
local host with fixed HyperAI synthetic records. This proves no seller intent,
provider contact, compute execution, independent verification, payment, or
settlement. Native platform support, broad accessibility/usability, and
release readiness remain unqualified.

## Validation report — 2026-10-02

The exact locked offline ignored source-install integration passed `1/1` on
Darwin arm64:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The installed CLI compared the fixed supply, previewed the selected quote,
rejected a stale preview (`QUOTE_PREVIEW_CHANGED`), approved the exact nested
quote digest, rejected a stale deal digest (`QUOTE_APPROVAL_CHANGED`), and
completed with the returned deal digest and mock receipt. The receipt retained
its digest and no-network/no-funds effect boundary. The enclosing test also
passed its installed lifecycle and compatibility checks, TUI PTY resize and
restoration, synthetic ACP/MCP session and revocation, and temporary-state
cleanup. The existing service on port 4317 was never targeted; the test service
used port 0.

The direct non-installed CLI quote test passed `1/1`. Root `lint:fast` passed
34 interface tests, 3 installer tests, pinned Rust formatting, and strict
all-target/all-feature workspace Clippy. `git diff --check` passed, and scoped
whitespace inspection found no trailing whitespace. The exact modified test
source SHA-256 is
`b2c2fed0bd56172cee276c28a7e6c6d2382ffee861f2ea47e77c05928f3ae262`.

Assurance is `INTERNAL_PASS` for the installed fixed local synthetic CLI path.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED` because
this exact integration ran in one environment. Custody is limited to fixed
HyperAI synthetic data and disposable install/home directories. This does not
establish native platform support, seller/provider contact, real execution,
independent verification, payment, release readiness, or production
authorization. Publish boundary: local code and documentation only; no
staging, commit, PR, release, publication, or deployment.
