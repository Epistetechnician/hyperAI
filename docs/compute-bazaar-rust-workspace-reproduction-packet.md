# Rust Workspace Linux Reproduction Packet

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v1`; a
current-source continuation is recorded below under
`hyperai.bazaar.rust_port_workspace_reproduction.v2`.
Parent: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. The source snapshot is defined by
`Cargo.toml`, `Cargo.lock`, every regular file under `.cargo/` when present,
every regular file under `crates/`, and the local path and compile-time inputs
listed in each continuation; the packet records a deterministic
content-manifest digest before execution.

## Source-scope correction (2026-09-29)

The v1-v4 manifests covered root Cargo files and `crates/`, but omitted local
path dependencies under `products/hyperintel/crates/` and compile-time files
referenced outside `crates/` (including interface assets, configuration,
fixtures, documentation, and migrations). A v5 pre-test build exposed this
gap by compiling `hyperintel-execution-journal` from the imported HyperIntel
tree. The v1-v4 test totals remain historical local execution evidence, but
their exact-source `REPRODUCED_LOCAL` claims are withdrawn; treat them as
`INTERNAL_PASS` for the recorded command results only. The v5 continuation
uses an expanded source manifest and does not retroactively alter those runs.

Pre-run source manifest: 107 regular files, 2,236,945 bytes, SHA-256
`0a7245238f140240cdf6aa51e0a2770d72cdc26bc13a872cec4843a22bbd428c`.
The pre-run `git status --porcelain=v1 --untracked-files=all` snapshot had
2,588 entries and SHA-256
`2b0442ef8b80ff34cbda8022f3ba24efd1a97585ef3872eaf3720d6487cf007b`.

## Purpose

Reproduce the complete root Rust workspace test suite in a clean, offline
Linux/amd64 container using the repository's pinned Rust toolchain. This is a
same-owner second environment on the existing Apple Silicon host. It provides
broader Rust evidence than the existing package-specific container runs; it
does not qualify JavaScript, browser behavior, native Linux hardware, or a
separate host.

## Frozen environment and command

- Container image: local `rust:1.93.1-bookworm`, Linux/amd64, image ID
  `sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`;
  recorded repository digest:
  `rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
- Toolchain: Rust/Cargo 1.93.1, verified inside the container before tests.
- Isolation: `docker run --rm --network none --platform linux/amd64`; mount
  the workspace and host Cargo caches read-only, copy caches into the
  disposable container, and direct Cargo build outputs to `/tmp/target`.
- Set `RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu` to use the image's
  already-installed toolchain without a rustup channel refresh. The exact
  override was preflighted from the read-only workspace with networking off.
- Resource budget: Docker daemon reports 14 CPUs and 12,525,842,432 bytes;
  limit the container to 10 GiB and four Cargo build jobs.
- Command: `cargo test --locked --offline --workspace --all-features`.
- No source files, imported repositories, datasets, providers, services,
  external reviewer, or network connections are inputs to this run.

## Proof gates

1. Record the deterministic source-manifest file count, byte total, and SHA-256.
2. Confirm the container image identity, architecture, Rust/Cargo versions,
   network isolation, and read-only source mount.
3. Run the exact locked offline workspace command once; retain its exit status,
   sanitized output digest, and test totals.
4. Confirm no source changes, no container remains, and the pre-existing
   HyperAI service on `127.0.0.1:4317` remains running.
5. Run `git diff --check` for the documentation update.

## Claim ceiling and limitations

At most, this can establish `REPRODUCED_LOCAL` for the exact Rust workspace
test command and source manifest in a clean Linux/amd64 container on the same
owner's Apple Silicon host. It does not establish independent review, a
separate-host reproduction, native Linux support, Windows/macOS parity,
JavaScript/browser parity, accessibility, external source rights, live
provider behavior, paid execution, settlement, or production readiness.
Reviewer status is `NOT_REQUESTED`. External operations remain closed under
their existing state slices. Temporary Cargo caches, build output, and test
data are destroyed with the container.

## Result

On 2026-09-28, the exact command completed in the frozen container:
`cargo test --locked --offline --workspace --all-features`. It ran 79 test
suites: 372 passed, 0 failed, and 9 ignored. The SHA-256 of the captured test
log was
`35fc3bf1a0860c37041ed6762e8645f1551c4b0f2a5b18eff726f74acc9474bf`.

The pre/post source manifest remained 107 files / 2,236,945 bytes with SHA-256
`0a7245238f140240cdf6aa51e0a2770d72cdc26bc13a872cec4843a22bbd428c`. The
pre/post Git status snapshot remained 2,588 entries with SHA-256
`2b0442ef8b80ff34cbda8022f3ba24efd1a97585ef3872eaf3720d6487cf007b`. The
container was removed, the workspace stayed read-only, and the existing
HyperAI service remained PID 85094 on `127.0.0.1:4317`. `git diff --check` and
the exact packet/state-slice trailing-whitespace check passed.

Assurance is `INTERNAL_PASS` for this recorded local command execution only;
the source manifest omitted local HyperIntel path packages and external
compile-time inputs, so exact-source reproduction was not established.
Reviewer status is `NOT_REQUESTED`. Native platform support, JavaScript and
browser reproduction, full accessibility, and clean separate-host
reproduction remain open; this result does not qualify the full roadmap.

An initial no-test invocation attempted a rustup channel refresh because the
workspace pins `1.93.1`; networking correctly prevented that refresh before
any Cargo test began. The explicit installed-toolchain override above passed
the offline preflight (`rustc` and `cargo` both reported 1.93.1) before the
successful run.

## Current-source continuation (2026-09-29)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v2`. This
append-only run preserves the completed v1 evidence above and uses the current
dirty source snapshot.

Pre-run Rust source manifest: 107 regular files / 2,259,690 bytes, SHA-256
`acdb453a9e6fe258bfbe49b85f851eefc30815c52da8a63940788fd8fd9ed7d1`. The
manifest includes `Cargo.toml`, `Cargo.lock`, and regular files under
`crates/`; there is no `.cargo/` directory. It sorts relative paths and hashes
path length, path bytes, content length, and the SHA-256 of each file, using
big-endian 64-bit length prefixes. Pre-run Git status contains 2,596 entries
with SHA-256
`bed78c12e4aa0af4f302f8fec4c8155d0dfadf6fb281b7fad0d55e9d5c8fb1a0`.

Environment preflight: Docker `29.7.2` on Linux/arm64 reports 14 CPUs and
12,525,842,432 bytes of memory. The local `rust:1.93.1-bookworm` image is
Linux/amd64 with image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`;
the recorded repository digest remains
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The host Cargo registry and Git caches are mounted read-only and copied into
the disposable container. The container has networking disabled, four Cargo
jobs, a 10 GiB memory cap, a read-only workspace mount, and its target
directory under `/tmp/target`. The existing service on `127.0.0.1:4317` was
verified as PID 85094 before the run.

Exact command: `cargo test --locked --offline --workspace --all-features`,
with `RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`. Inside the container,
`rustc` reported `1.93.1 (01f6ddf75 2026-02-11)` and Cargo reported
`1.93.1 (083ac5135 2025-12-15)`. The command exited 0: 79 suites, 377 passed,
0 failed, and 9 ignored. The captured test-log SHA-256 is
`ccb874f6b0d2f6a7c0d06e2ef3017da400758c67fe039d0745cdba25af5d320c`.

Post-run source manifest: 107 regular files / 2,259,690 bytes, SHA-256
`acdb453a9e6fe258bfbe49b85f851eefc30815c52da8a63940788fd8fd9ed7d1`, identical
to pre-run. Post-run Git status: 2,596 entries, SHA-256
`bed78c12e4aa0af4f302f8fec4c8155d0dfadf6fb281b7fad0d55e9d5c8fb1a0`, identical
to pre-run. The disposable container was removed. PID 85094 remained the owner
of `127.0.0.1:4317`.

Claim ceiling: `INTERNAL_PASS` for the recorded local command execution only.
The source manifest omitted local HyperIntel path packages and external
compile-time inputs, so exact-source `REPRODUCED_LOCAL` was not established.
This does not establish native Linux/Windows, JavaScript/browser,
accessibility, independent review, separate-host, external-source, or
production qualification. Reviewer status is `NOT_REQUESTED`.
Data and artifact custody: the read-only source tree, synthetic tests, and
captured test log stayed local. Cargo caches were copied into the disposable
container; build output and test data were destroyed with it. No external data,
provider, reviewer, or service was contacted.

## Current-source continuation v3 (2026-09-29)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v3`. This
append-only continuation freezes the source after the generic SQL AST-field
rejection change; the v1 and v2 results above remain bound to their original
manifests.

Pre-run Rust source manifest: 107 regular files / 2,263,812 bytes, SHA-256
`d6b5dc6cdea7f89d79c24dd18508e438baee395c8cc43376e04f42b3fa782a35`. It
covers `Cargo.toml`, `Cargo.lock`, and regular files under `crates/`, sorted
by POSIX relative path and hashed with big-endian length prefixes over each
path, content length, and content SHA-256. No `.cargo/` directory exists.
Pre-run Git status contains 2,597 entries, SHA-256
`f7cb922953b89b8b6f9f4a4e9ba0ecc74954fc6a6bbad84b6109618acaec40c7`.

Frozen environment: local `rust:1.93.1-bookworm`, Linux/amd64, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The host is macOS arm64 with pinned Rust/Cargo `1.93.1`; the Docker daemon
reports 14 CPUs and 12,525,842,432 bytes of memory. The run used four jobs
and a 10 GiB cap, `--network none`, a read-only source mount, and
read-only Cargo registry/Git cache mounts copied into the disposable
container. The target directory and logs remain under `/tmp` inside it.

Exact command: `cargo test --locked --offline --workspace --all-features`,
with `RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`. Preflight confirmed
the exact image is local, no container is active, and the pre-existing
service remains PID 85094 on `127.0.0.1:4317`.

Proof gates passed: the pinned toolchain was verified inside the container;
the command exited 0; test totals and captured test-log SHA-256 were retained;
the pre/post source manifest and Git status matched; the disposable container
was removed; and the existing service remained on port 4317.

Bootstrap receipt (no Cargo tests): the first container invocation used
`bash -lc`, whose login-shell startup replaced the image PATH, so the preflight
failed with `rustc: command not found` (exit 127) before Cargo ran. The
`--rm` container was removed, the mounted source was read-only, and no test
started. Image inspection confirmed the pinned toolchain is installed at
`/usr/local/rustup/toolchains/1.93.1-x86_64-unknown-linux-gnu`; a non-login
`bash -c` preserves the image PATH and resolves its `rustc`, `cargo`, and
`rustup` binaries. The corrected non-login preflight and exact workspace test
command then completed as recorded below.

Claim ceiling: `INTERNAL_PASS` for the recorded local command execution only.
The source manifest omitted local HyperIntel path packages and external
compile-time inputs, so exact-source `REPRODUCED_LOCAL` was not established.
It cannot establish native Linux behavior, Windows/macOS parity, JavaScript/browser behavior,
accessibility, independent review, separate-host reproduction, external
source rights, or production readiness. Reviewer status is `NOT_REQUESTED`.

