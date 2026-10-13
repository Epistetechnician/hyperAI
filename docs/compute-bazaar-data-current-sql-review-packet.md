# HyperAI Data Current-SQL Read-Only Review Packet

Status: bounded static source review completed for the exact frozen files.
This packet grants no code or test changes and no runtime, data, provider,
release, or external effect authority.

State slice: `hyperai.bazaar.data_sql_current_source_review.v1`.
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Implementation sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Frozen working-tree bytes are
identified below; the Git revision alone does not identify them.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose

Re-review the exact current read-only SQL request path after later local parser
and test changes made the earlier bounded source-review digest stale. Rebuild
the trust boundary from the parser and validator through schema-only planning,
verified snapshot access, query execution, result limits, and the authenticated
HTTP response. Compare the frozen fixed tests with that path. This is static
source review only; it does not re-run the earlier review's conclusions.

## Frozen files

SHA-256 and byte lengths are checked before and after review. Any mismatch
invalidates this packet; stop without a review report and freeze a new identity.

| File | Bytes | SHA-256 |
|---|---:|---|
| `Cargo.toml` | 3,449 | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | 152,766 | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `crates/hyperai-data/Cargo.toml` | 546 | `0670ca8d428e3d8d827d9444d1de8d5e8cc86dfb4dcb306efa76ea1644143a14` |
| `crates/hyperai-data/src/lib.rs` | 178,647 | `dd67c0073d00f448d4843bcce7e3406efe7a0dff7250f158bf61bb26cdb2a1f6` |
| `crates/hyperai-data/tests/query_contract.rs` | 17,147 | `fae583193d9ad4a0e3770eb6145456740a80a5077fd6a2c1d9ee653589d92a1e` |
| `crates/hyperai-local-service/src/server.rs` | 80,499 | `de1d5b6486110b6ed1859698ecee8db7e76149933d8ad84a1f5abca99e4fb950` |
| `crates/hyperai-local-service/tests/data_api.rs` | 33,490 | `434a811cf99d57e517f9fee364df6f69abff0a7aa937c2644797d68dde8c3e02` |

The lockfile pins DataFusion `52.5.0` and `sqlparser` `0.59.0`. The workspace
disables DataFusion default features and enables only `parquet`,
`recursive_protection`, and `sql`; `sqlparser` disables default features and
enables `std` and `visitor`.

## Scope and method

Read only the seven frozen files. Trace the authenticated query request through
request-size and SQL validation, AST visitor rules, local function registry,
schema-only planning, snapshot verification/capture order, DataFusion
execution, cancellation and resource limits, result serialization, and HTTP
status/error mapping. Check that tests correspond to the reviewed code paths;
do not treat their earlier pass records as proof of this review.

Exact writable files for recording this review:

- `docs/compute-bazaar-data-current-sql-review-packet.md`
- `docs/state-slices.md`
- `docs/compute-bazaar-phase-2-data-packet.md`
- `docs/compute-bazaar-rust-port-subplan.md`

Allowed effects: inspect the frozen local source and test files; update only
the four Markdown files above with the static review, exact source hashes,
claim ceiling, and open gates. No tests or executable code are changed or run.
Do not contact or read the service on `127.0.0.1:4317`; do not read dataset
bytes, run a query, create a listener, access external sources, or retain
credentials, user data, outputs, or generated artifacts.

Non-goals: modifying the parser, function allowlist, SQL semantics, storage,
DataFusion settings, API behavior, tests, dependencies, manifests, source
adapters, fixtures, or platform configuration; making a general SQL security,
memory-isolation, independent-review, source-rights, Phase 2 acceptance, or
production claim.

## Review gates and reporting

1. Confirm all seven source hashes and byte counts before reading the frozen
   files and again before recording a result.
2. Trace each trust-boundary stage listed above and record only behavior
   evidenced by these exact bytes.
3. Note contradictions, unsupported paths, or coverage gaps without testing or
   changing them in this slice. Any fix needs its own named implementation
   slice and focused regression.
4. Record exact revision and state slice, review status, data/artifact custody,
   known limitations, claim ceiling, and the next gate.

The review has no runtime assurance label. Reviewer status is
`NOT_REQUESTED`; review reproducibility is `NOT_REPRODUCED`. Any conclusion is
limited to these source bytes and fixed test definitions. Phase 2 remains
`NOT_ASSIGNED`; native/accessibility qualification, resource isolation,
separate-host reproduction, source rights, and independent review remain
separate gates.

## Static review result (2026-10-01)

All seven frozen files matched their recorded byte lengths and SHA-256 values
before and after inspection. The review read source and fixed test definitions
only. It did not run tests, contact the service, read dataset bytes, create a
listener, access an external source, or produce a runtime artifact.

No defect or confirmed policy bypass was identified in this bounded source
review. That statement applies only to the path and exact source identity in
this packet; it is not an exhaustive SQL-safety or security conclusion.

The inspected path has these properties:

- The service binds to `127.0.0.1`. Data query POSTs use the authenticated
  local API; browser-cookie writes require CSRF and an `Origin`, while the
  agent-tool capability is limited to the fixed query route and method. The
  request boundary checks loopback host/origin, requires JSON, caps request
  bodies at 1 MiB, and bounds request handling to 30 seconds.
- The query contract caps SQL at 4 KiB and permits exactly one parsed
  `SELECT` over the registered `dataset`, with no CTE and one supported table
  factor. It rejects unsupported query/select syntax and function forms. A
  schema-only physical-plan preflight runs against the synthetic schema before
  the selected dataset file is captured.
- Dataset selection is from the fixed local catalog. Manifest metadata and
  row count are checked, source bytes are capped at 16 MiB and checked against
  the manifest digest, then one verified byte snapshot is queried through an
  in-memory object store. SQL does not select a file or network URL.
- The bounded query future covers preflight, snapshot capture, and execution
  with a two-second deadline and cancellation receiver. The service admits at
  most two active Data queries. DataFusion uses a 64 MiB memory pool, one
  partition, and batches of 1,024 rows. Results are capped at 1,000 rows,
  64 columns, and 256 KiB for the complete serialized response; limit errors
  do not return partial rows.
- The API maps rejected SQL, missing data, limits, cancellation, and internal
  failures to stable status/code envelopes. The response envelope uses a
  fixed message rather than exposing DataFusion or filesystem details.

The frozen test definitions cover representative parser allow/deny cases,
function shapes, JSON/CSV output, SQL and row limits, snapshot integrity,
Parquet parity, cancellation, timeout, serialized-size boundaries, DataFusion
pool behavior, grammar combinations, and API error mapping. Their definitions
were inspected but they were not executed in this slice.

Known limitation: the 64 MiB limit is a DataFusion memory-pool ceiling, not a
process RSS or host memory boundary. The source snapshot, planner, adapters,
serialization, and service allocations are not shown here to be completely
charged to that pool. This review does not establish hostile-query isolation,
complete allocation accounting, independent review, clean second-host
reproduction, native-platform support, external data rights, overall Phase 2
acceptance, or production readiness.

Review status is `STATIC_REVIEW_COMPLETE` for the frozen source bytes. Runtime
assurance is not assigned; reviewer status is `NOT_REQUESTED`; review
reproducibility is `NOT_REPRODUCED`. Data and artifact custody was limited to
the seven listed source, dependency, and test files. The next gates remain
separate: resource isolation, native/accessibility qualification,
second-environment reproduction, external rights, and independent review.
Overall Phase 2 remains `NOT_ASSIGNED`.
