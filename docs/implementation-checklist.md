# HSAI × HIP-4 implementation checklist

Status date: 2026-07-11.

This repository began greenfield. There are no pre-existing HSAI modules,
migrations, tests, or Mesh compatibility surfaces to reuse or regress. The
architecture therefore follows the prompt's Rust greenfield branch.

## Phase status

- [x] Phase 0: repository baseline, workspace boundaries, official HIP-4 fact
  audit, capability matrix, references, and ADR decisions.
- [x] Phase 1: domain types, canonicalization, identifiers, schemas, transition
  tables, and golden/property tests.
- [x] Phase 2: transactional SQLite persistence, immutable events, filesystem
  CAS, idempotency, outbox, concurrency, rollback recovery, settlement restart
  recovery, and SQLite/PostgreSQL migration execution.
- [x] Phase 3: corpus/source/harness ports, admission, deterministic local
  runner, fixtures, solver-view isolation, and conformance tests. The local
  Docker candidate verifier is tested against a preloaded pinned image through
  PASS/FAIL, timeout, output flood, cleanup, evidence, finality, and mock
  reconciliation. Production hostile-code sandbox qualification remains future.
- [x] Phase 4: portable evidence, runtime Ed25519 signatures, distinct quorum,
  existential resolution, disputes, pauses/finality guards, and composed local
  TRUE/FALSE/ERROR scenarios.
- [x] Phase 5 local surface: machine-readable CLI, public/operator API auth,
  OpenAPI route contract, payload limit, granular mock settlement preflight and
  workflow, artifact-driven settlement CLI, fixed-cardinality operational
  snapshots, bounded unlabeled OpenMetrics rendering, and durable-job state
  semantics. Production identity and metrics transport remain future.
- [x] Phase 6 profile/conformance layer: immutable pins, generic-Git overlays,
  rights/readiness records, recorded/manual source boundaries, and pinned NYU
  harness metadata, plus deterministic materialization of regular files from an
  already-pinned local checkout and inert special-object inspection. Clone/ref,
  submodule/LFS retrieval, archive extraction, and image materialization remain
  future work.
- [x] Phase 7: HIP-4 read-only metadata, network-scoped identity, binding, and
  stale-metadata detection.
- [x] Phase 8: mock settlement, deterministic intents, manual handoff,
  reconciliation, crash recovery, and mainnet refusal.
- [x] Phase 9: threat model, runbooks, clean demo, complete gates, and handoff.

## Explicit external boundary

Official sources document read-only active and settled outcome metadata plus
user inventory actions. They do not document a public market-creation action,
resolution-publication action, authorized resolution signer role, or an HSAI
authorization path. Testnet resolution publishing is therefore
`UNSUPPORTED_UNVERIFIED`; mainnet mutation is structurally out of scope.