Execution result: the exact command exited 0 and ran 79 test-result groups:
380 passed, 0 failed, and 9 ignored. Rust reported
`1.93.1 (01f6ddf75 2026-02-11)` and Cargo reported
`1.93.1 (083ac5135 2025-12-15)`. SHA-256 of the captured Cargo output log:
`66ad024b9eeda28dfe060a5be81adca8382e1f294c4bc8bc0f61fb864c8fb116`.
A Docker stats sample during linking showed 9.636 GiB of the 10 GiB cap; the
run continued without OOM.

Post-run Rust source manifest: 107 files / 2,263,812 bytes, SHA-256
`d6b5dc6cdea7f89d79c24dd18508e438baee395c8cc43376e04f42b3fa782a35`, matching
pre-run. Post-run Git status: 2,597 entries, SHA-256
`f7cb922953b89b8b6f9f4a4e9ba0ecc74954fc6a6bbad84b6109618acaec40c7`, matching
pre-run. The disposable container is removed and PID 85094 still owns
`127.0.0.1:4317`.

Assurance: `INTERNAL_PASS` for this recorded local command execution only;
the source manifest omitted local path and compile-time inputs, so exact-source
reproduction was not established. Reviewer status is `NOT_REQUESTED`.
The run used only read-only local source, fixed synthetic tests, and copied
local Cargo caches; test data and build output were destroyed with the
container. It establishes no native platform, browser/JavaScript,
accessibility, independent, separate-host, source-rights, or production claim.

## Current-source continuation v4 (2026-09-29)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v4`. This
append-only continuation follows v3 and freezes the exact current Rust
workspace after the Data parser alias/function and authenticated API parity
changes.

Pre-run and post-run source manifest: 107 regular files / 2,282,056 bytes,
SHA-256
`d81a57b9431ab50cee09263913faec3402200cd6bbd3bd4062e95855d718f088`. The
manifest covers `Cargo.toml`, `Cargo.lock`, and regular files under `crates/`
(no `.cargo/` directory), sorted by POSIX relative path and hashed with the
packet's length-prefixed path/content digest scheme. Pre-run Git status had
2,597 entries and SHA-256
`f7cb922953b89b8b6f9f4a4e9ba0ecc74954fc6a6bbad84b6109618acaec40c7`; the
post-run count and digest matched exactly.

Environment: the local `rust:1.93.1-bookworm` image was Linux/amd64 with image
ID `sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`.
The Docker daemon was Linux/arm64 with 14 CPUs and 12,525,842,432 bytes of
memory. The container used `--network none`, a read-only `/workspace` mount,
four CPUs, and a 10 GiB memory cap. Host Cargo registry and Git caches were
mounted read-only and copied into disposable container storage. The container
reported Rust `1.93.1 (01f6ddf75 2026-02-11)` and Cargo
`1.93.1 (083ac5135 2025-12-15)`. The existing service was PID 85094 before and
after the run; the Docker container was removed.

Exact command: `cargo test --locked --offline --workspace --all-features`,
with `RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`. It exited 0 and ran
79 test-result groups: 387 passed, 0 failed, and 9 ignored. The captured Cargo
output SHA-256 is
`5a43784a1e9e734db0eb7d4f4cfd037e4b642c4cb12ca2dee3616d3081a7f310`. One
Docker stats sample during the run showed 4.75 GiB resident; this was not a
peak-memory measurement. `git diff --check` passed after recording the result.

Assurance is `INTERNAL_PASS` for this recorded local command execution only;
the source manifest omitted local path and compile-time inputs. Reviewer
status is `NOT_REQUESTED`. Source,
synthetic tests, and the output digest remained local; copied caches, test
data, and build output were destroyed with the container. This does not
establish native platform support, JavaScript/browser parity, accessibility,
independent review, separate-host reproduction, Bazaar reuse rights, or
production readiness. Overall Phase 2 Data acceptance remains
`NOT_ASSIGNED`.

## Current-source continuation v5 (2026-09-29)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v5`. This
append-only continuation targets the current Rust workspace after the v4
source snapshot; the earlier run outcomes remain bound to their original
commands, with the source-scope correction above applied to their assurance.

Exact read-only inputs: root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
`package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, and
`manifests/monorepo-imports.json`; all regular files under root `crates/`,
`apps/hyperai-interface/`, `config/`, `fixtures/`, `migrations/`, and
`scripts/`; `docs/openapi.json`; and the 99-file HyperIntel source import under
`products/hyperintel/` selected by the exclusions in
`manifests/monorepo-imports.json`. These include the HyperIntel workspace and
package manifests, all path-package source, and its fixture/config files.
The import tree matched its recorded 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. The generated
`products/hyperintel/target/` tree (about 1.78 GB) and other excluded build
outputs are not inputs; Cargo output is redirected to `/tmp/target` and the
read-only workspace prevents writes.

Pre-run expanded source manifest: 259 files / 4,068,437 bytes, SHA-256
`a5a3bba65df06ac6223c00407c1db405767e2910ce54db5aa26e896d080b8467`. Its
sorted relative paths are hashed with big-endian 64-bit path and content
lengths plus the SHA-256 of each file. Pre-run Git status contains 2,606
entries, SHA-256
`08036b2caa7448de5cd3eedb77ad7510f7afd5da94c76447e9ca4d5c477e225a`.
Exact documentation outputs: this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

Allowed effects: verify the exact HyperIntel import digest and compute the
sorted, path-and-content-bound source manifest and full Git-status digest;
inspect the already pinned local
`rust:1.93.1-bookworm` image and Cargo caches; run one disposable
`linux/amd64` container with networking disabled, a read-only workspace,
four Cargo jobs, and a 10 GiB memory cap; copy local Cargo caches into the
container; run `cargo test --locked --offline --workspace --all-features` with
the installed `1.93.1-x86_64-unknown-linux-gnu` toolchain; capture a sanitized
test-log digest and totals; remove the container; verify source and Git status
did not change and PID 85094 still owns `127.0.0.1:4317`; then record results
in the three documentation outputs.

Non-goals: source, dependency, or lockfile edits; JavaScript/browser or native
platform tests; external network, source, provider, service, reviewer, wallet,
chain, publication, or deployment activity; and stopping or contacting the
pre-existing service.

Proof gates: exact image and toolchain identity; offline and read-only
container configuration; exact command and exit status; test totals and
captured output digest; matching pre/post source-manifest and Git-status
digests; removed container; preserved service listener; and `git diff --check`.

Claim ceiling: at most `REPRODUCED_LOCAL` for the exact Rust source manifest
and command in same-owner Linux/amd64 emulation on this Apple Silicon host.
Reviewer status is `NOT_REQUESTED`. This does not establish native Linux,
separate-host, JavaScript/browser, accessibility, rights, independent-review,
or production qualification. If preflight cannot preserve these controls,
stop before running the suite and record the specific gate.

Initial attempt (stopped before tests, 2026-09-29): the first v5 invocation
used the incomplete 111-file root-only manifest (2,316,301 bytes; SHA-256
`4ad87090576927977e189050025a2694566a42075c48832f3671b260be85ad56`). Its
build log showed compilation from `products/hyperintel/crates/` while Cargo
had reported no test-result groups. The owned disposable container was stopped
and removed with exit 137; this is an aborted pre-test run, not a test failure
or reproduction. The local log SHA-256 is
`41188e322bc7a2b9db691599c64fc767cf0e0056db8fbfdd4954ac1f3ac543de`. Rust
and Cargo both reported 1.93.1. No repository source changed; the listener on
`127.0.0.1:4317` remained outside the container.

Corrected v5 execution (2026-09-29): the exact command
`cargo test --locked --offline --workspace --all-features` exited 0 in the
read-only Linux/amd64 container. The pinned image ID was
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`;
Rust reported `1.93.1 (01f6ddf75 2026-02-11)` and Cargo
`1.93.1 (083ac5135 2025-12-15)`. The command ran 82 test-result groups: 391
passed, 0 failed, and 10 ignored. Cargo reported 6m15s in the test profile.
Captured output was 64,075 bytes with SHA-256
`4eede42a3b51a7a84dc9f9d32aa572caa5a9ee604c2d5695d247dbac175f3275`.

The expanded source manifest remained 259 files / 4,068,437 bytes with SHA-256
`a5a3bba65df06ac6223c00407c1db405767e2910ce54db5aa26e896d080b8467`. The
HyperIntel import remained 99 files with the pinned tree digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`. The
full Git status changed from 2,606 entries / SHA-256
`08036b2caa7448de5cd3eedb77ad7510f7afd5da94c76447e9ca4d5c477e225a` to 2,607
entries / SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`. The
only added status entry is `?? docs/physical-ai-evidence-platform-prd.md`;
that out-of-scope user document was not opened or changed. Accordingly, the
full-worktree status equality gate did not pass, while every frozen source
input digest did match. The source tree was mounted read-only, Docker used
`--network none`, and the captured Docker stats sample was 8.952 GiB of the
10 GiB cap (not a peak measurement). The `--rm` container was removed; no
container remained; PID 85094 still owned `127.0.0.1:4317`.

Assurance is `REPRODUCED_LOCAL` for the exact 259-file source manifest and
command in same-owner Linux/amd64 emulation. The out-of-scope Git-status change
is recorded above and does not broaden that source-bound claim. Reviewer
status is `NOT_REQUESTED`; this is not independent review, native platform
support, browser/accessibility qualification, separate-host reproduction,
rights clearance, or production authorization. Source, fixture, and output
digests stayed local; Cargo caches were copied into disposable container
storage and its build output and test data were removed with the container.
The captured output digest is recorded above; its temporary host log was
removed after its digest was verified.

## Current-source continuation v6 (2026-09-29)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v6`. This
continuation freezes the current Rust workspace after the Phase 2
`DISTINCT ON` validator and API regressions. It supersedes no earlier run.

Read-only inputs and exclusions are the v5 manifest procedure: root Cargo and
pnpm manifests/lockfiles, import manifest, all regular files under root
`crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`, `migrations/`,
and `scripts/`, `docs/openapi.json`, and the exact 99-file HyperIntel source
import. HyperIntel remains pinned to `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`
with source-tree SHA-256
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
Generated target directories and all unrelated products remain excluded.

