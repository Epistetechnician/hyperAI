# Local Synthetic Market Deal-History Packet

State slice: `hyperai.compute_market.local_deal_history.v1`.

## Objective

Give HyperAI's local synthetic market one durable owner-controlled deal-history
record that can later be read consistently by its browser, CLI, TUI, and agent
interfaces. This slice establishes the service and storage contract first. It
does not implement those client projections.

## Frozen baseline

- Repository: HyperAI, branch `main`, base
  `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`.
- Pre-existing working-tree status digest before this packet was written:
  `a5aa1907d3b1d6a95ec2ce1d58418b36abf312e8bdc8c28c37908b54638a7441`
  (`sha256` of `git status --porcelain=v1 -z`). This includes unrelated dirty
  and untracked work; preserve it.
- Frozen implementation inputs (SHA-256):

| Path | SHA-256 |
|---|---|
| `crates/hyperai-market/src/lib.rs` | `16b07ec36107d29182020abecd23b8cd23862e3c3203914790877409a65a44f8` |
| `crates/hyperai-application/src/lib.rs` | `dcb505ad95e25352434a2959d6143f00030766ff9c095c1c305c27e25e3b8c05` |
| `crates/hyperai-local-service/src/lib.rs` | `806704fc3862d568af434ab54c428119b19dadb3ccfa9bce74765efa925d3180` |
| `crates/hyperai-local-service/src/storage.rs` | `449e748f5d28043f495130c749968d121cf34e98b137e790b31d97817cda82c6` |
| `crates/hyperai-local-service/src/server.rs` | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `crates/hyperai-local-service/tests/api_contract.rs` | `edf36bc61d69bdcdecb82925da01adc27b994f1b04192ec316f89aa9ed178fb8` |
| `docs/compute-market-refactor-plan.md` | `be38763a52d13351ef106e878063c266c52cf6ca1a589f3f3146c5b97d4d1cf8` |

## Included files

- `crates/hyperai-market/src/lib.rs`: typed market history page and entry
  contracts, bounded fields, stable cursor and page digest.
- `crates/hyperai-local-service/src/lib.rs` and new
  `crates/hyperai-local-service/src/market_history.rs`: dedicated store,
  idempotent persistence, conflict detection, and read validation.
- `crates/hyperai-local-service/src/storage.rs`: market-owned storage path and
  secure local directory/file handling. Keep the metadata database schema,
  backup behavior, and ownership unchanged.
- `crates/hyperai-local-service/src/server.rs`: persist approvals and mock
  completions; add owner/session-only paged read route; keep it out of ACP tool
  capabilities. Revalidate persisted data by recomputing it through the
  existing deterministic application facade.
- `crates/hyperai-local-service/tests/api_contract.rs`: service contract,
  authorization, replay, pagination, and restart-persistence qualification.
- `docs/compute-market-refactor-plan.md`, this packet, and
  `docs/state-slices.md`: update product contract and evidence.

## Storage and API contract

- Store only deterministic synthetic market approvals and their corresponding
  mock completion receipts in `<hyperAI-home>/market/deal-history.sqlite3`.
  This is Market-owned data, separate from `<hyperAI-home>/metadata/desk.sqlite3`.
- Use a versioned SQLite schema, one local connection, WAL and full synchronous
  commits. Reject symbolic-link paths, non-regular database files, unknown
  non-empty schemas, newer schema versions, malformed rows, and oversized
  serialized records. Restrict the directory to owner-only access where the
  platform supports Unix modes. Do not claim power-loss or hostile-filesystem
  qualification.
- Approval writes are idempotent for the same deal and exact approval bytes.
  Conflicting reuse of a deal identity returns a stable conflict. Completion
  requires the exact persisted approval and is likewise idempotent only for an
  identical receipt. Missing approval returns HTTP 409
  `MARKET_DEAL_NOT_APPROVED`.
- Add `GET /api/v1/market/deal-history` for the authenticated local owner, with
  a default page size of 20, a maximum of 50, stable newest-first ordering, an
  opaque continuation cursor, and a digest over the returned page. Reject
  malformed cursors and out-of-range limits. Do not grant this route to ACP
  bearer capabilities or remote callers.
- Revalidate typed approvals, receipt signatures/digests, deal linkage, row
  bounds, and page digest before returning any page. Fail the whole page closed
  on corruption; never return a partial history as complete.
- Do not automatically prune history. Retain it under the local HyperAI home
  until the owner removes that home. The existing metadata backup does not
  include this market database; backup/export integration is explicitly later.

## Allowed effects and non-goals

Allowed effects are local SQLite reads/writes for fixed synthetic mock-market
records, temporary test databases, and local service requests. No real user
payloads, raw prompts/traces, private inputs/outputs, provider receipts, keys,
credentials, or external data are stored.

