# Data Manifest Catalog Identity Red-Team Packet

State slice: [`hyperai.bazaar.data_manifest_duplicate_id_redteam.v1`](state-slices.md#hyperai-bazaar-data_manifest_duplicate_id_redteamv1).
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Related evidence: [manifest path mutation sweep](state-slices.md#hyperai-bazaar-data_manifest_path_mutation_sweepv1),
[unavailable-state projection](compute-bazaar-workload-trace-unavailable-client-parity-packet.md),
and the [Rust port subplan](compute-bazaar-rust-port-subplan.md).

Status: accepted local synthetic implementation packet. It authorizes only the
catalog identity check and temporary-home regression described here. It does
not assign the Phase 2 worker or authorize external data access.

## Frozen baseline

- Repository: HyperAI, branch `main`, base
  `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `0b5bec7e4d285a13f0b24971522373158baaf9473c2042392919e95e8d461263`
  (173 existing status records). Preserve all unrelated dirty/untracked work.
- Exact source, test, command, parent-contract, and state-slice input hashes
  are frozen in the linked state-slice record.

## Finding and threat boundary

`hyperai-data::catalog` scans each `.json` entry in the local `manifests/`
directory, parses its embedded `dataset_id`, sorts those IDs, then reloads the
canonical manifest for each ID. It did not reject duplicate IDs. Copying a
valid fixed synthetic manifest to an alias filename such as `shadow.json`
therefore made the catalog return the same canonical dataset twice.
`inspect_dataset_status_v1` then selected the first copy and returned
`bound`, hiding the conflicting directory state from the owner-backed
Workload Trace.

The bounded attacker is a writer to a disposable HyperAI synthetic data home
who can add or corrupt local manifest entries. The targeted effect is
catalog identity confusion and a false `bound` status. The test does not use
caller-selected outside paths, real data, external services, or a user home.

Existing controls include the fixed dataset-ID and relative-path allowlists,
manifest size/schema checks, root canonicalization, symlink rejection,
content digest verification, and the deterministic 1,036-candidate relative
path mutation sweep. The new change rejects repeated manifest identities
after sorting, before canonical records are loaded. A duplicate makes the
catalog return `DataError::InvalidCatalog`; the owner projection maps that
to `DatasetReadStatusV1::Conflicting` and emits no digest.

## Exact writable files

- `crates/hyperai-data/src/lib.rs`: reject duplicate IDs in the catalog scan.
- `crates/hyperai-data/tests/catalog_contract.rs`: reproduce the alias
  manifest and assert fail-closed catalog/status behavior.
- `docs/compute-bazaar-data-manifest-catalog-redteam-packet.md`: this packet
  and execution evidence.
- `docs/compute-bazaar-rust-port-subplan.md`: concise current-position update.
- `docs/state-slices.md`: the state-slice authorization and evidence.

`crates/hyperai-data/Cargo.toml`, `Cargo.lock`, API/domain schemas, service,
CLI, TUI, ACP/MCP, browser, imported products, and Bazaar reference remain
read-only. No dependency, route, storage schema, accepted path, or client
behavior changes.

## Allowed effects and proof gates

Allowed effects: test-owned synthetic JSON/Parquet data in `tempfile` roots;
copy one canonical JSON manifest to a second filename inside that temporary
`manifests/` directory; run the exact focused Data test, pinned format check,
root `lint:fast`, import verification, and diff/whitespace hygiene. Test
fixtures and temporary data are removed by test cleanup.

Proof gates:

1. On the frozen implementation, the new regression observes the alias
   catalog result and demonstrates the duplicate-identity defect.
2. After the fix, the public catalog rejects the same alias as
   `InvalidCatalog`; the owner status is `Conflicting`, not `Bound`, and no
   digest is exposed.
3. The existing deterministic relative-path mutation test still passes for
   both fixed datasets. No manifest or dataset bytes outside a temporary root
   are read or changed.
4. Run:

   ```sh
   CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --test catalog_contract
   sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check
   CARGO_NET_OFFLINE=true corepack pnpm run lint:fast
   python3 scripts/verify_imports.py
   git diff --check
   ```

5. Check authored untracked files for trailing whitespace, verify temporary
   cleanup, and confirm nothing is staged.

## Bounded red-team review and residuals

The source review covered malformed/unknown dataset IDs, duplicate embedded
IDs, parent and absolute relative paths, separator/encoding/Unicode
lookalikes, symlinked manifest/data paths, oversized manifests, and digest
mismatch. Existing tests cover the 1,036 relative-path mutations, dataset
symlink, size, and digest cases. The alias-file case was the uncovered path:
embedded identity was trusted without uniqueness validation. The new
duplicate check closes that case.

This review does not exercise concurrent same-user path replacement between
symlink validation and file open, malicious filesystem semantics, manifest
directory count exhaustion, or native behavior outside the tested host. Those
remain separate hardening/platform questions. The deterministic mutation
sweep and this review are same-owner internal evidence, not independent
security review. The bounded local path-replacement follow-up has its own
[open-handle snapshot packet](compute-bazaar-data-manifest-handle-snapshot-packet.md)
and [`hyperai.bazaar.data_manifest_handle_snapshot.v1`](state-slices.md#hyperai-bazaar-data_manifest_handle_snapshotv1)
state slice; that continuation does not expand this packet's claim ceiling.

## Claim ceiling

At most, this packet proves fail-closed handling of duplicate identities in
the fixed local synthetic manifest catalog, plus the already-recorded bounded
relative-path mutation cases on the tested host. It does not prove general
filesystem sandboxing, arbitrary source safety, DataFusion process isolation,
live data correctness, external validation, independent review, or production
readiness. Phase 2 remains `NOT_ASSIGNED`; native Ubuntu worker qualification,
numeric worker budgets, and exact worker lifecycle/confinement rules remain
open.

Assurance target: `INTERNAL_PASS` for the exact regression and bounded review
only. Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`.
Data/artifact custody: HyperAI source and disposable synthetic test homes.
Publish boundary: local source and docs only; no commit, PR, release, external
circulation, deployment, or production operation.

## Execution evidence (2026-10-02)

The new regression was first run against the frozen implementation, before
the catalog code changed. It exited `101` at the assertion requiring
`DataError::InvalidCatalog`; the alias manifest was accepted by the catalog.
After adding the duplicate-ID check, the exact locked offline
`hyperai-data --test catalog_contract` suite passed all 7 tests, including the
existing 1,036 relative-path mutations and the new owner-status regression.
The test asserts that the catalog now rejects the alias and
`inspect_dataset_status_v1` returns `Conflicting` rather than a digest-bearing
`Bound` status. The test's `tempfile` root was removed on exit.

`sh scripts/with-pinned-rust.sh cargo fmt --package hyperai-data -- --check`
passed. `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed all 34
interface tests, all 3 installer tests, workspace formatting, and strict
all-target/all-feature Clippy. `python3 scripts/verify_imports.py` passed the
2,189-file Simple Compute Market snapshot, 99-file HyperIntel snapshot, and
176-file JustAI snapshot at their recorded pins. The previous baseline failure
was the expected red phase; it does not weaken the final passing result.

Final implementation identities:

| Path | SHA-256 |
|---|---|
| `crates/hyperai-data/src/lib.rs` | `df4cdd924ab3085baa738036ce311f740bcf72b5e38d5a7fbf765f17a2f9d3ae` |
| `crates/hyperai-data/tests/catalog_contract.rs` | `4b08b36141fa9c891d3609d9848e0f756e5b06377a0f23d6b7ee1c2023b8a8d3` |
| Test executable `target/debug/deps/catalog_contract-3ce959ee176aa0a5` | `1d6ad16d52be4841b2669c148290a044a2bb74beb77a957716aecf4f570604d0` |

Assurance is `INTERNAL_PASS` for the exact duplicate-ID guard, catalog test
suite, and bounded source review only. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. Phase 2 remains `NOT_ASSIGNED`. No
provider, service, source adapter, imported tree, external data, paid action,
or production path was used.

Final hygiene also passed: `git diff --check`, the scoped trailing-whitespace
scan over the five authored paths, and the packet/state-slice link checks found
no issues. The temporary test root was removed and no files were staged. The
final NUL-delimited worktree status SHA-256 is
`a6f0634da20d7d1b6e343cbbca22650e0a4fba635b20d24ac35f0a1569a0050e` across
174 records; the single additional status record is this new packet file.
