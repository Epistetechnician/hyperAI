# Local Synthetic Market History Client Packet

State slice: `hyperai.compute_market.local_history_clients.v1`.

## Objective

Project the existing owner-only, durable Market deal-history API into the
HyperAI browser Deals surface, `hyperai market history`, and the TUI Deals
workspace. Keep the Market service contract authoritative and present the same
typed page content, digest, status, and opaque continuation cursor in each
client. This is local synthetic deal history, not a universal Activity ledger.

## Frozen baseline

- Repository: HyperAI, branch `main`, base
  `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`.
- Pre-change worktree status SHA-256:
  `3c843296a202100f9a06f4db8ab177e0ca6f9ac6585060335b33a11b27a76767`.
  This includes unrelated dirty and untracked work; preserve it.
- Frozen inputs:

| Path | Bytes | SHA-256 |
|---|---:|---|
| `docs/compute-market-refactor-plan.md` | 38910 | `b4c27f7c4aa4b2285cccfbd78f6de1a25dd68cd70b0d0347fbf11f77b3901206` |
| `apps/hyperai-interface/app.js` | 155052 | `654a5126ed216e769af0887acec72e4d064ed00311b0db4a022be6a65e940cfc` |
| `apps/hyperai-interface/app.css` | 93289 | `e9f56117a9d8350c360331f5983f21c92a04b060b4f77acea2aaafc115ba0f83` |
| `crates/outcomes-cli/src/lib.rs` | 153661 | `9367ccb2e2c1ac1d5d968e32d8e932938f97c8d82c6bee5903aed88db919354c` |
| `crates/hyperai-tui/Cargo.toml` | 529 | `df4e070e2588b487edd6428a21d8e5a39c44adf5e2b162b262e4421f8e601ac3` |
| `crates/hyperai-tui/src/lib.rs` | 140149 | `b45d17fb1a30b02ddc906986bf908b9b925c3c0d6306e936d503825e5aa97177` |
| `scripts/verify-interface-browser.mjs` | 190762 | `d2e50c8f9f44fbc1afbc1f16a0a9f40c0224ace51e7488ae3ec9317f252f5f7e` |
| `crates/outcomes-cli/tests/cli_compat.rs` | 70705 | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` |
| `Cargo.lock` | 152766 | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `docs/bazaar-hyperai-interface-plan.md` | 173202 | `0591aa0d10de9bd2d2ee8de9650d6f76476405c1dc4054139340f1a91435fd0f` |
| `docs/state-slices.md` (after adding this slice record) | 1242972 | `f498d4335f6564a10a76a1311e01de9cd73153548a1ada091ce39eaf06b9f853` |

## Exact writable files

- `apps/hyperai-interface/app.js`
- `apps/hyperai-interface/app.css`
- `crates/outcomes-cli/src/lib.rs`
- `crates/hyperai-tui/Cargo.toml`
- `crates/hyperai-tui/src/lib.rs`
- `crates/outcomes-cli/tests/cli_compat.rs` only if the existing CLI test
  additive command's JSON contract and existing entrypoints.
- `scripts/verify-interface-browser.mjs`
- `Cargo.lock` only for the direct TUI dependency on the existing
  `hyperai-market` crate.
- `docs/compute-bazaar-market-history-clients-packet.md`
- `docs/compute-market-refactor-plan.md`
- `docs/bazaar-hyperai-interface-plan.md`
- `docs/state-slices.md`

No other paths are writable in this slice. Do not modify the history store,
API schema, application facade, ACP/MCP capabilities, imported products, or
Bazaar reference checkout.

## Client contract

- Browser requests only `/api/v1/market/deal-history` through the existing
  same-origin authenticated session. It shows a clear loading state, empty
  state, error state, current page digest, newest-first records, and a
  **Load more** control only when the opaque `next_cursor` is present.
- CLI adds `hyperai market history [--limit N] [--cursor TOKEN]`. It sends the
  exact opaque token to the fixed local owner route, deserializes the typed
  `LocalMarketHistoryPageV1`, verifies the schema, bounds, order, and digest
  with `verify_local_market_history_page_v1`, then prints the verified page
  using the existing JSON output convention. Defaults and bounds match the
  service: 20 and 1–50.
- TUI adds the Market-owned history to its Deals workspace, verifies each
  typed page and digest, and supports an explicit next-page key only when the
  cursor exists. It preserves the same owner-only bearer and local service.
- Record labels distinguish **Approved locally** from **Mock completed**.
  Missing completion is unresolved local workflow state. Render the deal,
  quote, and receipt digests and the record owner. Never label these rows as
  executed jobs, correct computation, payment, or settlement.
- Page traversal forwards the opaque service cursor unchanged. Do not infer
  ordering or construct cursors in clients. Empty/error results are not
  silently presented as a complete history.
- The browser receives a page already revalidated and digested by the local
  service; the CLI and TUI additionally recompute the typed digest through the
  shared Rust verifier. All three display the server's page digest.
- MCP and ACP do not receive this endpoint or records. Cross-module Activity
  remains session-local and is not merged with this Market-owned history.

## Allowed effects and non-goals

Allowed effects are fixed synthetic requests to the existing owner-authenticated
loopback service, local rendering, and disposable browser/service/test state.
Client requests are reads only. Existing synthetic quote approval and mock
completion behavior remains unchanged.

No provider, registry, seller, worker, model, Fleet, wallet, chain, or external
service calls; no real user payloads; no new persistence; no new credentials;
no cross-module ledger; no Bazaar code, schema, fixture, data, dependency, or
asset reuse; no MCP/ACP access; and no public release or production claim.
HyperIntel, JustAI, Outcomes/HIP-4, SCM, and Bazaar keep their existing owners
and authority boundaries.

## Proof gates

- Add CLI command parsing and local service coverage for default/bounded
  limits, optional cursor forwarding, typed page verification, and malformed
  or tampered response rejection. Preserve existing CLI/API compatibility.
- Add TUI coverage for typed digest verification, explicit empty/error states,
  page cursor availability, next-page request forwarding, and no mutation.
- Extend the existing browser quote-lifecycle workflow to prove an approved
  and mock-completed synthetic record appears in Deals history, with exactly
  the expected owner route reads and no additional writes. Verify empty/error
  rendering with deterministic renderer tests if the existing test harness
  supports them without a second browser run.
- Run pinned formatting, focused CLI/TUI tests, `corepack pnpm run
  verify:interface`, the exact browser workflow, `corepack pnpm run lint:fast`,
  and `git diff --check`. Report every gate separately. Browser-runner cleanup
  failure is `INCONCLUSIVE`, not a pass.
- Recompute exact source hashes, changed paths, and the worktree status digest
  while preserving pre-existing modifications.

## Claim ceiling

At most, one tested local synthetic Market page is consistently available in
the authenticated browser, CLI, and TUI, with Rust clients recomputing the
page digest and each surface forwarding the opaque cursor contract. The fixed
quote fixture creates one row; do not claim multi-row HTTP continuation until
a separately approved deterministic multi-deal fixture exercises it. This
slice establishes no live marketplace, provider access, execution, correct
computation, payment, settlement, cross-platform qualification, independent
review, or production readiness.

Assurance result: `INTERNAL_PASS` for the exact local synthetic history
projections and checks recorded below. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`; data and artifacts stay in local HyperAI
source and disposable synthetic homes. The next blocked gates are a multi-deal
HTTP pagination fixture, wider browser/TUI platform and accessibility
qualification, and separate authorization for any external connection or
side effect. Publish boundary: local source and docs only; no commit, PR,
external circulation, or deployment.

