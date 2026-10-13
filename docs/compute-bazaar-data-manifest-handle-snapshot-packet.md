# Data Manifest Open-Handle Snapshot Packet

Status: accepted local synthetic implementation packet. It authorizes only the
manifest and dataset path-open hardening described here. It does not assign
the Phase 2 Data worker or grant source-ingestion, external-data, or production
authority.

State slice: `hyperai.bazaar.data_manifest_handle_snapshot.v1`.
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Related review: [catalog identity red-team packet](compute-bazaar-data-manifest-catalog-redteam-packet.md)
and [manifest directory-entry packet](compute-bazaar-data-manifest-directory-entry-packet.md).
Subplan: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

## Frozen baseline

- Repository: HyperAI, branch `main`, base
  `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `a86463f2ce0a4105f2c71fa9e8da95ab682e5257c3ba0b61558ae0eb8aa74870`
  (177 status records). Preserve all existing dirty and untracked work.
- `git diff --cached --name-only` was empty.
- Current source and input hashes:

| Path | SHA-256 |
|---|---|
| `crates/hyperai-data/src/lib.rs` | `1ad9ddcd6fd8d7334b99e0f5eae1bc7c7713c27f1de009141a7c5a804e2cd564` |
| `crates/hyperai-data/tests/catalog_contract.rs` | `e08784065e507f8801235d5129b605621083cf2d252992479d7495226ea72829` |
| `crates/hyperai-data/tests/query_contract.rs` | `fae583193d9ad4a0e3770eb6145456740a80a5077fd6a2c1d9ee653589d92a1e` |
| `crates/hyperai-data/Cargo.toml` | `0670ca8d428e3d8d827d9444d1de8d5e8cc86dfb4dcb306efa76ea1644143a14` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `docs/compute-bazaar-phase-2-data-packet.md` | `70501220bd09fcfd99e92f3aa65d93d65a6e42db456b7d2e50fb286ca3cf33b3` |
| `docs/compute-bazaar-data-manifest-catalog-redteam-packet.md` | `961f745f812b08b1b624be3c37b4afe414c5e8d60f7b8f7390e1f38d680385f2` |
| `docs/compute-bazaar-data-manifest-directory-entry-packet.md` | `cce0d0ec681ecb37d6c932babcf78295dd1b60855fd04b808a479849691a6eb9` |
| `docs/provenance.md` | `6e6b0ef176f996b1cafb2e0bf91264fc4dd2f11fda687d949466b2183b966ade` |
| `docs/compute-bazaar-rust-port-subplan.md` | `76175f435671169833e66b7568b485b6bde48c9cbea3ce43d12ff9cda34846c5` |
| `docs/state-slices.md` | `f298f32bdc87771f2c759bd56809612c3ece3038398ed7963db4f143556075cb` |

## Finding and bounded threat

The catalog checks directory-entry and path metadata, then opens manifests or
datasets again by pathname. `load_manifest_record` and `checked_dataset_path`
also validate a path before later `fs::read`, `File::open`, hashing, or record
counting. A concurrent writer to a disposable synthetic home can replace an
entry after those checks and before the pathname open. The query path already
passes captured dataset bytes to DataFusion's in-memory object store, but its
manifest and initial dataset validation still use separate pathname opens.

The test actor can concurrently rename, replace, or symlink files inside a
temporary HyperAI-authored synthetic data home. The target is to ensure that
catalog parsing, digest and row-count validation, and query execution use bytes
read from a handle opened relative to the selected root, without following a
symlink or reparse point at the final file component. Existing bounds remain:
64 manifest-directory entries, two known dataset IDs, 16 KiB manifests, and
16 MiB datasets.

The selected data-root path is still acquired through explicit local ambient
authority after the existing root validation. This slice does not claim
protection against replacement of the configured root or its ancestors during
that initial acquisition, a same-UID process independently reading the same
files, malicious filesystem semantics, or OS-level process isolation.

## Frozen implementation choices

- Add exact locked dependencies `cap-std = 4.0.3` and `cap-fs-ext = 4.0.3` to
  the root workspace and `hyperai-data`. Disable default features; enable only
  `cap-fs-ext`'s `std` feature. Do not enable UTF-8 conversion or networking
  features. The packages declare
  `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`.
- Open the already validated data root once as a `cap_std::fs::Dir`. Resolve
  manifest and dataset subdirectories relative to that directory. Use
  `DirExt::open_dir_nofollow` for directory components and
  `OpenOptionsFollowExt::follow(FollowSymlinks::No)` for final file components.
- Verify the opened handle is a regular file, enforce the existing byte limit
  while reading, and retain the bytes from that handle. Parse each manifest
  from its captured bytes. Compute dataset digest and row count from the
  captured dataset bytes; use those same bytes for the existing DataFusion
  in-memory query path. Do not reopen a validated manifest or dataset by path.
- Preserve current manifest schemas, Data and Market APIs, error envelope,
  result ordering, storage format, query limits, CLI/TUI/browser behavior, and
  all existing byte digests. Map symlink/reparse, directory, and non-regular
  file cases to the existing fail-closed catalog outcome.

The chosen APIs are documented by the
[`cap-std::fs::Dir` API](https://docs.rs/cap-std/4.0.3/cap_std/fs/struct.Dir.html),
[`cap-fs-ext::DirExt`](https://docs.rs/cap-fs-ext/4.0.3/cap_fs_ext/trait.DirExt.html),
and [`OpenOptionsFollowExt`](https://docs.rs/cap-fs-ext/4.0.3/cap_fs_ext/trait.OpenOptionsFollowExt.html).
The dependency project's README describes Linux, macOS, and Windows support;
the declared HyperAI platform matrix still requires native acceptance on each
target.

## Exact writable files

- `Cargo.toml` and `Cargo.lock`: register and lock the two capability-filesystem
  dependencies at the versions above.
- `crates/hyperai-data/Cargo.toml`: consume those workspace dependencies.
- `crates/hyperai-data/src/lib.rs`: bind manifest and dataset reads to
  no-follow handles and one bounded byte snapshot.
- `crates/hyperai-data/tests/catalog_contract.rs` and
  `crates/hyperai-data/tests/query_contract.rs`: add deterministic
  replacement-window, symlink refusal, snapshot digest, and query-byte parity
  cases; retain all existing tests.
- `docs/compute-bazaar-data-manifest-handle-snapshot-packet.md`: this packet
  and exact execution evidence.
- `docs/compute-bazaar-data-manifest-catalog-redteam-packet.md`: link this
  follow-up and preserve the earlier review's claim ceiling.
- `docs/compute-bazaar-rust-port-subplan.md` and `docs/state-slices.md`: link
  this continuation and report only observed evidence.

All other source trees, imported products, API schemas, local-service routes,
browser, CLI, TUI, ACP/MCP, and the ignored Bazaar checkout are read-only.

## Allowed effects and proof gates

Allowed effects: HyperAI-authored synthetic JSON and Parquet files in
test-owned temporary directories; deterministic rename and symlink/reparse
operations on those files; locked local dependency resolution; and the
validation commands below. Any outside-root sentinel is also test-owned
synthetic data in the same temporary parent. No user home, provider, registry,
Bazaar, HyperIntel, JustAI, SCM capture, network, cloud service, or external
path input is used.

Proof gates:

1. A deterministic test replaces a manifest entry with a symlink/reparse point
   after enumeration and before open. The no-follow open fails closed, the
   outside-root synthetic sentinel is not returned or hashed, and the owner
   status exposes no dataset digest.
2. A second test opens a regular manifest or dataset handle, replaces the
   directory entry, and proves later parsing, digesting, row counting, or query
   execution still uses the opened file's captured bytes.
3. A query test proves that the verified digest and DataFusion input are the
   same byte snapshot. A changed or mismatched snapshot fails closed.
4. Existing catalog identity, symlink, size, digest, SQL, result-bound,
   cancellation, and owner-status tests continue to pass. Manifest entry and
   dataset byte ceilings remain enforced when a file grows during reading.
5. Run the exact locked/offline package tests, formatter, root fast lint,
   import verifier, and hygiene checks:

   ```sh
   CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --tests
   sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check
   CARGO_NET_OFFLINE=true corepack pnpm run lint:fast
   python3 scripts/verify_imports.py
   git diff --check
   ```

6. Verify packet links and hashes, clean temporary roots, inspect all authored
   untracked files for trailing whitespace, confirm imported-tree identities,
   and leave the index empty. Report native OS results separately; a host test
   does not qualify Windows or Ubuntu behavior.

## Non-goals and assurance

This slice does not implement or qualify the Phase 2 worker, DataFusion process
isolation, numeric worker budgets, source ingestion, medallion transforms,
new datasets, API or storage migrations, broad filesystem sandboxing,
external review, or production readiness. Phase 2 remains `NOT_ASSIGNED`.
The only allowed data is the deterministic synthetic data already owned by
HyperAI. SCM, HyperIntel, JustAI, and Bazaar source/data reuse decisions remain
as recorded in [`provenance.md`](provenance.md); this packet does not change
those decisions.

Assurance target: `INTERNAL_PASS` for the exact local synthetic handle and
query tests only. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. The tests do not prove every concurrent schedule, every
filesystem implementation, native target support, same-user isolation, Data
worker containment, or correctness of external source data.

Data/artifact custody: the frozen HyperAI source, locked crate artifacts, and
temporary synthetic fixtures. Claim ceiling: bounded no-follow reads relative
to the selected local root and consistent use of the captured bytes in the
tested paths. Publish boundary: local files and evidence only; no staging,
commit, PR, release, external circulation, deployment, or production use.
