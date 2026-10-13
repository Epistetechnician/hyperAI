# HyperAI Synthetic HyperIntel Reconciliation Adapter Packet

Status: accepted test-only local implementation slice under
`hyperai.bazaar.interface.synthetic_hyperintel_reconciliation.v1`.

Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md),
[compute-desk Rust port subplan](compute-bazaar-rust-port-subplan.md), and
[HyperIntel provenance](provenance.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve all unrelated work.

## Purpose

The HyperAI Inference workspace already accepts a caller-selected serialized
HyperIntel execution lineage, calls the pinned source-owned reconciler, and
returns an identity-only report. The product plan identifies a missing proof:
the source reconciliation package builds cases in memory, but no HyperAI test
constructs the complete serialized input consumed by the adapter.

This slice builds one completed synthetic lineage in memory from the pinned
HyperIntel V4 evidence fixture, source-owned contracts, deterministic test
keys, an in-memory replay store and journal, and a scripted local provider.
It exercises the HyperAI adapter directly. It does not add a browser fixture,
service route, or user-visible behavior; authenticated API and rendered-browser
qualification remain separate follow-up gates in the end-to-end interface plan.

## Exact source and files

The only imported source inputs are the HyperIntel test fixture and public-key
registry at the committed snapshot recorded in `docs/provenance.md`:

- `products/hyperintel/crates/hyperintel-cli/tests/fixtures/evidence-v4.json`
- `products/hyperintel/crates/hyperintel-cli/tests/fixtures/trusted-keys-v4.json`

The test reads those files from the current checkout at runtime. It does not
copy them into HyperAI, embed them in the test binary, change the imported
HyperIntel snapshot, or retain any generated report. The committed test code
defines only fixed synthetic request/output text and deterministic test-only
signing keys. All full lineage JSON exists only in memory during the test.

Files in this slice:

- `crates/hyperintel-adapter/Cargo.toml`: add only test-scoped dependencies
  needed to compose the source-owned replay and execution contracts.
- `Cargo.lock`: update only the `hyperai-hyperintel-adapter` package's
  dependency edges for those test-scoped dependencies. Preserve every existing
  package version and unrelated pre-existing lockfile change.
- `crates/hyperintel-adapter/tests/support/reconciliation.rs`: read the pinned
  source fixture and construct the signed policy, selected route, admission,
  replay consumption, metadata-only journal, scripted execution, trust JSON,
  and serialized adapter input in memory.
- `crates/hyperintel-adapter/tests/reconciliation_input.rs`: exercise one
  completed reconciliation, content-free report projection, and fail-closed
  rejection after a lineage field is changed.
- `docs/compute-bazaar-hyperintel-reconciliation-packet.md`: record scope and
  evidence.
- `docs/state-slices.md`: record the named slice, proof, custody, and claim
  ceiling.
- `docs/bazaar-hyperai-interface-plan.md` and
  `docs/compute-bazaar-rust-port-subplan.md`: update the Phase 4 progress and
  keep the service/browser gates open.

If implementation requires another file or changes a source-owned HyperIntel
crate, stop and amend this packet before proceeding.

## Allowed effects and controls

- Read the two pinned V4 fixture files above and source-owned Rust contracts
  already declared as dependencies of `hyperai-hyperintel-adapter`.
- Construct fixed synthetic request and output strings, test-only Ed25519
  keys, a replay state, execution records, and the report in process memory.
- Use `SqliteReplayStoreV1::in_memory()` and
  `SqliteExecutionJournalV1::in_memory()` only. Do not create a file-backed
  store, socket, browser, service, provider process, or network request.
- Never print or retain the serialized lineage, synthetic stream text,
  signatures, private test keys, or trust registry. Assert the output report
  and its serialized projection do not contain the synthetic stream canary or
  source execution payload fields.
- Preserve HyperIntel's pinned records, schemas, and authority. The test calls
  source-owned APIs through the existing adapter; it does not copy or modify
  their implementation.
- Use only the pinned offline Rust toolchain and existing lockfile. Do not add
  a production dependency, network source, fixture asset, or generated output.
  The one scoped lockfile update may add dependency edges only; it must not
  refresh or replace the existing package set or versions.

No provider, model, seller, registry, remote worker, user payload, paid
execution, Fleet operation, Trade action, custody operation, wallet, chain,
reviewer, publication, release, or deployment is in scope. Do not access or
modify the pre-existing service on `127.0.0.1:4317`.

## Proof gates

1. The pinned V4 market-evidence bundle verifies using the public keys in the
   pinned source fixture plus the deterministic test-only identities.
2. The scripted local execution consumes the exact authenticated replay
   transition once and produces matching journal start, terminal, and
   completed execution records.
3. `reconcile_execution_input_v1` accepts the serialized in-memory input and
   returns the source-owned completed status with `authority_granted: false`
   and `inputs_persisted: false`.
4. The report digest verifies. The report JSON contains only the documented
   identities, counts, and status; it contains neither stream text nor raw
   chunks, messages, signatures, public-key material, or arbitrary errors.
5. Changing a digest-bound execution field causes fail-closed rejection and
   does not produce a positive report. Existing adapter unit tests continue
   covering duplicate keys, malformed input, unsupported schema, invalid
   trust, and the combined-size limit.
6. The focused locked/offline adapter test, pinned formatting, strict Clippy,
   root `corepack pnpm run lint`, and relevant import/provenance checks pass.
   Report the preserved JustAI import-inventory mismatch separately.

## Claim ceiling and next gates

Maximum claim: `INTERNAL_PASS` for one source-fixture-backed, synthetic
completed execution lineage reconstructed in memory by the pinned
HyperIntel source-owned reconciliation contract through the HyperAI adapter,
plus rejection of the tested tamper. This does not prove provider activity,
independent metering, hardware identity, output correctness, source truth,
payment entitlement, settlement finality, production safety, or independent
review.

Reviewer status is `NOT_REQUESTED`; separate-environment reproduction is
`NOT_REPRODUCED` unless performed under a new exact packet. Artifact custody is
the checked-out pinned HyperIntel fixture and local test source; generated
lineage and report remain in memory and are discarded. The authenticated
loopback route, browser file-selection workflow, Activity link, cross-client
parity, accessibility, native platform support, and clean second-host
reproduction remain open. Those gates do not prevent this lower-risk adapter
contract test.

Publish boundary: local test source and documentation only. No commit, PR,
public release, external circulation, or deployment is authorized.

## Qualification evidence (2026-09-29)

On Darwin arm64 with pinned Rust `1.93.1`, the focused test command
`sh scripts/with-pinned-rust.sh cargo test -p hyperai-hyperintel-adapter --locked --offline --test reconciliation_input`
passed both integration cases: one completed synthetic lineage returned a
digest-valid identity report without the stream canary or source payload
fields, and changing the routing evaluation time was rejected. The V4 source
fixture and trust registry were read at runtime from HyperIntel revision
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`; their SHA-256 digests were
`64e2b6410a2323df608c0e83ac554a46be7bfcee2f5feb8a45ce66b36921df51` and
`7f4d4e0857d79d7793697391eb985edad0df435a8778888603eac34197e2d6ed`,
respectively. The test source is in HyperAI revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the scoped local worktree.

Pinned formatting, focused strict Clippy, and `corepack pnpm run lint` all
passed. The root lint included workspace Clippy, contract tests, full
workspace tests, and doctests. `corepack pnpm run verify:imports` verified the
2,189-file Simple Compute Market import and 99-file HyperIntel import at their
recorded revisions. It still exits nonzero for the preserved JustAI inventory
mismatch: 177 files observed versus 176 documented at revision
`9a0e56536b968cbb2cb7173e142068b53c4f9fe3`; that unrelated import was not
changed. The existing service listener on `127.0.0.1:4317` (PID 85094) was
present after validation.

Assurance is `INTERNAL_PASS` for this adapter-level synthetic lineage and the
tested routing-time tamper only. Reviewer status is `NOT_REQUESTED` and
reproducibility status is `NOT_REPRODUCED`. The generated lineage, trust JSON,
private test keys, stream canary, and reconciliation result existed only in
process memory and were discarded. The authenticated service route, browser
file-selection flow, Activity link, client parity, accessibility, native
platform coverage, independent review, and production authority remain open.
