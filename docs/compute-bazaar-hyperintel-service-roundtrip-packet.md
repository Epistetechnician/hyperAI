# HyperAI Synthetic HyperIntel Service Round-Trip Packet

Status: accepted test-only local service qualification under
`hyperai.bazaar.interface.synthetic_hyperintel_service_roundtrip.v1`.

Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md),
[Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md),
[Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md), and
[adapter reconciliation packet](compute-bazaar-hyperintel-reconciliation-packet.md).
Source identity and fixture custody remain in [provenance](provenance.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve all unrelated work.

## Purpose

The versioned reconciliation route, session authentication, and Inference
file-selection UI already exist. The prior adapter slice proved serialized
source-owned reconciliation directly, but did not exercise the input through
the actual authenticated service. This slice adds one local integration test
for that boundary. It does not change production routes, schemas, UI behavior,
or persistence.

The test reuses the existing root-owned synthetic input builder at
`crates/hyperintel-adapter/tests/support/reconciliation.rs` by including that
test-support module at compile time. The builder reads only the two pinned
HyperIntel fixture files at runtime, creates the complete lineage in memory,
and uses in-memory replay and journal stores. The service runs on an ephemeral
IPv4 loopback port with a disposable HyperAI home. No serialized lineage is
written to disk.

## Exact files

- `crates/hyperai-local-service/Cargo.toml`: add only test-scoped dependencies
  required by the shared synthetic builder and result digest check.
- `Cargo.lock`: update only `hyperai-local-service` test dependency edges;
  preserve all package versions and unrelated pre-existing lockfile changes.
- `crates/hyperai-local-service/tests/hyperintel_reconciliation_api.rs`: start
  the local service on port 0, verify unauthenticated denial, submit the
  synthetic request with the same-origin session cookie and CSRF token, and
  assert the bounded response and absence of persisted canary text.
- `crates/hyperintel-adapter/tests/support/reconciliation.rs`: read-only reuse
  of the existing synthetic input builder; do not modify this file in this
  slice.
- `docs/compute-bazaar-hyperintel-service-roundtrip-packet.md` and
  `docs/state-slices.md`: scope and qualification evidence.
- `docs/bazaar-hyperai-interface-plan.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: current route-test status and
  remaining browser gate.

If test support needs another source file or any production behavior must
change, stop and amend this packet before proceeding.

## Allowed effects and controls

- Read the two explicitly pinned HyperIntel fixture files through the existing
  test support. Preserve the committed HyperIntel snapshot and its owner
  contracts.
- Construct one deterministic synthetic execution lineage and trust registry
  in process memory. Never log or retain the serialized request, stream canary,
  signatures, private test keys, or full trust registry.
- Start `hyperai-local-service` only inside the test on IPv4 loopback with an
  OS-assigned port (`0`). Use a disposable temporary HyperAI home and remove it
  when the test ends. Do not connect to or change the existing service at
  `127.0.0.1:4317`.
- Exercise the route's normal same-origin session cookie and CSRF check. Also
  confirm the request is rejected without authentication.
- Inspect only the disposable home for the unique synthetic stream canary to
  verify that the route does not persist its input. No browser, provider,
  registry, network peer, model, seller, paid execution, Fleet operation,
  Trade action, custody, wallet, chain, reviewer, or external publication is
  in scope.
- Use the pinned offline Rust toolchain and existing lockfile. The scoped
  lockfile change may add dependency edges only; it must not refresh package
  versions or the existing package set.

## Proof gates

1. The reused builder verifies the pinned V4 source fixture and returns a
   request below the adapter's 150 KB combined-input limit.
2. An unauthenticated POST to `/api/v1/inference/reconcile-execution` returns
   the existing `UNAUTHENTICATED` error contract.
3. The same request, submitted with the bootstrapped same-origin cookie and
   CSRF token, returns HTTP 200 and the source-owned completed report through
   the actual service route.
4. The response report digest recomputes; authority and persistence fields
   remain false; the stream canary and raw execution fields are absent.
5. No file in the disposable service home contains the stream canary. The
   service stops, the temporary home is removed, and the pre-existing listener
   on `127.0.0.1:4317` remains untouched.
6. The locked/offline integration test, pinned formatting, strict Clippy,
   root `corepack pnpm run lint`, and pinned import/provenance check complete.
   Document the preserved JustAI import-inventory mismatch separately.

## Qualification result (2026-09-29)

The focused locked/offline authenticated-route test passed (1 passed),
including unauthenticated denial, same-origin cookie and CSRF admission,
completed report digest verification, content-free output, and disposable-home
canary scan. Pinned formatting, strict Clippy, contract tests, and full root
lint passed. The import verifier separately passed Simple Compute Market and
HyperIntel; JustAI remains at 177 observed files versus 176 recorded. The
service and its temporary home were removed; the existing listener on
`127.0.0.1:4317` remained outside the test.

Assurance: `INTERNAL_PASS` for this exact local authenticated service path.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.

## Claim ceiling and next gate

Maximum claim: `INTERNAL_PASS` for the exact local same-origin authenticated
service round trip and synthetic content-free response. This does not qualify
the rendered browser's file-selection interaction, Activity navigation,
accessibility, other platforms, source truth, provider activity, correct
computation, independent metering, payment entitlement, settlement, or
production readiness.

Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The
synthetic lineage and report remain in memory; the checked-out pinned
HyperIntel fixture and source test helper remain the only source artifacts.
The populated browser flow is qualified separately under
[`hyperai.bazaar.interface.synthetic_hyperintel_browser_roundtrip.v1`](compute-bazaar-hyperintel-browser-roundtrip-packet.md).
It remains outside this service slice's claim ceiling. Publish boundary: local
test source and documentation only; no commit, PR, external circulation,
release, or deployment is authorized.
