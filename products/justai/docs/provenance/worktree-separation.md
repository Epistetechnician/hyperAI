# Worktree separation for the governed discovery slice

State slice: `justai-governed-discovery-monorepo-v1`.

The target checkout contained an existing dirty change in `crates/justai-contracts/src/records.rs` and acquired concurrent evidence-bridge/observatory changes while the governed slice was being built. Those changes remain in place. They are not evidence for this slice and are not included in the governed review commit.

`manifests/migration/freeze.json` defines three classes:

- `owned_paths`: new governed implementation, contracts, protocols, manifests, documentation, and tests.
- `mixed_paths`: files where only specific hunks may be staged for the governed slice.
- `preserved_concurrent_paths`: files left uncommitted because their changes belong to the existing observatory/evidence-bridge work or predate this slice.

The separation rule is exact: no whole mixed file is staged unless a read-only diff review establishes that every hunk belongs to this state slice. No reset, checkout, clean, broad deletion, source-repository mutation, or source archiving is allowed.

The freeze is a scope freeze, not an authorization. Model execution, provider calls, paid execution, live trading, external corpus acquisition, scientific assessment, hosted enforcement, release authority, and kill paths remain closed.
