# Workload Trace Denied-State Linux Reproduction Packet

Status: completed on 2026-09-28 under the current Rust-port subplan.

State slice: `hyperai.bazaar.workload_trace_denied_linux_reproduction.v1`.
Parent: [Compute Bazaar Rust port subplan](compute-bazaar-rust-port-subplan.md).
Prior host evidence: [denied-state client parity](compute-bazaar-workload-trace-client-denied-parity-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Preserve all unrelated work.

## Purpose and boundary

Reproduce the existing synthetic filesystem-denied checks for the authenticated
owner route, CLI, TUI, and MCP adapter in a fresh offline Linux/amd64 Docker
target. The three selected tests directly print whether removing read bits
caused an OS `PermissionDenied`; they assert the corresponding projection and
restore the file mode. The run makes no product or test-code changes.

Included source inputs are root `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, all files in `crates/`, `apps/`, `config/`,
`fixtures/`, `migrations/`, and `schemas/`, `docs/openapi.json`, and the exact
committed `products/hyperintel/` snapshot required by
`hyperai-hyperintel-adapter`. The additional root inputs are compile-time
inclusions referenced by the selected workspace packages. Exclude `.git`,
`target`, `references/`, `products/justai/`, Simple Compute Market source,
Bazaar source/data, all other docs, generated outputs, local data, credentials,
and unrelated ignored files. Freeze a sorted relative-path, byte-length, and
SHA-256 manifest before execution. Mount that snapshot and the Cargo cache
read-only; use a fresh writable target.

Frozen input: 270 files, 3,942,939 bytes; the canonical serialized manifest
SHA-256 is `a455b825a1830c3d9deda448cc7de5958f2153e7d62e269ed9675fce2098787e`.
The current `Cargo.lock` SHA-256 is
`b3708a2e0577a3e3d4a07c9110b33ae0ff0676b7aaaf495cf4278ba46e7e6736`.
The included HyperIntel snapshot is 99 files at its verified import digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.

The container uses the pre-existing
`rust:1.93.1-bookworm` image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
`--platform linux/amd64`, `--network none`, `--cap-drop=ALL`, one Cargo job,
`--locked --offline`, and
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu` with
`RUSTUP_AUTO_INSTALL=0`. Docker Desktop runs on this Apple Silicon host, so
this is same-owner container reproduction, not a separate host or native x86
hardware result.

## Exact checks

Run each command with `-- --exact --nocapture`:

```sh
cargo test --locked --offline --jobs 1 -p outcomes-cli --test cli_compat \
  market_trace_cli_matches_the_authenticated_service_projection -- --exact --nocapture
cargo test --locked --offline --jobs 1 -p hyperai-tui --test terminal_restore \
  workload_trace_tui_renders_the_authenticated_service_projection -- --exact --nocapture
cargo test --locked --offline --jobs 1 -p hyperai-mcp \
  tests::workload_trace_mcp_matches_changed_data_owner_route -- --exact --nocapture
```

Capture the direct permission-probe lines and each test result. A denied-state
reproduction claim requires all three probes to print `PermissionDenied` and
all three tests to pass. If any probe reports that the file remained readable,
record that test's denied-state result as `NOT_EXERCISED`; do not infer a
denied-state result from a passing fallback assertion.

## Allowed effects and exclusions

Allowed effects: a temporary manifest-bound source snapshot; read-only mounts
for source and Cargo cache; one fresh Cargo target; offline compilation and
the three named tests with HyperAI-authored synthetic files and test-owned
loopback services inside the network-isolated container; temporary logs whose
digests are recorded before cleanup.

No source, product, fixture, lockfile, or imported snapshot edits; no external
network, host service access, provider, model, seller, registry, federation,
third-party agent, paid execution, live Fleet, wallet, chain, funds,
publication, deployment, or production operation. Do not modify or contact the
existing host listener on `127.0.0.1:4317`.

## Proof gates

- Verify the HyperIntel import against the recorded provenance before and
  after the container run.
- Freeze and recheck the exact source manifest, `Cargo.lock`, and Docker image
  identity.
- Run all three exact commands offline with no capabilities and no network;
  retain test counts, direct permission probes, and output SHA-256 values.
- Confirm the source checkout and imported snapshot are unchanged, temporary
  services stopped, and all staged source, target, logs, and manifest data are
  removed after hashing.
- Run `corepack pnpm run lint`, `python3 scripts/verify_imports.py`, and
  `git diff --check` for the documentation update; preserve unrelated work.

## Local execution record — 2026-09-28

The manifest-bound 270-file source snapshot was rechecked against the selected
current workspace inputs before and after execution: 0 missing, 0 extra, and 0
changed files. The pinned HyperIntel import remained 99 files at digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`. Docker
used image
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
which reported rustc `1.93.1 (01f6ddf75 2026-02-11)` and Cargo
`1.93.1 (083ac5135 2025-12-15)`.

All three commands passed 1/1. The CLI, TUI, and MCP direct OS permission
probes each printed `PermissionDenied`. The container ran with `--network
none`, `--cap-drop=ALL`, read-only source and Cargo-cache mounts, and a fresh
target. It was removed after completion. No container remained, and the
pre-existing host listener on `127.0.0.1:4317` remained PID 85094. The combined
test-output SHA-256 is
`e862d076ebd5f361fc594ad65c4f89be8b436769a2f36e0ef38ff1adb67cc74e`.

Two setup attempts before this result did not produce test results: the first
used a temporary path resolving through the mounted Backup ED archive and was
stopped before tests; the second shell lacked Docker in its `PATH`. The final
run used a temporary directory on the repository's local disk and the explicit
Docker executable. These setup attempts do not change the test result above.

The run establishes `REPRODUCED_LOCAL` only for these exact source inputs and
three focused tests, using Docker amd64 emulation on the same Apple Silicon
host. It is not native Linux or separate-host evidence. Reviewer status is
`NOT_REQUESTED`; reproducibility is `REPRODUCED_LOCAL`. After this record was
added, `corepack pnpm run lint` passed with exit 0. The pinned import verifier,
`git diff --check`, and scoped documentation trailing-whitespace check passed.

## Claim ceiling

Maximum claim: `REPRODUCED_LOCAL` for the exact frozen inputs and named tests
only; each direct probe reported `PermissionDenied` and each test passed.
The reproduction is Docker emulation on the same Apple Silicon host. It does
not establish native Linux or x86 hardware, a separate-owner environment,
cross-platform permission behavior, full accessibility, complete
browser/CLI/TUI/ACP policy parity, external behavior, independent review,
production readiness, or authority for external effects. Reviewer status is
`NOT_REQUESTED`. Custody is local checked-out HyperAI source, the pinned
HyperIntel snapshot, read-only Cargo cache, and temporary synthetic fixtures.
Publish boundary: local documentation and evidence only; no commit, PR,
release, external circulation, or deployment.
