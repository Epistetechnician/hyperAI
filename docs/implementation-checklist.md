# HSAI × HIP-4 implementation checklist

Status date: 2026-07-10.

This repository began greenfield. There are no pre-existing HSAI modules,
migrations, tests, or Mesh compatibility surfaces to reuse or regress. The
architecture therefore follows the prompt's Rust greenfield branch.

## Phase status

- [x] Phase 0: repository baseline, workspace boundaries, official HIP-4 fact
  audit, capability matrix, references, and ADR decisions.
- [x] Phase 1: domain types, canonicalization, identifiers, schemas, transition
  tables, and golden/property tests.
- [x] Phase 2: transactional SQLite persistence, immutable events, filesystem
  CAS, idempotency, outbox, concurrency, and rollback recovery. PostgreSQL
  migration execution remains blocked by the unavailable local server/daemon.
- [x] Phase 3: corpus/source/harness ports, admission, deterministic local
  runner, fixtures, solver-view isolation, and conformance tests. Live OCI
  execution remains blocked by the unavailable Docker daemon.
- [x] Phase 4: portable evidence, runtime Ed25519 signatures, distinct quorum,
  existential resolution, disputes, pauses/finality guards, and composed local
  TRUE/FALSE/ERROR scenarios.
- [ ] Phase 5: API, CLI, workers, auth, OpenAPI, and observability.
- [ ] Phase 6: recorded adapters, overlays, rights/provenance records, and NYU
  recorded harness path.
- [ ] Phase 7: HIP-4 read-only metadata, network-scoped identity, binding, and
  stale-metadata detection.
- [ ] Phase 8: mock settlement, deterministic intents, manual handoff,
  reconciliation, crash recovery, and mainnet refusal.
- [ ] Phase 9: threat model, runbooks, clean demo, complete gates, and handoff.

## Explicit external boundary

Official sources document read-only active and settled outcome metadata plus
user inventory actions. They do not document a public market-creation action,
resolution-publication action, authorized resolution signer role, or an HSAI
authorization path. Testnet resolution publishing is therefore
`UNSUPPORTED_UNVERIFIED`; mainnet mutation is structurally out of scope.