This slice does not read Bazaar or registries, contact sellers/providers,
fulfill real jobs, enable paid execution, custody funds, settle, publish,
federate, write to a chain, alter Outcomes/HIP-4, project into HyperIntel or
JustAI, or expose the history through browser/CLI/TUI/MCP/ACP clients. Bazaar
remains a design reference; no Bazaar code, schemas, fixtures, data, or assets
are copied. SCM, HyperIntel, and JustAI remain separate authorities and build
graphs.

## Proof gates

- Format and compile the changed Rust packages.
- Focused store/API coverage proves reopen persistence; exact retry; conflicting
  retry rejection; completion gating; receipt/link verification; stable bounded
  store pagination and page digest; authorization denial for unauthenticated
  and scoped ACP callers; owner/session read access; corruption and
  unsupported-schema fail-closed behavior. The fixed local quote fixture
  produces one deal, so HTTP continuation across multiple real history rows
  remains unexercised until an authorized synthetic multi-deal fixture exists.
- Run the root `lint:fast` gate and the owning service's strict selected-package
  validation. Report each result separately. Preserve the outputs needed to
  reproduce these checks in this packet.
- `git diff --check` passes. Recompute source hashes and worktree status digest
  at completion; report the exact changed paths and assurance level.

## Implementation evidence (2026-10-02)

- `sh scripts/with-pinned-rust.sh cargo check -p hyperai-local-service` passed.
- `sh scripts/with-pinned-rust.sh cargo test -p hyperai-local-service` passed:
  19 unit tests and 36 integration tests passed; one browser integration test
  was ignored because it requires a built UI binary and local Chrome/Chromium.
  After the final error-mapping cleanup, the five Market-history unit tests and
  the owner-only/restart API lifecycle test passed again.
- `corepack pnpm run lint:fast` passed after the final Rust changes: interface
  checks (34 tests), installer checks (3 tests), pinned formatting, and strict
  all-target/all-feature workspace Clippy.
- All test records and persisted fixtures were deterministic synthetic data in
  temporary local homes. Only local loopback HTTP was used for the API test;
  no external network, provider, wallet, funds, publication, chain, HyperIntel,
  JustAI, or Bazaar source/data operation occurred.
- `git diff --check` passed and no trailing whitespace was found in the scoped
  files. Final NUL-delimited worktree status digest:
  `3c843296a202100f9a06f4db8ab177e0ca6f9ac6585060335b33a11b27a76767`.
- Final SHA-256 values for the implementation inputs and evidence documents:

| Path | Final SHA-256 |
|---|---|
| `crates/hyperai-market/src/lib.rs` | `71a236039ad80068b905f8efea2e6d9c5322cea5c0b0492e709993a0247357cc` |
| `crates/hyperai-application/src/lib.rs` (unchanged) | `dcb505ad95e25352434a2959d6143f00030766ff9c095c1c305c27e25e3b8c05` |
| `crates/hyperai-local-service/src/lib.rs` | `7797d9ae1702be94c487fbdf08103cf8c94c2fe9aa771df3dca37e6b7823941e` |
| `crates/hyperai-local-service/src/storage.rs` | `b15d08c5f514c9a6b0d338142b155b5cb314c8cd6d4f205eb7f2a3188f7919e5` |
| `crates/hyperai-local-service/src/server.rs` | `f8430e03d63f77401ac0325b3ceb571d482428bf6c76a7cafabd729381a39e07` |
| `crates/hyperai-local-service/src/market_history.rs` | `bfa5080e70927f4ad16a1456549fd782ff25ba85f1d73c539505e07f963e37ba` |
| `crates/hyperai-local-service/tests/api_contract.rs` | `8b6ec03f789b9becfe7e4b0807c0d77bb81a6ed6280a88d48435999ab560bac8` |
| `docs/compute-market-refactor-plan.md` | `b4c27f7c4aa4b2285cccfbd78f6de1a25dd68cd70b0d0347fbf11f77b3901206` |
| `docs/state-slices.md` | `b210f4151b9fde704b80faaed989bd8a8c4c12e92d48e32c4050a7843e24ebe4` |

## Claim ceiling and gates

Claim ceiling: `INTERNAL_PASS` can establish only a local synthetic, single-host
service/storage contract for the tested platform and cases. It does not prove
independent review, cross-platform filesystem behavior, backup/recovery,
multi-process concurrency, live market connectivity, correct compute, payment
entitlement, escrow safety, or production readiness.

Reviewer status: `NOT_REQUESTED`. Reproducibility status: `NOT_REPRODUCED`
unless a clean second environment is explicitly recorded. Data custody is the
owner's local HyperAI home plus temporary synthetic test homes. The next
blocked authority gates are separately named client projection/backup work,
rights-cleared Bazaar data/source use, and any live, paid, external, chain, or
production activity.
