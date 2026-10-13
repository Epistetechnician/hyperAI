# HyperAI Local Storage `HomeBusy` Diagnostic Packet

State slice: `hyperai.bazaar.desk_runtime.storage_busy_diagnostic.v1`.
Status: bounded test-only diagnosis completed; prior failure cause remains
inconclusive.
Parent: [Phase 1 local storage contract](compute-bazaar-phase-1-contract-packet.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Observed failure and source path

The 2026-10-01 root `verify:full` run failed once in
`storage::restore_failure_tests::failed_row_copy_rolls_back_and_reports_the_safety_backup`
with `HomeBusy`. The full suite must not be repeated blindly.

Current source shows the test's `initialize_home` call returns
`InitializationBusy` directly. The following `backup_metadata` call maps
that same initialization-lock error to `HomeBusy`. The test calls the private
`restore_metadata_rows` helper directly, so it does not acquire the public
restore service lock. The current evidence therefore points to the
`backup_metadata` step, but no call-site context was retained with the prior
failure.

This diagnostic adds test-only context to the initialization and backup
calls, then runs only the `hyperai-local-service` library unit-test target
with default parallel test execution. Production storage code and behavior
remain unchanged.

## Frozen baseline and effects

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the current dirty/untracked tree preserved. Starting
`crates/hyperai-local-service/src/storage.rs` was 38,596 bytes with SHA-256
`405ac552285fb2b48c6d5c4e254349ce9675b850d2da89b42bdf3b62cf5b1c84`.

Exact source edit: add stage labels only to the unit test's `initialize_home`
and `backup_metadata` error paths. No production code, service lifecycle,
lock policy, database behavior, or test assertion changes.

The bounded command is:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test \
  -p hyperai-local-service --lib -- --nocapture
```

The test target uses disposable local homes and may spawn its own synthetic
scorer child. It does not require any provider, seller, registry, browser,
pre-existing service, external host, wallet, chain, network access, or
persistent user data. Root `verify:full` is excluded from this diagnostic.

## Proof gates and claim ceiling

1. If the storage test fails, its output identifies whether initialization or
   backup returned the error.
2. The bounded crate library unit suite completes and records its exact
   pass/fail output.
3. Pinned Rust formatting, root `lint:fast`, import verification, and
   `git diff --check` pass.
4. No production storage code or unrelated dirty work changes.

Claim ceiling: evidence about the one local storage-test path and its
test-isolation behavior only. Reviewer status is `NOT_REQUESTED`; reproduction
is `NOT_REPRODUCED`. A passing package suite would not explain why the prior
root suite failed. A repeated failure requires source-level diagnosis before
any further retry. Root `verify:full` remains unverified until the failure is
explained and the permitted broader gate completes.

## Result (2026-10-01)

The post-change source was 38,804 bytes with SHA-256
`449e748f5d28043f495130c749968d121cf34e98b137e790b31d97817cda82c6`.
Toolchain: Rust `1.93.1 (01f6ddf75 2026-02-11)` and Cargo
`1.93.1 (083ac5135 2025-12-15)`.

The exact offline `hyperai-local-service --lib` test command exited 0 with
11 passed, 0 failed, and 0 ignored under default parallel libtest execution.
The instrumented restore-failure test passed; neither labeled call returned
an error. Therefore the previous `HomeBusy` did not recur in this bounded
package run, and its cause remains inconclusive. This package result does not
explain the prior root `verify:full` failure.

`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` exited 0: 21 interface
tests and 3 installer tests passed, pinned formatting passed, and strict
all-target/all-feature Clippy passed. `python3 scripts/verify_imports.py`
passed Simple Compute Market (2,189 files), HyperIntel (99), and JustAI
(176). `git diff --check` passed. No production storage logic changed.

The test used its own temporary home, fixed synthetic SQLite values, and the
existing synthetic scorer test child; temporary homes were test-owned and no
result artifact was persisted outside the tool output. No network, provider,
browser, listener, or user home was used. Assurance is `INTERNAL_PASS` for
this exact local-service library test set. For diagnosis of the previous root
failure, status remains `INCONCLUSIVE`; reviewer status is `NOT_REQUESTED`,
reproduction is `NOT_REPRODUCED`, and root `verify:full` remains unverified.
Next gate: determine a supported cause before another full-workspace retry.
