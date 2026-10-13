# Data Manifest Directory Entry Bound Packet

State slice: [`hyperai.bazaar.data_manifest_directory_entry_bound.v1`](state-slices.md#hyperai-bazaar-data_manifest_directory_entry_boundv1).
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Previous local bounds: [duplicate-ID review](compute-bazaar-data-manifest-catalog-redteam-packet.md)
and [distinct-ID cap](compute-bazaar-data-manifest-catalog-cardinality-packet.md).

Status: local synthetic implementation packet. It authorizes a maximum of 64
manifest-directory entries per catalog read, plus at most one iterator item
to detect overflow. It does not assign or qualify the Phase 2 worker.

## Finding and decision

The catalog currently iterates every entry under the local `manifests`
directory. It considers `.json` entries, ignores other regular entries, and
retains at most two distinct dataset IDs after the prior cardinality slice.
An arbitrary number of non-JSON entries can still extend the catalog loop.

For the current v1 synthetic contract, allow at most 64 total directory
entries, including JSON files, non-JSON files, directories, and symlinks. If a
65th entry is yielded, return `DataError::InvalidCatalog` before calling
`file_type`, reading file metadata, or parsing another manifest. This keeps
the existing two-dataset catalog and bounded non-JSON tolerance while placing
a fixed ceiling on application-level entry processing. The directory order
does not affect the result. More than 64 entries require user cleanup or a
separate reviewed contract change.

This is an application-loop bound. `std::fs::read_dir` and the operating
system may buffer directory records internally; the bound is not a filesystem
sandbox or whole-process memory guarantee.

## Frozen baseline

- HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, branch `main`, plus the
  existing dirty worktree.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `5f5539257f7c554d61ab83855f315f0fc2ea05adf3f343cc97dfaa37db80fb22`
  (175 records). Preserve all unrelated dirty and untracked work.
- Frozen input hashes are recorded in the state-slice entry.

## Exact writable files

- `crates/hyperai-data/src/lib.rs`: stop catalog-level directory processing
  after 64 entries and return the stable invalid-catalog error on overflow.
- `crates/hyperai-data/tests/catalog_contract.rs`: add a test that accepts 64
  entries, then rejects the 65th; preserve the deterministic, duplicate-ID,
  and distinct-ID cases.
- `docs/compute-bazaar-data-manifest-directory-entry-packet.md`: this packet
  and execution evidence.
- `docs/compute-bazaar-rust-port-subplan.md`: record the bounded outcome and
  remaining Phase 2 limits.
- `docs/state-slices.md`: this authorization and result.

Read-only inputs: `crates/hyperai-data/Cargo.toml`, `Cargo.lock`, the Phase 2
Data contract packet, prior catalog review packets, service/client schemas,
import manifests, and the ignored Bazaar reference.

## Allowed effects and proof gates

Allowed effects: at most 65 empty files in disposable `tempfile` homes; the
focused offline Data catalog contract suite; pinned formatting; root
`lint:fast`; import verification; and documentation, link, whitespace, and
staging checks. No pre-existing service or user data is accessed.

Proof gates:

1. Before the implementation change, a fixture with 64 entries returns a
   ready two-dataset catalog, and the new assertion that a 65th entry returns
   `InvalidCatalog` fails.
2. After the change, exactly 64 entries remain ready and deterministic; the
   65th entry returns `InvalidCatalog` before its `file_type` or contents are
   inspected.
3. Existing deterministic two-dataset, duplicate-ID, and third-distinct-ID
   tests continue to pass.
4. Run:

   ```sh
   CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --test catalog_contract
   sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check
   CARGO_NET_OFFLINE=true corepack pnpm run lint:fast
   python3 scripts/verify_imports.py
   git diff --check
   ```

5. Scan the five writable files for trailing whitespace; remove temporary
   roots and confirm nothing is staged.

## Limits and claim ceiling

This packet can establish only that the current host's catalog loop processes
at most 64 manifest-directory entries and rejects the overflow sentinel, with
the fixed synthetic catalog regressions passing. It does not limit OS-level
directory buffering, filesystem races, overall process memory, DataFusion
worker resource use, platform behavior, source rights, or Phase 2 acceptance.
Phase 2 remains `NOT_ASSIGNED`.

Assurance target: `INTERNAL_PASS` for the exact local synthetic catalog
contract only. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. Data/artifact custody: HyperAI source and empty temporary
test files. No Bazaar content, external data, provider, seller, service, paid
action, wallet, chain, or production path is used.

Publish boundary: local source and docs only; no commit, PR, external release,
circulation, deployment, or production operation.

## Execution record (2026-10-02)

The 65-entry regression failed on the frozen implementation with exit code
101 because the catalog returned a ready result. The implementation now uses
`enumerate()` and returns `InvalidCatalog` when the iterator yields entry 65,
before calling `file_type`, reading metadata, or parsing that entry. Exactly
64 entries still produce the ready two-dataset catalog.

The first `lint:fast` attempt caught Clippy's `explicit_counter_loop` warning
on a manual counter. The implementation was changed to the iterator index;
the final fast gate then passed.

Final validation:

- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --test catalog_contract` — 9 passed, 0 failed.
- `sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check` — passed.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` — passed, including 34 interface tests, 3 installer tests, workspace formatting, and strict Clippy.
- `python3 scripts/verify_imports.py` — passed for pinned Simple Compute Market (2,189 files), HyperIntel (99 files), and JustAI (176 files).

Final source digests:

- `crates/hyperai-data/src/lib.rs`: `1ad9ddcd6fd8d7334b99e0f5eae1bc7c7713c27f1de009141a7c5a804e2cd564`
- `crates/hyperai-data/tests/catalog_contract.rs`: `e08784065e507f8801235d5129b605621083cf2d252992479d7495226ea72829`
- `docs/compute-bazaar-rust-port-subplan.md`: `37634fed66712d52319327181cceb2862d38e276d1fc2453284dbbfa84c2b046`

Assurance is `INTERNAL_PASS` for the exact local synthetic catalog boundary
and checks above. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Test files were empty, temporary, and removed by the test
harness. No external source data or Bazaar material was read or copied.

The cap applies to entries processed by the application loop. It does not
bound OS-level directory buffering, whole-process memory, filesystem races,
worker isolation, or native platform behavior. Phase 2 remains
`NOT_ASSIGNED`.
