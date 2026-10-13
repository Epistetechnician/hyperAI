# HyperAI Compute Desk Phase 2: Local Data Contract

Status: accepted local implementation packet under state slice
`hyperai.bazaar.data_local.v1`.

Parent: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Product framing: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Planning baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on
`main`, plus the existing uncommitted monorepo and Phase 1 implementation.

This packet makes the Data workspace the HyperAI-owned local catalog and query
surface. It keeps Simple Compute Market capture exploration in its current
caller-pinned, in-memory path. No Bazaar implementation or asset is included.

## Outcome

The same Rust-owned Data operations serve the authenticated loopback API, the
`hyperai data` CLI, the terminal desk, and the browser workspace. The first
dataset is an explicitly created, deterministic HyperAI synthetic snapshot.
Users can list and inspect its manifest, run a bounded read-only SQL query, and
save a query model or display view beneath their own HyperAI home. The UI shows
dataset identity, source label, snapshot digest, freshness state, query limits,
and the synthetic claim ceiling beside the result.

The feature does not ingest or fetch upstream data. It creates no inference
route, seller offer, deal, outcome, payment, public index, or settlement
record.

## Dependency decision

Pin Apache DataFusion `=52.5.0` with default features disabled and enable
`sql`, `parquet`, and `recursive_protection`; its JSON reader is built in and
has no separate Cargo feature. Pin its `sqlparser` AST directly at
`=0.59.0`, with only `std` and `visitor`, for the pre-plan read-only
allowlist. Its Apache-2.0
workspace manifest declares Rust 1.88.0, which fits the monorepo's pinned Rust
1.93.1 toolchain. DataFusion 55.1.0 declares Rust 1.94.0, so it cannot be
selected without a separate toolchain slice. The DataFusion 52.5 workspace
manifest includes separate JSON and Parquet data-source crates and Arrow
57.1.0; keep all Arrow-facing types inside `hyperai-data` so the engine does
not become a monorepo-wide contract. See the [52.5.0 manifest](https://raw.githubusercontent.com/apache/datafusion/52.5.0/Cargo.toml),
the [55.1.0 manifest](https://raw.githubusercontent.com/apache/datafusion/55.1.0/Cargo.toml),
and [DataFusion's SQL DDL documentation](https://datafusion.apache.org/user-guide/sql/ddl.html).

The implementation must pin transitive resolution through `Cargo.lock`,
record the resulting dependency tree and enabled features in the review notes,
and measure a clean compile's peak memory before expanding features. If the
selected version or feature names cannot build on the pinned toolchain, stop
and amend this packet and the state-slice record before changing the Rust
toolchain or widening features.

## Contracts

### Catalog and manifests

- `hyperai.data.catalog.v1` lists only catalog entries verified beneath the
  HyperAI data root. An entry carries a stable dataset ID, schema version,
  format, source label, generated/observed time, snapshot digest, schema
  digest, row count when known, freshness classification, and claim ceiling.
- `hyperai.data.manifest.v1` binds the dataset ID and schema to a root-relative
  file path, format (`json` or `parquet`), byte length, SHA-256 content digest,
  schema digest, source/provenance label, and deterministic snapshot time.
- Reject absolute paths, parent traversal, symlinks, duplicate IDs, unknown
  manifest versions, digest/size mismatch, unsupported schemas, and files that
  exceed the configured byte ceiling. Canonicalize each accepted path and
  verify it remains under the selected data root before opening it.
- `data seed-synthetic` creates one deterministic sample release under the
  local HyperAI data root using atomic create-without-overwrite behavior.
  Repeating the command returns the same manifest digest and never overwrites
  a changed local file. Tests generate their own synthetic JSON and Parquet
  inputs in temporary homes; no new upstream fixture is imported.
- Missing, invalid, stale, and unknown freshness remain distinct statuses.
  Missing values remain null/unknown; never coerce them to zero.

### Query boundary and resource limits

- Accept exactly one SQL `SELECT` statement over tables registered from a
  verified local manifest. Parse the statement before planning. Reject DDL,
  DML, `COPY`, `SET`, multiple statements, recursive CTEs, unknown tables,
  table-valued functions, extension loading, and expressions outside the
  crate's documented function allowlist.
- Register local scans in Rust from verified manifest paths. Do not expose
  SQL-created tables or arbitrary file/object-store URLs. No network-enabled
  object-store feature is selected. Spill and temporary disk writes are off.
- Defaults and hard ceilings: 4 KiB SQL text, 16 MiB per source file, 16 MiB
  maximum bytes scanned per query, 2 seconds wall time, 64 MiB query memory,
  one execution partition, 1,024-row batches, 1,000 result rows, and 256 KiB
  serialized result bytes. A caller may request lower values only. Exceeding a
  ceiling returns a stable typed limit error and no partial success result.
- Stream record batches and enforce row/serialized-byte limits before appending
  output. Propagate explicit cancellation through planning and execution. Do
  not claim a memory bound based only on DataFusion configuration: record the
  actual host, toolchain, fixture digest, query, peak-RSS method, and measured
  result in the implementation snapshot.
- JSON and CSV output use stable column order from the planned schema, stable
  null encoding, and UTF-8. A repeated query over the same verified snapshot
  emits byte-identical output. CSV contains a header and RFC-compatible
  escaping.

DataFusion is an in-process Arrow engine. Its configuration defaults include
parallel partitions and an unset memory limit, so the implementation must set
the packet's lower partition and memory ceilings explicitly. Its SQL supports
external-table DDL and file/remote locations; those statements must remain
unavailable by never exposing DDL or unregistered scan functions. See the
[DataFusion memory and execution configuration](https://datafusion.apache.org/user-guide/configs.html)
and [DDL reference](https://datafusion.apache.org/user-guide/sql/ddl.html).

### Process isolation design gate

The staged platform order and current disposition are recorded in the
[Data worker platform decision](compute-bazaar-data-worker-platform-decision-v1.md):
Linux cgroup v2 is the first qualification candidate, Windows Job Objects are
a follow-on candidate, and bounded worker mode remains unavailable on
macOS. No platform is qualified yet, and existing local query behavior is
unchanged.

The `RuntimeEnv` memory pool is a DataFusion query-execution budget. It is not
a whole-process memory limit. The local API's two-slot semaphore limits
admitted operations, not bytes. Peak RSS results are workload measurements,
not enforcement. Keep those three facts distinct in UI copy, reports, and
acceptance records.

A candidate follow-on architecture is a supervised, per-query worker behind
the current HyperAI service. The parent retains authentication, caller
authorization, SQL validation, manifest and path checks, request limits, API
error mapping, and persistence. The worker receives only a versioned bounded
request for one authorized query and one immutable digest-bound snapshot. It
has no credentials, general HyperAI home access, network, or authority to
change records. The DataFusion pool remains enabled inside the worker; an
independently enforced operating-system limit would contain process memory.

The parent owns the wall-time deadline, cancellation, worker termination and
reaping, bounded stderr/result handling, complete-response validation, and
no-partial-success behavior. The transport-neutral framing candidate, byte
ceilings, snapshot handoff, typed failure mapping, and lifecycle contract are
recorded in the [Data worker IPC packet](compute-bazaar-data-worker-ipc-contract-v1.md).
A later implementation packet must freeze the platform transport, worker
identity, process-tree containment, supported platforms, and total capacity,
then test those contracts through the shared service so API, CLI, TUI,
browser, and ACP keep one execution authority and compatible errors.

Platform enforcement is a separate proof gate. Linux cgroup v2 documents
`memory.max`; Windows Job Objects can manage process groups and set a
job-wide committed-memory limit. The native macOS v3 probe set soft
`RLIMIT_AS` to `TASK_VM_INFO.virtual_size + 64 MiB`: the baseline was
445,746,020,352 bytes (about 415 GiB) and one 128 MiB allocation was refused
with `ENOMEM`. The raw `ru_maxrss=1392640` is not a demonstrated worker limit
or RSS ceiling. Based on this result, do not select `RLIMIT_AS` as a useful
macOS DataFusion worker memory budget: it constrains address-space growth from
a very large process baseline, not the worker's resident-memory use. Native
macOS worker enforcement remains unresolved; a separate platform spike must
identify and qualify an alternative mechanism before a bounded-worker claim.
Do not claim a bounded worker mode on any platform until its native limit
behavior, cancellation, crash recovery, cleanup, and combined
service-plus-worker capacity are demonstrated. If a platform cannot enforce
the selected boundary, keep the bounded mode unavailable there. These OS
mechanisms are references for future spikes, not selected or implemented
dependencies ([v3 macOS probe](compute-bazaar-data-macos-resource-limit-probe-v3.md),
[Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html),
[Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[Windows job-wide committed-memory limit](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information)).

The first native macOS `RLIMIT_AS` probe is `INCONCLUSIVE`. On macOS 26.6.1
arm64, the child received `EINVAL` when requesting a 128 MiB soft limit and
exited before allocation; its executed format strings also differed from the
source frozen in the packet. Current [Apple XNU resource-limit code](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c)
routes `RLIMIT_AS` to `vm_map_set_size_limit`, which may reject a limit below
the process's current address-space usage. This run did not measure that
usage, so the explanation remains unconfirmed. The exact result and custody
are in the [native probe packet](compute-bazaar-data-macos-resource-limit-probe.md)
and [state-slice record](state-slices.md#hyperai-bazaar-data_macos_rlimit_probev1).
A corrected attempt requires a fresh state slice and must record the baseline
virtual-map size first; the v1 packet authorizes no retry. A separate v2
probe recorded `TASK_VM_INFO.virtual_size` at 445,746,184,192 bytes, set the
soft limit 64 MiB above that baseline, and observed a 128 MiB allocation
refused with `ENOMEM`. The overall v2 slice is `INCONCLUSIVE` because its
frozen proof gate required an RSS report, which the helper did not emit on
allocation refusal. This raw result does not establish an RSS ceiling. The
baseline is about 415 GiB of virtual address space, not RSS; the result does
not establish a useful whole-process or DataFusion worker memory budget.
Native macOS worker qualification remains open. A fresh slice must report RSS
on both allocation branches. See the [v2 packet](compute-bazaar-data-macos-resource-limit-probe-v2.md)
and [v2 state slice](state-slices.md#hyperai-bazaar-data_macos_rlimit_probev2).
A v3 continuation emitted raw `ru_maxrss` on the refusal branch: the baseline
was 445,746,020,352 bytes of virtual address space, the soft limit was 64 MiB
above baseline, the 128 MiB allocation returned `ENOMEM`, and the raw
`ru_maxrss` value was 1,392,640. The exact native helper gates are
`INTERNAL_PASS`; the result still does not prove an RSS ceiling, useful
whole-process budget, or DataFusion worker behavior. Native macOS worker
qualification remains open. See the [v3 packet](compute-bazaar-data-macos-resource-limit-probe-v3.md)
and [v3 state slice](state-slices.md#hyperai-bazaar-data_macos_rlimit_probev3).

This is a design proposal only under
[`hyperai.bazaar.data_query_process_isolation_design.v1`](state-slices.md#hyperai-bazaar-data_query_process_isolation_designv1).
No worker, OS resource limit, new error code, API change, or capacity target
is authorized by that documentation or the linked planning-only
[`hyperai.bazaar.data_worker_ipc_contract_design.v1`](state-slices.md#hyperai-bazaar-data_worker_ipc_contract_designv1).
macOS `RLIMIT_DATA` candidate probe (2026-10-01): the exact helper compiled
once with exit 0, then its sole execution stopped in the first child because
`setrlimit(RLIMIT_DATA, 64 MiB)` returned `EINVAL` (errno 22; helper exit 72).
No allocation was attempted, no raw `ru_maxrss` was emitted, and the `mmap`
and `malloc` cases did not run. The temporary directory was removed, and
Simple Compute Market, HyperIntel, and JustAI import digests matched before
and after. This is `INCONCLUSIVE`. Current public XNU source returns `EINVAL`
when the requested data limit is below current usage; because this probe did
not capture that baseline, this cause is plausible but unconfirmed. The run
establishes no allocation or DataFusion memory bound. Do not retry under
the same slice. The next step is a fresh named design slice for another
macOS worker-isolation mechanism; no runtime behavior is authorized. See the
[native probe packet](compute-bazaar-data-macos-rlimit-data-probe-v1.md) and
[state-slice record](state-slices.md#hyperai-bazaar-data_macos_rlimit_data_probev1).

macOS isolation options (2026-10-01): the documentation-only assessment
recommends a separately scoped Virtualization.framework Linux guest as the
leading feasibility candidate, with a fixed guest memory size and optional
cgroup v2 worker cap. The guest, entitlement/signing path, no-network IPC,
OOM/cancellation behavior, cleanup, and combined host/guest capacity remain
unverified. The recommendation does not authorize a VM, dependency, API, or
platform-support change. If its entitlement/package path is infeasible,
macOS bounded-worker support stays unqualified. See the [options packet](compute-bazaar-data-macos-isolation-options-v1.md)
and [state-slice record](state-slices.md#hyperai-bazaar-data_macos_isolation_optionsv1).

### Local API concurrency

The authenticated loopback service uses one shared two-slot semaphore for
`POST /api/v1/data/query`, the equivalent read-only agent-tool query route,
and `GET /api/v1/data/synthetic-release`. If both slots are occupied, another
request to either operation fails immediately with HTTP 429 and API error code
`DATA_QUERY_BUSY`; requests are not queued. The limit is a concurrency count,
not a process-memory bound or DataFusion allocation guarantee. A focused
service test holds both query handlers before dataset access and checks that
an additional query and synthetic-release request receive the same rejection.

The focused regression passed on 2026-09-30 under
`hyperai.bazaar.data_query_capacity_contract.v1`: with Rust/Cargo 1.93.1, the
locked offline `hyperai-local-service` library test returned 1 passed, 0
failed. It observed HTTP 429 and `DATA_QUERY_BUSY` for both over-capacity
requests. Both admitted query handlers were stopped at a test-only gate before
dataset access, so this result covers API back-pressure only. It does not
measure runtime memory, prove DataFusion isolation, or close Phase 2, which
remains `NOT_ASSIGNED`.

### Saved models and views

- `hyperai.data.model.v1` stores one name, one validated SQL `SELECT`, one
  dataset ID, and its exact manifest digest. The saved-definition receipt
  returns the digest used by views; each query result carries the source
  manifest's claim ceiling. Listing reports whether the input snapshot remains
  current, stale, or missing.
- `hyperai.data.view.v1` stores a view name, saved-model digest, selected
  columns, and one of the bounded `table`, `bar`, or `line` presentation
  modes. It stores no record payload, query result, credential, or source bytes.
- Create and update are explicit user actions and write atomically beneath
  `<HyperAI home>/data/models/` and `<HyperAI home>/data/views/`. Refuse
  symlink/path escape, name collision without an explicit replace action, and
  stale model/input digest mismatch. Existing saved bytes are never silently
  overwritten.
- Service mutations use the Phase 1 idempotency and server-recomputed digest
  contract. Read/query routes remain read-only. There is no generic job route
  in this phase; queries must finish or cancel within the synchronous ceiling.

## Client surface

```text
hyperai data seed-synthetic
hyperai data --home <path> tables
hyperai data --home <path> describe <dataset-id>
hyperai data --home <path> sql <dataset-id> --query <select> [--format json|csv] [--max-rows N]
hyperai data --home <path> model list|run <model-id>|preview <definition.json>|save <definition.json>
hyperai data --home <path> view list|preview <definition.json>|save <definition.json>
```

The CLI requires the authenticated local service to be running and emits a
machine-readable JSON response; CSV is included as the query result's `csv`
field. Model and view definition files are JSON input to the CLI's explicit
preview/save commands. The TUI exposes catalog rows, source, freshness, saved
definition state, and a bounded SQL query editor. Press `e` to edit, Enter to
run, Ctrl+U to clear while editing, and Esc to cancel editing while retaining
the draft. It sends the query through the same authenticated API, displays a
bounded result or stable sanitized service error, clears stale results when a
new query is edited or submitted, and keeps query text and result data in TUI
memory only. The browser Data workspace has two visibly
separate sections: **HyperAI local datasets** and **Verified SCM capture**.
Its query editor uses the same API, disables query submission while work is
active, offers cancellation, and requires a separate preview and confirm step
for model and view writes. Results remain in browser memory; definitions are
stored locally without query-result payloads.

When the local catalog is empty, its Data workspace also exposes **Create
local synthetic snapshot**. That explicit action uses the existing preview
and seed contracts for the fixed HyperAI-owned JSON/Parquet fixture, then
refreshes the read-only workload trace so the verified manifest can appear as
a source-labelled Data reference. It does not fetch market data or activate a
Bazaar, SCM, HyperIntel, JustAI, provider, or remote source.

## Implementation files

The exact Phase 2 implementation map is:

- `Cargo.toml`, `Cargo.lock`.
- `crates/hyperai-data/Cargo.toml`, `src/lib.rs`,
  `tests/catalog_contract.rs`, `tests/query_contract.rs`.
- `crates/hyperai-application/Cargo.toml`, `src/lib.rs`.
- `crates/hyperai-local-service/Cargo.toml`, `src/server.rs`,
  `tests/data_api.rs`.
- `crates/outcomes-cli/src/lib.rs` and
  `crates/outcomes-cli/tests/cli_compat.rs` for the authenticated Data commands
  and local loopback readiness contract.
- `crates/hyperai-tui/src/lib.rs`.
- `apps/hyperai-interface/app.js`, `app.css`.
- `README.md`, `docs/state-slices.md`,
  `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`.

If implementation needs another file, stop and amend the accepted state slice
before touching it. Do not change existing outcome/market schemas, the SCM
capture adapter, HyperIntel, JustAI, Simple Compute Market, Bazaar checkout,
package scripts, toolchain pin, or import manifest.

## Acceptance and report

Run pinned-toolchain focused `hyperai-data`, application, service, TUI, and CLI
checks; root `corepack pnpm run lint`; JavaScript syntax and local browser
checks; and Markdown/link hygiene. Tests cover manifest determinism and
tampering, schema and path failures, SQL rejection cases, JSON/Parquet reads,
cancellation, each limit, stale/absent source labels, byte-stable JSON/CSV,
model/view digest binding and atomic refusal, API auth/errors, and client
compatibility. These are acceptance targets; the validation snapshot below
records which were exercised and which remain open. Run no nested product
workspace suites unless their source is changed; report root and nested
workspace results separately.

Evidence reporting must name revision and state slice, assurance level,
reviewer status, reproduction status, fixture and artifact custody, measured
memory method/result, known limitations, claim ceiling, and the next blocked
authority gate. A local test does not establish independent review, rights to
external data, source truth, or production readiness. Non-synthetic import,
network access, and external publication each require their own packet and
authority.

## Validation snapshot

Snapshot date: 2026-09-24. Baseline revision:
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the exact included
files listed above in the existing dirty working tree. No commit or frozen
working-tree digest was created.

- `corepack pnpm run lint` passed: pinned formatting, strict workspace Clippy,
  workspace contract tests, and `cargo test --workspace --all-features`.
- Focused `hyperai-data`, application, local-service, TUI, and CLI tests passed
  as part of the root suite. The service/API tests cover authentication,
  JSON/Parquet queries, seed and definition idempotency, cancellation, and
  rejection paths. The root output marks opt-in PostgreSQL and Docker tests as
  ignored because their external local services/images were not selected.
- A disposable-home CLI smoke passed service start/stop, synthetic seed,
  catalog reads, JSON manifest inspection, CSV query, and model/view inventory. The pinned
  import verifier passed for Simple Compute Market, HyperIntel, and JustAI at
  their recorded revisions. JavaScript syntax and `git diff --check` passed.
- Interactive browser acceptance passed for the synthetic catalog, query, model
  preview/save, and view preview/save flows. The saved entries appeared in the
  local inventory and reported their definition digests. A 390 px viewport had
  no document-level horizontal overflow; keyboard activation reran a query and
  exposed its completion message. Browser console errors were empty. Full
  keyboard navigation, screen-reader, reduced-motion, and broader responsive
  acceptance remain open.

- Focused `hyperai-data` tests now include 128-case bounded property testing of
  arbitrary SQL bytes, explicit JSON and CSV serialized-output ceiling checks,
  and a DataFusion pool reservation test showing a 64 MiB reservation succeeds
  while one additional byte is rejected. These exercise the parser boundary,
  serializer, and configured pool primitive; they do not establish that every
  DataFusion allocation is accounted for.

## Local follow-up (2026-09-25)

Under `hyperai.bazaar.data_local.v1`, a focused query-level test groups 1,000
synthetic rows by distinct 12 KiB keys. The exact temporary input is 12,039,000
bytes with SHA-256
`675823dba2fc6f3c2c6d2fa5f1ebc46e61036707e66c4654e98c1d4ee9d1d555`. The SQL
`SELECT amount FROM dataset GROUP BY group_key, amount` returns small integer
values, keeping the result within the row and serialized-byte ceilings. With a
test-selected 1 MiB pool, the executor returns stable `LimitExceeded` without
partial rows. With the configured 64 MiB pool, `run_query` returns all 1,000
rows within its 2-second timeout; the test completed in 0.40 seconds. The
fixture stays beneath the 16 MiB input ceiling and is removed with its
disposable test directory.

For the 64 MiB query, `/usr/bin/time -l` measured three fresh sequential test
processes on Darwin 26.6.1 arm64: 94,814,208, 94,846,976, and 94,765,056 bytes
peak RSS (median 94,814,208 bytes / 90.422 MiB). Three idle runs of the same
test binary with no matching test measured 9,453,568, 9,404,416, and 9,404,416
bytes (median 9,404,416 bytes / 8.969 MiB). Each run regenerated and removed
the temporary fixture; no application cache or concurrent test workload was
used. Rust version: `1.93.1 (01f6ddf75 2026-02-11)`. The test source SHA-256 is
`d1afa9f498f6c47dc76eff083f52e6c2a827389f73e48148a0e7490abd6980d2`; the test
executable SHA-256 is
`39e3fd3a2c58df58cd8d313fd9712c21d98f6cb09467794ac2e52f8d4f0faab9`.

This shows the engine accepts the workload under its 64 MiB pool and rejects
it under the 1 MiB pool. The 64 MiB pool does not cap process RSS; this test
process reached a 90.422 MiB median peak and includes the test harness,
allocator, temporary input, and engine allocations. It does not establish
complete allocation accounting or production memory capacity. The focused
`hyperai-data` package tests, strict scoped Clippy, and pinned Rust formatting
passed. Assurance remains `NOT_ASSIGNED`; reviewer status is `NOT_REQUESTED`;
reproduction status is `NOT_REPRODUCED`.

On Darwin 25.6 arm64, `/usr/bin/time -l` observed 54,919,168 bytes maximum RSS
(52.38 MiB) for the final local service process after synthetic seed and one
three-row JSON query. The fixture SHA-256 was
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`; the
response reported a 67,108,864-byte DataFusion memory pool and a 2,000 ms
query limit. This single measurement is not a bound or production capacity
claim.

Assurance level: `NOT_ASSIGNED`. The bounded property test is a local parser
robustness check, not a target-specific fuzz campaign. A bounded red-team
review record, complete allocation accounting, process-level memory bound,
screen-reader/reduced-motion acceptance, and clean second-environment
reproduction remain incomplete. A
pending-future test exercises the same timeout helper used by query execution.
Reviewer status: `NOT_REQUESTED`.
Reproducibility status: `NOT_REPRODUCED`; no clean second environment was
used. Fixture data and query results stayed in disposable local homes, which
were removed after validation. The next authority gate remains per-source
rights, data classification, freshness, and retention approval before any
non-synthetic import or source connection.

### Local follow-up: reject `SELECT INTO` (2026-09-25)

State slice: `hyperai.bazaar.data_local.v1`. Baseline commit:
`dcf41e31aa1f98438a6f8ec6b253801e26dc0aa`; the scoped implementation and
regression test remain uncommitted in the existing dirty working tree. The
validator now checks the parsed top-level `Select.into` field and rejects it
before query planning. The contract test confirms that SQLparser's
`GenericDialect` parses `SELECT gpu_model INTO TABLE exported FROM dataset`
as a `SELECT` with `into` populated, then confirms the Data API returns
`QueryRejected`.

On macOS 26.6.1 arm64 with Rust `1.93.1 (01f6ddf75 2026-02-11)`,
`sh scripts/with-pinned-rust.sh cargo test --locked -p hyperai-data` passed
all 20 package tests. The authenticated service-route
regression `query_api_rejects_unregistered_functions_and_cancel_unknown_query`
returned HTTP 422 and `DATA_QUERY_REJECTED` for the same SQL; all four tests in
the `hyperai-local-service --test data_api` target passed. Strict Clippy passed
for `hyperai-data` and `hyperai-local-service` with all targets; formatting
checks passed for both packages. No-index whitespace checks against `/dev/null`
emitted no diagnostics for the five changed, untracked files. The source and
test SHA-256 digests are
`03d281e7fa499bc3d5bc9d94db05c1b1a0062da8e8063d26b826a82c30b19773`,
`ed64fa6470582f15277080f10324bc70ab82a601f049fbfbc97e804ddd8d7a76`, and
`7a85149e98976f8bb8a52902d061e2d7d0b523985ba652953462cdc43214650b`.

Assurance remains `NOT_ASSIGNED`; reviewer status is `NOT_REQUESTED`, and
reproduction status is `NOT_REPRODUCED`. Test inputs are the HyperAI-owned
synthetic fixture in disposable local homes; no source, network, or external
data was used. This closes only the `SELECT INTO` parser gap. It does not
establish complete allocation accounting, a process-level memory bound,
accessibility acceptance, bounded adversarial review, or clean reproduction.
The claim ceiling remains bounded read-only behavior over local synthetic
data. The next source gate remains rights, classification, freshness, and
retention approval before any non-synthetic import or connection.

### Local follow-up: query status and stale-result handling (2026-09-25)

State slice: `hyperai.bazaar.data_local.v1`, baseline commit
`dcf41e31aa1f98438a6f8ec6b253801e26dc0aa`. The browser Data query now moves
focus to a `role="status"`, `aria-live="polite"` message while a query runs and
after it completes or fails. Starting a new query clears the previous result
and digest-bound model/view previews, so a rejected query cannot leave earlier
rows visible as if they belonged to the failed request. The existing service
error banner remains alongside the query-specific failure status.

On macOS 26.6.1 arm64, a fresh pinned `hyperai` build served the updated shell
from an isolated disposable home on loopback port 4318. With the HyperAI
synthetic JSON/Parquet sample (JSON manifest SHA-256
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`), browser
checks covered keyboard-submitted success (3 rows), rejected `SELECT INTO`,
focus restoration on both outcomes, status role/live attributes, removal of
the prior successful result after failure, and a 390 px viewport with no
document or body horizontal overflow. Browser console errors were empty.
`node --check apps/hyperai-interface/app.js` passed. The disposable service
was stopped and its temporary home removed; the pre-existing service on port
4317 was left running.

Browser asset SHA-256: `apps/hyperai-interface/app.js`
`5dce7ccf89dff9cd06a9f9ecf63ac821fd70579440ca14ec9c688731962d80a4`;
`apps/hyperai-interface/app.css`
`f9305820df291fdafccc9bda6819733bb4a9f91b5d1e0e4e04f9cc0fd5a497b0`.

This is a focused local browser check, not full accessibility acceptance.
Screen-reader behavior, complete keyboard-only workflows, reduced motion,
broader responsive coverage, bounded adversarial review, and clean
second-environment reproduction remain open. Assurance remains
`NOT_ASSIGNED`; reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. The claim ceiling remains read-only query behavior over
HyperAI-authored synthetic data.

### Local follow-up: model/view action feedback and focus (2026-09-25)

State slice: `hyperai.bazaar.data_local.v1`, baseline revision
`dcf41e31aa1f98438a6f8ec6b253801e26dc0aa`. The browser model and view
preview/save actions now publish persistent `role="status"`,
`aria-live="polite"`, `aria-atomic="true"` feedback and restore focus after
the full page render. A keyboard user lands on the status message, then the
next Tab reaches the exact confirmation button after a successful preview.
Failed actions preserve the entered model/view fields so the user can correct
and retry.

On macOS 26.6.1 arm64, a HyperAI-authored synthetic dataset in isolated home
`/tmp/hyperai-data-a11y.tYhqYa` exercised the full browser flow on loopback
port 4318: query returned 3 rows; model preview and explicit save succeeded
with definition digest
`d1074648ee445f8f3148b95aa8f72b5ba36fce21e6af1937953ba9d884b11839`; view
preview and explicit save succeeded with definition digest
`3b587e548ae853d67b0253c0f47a917ded422bb5e6e1f10d5f6e81782e9bd998`.
Keyboard activation confirmed that Tab from each preview status focused its
confirmation button.

To exercise failure recovery, the isolated service was stopped before model
preview and view save. Both rendered focused error status; the model name,
view name, and columns remained in their fields. This was an intentional
local connectivity failure, not a service-contract failure. Status semantics
and active focus were confirmed from the browser DOM/accessibility tree; no
screen reader was run. At viewport widths of 390 and 320 px, document/body
scroll widths equaled the viewport (390 and 320 px respectively); the 320 px
status region had equal 246 px content and scroll widths. Browser console
errors were empty. The browser test used only the disposable synthetic home;
the service was stopped, the test tab closed, and the temporary home removed.
The pre-existing service on port 4317 remained untouched.

Browser asset SHA-256: `apps/hyperai-interface/app.js`
`da4add25cf8a8849392b6080ab69402b071c17fc168f5d446a7ae2d9236a680a`;
`apps/hyperai-interface/app.css`
`52a34558e3e1aa6eac20fd52a4c61ddd1163e8c9a30efd642cc2189bef6ffcf1`.
`node --check apps/hyperai-interface/app.js` passed after these changes.

This closes only the model/view feedback and focus behavior for these local
browser paths. Complete keyboard-only and screen-reader acceptance, reduced
motion, broader responsive coverage, bounded adversarial review, clean
second-environment reproduction, and complete DataFusion allocation
accounting remain open. Assurance is `NOT_ASSIGNED`; reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The claim ceiling remains
local synthetic catalog/query/definition behavior only.

### Local follow-up: focused validation and current root gate (2026-09-25)

State slice: `hyperai.bazaar.data_local.v1`, baseline revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`. The pinned command
`sh scripts/with-pinned-rust.sh cargo test --locked -p hyperai-data` passed
all 20 tests. The pinned command
`sh scripts/with-pinned-rust.sh cargo test --locked -p hyperai-local-service --test data_api`
passed all 4 tests. `corepack pnpm run lint:fast` passed formatting and strict
workspace Clippy.

The latest `corepack pnpm run lint` attempt passed `lint:fast` and then failed
`verify:contracts` at
`outcomes-domain::fixtures::tests::contract_generated_assets_match_typed_sources`
(`crates/outcomes-domain/src/fixtures.rs:251`). The generated fixture bytes do
not match the current typed source in the pre-existing dirty outcome-contract
tree, which is outside this Data state slice. The root script therefore
skipped `verify:full`. No outcome schema or generated fixture was changed as
part of this slice. The earlier 2026-09-24 root-lint pass remains a historical
baseline and does not describe this later working-tree result.

Assurance remains `NOT_ASSIGNED`; reviewer status is `NOT_REQUESTED`;
reproduction is `NOT_REPRODUCED`. The tests used repository-local synthetic
fixtures; no external source, provider, or user data was contacted. Root
contract verification remains incomplete and is not a Data-only acceptance
result.

### Local follow-up: mobile navigation and Data keyboard path (2026-09-25)

State slice: `hyperai.bazaar_interface.mobile_nav_keyboard.v1`, baseline
revision `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, with implementation in
the existing dirty working tree. A browser check found that the closed mobile
drawer remained in the keyboard/accessibility order and that opening it did
not place focus in its navigation. The fix now marks the closed off-canvas
sidebar `inert` and `aria-hidden`, associates the toggle through
`aria-controls`, focuses the current route when opening, and restores focus to
the toggle on Escape. Selecting a route closes the drawer and focuses the page.

On macOS 26.6.1 arm64, the pinned Rust 1.93.1 interface service ran on
`127.0.0.1:4318` with a disposable HyperAI home seeded only with the synthetic
JSON snapshot (SHA-256
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`).
Keyboard checks verified that the closed menu is skipped, Enter opens it and
focuses the current navigation button, Tab reaches subsequent destinations,
Enter routes to Data and Fleet, and Escape restores focus to the toggle. A
keyboard-submitted Data query returned 3 rows across 3 columns and focused its
live status. The active toggle had a visible 3 px focus outline; `aria-expanded`,
`aria-controls`, `aria-hidden`, and inert state followed the disclosure state.

At widths 320, 390, 768, 1024, and 1440 px, document and body widths matched
the viewport. At mobile widths, the toggle was visible and a closed drawer was
hidden/inert; at desktop widths, the sidebar was available and the toggle was
hidden. Browser console errors were empty. `node --check` and
`corepack pnpm run lint:fast` passed. The temporary home and tab were removed
and closed, the 4318 service was stopped, and the pre-existing 4317 service was
left running.

This qualifies the tested local browser keyboard paths only. Screen-reader
operation, an emulated reduced-motion preference, cross-browser behavior, and
full-app keyboard acceptance remain open. Assurance is `NOT_ASSIGNED`,
reviewer status is `NOT_REQUESTED`, and reproduction status is
`NOT_REPRODUCED`.

### Rust package clean-container follow-up (2026-09-27)

State slice: `hyperai.bazaar.data_local.v1`. Baseline revision:
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the current dirty working
tree. The frozen root-owned input set contains 268 files / 3,667,946 bytes;
its sorted-path canonical JSONL manifest SHA-256 is
`fb7270573ce12f9d985c7a71024e23ed3a3800cf50f40ef524127d88ecf601f3`.
`Cargo.lock` SHA-256 is
`3bdd0d107eb832c8876cf05fa00321481d2f78012b23802d7eb52ecda2a64e40`.

Environment: macOS 26.6.1 arm64, Docker Desktop emulating `linux/amd64`, image
`rust:1.93.1-bookworm`, image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repo digest
`sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
rustc `1.93.1 (01f6ddf75 2026-02-11)`, and Cargo
`1.93.1 (083ac5135 2025-12-15)`. The run used `--network none`, `--locked
--offline`, one Cargo job, a fresh target, read-only source and Cargo cache
mounts, and a 30-minute stop bound. The image has no Clippy or rustfmt
components; the test-only runner selected the already-installed exact
toolchain with `RUSTUP_TOOLCHAIN=1.93.1-x86_64-unknown-linux-gnu` and disabled
auto-install with `RUSTUP_AUTO_INSTALL=0`.

Command:

```sh
cargo test --locked --offline --jobs 1 --all-targets \\
  -p hyperai-data -p hyperai-application -p hyperai-local-service \\
  -p outcomes-cli -p hyperai-tui
```

Result: 95 passed, 0 failed, and 2 ignored across 22 test binaries. The run
took 1,078 seconds. Cargo output SHA-256 is
`9b851052070b6f440e5c2182b43fdfe485da316121e5fb28d7cec464bf876f41`;
the exact temporary runner script SHA-256 is
`6b91bb12c27267c99b0d87c8ff5ff9cb1a820664c95ad9fe627570a9e3d2f78e`.
Import verification passed before and after, and the frozen source manifest
was unchanged. Two earlier bootstrap attempts ran no tests; both returned
exit 1 with 880-byte output digests
`655850aef146f7ea1e53a28c361d9face9c46f4cd4703bcc9d590cdd5ea6eaa2` and
`9796375ecb838dd37da53292bc6c190c0d69fca460fa72ce2eba5d1e364f3728`.
The second diagnostic identified Rustup trying to refresh toolchain metadata
while networking was disabled; the corrected exact-toolchain override passed.

Assurance: `REPRODUCED_LOCAL` for this package command and exact manifest
only. Reviewer status: `NOT_REQUESTED`. This does not reproduce root lint,
JavaScript, browser, or full Phase 2 acceptance; Docker emulation is not native
Linux or a separate physical host. Synthetic fixtures, temporary snapshot,
target, log, and container were removed. Full packet reproducibility remains
`NOT_REPRODUCED`; UI accessibility, adversarial review, independent review,
and source-specific rights remain open.

### Workload Trace owner-backed status follow-up (2026-09-28)

Under `hyperai.bazaar.workload_trace_data_read_served.v1`, the existing local
catalog verifier now returns a bounded status to the Workload Trace without
re-reading its manifest. `bound` is the only status that adds a digest to the
buyer intent. The owner reference can also report `not_seeded`, `missing`,
`conflicting`, `denied`, or `unavailable` without a digest. The catalog and
manifest wire contracts, workload-trace schema, owner route, and scoped ACP
allowlist remain unchanged.

The focused Rust packages passed. The served loopback browser observed the
unseeded and digest-bound states, then actual permission denial, a missing
snapshot file, a byte-integrity conflict, and recovery to the original
verified digest. At 390 by 844 and 1280 by 900, document and body widths
matched the viewport; browser error logs were empty. The disposable service
home was removed and port 4317 was preserved. Root lint, pinned import
verification, the offline `hyperai` binary build, and whitespace checks are
recorded in the linked state-slice evidence.

This is `INTERNAL_PASS` for those exact local synthetic Data and trace paths.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
No external source, Bazaar code/data, remote worker, or non-synthetic payload
was used. It establishes local manifest/content integrity only, not market
truth, freshness, execution, correctness, payment entitlement, independent
review, full accessibility, cross-browser behavior, or production readiness.
See the [owner-backed Data read packet](compute-bazaar-workload-trace-data-read-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-workload_trace_data_read_servedv1).

### Local follow-up: keyboard-accessible Data tables (2026-09-28)

State slice: `hyperai.bazaar.data_local.v1`, based on HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing uncommitted
working tree. On macOS 26.6.1 arm64, the pinned Rust 1.93.1 local service was
opened in the Codex in-app browser; its engine version was not captured. The
disposable home contained only the fixed HyperAI synthetic JSON/Parquet
snapshot and a temporary saved model. The queried JSON snapshot SHA-256 was
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`.

The browser accessibility tree exposed the derived-summary and query-result
tables as named regions. Each scroll wrapper had `tabindex="0"`; each table
had a caption and each column header had `scope="col"`. The saved-model
control's accessible name included both model and dataset identity. Invoking
it completed the same bounded three-row, three-column query. Starting at the
SQL field, Tab reached the query button and the next Tab reached the query
table region. Its keyboard focus displayed a 3 px outline; ArrowRight advanced
the horizontal scroll from 0 to 40 px (204 px maximum) at 390 px viewport
width.

At the default 1280×720 viewport, and at 390×844 and 320×844, document and
body widths matched the viewport. At 390 px, the summary table wrapper was
316 px wide with 701 px of content, and the query wrapper was 316 px wide
with 520 px of content. At 320 px, the wrappers were 246 px wide with the
same content widths. Browser error and warning logs were empty.

Served asset SHA-256 values were `index.html`
`c2f4822cce6906b4c530e335824e352c3d4623cd3f9cdd3b95713d51e45628d4`,
`app.js` `5d8e8b8f248aada083c6456037e8784250ace6c839f421f338ec144799ebe5d4`,
and `app.css`
`1478fcd36fa45374ca1443cdfa9eca36175cf81cd8601b08be3cce3e8dc8ca42`.
`corepack pnpm run lint` passed, including all 14 interface tests and the
root Rust checks. `python3 scripts/verify_imports.py` passed for the pinned
Simple Compute Market, HyperIntel, and JustAI snapshots. The temporary home
and browser tab were removed and the service was stopped.

This is scoped local browser evidence for table naming, keyboard focus, and
horizontal scrolling. No screen reader was run. Full screen-reader and
cross-browser acceptance, reduced-motion coverage for the Data workspace,
broader responsive and keyboard workflows, bounded adversarial review, and
clean second-host reproduction remain open. Assurance remains
`NOT_ASSIGNED`; reviewer status is `NOT_REQUESTED`; reproduction status is
`NOT_REPRODUCED`. The evidence does not qualify the full Phase 2 slice or
establish external source, market, execution, or payment claims.

### Local follow-up: hostile query output escaping (2026-09-28)

State slice: `hyperai.bazaar.data_local.v1`, based on HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing uncommitted
working tree. On macOS 26.6.1 arm64, the pinned Rust 1.93.1 local service was
opened in the Codex in-app browser; its engine version was not captured. The
temporary home contained only the fixed HyperAI synthetic JSON/Parquet
snapshot, whose JSON bytes have SHA-256
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`.

Two bounded one-row queries supplied HTML-like text as a column alias and a
cell value:

- `SELECT gpu_model AS "<img src=x onerror=alert(1)>" FROM dataset LIMIT 1`
- `SELECT '<svg onload=alert(1)>' AS payload FROM dataset LIMIT 1`

Both queries returned one row and one column. The alias appeared as the
literal `<img ...>` header text and the value as literal `<svg ...>` cell text;
their DOM `innerHTML` used escaped angle brackets. The result table contained
no `img`, `svg`, `script`, `iframe`, `onerror`, or `onload` element/attribute.
Browser error and warning logs were empty. The disposable home and browser tab
were removed, and the service was stopped.

This is a bounded output-encoding probe for query column names and cell values.
It is not a review of every HTML surface or a completion of the broader
adversarial-review gate. Assurance remains `NOT_ASSIGNED`; reviewer status is
`NOT_REQUESTED`; reproduction status is `NOT_REPRODUCED`. The local result
does not establish full application security, screen-reader behavior,
cross-browser compatibility, source truth, execution, or payment claims.

### Focused Data and local-service test follow-up (2026-09-28)

State slice: `hyperai.bazaar.data_local.v1`. The pinned offline command
`sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data -p hyperai-local-service`
passed on HyperAI revision `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus
the existing uncommitted working tree: 65 tests passed across 12 non-empty
test binaries, with no failures. The `hyperai-data` tests covered deterministic
manifests, changed-byte and symlink refusal, bounded JSON/CSV queries,
unregistered functions/tables, writes and multi-statement SQL, recursive SQL,
query cancellation, stale snapshots, output limits, and DataFusion memory-pool
behavior. The `hyperai-local-service` tests included authenticated Data routes,
stable rejection of unsupported query functions, cancellation without partial
rows, and digest-preview/idempotency requirements for model and view writes.

This records the package-test result only. The command also ran other tests in
the shared local-service package; those do not qualify Fleet or Trade. It did
not rerun root lint, JavaScript checks, browser acceptance, a clean-environment
reproduction, or the bounded adversarial review. Tests used their local
synthetic fixtures and temporary homes; Cargo was offline and no Bazaar source,
external data, provider, or remote service was accessed. No frozen working-tree
manifest was created for this rerun. Assurance remains `NOT_ASSIGNED`;
reviewer status is `NOT_REQUESTED`; reproduction status for this rerun is
`NOT_REPRODUCED`. The test results do not establish a process-level memory
bound, application-wide output safety, market truth, execution correctness,
payment entitlement, independent review, or production readiness. The broader
accessibility, adversarial-review, and clean second-host gates remain open.

Source-review limitation (not race-tested): the query path hashes the dataset
before and after DataFusion scans it by path, but does not hold an immutable
file snapshot across the scan. This detects persistent byte changes; it does
not establish behavior against a hostile same-user process that changes and
restores the file during execution. The current local synthetic slice assumes
the HyperAI data home is not concurrently controlled by such a process. If
that actor enters the threat model, immutable snapshot semantics need a
separately named design and state slice before making that claim.

### Focused SQL table-modifier rejection follow-up (2026-09-29)

State slice: `hyperai.bazaar.data_local.v1`. Review of the pinned
`sqlparser` 0.59.0 AST found that the Data query validator recognized the
`dataset` table name but did not reject optional table modifiers. In
particular, the parser accepted `SELECT gpu_model FROM dataset TABLESAMPLE
BERNOULLI (50)`, and the validator previously allowed it. The validator now
accepts only the unmodified registered `dataset` relation and rejects table
hints, version clauses, ordinality, partitions, JSON paths, sampling, and
index hints. The unit test confirms the parser produces a sampling modifier
and that policy rejects the query; the Data contract suite includes the same
negative case, and the authenticated Data API test confirms HTTP 422 with
`DATA_QUERY_REJECTED`.

The following pinned offline checks passed on HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing uncommitted
working tree:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --all-targets`: 23 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-local-service --test data_api`: 4 passed.
- `sh scripts/with-pinned-rust.sh cargo fmt --all -- --check`: passed.
- `sh scripts/with-pinned-rust.sh cargo clippy --locked --offline -p hyperai-data -p hyperai-local-service --all-targets --all-features -- -D warnings`: passed.
- `corepack pnpm run lint`: passed; optional PostgreSQL, Docker, and staged-binary checks remain environment-gated/ignored by that root run.

The changed-input digests are `crates/hyperai-data/src/lib.rs`
`4dc38c6e9ff434e9cc6ded05eeb5e45b56cd5f132716b0ef730e98d0be1d1ca9`,
`crates/hyperai-data/tests/query_contract.rs`
`2094adb38ab71af35ae7b2d5b983e7262ab489ed5e12190bcf509964b1ebabc1`, and
`crates/hyperai-local-service/tests/data_api.rs`
`fe5035bc8063e10f2358c7e0e46719f81d07eef0191f0360ec6f0ee5abf61494`.
The lockfile digest was
`b3708a2e0577a3e3d4a07c9110b33ae0ff0676b7aaaf495cf4278ba46e7e6736`.

Assurance is `INTERNAL_PASS` for this exact table-modifier rejection change;
the overall Phase 2 Data slice remains `NOT_ASSIGNED`. Reviewer status is
`NOT_REQUESTED`; reproduction status is `NOT_REPRODUCED`. Inputs were the
HyperAI-authored synthetic fixture and the pinned local dependency set; no
Bazaar source/data, external source, provider, network request, or external
write was used. The result closes this SQL modifier gap only. Full bounded
adversarial review, immutable-snapshot semantics, process-memory accounting,
accessibility, clean second-host and native-platform qualification remain
open. The claim ceiling remains local synthetic query-policy behavior; the
next source-adapter gate still requires explicit rights, field classification,
freshness, retention, and data-root decisions.

### Focused ignored-query-clause rejection follow-up (2026-09-29)

State slice: `hyperai.bazaar.data_local.v1`, HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing dirty working
tree; no frozen working-tree manifest was created. Review of the pinned
`sqlparser` 0.59.0 AST and DataFusion 52.5.0 planner found that generic-dialect
queries can carry `FOR JSON`, `FOR XML`, and `FORMAT JSON` clauses that the
planner ignores, while `FETCH FIRST` is unsupported by the planner. The
validator now rejects `Query.for_clause`, `Query.format_clause`, and
`Query.fetch` before planning. Unit tests assert that the parser populates
these AST fields and that policy returns `QueryRejected`; contract tests cover
the same four queries, and the authenticated API test confirms HTTP 422 with
`DATA_QUERY_REJECTED`.

Validation passed on macOS 26.6.1 arm64 with the pinned Rust 1.93.1 toolchain:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --all-targets`: 24 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-local-service --test data_api query_api_rejects_unregistered_functions_and_cancel_unknown_query`: passed.
- `sh scripts/with-pinned-rust.sh cargo fmt --all -- --check`: passed.
- `sh scripts/with-pinned-rust.sh cargo clippy --locked --offline -p hyperai-data -p hyperai-local-service --all-targets --all-features -- -D warnings`: passed.
- `corepack pnpm run lint`: passed, including root formatting, Clippy, contract checks, and `cargo test --workspace --all-features`.

Current changed-input SHA-256 digests are `crates/hyperai-data/src/lib.rs`
`d210994d1f9d101dcaf9cb5abdba4f0ea5305cec5b306a6f90e5de78ff99dc9d`,
`crates/hyperai-data/tests/query_contract.rs`
`d1e587b42bcb55a133b517ee57b711ac3a62d3bce62cf1b6917044080c52d8a4`, and
`crates/hyperai-local-service/tests/data_api.rs`
`bd9d990c7d452b6bb40445efc6e851f38a46faa546165f5ebdf4f11817744322`.

Assurance is `INTERNAL_PASS` for this exact ignored-query-clause rejection;
the overall Phase 2 Data slice remains `NOT_ASSIGNED`. Reviewer status is
`NOT_REQUESTED`; reproducibility status is `NOT_REPRODUCED`. Tests used only
HyperAI-authored synthetic inputs and local disposable test homes. Cargo ran
offline; no Bazaar bytes, external source, provider, network request, or
external write was used. This does not close the broader bounded
adversarial-review, immutable-snapshot, process-memory, accessibility,
second-host, or native-platform gates. The claim ceiling remains local
synthetic query-policy behavior. The next source-adapter gate still requires
explicit rights, field classification, freshness, retention, and data-root
decisions.

### Focused SELECT-clause policy follow-up (2026-09-29)

State slice: `hyperai.bazaar.data_local.v1`, HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the existing dirty working
tree; no frozen working-tree manifest was created. Review of the pinned
`sqlparser` 0.59.0 AST against DataFusion 52.5.0 found a filter-loss gap:
`GenericDialect` parses `PREWHERE` into `Select.prewhere`, but DataFusion's
`select_to_plan` does not consume that field. The validator now rejects it
before planning. The same pre-planning policy rejects `TOP`, `LATERAL VIEW`,
`CLUSTER BY`, and `SORT BY`, which the pinned planner reports as unsupported.
The unit regression asserts the AST fields are populated and returns
`QueryRejected`; query-contract and authenticated API tests cover the same
queries, with HTTP 422 and `DATA_QUERY_REJECTED` at the route.

Validation passed on macOS 26.6.1 arm64 with the pinned Rust 1.93.1 toolchain:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --all-targets`: 25 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-local-service --test data_api`: 4 passed.
- `corepack pnpm run lint`: passed for this exact source snapshot, including
  `lint:fast`, `verify:contracts`, and `verify:full`.

Current code-input SHA-256 digests are `crates/hyperai-data/src/lib.rs`
`03501bf5bf1bd3c56d1dffcd05c9151d4a6337e576974529eb190601672ea1f9`,
`crates/hyperai-data/tests/query_contract.rs`
`ac0c567db0cd30bf7a7aea014651badf8e188f42a50291da007d43ad8f0733ed`, and
`crates/hyperai-local-service/tests/data_api.rs`
`38642211473b1be37fc087980d5ab6ae380931538b98497c1620c99e0301f969`.

Assurance is `INTERNAL_PASS` for these exact query-policy regressions; the
overall Phase 2 Data slice remains `NOT_ASSIGNED`. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The parser/planner source
review and tests used pinned local dependencies and HyperAI-authored synthetic
inputs only; no Bazaar bytes, external source, provider, network request, or
external write was used. The claim ceiling is local synthetic query-policy
behavior. Broader bounded adversarial review, immutable-snapshot semantics,
process-memory accounting, accessibility, clean second-host, and native
platform qualification remain open. The next source-adapter gate still
requires explicit rights, field classification, freshness, retention, and
data-root decisions.

### Verified immutable query-input continuation (2026-09-29)

State slice: `hyperai.bazaar.data_immutable_query_snapshot.v1`, scoped under
the accepted `hyperai.bazaar.data_local.v1` local synthetic contract. The
the prior query path hashed the file before and after DataFusion read its path,
but the planner reopened that path; those checks did not bind the scanned bytes
to one immutable snapshot. The query now reads at most the existing 16 MiB
input ceiling once into a bounded buffer, verifies its length and digest
against the manifest, and passes that buffer through a private per-query
in-memory object store. DataFusion receives a fixed `memory://` table path,
not the mutable local dataset path. The object store and captured bytes are
dropped when the query returns or is cancelled.

The implementation uses only the already-locked `object_store` crate at the
exact current version with default features disabled. It enables no cloud or
HTTP backend and writes no temporary file. The public request and result
schemas, JSON/CSV bytes, query limits, and error envelope remain unchanged.
The additional input buffer remains outside the DataFusion pool;
this does not establish a process-memory bound. The authenticated HTTP
cancellation check uses a `cfg(test)`-only service gate after active-query
registration, so it does not depend on fixture size or query timing; the hook
must not exist in non-test builds or add a production route or delay.

Proof gates: JSON and Parquet query outputs remain byte-identical; a test
captures bytes, replaces the original file, then proves execution still reads
the captured snapshot; changed and oversized bytes fail before planning; the
authenticated Data API keeps its current success/error contract; a
deterministically registered active query is cancelled through the HTTP route
and returns no partial rows; output limits remain effective; and the locked
feature tree contains no cloud or HTTP object-store backend. Inputs are
HyperAI-authored synthetic fixtures only. No Bazaar source/data, external
source, provider, network, or external write is in scope. Claim ceiling: the
query result refers to the exact synthetic bytes captured and scanned by this
local build. This does not prove source authenticity, freshness, or truth.

The named state-slice record defines the exact files, allowed effects,
non-goals, proof gates, assurance fields, and publication boundary. No code
change is authorized outside that list.

Implementation and local validation (2026-09-29): the Data query captures the
verified bytes once, checks length and digest, and gives DataFusion a private
per-query in-memory object store for those bytes. JSON and Parquet
source-replacement tests return the same results as the captured snapshot;
changed and oversized captures fail before planning. The authenticated service
test pauses only under `cfg(test)` after the query enters the active registry,
confirms the real cancel route returns `202`, then releases execution and
checks the stable `DATA_QUERY_CANCELLED` response contains no partial rows.
The former oversized-fixture polling test was removed because it depended on
query duration.

Validation passed on macOS 26.6.1 arm64 with pinned Rust `1.93.1`:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --all-targets`: 27 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-local-service --lib http_cancel_route_cancels_a_registered_query_without_partial_rows -- --nocapture`: 1 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-local-service --test data_api`: 3 passed.
- `sh scripts/with-pinned-rust.sh cargo clippy --locked --offline -p hyperai-data -p hyperai-local-service --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check` and the second `corepack pnpm run lint`: passed.
- The locked `object_store v0.12.5` feature tree shows `fs` and `walkdir`; no cloud or HTTP object-store feature is enabled.

The first root-lint attempt hit an existing ACP synthetic-peer connection-close
failure while its local route probe completed. The exact test passed in
isolation, then the full root lint passed on rerun; no ACP source changed.
Assurance is `INTERNAL_PASS` for this local synthetic contract only. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Inputs and
test artifacts stayed in disposable local homes and consisted only of
HyperAI-authored synthetic JSON and Parquet. This does not establish a
process-memory bound, source authenticity, freshness, or market truth. Overall
Phase 2 Data acceptance, accessibility, separate-host reproduction, and
native-platform qualification remain open; external sources still require
their separate rights and data-governance decisions.

### Near-limit nested SQL validation follow-up (2026-09-29)

State slice: `hyperai.bazaar.data_sql_nesting.v1`, on HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the pre-existing dirty working
tree. The deterministic test constructs a balanced 1,900-level parenthesis
nest around `available_count` in a single `SELECT` over the registered
`dataset`. The resulting SQL is 3,835 bytes, under `MAX_SQL_BYTES` (4,096).
It calls the private validator inside `catch_unwind` and accepts only normal
validation success or the typed `QueryRejected` result. No production parser,
policy, or limit behavior changed.

Validation passed on macOS 26.6.1 arm64 with pinned Rust `1.93.1`:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data near_limit_nested_parentheses_do_not_panic -- --nocapture`: 1 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --all-targets`: 28 passed.
- `corepack pnpm run lint`: passed `lint:fast`, workspace contract tests,
  and `verify:full`; opt-in PostgreSQL, Docker, and staged-binary checks were
  ignored.

The source digest is `crates/hyperai-data/src/lib.rs`
`b8c071c4d9c5a2cf19b8b53e4cf397e92682d5ad43de8775ea3d56777ddda158`. Assurance
is `INTERNAL_PASS` for this exact validator case; reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The request was generated
in memory from a fixed synthetic SQL string, and build outputs stayed in the
local workspace. No Bazaar bytes, external data, provider, network request, or
external write was used. General parser robustness, arbitrary-depth behavior,
process-memory bounds, SQL security, independent review, and complete Phase 2
acceptance remain open. The claim ceiling is the direct validator behavior for
this exact query on this host and toolchain. The next source-adapter gate
remains explicit rights, field classification, freshness, retention, and
data-root decisions before any non-synthetic source.

### Near-limit left-associative SQL expression follow-up (2026-09-29)

State slice: `hyperai.bazaar.data_sql_expression_chain.v1`, on HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the pre-existing dirty working
tree. The deterministic test creates a single `SELECT` over the registered
`dataset` with 582 `1=1` terms joined by `OR`. Its SQL is 4,090 bytes, below
the 4,096-byte limit. The validator returns `Ok(())` without panic; the test
makes no production parser, policy, or limit change.

Validation passed on macOS 26.6.1 arm64 with pinned Rust `1.93.1`:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data near_limit_left_associative_expression_chain_does_not_panic -- --nocapture`: 1 passed.
- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data --all-targets`: 29 passed.
- `corepack pnpm run lint`: passed `lint:fast`, workspace contract tests,
  and `verify:full`; opt-in PostgreSQL, Docker, and staged-binary checks were
  ignored.

The source digest is `crates/hyperai-data/src/lib.rs`
`a3a20e31925c1baf01020fe4f9ccaf17da1bfd1ae8eb093a93d8a1ee1b92bcda`.
Assurance is `INTERNAL_PASS` for this exact validator case; reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The SQL string was
generated in memory and build outputs stayed in the local workspace. No
Bazaar bytes, external data, provider, network request, or external write was
used. This does not establish general SQL security, arbitrary-depth parser
safety, bounded process memory, independent review, or complete Phase 2
acceptance. The claim ceiling is direct validation of this exact expression on
this host and toolchain. The next source-adapter gate remains explicit rights,
field classification, freshness, retention, and data-root decisions before
any non-synthetic source.

### Full public query-path memory baseline (2026-09-29)

State slice: `hyperai.bazaar.data_immutable_query_snapshot.v1`, scoped under
`hyperai.bazaar.data_local.v1`. The configured-pool stress case now runs
through public `query()` rather than constructing a DataFusion context and
calling its executor directly. The test writes a deterministic synthetic
fixture of 12,039,000 bytes with SHA-256
`675823dba2fc6f3c2c6d2fa5f1ebc46e61036707e66c4654e98c1d4ee9d1d555`, registers
its manifest, and then exercises manifest loading, byte verification,
immutable capture, the per-query memory object store, the configured 64 MiB
DataFusion pool, the 2-second query timeout, and the bounded result path. It
asserts the returned manifest digest, 1,000 expected rows, and reported pool
size. No production code, limit, or wire contract changed.

The focused test passed on macOS 26.6.1 arm64 with pinned Rust
`1.93.1 (01f6ddf75 2026-02-11)`. `/usr/bin/time -l` around the exact compiled
test executable measured maximum RSS of 119,193,600, 120,602,624, and
118,931,456 bytes across three fresh processes (median 119,193,600 bytes /
113.671875 MiB). Three no-match runs of the same test executable measured
9,486,336 bytes each (9.046875 MiB). The test executable
SHA-256 is `5ed25c56a7f71c14930f3100835fe1fc30651dea5878c33e22f2b7787ed47284`;
the source SHA-256 is
`b4079f71b2647a47c8327bfbf2886eee577d677792ed6f529fadaa320e6dc38c`.

This measures the full public query path for this exact fixture and process;
it is not a process-memory limit or production capacity claim. The 64 MiB
DataFusion pool excludes other process allocations, including the captured
input, test harness, and allocator. Assurance is `INTERNAL_PASS` for this
synthetic path and measurement only; reviewer status is `NOT_REQUESTED` and
reproduction is `NOT_REPRODUCED`. Data and outputs remained in disposable
local test homes. This does not close overall Phase 2 acceptance, broad
adversarial review, accessibility, separate-host reproduction, or native
platform qualification. External sources still require their separate
rights, classification, freshness, retention, and authorization decisions.

### Focused immutable-query package reproduction (2026-09-29)

State slice: `hyperai.bazaar.data_immutable_query_snapshot.v1`. The current
source passed the focused Data and local-service tests in a disposable,
network-disabled Linux/amd64 container on the same owner's Apple Silicon
host. The read-only Rust source manifest contained 107 regular files /
2,261,414 bytes, SHA-256
`67c66bb71c96e3630402c2f02a110e6640e58527550f6f7da7e63d7c7f9b397b`; the
pre/post Git status contained 2,597 entries, SHA-256
`f7cb922953b89b8b6f9f4a4e9ba0ecc74954fc6a6bbad84b6109618acaec40c7`. Both
matched before and after execution. The local-service targets also compile
the pinned HyperIntel source snapshot, verified at revision
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`, 99 files, tree SHA-256
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`.

Environment: local `rust:1.93.1-bookworm` image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`,
repository digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`;
Rust `1.93.1 (01f6ddf75 2026-02-11)`, Cargo `1.93.1 (083ac5135 2025-12-15)`,
four Cargo jobs, 10 GiB memory cap, read-only source/cache mounts, and
`--network none`. Commands and results:

- `cargo test --locked --offline -p hyperai-data --all-targets`: 29 passed.
- `cargo test --locked --offline -p hyperai-local-service --lib http_cancel_route_cancels_a_registered_query_without_partial_rows -- --nocapture`: 1 passed.
- `cargo test --locked --offline -p hyperai-local-service --test data_api`: 3 passed.

Across six test-result groups, 33 passed, 0 failed, and 0 were ignored. The
combined test-log SHA-256 is
`646f66630ae43aa5532c91acdf2d224c061ef6003dbdf88c63f2cef046d3ae34`. The
container was removed; service PID 85094 remained on `127.0.0.1:4317`. The
repository-wide import verifier passed the SCM and HyperIntel trees but exited
1 for the existing JustAI import mismatch; JustAI is not an input to this
slice.

Assurance is `REPRODUCED_LOCAL` for these exact focused Rust package checks
and source identities only. Reviewer status is `NOT_REQUESTED`. Inputs were
HyperAI-authored synthetic fixtures and the pinned local HyperIntel snapshot;
temporary test data and build output were destroyed with the container. This
does not establish native Linux behavior, separate-host reproduction,
independent review, full Phase 2 acceptance, or production readiness. Claim
ceiling: local query snapshot integrity, cancellation, and authenticated Data
API behavior for this source manifest. The remaining Phase 2 gates include
broader adversarial parser review, accessibility, native-platform checks, and
separate-host reproduction. Non-synthetic sources retain their independent
rights, classification, freshness, retention, data-root, and authorization
gates.

### Focused generic SQL AST-field rejection follow-up (2026-09-29)

State slice: `hyperai.bazaar.data_sql_ast_coverage.v1`, on baseline revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the current dirty and
untracked working tree. The pinned `sqlparser 0.59.0` `GenericDialect` parses
several optional forms into AST fields the local planner path does not handle.
The validator now rejects `DISTRIBUTE BY` (`Select.distribute_by`), named
`WINDOW` (`Select.named_window`), `QUALIFY` (`Select.qualify`), `CONNECT BY`
(`Select.connect_by`), and `FROM ... SELECT` (`Select.flavor == FromFirst`).
For every form, the test first proves the expected AST field/flavor is
populated, then proves validation returns `QueryRejected` before DataFusion
planning. No accepted syntax, API, source, query budget, or resource limit
changed. `crates/hyperai-data/src/lib.rs` SHA-256 for this run:
`9701cfb198f6333595f2d4376386490cb8f9a097e8883f7f913e5bc48d888faf`.

Validation on pinned Rust `1.93.1`:

- `sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-data`:
  30 passed, 0 failed, 0 ignored across four test binaries.
- `corepack pnpm run lint`: passed, including root lint-fast, workspace
  contract tests, and all-features tests.
- Scoped Markdown whitespace, final-newline, and link-target checks passed.

Assurance: `INTERNAL_PASS` for these five parsed forms only. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Custody is local
HyperAI source and fixed in-memory synthetic SQL. This does not establish
complete SQL security, rejection of every parser AST field, independent
review, or full Phase 2 acceptance. Overall Phase 2 remains `NOT_ASSIGNED`;
broader parser review, accessibility, separate-host reproduction, native
platform checks, and separate source-rights decisions remain open.

### Per-key LIMIT syntax coverage (2026-09-29)

State slice: `hyperai.bazaar.data_sql_limit_by.v1`. The pinned
`sqlparser 0.59.0` parser accepts `LIMIT 1 BY gpu_model` in `GenericDialect`
and stores `gpu_model` in `Query.limit_clause` as a nonempty `limit_by` list.
The local Data query contract supports a bounded total-row limit; per-key
limits have no contract or client path. The validator now rejects a nonempty
`limit_by` list before planning. The regression asserts the parser populated
the field, asserts `QueryRejected`, and confirms a normal `LIMIT 1` query
remains accepted. No DataFusion execution, API, dependency, or resource-budget
change was made. Overall Phase 2 remains `NOT_ASSIGNED`.

Validation on 2026-09-29: focused `hyperai-data` tests passed 31/31, and
`corepack pnpm run lint` passed `lint:fast`, `verify:contracts`, and
`verify:full`, including workspace formatting, Clippy, contract tests, and
all-feature workspace tests. Exact source identity and scope are recorded in
[`hyperai.bazaar.data_sql_limit_by.v1`](state-slices.md#hyperai-bazaar-data_sql_limit_byv1).
Assurance is `INTERNAL_PASS`; reviewer is `NOT_REQUESTED`; this exact code hash
is `NOT_REPRODUCED`. The earlier v3 `REPRODUCED_LOCAL` workspace run predates
this change. This case does not establish complete SQL security, exhaustive
parser coverage, DataFusion resource isolation, or overall Phase 2 acceptance.

### Wildcard projection modifier coverage (2026-09-29)

State slice: `hyperai.bazaar.data_sql_wildcard_options.v1`. The pinned
`sqlparser 0.59.0` `GenericDialect` parser can populate wildcard option fields
for `ILIKE`, `EXCLUDE`, `EXCEPT`, `REPLACE`, and `RENAME`. The Data validator
now rejects them before planning. The regression asserts each exact AST option,
checks both unqualified and qualified wildcard variants, and requires
`QueryRejected`; plain `SELECT * FROM dataset` remains accepted. It adds no
dataset read, DataFusion execution, API, dependency, or resource-budget
change. Overall Phase 2 remains `NOT_ASSIGNED`.

Validation on 2026-09-29: focused `hyperai-data` tests passed 32/32, and
`corepack pnpm run lint` passed `lint:fast`, `verify:contracts`, and
`verify:full`. Seven environment-dependent workspace tests remained ignored.
Exact code hash, custody, assurance, and claim ceiling are recorded in the
[`hyperai.bazaar.data_sql_wildcard_options.v1`](state-slices.md#hyperai-bazaar-data_sql_wildcard_optionsv1)
state slice. This is `INTERNAL_PASS` for these five parsed option fields only;
reviewer is `NOT_REQUESTED`, and this exact hash is `NOT_REPRODUCED`. Broader
parser review and overall Phase 2 acceptance remain open.

### Advanced GROUP BY syntax coverage (2026-09-29)

State slice: `hyperai.bazaar.data_sql_group_modifiers.v1`. The pinned
`sqlparser 0.59.0` `GenericDialect` parses `GROUP BY ALL`, direct `ROLLUP`,
`CUBE`, and `GROUPING SETS` expressions, plus `WITH ROLLUP`, `WITH CUBE`,
`WITH TOTALS`, and post-expression `GROUPING SETS` modifiers. The local query
contract exposes ordinary grouping by explicit expressions. The validator
rejects each covered AST form before planning, while ordinary
`GROUP BY gpu_model` remains accepted. It adds no dataset read, DataFusion
execution, API, dependency, or resource-budget change. Overall Phase 2 remains
`NOT_ASSIGNED`.

Validation on 2026-09-29: focused `hyperai-data` tests passed 33/33, and
`corepack pnpm run lint` exited 0 across `lint:fast`, `verify:contracts`, and
`verify:full`; seven environment-dependent tests remained ignored. The exact
source hash, custody, assurance, reviewer status, reproduction status, and
claim ceiling are recorded in
[`hyperai.bazaar.data_sql_group_modifiers.v1`](state-slices.md#hyperai-bazaar-data_sql_group_modifiersv1).
This is `INTERNAL_PASS` for the covered AST variants only; the exact code hash
is `NOT_REPRODUCED`. Broader pinned-parser review, accessibility,
separate-host/native-platform evidence, non-synthetic source rights, and
overall Phase 2 acceptance remain open.

### ORDER BY modifier coverage (2026-09-29)

State slice: `hyperai.bazaar.data_sql_order_by_modifiers.v1`. The pinned
`sqlparser 0.59.0` `GenericDialect` can populate `OrderByExpr.with_fill` for
`ORDER BY ... WITH FILL` and `OrderBy.interpolate` for `ORDER BY ...
INTERPOLATE`. The validator rejects both modifier fields before planning with
`QueryRejected`. The pinned DataFusion 52.5.0 planner rejects these forms
later in planning as well; ordinary expression-based `ORDER BY` remains
accepted. No dataset read, DataFusion execution, API, dependency, or
resource-budget change is included. Overall Phase 2 remains `NOT_ASSIGNED`.

Validation on 2026-09-29: focused `hyperai-data` tests passed 34/34, and
`corepack pnpm run lint` exited 0 across `lint:fast`, `verify:contracts`, and
`verify:full`, including all-feature workspace and doc tests. The exact source
hash, custody, assurance, reviewer status, reproduction status, and claim
ceiling are recorded in
[`hyperai.bazaar.data_sql_order_by_modifiers.v1`](state-slices.md#hyperai-bazaar-data_sql_order_by_modifiersv1).
This is `INTERNAL_PASS` for the exact `WITH FILL` and `INTERPOLATE` AST forms
only; however, that original label did not record the threat model and bounded
red-team review required by the global assurance overlay. Treat its exact
source hash and tests as historical implementation evidence; the current
assurance assessment is the continuation below. Broader parser coverage,
accessibility, separate-host/native-platform evidence, non-synthetic source
rights, and overall Phase 2 acceptance remain open.

### ORDER BY modifier red-team continuation (2026-09-29)

State slice: `hyperai.bazaar.data_sql_order_by_redteam.v1`. The original
validator already checked every expression in an `ORDER BY` list. The new
regression attempts to bypass that guard by attaching `WITH FILL` only to the
second sort expression; SQLParser populates only the second AST field, the
validator returns `QueryRejected`, and ordinary two-column sorting remains
accepted. The prior first-expression `WITH FILL` and `INTERPOLATE` cases remain
covered.

The bounded threat model treats caller-supplied SQL as untrusted local input.
It records parser panic, unsupported AST acceptance, and a later-expression
modifier bypass as risks. Review confirmed the existing 4 KiB input limit,
single-statement check, nested-query rejection, and all-expression modifier
check. The arbitrary-SQL property test now uses 128 generated byte vectors
with fixed seed `0x4859504552414931`.

Validation on 2026-09-29: `hyperai-data` passed 35/35 tests, pinned Rust
formatting passed, and `corepack pnpm run lint` exited 0 across all root gates,
including strict Clippy, all-feature workspace tests, and doc tests. The exact
source hash, red-team scope, claim ceiling, and assurance record are in the
[`hyperai.bazaar.data_sql_order_by_redteam.v1` state slice](state-slices.md#hyperai-bazaar-data_sql_order_by_redteamv1).
That slice is `INTERNAL_PASS` for the bounded local parser scope; its exact
source hash is `NOT_REPRODUCED`, reviewer status is `NOT_REQUESTED`, and
overall Phase 2 remains `NOT_ASSIGNED`. No dataset read or DataFusion query
execution was needed for these modifier regressions.

### Typed table-alias column parser coverage (2026-09-29)

State slice: `hyperai.bazaar.data_table_alias_column_type.v1`, on baseline
HyperAI revision `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus its existing
dirty and untracked worktree. Source rights, datasets, and the local service
are outside this slice.

The pinned `sqlparser 0.59.0` `GenericDialect` accepts
`SELECT d.gpu_model FROM dataset AS d (gpu_model INT)` and stores the type in
`TableAliasColumnDef.data_type`. The validator previously ignored alias
columns. DataFusion 52.5.0 applies a table alias by using its column names;
its alias application does not consume the declared column types. The
validator now rejects aliases containing any typed column definition before
planning. Regressions prove that the parser populated the exact field, that
the validator returns `QueryRejected`, and that a plain alias and a name-only
column alias remain accepted by validation. No dataset read or DataFusion
execution is part of these regressions.

Threat model: a local authenticated user or scoped local client can submit SQL
to the read-only query boundary. The bounded case looks for accepted syntax
whose declared type is present in the parser AST but ignored by the planner.
Existing single-statement, input-size, nesting, and table-factor restrictions
remain unchanged. The existing fixed-seed 128-case arbitrary-SQL byte
property test also runs in the focused package suite.

Validation on pinned Rust 1.93.1: formatting passed, and the locked offline
`hyperai-data` suite passed 36 tests (0 failed, 0 ignored): 22 unit, 5
catalog, 2 derived-data, and 7 query-contract tests. The typed-alias
regression confirms the parsed AST type, requires `QueryRejected`, and keeps
plain and name-only aliases accepted. The fixed-seed 128-case arbitrary-SQL
property test also passed. `corepack pnpm run lint` exited 0 across
`lint:fast`, `verify:contracts`, and `verify:full`, including strict Clippy,
all-feature workspace tests, and doc tests; environment-gated tests remained
ignored. Scoped Markdown hygiene and `git diff --check` passed. The exact
`crates/hyperai-data/src/lib.rs` SHA-256 is
`2ef617375f2956b71669cccf385d5fdcdd0ca4ab3e30c5b2971dafc0db426cb6`.

Assurance: `INTERNAL_PASS` for this exact source hash, bounded arbitrary-input
property test, source review, threat model, regression cases, and root gates.
Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Custody
is the local HyperAI source tree and fixed in-memory synthetic SQL; no dataset
or case-specific test artifact is retained; Cargo build outputs remain under
the existing ignored `target/` directory. The claim ceiling is rejection of the
exact typed table-alias field while preserving plain and name-only aliases.
This does not establish complete SQL security, exhaustive parser coverage,
DataFusion resource isolation, independent review, or full Phase 2 acceptance.
Next gate: broader pinned-parser review and overall Phase 2 qualification. No
commit, release, external write, or production claim is included.

### Parametric function-argument parser coverage (2026-09-29)

State slice: `hyperai.bazaar.data_sql_function_parameters.v1`, on baseline
HyperAI revision `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus its existing
dirty and untracked worktree. Source rights, datasets, and the local service
are outside this slice.

The pinned `sqlparser 0.59.0` `GenericDialect` parses
`SELECT ABS(999)(gpu_model) FROM dataset` into a nonempty
`Function.parameters` list and a separate ordinary argument list. The
DataFusion 52.5.0 function normalizer destructures the function without
consuming `parameters`, then plans the ordinary argument list. The query
validator now rejects nonempty parameter lists before planning. The regression
asserts both parser fields, requires `QueryRejected`, and keeps
`SELECT ABS(gpu_model) FROM dataset` accepted. No dataset read or DataFusion
query execution is involved.

Threat model: a local authenticated user or scoped local client can submit SQL
to the read-only query boundary. The bounded case checks for parameter syntax
that is present in the AST but dropped by the planner. Existing function
allowlisting, query-size, single-statement, and nesting controls remain
unchanged. The existing fixed-seed 128-case arbitrary-SQL byte property test
also runs in the focused package suite.

Validation on pinned Rust 1.93.1: the locked offline `hyperai-data` package
suite passed 37 tests (0 failed, 0 ignored): 23 unit, 5 catalog, 2
derived-data, and 7 query-contract tests. The property test, exact AST
assertions, rejection, and ordinary-call compatibility passed.
`corepack pnpm run lint` exited 0 across `lint:fast`, `verify:contracts`, and
`verify:full`, including formatting, strict Clippy, all-feature workspace
tests, and doc tests; environment-gated tests remained ignored. Scoped Markdown
hygiene and `git diff --check` passed. The exact
`crates/hyperai-data/src/lib.rs` SHA-256 is
`d7f6324b0b74df4c9a40fcd49fa51140626ca0421e36d0a2d73ea9dc36b1cbfd`.

Assurance: `INTERNAL_PASS` for this exact source hash, bounded arbitrary-input
property test, source review, threat model, regression cases, and root gates.
Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Custody
is the local HyperAI source tree and fixed in-memory synthetic SQL; no dataset
or case-specific artifact was retained. Cargo outputs remain under the
existing ignored `target/` directory. Claim ceiling: reject the exact
nonempty `Function.parameters` field on allowlisted calls while preserving
ordinary `ABS`; this does not establish complete SQL security, exhaustive AST
coverage, or planner equivalence. Overall Phase 2 remains `NOT_ASSIGNED`; next
gate is broader pinned-parser review and Phase 2 qualification. No commit,
release, external write, or production claim is included.

### Authenticated API parity for parser rejections (2026-09-29)

State slice: `hyperai.bazaar.data_sql_parser_api_parity.v1`, on baseline
HyperAI revision `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus its existing
dirty and untracked worktree. The test uses only fixed synthetic SQL in the
existing disposable-home, authenticated loopback harness.

The API regression sends both `SELECT d.gpu_model FROM dataset AS d
(gpu_model INT)` and `SELECT ABS(999)(gpu_model) FROM dataset` through
`POST /api/v1/data/query`. Each returns HTTP 422 and the stable code
`DATA_QUERY_REJECTED`, using the shared API assertion. These are validator
rejections before DataFusion execution or dataset scanning; no production API,
authentication, error schema, or client behavior changed.

Threat model: an authenticated local client can submit query text. The cases
check that the service does not remap or partially accept these parser-policy
rejections at its public API boundary. The service test owns an ephemeral
loopback listener and disposable home; the home is removed after the test.

Validation on pinned Rust 1.93.1: formatting passed. The locked offline
`data_api` integration test
`query_api_rejects_unregistered_functions_and_cancel_unknown_query` passed
(1 passed, 0 failed). `corepack pnpm run lint` exited 0 across `lint:fast`,
`verify:contracts`, and `verify:full`, including strict Clippy, all-feature
workspace tests, and doc tests; environment-gated tests remained ignored.
Scoped Markdown hygiene and `git diff --check` passed. The exact
`crates/hyperai-local-service/tests/data_api.rs` SHA-256 is
`026b5f2ad2258cbf2f1143c1368e0339c815aba3bd78571921699b0d337bc227`.

Assurance: `INTERNAL_PASS` for the exact test hash and these two API cases.
Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Custody
is fixed SQL literals and a disposable local service home; no dataset or
case-specific artifact was retained. Claim ceiling: HTTP 422 plus
`DATA_QUERY_REJECTED` for these two validator cases in this test harness. This
does not establish all-route authorization parity, complete SQL security, or
overall Phase 2 acceptance. Next gate: broader pinned-parser review and Phase 2
qualification. No commit, release, external write, or production claim is
included.

### Scalar-function modifier parser coverage (2026-09-29)

State slice: [`hyperai.bazaar.data_sql_scalar_function_modifiers.v1`](state-slices.md#hyperai-bazaar-data_sql_scalar_function_modifiersv1), on baseline HyperAI revision `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus its existing dirty and untracked worktree. Pinned `sqlparser 0.59.0` parses `FILTER`, `IGNORE NULLS`, and function-argument `ORDER BY` modifiers on scalar calls, while the scalar path in DataFusion `52.5.0` normalizes the ordinary arguments without carrying those modifiers into the planned scalar expression. The local Data validator now rejects those unsupported fields for its allowlisted scalar functions before planning.

Regression cases assert the parsed AST fields and rejection for:

- `SELECT ABS(asking_usd_micros_per_hour) FILTER (WHERE asking_usd_micros_per_hour < 0) FROM dataset`
- `SELECT ABS(asking_usd_micros_per_hour) IGNORE NULLS FROM dataset`
- `SELECT ABS(asking_usd_micros_per_hour ORDER BY asking_usd_micros_per_hour) FROM dataset`

Plain `ABS(...)` remains accepted by the validator, and aggregate `SUM(...) FILTER (...)` retains its supported behavior. The authenticated local API returns HTTP 422 with `DATA_QUERY_REJECTED` for each scalar case before query planning or dataset access.

On pinned Rust 1.93.1, the focused `hyperai-data` suite passed 38 tests and the `data_api` regression `query_api_rejects_unregistered_functions_and_cancel_unknown_query` passed (1 passed, 0 failed, 2 filtered). Pinned Rust formatting passed. `corepack pnpm run lint` exited 0 across `lint:fast`, `verify:contracts`, and `verify:full`; environment-gated checks remained ignored. Source SHA-256 values are `625f53ed1e20b4d5b3d1e625d7f2a13aa2251cb14933b4de241b791d60806396` for `crates/hyperai-data/src/lib.rs` and `4978725d7eef4a8661876d6cee2e7c127d1340c9b20540285c60691e2a341dd3` for `crates/hyperai-local-service/tests/data_api.rs`.

Assurance is `INTERNAL_PASS` for these parser and API cases only. Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The test used fixed SQL literals and a disposable local service home; it did not read a dataset or contact the pre-existing service. This does not establish complete SQL security, exhaustive parser coverage, planner equivalence, memory isolation, or overall Phase 2 acceptance. Phase 2 remains `NOT_ASSIGNED`; broader parser review and the packet's remaining qualification gates stay open. No external source, provider, dataset, publication, or production action is included.

### Scalar DISTINCT parser coverage (2026-09-29)

State slice: [`hyperai.bazaar.data_sql_scalar_distinct.v1`](state-slices.md#hyperai-bazaar-data_sql_scalar_distinctv1), on baseline HyperAI revision `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` plus its existing dirty and untracked worktree. In the pinned `sqlparser 0.59.0` AST, `ABS(DISTINCT gpu_model)` populates `FunctionArgumentList.duplicate_treatment`. DataFusion `52.5.0` extracts that field into `FunctionArgs.distinct`, while the scalar-expression path builds a scalar function from ordinary arguments without applying `distinct`. The local Data validator now rejects this modifier on its allowlisted scalar functions before planning.

The regression asserts that the parser populated `duplicate_treatment` as `Distinct`, requires `SELECT ABS(DISTINCT gpu_model) FROM dataset` to return `QueryRejected`, preserves ordinary `ABS(gpu_model)`, and preserves aggregate `COUNT(DISTINCT gpu_model)` validation. The authenticated local API returns HTTP 422 with `DATA_QUERY_REJECTED` for the scalar form before query planning or dataset access.

On pinned Rust 1.93.1, the focused `hyperai-data` suite passed 38 tests. The authenticated `data_api` regression `query_api_rejects_unregistered_functions_and_cancel_unknown_query` passed (1 passed, 0 failed, 2 filtered). `corepack pnpm run lint` exited 0 across `lint:fast`, `verify:contracts`, and `verify:full`, including formatting, strict Clippy, all-feature tests, and doc tests; environment-gated tests remained ignored. Source SHA-256 values are `fe7ba3ce69a482247a680340fe8a4731893cb596e9a72d199dbec42c524ea65c` for `crates/hyperai-data/src/lib.rs` and `6ad9dd2546eb99ed3b7dd3b6c642d886f72227226b9afa5a889fde41cfd24d40` for `crates/hyperai-local-service/tests/data_api.rs`.

Assurance is `INTERNAL_PASS` for this parser and API case only. Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The test used fixed SQL literals and a disposable local service home; it did not read a dataset or contact the pre-existing service. This does not establish complete SQL security, exhaustive parser coverage, planner equivalence, memory isolation, or overall Phase 2 acceptance. Phase 2 remains `NOT_ASSIGNED`; broader parser review and the packet's remaining qualification gates stay open. No external source, provider, dataset, publication, or production action is included.

### Maximum-byte SQL nesting boundary (2026-09-29)

State slice: `hyperai.bazaar.data_sql_max_bytes_nesting.v1`. The existing
balanced-parenthesis case covered 1,900 levels in a 3,835-byte query under the
4,096-byte SQL cap. This continuation raises that deterministic input to the
largest balanced depth that still leaves the fixed `SELECT available_count
FROM dataset` query valid and under the cap: 2,030 levels and 4,095 bytes.
The private validator must return normally with either success or typed
`QueryRejected`; a panic is a failure. The boundary regression itself reads no
dataset and executes no DataFusion query. Other pre-existing tests in the
package suite use only HyperAI-authored synthetic fixtures in disposable test
homes. No query-policy limit is changed.

The bounded threat model is stack exhaustion or panic from maximally nested,
authenticated local SQL. Inputs are fixed and HyperAI-authored. The only source
change is the existing unit regression in `crates/hyperai-data/src/lib.rs`;
this packet, its state-slice record, and the subplan progress summary are the
only documentation changes.
Validation uses pinned Rust 1.93.1, the named test, the full locked offline
`hyperai-data` package suite, pinned formatting, and `git diff --check`.
No source, provider, real-user data, network, or persistent Data home is
involved.

Validation on 2026-09-29 used macOS 26.6.1 arm64 with Rust/Cargo 1.93.1.
The named boundary test passed (1 passed, 0 failed), and the locked offline
`hyperai-data --all-targets` suite passed 38 tests: 24 unit, 5 catalog, 2
derived-data, and 7 query-contract tests. Pinned workspace formatting passed,
as did `git diff --check` and scoped Markdown whitespace/final-newline checks.
The post-change SHA-256 of `crates/hyperai-data/src/lib.rs` is
`86fdaf03a35ccbc2d51bd5d4e45dc70fe31a0653520568fcceb997a9577d59d3`.

Assurance is `INTERNAL_PASS` for the exact 4,095-byte regression, bounded
threat model, source review, and focused package checks. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The boundary fixture is
one fixed in-memory SQL string; the full package suite's synthetic data and
query results stay in automatically removed temporary test homes. Overall
Phase 2 remains `NOT_ASSIGNED`.

Claim ceiling: at most bounded non-panic behavior for this exact 4,095-byte
balanced expression on the tested host. This is not a general stack or process
memory bound, exhaustive parser review, SQL-security claim, or Phase 2
acceptance. Reviewer status is `NOT_REQUESTED`; independent and separate-host
reproduction remain unclaimed. The next gate remains broader parser review and
the outstanding Phase 2 accessibility, platform, and reproduction criteria.

### DISTINCT ON parser/planner compatibility (2026-09-29)

State slice: [`hyperai.bazaar.data_sql_distinct_on_planner_mismatch.v1`](state-slices.md#hyperai-bazaar-data_sql_distinct_on_planner_mismatchv1), on HyperAI revision `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus its existing dirty and untracked worktree. With the pinned `sqlparser 0.59.0` `GenericDialect`, both empty and nonempty `DISTINCT ON` syntax parse into `Distinct::On`. In the pinned DataFusion `52.5.0` configuration, the empty form fails during planning, while the nonempty form reaches optimizer rule `replace_distinct_aggregate` and fails because `first_value` is not registered. These failures reached HyperAI as `QueryFailed`, which did not match the API's stable rejected-query contract. Validation now rejects the complete `Distinct::On` AST variant before planning or dataset lookup; ordinary `SELECT DISTINCT` and aggregate `COUNT(DISTINCT ...)` behavior is unchanged.

The Data unit regression asserts the parser AST for empty and nonempty forms, requires validation rejection for both, and accepts standard `SELECT DISTINCT`. The query contract executes ordinary `SELECT DISTINCT` against a freshly created HyperAI synthetic snapshot and checks both rejected forms against an unseeded temporary root, which remains absent. The authenticated API regression returns HTTP 422 with `DATA_QUERY_REJECTED` for both forms in its disposable unseeded home.

On macOS 26.6.1 arm64 with pinned Rust/Cargo 1.93.1, locked offline `hyperai-data --all-targets` passed 40 tests (25 unit, 5 catalog, 2 derived-data, and 8 query-contract tests). The named authenticated `data_api` regression passed (1 passed, 0 failed, 2 filtered), pinned Rust formatting passed, and `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 across the root lint stages, strict Clippy, workspace contract tests, all-feature tests, and doc tests. Existing environment-gated tests remained ignored. `git diff --check` and scoped Markdown hygiene passed. Source SHA-256 values are `3cc24a59e14a9d1d3412adfc1676462135a59419d6b7655c625c96799fe44689` for `crates/hyperai-data/src/lib.rs`, `fb715312bae4258f931fb3831f55ea9c78856509fa259b683e1ef7147454ba51` for `crates/hyperai-data/tests/query_contract.rs`, and `0b6b96da1c81e0144029322e48c91f822271aee6a3a97393fd3448402de5fcd9` for `crates/hyperai-local-service/tests/data_api.rs`.

Assurance is `INTERNAL_PASS` for these exact parser, query, and API regressions only. Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Test custody is fixed SQL literals and HyperAI-authored synthetic data in automatically removed temporary roots; no test-specific dataset or artifact was retained. The pre-existing service on `127.0.0.1:4317` remained running and was not contacted. Claim ceiling: pre-planning rejection of the tested `DISTINCT ON` forms and preservation of the tested ordinary `SELECT DISTINCT` local synthetic query. This does not establish exhaustive SQL safety, general planner equivalence, DataFusion resource isolation, independent review, clean reproduction, or overall Phase 2 acceptance. Phase 2 remains `NOT_ASSIGNED`; broader parser qualification and the remaining accessibility, platform, and reproduction criteria remain open. No upstream source/data, provider, network call, external write, or production action is included.

### Dedicated function-like SQL AST allowlist follow-up (2026-09-29)

State slice: [`hyperai.bazaar.data_sql_special_expression_allowlist.v1`](state-slices.md#hyperai-bazaar-data_sql_special_expression_allowlistv1). A read-only review of the locally pinned `sqlparser 0.59.0` and DataFusion `52.5.0` sources found that function-like SQL forms represented by dedicated AST variants bypass the existing `Expr::Function` name check. DataFusion's selected features disable its datetime and Unicode expression packages. The validator now rejects dedicated `CONVERT`, `EXTRACT`, `POSITION`, `SUBSTRING`, `TRIM`, `OVERLAY`, `FLOOR`, and `CEIL` forms, plus `CAST ... FORMAT`, before dataset access or planning. It retains ordinary `CAST` and `TRY_CAST`; no functions or DataFusion features were added.

The unit regression confirms that each fixed query parses to the expected dedicated AST variant and returns `QueryRejected`; ordinary casts remain accepted by validation. The query-contract test sends the same nine inputs against an absent data root and confirms that every request is rejected before that root is created. The authenticated local-service test returns HTTP 422 with `DATA_QUERY_REJECTED` for all nine forms.

On macOS 26.6.1 arm64 with pinned Rust/Cargo 1.93.1, the new unit and query-contract regressions passed. The locked offline `hyperai-data --all-targets` suite passed 42 tests: 26 unit, 5 catalog, 2 derived-data, and 9 query-contract tests. The complete `data_api` target passed 3 tests. `cargo fmt --all -- --check`, root strict Clippy, contract-filtered tests, JavaScript interface checks (14), installer checks (3), and the root all-feature workspace/doc-test stage passed. A separate `CARGO_NET_OFFLINE=true corepack pnpm run verify:full` returned exit 0. `git diff --check` and scoped whitespace/final-newline checks passed.

Source SHA-256 values are `852701929cce1d58b7721514a3ad5c23889e37b4b77b5f3bd5bc3d3d74f9ce21` for `crates/hyperai-data/src/lib.rs`, `7eb172a7227fbb8f37b7f56d14ef5714aa2f8dbf1bbdb88f9854d315167d9121` for `crates/hyperai-data/tests/query_contract.rs`, and `90217d388f272e2a259215dab131db9a05f6ab936c0ad20fb2532da231116622` for `crates/hyperai-local-service/tests/data_api.rs`.

Assurance is `INTERNAL_PASS` for the enumerated parser, query-boundary, and API cases only. Reviewer status is `NOT_REQUESTED`; reproduction remains `NOT_REPRODUCED` for these changed source bytes. Fixed SQL literals and disposable empty/synthetic homes remained local. The existing service on `127.0.0.1:4317` remained PID 85094 and was not contacted. The Simple Compute Market and HyperIntel imports still match their pins. The existing JustAI import inventory mismatch (177 observed files versus 176 recorded) remains outside this slice and unchanged. This does not establish exhaustive SQL safety, planner equivalence, resource isolation, independent review, external data qualification, or Phase 2 acceptance. Broader parser review, accessibility, platform, and clean-reproduction gates remain open.

### Function argument-shape policy continuation (2026-09-30)

State slice: `hyperai.bazaar.data_sql_function_argument_shapes.v1`. The
read-only review covers the pinned `sqlparser 0.59.0` function argument AST
and DataFusion `52.5.0` argument conversion. The local query subset accepts
positional expression arguments and the ordinary `COUNT(*)` form; empty lists,
named arguments, qualified wildcards, and other wildcards are outside that
subset. Regression tests assert the exact `GenericDialect` AST argument shape
before requiring `QueryRejected`. The validator rejects unsupported argument
shapes before DataFusion planning, while retaining ordinary positional calls,
`COUNT(DISTINCT expression)`, and `COUNT(*)`. This slice also repairs the
local query session's missing function registry by selectively registering
only the validator's existing scalar and aggregate allowlist; it does not
enable DataFusion's default feature bundle or register table functions or
window UDFs.

On macOS 26.6.1 arm64 with Rust/Cargo 1.93.1, the locked offline
`cargo test -p hyperai-data --all-targets` suite passed 44 tests: 27 unit,
5 catalog, 2 derived-data, and 10 query-contract tests. The query contract
executes all eight allowlisted functions (`abs`, `coalesce`, `round`, `avg`,
`count`, `max`, `min`, and `sum`) against the HyperAI synthetic snapshot and
preserves scalar/aggregate row counts. The complete `hyperai-local-service
--test data_api` target passed all 3 tests. Scoped pinned Rust formatting,
`CARGO_NET_OFFLINE=true corepack pnpm run lint` (exit 0), and
`git diff --check` passed. Root lint includes interface and installer checks,
workspace formatting, strict all-feature Clippy, contract tests, all-feature
workspace tests, and doc tests; environment-gated PostgreSQL, Docker, and
browser tests remained ignored.

Exact changed-source SHA-256 values are `0a0a8deccb33b5340a2fe93fc73370183a6a351f0c7c38c5f38070f134e38290`
for `crates/hyperai-data/src/lib.rs`,
`71457fbd8a7246df50d339e4bcbe8d1e3bee5593fb6ed79e5d245eb0bb9ce4d9` for
`crates/hyperai-data/tests/query_contract.rs`, and
`6d8a24d5a56fe397f4355a6cbc4125078753f557d1326d288e22e9e3d50ddf84` for
`crates/hyperai-local-service/tests/data_api.rs`. Import preservation confirms
Simple Compute Market and HyperIntel; the unchanged JustAI import inventory
still reports 177 observed files versus 176 recorded. Tests used fixed SQL
literals and HyperAI synthetic data in disposable homes. The service on
`127.0.0.1:4317` was not contacted.

Assurance is `INTERNAL_PASS` for these exact parser, local registry, query, and
API cases; reviewer status is `NOT_REQUESTED`, and reproduction is
`NOT_REPRODUCED`. Phase 2 remains `NOT_ASSIGNED`. The result does not establish
exhaustive SQL safety, complete DataFusion allocation accounting, production
memory isolation, independent review, external data rights, or production
readiness. Broader parser review, accessibility, platform coverage, and clean
second-environment reproduction remain open.

The exact implementation file list, allowed effects, non-goals, proof gates,
threat model, claim ceiling, and publish boundary are recorded in the named
[state slice](state-slices.md#hyperai-bazaar-data_sql_function_argument_shapesv1).
No external data, provider, service on port 4317, or production path is in
scope. Phase 2 remains `NOT_ASSIGNED` until its broader parser, data, client,
accessibility, platform, and reproduction requirements are addressed.

## Focused aggregate argument clause continuation (2026-09-30)

State slice: `hyperai.bazaar.data_sql_aggregate_argument_clauses.v1`.

The validator now rejects unsupported aggregate argument clauses before
DataFusion planning. Coverage includes argument `LIMIT`, both forms of
`IGNORE NULLS`, `HAVING`, `SEPARATOR`, `ON OVERFLOW`, JSON null handling, and
JSON `RETURNING`. Aggregate `FILTER`, aggregate argument `ORDER BY`, and
`COUNT(DISTINCT ...)` remain covered as supported paths. Rejected queries are
shown to fail before an absent data root is created and to map through the
authenticated service to HTTP 422 / `DATA_QUERY_REJECTED`.

On the pinned Rust 1.93.1 toolchain, the validator unit regression, query
contract regression, and authenticated API regression each passed. The
complete `CARGO_NET_OFFLINE=true corepack pnpm run lint` gate exited 0,
including interface and installer checks, pinned formatting, strict all-target
Clippy, contract tests, the all-feature workspace suite, and doc tests. The
all-feature run passed the new case within 28 `hyperai-data` unit tests, 11
query-contract tests, and 3 authenticated `data_api` tests. Environment-gated
tests remained ignored.

Exact implementation SHA-256 values:

- `crates/hyperai-data/src/lib.rs`:
  `d3822aec72d5a1ba21c2e94efaed24a8d26e30ede6e79f6e1046fe656871f9f5`
- `crates/hyperai-data/tests/query_contract.rs`:
  `febfb0c2715007289a3ee2cb7ef277710810bedf51fcfef544f6750906ab5b14`
- `crates/hyperai-local-service/tests/data_api.rs`:
  `2aa0596560c4b5b720d0ecb14c02bf3a80281f675ef8c77500479d5d06e1f13b`

Import verification matched Simple Compute Market at
`a1c46d20b1b2f1aabe53e1b052921a87cf9d7360` and HyperIntel at
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`. The previously known JustAI
inventory discrepancy remains at 177 observed files versus 176 recorded.
Fixed SQL strings ran only against disposable synthetic roots. The existing
listener on `127.0.0.1:4317` remained owned by PID 85094 and was not contacted.

Assurance is `INTERNAL_PASS` for these exact local parser, query, and API
cases. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED` for this source digest. Bazaar rights, broader pinned-parser
coverage, DataFusion resource isolation, accessibility, platform support,
clean second-environment reproduction, and independent review remain open.
This continuation does not change overall Phase 2 status, which remains
`NOT_ASSIGNED`. See the [state-slice record](state-slices.md#hyperai-bazaar-data_sql_aggregate_argument_clausesv1).

## ODBC function-escape allowlist regression (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_odbc_function_allowlist.v1`](state-slices.md#hyperai-bazaar-data_sql_odbc_function_allowlistv1).

The pinned `sqlparser 0.59.0` `GenericDialect` parses `{ fn ... }` into an
ODBC-marked `Function` while retaining its function name and arguments. The
pinned DataFusion `52.5.0` normalizer drops the syntax marker, but the local
validator still checks the retained name against its existing allowlist. No
runtime or accepted-query behavior changed. Regression coverage confirms
escaped allowlisted `ABS` produces the same synthetic rows as ordinary `ABS`,
while escaped unallowlisted `RANDOM` is rejected before creating or reading a
data root and maps through the authenticated API to HTTP 422 /
`DATA_QUERY_REJECTED`.

On pinned Rust/Cargo 1.93.1, the locked offline `hyperai-data --all-targets`
suite passed 48 tests (29 unit, 5 catalog, 2 derived-data, 12 query-contract);
the `hyperai-local-service --test data_api` target passed 3 tests; pinned Rust
formatting passed; and `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited
0. The local package run also passed its bounded arbitrary-SQL-byte property
test. Assurance is `INTERNAL_PASS` only for the exact escaped allowlisted and
unallowlisted function cases. Reviewer status is `NOT_REQUESTED`, and
reproducibility is `NOT_REPRODUCED`.

Source SHA-256 values: `crates/hyperai-data/src/lib.rs`
`15455fa9a16a77db81a485db5859b9627d631fbebfa9ff41d03335a0ff9adb8e`,
`crates/hyperai-data/tests/query_contract.rs`
`ce1ab42942845780fc620adf447086b946f8330e7657d4f77e93ec98fabbd9aa`, and
`crates/hyperai-local-service/tests/data_api.rs`
`a3c23febf5095e0dd17bb171c8eb30ee21a0f8ceed734dd187466d971b0ed47c`. Import
preservation passed for Simple Compute Market and HyperIntel. The import
verifier exited 1 for the existing JustAI count mismatch (177 observed versus
176 recorded); no JustAI source was changed. Fixed SQL and HyperAI synthetic
data remained in disposable local homes. The existing service on
`127.0.0.1:4317` was not contacted. This does not establish exhaustive SQL
safety, other ODBC forms, independent review, clean reproduction, or overall
Phase 2 acceptance; Phase 2 remains `NOT_ASSIGNED`.

### Authenticated query-limit API mapping (2026-09-30)

State slice: [`hyperai.bazaar.data_api_query_limit_mapping.v1`](state-slices.md#hyperai-bazaar-data_api_query_limit_mappingv1).

The dedicated authenticated API regression sends a 4,097-byte SQL string and
a requested row count of 1,001 through the local query route. The service
returns HTTP 413 with `DATA_LIMIT_EXCEEDED` for both and includes no `rows`
field in either response. The service home is unseeded, so a response from the
unknown dataset would have been distinguishable from the limit response; both
limits are rejected before dataset lookup. No runtime limit or API behavior
was changed.

The focused locked/offline `hyperai-local-service --test data_api` case passed
1 test. Pinned Rust formatting and `CARGO_NET_OFFLINE=true corepack pnpm run
lint` passed. The final test source SHA-256 is
`137c733e411147be4afcadd97e87bfc82cbad7fe51fd7149c0211dac803d2f8a`.
Simple Compute Market and HyperIntel imports passed; the existing JustAI
inventory check remains mismatched at 177 observed files versus 176 recorded.
The service used an ephemeral loopback port and temporary home; the existing
listener on port 4317 was not contacted.

Assurance is `INTERNAL_PASS` only for these two request-limit API mappings.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. This
does not establish complete API limit coverage, DataFusion resource isolation,
SQL safety, accessibility, external data rights, independent review, or Phase
2 acceptance. Overall Phase 2 remains `NOT_ASSIGNED`.

### Repeatable populated Data browser workflow (2026-09-30)

State slice:
[`hyperai.bazaar.interface.data_workflow_browser.v1`](state-slices.md#hyperai-bazaar-interface-data_workflow_browserv1).
The browser runner now has an opt-in pnpm command that creates a fixed
HyperAI synthetic dataset in a fresh temporary home and exercises the actual
Data workspace. It runs a successful query, verifies the expected unknown-
table HTTP 422 clears prior rows while retaining SQL and focus, retries the
valid query, then previews and explicitly saves a local model and table view.
It asserts the exact eight Data POST requests and digest-bearing saved
inventories. This makes prior manual coverage reproducible; it does not change
Data runtime behavior or close Phase 2.

The final headless-Chrome run passed on macOS 26.6.1 arm64 with Chrome
154.0.8037.58, Rust 1.93.1, Node.js 22.23.2, and pnpm 11.7.0. The 33
route/viewport checks and 33 reduced-motion route/viewport checks passed; the
populated Data path returned three rows for snapshot digest
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`. The
query failure/recovery, focus behavior, and model/view preview and save
digests are recorded in the
[browser workflow packet](compute-bazaar-interface-data-browser-workflow-packet.md).
The eight POSTs were limited to synthetic seed, three queries, model preview
and save, and view preview and save. No other browser write or external
request occurred.

Two earlier attempts are retained as failures: the first stopped at argument
parsing before starting the service; the next exposed both the expected query
422 network log and a real invalid model/view HTML pattern. The latter was
fixed under the separate state slice below. Screenshots from the final run
were visually inspected and removed; the temporary home/profile and runner
processes were cleaned; port 4317 remained owned by PID 85094 and was not
contacted. Assurance is `INTERNAL_PASS` for this exact same-host browser
workflow only; reviewer status is `NOT_REQUESTED`, reproduction is
`NOT_REPRODUCED`, and Phase 2 remains `NOT_ASSIGNED`.

### Data model/view name pattern compatibility (2026-09-30)

State slice:
[`hyperai.bazaar.interface.data_form_pattern.v1`](state-slices.md#hyperai-bazaar-interface-data_form_patternv1).
Chrome showed that the existing Data model and view input patterns were
invalid when compiled with the `v` flag, so `requestSubmit()` failed before
the preview/save endpoints. The [HTML Standard](https://html.spec.whatwg.org/multipage/input.html)
specifies `RegExpCreate(pattern, "v")` for `pattern`; the
[ECMAScript RegExp specification](https://tc39.es/ecma262/2024/multipage/text-processing.html)
defines the class-set syntax that requires hyphen escaping. Both Data fields
now encode the literal hyphen with `\\x2d`, preserving the established
ASCII identifier set. Browser checks confirm valid hyphenated names pass and
names with a space or slash fail native constraint validation. Server
validation and the data contract are unchanged.

The final browser run reported no invalid-pattern/rendering or console/runtime
errors. The current-source interface file SHA-256 is
`f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd`.
Assurance is `INTERNAL_PASS` only for the two browser name fields and the
joined synthetic workflow. This does not establish screen-reader or
physical-keyboard acceptance, other browser/platform behavior, exhaustive
Data validation, independent review, or production readiness. Reviewer status
is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.

The root command `CARGO_NET_OFFLINE=true corepack pnpm run lint` passed,
including `lint:fast`, `verify:contracts`, and `verify:full`; the full gate
retained only declared environment-gated skips for Docker, PostgreSQL, and
staged-sidecar integration. `git diff --check`, scoped documentation hygiene,
and browser-runner syntax validation passed. Import preservation passed for
Simple Compute Market and HyperIntel; the known JustAI inventory discrepancy
(177 observed files versus 176 recorded) remains, and no imported tree was
modified. Overall Phase 2 remains `NOT_ASSIGNED`.

### Data preview failure and stale-confirmation browser qualification (2026-09-30)

State slice:
[`hyperai.bazaar.interface.data_failure_states_browser.v1`](state-slices.md#hyperai-bazaar-interface-data_failure_states_browserv1).
The separate opt-in browser run reaches real local model and view preview
errors: changing one byte in the disposable source file produces an integrity
conflict (HTTP 409), and a malformed view column is rejected (HTTP 422). It
checks that each form retains its editable values, exposes and focuses a
polite accessible status, and presents no confirmation after failure. A new
valid query clears old model and view confirmations; the runner retries and
saves each valid definition with a digest. The modified synthetic bytes are
restored exactly before the remaining workflow. No UI, API, or service behavior
changed.

On macOS 26.6.1 arm64 with Rust/Cargo 1.93.1, Node.js 22.23.2, pnpm 11.7.0, and
Chrome 154.0.8037.58, the browser command passed with 43 observed requests and
exactly 12 Data POSTs. The expected responses were one 409 model preview, one
422 view preview, and successful responses for the other ten POSTs. All 33
route/viewport and 33 reduced-motion checks passed. The screenshot was
inspected and deleted; temporary files/processes were cleaned and the listener
on `127.0.0.1:4317` remained PID 85094. The full offline root lint passed,
including all-feature workspace tests; environment-gated tests remained
ignored. Simple Compute Market and HyperIntel import inventories passed. The
known JustAI inventory mismatch remains 177 observed versus 176 recorded and
was not changed.

Assurance is `INTERNAL_PASS` for this exact same-host synthetic browser slice;
reviewer status is `NOT_REQUESTED` and reproduction is `NOT_REPRODUCED`. It
does not qualify screen-reader/physical-keyboard use, cross-browser or native
platform behavior, broader client-policy parity, source rights, independent
review, clean second-host reproduction, or full Phase 2 acceptance. Phase 2
remains `NOT_ASSIGNED`. See the
[packet](compute-bazaar-interface-data-failure-browser-packet.md) for exact
source digests, request/status counts, limits, and cleanup details.

### Data view keyboard correction continuation (2026-09-30)

The test-harness-only `verify:interface:data-keyboard` workflow passed on the
fixed synthetic sample. After setup, CDP keyboard events corrected an invalid
view column selection, submitted the failed and corrected previews, and
confirmed the digest-bound save. The rejected value remained editable and the
accessible status received focus. The runner made exactly the seven frozen
Data POSTs; the failure preview returned 422 and the corrected preview and
save returned 200. Root offline `corepack pnpm run lint` passed. This is
`INTERNAL_PASS` for the same-host headless-Chrome workflow only; it does not
qualify physical keyboard hardware, assistive technology, other browsers or
platforms, full Data accessibility, source rights, or overall Phase 2.
Reviewer status is `NOT_REQUESTED`, reproduction is `NOT_REPRODUCED`, and
Phase 2 remains `NOT_ASSIGNED`. See the [keyboard browser packet](compute-bazaar-interface-data-keyboard-browser-packet.md)
and its [state slice](state-slices.md#hyperai-bazaar-interface-data_keyboard_view_browserv1).

### Complete Data workflow keyboard continuation v2 (2026-09-30)

The fresh v2 identity corrected the v1 harness's failed select-all sequence by
selecting the full single-line SQL value with keyboard Home and Shift+End and
verifying the selection before typing. The complete synthetic keyboard path
then passed: route navigation, query, named focused result, model preview/save,
invalid view preview with retained input and HTTP 422, corrected preview, and
explicit digest-bound view save. The runner verified focus indicators and
exact same-origin request boundaries. This is `INTERNAL_PASS` for the exact
same-host headless-Chrome CDP workflow only; v1 remains `INCONCLUSIVE` and was
not repeated. It does not establish physical-keyboard or screen-reader use,
cross-browser/platform support, or overall Phase 2 acceptance. Reviewer status
is `NOT_REQUESTED`, reproduction is `NOT_REPRODUCED`, and Phase 2 remains
`NOT_ASSIGNED`. See the [v2 packet](compute-bazaar-interface-data-full-keyboard-v2-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-interface-data_full_keyboard_journeyv2).
The implementation and initial execution record passed the complete offline
root lint. Final gate-result addenda were documentation-only; `git diff --check`
and local Markdown-link validation passed. Simple Compute Market and
HyperIntel import pins matched; the existing JustAI inventory mismatch remains
177 observed versus 176 recorded and unchanged.

### Full Data keyboard AX-name continuation v3/v4 (2026-09-30)

V3 used exact Chromium role/name strings and stopped at the model-name field:
Chromium reported `MODEL NAME` because the existing CSS uppercases field
labels. It is recorded as `INCONCLUSIVE` for the full journey and was not
repeated. V4 uses exact roles and NFC, Unicode-whitespace, ASCII-case
normalization for names while retaining raw AX output. The full synthetic
keyboard query/model/view workflow passed with all 13 expectation rows, 33
route/viewport checks, 33 reduced-motion checks, and the frozen seven Data
POSTs. Root offline lint passed. Assurance is `INTERNAL_PASS` for this exact
same-host Chromium path; reviewer status is `NOT_REQUESTED`, reproduction is
`NOT_REPRODUCED`, and overall Phase 2 remains `NOT_ASSIGNED`. Simple Compute
Market and HyperIntel import pins passed; the unchanged JustAI mismatch remains
177 observed versus 176 recorded. Neither result establishes OS
screen-reader or physical-keyboard behavior, cross-browser/platform parity,
independent review, or production readiness. See the [v3 stop packet](compute-bazaar-interface-data-full-keyboard-ax-v3-packet.md),
[v4 packet](compute-bazaar-interface-data-full-keyboard-ax-v4-packet.md), and
[v4 state slice](state-slices.md#hyperai-bazaar-interface-data_full_keyboard_ax_semantic_namesv1).

### Related Phase 8 ACP keyboard and accessibility qualification (2026-09-30)

The local Agents & access ACP keyboard workflow is tracked separately from
Phase 2 Data acceptance. Its v1 runner failure and v2 Chrome-version mismatch
remain `INCONCLUSIVE`; v3 passed on exact Chrome 154.0.8037.92 with all 12 AX
expectations, keyboard focus restoration, the fixed synthetic response, and
exactly three HTTP 200 ACP POSTs. Root lint passed. Import verification passed
for Simple Compute Market and HyperIntel; the JustAI 177-observed versus
176-recorded mismatch remains unchanged. This local ACP result does not change
Phase 2's `NOT_ASSIGNED` status or establish full Phase 8, screen-reader,
physical-keyboard, cross-client, independent-review, or production readiness.
See the [v1 packet](compute-bazaar-interface-acp-keyboard-ax-packet.md),
[v2 packet](compute-bazaar-interface-acp-keyboard-ax-v2-packet.md), and
[v3 packet](compute-bazaar-interface-acp-keyboard-ax-v3-packet.md).

### Optional SELECT AST-field guards (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_select_ast_field_guards.v1`](state-slices.md#hyperai-bazaar-data_sql_select_ast_field_guardsv1).

The pinned `sqlparser 0.59.0` review found four `Select` fields without an
explicit local policy guard: `exclude`, `top_before_distinct`,
`value_table_mode`, and `window_before_qualify`. Production still parses only
with `GenericDialect`; test-only Redshift and BigQuery parses construct the
other dialect-specific AST shapes without adding dialect support. A shared
SELECT policy predicate now rejects all four fields. The window-order flag is
also tested after its associated named-window and qualify fields are removed,
so the regression exercises the flag's own guard. The ordinary GenericDialect
SELECT remains accepted. This is defensive parser coverage; it does not
identify a current data-access bypass or change accepted production SQL.

On pinned Rust/Cargo 1.93.1, `cargo test --locked --offline -p hyperai-data
--all-targets` passed 49 tests (30 unit, 5 catalog, 2 derived-data, and 12
query-contract). Pinned formatting passed. The full
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0, including workspace
Clippy, contract tests, the all-feature workspace suite, and doc tests;
environment-gated Docker, PostgreSQL, browser, and staged-sidecar checks
remained ignored. The changed source digest is
`d1ac65777103bca5cd1438397328c07453373e827d4714a99a526e37695aa483`.

Import preservation matched Simple Compute Market and HyperIntel. The
pre-existing JustAI inventory remains 177 observed files versus 176 recorded;
no imported tree changed. `git diff --check` passed. Assurance is
`INTERNAL_PASS` for the four AST-field guards and the exact ordinary SELECT
case. Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
This does not establish exhaustive SQL safety, planner equivalence,
DataFusion resource isolation, source rights, independent review, or full
Phase 2 acceptance; Phase 2 remains `NOT_ASSIGNED`.

### Incremental serialized-result size preflight (2026-09-30)

State slice: [`hyperai.bazaar.data_output_size_preflight.v1`](state-slices.md#hyperai-bazaar-data_output_size_preflightv1).

The JSON/CSV result collector previously serialized the full accumulated
result after every row and appended a row before checking the 256 KiB result
ceiling. It now counts the exact encoded size of each cell without first
materializing the formatted cell string, maintains an incremental result-byte
counter, and commits a row only after confirming it fits. Regressions compare
the counter with both serializers across null, Unicode, quote, backslash,
control-character, delimiter, newline, and carriage-return cases; they also
cover exact-limit acceptance and rejection without changing existing rows or
the committed byte count.

On pinned Rust/Cargo 1.93.1, `cargo test --locked --offline -p hyperai-data
--all-targets` passed 51 tests (32 unit, 5 catalog, 2 derived-data, and 12
query-contract). Pinned formatting and `CARGO_NET_OFFLINE=true corepack pnpm
run lint` passed. The source digest is
`6447a815b4f1d757eeb51823c0a20903cfd4e8a853e7290571f35428658389c5`.

Import verification matched Simple Compute Market (2,189 files) and
HyperIntel (99 files). The pre-existing JustAI inventory remains 177 observed
files versus 176 recorded; no imported tree changed. `git diff --check`
passed. Assurance is `INTERNAL_PASS` for the tested JSON/CSV byte-accounting
and no-over-limit-row-retention cases. Reviewer status is `NOT_REQUESTED`;
reproduction is `NOT_REPRODUCED`. No benchmark was run, so this records no
performance improvement; it does not establish CPU or whole-process memory
bounds, complete DataFusion allocation accounting, exhaustive SQL safety,
independent review, or full Phase 2 acceptance. Phase 2 remains
`NOT_ASSIGNED`.

### Interactive TUI query input (2026-09-30)

State slice:
[`hyperai.bazaar.data_tui_query_input.v1`](state-slices.md#hyperai-bazaar-data_tui_query_inputv1).
The Data workspace in the existing TUI now edits, clears, cancels, and submits
one bounded SQL line through the authenticated local Data route. Enter runs
the saved query; `e` edits; Ctrl+U clears while editing; Escape cancels editing
and preserves the draft. Input is limited to 4 KiB of UTF-8, control
characters are rejected, and editing or submitting clears stale rows. The
selected local catalog dataset is queried with a 100-row limit. Results and
query text stay in TUI memory; service errors display the stable sanitized
status, code, and message without exiting the TUI.

On macOS arm64 with pinned Rust/Cargo 1.93.1, the focused TUI suite passed 12
unit tests and 10 terminal-restore integration tests. A local-service test
used a temporary home and ephemeral IPv4 loopback port, seeded the fixed
HyperAI synthetic snapshot, and confirmed the edited query returned its
snapshot digest. `CARGO_NET_OFFLINE=true corepack pnpm run lint` passed. Port
4317 was not contacted.

Assurance is `INTERNAL_PASS` for this exact local interactive-query path;
reviewer status is `NOT_REQUESTED`, reproduction is `NOT_REPRODUCED`, and
overall Phase 2 remains `NOT_ASSIGNED`. This does not close parser safety,
accessibility, native platform, clean second-host, independent-review,
source-rights, or production gates.

### Focused Linux TUI query reproduction v1 (2026-09-30)

Under `hyperai.bazaar.data_tui_query_input_reproduction.v1`, the exact
`cargo test --locked --offline -p hyperai-tui --all-targets` command exited
101 before tests ran. The disposable Linux/amd64 container mounted its
temporary `/tmp` filesystem `noexec`, preventing Cargo from launching the
compiled `proc-macro2` build script. The frozen 286-file source, HyperIntel
import, and Git-status digests matched after the attempt; Simple Compute
Market still matched its import, and the preserved JustAI inventory mismatch
remained 177 observed files versus 176 recorded. The log, staged source, and
container were removed; no request reached port 4317 or a non-loopback source.

This result is `INCONCLUSIVE` environment evidence, not a TUI test failure or
pass. Reviewer status is `NOT_REQUESTED`, reproduction remains
`NOT_REPRODUCED`, and Phase 2 remains `NOT_ASSIGNED`. A fresh one-run
continuation explicitly enabled executable `/tmp` and passed 22 focused TUI
tests; see the v2 result immediately below and the
[`hyperai.bazaar.data_tui_query_input_reproduction.v2`](state-slices.md#hyperai-bazaar-data_tui_query_input_reproductionv2)
record.

### Focused Linux TUI query reproduction v2 (2026-09-30)

Under the v2 state slice, the same locked/offline `hyperai-tui` all-targets
command passed in one fresh Linux/amd64 container after `/tmp` was explicitly
mounted executable. It ran two result groups: 12 unit tests and 10 terminal
restoration tests, 22 passed and 0 failed. The test used the frozen 286-file
source manifest, Rust/Cargo 1.93.1, no network, read-only source and Cargo
cache mounts, and fixed HyperAI synthetic data. Pre/post source, HyperIntel,
and Git-status digests matched; Simple Compute Market still matched its
import, and the preserved JustAI inventory mismatch remained 177 observed
files versus 176 recorded. The output log was hashed and removed, followed by
the staged source and container. Port 4317 and non-loopback sources were not
contacted.

This is `REPRODUCED_LOCAL` for that exact TUI test path only, in same-owner
Linux/amd64 emulation. Reviewer status is `NOT_REQUESTED`; it is not native
Linux qualification and does not close accessibility, broad client-policy
parity, or overall Phase 2, which remains `NOT_ASSIGNED`.

### COLLATE parser/planner rejection (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_collate_guard.v1`](state-slices.md#hyperai-bazaar-data_sql_collate_guardv1).

The pinned `sqlparser 0.59.0` `GenericDialect` parser represents
`SELECT gpu_model COLLATE utf8_bin FROM dataset` as `Expr::Collate`. DataFusion
`52.5.0` has no matching SQL expression planner arm, so the shared validator
now rejects this expression before reading the registered dataset. The query
contract confirms rejection without creating an absent data root; the
authenticated API preserves HTTP 422 and `DATA_QUERY_REJECTED`.

Locked/offline `hyperai-data --all-targets` passed 51 tests. The named
authenticated `data_api` test passed, and root `corepack pnpm run lint` passed
formatting, strict Clippy, contract tests, the all-feature workspace suite,
and doc tests. `crates/hyperai-data/src/lib.rs` SHA-256 is
`2828b4c6d47f8d25bf35389e63922773dfd5c2d335c8a3c95002e69d635cbb41`.
Import verification still passes Simple Compute Market and HyperIntel; it
exits 1 on the unchanged JustAI count discrepancy (177 observed, 176
recorded). Assurance is `INTERNAL_PASS` for this exact local case; reviewer
status is `NOT_REQUESTED`, reproducibility is `NOT_REPRODUCED`. This does not
establish exhaustive SQL safety, complete planner equivalence, resource
isolation, or overall Phase 2 acceptance, which remains `NOT_ASSIGNED`.

### Unicode normalization parser/planner rejection (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_unplanned_expr_guards.v1`](state-slices.md#hyperai-bazaar-data_sql_unplanned_expr_guardsv1).

Revision: HyperAI baseline `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on
`main` plus the dirty local worktree. Assurance is `INTERNAL_PASS`; reviewer
status is `NOT_REQUESTED`, and reproducibility is `NOT_REPRODUCED`.

The pinned `sqlparser 0.59.0` `GenericDialect` parser represents
`SELECT gpu_model IS NFC NORMALIZED FROM dataset` as `Expr::IsNormalized`.
DataFusion `52.5.0` does not plan this expression, so the shared validator
rejects it before dataset access. The public query contract confirmed the
absent data root remains absent; the authenticated API returned HTTP 422 with
`DATA_QUERY_REJECTED`.

Locked/offline `hyperai-data --all-targets` passed 51 tests (32 unit, 5
catalog, 2 derived-data, and 12 query-contract); the focused authenticated API
test passed. `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 after
formatting, strict Clippy, contract checks, workspace tests, and doc tests.
Assurance is `INTERNAL_PASS` for this exact local parser,
pre-storage, and API behavior; reviewer status is `NOT_REQUESTED`, and
reproduction is `NOT_REPRODUCED`. The JSON-colon syntax check was removed from
scope because the production GenericDialect parser rejected it before AST
validation. This does not establish exhaustive SQL safety, planner
equivalence, complete resource accounting, external data rights, or overall
Phase 2 acceptance, which remains `NOT_ASSIGNED`. Custody was limited to the
fixed synthetic SQL and a disposable authenticated-API home; no dataset
payload was retained. The next gates are broader pinned-parser review,
DataFusion resource isolation, external-data rights, and a clean second
environment.

### `MEMBER OF` parser/planner rejection (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_member_of_guard.v1`](state-slices.md#hyperai-bazaar-data_sql_member_of_guardv1).

Revision: HyperAI baseline `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on
`main` plus the dirty local worktree. The fixed query
`SELECT gpu_model MEMBER OF (ARRAY['A100']) FROM dataset` parses under pinned
`sqlparser 0.59.0` `GenericDialect` as `Expr::MemberOf`. DataFusion `52.5.0`
does not plan that AST variant, so the shared validator rejects it before
opening the registered dataset. The public query contract leaves an absent
data root absent, and the authenticated API maps it to HTTP 422 with
`DATA_QUERY_REJECTED`.

Locked/offline `hyperai-data --all-targets` passed 52 tests (33 unit, 5
catalog, 2 derived-data, and 12 query-contract). The named authenticated API
test passed 1 test with 3 filtered; the final root API suite passed all 4
`data_api` tests. `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 with
the pinned Rust 1.93.1 toolchain: formatting, strict Clippy, interface and
installer checks, contract tests, the all-feature workspace suite, and doc
tests passed. A direct PATH `cargo` attempt initially selected Rust 1.87.0
and stopped before compilation; rerunning through the repository's pinned
toolchain succeeded without changing toolchain configuration.
An intermediate root lint also flagged the expanded expression test for
`clippy::too_many_lines` (104/100); the new `MemberOf` case was extracted
into its own test, and the final root lint passed.

Import verification passed Simple Compute Market (2,189 files; manifest
SHA-256 `afe6d9363a24fa627d9530240c190b6e68cdd1b7d6425d54d0b7547d829207ab`)
and HyperIntel (99 files; manifest SHA-256
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`). It
exited 1 only on the preserved JustAI mismatch: 177 observed files versus 176
recorded. No imported tree changed.

Source SHA-256 values are
`57f8e467442f5572066e75f4d4efb0964ddde84257783239a43557944d201a52` for
`crates/hyperai-data/src/lib.rs`,
`0a508bdfab88c6c39f205cdf9509cb34eff4c8e2b16af8093dcd61796b2b99d0` for
`crates/hyperai-data/tests/query_contract.rs`, and
`edf24fcb606c85a353f644774bc79d2eefb1d9660cb6ce5316d64620ef1398a5` for
`crates/hyperai-local-service/tests/data_api.rs`.

Assurance is `INTERNAL_PASS` for this fixed parser rejection, pre-storage
behavior, authenticated API mapping, and local root gate. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. This does not establish
exhaustive SQL safety, full planner equivalence, DataFusion resource
isolation, external-data rights, or overall Phase 2 acceptance, which remains
`NOT_ASSIGNED`. No provider, non-synthetic dataset, external service, or
port-4317 process was contacted.

### Schema-only DataFusion planning preflight (2026-09-30)

State slice: [`hyperai.bazaar.data_query_plan_preflight.v1`](state-slices.md#hyperai-bazaar-data_query_plan_preflightv1).

Purpose: move DataFusion SQL-to-logical and logical-to-physical planning ahead
of verified dataset snapshot capture by registering only an in-memory empty
table with the exact six-column HyperAI synthetic schema. This covers parser
forms the `GenericDialect` validator represents but the pinned DataFusion
planner does not support; it does not replace the existing AST restrictions,
function allowlist, resource limits, or verified execution snapshot.

Fixed regression: `SELECT gpu_model IN UNNEST(ARRAY['A100']) FROM dataset`.
The pinned parser represents this as `Expr::InUnnest`; DataFusion 52.5.0 has
no SQL-to-logical expression arm for that variant. The preflight must reject
it as `QueryRejected` before opening data, and the authenticated API must
return HTTP 422 / `DATA_QUERY_REJECTED` while leaving an unseeded data root
absent. Ordinary JSON/Parquet queries remain part of the proof gate.

This slice does not widen the query language or change the data schema,
dependencies, API, storage, clients, or source policy. The preflight must run
inside the existing two-second query timeout and cancellation boundary, must
not execute its empty-table plan, and must perform no file, network, or write
operation. Assurance is `INTERNAL_PASS` only for the exact preflight, query,
and authenticated-route cases after their gates pass; reviewer status is
`NOT_REQUESTED`, reproducibility is `NOT_REPRODUCED`, and overall Phase 2
remains `NOT_ASSIGNED`.

Execution result (2026-09-30): locked/offline `hyperai-data --all-targets`
passed 56 tests (35 unit, 5 catalog, 2 derived-data, 14 query-contract). The
focused authenticated API regression passed 1 test (4 filtered); the root
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 across formatting,
strict Clippy, contract tests, full workspace tests, and doc tests. Source
digests are recorded in the [state-slice result](state-slices.md#hyperai-bazaar-data_query_plan_preflightv1).
Import preservation matched Simple Compute Market and HyperIntel and retained
the known JustAI 177-versus-176 inventory discrepancy. This is
`INTERNAL_PASS` for the exact synthetic planner/API path only; reviewer status
is `NOT_REQUESTED`, reproduction is `NOT_REPRODUCED`, and overall Phase 2
remains `NOT_ASSIGNED`.

### Memory-capacity fixture schema alignment (2026-09-30)

State slice: [`hyperai.bazaar.data_query_capacity_schema_alignment.v1`](state-slices.md#hyperai-bazaar-data_query_capacity_schema_alignmentv1).

The first Data-package run after adding the preflight surfaced an existing
test-fixture inconsistency: the memory stress fixture declared `group_key` and
`amount`, but its manifest advertised the production six-column
`SYNTHETIC_SCHEMA`. The schema-only planner preflight correctly rejected the
public query before it reached the 64 MiB aggregation test. This is a fixture
failure, not evidence that production query behavior or the memory limit
changed. Historical measurements remain tied to the original two-column
fixture and must not be carried forward as measurements of the corrected
fixture.

The authorized correction changes only the test fixture and query in
`crates/hyperai-data/src/lib.rs`: emit 1,000 unique `instance_id` values and
the other five existing schema fields, retain exactly 12,039,000 bytes, group
by `instance_id` and `asking_usd_micros_per_hour`, and regenerate the fixture
digest. The proof gate requires the 1 MiB pool to reject with
`LimitExceeded` and no partial output, while the public verified query passes
the planner preflight and returns 1,000 rows under the configured 64 MiB pool.
Record updated peak RSS only from a fresh measurement bound to the new fixture
and test executable; if no fresh measurement is obtained, leave updated RSS
unknown.

Execution result (2026-09-30): exact revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main` plus the existing dirty
and untracked worktree. The corrected local fixture is 12,039,000 bytes,
contains 1,000 distinct `instance_id` values, and has SHA-256
`541516e964efa456f7b836c3495e61303da5ea9fe2b531737455aca1e4cde81b`. Locked,
offline `hyperai-data --all-targets` passed 54 tests (34 unit, 5 catalog, 2
derived-data, 13 query-contract); the 1 MiB pool rejected aggregation with
`LimitExceeded`, while the verified public query passed planner preflight and
returned 1,000 rows under the 64 MiB pool. The focused authenticated API case
passed and its full API suite passed all 5 tests.

A fresh `/usr/bin/time -l` run of the 64 MiB public-query test reported maximum
resident set size 124,256,256 bytes (118.5 MiB), with peak memory footprint
84,116,008 bytes. This is one local process measurement tied to the corrected
fixture and measured test binary; historical measurements remain bound to the
old two-column fixture.

`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0 with pinned Rust
1.93.1: formatting, strict Clippy, contract tests, all-feature workspace tests,
and doc tests passed. `git diff --check` passed. Import verification passed
Simple Compute Market (2,189 files; manifest SHA-256
`afe6d9363a24fa627d9530240c190b6e68cdd1b7d6425d54d0b7547d829207ab`) and
HyperIntel (99 files; manifest SHA-256
`62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`); it
exited 1 on the preserved JustAI mismatch (177 observed files versus 176
recorded). No imported tree changed.

The measured local test binary digest is
`10c181fa258647aa72bef07010cf5a6f5cb8e60af8226c69688d8b8cb47dbd08`; source
digests are recorded in the [state-slice result](state-slices.md#hyperai-bazaar-data_query_capacity_schema_alignmentv1).
Fixture bytes stayed in disposable local test directories. Assurance is
`INTERNAL_PASS` for this exact synthetic capacity, planner, API, and root-gate
scope; reviewer status is `NOT_REQUESTED`, reproducibility is
`NOT_REPRODUCED`, and overall Phase 2 remains `NOT_ASSIGNED`. This does not
establish general capacity, exhaustive parser safety, DataFusion isolation,
external-data rights, or production readiness. Non-synthetic source access,
provider calls, Fleet operation, paid Eval, financial Trade, settlement,
publication, and deployment remain outside this slice.

### `SELECT INTO` pre-storage regression coverage (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_select_into_pre_storage_test.v1`](state-slices.md#hyperai-bazaar-data_sql_select_into_pre_storage_testv1).

The production validator already rejects the pinned parser's `Select.into`
field. Existing seeded-root coverage proves rejection, but does not establish
that the public query and authenticated route reject this write-intent form
before any data-root creation or lookup. The new test-only slice adds the
parsed-AST assertion, absent-root public query case, and absent-root HTTP 422 /
`DATA_QUERY_REJECTED` case. It does not change production behavior or add SQL
write support. Locked/offline `hyperai-data --all-targets` passed 56 tests, the
focused authenticated API case passed 1 test (4 filtered), and
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0. Targeted whitespace
checks passed. Import verification matched Simple Compute Market and HyperIntel
and retained the known JustAI mismatch (177 observed files versus 176 recorded);
no imported tree changed. Assurance is `INTERNAL_PASS` for this exact regression
and root-gate scope; reviewer status is `NOT_REQUESTED`, reproduction is
`NOT_REPRODUCED`, and overall Phase 2 remains `NOT_ASSIGNED`. Port 4317 was not
contacted, and no dataset or external service was used.

### Complete serialized response-byte limit (2026-09-30)

State slice: [`hyperai.bazaar.data_serialized_response_limit.v1`](state-slices.md#hyperai-bazaar-data_serialized_response_limitv1).

`QueryLimitsV1.max_result_bytes` now caps the complete compact JSON encoding
of `QueryResultV1`, including both the `rows` and optional `csv` fields. The
existing `output_bytes` field continues to mean bytes in the selected JSON or
CSV representation. A bounded counting writer checks the response encoding
without allocating a second full response buffer. The schema, query API, and
stable `DATA_LIMIT_EXCEEDED` mapping remain unchanged.

The exact local regression confirms compact serialization at the 262,144-byte
boundary and rejection one byte above it. A fixed three-row synthetic CSV
query contains a 50,000-byte U+0001 value: its selected CSV output remains
under the cap, but JSON escaping and the duplicate `rows`/`csv` representation
make the complete response exceed it. The query returns `LimitExceeded`; the
authenticated API returns HTTP 413 / `DATA_LIMIT_EXCEEDED` without partial
`rows` or `csv` output.

Locked/offline `hyperai-data --all-targets` passed 58 tests (36 unit, 5
catalog, 2 derived-data, 15 query-contract); the focused authenticated
`data_api` suite passed all 6 tests. `CARGO_NET_OFFLINE=true corepack pnpm run
lint` exited 0. Assurance is `INTERNAL_PASS` for this exact local synthetic
contract; reviewer status is `NOT_REQUESTED`, reproducibility is
`NOT_REPRODUCED`, and overall Phase 2 remains `NOT_ASSIGNED`. Simple Compute
Market and HyperIntel imports match; the pre-existing JustAI manifest
mismatch remains 177 observed files versus 176 recorded. Exact source digests
and the full claim ceiling are in the [state-slice record](state-slices.md#hyperai-bazaar-data_serialized_response_limitv1).

### Bounded SQL red-team review (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_redteam_review.v1`](state-slices.md#hyperai-bazaar-data_sql_redteam_reviewv1).

The bounded source review found no confirmed query path to an arbitrary table,
file or object-store URL, network source, SQL write, or partial successful
response in the exact pinned implementation. Validation restricts requests to
one parsed `SELECT` over the registered `dataset`; schema-only DataFusion
planning runs before snapshot capture; execution applies the configured query
and output limits, and the service maps policy and limit errors to stable
HTTP 422 and 413 responses.

The review is `INTERNAL_PASS` for the exact source and fixed local query
contract only. It reviewed source and existing tests; this slice added and ran
no code or tests. Parent reproduction v15 covered the same 286-file source
manifest with 421 passed, 0 failed, and 10 ignored across 82 result groups,
but reproducing that test suite does not reproduce the manual review.
Reviewer status is `NOT_REQUESTED`, review reproducibility is
`NOT_REPRODUCED`, and overall Phase 2 remains `NOT_ASSIGNED`.

The finding does not establish exhaustive SQL safety. At the time of this
review, the fixed tests did not generate queries from grammar productions. A
later finite deterministic corpus now covers selected supported combinations
and parsed negative forms; it is structured coverage, not grammar-aware
fuzzing. AST/dependency changes need a fresh review. A 1 MiB
injected-pool test rejects a fixed aggregation while a fixed workload succeeds
under the 64 MiB DataFusion pool; neither proves process isolation. Default
pool exhaustion through the authenticated route, complete allocation
accounting, process-level memory isolation, accessibility and native platform
qualification, separate-host reproduction, and independent review remain
open. Source rights and authorization remain prerequisites for non-synthetic
inputs. Test data stayed in disposable local homes; port 4317 and external
services were not contacted. See the exact scope, hashes, claim ceiling, and
custody record in the [state slice](state-slices.md#hyperai-bazaar-data_sql_redteam_reviewv1).

### Structured SQL grammar corpus (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_grammar_corpus.v1`](state-slices.md#hyperai-bazaar-data_sql_grammar_corpusv1).

This test-only continuation closes one bounded gap in the parser review. The
existing fixed-seed 128-case arbitrary-byte property test checks that
validation does not panic; it does not generate query syntax from grammar
productions or exercise combinations of supported clauses.

The deterministic unit corpus checks 216 positive query combinations: four
known columns, registered `dataset` and `dataset AS d`
relations, three predicate forms, three ordinary ordering forms, and three
bounded `LIMIT`/`OFFSET` forms. It also checks 11 fixed parser-valid negative
statements covering unsupported table sampling, unknown table, unallowlisted
function, typed alias columns, `SELECT INTO`, CTE, set operation, join,
`FETCH`, `GROUP BY ALL`, and per-key `LIMIT`. Positive combinations must
validate; each negative input must parse as one statement and return
`QueryRejected`.

The test calls only the pinned parser and private validator. It performs no
DataFusion planning or query execution, opens no dataset or service, and uses
no new dependency or generated fixture. The focused locked/offline test passed
1 case, and the locked/offline `hyperai-data --all-targets` suite passed 59
tests across four result groups with 0 failures and 0 ignored. The resulting
`crates/hyperai-data/src/lib.rs` SHA-256 is
`89eb4d74c0479976f194bd124017fc7444f34160b3ff0e844412425550d8732a`.
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0, including pinned
formatting, strict Clippy, workspace contract tests, and all-feature tests.
Environment-gated browser, sidecar, and Docker cases remained ignored.
Scoped Markdown whitespace, final-newline, and local-link checks passed. The
claim ceiling is
the exact finite grammar corpus and pinned parser/code. This is not full
grammar fuzzing, exhaustive SQL safety, planner equivalence, resource isolation,
independent review, or overall Phase 2 acceptance. Full execution evidence is
in the [state-slice record](state-slices.md#hyperai-bazaar-data_sql_grammar_corpusv1).
Phase 2 remains `NOT_ASSIGNED`.

### Default-pool authenticated API probe (2026-09-30)

State slice: [`hyperai.bazaar.data_default_pool_api_probe.v1`](state-slices.md#hyperai-bazaar-data_default_pool_api_probev1).

This bounded local probe targets the remaining question: can an authenticated
query exhaust the configured 64 MiB DataFusion pool while staying beneath the
separate input, row, response-size, and query-time ceilings? The local
synthetic manifest contract fixes `row_count` at three, so the probe creates
three rows with distinct `instance_id` values, multi-megabyte synthetic
padding values, and an exact total input size of 15,990,000 bytes. The
authenticated query is
`SELECT COUNT(*) FROM dataset GROUP BY instance_id`, capped at three rows; its
expected output is compact and contains no source padding.

The test uses a disposable local HyperAI home and the authenticated loopback
route. It changes no production behavior, dependency, SQL policy, or resource
limit. The probe distinguishes a fast HTTP 413 / `DATA_LIMIT_EXCEEDED` with no
partial rows and elapsed time below 1,900 ms from a complete three-row result.
Given that the fixture and response are under their limits, a fast 413 would
support the narrow inference that the DataFusion pool limit caused the
rejection before the two-second query timeout. A successful result means this
candidate did not exhaust the pool. A slower 413 or any other unexpected
response remains inconclusive. Neither outcome establishes process-level
memory isolation or complete allocation accounting.

Initial fixture attempt: an otherwise similar 1,000-row manifest was rejected
with HTTP 500 / `DATA_OPERATION_FAILED` in 18.2 ms because the local catalog
contract requires exactly three rows. The rejection occurred before the
DataFusion query and provides no pool evidence. The authorized probe now
preserves that contract and uses three large synthetic rows instead.

The initial 1,000-row fixture attempt returned HTTP 500 /
`DATA_OPERATION_FAILED` in 18.2 ms because the current synthetic manifest
contract fixes `row_count` at three; it was rejected before query execution.
The corrected three-row workload passed the authenticated route in
297.392125 ms and returned 3 rows, a 642-byte JSON response, and 51 result
bytes under the configured 67,108,864-byte DataFusion pool. Its exact input is
15,990,000 bytes with SHA-256
`87d18776eca0db963039f77ef345c06e602a8b822b0a26a19ba9f5a082cb57d8`. The
focused test passed 1/1, the locked/offline `data_api` target passed 7/7, and
the final offline root lint passed. This candidate did not exhaust the pool;
the broader default-pool gate remains open.

Assurance is `INTERNAL_PASS` for this bounded local synthetic API case only;
reviewer status is `NOT_REQUESTED`, reproduction is `NOT_REPRODUCED`, and
overall Phase 2 remains `NOT_ASSIGNED`. No provider, external source, paid
execution, live Fleet, Trade, settlement, federation, or publication is
authorized. Process memory isolation, complete allocation accounting, and
pool exhaustion with a larger supported workload are not established. Exact
scope and custody are in the [state-slice
entry](state-slices.md#hyperai-bazaar-data_default_pool_api_probev1).

### Fixed-seed grammar SQL sweep (2026-09-30)

State slice: [`hyperai.bazaar.data_sql_grammar_fuzz.v1`](state-slices.md#hyperai-bazaar-data_sql_grammar_fuzzv1).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main` plus
the preserved dirty and untracked worktree.

The test-only generator uses fixed seed `0x4932`, retains no corpus, and
streams 4,096 positive and 4,096 negative SQL strings through the pinned
`sqlparser 0.59.0` parser and private `validate_query`. Positive productions
combine the existing allowlisted projections, registered table forms,
predicates, orderings, and bounded `LIMIT`/`OFFSET`; generated cases vary
keyword casing, SQL whitespace, and numeric/string literals. Negative
productions cover unregistered table names, generated unallowlisted function
names, `TABLESAMPLE`, per-key `LIMIT`, and `GROUP BY ALL`. Every case is
bounded by `MAX_SQL_BYTES`; negatives must parse as one query before being
required to return `QueryRejected`. The sweep performs no DataFusion planning
or query execution and reads no data root.

The focused locked/offline test passed 1/1 in 0.22 seconds. The locked/offline
`hyperai-data --all-targets` run passed 60 tests across four groups before the
final formatting-only and fixed-width PRNG cleanup; the subsequent full root
`CARGO_NET_OFFLINE=true corepack pnpm run lint` passed on the final bytes,
including the 38 Data unit tests, 5 catalog tests, 2 derived-data tests, and
15 query-contract tests. Root lint also passed formatting, strict Clippy,
workspace contract tests, all-feature tests, and doc tests. Environment-gated
browser, PostgreSQL, Docker, and staged-binary tests remained ignored.

The import verifier matched Simple Compute Market and HyperIntel; the
preserved JustAI inventory remains 177 observed files versus 176 recorded.
The exact final test source SHA-256 is
`cd256a4a88d5abb012a62af0102870ab2fce53404ba8eeebb7756ff2c8d598d6`.
Assurance is `INTERNAL_PASS` for this bounded validator sweep and the root
local test command only; reviewer status is `NOT_REQUESTED`, and
reproducibility is `NOT_REPRODUCED`. Data/artifact custody is limited to the
transient SQL strings and validator results; no dataset or generated artifact
was retained. This does not establish exhaustive SQL safety, DataFusion
planner equivalence, process-memory isolation, or overall Phase 2 acceptance.
Phase 2 remains `NOT_ASSIGNED`; its resource, accessibility, native-platform,
clean-reproduction, source-rights, and independent-review gates remain open.

### Manifest row-count binding and authenticated 1,000-row pool candidate (2026-09-30)

State slice: [`hyperai.bazaar.data_manifest_row_count_binding.v1`](state-slices.md#hyperai-bazaar-data_manifest_row_count_bindingv1).

A current-source audit found that the internal 1,000-row memory fixture
contained 1,000 distinct rows while `build_manifest` fixed `row_count` to 3
and `load_manifest_record` required that value. The content digest covered the
bytes, but the manifest's source-row count did not describe them. The previous
authenticated default-pool probe correctly used three rows and found that its
15,990,000-byte aggregate did not exhaust the 64 MiB pool.

This continuation binds manifest `row_count` to non-empty NDJSON records or
Parquet file metadata. The deterministic seed remains three rows. The local
1,000-row stress fixture uses an accurate count and keeps the existing
16 MiB file, 1,000 result-row, 256 KiB response, and two-second query ceilings
unchanged. One authenticated loopback request then distinguishes a complete
bounded result from a fast resource-limit refusal without partial rows.

Implementation outcome: `DatasetManifestV1.row_count` is checked against the
number of non-empty NDJSON records or the Parquet footer row count. The
deterministic JSON and Parquet seeds remain three rows. The internal 1,000-row
stress fixture now has an accurate manifest. One focused Data regression
rewrites each seeded manifest count to two and confirms both catalog and query
reject it as `InvalidCatalog`.

Execution (2026-09-30): pinned Rust/Cargo `1.93.1 (01f6ddf75 2026-02-11)`;
`CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked
--offline -p hyperai-data --all-targets` passed 61 tests across its unit and
integration targets. The authenticated API candidate then passed under
`CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked
--offline -p hyperai-local-service --test data_api
authenticated_thousand_row_aggregate_probes_default_memory_pool --
--exact --nocapture`. It returned HTTP 200 in 336 ms with all 1,000 rows,
the expected snapshot digest, a 6,632-byte response, and the configured
67,108,864-byte pool. The exact 15,990,000-byte synthetic input digest is
`b5dd89ba08093290ddde6fa557bb2c997b69181692fb3fee8200be54ff820c39`.
This candidate did not exhaust the pool. It is not a process-memory bound or
allocation-accounting result.

The first root lint attempt stopped at a rustfmt difference in the new test
assertion. After formatting that assertion, the next attempt found
Clippy's `large_stack_arrays` warning for the fixed-width NDJSON counter
buffer; it was moved to a heap-backed buffer. The final
`CARGO_NET_OFFLINE=true corepack pnpm run lint` passed interface and installer
checks, formatting, strict workspace Clippy, contract tests, all-feature
workspace tests, and doc tests. Environment-gated browser, Docker, staged
sidecar, and Postgres tests remained ignored. The Simple Compute Market and
HyperIntel import inventories matched their recorded pins. Import verification
still exits nonzero on the pre-existing JustAI inventory mismatch: 177 files
observed versus 176 recorded; this slice leaves it unchanged. No test used or
connected to the pre-existing port 4317.

Final source hashes: `crates/hyperai-data/src/lib.rs`
`cfc0e768699ddf8ac3679113c10b434fac776e459d9273c2b436ab6ea46d6213` and
`crates/hyperai-local-service/tests/data_api.rs`
`434a811cf99d57e517f9fee364df6f69abff0a7aa937c2644797d68dde8c3e02`.
Data custody was limited to HyperAI-authored synthetic inputs under disposable
temporary homes, removed with their test directories. Assurance is
`INTERNAL_PASS` for the exact manifest and local authenticated-query
behaviors; reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Phase 2 remains `NOT_ASSIGNED`. Native-platform and full
accessibility qualification, separate-host reproduction, broader DataFusion
resource isolation, upstream source rights, and independent review remain
open. No provider, external source, paid execution, live Fleet, Trade,
settlement, federation, publication, or production action is authorized.

### Supported SQL grammar schema-plan continuation (2026-10-01)

State slice: [`hyperai.bazaar.data_sql_grammar_planner.v1`](state-slices.md#hyperai-bazaar-data_sql_grammar_plannerv1).

The existing 216-case deterministic positive grammar corpus now checks the
production schema-only DataFusion preflight after the existing validator
assertion. Every statement stayed within the pinned read-only subset and
produced a physical plan using the synthetic Arrow schema and an in-memory
`EmptyTable`. The 11 fixed negative statements remain parser-valid and
rejected by the validator. No dataset bytes, query execution, service, or
external endpoint were involved.

The first focused command accidentally used `--exact` without the
module-qualified test name and ran zero tests. That invocation is not counted.
The corrected locked/offline focused command ran the named test and passed
1/1. The complete locked/offline `hyperai-data --all-targets` suite passed
61/61 (39 unit, 5 catalog, 2 derived-data, and 15 query-contract tests).
`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0: 16 interface tests,
3 installer tests, formatting, strict workspace Clippy, contract-filtered
tests, all-feature workspace tests, and doc tests passed. Environment-gated
browser, Docker, staged-sidecar, and PostgreSQL tests were not run.

`python3 scripts/verify_imports.py` exited 1 because the preserved JustAI
inventory is 177 files observed versus 176 recorded. The Simple Compute
Market 2,189-file and HyperIntel 99-file inventories match. This result is
unrelated to this test-only slice and was left unchanged. The exact code hash,
commands, custody, and claim ceiling are in the
[state-slice record](state-slices.md#hyperai-bazaar-data_sql_grammar_plannerv1).
This is `INTERNAL_PASS` for the exact 216 accepted statements and the local
root gates, not query execution, exhaustive planner equivalence, SQL safety,
independent review, or overall Phase 2 acceptance. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`; Phase 2 remains
`NOT_ASSIGNED`. Native/accessibility qualification, separate-host
reproduction, resource isolation, source rights, and independent review
remain open. No external data, provider, live Fleet, paid Eval, financial
Trade, settlement, federation, publication, or production action is enabled.

### macOS Virtualization host preflight (2026-10-01)

State slice: [`hyperai.bazaar.data_macos_virtualization_preflight.v1`](state-slices.md#hyperai-bazaar-data_macos_virtualization_preflightv1).

On macOS 26.6.1 arm64 with Swift 6.4.0, a standalone ad-hoc-signed helper
carrying only `com.apple.security.virtualization` passed strict signature
verification. `VZVirtualMachine.isSupported` returned true, and a no-device,
one-vCPU, 512-MiB placeholder Linux configuration passed
`VZVirtualMachineConfiguration.validate()`. No VM was instantiated or
started; no Linux image, dataset, network device, or external source was used.
Helper source and binary digests, exact commands, and cleanup are in the
[preflight packet](compute-bazaar-data-macos-virtualization-preflight-v1.md).

Assurance is `INTERNAL_PASS` for this host support/entitlement/configuration
preflight only. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. This establishes neither a distribution signing path nor
guest/cgroup/DataFusion resource enforcement, lifecycle recovery,
host-plus-guest capacity, or macOS product support. Overall Phase 2 remains
`NOT_ASSIGNED`. Next gate: freeze and rights-check a specific arm64 Linux
guest image and the bounded no-network IPC, cgroup, failure, cleanup, and
capacity contract before starting any VM.

### macOS guest source and format decision (2026-10-01)

Under [`hyperai.bazaar.data_macos_guest_source_check.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_source_checkv1), the official dated
Ubuntu arm64 snapshot and its signed-checksum verification method, signer
fingerprint, internal-use rights, Apple's RAW/ASIF disk requirement, and local
verification/ext4-tool availability were recorded. The offered bootable `.img`
is QCOW2 and cannot be attached directly; building a RAW disk from the root
filesystem archive is only a candidate because the exact boot composition was
not tested or frozen. No image, checksum, signature, or key bytes were
downloaded. Assurance is `PLANNING_ONLY` for the source/rights/format review;
the ready-to-boot artifact path is `INCONCLUSIVE`. The date-specific URL does
not freeze artifact bytes; its signed SHA256 digest must be verified in a new
slice. Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_APPLICABLE`.
Phase 2 remains `NOT_ASSIGNED`. A new acquisition/conversion/lifecycle packet
is required before any artifact download or VM launch. See the [source-check
packet](compute-bazaar-data-macos-guest-source-check-v1.md).

### macOS guest image construction slice (2026-10-01)

[`hyperai.bazaar.data_macos_guest_image_construction.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_image_constructionv1)
authenticated the dated Ubuntu arm64 root archive and inspected its bounded
member list, but the sole RAW build attempt failed on the mode-restricted
`/var/lib/snapd/void` directory. No candidate image was completed or exported;
the v1 slice is closed `INCONCLUSIVE`, with its warning and exact digests in
the [construction packet](compute-bazaar-data-macos-guest-image-construction-v1.md).
The separate [`hyperai.bazaar.data_macos_guest_image_continuation.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_image_continuationv1)
completed `INTERNAL_PASS` for one offline ext4 construction and read-only
integrity check using only `DAC_READ_SEARCH` plus `MKNOD` inside a
capability-limited container. The exported 4-GiB image is local-only and
remains in the named temporary directory; the archive contains no
kernel/initramfs, so the image is not bootable evidence. Neither slice permits
a VM launch or product changes. Phase 2 remains `NOT_ASSIGNED`;
cgroup/resource enforcement, no-network IPC, failure recovery, cleanup, and
host-plus-guest capacity remain open. The separate
[`hyperai.bazaar.data_macos_guest_boot_inputs.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_boot_inputsv1)
completed `INTERNAL_PASS` for one bounded download and signature/checksum
verification of the matching dated arm64 kernel and initrd. The bytes are
local-only; this still does not authorize VM creation or boot. A new exact VM
packet must bind these digests to the root disk and fixed lifecycle/resource
limits. See the [continuation packet](compute-bazaar-data-macos-guest-image-continuation-v1.md)
and [boot-input packet](compute-bazaar-data-macos-guest-boot-inputs-v1.md).

### macOS guest boot packet (2026-10-01)

The separate
[`hyperai.bazaar.data_macos_guest_boot.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_bootv1)
packet bound the previously authenticated kernel/initrd and constructed RAW
root disk to one local Apple Virtualization attempt: 1 vCPU, 1 GiB, read-only
ext4 root, output-only console, no network or shared paths, a 120-second boot
ceiling, and a 1 MiB log cap. Its one helper compile failed on the Swift
`@main` entry-point flag and serial-port API type; no VM validation or start
occurred. The slice is closed `INCONCLUSIVE`; a fresh continuation packet is
required to correct those diagnostics before another attempt. Overall Phase 2
remains `NOT_ASSIGNED`; DataFusion resource enforcement, hostile-code
isolation, cancellation/recovery, capacity, and macOS product support remain
unqualified.

### macOS guest boot continuation (2026-10-01)

The first exact guest-boot packet closed `INCONCLUSIVE` because its one
helper compile had two Swift/API diagnostics; no VM was instantiated or
started. The fresh
[`hyperai.bazaar.data_macos_guest_boot_continuation.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_boot_continuationv1)
packet addressed those diagnostics with `-parse-as-library` and the SDK's
`serialPorts` configuration. The helper compiled, its single virtualization
entitlement passed strict signature verification, and configuration
validation passed. Its one VM start call returned `VZErrorDomain` code `1`
(`VZErrorInternal`) before guest output; no stop callback was observed, no guest boot occurred,
and the root disk hash stayed unchanged. The slice is closed `INCONCLUSIVE`.
Another start would require a fresh diagnostic packet and authorization.
Phase 2 remains `NOT_ASSIGNED`; resource isolation, recovery, and platform
qualification remain open.

### macOS guest start diagnostic (2026-10-01)

The failed VM start returned `VZErrorInternal`, the generic internal error
classification. A single bounded, read-only host-log query is scoped by
[`hyperai.bazaar.data_macos_guest_start_diagnostic.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_start_diagnosticv1)
to look for an explicit cause. It authorizes no helper execution or new VM
start. The query found a DiskImages2 message that RAW does not support Disk
Image user-data retrieval (error 45), but no record linked it to the
`VZErrorInternal` start failure. The exact query-window end was not captured,
so the diagnostic is `INCONCLUSIVE` for its frozen time-window boundary; the
cause remains unknown. Any additional VM start requires a separate exact
packet and explicit authorization. Overall Phase 2 remains `NOT_ASSIGNED`.

### Current SQL source review (2026-10-01)

The fresh read-only review under
[`hyperai.bazaar.data_sql_current_source_review.v1`](state-slices.md#hyperai-bazaar-data_sql_current_source_reviewv1)
matched all seven frozen source, test, dependency, and lockfile inputs before
and after inspection. No defect or confirmed policy bypass was identified in
the reviewed authenticated query path. The source binds the service to
loopback, authenticates the query route, limits SQL and request size, validates
one supported `SELECT`, preflights it against a schema-only table before
capturing a digest-verified local snapshot, and applies query, row, column, and
complete-response limits with typed API errors.

The review inspected fixed test definitions but ran no tests and read no
dataset bytes. It does not establish exhaustive SQL safety. The 64 MiB
DataFusion memory pool does not establish process-level or host memory
isolation, and full allocation accounting remains open. Runtime assurance is
not assigned; reviewer status is `NOT_REQUESTED`; review reproducibility is
`NOT_REPRODUCED`. External data rights, native/accessibility qualification,
second-environment reproduction, and independent review remain open. Overall
Phase 2 remains `NOT_ASSIGNED`. Exact hashes, scope, custody, and limits are in
the [review packet](compute-bazaar-data-current-sql-review-packet.md).

### Instrumented macOS guest-start continuation (2026-10-01)

Under [`hyperai.bazaar.data_macos_guest_start_instrumented.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_start_instrumentedv1), the exact signed helper and host preflight passed, but the sole VM start returned `VZErrorDomain` code `1` after 86 ms with the generic description “Internal Virtualization error. The virtual machine failed to start.” The post-error state was `rawValue: 3`, `canStart=true`, `canStop=false`; no serial output appeared. This is `INCONCLUSIVE` for guest boot and establishes no memory bound, cgroup control, worker isolation, or macOS Data support.

The post-run RAW disk, kernel, and initrd sizes and digests match their
pre-run values. The helper and Virtualization service exited and the
temporary run root was removed. The exact helper log query saw the earlier
DiskImages2 RAW user-data error 45, but no daemon record linked it to the
start failure.

### Bounded Virtualization log follow-ups (2026-10-01)

The exact-window `virtualizationd`-only query under
[`hyperai.bazaar.data_macos_virtualizationd_window.v1`](state-slices.md#hyperai-bazaar-data_macos_virtualizationd_windowv1)
returned no records. A second bounded query across Virtualization-related
processes/subsystems under
[`hyperai.bazaar.data_macos_virtualization_subsystem_window.v1`](state-slices.md#hyperai-bazaar-data_macos_virtualization_subsystem_windowv1)
returned 22 records: `launchd` started a Virtualization VM service for helper
PID `24461`; that service ran for 84 ms and exited with status 0. The
correlated framework breadcrumb did not expose a cause, and the guest serial
file was empty. Both queries are `INTERNAL_PASS` only for their exact bounded
log-query scope; the VM cause remains `INCONCLUSIVE`. After two identical
start errors and this bounded research, no further VM retry is authorized
under these identities. A changed memory, serial, storage, boot-input, or
signing configuration requires new source-grounded analysis, a fresh exact
packet, and explicit user authorization. Overall Phase 2 remains
`NOT_ASSIGNED`.

### Docker Desktop Linux cgroup candidate probe (2026-10-01)

Under
[`hyperai.bazaar.data_docker_cgroup_probe.v1`](state-slices.md#hyperai-bazaar-data_docker_cgroup_probev1),
the cached Rust 1.93.1 container image ran with `--network=none`. Docker
Desktop's Linux/aarch64 VM exposed cgroup v2. The cached image was
`linux/amd64` and ran under emulation. With a 128 MiB container limit and
swap disabled, the helper read `memory.max=134217728` and
`memory.swap.max=0`; its parent observed the 512 MiB synthetic child killed
by signal 9 and `memory.events` incrementing `oom` and `oom_kill` to 1. Both
containers and the exact temporary home were removed; source and binary hashes
are recorded in the [probe packet](compute-bazaar-data-docker-cgroup-probe-v1.md).

This is `INTERNAL_PASS` only for cgroup enforcement inside this Docker Desktop
Linux VM. It is not native Linux-host or macOS process isolation evidence and
does not qualify a HyperAI worker, IPC, cancellation, capacity, or product
runtime dependency on Docker. It does not resolve the failed macOS
Virtualization feasibility path. Overall Phase 2 remains `NOT_ASSIGNED`; a
separate platform/worker architecture decision is still required.

### Linux worker launch-boundary assessment (2026-10-01)

Under
[`hyperai.bazaar.data_worker_launch_boundary_design.v1`](state-slices.md#hyperai-bazaar-data_worker_launch_boundary_designv1),
the current foreground service path was traced from the authenticated Data
route through the application facade to in-process DataFusion execution. No
worker binary exists. Official systemd documentation shows that an
unprivileged `Delegate=yes` unit does not receive controller access, so the
current user-launched CLI has no established authority to enforce cgroup
`memory.max`. Landlock and seccomp are separate confinement controls with
kernel/ABI and architecture constraints; neither makes cgroup memory authority
appear. No launch model was selected, and no code, dependency, service,
cgroup, or VM was changed. The in-process path remains the current behavior;
bounded-worker mode remains unavailable. Overall Phase 2 stays
`NOT_ASSIGNED`. See the [launch-boundary packet](compute-bazaar-data-worker-launch-boundary-v1.md)
and [state-slice record](state-slices.md#hyperai-bazaar-data_worker_launch_boundary_designv1).

### Linux worker launch-model decision

The follow-on [`hyperai.bazaar.data_worker_linux_launch_model.v1`](state-slices.md#hyperai-bazaar-data_worker_linux_launch_modelv1)
selects an operator-managed delegated cgroup v2 subtree for the first Linux
worker candidate. It adds no worker code or host configuration. Numeric
resource limits, Landlock/seccomp policy, lifecycle behavior, and
service-plus-two-worker capacity remain open for a separate native Linux
implementation packet. Phase 2 remains `NOT_ASSIGNED`.

### Current-source focused package reproduction (2026-10-02)

Under
[`hyperai.bazaar.data_current_source_reproduction.v1`](state-slices.md#hyperai-bazaar-data_current_source_reproductionv1),
the exact locked/offline Data crate, authenticated Data API, and TUI library
commands passed in a disposable no-network Linux/amd64 container against the
current 226-file Rust source manifest. Across six result groups, 93 tests
passed with no failures or ignored cases. The combined test-log digest and
environment are recorded in the [reproduction packet](compute-bazaar-data-current-source-reproduction-v1.md).
This is `REPRODUCED_LOCAL` for those exact packages and source identity only;
it does not qualify native Linux, Data-worker process isolation, or overall
Phase 2. Phase 2 remains `NOT_ASSIGNED` pending its separate worker/platform
and broader qualification gates.