The pre-run expanded manifest is 259 files / 4,071,532 bytes, SHA-256
`840010f90ec4cabf654c14e1b2490e50a21f482d2270a78941b13a7c86f5eed7`.
The captured full Git status has 2,607 entries, SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`.
The image is the already pinned local `rust:1.93.1-bookworm`, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
platform `linux/amd64`.

Toolchain bootstrap correction: an image-only preflight using
`sh -lc 'rustc --version && cargo --version && uname -m'` exited 127 with
`rustc: not found`; no tests started, and the `--rm` container exited. Image
inspection confirms `RUST_VERSION=1.93.1` and `PATH=/usr/local/cargo/bin:...`.
The v3 packet records the same login-shell PATH failure and its successful
non-login `bash -c` correction. Five candidate controls were considered:
invoke the tool directly, use non-login `bash -c`, explicitly restore PATH,
set the exact installed `RUSTUP_TOOLCHAIN`, and disable auto-install. The
selected approach uses non-login `bash -c` with
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu` and
`RUSTUP_AUTO_INSTALL=0`; rustup documents that the environment override takes
precedence over `rust-toolchain.toml` and that `RUSTUP_AUTO_INSTALL=0` prevents
automatic installation. See [Docker run](https://docs.docker.com/engine/containers/run/),
[rustup overrides](https://rust-lang.github.io/rustup/overrides.html), and
[rustup environment variables](https://rust-lang.github.io/rustup/environment-variables.html).

## Result

The exact command `cargo test --locked --offline --workspace --all-features`
exited 0 in the frozen `rust:1.93.1-bookworm` image. The container reported
Rust `1.93.1 (01f6ddf75 2026-02-11)` and Cargo
`1.93.1 (083ac5135 2025-12-15)`. It ran 82 test-result groups: 393 passed,
0 failed, 10 ignored, 0 measured, and 0 filtered. Cargo reported a 6m37s
test-profile build. The captured output was 64,339 bytes with SHA-256
`de7beb1ff889f7daba8f690933707a50f9b3a0d52fce3b8cc3751f3166822498`.

The post-run expanded source manifest matched at 259 files / 4,071,532 bytes,
SHA-256 `840010f90ec4cabf654c14e1b2490e50a21f482d2270a78941b13a7c86f5eed7`.
The HyperIntel import remained at 99 files with tree digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` and
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Full Git status remained
2,607 entries with SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`; the
pre-existing dirty and untracked work was preserved. The disposable container
was removed, and PID 85094 remained the listener on `127.0.0.1:4317`.

One Linux-specific compiler warning reports unused `raw_pid` in
`crates/hyperai-acp/tests/synthetic_peer.rs:307`; no out-of-scope source edit
was made. Assurance is `REPRODUCED_LOCAL` for this exact Rust source manifest
and command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`. Source and fixed synthetic fixtures were read-only; copied
Cargo caches, build output, and test data stayed in disposable container
storage. The captured host log was hashed and removed. This does not establish
native platform support, browser/JavaScript parity, independent review,
separate-host reproduction, reuse rights, or production readiness.

`git diff --check` passed. A scoped Markdown check also found no trailing
whitespace and a final newline in each of the three documentation outputs.

Host-shell bootstrap receipt: a separate attempt under non-login `/bin/zsh`
could not resolve the Docker CLI and exited before creating a container. The
configured login shell resolved Docker at `/usr/local/bin/docker`; the failed
host attempt ran no tests and changed no repository state.

Documentation outputs are this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`. No source, dependency, lockfile,
build-graph, JS/browser, native-platform, external-service, provider, or
production action is included. Claim ceiling remains at most
`REPRODUCED_LOCAL` for the exact Rust source manifest and command in same-owner
Linux/amd64 emulation; reviewer status is `NOT_REQUESTED`.

## Current-source continuation v7 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v7`. This is an
append-only continuation after the Data SQL function-argument and selective
function-registry changes; v1-v6 remain bound to their recorded inputs.

Status: pre-run freeze recorded; the v7 container command has not run yet.
Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Do not modify source inputs.

Exact read-only inputs are the v5/v6 expanded source set: root `Cargo.toml`,
`Cargo.lock`, `rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`,
`pnpm-lock.yaml`, and `manifests/monorepo-imports.json`; all regular files
under root `crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`,
`migrations/`, and `scripts/`; `docs/openapi.json`; and the 99-file HyperIntel
source import selected by the exclusions in the import manifest. Exclude
generated targets and unrelated products. Hash sorted POSIX paths, path and
content lengths as big-endian 64-bit integers, and each file's SHA-256.

Pre-run source freeze: 259 files / 4,087,597 bytes, SHA-256
`4009e200ff8f8568b991926075cecbe64c4753846416e10fc1715653798025e9`. The
HyperIntel import has 99 files and matches tree digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Pre-run Git status has
2,607 entries and SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`.

Preflight: Docker `29.7.2` runs on Linux/arm64 with 14 CPUs and
12,525,842,432 bytes of memory. The pinned local
`rust:1.93.1-bookworm` image is Linux/amd64, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The host Cargo registry and Git caches are present at 1.2 GiB and 18 MiB.
No containers were running. The existing listener remains PID 85094 on
`127.0.0.1:4317` and is outside this slice.

Allowed effects: start one disposable Linux/amd64 container with networking
disabled, the workspace and host Cargo caches mounted read-only, cache copies
and target output under disposable container storage, four Cargo jobs, and a
10 GiB memory cap. Use non-login `bash -c`,
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`, and
`RUSTUP_AUTO_INSTALL=0`; run exactly
`cargo test --locked --offline --workspace --all-features` once; capture the
exit status, test totals, and output digest; remove the container; recheck the
source, HyperIntel import, full Git-status digest, and listener; update only
this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

Proof gates: establish image and toolchain identity, Linux/amd64 architecture,
network isolation, read-only source and cache mounts, resource limits, the
exact command and output digest, unchanged pre/post source and HyperIntel
import digests, preserved Git status, container cleanup, and listener
preservation. Record any failure literally; do not retry an identical failed
command blindly.

Claim ceiling: at most `REPRODUCED_LOCAL` for the exact frozen Rust workspace
manifest and command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`; external review is not a gate for this bounded local run.
Reproducibility is not established until all exact-source gates pass. Data and
artifact custody is limited to local source, fixed synthetic tests, disposable
container caches/build output, and a locally hashed temporary test log.
Non-goals include source or lockfile edits, native platform or browser claims,
independent review, separate-host reproduction, upstream/source-rights claims,
providers, network access, paid execution, custody, settlement, publication,
commit, or deployment. Publish boundary is this local evidence and the three
named documentation outputs only.

### Result

Execution (2026-09-30): the exact command
`cargo test --locked --offline --workspace --all-features` exited 0 in the
frozen image. The container reported Rust `1.93.1 (01f6ddf75 2026-02-11)`,
Cargo `1.93.1 (083ac5135 2025-12-15)`, and `x86_64`. It ran 82 test-result
groups: 397 passed, 0 failed, 10 ignored, 0 measured, and 0 filtered. Cargo
reported `Finished test profile` in 10m12s. The 64,623-byte captured output
has SHA-256
`1d8dd725520d89356d1fb759fb7042f7f65da89f14c1e3d170047e7469c1e1be`.

Post-run source manifest matched at 259 files / 4,087,597 bytes, SHA-256
`4009e200ff8f8568b991926075cecbe64c4753846416e10fc1715653798025e9`. The
HyperIntel import remained at its pinned 99 files and tree digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`, revision
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Full Git status remained 2,607
entries with SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`. Docker
reported no remaining running container; the temporary captured log was
hashed and removed. PID 85094 remained the listener on `127.0.0.1:4317`.

One Linux-specific warning reports unused `raw_pid` in
`crates/hyperai-acp/tests/synthetic_peer.rs:307`; no source edit was made.
Assurance is `REPRODUCED_LOCAL` for this exact source manifest and command in
same-owner Linux/amd64 emulation. Reviewer status is `NOT_REQUESTED`. Source
and synthetic fixtures were read-only; Cargo caches, build output, and test
data stayed in disposable container storage. This result does not establish
native platform support, browser/JavaScript parity, accessibility, independent
review, separate-host reproduction, source rights, or production readiness.

## Current-source continuation v8 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v8`. This
append-only run freezes the source after the aggregate-argument-clause SQL
validator and API regressions; v1-v7 remain bound to their recorded inputs.

Status: pre-run freeze recorded; the v8 container command has not run yet.
Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. No source input may be changed.

Exact read-only inputs follow the expanded v5-v7 set: root `Cargo.toml`,
`Cargo.lock`, `rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`,
`pnpm-lock.yaml`, and `manifests/monorepo-imports.json`; all regular files
under root `crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`,
`migrations/`, and `scripts/`; `docs/openapi.json`; and the 99-file HyperIntel
source import selected by the import-manifest exclusions. Generated targets
and unrelated products are excluded. Hash sorted POSIX paths with big-endian
64-bit path and content lengths and each file's SHA-256. Pre-run source
manifest: 259 files / 4,096,098 bytes, SHA-256
`21c544117cd2d08b316c6f498a1b6ffb90f68f45c06b7b965593487a7456fd59`.

The HyperIntel import is 99 files with tree digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Pre-run full Git status
contains 2,607 entries with SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`. Import
verification matches Simple Compute Market and HyperIntel; the known JustAI
inventory remains 177 observed files versus 176 recorded.

Environment preflight: Docker `29.7.2` runs on Linux/aarch64 with 14 CPUs and
12,525,842,432 bytes of memory. The local `rust:1.93.1-bookworm` image is
Linux/amd64, ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The host Cargo registry and Git caches are 1.2 GiB and 18 MiB. No Docker
containers exist before this run. PID 85094 owns the existing listener on
`127.0.0.1:4317`; it is outside this slice.

Allowed effects: run one disposable Linux/amd64 container with `--network
none`, the workspace and host Cargo caches mounted read-only, cache copies and
build output in disposable container storage, four Cargo jobs, and a 10 GiB
memory cap. Use non-login `bash -c`,
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`, and
`RUSTUP_AUTO_INSTALL=0`; verify toolchain, architecture, mount, and resource
limits; run exactly `cargo test --locked --offline --workspace --all-features`
once; capture exit status, test totals, and output SHA-256; remove the
container; verify pre/post source, HyperIntel import, full Git-status digests,
and listener; update only this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

Proof gates: exact source and import manifests; image, architecture, and
toolchain identity; network isolation; read-only mounts and resource caps;
exact command and output digest; unchanged pre/post source/import/Git status;
container cleanup; listener preservation; and documentation hygiene. Record
failures literally; do not repeat an identical failed command blindly.

Claim ceiling: at most `REPRODUCED_LOCAL` for the exact Rust source manifest
and command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`; reproduction is not established until every source and
environment gate passes. Custody is limited to the local source, fixed
synthetic tests, disposable container caches/build/test data, and a hashed
temporary test log. No native-platform, browser/JavaScript, accessibility,
independent-review, separate-host, rights, provider, payment, settlement,
production, or public-release claim is included. Publish boundary is local
evidence and the three named documentation outputs only.

### v8 execution record (2026-09-30): `INCONCLUSIVE`

The single frozen container ran with the recorded read-only mounts and
resource limits. Inside it, `rustc` and Cargo reported 1.93.1, the target was
`x86_64`, and the workspace and both Cargo-cache mounts were confirmed
read-only. Docker events show the container exited with code 1 and was
removed. The error was in the post-command summary collector: Python rejected
an escaped quote inside an f-string expression with `SyntaxError`. The Cargo
test command had already been invoked, but its exit status was not printed;
its log lived only in the `--rm` container and was destroyed with it. The
Cargo result, test totals, and log digest are therefore unknown. A Docker
stats sample reached 9.798 GiB of the 10 GiB cap; this is not a peak-memory
measurement.

After the container was removed, the source manifest still matched at 259
files / 4,096,098 bytes / SHA-256
`21c544117cd2d08b316c6f498a1b6ffb90f68f45c06b7b965593487a7456fd59`; the
HyperIntel import remained at its pinned 99-file digest; full Git status
remained 2,607 entries / SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`; and
PID 85094 still owned `127.0.0.1:4317`. No source changed. Assurance for v8
is `INCONCLUSIVE`; it does not establish `REPRODUCED_LOCAL`.

## Current-source continuation v9 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v9`. This is a
new local continuation after v8's test-log collector error and near-cap
memory sample. It uses two Cargo build jobs and a writable temporary log mount
outside the repository so the result survives automatic container removal.
The Rust source and test command remain the same; v8 remains inconclusive.

Status: v9 completed with `REPRODUCED_LOCAL` for its exact source manifest and
command in same-owner Linux/amd64 emulation. The v8 result remains
`INCONCLUSIVE`.
Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Source inputs are exactly those
listed for v8, frozen at 259 files / 4,096,098 bytes, SHA-256
`21c544117cd2d08b316c6f498a1b6ffb90f68f45c06b7b965593487a7456fd59`.
HyperIntel remains 99 files with digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`; full Git status is 2,607 entries
with digest
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`.

Environment is the same verified local `rust:1.93.1-bookworm` Linux/amd64
image and Docker host recorded for v8. Run one disposable container with
`--network none`, read-only workspace/Cargo-cache mounts, four CPU limit,
10 GiB memory limit, copied caches and `/tmp/target` inside the container,
and `CARGO_BUILD_JOBS=2`. Mount a newly created host temporary directory at
`/host-output` and redirect the full Cargo output to `/host-output/cargo.log`.
Use the installed `1.93.1-x86_64-unknown-linux-gnu` toolchain through
non-login `bash -c` and `RUSTUP_AUTO_INSTALL=0`. Verify the source/cache mounts
and cgroup limits before running exactly
`cargo test --locked --offline --workspace --all-features` once. Preserve the
Cargo exit code, totals, and log SHA-256 outside the container; then remove the
container and temporary log after post-run checks.

Exact documentation outputs are this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`. Allowed effects are one local
offline synthetic test run, local temporary log capture and hashing, source,
import, full-status, container, and listener checks, and updates to those
three files only. No source write, provider, network, service, paid execution,
settlement, custody, publication, commit, or deployment is authorized.

Proof gates: pre/post source, HyperIntel import, and full Git-status digests
match; exact image/toolchain/architecture, network, read-only mounts, and
resource limits are established; the exact command runs once; its exit status,
all test totals, and output digest are retained; the container and temporary
log are removed; PID 85094 remains the listener; and the three docs pass
scoped hygiene checks. If any gate fails, preserve the exact failure and do
not claim reproduction.

Claim ceiling is at most `REPRODUCED_LOCAL` for the exact frozen Rust source
and command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`; separate-host, native-platform, browser/JavaScript,
accessibility, independent-review, source-rights, and production claims remain
out of scope. Data custody is local source and fixed synthetic fixtures; test
output stays in a temporary local file outside the repository until hashed
and removed. Publish boundary is local evidence and the three documentation
outputs only.

### Result (2026-09-30)

The exact command `cargo test --locked --offline --workspace --all-features`
exited 0 with `CARGO_BUILD_JOBS=2`. It completed 82 test-result groups:
399 passed, 0 failed, 10 ignored, 0 measured, and 0 filtered. The
64,726-byte Cargo log SHA-256 is
`9a1c6a8b1c2604f543874c73f685f41feaad92fc74b42b82bdf083b809dbd49b`.
Rust reported `1.93.1 (01f6ddf75 2026-02-11)`, Cargo reported
`1.93.1 (083ac5135 2025-12-15)`, and the container reported `x86_64`.
Read-only source/cache mounts and the 10 GiB / four-CPU container limits were
verified before execution. A Docker stats sample during tests was 4.814 GiB;
it is not a peak measurement.

Post-run source matched at 259 files / 4,096,098 bytes with SHA-256
`21c544117cd2d08b316c6f498a1b6ffb90f68f45c06b7b965593487a7456fd59`.
HyperIntel remained at its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` and
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`; full Git status remained
2,607 entries with SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`. The
container and temporary hashed log were removed. PID 85094 remained the owner
of `127.0.0.1:4317`.

Assurance is `REPRODUCED_LOCAL` for this exact Rust source manifest and
command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`. This does not establish native platform support,
browser/JavaScript parity, accessibility, independent review, separate-host
reproduction, source rights, or production readiness. Overall Phase 2 remains
`NOT_ASSIGNED`; its parser breadth, client/accessibility, native platform, and
other qualification gates remain open.

## Current-source continuation v10 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v10`. This
append-only continuation follows the ODBC function-escape regression and
expands the source manifest to include all root schema fixtures read by the
Rust workspace tests. It does not alter the v1-v9 run records. The v9 result
remains bound to its 259-file manifest; this v10 scope closes the omitted
schema-input inventory before making a broader current-source claim.

Status: pre-run freeze recorded; the v10 container command has not run.
Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve every pre-existing path.

Exact read-only inputs: root `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`,
`pnpm-lock.yaml`, `manifests/monorepo-imports.json`, and `docs/openapi.json`;
all regular files under root `crates/`, `apps/hyperai-interface/`, `config/`,
`fixtures/`, `migrations/`, `scripts/`, and `schemas/`; and the pinned 99-file
HyperIntel source import under `products/hyperintel/`, selected with the
import-manifest exclusions. The schema directory is included because
`crates/outcomes-domain/src/fixtures.rs` compares the Rust contracts against
the schema fixtures during the workspace suite. Exclude generated targets,
`.git`, caches, Bazaar references, Simple Compute Market and JustAI source
trees, and unrelated products. Hash sorted POSIX relative paths, big-endian
64-bit path and content lengths, and each file's SHA-256.

Pre-run source identity: 286 files / 4,152,372 bytes, SHA-256
`80dd6e5ea8d916e6bb6ae20c00d74a137dbabb8f2384ab6a82531ddb1c60c5ea`.
HyperIntel matches its pinned 99-file tree digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. The pre-run full Git
status has 2,607 entries and SHA-256
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`, computed
from `git status --porcelain=v1 --untracked-files=all` output. The pinned
Simple Compute Market import matches. The preserved JustAI import inventory
is 177 files versus 176 recorded and remains outside this slice; no JustAI
path may be changed to repair it here.

Environment preflight: Docker client/server `29.7.2`; daemon Linux/aarch64
with 14 CPUs and 12,525,842,432 bytes of memory; local
`rust:1.93.1-bookworm` image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
platform Linux/amd64 and repo digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The host Cargo registry and Git caches are present at 1.2 GiB and 18 MiB.
No Docker containers are running. PID 85094 remains the owner of
`127.0.0.1:4317`; it is outside this slice and must not be contacted.

Allowed effects: run one disposable Linux/amd64 container with
`--network none`, the full workspace and host Cargo caches mounted read-only,
copied caches and `/tmp/target` in container storage, four CPU limit, 10 GiB
memory limit, and `CARGO_BUILD_JOBS=2`. Use non-login `bash -c`,
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`, and
`RUSTUP_AUTO_INSTALL=0`; verify image, toolchain, architecture, network,
mounts, and limits; run exactly
`cargo test --locked --offline --workspace --all-features` once; capture the
exit status, test totals, and full-log SHA-256 in a new temporary host
directory outside the repository; remove the container and temporary log;
recheck source, HyperIntel import, full Git status, and listener; update only
this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

Proof gates: exact source/import manifests; pinned image/toolchain/architecture;
network isolation; read-only source and cache mounts; resource limits; one
exact command with retained exit status, all totals, and log digest; matching
pre/post source, import, and Git-status digests; removed container and log;
listener preservation; and scoped documentation hygiene. Stop and record any
failed gate literally; do not retry an identical failed command blindly.

Claim ceiling: at most `REPRODUCED_LOCAL` for this exact expanded Rust source
manifest and command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`. Reproduction is unproven until every gate passes. Data and
artifact custody are limited to local source, fixed synthetic tests,
container-local copied caches/build output/test data, and a temporary hashed
log. Native-platform, separate-host, JavaScript/browser, accessibility,
independent-review, source-rights, provider, payment, settlement, production,
and public-release claims remain outside scope. Overall Phase 2 remains
`NOT_ASSIGNED`. Publish boundary is local evidence and these three documentation
outputs only; no commit or external circulation.

### v10 preflight result (2026-09-30): `INCONCLUSIVE`

The disposable container reported the pinned Rust/Cargo 1.93.1 toolchain,
`x86_64`, a 10 GiB memory limit, and a four-CPU quota. The read-only mount
assertion then exited 1 before cache copying or Cargo execution because its
regular expression expected an extra field after the mount point. Therefore
the test command did not run, no Cargo result or log exists, and v10 establishes
no reproduction. Docker `--rm` removed the container; its empty temporary host
output directory `/tmp/hyperai-bazaar-v10.OHt2Oz` was removed. Post-attempt
checks matched the 286-file source manifest, pinned HyperIntel import, and
2,607-entry Git-status snapshot; PID 85094 remained on `127.0.0.1:4317`.
Reviewer status is `NOT_REQUESTED`; assurance is `INCONCLUSIVE` and
reproducibility is `NOT_REPRODUCED`.

## Current-source continuation v11 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v11`. This is a
new continuation after v10 stopped before Cargo. It keeps the v10 source,
resource, network, authority, and claim boundaries, and corrects only the
read-only mount assertion. Linux `mountinfo` identifies the mount point in
field 5 and the per-mount options in field 6 ([Linux `/proc` documentation](https://kernel.org/doc/html/v6.15/filesystems/proc.html)); v11 checks those fields without expecting an intervening token.

Status: pre-run freeze recorded; the v11 container command has not run.
Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve every pre-existing path.

Exact source inputs and serializer are those frozen for v10: the root Cargo
and pnpm manifests/lockfiles, import manifest, OpenAPI document, all regular
files under `crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`,
`migrations/`, `scripts/`, and `schemas/`, and the pinned 99-file HyperIntel
import. Generated outputs, `.git`, caches, Bazaar references, Simple Compute
Market and JustAI source trees, and unrelated products remain excluded. The
source identity remains 286 files / 4,152,372 bytes, SHA-256
`80dd6e5ea8d916e6bb6ae20c00d74a137dbabb8f2384ab6a82531ddb1c60c5ea`;
HyperIntel remains 99 files with digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Full Git status remains 2,607
entries with digest
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`.

Environment remains the verified local `rust:1.93.1-bookworm` Linux/amd64
image ID `sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
Docker 29.7.2 on Linux/aarch64, network disabled, four CPUs, and 10 GiB
memory. Host Cargo registry and Git caches are read-only bind mounts copied
into disposable container storage; the workspace is read-only. Use
`CARGO_BUILD_JOBS=2`, non-login `bash -c`,
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`, and
`RUSTUP_AUTO_INSTALL=0`.

Allowed effects: verify the three read-only mounts by matching mountinfo field
5 to each target and requiring `ro` in field 6; verify toolchain, architecture,
network isolation, CPU and memory caps; copy caches into the disposable
container; run exactly `cargo test --locked --offline --workspace
--all-features` once; capture exit status, totals, and full-log SHA-256 in a
new temporary host directory outside the repository; remove the container and
log; verify source/import/status/listener preservation; update only this
packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

Proof gates: corrected mount check passes for workspace and both caches;
source/import/environment identities match the freeze; the exact Cargo command
runs once and its exit, totals, and log digest are retained; pre/post source,
HyperIntel, and Git-status digests match; container and log are removed; PID
85094 remains the listener; and scoped documentation hygiene passes. If a gate
fails, record the literal result and do not repeat the same command blindly.

Claim ceiling: at most `REPRODUCED_LOCAL` for the exact expanded manifest and
command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`. Custody, nonclaims, and publish boundary are unchanged from
v10. Overall Phase 2 remains `NOT_ASSIGNED`; no provider, paid execution,
settlement, publication, or production authority is opened.

### v11 execution result (2026-09-30): `REPRODUCED_LOCAL`

The corrected read-only mount checks passed for `/workspace`,
`/host-cargo-registry`, and `/host-cargo-git`. Docker inspection confirmed
`NetworkMode=none`, read-only workspace/cache mounts, four CPUs, and the 10
GiB memory cap. Rust was `1.93.1 (01f6ddf75 2026-02-11)`, Cargo was
`1.93.1 (083ac5135 2025-12-15)`, and the container architecture was `x86_64`.

The exact command `cargo test --locked --offline --workspace --all-features`
exited 0 with `CARGO_BUILD_JOBS=2`. It completed 82 test-result groups:
401 passed, 0 failed, 10 ignored, 0 measured, and 0 filtered. Cargo reported
`Finished test profile` in 10m01s. The captured log was 64,902 bytes with
SHA-256 `2735e6a53a3f3b04d83593c8d8fcaaace0aa9e621d933b686f933aa9542ea930`.
One Linux-only unused-`raw_pid` warning appeared in the ACP synthetic-peer
test; no source change was made.

Post-run source matched at 286 files / 4,152,372 bytes with SHA-256
`80dd6e5ea8d916e6bb6ae20c00d74a137dbabb8f2384ab6a82531ddb1c60c5ea`.
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`, revision
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Full Git status matched 2,607
entries and digest
`ca41abaf96971ebb3a138954e4cfaff0fd8cd302e8f78e93f9a20b23a137ef67`.
Simple Compute Market import verification passed. The pre-existing JustAI
import remains 177 observed files versus 176 recorded; no JustAI files were
changed. Docker reported exit code 0 and removed the container. The temporary
log was hashed and removed. PID 85094 remained the owner of
`127.0.0.1:4317`.

Assurance is `REPRODUCED_LOCAL` for this exact expanded source manifest and
command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`; the run is not independent review, native-platform
qualification, or separate-host reproduction. Data custody was local source,
schema fixtures, and synthetic tests; caches, build output, and test data were
container-local, and the hashed log was removed. Overall Phase 2 remains
`NOT_ASSIGNED`; parser breadth, DataFusion resource qualification,
client/accessibility, native platform, and other qualification gates remain
open. Provider, paid execution, settlement, publication, source rights, and
production remain out of scope.

## Current-source continuation v12 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v12`. This
append-only continuation follows the authenticated Data query-limit regression
and current browser-runner/package updates. It does not alter the v1-v11 run
records or their source identities.

Status: completed; `REPRODUCED_LOCAL` for the exact v12 source manifest and
locked offline Rust workspace test command in the same-owner Linux/amd64
container.
Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve every pre-existing path.

Exact read-only inputs and serializer match v11: root `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`,
`manifests/monorepo-imports.json`, and `docs/openapi.json`; all regular files
under root `crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`,
`migrations/`, `scripts/`, and `schemas/`; plus the pinned 99-file HyperIntel
import under `products/hyperintel/`, selected with the import-manifest
exclusions. Exclude generated targets, `.git`, caches, Bazaar references,
Simple Compute Market and JustAI source trees, and unrelated products. Hash
sorted POSIX paths with big-endian 64-bit path/content lengths and each file's
SHA-256. The pre-run source manifest is 286 files / 4,198,878 bytes, SHA-256
`f910187649dd28d215636e77cbc7b72489159b94d9bd6a5dd84559fc5cd1e1a9`.
HyperIntel remains 99 files with digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`.

Environment preflight on 2026-09-30: Docker client/server `29.7.2` runs on
Linux/aarch64 with 14 CPUs and 12,525,842,432 bytes of memory. The local
`rust:1.93.1-bookworm` image is Linux/amd64, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The host Cargo registry and Git caches are 1.2 GiB and 18 MiB. No container
was running at preflight. The pre-existing listener on
`127.0.0.1:4317` was PID 85094 and remains outside this slice.

Host-shell bootstrap: the first detached-container launch attempt exited 127
because non-login `/bin/bash` did not have Docker on `PATH`; Docker did not
start, and no Cargo command ran. The configured login shell resolved
`/usr/local/bin/docker`, whose verified symlink targets
`/Applications/Docker.app/Contents/Resources/bin/docker`; that absolute path
successfully queried client/server 29.7.2 from non-login `/bin/bash`. Use the
verified absolute CLI path for the single frozen container launch, while
keeping the container's test shell non-login as specified. This corrects only
host executable resolution and does not broaden the container or test scope.

Allowed effects: run exactly one disposable `linux/amd64` container with
`--network none`, four CPUs, and a 10 GiB memory cap; mount source and Cargo
caches read-only, verify each mount using `/proc/self/mountinfo`, copy caches
into disposable container storage, and use `/tmp/target`. Use Rust/Cargo
1.93.1, `CARGO_BUILD_JOBS=2`,
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`, and
`RUSTUP_AUTO_INSTALL=0`. Run exactly
`cargo test --locked --offline --workspace --all-features` once. Capture its
exit status, test totals, and full-log SHA-256 in a temporary host directory
outside the repository; remove the container and temporary log. Verify
pre/post source, HyperIntel import, and full Git-status digests, the unchanged
listener, and that no other pre-existing path was altered. Record the exact
post-run Git-status digest in the execution record, after comparing it with
the immediately pre-run snapshot.

Proof gates: source/import/environment identities match the freeze; network is
disabled; source/cache mounts are read-only; CPU and memory limits are
inspected; the exact command runs once with retained exit/totals/log digest;
pre/post source, HyperIntel import, and Git status match; the container and log
are removed; PID 85094 remains on port 4317; and scoped documentation hygiene
passes. On any mismatch, record the literal result and stop without repeating
the identical command.

Claim ceiling: at most `REPRODUCED_LOCAL` for this exact expanded manifest and
Rust workspace command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`. This does not establish independent review, native platform
support, separate-host reproduction, JavaScript/browser behavior,
accessibility, source rights, provider behavior, settlement, or production
readiness. Data/artifact custody is local source, pinned compile-time inputs,
synthetic tests, disposable container cache/build/test data, and a temporary
hashed-then-removed log. Overall Phase 2 remains `NOT_ASSIGNED`; Fleet live
operations, paid Eval, financial Trade, and external operations remain closed.
Publish boundary is the local reproduction record in this packet,
`docs/state-slices.md`, and `docs/compute-bazaar-rust-port-subplan.md`.

### v12 execution result (2026-09-30): `REPRODUCED_LOCAL`

The first host launch attempt stopped before Docker because non-login
`/bin/bash` could not resolve the CLI. After verifying the executable and
using `/usr/local/bin/docker`, the single container passed its in-container
checks: x86_64, Rust/Cargo 1.93.1, read-only workspace and cache mounts,
`NetworkMode=none`, four CPUs, and a 10 GiB memory limit. The container used
`CARGO_BUILD_JOBS=2` and ran the exact command
`cargo test --locked --offline --workspace --all-features` once. It exited 0:
82 test-result groups, 402 passed, 0 failed, 10 ignored, 0 measured, and 0
filtered. Cargo completed the clean test profile in 10m19s. The changed
authenticated Data API target ran four tests, including
`query_api_returns_typed_limit_errors_without_partial_results`, which passed.

The 65,051-byte captured log SHA-256 is
`01642e759ab4693a794f76aba6d20437e8a67ecbf64bc90b1f0a9cd18c0befd6`. One
Linux-only unused-`raw_pid` warning appeared in the ACP synthetic-peer test;
the test command still exited 0 and no source change was made. The first
read-only totals-parser attempt had a Python syntax error after log capture;
the corrected parser extracted totals and digest from the same saved log, and
Cargo was not rerun.

Post-run source matched at 286 files / 4,198,878 bytes and SHA-256
`f910187649dd28d215636e77cbc7b72489159b94d9bd6a5dd84559fc5cd1e1a9`.
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. The immediately pre/post full
Git-status snapshots each had 2,610 entries and SHA-256
`52488c4a1e962ff0d5e5c0f2661327e973a22ee74a85c4470e0ae5236def9d87`. The
Simple Compute Market inventory passed at 2,189 files and its recorded digest;
the existing JustAI mismatch remains 177 observed versus 176 recorded, and
neither imported tree changed. The stopped container and temporary log were
removed. PID 85094 remained on `127.0.0.1:4317`.

Assurance is `REPRODUCED_LOCAL` for this exact expanded manifest and command in
same-owner Linux/amd64 emulation. Reviewer status is `NOT_REQUESTED`; this is
not independent review, native Linux qualification, or separate-host
reproduction. The log was hashed then removed; source, fixtures, copied Cargo
caches, build output, and test data remained within the declared local or
disposable custody. Overall Phase 2 remains `NOT_ASSIGNED`; broader SQL
qualification, DataFusion resource accounting, accessibility, platform
qualification, independent review, and external-source rights remain open.
No Fleet live operation, paid Eval, financial Trade, provider action,
settlement, publication, or production authority is opened.

## Current-source continuation v13 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v13`. This
append-only continuation binds to the current source after the incremental
Data JSON/CSV result-size preflight; it does not alter v1-v12 evidence.

Pre-run source manifest: 286 files / 4,249,591 bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`.
The source roots, path filtering, and length-prefixed hashing follow v12.
HyperIntel matches the pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. Pre-run Git status contains
2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.

Initial preflight found the installed Docker client at
`/usr/local/bin/docker`, but the Docker daemon was stopped. No listener was
present on `127.0.0.1:4317`. The state slice permitted starting the existing
Docker Desktop locally, without installing/updating software or pulling
images. The one container was later removed.

Use the v12 frozen image, cache, read-only mounts, resource limits, network
isolation, and one-command protocol only after verifying the Docker daemon and
image locally. If any exact preflight fails, stop before running Cargo. The
full pre-run authorization, proof gates, limits, and publish boundary are in
the [v13 state-slice record](state-slices.md#hyperai-bazaar-rust_port_workspace_reproductionv13).

### v13 execution result (2026-09-30): `INCONCLUSIVE`

The local daemon became available. Docker inspection confirmed the exact v12
image ID and repository digest, `NetworkMode=none`, four CPUs, a 10 GiB
memory cap, and read-only source, Cargo registry, and Cargo Git mounts. The
single container was created and `docker exec` exited 1. The host summary
reported a 990-byte log with SHA-256
`a3919030ca66e0bcee5c794889592f2a7ae1710965388e5a068bc09b5c9b3bce` and
zero parsed test-result groups.

The temporary log was removed by the cleanup trap before its contents were
printed, so the failing step is unknown: it may have been the in-container
mount/toolchain/cache preflight or Cargo itself. No test pass is established.
The container and log are absent, no Cargo process remains, and port 4317
remains unbound. Pre/post source stayed at 286 files / 4,249,591 bytes with
SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`.
HyperIntel stayed at its pinned 99-file digest. Git status stayed at 2,619
entries with SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.

Assurance is `INCONCLUSIVE`; reviewer status is `NOT_REQUESTED`, and
reproduction is `NOT_REPRODUCED`. This attempt does not establish
`REPRODUCED_LOCAL`. The next step requires a new preflight-only diagnostic
slice that preserves failure output before any later workspace-test attempt.

## Preflight-only diagnostic v1 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction_diagnostic.v1`.
Purpose: identify v13's failure point without repeating its Cargo test command.
The source freeze remains 286 files / 4,249,591 bytes,
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`;
HyperIntel remains pinned at 99 files and digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`;
Git status remains 2,619 entries with digest
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.

The single network-disabled diagnostic container may check read-only mounts,
Python and toolchain availability, cgroup limits, cache copying, and offline
`cargo metadata`. It must not run Cargo tests or compile workspace targets.
Diagnostics are printed directly so failure text is retained. The exact
scope, allowed effects, proof gates, claim ceiling, and publish boundary are
in the [diagnostic state-slice record](state-slices.md#hyperai-bazaar-rust_port_workspace_reproduction_diagnosticv1).

### Diagnostic v1 result (2026-09-30): `INCONCLUSIVE`

Docker confirmed the exact pinned image, `NetworkMode=none`, four CPU quota,
10 GiB memory, and read-only workspace/registry/Git mounts. The container
reported Python 3.11.2, x86_64, Rust/Cargo 1.93.1, and copied the caches
successfully (1.3 GiB registry, 19 MiB Git). The chosen probe exited 1 because
`cargo metadata` rejects `--workspace`; pinned local Cargo help confirms its
usage is `cargo metadata [OPTIONS]`. No workspace tests or compilation ran.

The first outer-shell cleanup assignment targeted zsh's reserved `status`
variable and failed, leaving the diagnostic container alive temporarily. It
was then explicitly stopped and removed. Final container inventory and process
checks were empty, and port 4317 remained unbound. The source, pinned
HyperIntel import, and Git status matched the v13 freeze. The preflight
therefore establishes the container/toolchain/cache setup, but does not resolve
whether v13 failed while compiling or running a test. Assurance remains
`INCONCLUSIVE`; reproduction remains `NOT_REPRODUCED`.

## Focused Data package diagnostic v1 (2026-09-30)

State slice: `hyperai.bazaar.data_output_size_linux_focused_diagnostic.v1`.
Purpose: isolate the current Data package after the v13 full-workspace attempt
failed without retaining its command output. This focused run is diagnostic
only and does not replace the full workspace reproduction gate.

Pre-run source manifest: 286 regular files / 4,249,591 bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`.
The pinned HyperIntel source contains 99 files with digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
The pre-run Git status contains 2,619 entries with digest
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Use the v13 source roots and exclusions.

Run once in the already-present `rust:1.93.1-bookworm` image, Linux/amd64,
`--pull=never`, with networking disabled, four CPUs, a 10 GiB memory limit,
and read-only workspace and Cargo cache mounts. Copy the caches into
disposable container storage and run exactly
`cargo test --locked --offline -p hyperai-data --all-targets` with two Cargo
jobs. Stream command output directly to the tool result; retain any failure
text. No other targets or compilation commands are in scope.

Before and after the run, verify source/import/Git status digests, toolchain,
image, architecture, resource and network limits, and read-only mounts. Record
the exact exit status, test totals, and output digest; remove the container and
confirm its processes are gone. The result can establish at most
`INTERNAL_PASS` for this local Data package command; it cannot establish
full-workspace reproduction, external review, platform support, or Phase 2
acceptance. Reviewer status is `NOT_REQUESTED`.

### Focused Data package diagnostic result (2026-09-30): `INCONCLUSIVE`

The exact pinned image and container limits were inspected: Linux/amd64 image
ID `sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
network mode `none`, four CPUs, 10 GiB memory, and read-only workspace and
Cargo-cache mounts. Inside the container, all three mounts reported read-only,
architecture was `x86_64`, and Rust/Cargo reported 1.93.1. The caches copied
to disposable container storage.

The exact command `cargo test --locked --offline -p hyperai-data --all-targets`
was invoked once, but `docker exec` started in `/` rather than `/workspace`.
Cargo exited 101. The retained error output was:

```text
error: could not find `Cargo.toml` in `/` or any parent directory
```

No test or compilation target ran; test totals are unavailable. The one
authorized attempt is spent and was not retried.

The container was removed; no Cargo/Rust process or container remains, and
port 4317 remained unbound. Post-run source matched at 286 files / 4,249,591
bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`;
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`;
Git status matched at 2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Simple Compute Market still matches its manifest. The preserved JustAI
mismatch remains 177 files with digest
`08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`, versus
176 recorded; its tree was not changed by this attempt.

Assurance is `INCONCLUSIVE`; reviewer status is `NOT_REQUESTED`, and
reproduction is `NOT_REPRODUCED`. A corrected retry requires a fresh named
slice that sets the container working directory to `/workspace`; it cannot
reuse this one-attempt authorization.

## Focused Data package diagnostic v2 (2026-09-30)

State slice: `hyperai.bazaar.data_output_size_linux_focused_diagnostic.v2`.
This fresh one-run diagnostic follows v1's `INCONCLUSIVE` result, where the
container command began in `/` and Cargo could not find the workspace
manifest. The source and exact Cargo command are unchanged; this continuation
sets the container working directory to `/workspace` before invocation.

Pre-run source manifest: 286 regular files / 4,249,591 bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`.
HyperIntel is 99 files with digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
Git status contains 2,619 entries with digest
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
The Simple Compute Market import matches its recorded manifest. The known
JustAI mismatch remains 177 observed files versus 176 recorded; preserve it.
Use the v13 source roots and exclusions.

Run exactly once in the existing `rust:1.93.1-bookworm` image, Linux/amd64,
`--pull=never`, with `--network none`, four CPUs, a 10 GiB memory limit, and
read-only workspace and Cargo-cache mounts. Copy caches into disposable
container storage. Set the container working directory to `/workspace`, use
Rust/Cargo 1.93.1 and two Cargo jobs, and run exactly
`cargo test --locked --offline -p hyperai-data --all-targets`. Stream all
output directly to the tool result. Run no other target or compilation
command.

Verify the image, resource/network limits, architecture, working directory,
toolchain, read-only mounts, source/import/Git status before and after; record
the exact exit status, output, totals if available, and log digest if retained;
remove the container and confirm no process remains. This can establish at
most `INTERNAL_PASS` for the focused package command. It cannot close Phase 2,
reproduce the full workspace, or establish independent review, native Linux
support, or production readiness. Reviewer status is `NOT_REQUESTED`.

### Focused Data package diagnostic v2 result (2026-09-30): `INTERNAL_PASS`

The container used working directory `/workspace`. Preflight confirmed the
same pinned Linux/amd64 image, network mode `none`, four CPUs, 10 GiB memory,
read-only workspace/cache mounts, x86_64 architecture, and Rust/Cargo 1.93.1.
The exact command
`cargo test --locked --offline -p hyperai-data --all-targets` exited 0 after
compiling in 4m58s. Four test-result groups ran: 32 unit tests, 5 catalog
contract tests, 2 derived-data contract tests, and 12 query contract tests;
51 passed, 0 failed, 0 ignored, 0 measured, 0 filtered. Output was streamed
directly to the tool result; no separate log file was retained.

The container was removed, no Cargo/Rust process or container remains, and
port 4317 remained unbound. Post-run source matched at 286 files / 4,249,591
bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`;
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`;
Git status matched at 2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Simple Compute Market still matches its manifest. The preserved JustAI
mismatch remains 177 files with digest
`08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`, versus
176 recorded; its tree was not changed by this attempt.

Assurance is `INTERNAL_PASS` only for the focused Data package suite; reviewer
status is `NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`. This does not
resolve the full-workspace v13 `INCONCLUSIVE` result or assign overall Phase 2,
which remains `NOT_ASSIGNED`.

## Current-source continuation v14 (2026-09-30)

State slice: `hyperai.bazaar.rust_port_workspace_reproduction.v14`. This
fresh one-run root test follows v13's `INCONCLUSIVE` result and the focused
Data package v2 diagnostic. Neither earlier result is rewritten: the v2 pass
covers only the Data package, while v13's failing step remains unknown.

Pre-run source manifest: 286 files / 4,249,591 bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`. The
manifest roots and length-prefixed algorithm follow v12/v13. HyperIntel is
pinned at `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`, 99 files, digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
Pre-run Git status contains 2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Simple Compute Market matches its 2,189-file import manifest. The known
JustAI mismatch remains 177 observed files versus 176 recorded; preserve it.

Preflight: Docker `29.7.2` on Linux/aarch64 reports 14 CPUs and
12,525,842,432 bytes of memory. The cached `rust:1.93.1-bookworm` image is
Linux/amd64, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The local Cargo registry is 1.2 GiB and Git cache is 18 MiB. No container or
Cargo/Rust process was present before launch; port 4317 had no listener.

Run exactly one container without pulling the image, with Linux/amd64,
`--network none`, four CPUs, 10 GiB memory, `/workspace` as its configured
working directory, and read-only mounts for the workspace, Cargo registry,
and Cargo Git cache. Explicitly pass `/workspace` to `docker exec` as well.
Verify the exact image, working directory, cgroup limits, network mode, and
read-only mounts before testing. Copy caches into container-local storage;
use Rust/Cargo 1.93.1, two Cargo jobs, offline mode, and `/tmp/target`.
Command: `cargo test --locked --offline --workspace --all-features`, once.
Stream all output directly to the tool and to
`/tmp/hyperai-v14-root-tests.log`; retain exit status and test totals, hash the
log, then remove it and the container. The temporary log is local test output
only and is not an artifact for publication.

The claim ceiling is at most `REPRODUCED_LOCAL` for this exact root Rust
workspace command and source manifest in same-owner Linux/amd64 emulation.
This cannot establish native Linux support, separate-host reproduction,
JavaScript/browser behavior, accessibility, independent review, source rights,
Phase 2 acceptance, or production readiness. Reviewer status is
`NOT_REQUESTED`; Phase 2 remains `NOT_ASSIGNED`. Fleet live operations, paid
Eval, financial Trade, provider calls, and external operations remain closed.

### v14 execution result (2026-09-30): `INTERNAL_PASS`

The container passed its pinned image, Linux/amd64, `/workspace`, offline,
read-only mount, and resource-limit checks. With Rust/Cargo 1.93.1,
`CARGO_BUILD_JOBS=2`, and `CARGO_NET_OFFLINE=true`,
`cargo test --locked --offline --workspace --all-features` exited 0. Cargo
reported 82 test-result groups: 405 passed, 0 failed, 10 ignored, 0 measured,
and 0 filtered. The 10 ignored tests require optional browser binaries,
size-limited tmpfs, selected Docker images, PostgreSQL, or staged sidecars.
One Linux-only unused-`raw_pid` warning appeared in the ACP synthetic-peer
test; no source change was made.

The 65,553-byte captured log SHA-256 is
`4b4ec4c70fb529d782a223f5f35bcfb077aca882799b8c6113a1875ad0ca102c`; it was
hashed and removed. Post-run source matched the pre-run manifest: 286 files /
4,249,591 bytes, SHA-256
`db8cc32babd90922db2a07663579f4bcdda47a05bd2001a28e69db355732543f`.
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
Pre/post Git status matched at 2,619 entries with SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`. The
Simple Compute Market inventory passed. The known JustAI inventory mismatch
remains 177 observed files versus 176 recorded; its observed digest is
`08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`.

The disposable container and log were removed; no Cargo/Rust process or
container remains, and port 4317 is unbound. Assurance is `INTERNAL_PASS` for
this exact local run, reviewer status is `NOT_REQUESTED`, and reproduction is
`NOT_REPRODUCED`. This does not establish native Linux, separate-host,
JavaScript/browser, accessibility, independent-review, source-rights, Phase 2,
or production readiness. Overall Phase 2 remains `NOT_ASSIGNED`; the scoped
platform, browser/client, accessibility, and Phase 2 gates remain open.

### v14 assurance and limits

The run establishes only that the current frozen Rust workspace passed this
locked/offline test command in same-owner Linux/amd64 emulation. It does not
qualify a second host or native Linux. The ten ignored cases did not run, and
their external fixtures/services remain unqualified. The claim ceiling stays
at most `REPRODUCED_LOCAL`; no independent acceptance or production authority
is conferred. Fleet live operations, paid Eval, financial Trade, provider
calls, external operations, and publication remain closed.

## Focused interactive TUI Data reproduction v1 (2026-09-30)

State slice: `hyperai.bazaar.data_tui_query_input_reproduction.v1`. This
one-run attempt targets the current TUI SQL edit/run/clear/cancel tests and
terminal restoration tests. Its fresh source manifest contains 286 regular
files / 4,271,842 bytes, SHA-256
`dc33c2d0266dd177eaec0a78d21675aaec24c766f89c9cb0288dbb3a7b5ec7fb`; it
uses the v14 roots and length-prefixed path/content algorithm, with the
current TUI input changes included. HyperIntel matched the pinned 99-file
digest. Simple Compute Market matched its import; the known JustAI mismatch
remained 177 observed files versus 176 recorded. Git status matched at 2,619
entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Bazaar references and the Simple Compute Market and JustAI source trees were
excluded from the staged source.

The cached image was Linux/amd64, ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`;
the container used `--network none`, four CPUs, a 10 GiB limit, read-only
root/source/cache mounts, and `/workspace`. Rust/Cargo 1.93.1 versions were
verified using their installed binaries after the rustup shim could not write
its update temporary file under the read-only root filesystem.

The exact command `cargo test --locked --offline -p hyperai-tui --all-targets`
exited 101 before any test result. Cargo compiled the `proc-macro2` build
script but could not execute it from `/tmp/target` (`Permission denied`). The
container mount table showed `/tmp` had `noexec`, explaining the environment
failure; this does not assess the implementation. The 401-byte output log
SHA-256 is
`43678f93bd43b67ebc99d7673a7331dcb067b465dca3c16ed35cb0c9aa523e3e`; it was
removed. Post-run source, HyperIntel, and Git-status digests matched. The
container and source stage were removed, and port 4317 and non-loopback
sources were not contacted. Assurance is `INCONCLUSIVE` for this attempt;
reviewer status is `NOT_REQUESTED`, and reproduction remains
`NOT_REPRODUCED`.

The fresh v2 slice,
[`hyperai.bazaar.data_tui_query_input_reproduction.v2`](state-slices.md#hyperai-bazaar-data_tui_query_input_reproductionv2),
kept the same source and test command and explicitly mounted `/tmp` with
`exec`. The v1 result remains unchanged.

## Focused interactive TUI Data reproduction v2 (2026-09-30): `REPRODUCED_LOCAL`

State slice: `hyperai.bazaar.data_tui_query_input_reproduction.v2`. This
fresh one-run continuation changes the disposable `/tmp` tmpfs to include
`exec`, addressing only v1's verified `noexec` environment failure.

The pre/post frozen source manifest matched at 286 files / 4,271,842 bytes,
SHA-256
`dc33c2d0266dd177eaec0a78d21675aaec24c766f89c9cb0288dbb3a7b5ec7fb`;
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`, and
Git status matched at 2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Simple Compute Market matched its 2,189-file import. The known JustAI
inventory mismatch remained 177 observed files versus 176 recorded; no
imported tree changed.

The exact command `cargo test --locked --offline -p hyperai-tui --all-targets`
exited 0 in the cached Linux/amd64 image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
with Rust 1.93.1 and Cargo 1.93.1. The container used `--network none`, four
CPUs, 10 GiB memory, read-only root/source/cache mounts, `/workspace`, and an
executable `/tmp` tmpfs. The 8m14s test profile produced two result groups:
12 unit tests and 10 terminal-restoration integration tests; 22 passed, 0
failed, 0 ignored, 0 measured, and 0 filtered. The 16,964-byte output log
SHA-256 is
`dee312b9ae9e9ddaee245a5694e5c33278c004dd932992d0bdcafbe0b2aba939`; it was
removed after hashing.

The synthetic test service used only loopback and temporary container data.
The container, source staging directory, copied Cargo caches, target output,
and log were removed; no test process or container remained. Port 4317 and
non-loopback sources were not contacted. Assurance is `REPRODUCED_LOCAL` for
this exact TUI package test and manifest in same-owner Linux/amd64 emulation;
reviewer status is `NOT_REQUESTED`. This does not establish native Linux,
accessibility, full client-policy parity, independent review, source rights,
overall Phase 2 acceptance, or production readiness. Phase 2 remains
`NOT_ASSIGNED`.

## Current-source continuation v15 (2026-09-30)

State slice: [`hyperai.bazaar.rust_port_workspace_reproduction.v15`](state-slices.md#hyperai-bazaar-rust_port_workspace_reproductionv15).

Status: completed; `REPRODUCED_LOCAL` for one locked/offline full Rust
workspace test run after the Data serialized-response-limit source change.
The v14 result remains bound to its earlier source manifest and is not evidence
for the current source.

The read-only input roots, import filtering, and length-prefixed manifest
algorithm are the same as v12-v14: root Cargo and pnpm manifests/lockfiles,
`manifests/monorepo-imports.json`, `docs/openapi.json`, all regular files under
`crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`, `migrations/`,
`scripts/`, and `schemas/`, and the pinned HyperIntel import. Exclude generated
targets, `.git`, caches, Bazaar references, Simple Compute Market and JustAI
trees, and unrelated products. Hash sorted POSIX paths with big-endian 64-bit
path/content lengths and each file's SHA-256.

Pre-run source manifest: 286 files / 4,288,313 bytes, SHA-256
`12a19f482385845cafab9826e2426f524e259edcb9bee659a294571f0ad18d1d`.
HyperIntel matched its pinned 99-file digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
Pre-run Git status contained 2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Import verification passed Simple Compute Market; the preserved JustAI
inventory mismatch remained 177 observed files versus 176 recorded.

Use the already pinned local `rust:1.93.1-bookworm` Linux/amd64 image
(`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`)
in one disposable container. Require `--network none`, four CPUs, a 10 GiB
memory ceiling, a read-only `/workspace` source mount, read-only Cargo cache
mounts copied into container-local storage, `/workspace` as the working
directory, two Cargo build jobs, `CARGO_NET_OFFLINE=true`, and
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu`. Verify the image,
architecture, toolchain, mount modes, and resource limits before running the
exact command once:

```sh
cargo test --locked --offline --workspace --all-features
```

Record the manifest and Git-status digests before and after, test-result
totals, exit code, and bounded log SHA-256. Remove the container, copied
caches, target output, and log. No port-4317 or external-source connection is
allowed. If setup or tests fail, keep the literal result and do not retry in
this slice.

Environment preflight: Docker client/server `29.7.2` ran on Linux/aarch64
with 14 CPUs and 12,525,842,432 bytes of memory. The cached
`rust:1.93.1-bookworm` image was Linux/amd64, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The Cargo registry and Git cache were 1.2 GiB and 18 MiB. Before start, the
container had `--network none`, four CPUs, a 10 GiB memory ceiling, `/workspace`
as its working directory, and read-only workspace and Cargo cache mounts.

Execution result (2026-09-30): Rust reported
`1.93.1 (01f6ddf75 2026-02-11)` and Cargo reported
`1.93.1 (083ac5135 2025-12-15)`. With `CARGO_BUILD_JOBS=2` and
`CARGO_NET_OFFLINE=true`,
`cargo test --locked --offline --workspace --all-features` exited 0 and ran
82 test-result groups: 421 passed, 0 failed, 10 ignored, 0 measured, and 0
filtered. One Linux-only `raw_pid` unused-variable warning appeared in the
`hyperai-acp` synthetic-peer test; no source change was made.

The 66,615-byte Cargo test log SHA-256 is
`68fbb77afb63d0ef43d58387eb7668094ed7004bfbd8e74f4dc8c92e97f5a939`. The
post-run source manifest matched at 286 files / 4,288,313 bytes with SHA-256
`12a19f482385845cafab9826e2426f524e259edcb9bee659a294571f0ad18d1d`.
HyperIntel matched its pinned 99-file digest. Post-run Git status, before the
result-documentation update, matched at 2,619 entries with SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
Simple Compute Market matched its import manifest. The preserved JustAI
mismatch remained 177 observed files versus 176 recorded, with observed
digest
`08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`.

The stopped container was removed. Its copied caches, target output, and
in-container test log were deleted; the host capture was hashed and removed.
No Cargo/Rust process or container remained, and port 4317 and external
sources were not contacted.

Assurance is `REPRODUCED_LOCAL` for the exact current source manifest and root
Rust test command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`. This cannot establish native Linux or Windows support,
separate-host reproduction, independent review, JavaScript/browser or
accessibility qualification, source rights, Phase 2 acceptance, or production
readiness. Overall Phase 2 remains `NOT_ASSIGNED`; Fleet live operations,
paid Eval, financial Trade, provider calls, and all external operations remain
closed.

## Current-source continuation v16 (2026-09-30)

State slice: [`hyperai.bazaar.rust_port_workspace_reproduction.v16`](state-slices.md#hyperai-bazaar-rust_port_workspace_reproductionv16).
This continuation follows the Data manifest row-count and authenticated
1,000-row API probe change. It preserves v15 and earlier results against their
own source manifests.

The root source manifest covers Cargo and pnpm workspace manifests/lockfiles,
`manifests/monorepo-imports.json`, `docs/openapi.json`, all regular source and
fixture files under `crates/`, `apps/hyperai-interface/`, `config/`,
`fixtures/`, `migrations/`, `scripts/`, and `schemas/`, and the pinned
HyperIntel import. It excludes generated targets, `.git`, caches, Bazaar
references, Simple Compute Market and JustAI trees, and unrelated products.
Sorted POSIX paths are hashed using big-endian 64-bit path and content lengths
and each file's SHA-256.

Pre-run source manifest: 286 regular files / 4,315,607 bytes, SHA-256
`6f88de581b7a8f1e18170445c18b77e5b5fab3c64ebbc5517d6a8d8c99713df2`.
Pre-run Git status: 2,619 entries, SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`.
The HyperIntel import matches its 99-file pin and digest. Docker client/server
is `29.7.2` on Linux/arm64. The cached image is Linux/amd64, ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
The local Cargo registry and Git caches are 1.2 GiB and 18 MiB. The planned
container uses no network, a read-only workspace and cache mounts, four CPUs,
a 10 GiB memory ceiling, two Cargo build jobs, and container-local cache and
target copies. Rust and Cargo must report 1.93.1 before the one full workspace
test command. Do not access port 4317.

Execution result (2026-09-30): the pinned `rust:1.93.1-bookworm`
Linux/amd64 image ran with `--network none`, read-only workspace and Cargo
cache mounts, four CPUs, a 10 GiB memory ceiling, and two Cargo jobs. The
container preflight confirmed the cgroup limits, and Rust/Cargo reported
`1.93.1 (01f6ddf75 2026-02-11)` and `1.93.1 (083ac5135 2025-12-15)`. The
single `cargo test --locked --offline --workspace --all-features` command
exited 0: 82 test-result groups, 425 passed, 0 failed, 10 ignored, 0 measured,
and 0 filtered. It emitted one Linux-only `raw_pid` unused-variable warning
in the ACP synthetic-peer test; no source change was made.

The 66,978-byte Cargo log SHA-256 is
`15f117b47c43da92700a4f2816fc83a614eda0de1d883162d0b28ab4719bf5ae`. The
post-run manifest matched at 286 files / 4,315,607 bytes with SHA-256
`6f88de581b7a8f1e18170445c18b77e5b5fab3c64ebbc5517d6a8d8c99713df2`; the
pre/post Git status matched at 2,619 entries with SHA-256
`8fdb9afafb48489d74aa1e4ab2bf170cc2f1b83f77aac3244e03541cc8bcd8cc`. The
container and its copied caches, target, and log were removed. Simple Compute
Market and HyperIntel import pins matched; import verification still exits 1
on the unchanged JustAI inventory mismatch (177 observed files versus 176
recorded). Scoped Markdown hygiene passed. Port 4317 and external sources
were not contacted.

Assurance is `REPRODUCED_LOCAL` for this exact source manifest and root Rust
test command in same-owner Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`; this does not establish a separate-host reproduction,
independent review, native platform support, browser/accessibility
qualification, source rights, Phase 2 acceptance, or production readiness.
Overall Phase 2 remains `NOT_ASSIGNED`; Fleet live operations, paid Eval,
financial Trade, provider calls, and all external operations remain closed.

## Current-source continuation v17 (2026-10-01)

State slice: [`hyperai.bazaar.rust_port_workspace_reproduction.v17`](state-slices.md#hyperai-bazaar-rust_port_workspace_reproductionv17).
This continuation follows the current full root lint, import verification,
installed ACP integration, and interface qualification. It does not alter
v1-v16 evidence or claim native Linux support.

The root source manifest includes `Cargo.toml`, `Cargo.lock`, root pnpm
workspace manifests/lockfiles, `rust-toolchain.toml`,
`manifests/monorepo-imports.json`, `docs/openapi.json`, regular files under
`crates/`, `apps/hyperai-interface/`, `config/`, `fixtures/`, `migrations/`,
`scripts/`, `schemas/`, and the exact 99-file pinned HyperIntel import. It
excludes generated targets, `.git`, caches, Bazaar references, Simple Compute
Market and JustAI trees, and unrelated products. The source manifest hashes
sorted POSIX paths with big-endian 64-bit path and content lengths plus each
file's SHA-256 digest.

Pre-run source manifest: 293 regular files / 4,522,620 bytes, SHA-256
`0ef8866db0e45bbf1741233d6400fc0bd3cc46e8f2c544d866bb915f0b73e53a`. The
pre-run full Git-status digest is recorded in the matching state-slice entry
before execution. The pinned HyperIntel import has 99 files and digest
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b` at
revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`.

The available image is `rust:1.93.1-bookworm`, ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
Linux/amd64. Docker Desktop 29.7.2 runs on Linux/aarch64 with 14 CPUs and
12,525,842,432 bytes reported memory. The local Cargo registry and Git caches
are 1.2 GiB and 18 MiB.

Allowed effects: inspect the pinned image and caches; run one disposable
`linux/amd64` container using `--network none`, four CPUs, a 10 GiB memory
limit, a read-only workspace and read-only Cargo-cache mounts, and
container-local cache/target copies; verify Rust/Cargo 1.93.1 and the runtime
limits before running exactly one
`cargo test --locked --offline --workspace --all-features` command; retain
only its exit status, bounded totals, output size, and SHA-256; remove all
container-local caches, target output, and logs. Verify pre/post source and
Git-status digests and import pins. Do not connect to `127.0.0.1:4317` or any
external service.

No product source, imported tree, manifest, API, dependency, dataset, runtime,
or platform configuration may change. No Bazaar material, user payload,
provider, seller, remote worker, paid execution, live Fleet, financial Trade,
settlement, publication, deployment, or external review is in scope. If the
preflight, one test command, digest check, or cleanup fails, retain the result
and stop without retrying under v17.

Claim ceiling: at most `REPRODUCED_LOCAL` for this exact source manifest and
root Rust command in same-owner Linux/amd64 emulation. This does not establish
native Linux support, a separate-host reproduction, independent review,
browser/accessibility qualification, Bazaar rights, Phase 2 acceptance, or
production readiness. Reviewer status is `NOT_REQUESTED`; publish boundary
is local documentation only.

Execution result (2026-10-01): `INCONCLUSIVE` before Cargo. Docker inspection
confirmed `network=none`, the 10 GiB memory limit, four CPUs, and read-only
workspace/cache mounts. Inside the x86_64 container, `cpu.max` was
`400000 100000` and `memory.max` was `10737418240`; the Cargo registry and Git
caches were copied into the disposable container. The `sh -lc` preflight then
exited 127 with `rustc: not found`; the locked workspace test command did not
run. The cached image configuration advertises
`PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`,
`RUSTUP_HOME=/usr/local/rustup`, and `RUST_VERSION=1.93.1`. The failure is
consistent with the login shell changing the image PATH, but the inner PATH
was not captured, so that cause is not proven. The container was removed.

Post-run source matched at 293 files / 4,522,620 bytes with SHA-256
`0ef8866db0e45bbf1741233d6400fc0bd3cc46e8f2c544d866bb915f0b73e53a`. The
full Git-status output remained 229,757 bytes with SHA-256
`c6824e1e3548b93b73edee94c25d2c5c503dbdf79c21cc46b581d64530e4b28a`.
Simple Compute Market, HyperIntel, and JustAI import verification passed.
`git diff --check` passed. No test log was created and no source or imported
tree changed. The v17 stop condition was followed; any continuation must use
a fresh state slice.

## Current-source continuation v18 (2026-10-01)

State slice: [`hyperai.bazaar.rust_port_workspace_reproduction.v18`](state-slices.md#hyperai-bazaar-rust_port_workspace_reproductionv18).
This fresh continuation follows v17's preflight-only `rustc: not found`
result. It preserves v17 as `INCONCLUSIVE` and avoids repeating its login
shell invocation. The exact workspace source manifest remains the frozen v17
manifest; the current pre-run identities are recorded in the v18 state-slice
entry.

Shell correction: start the pinned image with its non-login `bash -c` shell so
the image-provided PATH retains `/usr/local/cargo/bin`. Set
`RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu` and preserve the image's
`RUSTUP_HOME=/usr/local/rustup`; set only `CARGO_HOME` and `CARGO_TARGET_DIR`
to disposable container-local paths. Before the single Cargo command, assert
that `/usr/local/cargo/bin/rustc` is executable, `rustc` and `cargo` report
the pinned 1.93.1 versions, architecture is x86_64, `cpu.max` is
`400000 100000`, `memory.max` is `10737418240`, and source/cache mounts are
read-only. The container itself remains network-disabled with four CPUs and
a 10 GiB memory limit.

The allowed command is exactly one
`cargo test --locked --offline --workspace --all-features`. If preflight
fails, do not run Cargo. If the test command fails, record it and stop. In
either case, clean the container and verify the source manifest, complete Git
status, and imported trees. No code or imported tree changes are allowed.
Reviewer status is `NOT_REQUESTED`; maximum claim remains
`REPRODUCED_LOCAL` for the exact Rust source manifest and command in same-owner
Linux/amd64 emulation. Native Linux support, separate-host reproduction,
browser/accessibility behavior, Phase 2 acceptance, source rights, and
production readiness remain open.

Execution result (2026-10-01): the corrected non-login `bash -c` preflight
passed with image PATH intact, x86_64 architecture, `cpu.max=400000 100000`,
`memory.max=10737418240`, Rust 1.93.1, and Cargo 1.93.1. The locked offline
`cargo test --locked --offline --workspace --all-features` command exited 0
across 82 test-result groups: 441 passed, 0 failed, 10 ignored, 0 measured,
and 0 filtered. The test log was 68,744 bytes with SHA-256
`6a920b549a938975e30891ca2ef103ca4f6aa50adbbb64d612def9d1e355aed0`; only
its bounded summary and digest were retained.

Post-run source matched at 293 files / 4,522,620 bytes with SHA-256
`0ef8866db0e45bbf1741233d6400fc0bd3cc46e8f2c544d866bb915f0b73e53a`. The
full Git-status output matched at 229,757 bytes with SHA-256
`c6824e1e3548b93b73edee94c25d2c5c503dbdf79c21cc46b581d64530e4b28a`.
Simple Compute Market, HyperIntel, and JustAI import verification passed
before and after execution. The container exited 0 and was removed with its
copied caches, target, and log. `git diff --check` passed after recording the
result. No source or imported tree changed, and no service or external source
was contacted.

Assurance is `REPRODUCED_LOCAL` for this exact Rust source manifest and
command in same-owner Docker Desktop Linux/amd64 emulation. Reviewer status is
`NOT_REQUESTED`; this is not independent review, native Linux support,
separate-host reproduction, browser/accessibility reproduction, overall
Phase 2 acceptance, source-rights clearance, or production readiness. Test
custody was limited to the pinned read-only workspace, HyperIntel snapshot,
synthetic tests, local Cargo caches, and disposable container output.
