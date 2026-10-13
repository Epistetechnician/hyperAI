# HyperAI Local Installer Transaction Packet

State slice: `hyperai.bazaar.desk_runtime.install_bundle_transaction.v1`.
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md),
[installed CLI lifecycle packet](compute-bazaar-installed-cli-lifecycle-packet.md),
and [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

The local installer currently runs the MCP sidecar and CLI package installs
directly into the requested destination. Stage the complete four-binary bundle
first, validate it, and then promote the dedicated install root. A Cargo
failure must leave an existing bundle untouched; a failed directory promotion
must restore the prior bundle when possible and preserve its backup if
recovery itself fails.

This is a local source-installer transaction, not a signed release updater.
The HyperAI data home stays outside the install root and is never modified by
an install or upgrade.

## Exact files

- `scripts/install-hyperai-local.mjs`: build into a sibling staging root,
  validate all product binaries, promote as one dedicated directory, and
  restore the prior root on promotion failure.
- `scripts/tests/install-hyperai-local.test.mjs`: hermetic fake-Cargo tests
  for initial install, successful replacement, failed second-package build,
  promotion rollback, cleanup, and data-home preservation.
- `package.json`: expose the installer transaction test and include it in the
  fast root gate.
- `docs/compute-bazaar-local-installer-transaction-packet.md`: scope and
  evidence.
- `docs/state-slices.md`: accepted state slice, custody, assurance, and claim
  ceiling.
- `docs/compute-bazaar-phase-1-contract-packet.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: report the result and retain the
  remaining native-platform and release gates.

## Allowed effects and boundaries

Run tests with a fake Cargo executable and a disposable temporary parent
directory. Run the existing offline source-install integration into its
temporary root after the focused installer test. The installer may create a
staging directory and a temporary rollback directory beside the explicitly
selected dedicated install root. It may replace that root only after all four
expected binaries have been built and validated.

Do not use the user's HyperAI home, the existing install root, the service on
`127.0.0.1:4317`, providers, sellers, registries, external agents, models,
remote workers, paid execution, live Fleet, financial Trade, wallets, chains,
publication, or deployment. Do not modify runtime APIs, binary contents,
source dependencies, product workspaces, or imported source trees.

## Proof gates and claim ceiling

- Verify a first staged bundle contains `hyperai`, `hsai`, `hyperai-ui`, and
  `hyperai-mcp` from the same fake source version.
- Verify a successful second install promotes all four binaries together and
  leaves an unrelated synthetic data-home sentinel byte-identical.
- Inject failure in the second staged package and verify every prior binary
  remains byte-identical and no staging directory remains.
- Inject a staged-directory promotion failure and verify the previous bundle
  is restored; if rollback fails, retain the backup and report its location.
- Run the exact offline installed-binary integration, root lint, pinned Rust
  formatting, scoped whitespace/link checks, and `git diff --check`.
- Confirm the test-owned install roots are removed and the existing listener
  on `127.0.0.1:4317` remains untouched.

Maximum claim: local installer preserves or replaces a complete four-binary
bundle across the tested fake-Cargo failure paths and the tested macOS source
install. This does not establish crash-atomic replacement, native Linux or
Windows behavior, installer signing, public upgrade infrastructure,
independent review, release readiness, or production authorization. Reviewer
status is `NOT_REQUESTED`; reproducibility remains `NOT_REPRODUCED` until a
separate environment runs the exact tests.

No commit, PR, release, external circulation, or deployment is authorized.

## Local evidence — 2026-09-29

`corepack pnpm run verify:installer` passed all three hermetic tests: first
install plus replacement, second-package failure preserving the old bundle
and synthetic data-home sentinel, promotion failure restoring the old root,
and rollback failure retaining and reporting the recovery backup.

The locked offline installed-binary integration also passed:

```sh
sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli \
  --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer -- \
  --ignored --exact --nocapture
```

`corepack pnpm run lint` completed successfully, including interface tests,
installer tests, pinned formatting, strict Clippy, workspace contract tests,
full workspace tests, and doc tests. A separate pinned-format check and
`git diff --check` passed. The source import verifier passed Simple Compute
Market and HyperIntel; it returned a known JustAI inventory mismatch for an
extra `products/justai/pnpm-lock.yaml`, which remains untouched.

Host: macOS 26.6.1 arm64; pinned Rust/Cargo 1.93.1; Node 22.23.2; pnpm 11.7.0;
Python 3.14.5. The fake-Cargo fixtures used only `v1`/`v2` labels and a
synthetic data-home sentinel; test-owned temporary directories were cleaned
after each test. The installed-binary integration used its disposable install
root and home. No user data or external service was used. The existing local
service remained at `127.0.0.1:4317` under PID 85094.

Scoped source digests at completion:

| File | SHA-256 |
|---|---|
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` |
| `scripts/tests/install-hyperai-local.test.mjs` | `44a22ac5ea0884a04325888f9fe9c41f01a848966204b37d08e3652ab9777acb` |
| `package.json` | `286700e6aaab8986a096a97678bcfa0579631768b2fcafa42b52d6ee741293c3` |

Assurance is `INTERNAL_PASS` for these local transaction and source-install
paths only. Reviewer status remains `NOT_REQUESTED`; reproduction remains
`NOT_REPRODUCED`. Native Linux, Windows, Intel macOS, process-crash recovery,
signed distribution, and public upgrade behavior remain outside the evidence.
Next gates are native-platform install/upgrade/rollback qualification and the
separately authorized signing/release review.