## Execution evidence (2026-10-02)

Changed paths in this slice: `apps/hyperai-interface/app.js`,
`apps/hyperai-interface/app.css`, `crates/outcomes-cli/src/lib.rs`,
`crates/outcomes-cli/tests/cli_compat.rs`,
`crates/hyperai-tui/Cargo.toml`, `crates/hyperai-tui/src/lib.rs`,
`scripts/verify-interface-browser.mjs`, `Cargo.lock`,
`docs/compute-bazaar-market-history-clients-packet.md`,
`docs/compute-market-refactor-plan.md`,
`docs/bazaar-hyperai-interface-plan.md`, and `docs/state-slices.md`.

Proof gates passed:

- `corepack pnpm run lint:fast`: interface syntax and 34 Node tests, installer
  tests, pinned workspace formatting, and Clippy for all workspace targets and
  features.
- `cargo test -p outcomes-cli --lib market_history --locked --offline`: 2
  focused tests passed.
- `cargo test -p outcomes-cli --test cli_compat
  market_history_cli_returns_the_verified_authenticated_service_page
  --locked --offline`: 1 authenticated local-service test passed.
- `cargo test -p hyperai-tui --locked --offline`: 23 unit tests and 10 terminal
  restoration tests passed.
- `corepack pnpm run verify:interface:supply-quote-lifecycle-browser`: passed
  33 route checks and the synthetic buyer workflow. It observed one comparison
  read, one quote preview, one approval, one mock completion, three history
  reads, zero other writes, and successful disposal of the browser, service,
  listeners, and temporary state.

