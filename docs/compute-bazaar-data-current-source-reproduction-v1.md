# HyperAI Compute Desk: Current-Source Data Reproduction

Status: completed bounded local reproduction packet. It records no Data
implementation changes and does not establish overall Phase 2 acceptance.

State slice: `hyperai.bazaar.data_current_source_reproduction.v1`.
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Program plan: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Why this continuation exists

The existing Linux/amd64 package reproduction in the Phase 2 packet used a
107-file Rust source manifest with SHA-256
`67c66bb71c96e3630402c2f02a110e6640e58527550f6f7da7e63d7c7f9b397b` from
2026-09-29. The present source is a different 226-file manifest. Re-running a
small fixed package set against this exact source answers whether the Data
crate, authenticated Data API integration, and TUI library tests still pass in
a disposable Linux/amd64 environment. It is not a substitute for native
platform qualification, a separate-owner reproduction, or the unassigned
Data-worker phase.

## Frozen pre-run identity

- HyperAI revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`.
- Read-only source manifest: 226 regular files, 4,317,298 bytes,
  SHA-256 `c4a4a31e870b12a53021aa7b838e2019a08245b029f25b87cd8dbc22c1c703a0`.
- Source manifest inputs: root `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, regular files beneath `.cargo/`, `crates/`, `apps/`,
  and `products/hyperintel/`; excludes `.git`, build/cache directories, and
  symlinks. Each sorted entry binds its relative path, byte length, and raw
  SHA-256 digest.
- Pre-existing worktree: 171 status entries; NUL-delimited status SHA-256
  `dd99e14e088d40c8fb744a4f5354e25a4ec1f4af6d3368e144167de940a108a3`.
- Pre-test unrelated-worktree identity after the scoped documentation files
  were added: 2,696 NUL-delimited records from
  `git status --porcelain=v1 --untracked-files=all -z`, excluding exactly the
  four writable paths below, SHA-256
  `e9fa4a2d378b5ce2fce178d4ece5e60e3198377cdf2105e901c36db493656cac`.
  Preserve status-record order and the final NUL when hashing.
- Docker host: Linux/aarch64 VM on the same owner's Apple Silicon host; 14
  CPUs and 12,525,842,432 reported bytes of memory. The cached image is
  `rust:1.93.1-bookworm`, `linux/amd64`, image ID
  `sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
  repository digest
  `rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
- The pre-run Docker container list was empty. No host listener was present on
  `127.0.0.1:4317` at preflight. The container has no network and no published
  ports; tests may use only their own container loopback sockets.

## Exact writable files

- `docs/compute-bazaar-data-current-source-reproduction-v1.md`: this scope
  and the exact result.
- `docs/state-slices.md`: this authorization and result.
- `docs/compute-bazaar-phase-2-data-packet.md`: link this package-specific
  reproduction and preserve the overall Phase 2 status.
- `docs/compute-bazaar-rust-port-subplan.md`: concise current-position update.

The three existing planning files are untracked in the starting worktree.
Changes must be append-only or narrowly additive; preserve their existing
bytes. No Rust, JavaScript, fixture, manifest, dependency, or import-tree file
is writable in this slice.

## Allowed effects and exact checks

Run only the following locked/offline Rust tests, in one disposable
`linux/amd64` container:

```sh
cargo test --locked --offline -p hyperai-data --all-targets
cargo test --locked --offline -p hyperai-local-service --test data_api
cargo test --locked --offline -p hyperai-tui --lib
```

The invocation must use `--pull=never`, `--platform linux/amd64`,
`--network=none`, `--cpus=4`, and `--memory=10g`; bind the checkout read-only;
mount the host Cargo registry and Git caches read-only, copying them into the
container's disposable `/tmp/cargo` before running Cargo; set
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu` and
`CARGO_TARGET_DIR=/tmp/target`; and delete the container with `--rm`. The
container must print `rustc --version` and `cargo --version`. The Cargo cache,
build output, and combined test log must not be written into the checkout.
Bound the run to 25 minutes using GNU `timeout` with a 1,500-second deadline
and 15-second kill grace. Keep the full log only in the disposable container
at `/tmp/hyperai-data-current-source-tests.log`, report each `test result:`
line and the combined log SHA-256, and retain no fixture or raw test output in
the repository.

Reproduction wrapper used for this slice:

```sh
set -o pipefail
HYPERAI_CARGO_CACHE="$HOME/.cargo"
docker run --rm --pull=never --platform linux/amd64 --network=none --cpus=4 --memory=10g \
  --mount "type=bind,src=$PWD,dst=/src,readonly" \
  --mount "type=bind,src=$HYPERAI_CARGO_CACHE/registry,dst=/cache/registry,readonly" \
  --mount "type=bind,src=$HYPERAI_CARGO_CACHE/git,dst=/cache/git,readonly" \
  --workdir /src \
  --env RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu \
  --entrypoint /usr/bin/timeout \
  rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31 \
  --signal=TERM --kill-after=15s 1500s \
  /bin/bash -euo pipefail -c 'set -euo pipefail
