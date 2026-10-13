# Data Manifest Catalog Cardinality Packet

State slice: [`hyperai.bazaar.data_manifest_catalog_cardinality.v1`](state-slices.md#hyperai-bazaar-data_manifest_catalog_cardinalityv1).
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Previous bounded review: [duplicate-ID catalog packet](compute-bazaar-data-manifest-catalog-redteam-packet.md).

Status: local synthetic implementation packet. It authorizes a bounded
manifest-identity collection and its catalog regression only. It does not
assign the Phase 2 worker or authorize external data access.

## Finding and decision

`hyperai-data::catalog` currently parses every `.json` file in the manifest
directory and stores every embedded `dataset_id` in a `Vec` before duplicate
or supported-ID validation. The current `hyperai.data.manifest.v1` reader
supports exactly two IDs: the fixed JSON and Parquet synthetic datasets.
Repeated aliases are now rejected, but the rejection occurs after the scan;
many distinct unsupported IDs can also grow the vector before the later
catalog read rejects them.

The current v1 catalog will retain at most two distinct manifest identities.
Reject a repeated identity immediately as `InvalidCatalog`. Reject a third
distinct identity as `InvalidCatalog` before inserting it. Use an ordered set
so accepted catalog output remains deterministic. The fixed two-ID contract is
the source of this bound; admitting another dataset requires a separately
reviewed contract and a matching bound update.

This bounds retained identity state and the number of accepted distinct JSON
manifests. It does not bound the number of non-JSON directory entries that
`read_dir` must visit, prove a filesystem sandbox, or establish a whole-process
memory limit.

## Frozen baseline

- HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, branch `main`, plus the
  existing uncommitted worktree.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `a6f0634da20d7d1b6e343cbbca22650e0a4fba635b20d24ac35f0a1569a0050e`
  (174 records). Preserve unrelated dirty and untracked work.
- Frozen input hashes are recorded in the state-slice entry.

## Exact writable files

- `crates/hyperai-data/src/lib.rs`: bound unique manifest identities to the
  two supported v1 dataset IDs and reject duplicates before retaining them.
- `crates/hyperai-data/tests/catalog_contract.rs`: add a synthetic third-ID
  regression; preserve the duplicate-alias and deterministic-catalog tests.
- `docs/compute-bazaar-data-manifest-catalog-cardinality-packet.md`: this
  packet and execution evidence.
- `docs/compute-bazaar-rust-port-subplan.md`: report the narrow correction.
- `docs/state-slices.md`: this authorization and result.

Read-only inputs: `crates/hyperai-data/Cargo.toml`, `Cargo.lock`,
`docs/compute-bazaar-phase-2-data-packet.md`, the prior red-team packet,
service/client schemas, imported workspaces, provenance manifests, and the
ignored Bazaar checkout.

## Allowed effects and proof gates

Allowed effects: test-owned synthetic manifests in `tempfile` homes; one
extra manifest with a distinct unsupported ID; the focused offline Data
catalog contract suite; pinned formatting; root `lint:fast`; import
verification; documentation/link/whitespace checks; and confirmation that no
file is staged. No pre-existing service or user data is accessed.

Proof gates:

1. Before the implementation change, a three-distinct-ID fixture does not
   produce the required `InvalidCatalog` cardinality rejection.
2. After the change, the same fixture returns `InvalidCatalog`; the ordinary
   two-dataset catalog remains byte-deterministic and ready.
3. The duplicate-ID regression still returns `InvalidCatalog`, and owner
   status remains `Conflicting` without a digest.
4. Run:

   ```sh
   CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --test catalog_contract
   sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check
   CARGO_NET_OFFLINE=true corepack pnpm run lint:fast
   python3 scripts/verify_imports.py
   git diff --check
   ```

5. Scan the five writable files for trailing whitespace; confirm temporary
   roots were removed and no file is staged.

## Limits and claim ceiling

This packet can establish only a two-identity in-memory bound for the current
synthetic catalog, rejection of a third distinct manifest identity, and the
existing duplicate-ID behavior on the tested host. Non-JSON directory-entry
enumeration, same-user path-swap races, native platform behavior, source rights,
DataFusion worker isolation, and arbitrary filesystem safety remain open.
Phase 2 remains `NOT_ASSIGNED`.

Assurance target: `INTERNAL_PASS` for the exact local synthetic catalog
contract only. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. Data/artifact custody: HyperAI source and temporary
synthetic test homes. No Bazaar content, external data, provider, seller,
service, paid action, wallet, chain, or production path is used.

Publish boundary: local source and docs only; no commit, PR, release,
external circulation, deployment, or production operation.

## Execution record (2026-10-02)

The regression failed before the implementation change with exit code 101 at
the required `InvalidCatalog` assertion. `catalog()` now keeps manifest IDs in
an ordered set capped at the two supported v1 IDs, rejects duplicates and a
third distinct ID before insertion, and preserves deterministic iteration.

Validation passed:

- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --test catalog_contract` — 8 passed, 0 failed.
- `sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check` — passed.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` — passed, including 34 interface tests, 3 installer tests, workspace formatting, and strict Clippy.
- `python3 scripts/verify_imports.py` — passed for pinned Simple Compute Market (2,189 files), HyperIntel (99 files), and JustAI (176 files).

Final source digests:

- `crates/hyperai-data/src/lib.rs`: `baa1b95410919901518c6c073de92c81a97eb11c09e622a3e6936fcb5d99b8fc`
- `crates/hyperai-data/tests/catalog_contract.rs`: `6492252aae858801982fcc1a65df219c89a76e438fe7be65d40f0553fa048478`
- `docs/compute-bazaar-rust-port-subplan.md`: `0ab6e22e1e9d97740d9a9e896ecb278514a167f79a41befa413a6dad6c42cf77`

Assurance is `INTERNAL_PASS` for the exact local synthetic catalog behavior
and checks above. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Test data was created in temporary homes and removed by the
test harness. No external source data or Bazaar material was read or copied.

The bound covers retained distinct manifest identities only. Directory-entry
enumeration, total process memory, same-user path-swap races, worker isolation,
platform qualification, and full Phase 2 acceptance remain unproven. Phase 2
remains `NOT_ASSIGNED`.