The browser run exercised one history row and did not exercise multi-row HTTP
cursor traversal. TUI cursor forwarding was checked with a fixed opaque token;
it does not qualify multi-row browser or service pagination. These results
qualify local synthetic client integration only. They do not qualify external
connections, execution, correctness, payment, settlement, independent review,
cross-platform accessibility, or production readiness.

Post-change SHA-256 values for the scoped implementation and evidence files
(the packet is omitted to avoid a self-referential digest):

| Path | SHA-256 |
|---|---|
| `apps/hyperai-interface/app.js` | `3c960ee6022a76a1edc50739bc22854701a35344576b32d63e819bece3093f58` |
| `apps/hyperai-interface/app.css` | `3d0702db97e4b18d60585be7c802c1cc6d6a406e338da8519aa09bd4610ed8e0` |
| `crates/outcomes-cli/src/lib.rs` | `b615e25e29f1569b8d3ed0dbd1914d6418f27a2dd75e6c85b7f9c77f083b1945` |
| `crates/outcomes-cli/tests/cli_compat.rs` | `eeb6caf4686c74a72b231b0729888fe60af33b16e4140541cb4ff363e16be2d9` |
| `crates/hyperai-tui/Cargo.toml` | `20655975279ba70010e82fb028546a41050da2155277e9230b4f5b3791608e10` |
| `crates/hyperai-tui/src/lib.rs` | `ad584548a119e75418231ea16134e92064ecd4371c8fd67bdbf91bb795b56b67` |
| `scripts/verify-interface-browser.mjs` | `c2e26a7a7dd61fe6da26f7b75ba0195068ecdc21faf8f794ac3661bb1e0a3e5a` |
| `Cargo.lock` | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `docs/compute-market-refactor-plan.md` | `1568b3530930d03ae3d604eb0d85d2c64e71069b43c22f569e1fd828db609f42` |
| `docs/bazaar-hyperai-interface-plan.md` | `07f93bd199cee23e2dd4d845ad911c00acef23c6a0ba6d6020cff564fc68d129` |
| `docs/state-slices.md` | `321041938f5e15a7b5ee6bbe9dd9c1e6b1ecb591f17af568f644d81d519c304f` |

`git diff --check` passed and the scoped files had no trailing whitespace.
The final NUL-delimited full-worktree status digest is SHA-256
`40f403a55dab64bcf4b5b85890a9fa48aa4e5494d37bd804f7f5e4dc93f0cd54`;
it includes unrelated pre-existing dirty and untracked work recorded in the
baseline.
