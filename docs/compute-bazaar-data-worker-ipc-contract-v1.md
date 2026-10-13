# Data Query Worker IPC Contract v1 (design)

State slice: `hyperai.bazaar.data_worker_ipc_contract_design.v1`.
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Implementation sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Parent architecture: [process isolation design gate](compute-bazaar-phase-2-data-packet.md#process-isolation-design-gate).

Status: planning-only contract. This packet specifies a candidate internal
request/result boundary for a future supervised Data query worker. It does
not authorize runtime changes or qualify an operating-system memory limit.

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the preserved dirty and untracked working tree. The NUL-delimited Git status
digest at freeze was
`57a9472033ae0a7fdf72dd69db9bc78bd2ef340e438699f6a3618c3b27903d13`.

## Frozen basis

The following working-tree bytes were read as design inputs. The revision
alone does not identify these dirty bytes.

| File | Bytes | SHA-256 |
|---|---:|---|
| `crates/hyperai-data/src/lib.rs` | 178,647 | `dd67c0073d00f448d4843bcce7e3406efe7a0dff7250f158bf61bb26cdb2a1f6` |
| `crates/hyperai-local-service/src/server.rs` | 80,971 | `b9cc9e8decb2a341ae4dab63a5ef3a0b1a8ac6714bbb1dc32355c590640cfca1` |
| `docs/compute-bazaar-data-current-sql-review-packet.md` | 8,496 | `e8d3ff6c0f4891d1e011fed7a7f2318bd6aaf79d4b3c48ee2e2b1f002256c4e7` |
| `docs/compute-bazaar-data-macos-isolation-options-v1.md` | 6,228 | `2f5e175d902b0668dca5b4800f6a3d87f4a290875b4c343a583fc7c40b0158ac` |
| `docs/compute-bazaar-data-docker-cgroup-probe-v1.md` | 8,439 | `ee4b44f761ac154dafd8bccf82a8349ea226ed20f93af801c768029deb533cce` |
| `docs/state-slices.md` | 1,092,111 | `22c5574d40dee91dc0a3d686003b3d6528fd9b6d8eb207d3e60c1c3753e0b83c` |
| `docs/compute-bazaar-phase-2-data-packet.md` | 173,071 | `0df92c6d1b8d40f6247277585208d332d018fefe22f8d37da6dff30cc0192d4c` |
| `docs/compute-bazaar-rust-port-subplan.md` | 215,493 | `d333d142075a08e693df6e2f590dd67bc4c16322467c1d5269a8aa36cf016e4c` |

Exact writable files are this packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-2-data-packet.md`, and
`docs/compute-bazaar-rust-port-subplan.md`. Source files and all imported or
reference workspaces are read-only.

## Purpose and compatibility boundary

The current service owns authentication, request authorization, the
`DataQueryEnvelopeV1` HTTP request (`query_id` plus `QueryRequestV1`), query
cancellation, and the `hyperai.interface.error.v1` HTTP error envelope.
`hyperai-data` owns the read-only SQL subset and returns
`hyperai.data.query-result.v1`. Its current
limits are 4 KiB SQL, 16 MiB source bytes, 1,000 rows, 64 columns, 256 KiB
serialized result, a 64 MiB DataFusion pool, one partition, and a two-second
query deadline. The local service has two shared Data admission slots.

The worker boundary is internal. `/api/v1/data/query`, CLI/TUI/browser/ACP
callers, response JSON, stable public error codes, saved models, and saved
views retain their current contracts. This packet adds no route, public field,
error code, or persistence format. The service stays the sole query
authority; clients do not launch or address workers directly.

The candidate is one fresh worker attempt per accepted query. No worker is
reused and no automatic retry occurs. The existing two-slot admission limit
applies to the whole query lifecycle, including snapshot transfer, worker
startup, execution, result validation, termination, and reaping. Worker
startup and IPC time count against the existing two-second query budget.

## Wire framing candidate

The logical framing is transport-neutral; this packet does not select pipes,
Unix sockets, Windows handles, a VM device, or another platform transport.
Every implementation must provide one private request stream and one private
response stream bound to the same one-shot worker instance.

Each frame starts with exactly 12 bytes:

| Offset | Width | Meaning |
|---:|---:|---|
| 0 | 4 | ASCII magic `HAQW` |
| 4 | 2 | Unsigned big-endian protocol version; `1` for this contract |
| 6 | 2 | Unsigned big-endian kind: `1` request, `2` success, `3` typed failure |
| 8 | 4 | Unsigned big-endian JSON metadata/body byte length |

The request metadata JSON is UTF-8, at most 32,768 bytes, has no duplicate or
unknown fields, and contains exactly:

| Field | Contract |
|---|---|
| `schema_version` | `hyperai.data.worker.request.v1` |
| `attempt_id` | Fresh 32-character lowercase hexadecimal value; not the caller's query ID |
| `dataset_id` | A registered dataset ID already authorized by the parent |
| `source_format` | `json` or `parquet`, selected from the verified manifest; current JSON fixture bytes are newline-delimited |
| `snapshot_bytes` | Integer from 1 through 16,777,216 |
| `snapshot_sha256` | Lowercase SHA-256 of the following exact snapshot bytes |
| `schema_sha256` | Digest of the schema identity already checked by the parent |
| `row_count` | Verified manifest row count for the snapshot |
| `sql` | One validated read-only `SELECT`, UTF-8 length at most 4,096 bytes |
| `output_format` | `json` or `csv` |
| `max_rows` | Integer from 1 through 1,000, already lowered to the caller's request |
| `max_columns` | Exactly 64 for this version |
| `max_result_bytes` | Exactly 262,144 for this version |
| `remaining_budget_ms` | Integer from 1 through 2,000, measured by the parent immediately before transfer |

Immediately after the request metadata, the parent writes exactly
`snapshot_bytes` raw bytes. The parent then closes the request stream. The
worker verifies the length and SHA-256 before parsing the snapshot, checks
that the bytes match the declared source format and verified schema/row-count
identity, and requires end-of-stream after the snapshot. The request contains
no filesystem path, home directory, credential, network location, or mutable
catalog handle. It carries only one immutable authorized snapshot and one
query.

A success frame contains UTF-8 JSON serialization of the existing
`QueryResultV1`, with its current `hyperai.data.query-result.v1` schema
identifier and no additional envelope. Its body may not exceed 262,144 bytes.
The parent validates the result schema, dataset and snapshot identity,
column/row counts, result-size accounting, `claim_ceiling`, and
`effect_boundary` against the request and current Data contract before
returning it to the API. The 12-byte internal frame header is not counted in
the existing 262,144-byte public result-body limit.

A typed-failure frame contains one strict JSON object, at most 4,096 bytes:

```json
{
  "schema_version": "hyperai.data.worker.failure.v1",
  "attempt_id": "0123456789abcdef0123456789abcdef",
  "code": "query_rejected"
}
```

`code` is one of `query_rejected`, `limit_exceeded`, `integrity_mismatch`,
`dataset_not_found`, `query_failed`, or `invalid_worker_request`. The body has
no free-form message, SQL echo, filesystem path, stack trace, or raw output.
The attempt ID must match the request. No other frame kind or protocol version
is accepted. The worker writes exactly one response frame, closes the
response stream, and exits; any trailing byte, truncated frame, duplicate
response, malformed JSON, oversized body, mismatched attempt ID, or
non-success exit after a frame invalidates the entire response.

## Authority, lifecycle, and failure rules

1. The service authenticates the caller, applies the existing admission slot,
   validates the query ID/request, validates SQL, and completes the current
   schema-only plan preflight before reading dataset bytes.
2. The parent resolves the registered dataset, confines its path, validates
   the manifest and row count, reads one bounded snapshot, and verifies its
   digest. It starts the existing two-second monotonic deadline before query
   work; the transferred budget is only the remaining duration, never a new
   two-second allowance.
3. The parent launches one worker with only the required protocol streams and
   a sanitized environment. A future platform adapter must close unrelated
   handles, exclude credentials and home access, prevent network access, and
   place the worker under its qualified resource boundary. Until those
   controls are proven on a platform, the worker mode stays unavailable there.
4. The worker independently checks framing, request bounds, snapshot digest,
   source format, and SQL policy before query execution. It retains the
   current DataFusion pool, one-partition, row, column, and output limits.
   It builds and bounds the complete result before writing its success frame;
   it never streams partial rows as success.
5. The parent reads no more than the declared response ceiling, validates the
   complete frame and result, observes worker exit, and only then returns the
   unchanged public response. Stderr is drained under a 4,096-byte cap,
   discarded, and never returned or persisted.
6. Cancellation or timeout is parent-owned. The supervisor terminates the
   whole worker process tree or guest task, waits for termination, closes
   streams, releases the admission slot, and returns no partial result. A
   worker crash, forced kill, OOM, malformed response, unknown exit, or
   unproven resource-limit event remains a generic failed operation. Report a
   memory-limit cause only when the selected platform boundary supplies
   evidence for that cause.

The parent maps typed worker errors to the existing stable API semantics:
`query_rejected` to `DATA_QUERY_REJECTED`, `limit_exceeded` to
`DATA_LIMIT_EXCEEDED`, `integrity_mismatch` to `CONFLICT`, and
`dataset_not_found` to `NOT_FOUND`. `query_failed` and
`invalid_worker_request` use the existing generic Data operation failure.
Parent cancellation retains `DATA_QUERY_CANCELLED`; a parent-owned timeout uses
the existing generic Data operation failure. There is no new timeout or
worker-specific public error code in this contract; changing those semantics
requires a separate API state slice.

The protocol does not by itself sandbox a child. A child with inherited
credentials, filesystem handles, environment secrets, network access, or
unbounded descendants is outside this contract. The operating-system
containment adapter must demonstrate worker identity, process-tree ownership,
no-network/no-home controls, memory enforcement, and cleanup before the
platform can enable this execution path.

## Future implementation proof gates

A separate implementation slice must freeze its exact service/API identity,
worker binary identity, transport adapter, supported platforms, source
snapshot and retention behavior, resource budgets, and claim ceiling. Its
focused proof must cover:

- exact and malformed framing; duplicate/unknown fields; truncated, extra,
  and oversized input/output; invalid UTF-8; digest/schema/row-count mismatch;
- JSON and CSV parity with the current `QueryResultV1`, including exact
  output-byte boundaries and no partial result;
- the existing supported/rejected SQL corpus through the same authenticated
  service route and stable error envelope;
- cancellation before spawn, during snapshot transfer, during execution,
  while blocked on output, and after a result frame; timeout across startup
  and execution; worker crash, forced kill, descendant termination, and
  parent shutdown;
- two-slot concurrency, bounded stderr, temporary resource cleanup, and
  combined service-plus-worker memory/capacity under the selected platform
  boundary; and
- browser, CLI, TUI, and ACP parity through the shared service, with no client
  direct worker path.

Linux cgroup v2, Windows Job Objects, and a macOS Linux guest remain candidate
platform mechanisms only. The Docker Desktop cgroup probe proves its exact
Linux VM container boundary, not a native host or HyperAI worker. The prior
macOS Virtualization start attempts ended `INCONCLUSIVE`; this packet does not
authorize another VM start, select a new macOS mechanism, or establish macOS
support. The future implementation slice must fail closed on every platform
whose resource and lifecycle controls are not qualified.

## Assurance and publish boundary

This is `PLANNING_ONLY`; no runtime assurance is assigned. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_APPLICABLE`. Custody is limited to
the frozen local source and documentation listed above. No test, executable,
service, dataset, imported workspace, provider, network endpoint, VM, or
external system was contacted for this packet. Bazaar source/data rights,
native worker isolation, broader Phase 2 acceptance, independent review, and
production support remain open. Overall Phase 2 remains `NOT_ASSIGNED`.

Publish boundary: local documentation only; no commit, PR, release,
deployment, API exposure, or production operation.
