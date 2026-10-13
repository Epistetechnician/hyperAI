# HyperAI Workload Trace Owner-Backed Data Read Packet

Status: authorized local implementation slice following the accepted Compute
Bazaar Rust port subplan. This packet projects the result of HyperAI Data's
existing read-only catalog verification into the existing Workload Trace.

State slice: `hyperai.bazaar.workload_trace_data_read_served.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Data owner: [`hyperai-data`](../crates/hyperai-data/src/lib.rs).
Trace owner: [`hyperai-application`](../crates/hyperai-application/src/lib.rs).
Baseline: `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`, plus the
existing dirty and untracked worktree. Preserve unrelated work.

## Frozen contract

The existing owner-authenticated loopback route remains
`GET /api/v1/market/workload-trace`; its path, method, authorization policy,
WorkloadTraceV1 wire schema, and ACP capability do not change. When the route
has no Data root, report `not_requested` and perform no Data read. When the
route has a Data root, call the Data owner once to inspect the fixed HyperAI
synthetic JSON dataset and return only a bounded status and optional verified
content digest:

- `not_seeded`: the root, manifest directory, or requested dataset is absent;
- `bound`: the Data owner verified the manifest and exact dataset bytes; bind
  their existing content digest into the workload intent and trace;
- `missing`: a catalog/manifest read reached a referenced file that is absent;
- `conflicting`: the catalog, manifest, path, or content-integrity contract is
  rejected;
- `denied`: the local operating system returns `PermissionDenied`;
- `unavailable`: another bounded local read or verification failure prevents
  classification.

Only `bound` creates a buyer-intent Data reference and includes a digest. Every
unresolved status leaves the buyer intent's Data references empty while
retaining one owner-labelled Data trace reference with no digest. Statuses and
link bases distinguish observed local filesystem/catalog outcomes; no raw
error, path, permission metadata, or file content is exposed. The existing
fixed `missing`, `conflicting`, and `denied` preview route remains a separate
no-read fixture and keeps its current claim ceiling.

The Data catalog continues to read only local HyperAI-authored synthetic
files. It performs no provider access, query, write, or network request. A
verified digest establishes only that the bytes match the local manifest; it
does not establish market truth, source freshness, computation, fulfillment,
or payment entitlement.

## Exact files and allowed effects

- `crates/hyperai-data/src/lib.rs`: add an internal-facing typed status result
  over the existing verified catalog path and distinguish absent paths from
  permission/other metadata errors without changing manifest/catalog wire
  schemas.
- `crates/hyperai-data/tests/catalog_contract.rs`: cover each status, digest
  presence, digest absence, and bounded error mapping with temporary
  HyperAI-authored fixtures.
- `crates/hyperai-application/src/lib.rs`: consume the Data owner's single
  verified result; preserve `not_requested`; project the status, link basis,
  and claim ceiling without adding unresolved Data references to the intent.
- `crates/hyperai-local-service/tests/agent_api.rs`: verify the existing route
  serves the bounded projection with owner authentication and still rejects
  unauthenticated access and the scoped ACP capability.
- `crates/hyperai-mcp/src/lib.rs`: update the existing end-to-end contract test
  so the MCP projection confirms the trace's digest-free `conflicting` status
  is forwarded as data rather than normalized into a service error; the MCP
  route, allowlist, and capability remain unchanged.
- This packet, `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-workload-trace-packet.md`,
  `docs/bazaar-hyperai-interface-plan.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and `docs/state-slices.md`:
  record the exact boundary and evidence.

Allowed effects: read and verify only the fixed local synthetic Data catalog
and snapshot; return its bounded outcome on the already authenticated
loopback route; create, seed, tamper with, delete, or permission-limit fixtures
only inside a disposable local test/service home; inspect the existing
Workload Trace browser view; and run the listed local validation gates.

Stop if this work needs Bazaar code/data, an external source, a provider or
seller call, federation, paid execution, a model, remote Fleet, custody,
wallet/chain activity, reviewer submission, external publication, testnet
write, or deployment. Do not change settlement, outcome resolution, HIP-4, or
Data catalog/manifest wire schemas.

## Proof gates and claim ceiling

- Data contract tests cover `not_seeded`, `bound`, `missing`, `conflicting`,
  `denied`, and `unavailable`; only `bound` has a digest.
- Application tests validate the full trace digest, status-specific link
  basis/claim ceiling, no Data intent reference for unresolved statuses, and
  exact intent/trace digest pairing for `bound`.
- The route test verifies owner access, unauthenticated denial, unchanged
  response schema/route, and scoped ACP denial.
- The existing MCP bridge test verifies the same bounded conflicting trace is
  returned without raw source details or a stale error expectation.
- A served local browser walk-through verifies valid seeded data, unseeded
  data, missing backing bytes, conflicting bytes, permission denial where the
  host permits a deterministic permission test, and recovery to a valid
  trace without stale status content. Check narrow and desktop layout and the
  browser error log; no browser files should change for this slice.
- Run focused Rust tests, `corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, the offline `hyperai` binary build,
  and `git diff --check`. Stop the disposable service and remove its home;
  preserve the user-owned service on port 4317.

Maximum claim: the existing local Workload Trace can project the HyperAI Data
owner's verified status for one fixed synthetic dataset and bind the exact
manifest-verified byte digest when available. It establishes no external
availability, freshness, market truth, compute execution, correct
computation, seller identity, payment entitlement, independent review, full
accessibility, cross-browser behavior, or production readiness. Assurance is
`INTERNAL_PASS` only for the exact local checks completed; reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED` unless a clean second
environment is used. Custody is limited to checked-out HyperAI source and a
removed disposable synthetic home. Publish boundary: local source and docs;
no commit, PR, release, external circulation, or deployment.