mkdir -p /tmp/cargo
cp -a /cache/registry /tmp/cargo/registry
cp -a /cache/git /tmp/cargo/git
export CARGO_HOME=/tmp/cargo
export CARGO_TARGET_DIR=/tmp/target
export CARGO_NET_OFFLINE=true
rustc --version
cargo --version
: > /tmp/hyperai-data-current-source-tests.log
cargo test --locked --offline -p hyperai-data --all-targets 2>&1 | tee -a /tmp/hyperai-data-current-source-tests.log
cargo test --locked --offline -p hyperai-local-service --test data_api 2>&1 | tee -a /tmp/hyperai-data-current-source-tests.log
cargo test --locked --offline -p hyperai-tui --lib 2>&1 | tee -a /tmp/hyperai-data-current-source-tests.log
grep "test result:" /tmp/hyperai-data-current-source-tests.log
sha256sum /tmp/hyperai-data-current-source-tests.log'
```

Use only HyperAI-authored synthetic fixtures generated by the selected tests
and the pinned committed HyperIntel source snapshot required to compile the
local service. Do not invoke the host service, external providers, Bazaar,
Simple Compute Market, JustAI, any browser, or any network source. Do not run
root `verify:full` or `pnpm run lint` under the active root-focused-test slice.

## Proof and stop gates

Before and after execution, recompute the source manifest and Git status. The
source manifest must match exactly. After excluding only the four writable
documentation paths above from `git status --porcelain=v1
--untracked-files=all -z`, the ordered NUL-delimited status identity must match
the frozen pre-test digest above; the existing documentation prefixes must
remain byte-identical. Check that the disposable
container is gone and no test-owned process or temporary directory remains.

Stop without retry if the image digest, source manifest, initial unrelated
worktree identity, or offline caches differ; if Docker cannot enforce the
specified isolation and limits; or if any test fails, times out, or attempts
network access. Preserve the literal result and diagnose in a new slice
before changing commands, fixtures, source, or resource limits.

## Execution result (2026-10-02)

Rust was `1.93.1 (01f6ddf75 2026-02-11)` and Cargo was
`1.93.1 (083ac5135 2025-12-15)`. All six test-result groups passed:

- `hyperai-data --all-targets`: 39 + 6 + 2 + 15 = 62 passed.
- `hyperai-local-service --test data_api`: 7 passed.
- `hyperai-tui --lib`: 24 passed.
- Total: 93 passed, 0 failed, 0 ignored.

The combined in-container test log SHA-256 is
`4ce09f3560bf948d8377375a8b960f5612a40debc2958d4d6ccd335c49a2ed5c`.
The post-run source manifest remained 226 files / 4,317,298 bytes with SHA-256
`c4a4a31e870b12a53021aa7b838e2019a08245b029f25b87cd8dbc22c1c703a0`. The
2,696 ordered status records outside the four writable documentation paths
retained SHA-256
`e9fa4a2d378b5ce2fce178d4ece5e60e3198377cdf2105e901c36db493656cac`; the
original bytes of the existing planning records were preserved. Docker
removed the container; a post-run image-filtered container listing was empty,
and port `127.0.0.1:4317` remained unused. No test log or build output was
written into the checkout or another host file; the container-local copy was
removed with the container.

## Claim ceiling and reporting

Assurance is `REPRODUCED_LOCAL` for these exact package commands and source
manifest only. This is same-owner emulated Linux/amd64 reproduction, not
native Linux qualification, separate-owner reproduction, or
`INDEPENDENT_ACCEPT`. Reviewer status is `NOT_REQUESTED`. The result does not
establish SQL security, an OS-enforced query memory ceiling, source
authenticity, Data-worker isolation, overall Phase 2 acceptance, production
readiness, or any external data/provider path. Data and artifacts were
HyperAI-authored synthetic fixtures and temporary output held inside the
disposable container. Overall Phase 2 remains `NOT_ASSIGNED`.
