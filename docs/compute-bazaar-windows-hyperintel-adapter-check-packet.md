# HyperIntel Adapter Windows Target Check Packet v1

State slice: [`hyperai.bazaar.windows_hyperintel_adapter_crosscheck.v1`](state-slices.md#hyperai-bazaar-windows_hyperintel_adapter_crosscheckv1).
Parent: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md).
Integration: [HyperAI Rust port subplan](compute-bazaar-rust-port-subplan.md).

Status: executed once; target result is `INCONCLUSIVE`. The run reached the
bundled SQLite C dependency, where the macOS host lacked Windows C headers.
It does not establish whether the HyperAI-owned offline HyperIntel adapter
type-checks under a complete MSVC toolchain and does not change the adapter,
imported HyperIntel source, package graph, or runtime behavior.

## Frozen baseline

- HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, branch `main`.
- Pre-check `git status --porcelain=v1 -z` SHA-256:
  `d80f0978d1aedd5cc4a3a538900ae4378b0aae0d89ea948544c93e27f796358f`
  across 176 records. Preserve all dirty and untracked work.
- Pinned toolchain: Rust/Cargo `1.93.1` through
  `scripts/with-pinned-rust.sh`.
- Installed target: `x86_64-pc-windows-msvc`.
- HyperIntel snapshot: local commit `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`,
  99 imported files with the manifest tree digest
  `62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.
  The read-only `python3 scripts/verify_imports.py` check passed at freeze.

Frozen file hashes:

- `Cargo.toml`: `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487`
- `Cargo.lock`: `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2`
- `crates/hyperintel-adapter/Cargo.toml`:
  `2b64d7f71c0e40ad5b4797bb6b6ed0838961c8ffd956272d1c0ba9251ae7be8a`
- `crates/hyperintel-adapter/src/lib.rs`:
  `8018f32dd20fe94a2439e8b3a88e188af5e8d921ba61af8881594e8cabc49a98`
- `scripts/with-pinned-rust.sh`:
  `d0bb19b6f621a31e10a7b46be852889a4f69bd0196ba7545abd9219b36f51171`
- `docs/provenance.md`:
  `6e6b0ef176f996b1cafb2e0bf91264fc4dd2f11fda687d949466b2183b966ade`
- `docs/compute-bazaar-rust-port-subplan.md`:
  `0e4e2b670945b1bb3af9355332559ea79c076ade8be165ee869b757f2c6d5d53`
- `docs/state-slices.md`:
  `d95114c278f576a9cc1ca22c2eb1effb5ed60d9d7c7c49217520ab6d664b5762`

## Exact scope

Read-only implementation inputs are the HyperIntel adapter and its six
recorded HyperIntel path dependencies, `Cargo.toml`, `Cargo.lock`, the pinned
Rust wrapper, the imported-source manifest, and provenance. Exact writable
files are this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md` for recording the result.

Allowed effects are one offline type-check of the HyperIntel adapter library,
the read-only import verifier, and Markdown/hash/staging hygiene. Cargo may
write ordinary build products under ignored `target/`. No Windows binary is
linked or executed. No tests, service, data files, provider, network endpoint,
ACP peer, wallet, chain, paid operation, or external system is used.

Run exactly once:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo check --locked --offline --jobs 1 --target x86_64-pc-windows-msvc -p hyperai-hyperintel-adapter --lib
```

If the command fails, retain the literal result and stop; do not fetch
dependencies or retry under this packet.

## Proof gates and claim ceiling

### Execution result

Exact frozen command:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo check --locked --offline --jobs 1 --target x86_64-pc-windows-msvc -p hyperai-hyperintel-adapter --lib
```

Exit code: `101`. `libsqlite3-sys v0.30.1` attempted to compile
`sqlite3/sqlite3.c` with the macOS Clang driver and Windows MSVC target flags.
Compilation stopped because Clang's `mm_malloc.h:13` could not find
`stdlib.h`; the host does not provide the Windows C headers/toolchain needed
for this cross-target C dependency. This is not a Rust type diagnostic for
`hyperai-hyperintel-adapter`. No dependencies were fetched and the packet was
not retried.

Result: `INCONCLUSIVE` for the adapter's Windows target type-check. The packet
is closed. Any further attempt requires a separately frozen native Windows
build/lifecycle slice with the required MSVC toolchain and Windows SDK.

Closure checks passed after the target result: `python3
scripts/verify_imports.py` matched Simple Compute Market (2,189 files),
HyperIntel (99), and JustAI (176) to their recorded pins. All six frozen
implementation-input hashes matched. `git diff --check`, touched-document
trailing-whitespace and local-link checks passed, and
`git diff --cached --name-only` was empty. The ignored `target/` output added
no tracked or staged path. The one-shot command remains `INCONCLUSIVE` at the
host C-header boundary recorded above.

No adapter Windows type-check claim is made because the command did not pass
the native C dependency. It also provides no evidence for linking, executable
packaging, native Windows runtime, Windows filesystem/auth/process behavior,
adapter correctness beyond existing tests, or Windows product support.
Assurance is `INCONCLUSIVE`; reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. Data/artifact custody is limited to the
pinned local source trees, Cargo cache, and ignored build output.

Next authority gate remains native Windows build and lifecycle qualification.
No Bazaar source/data is imported, and no user-facing API or runtime contract
changes. Publish boundary: local documentation only; no commit, PR, release,
deployment, or external circulation.
