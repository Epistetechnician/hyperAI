# Buyer supply comparison packet

State slice: `hyperai.bazaar.buyer_supply_comparison.v1`\
Status: bounded local implementation completed; external and paid gates remain closed.\
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with pre-existing dirty and untracked work preserved.\
Claim ceiling: deterministic projection of fixed local synthetic market records only.

## Purpose

Implement the next buyer-facing slice in the HyperAI interface plan: compare a
fixed buyer request with source-attributed synthetic offers and show advisory
price/availability observations alongside the offers without letting those
observations affect eligibility or selection. The browser, authenticated local
HTTP API, CLI, TUI, and a new append-only MCP V5 read tool consume the same
versioned projection. The existing offline Simple Compute Market capture
explorer remains a separate source-owned view and is never joined to this
synthetic comparison.

## Frozen input state

The exact pre-change Git-status byte stream has SHA-256
`773e358503d423625f7e74c1dc03f2367ee342bf7c54a70a04d32ae61a38b7af`.
Existing work, including untracked plans and packet records, is outside this
slice unless listed below and must be preserved. The primary source baseline
was recorded before this packet:

| File | Bytes | SHA-256 |
|---|---:|---|
| `crates/hyperai-market/src/lib.rs` | 105437 | `31b0664b547100450789e3fe4b8bbaae21e864caa3dac9e94447eb79e62d9970` |
| `crates/hyperai-application/src/lib.rs` | 106711 | `1cb64bdea3843140d522334ca3c607eb7955e705bf834f6c3e9a0365198d7f41` |
| `crates/hyperai-local-service/src/server.rs` | 80971 | `b9cc9e8decb2a341ae4dab63a5ef3a0b1a8ac6714bbb1dc32355c590640cfca1` |
| `crates/hyperai-local-service/src/route_policy.rs` | 7537 | `137cddf675449fcae9531bc9245dfd09b2d3dc8c0b7faa226a15909e1f22806a` |
| `crates/hyperai-local-service/tests/api_contract.rs` | 12814 | `36b9ba02ea16daeb69ff924fa1fc754c7c50d46389d8584358d63855d4ab909e` |
| `crates/outcomes-cli/src/lib.rs` | 147677 | `7a9f112f1c67e13ae542646ca0f0dbf72cdfe65da0be98804d879d5f16e79a93` |
| `crates/hyperai-tui/src/lib.rs` | 96738 | `bf9b2f124345f869b79b30c9d7b8cbc908994fe5dbcdb68e6bdae5e5ac551ff5` |
| `crates/hyperai-acp/src/tools.rs` | 5573 | `e8406f28137afeb26854a77758a09469dae77868c49feb72fc8753c074aac1ac` |
| `crates/hyperai-mcp/src/lib.rs` | 33894 | `e5da96be4c3c161ece1299353ebd99e817c87bf6901d4048fbdeb513d7ffc40b9` |
| `apps/hyperai-interface/index.html` | 4915 | `1447f2637de79aca50247665ee1355a938dac8870656019fbabcd9e758709a22` |
| `apps/hyperai-interface/app.js` | 152614 | `fa6f6e6c46ddf72f786be9520fcede454137618c0608f08904c69b0e7bc3c142` |
| `apps/hyperai-interface/app.css` | 90133 | `cae429ce4f38abe89b05923853cfbc6f009f89315048a5cae1dc8ba178eea3b0` |
| `scripts/verify-interface-browser.mjs` | 150103 | `837f73d3e14297611289f74e5643480d09417380e32bfe763a99374ba168cfe0` |
| `package.json` | 6864 | `f5bcb21883d20894bfd58c8438bc87895df3cb64f4fbe9033dce03302ca3f1d6` |

These hashes describe the source bytes before packet documentation was added;
they are preservation baselines, not expected final-file hashes. The exact
pre-existing status digest remains a preservation check, with only this
packet, the state record, and explicitly listed plan updates allowed to add
documentation paths.

## Exact writable files

- `crates/hyperai-market/src/lib.rs`
- `crates/hyperai-application/src/lib.rs`
- `crates/hyperai-local-service/src/server.rs`
- `crates/hyperai-local-service/src/route_policy.rs`
- `crates/hyperai-local-service/tests/api_contract.rs`
- `crates/hyperai-local-service/tests/agent_api.rs`
- `crates/outcomes-cli/src/lib.rs`
- `crates/outcomes-cli/tests/cli_compat.rs`
- `crates/hyperai-tui/src/lib.rs`
- `crates/hyperai-tui/tests/terminal_restore.rs`
- `crates/hyperai-acp/src/tools.rs`
- `crates/hyperai-acp/src/session.rs`
- `crates/hyperai-acp/src/lib.rs`
- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`
- `crates/hyperai-acp/tests/synthetic_peer.rs`
- `crates/hyperai-mcp/src/lib.rs`
- `apps/hyperai-interface/index.html`
- `apps/hyperai-interface/app.js`
- `apps/hyperai-interface/app.css`
- `apps/hyperai-interface/market-supply-comparison.js` (new)
- `apps/hyperai-interface/market-supply-comparison.test.mjs` (new)
- `scripts/verify-interface-browser.mjs`
- `package.json`
- `docs/compute-bazaar-buyer-supply-comparison-packet.md`
- `docs/state-slices.md`
- `docs/bazaar-hyperai-interface-plan.md`
- `docs/compute-bazaar-rust-port-subplan.md`

No Cargo manifest, lockfile, imported product tree, ignored Bazaar reference,
generated data, or runtime artifact is writable in this slice.

## Contract and behavior

1. Add an additive, deny-unknown-fields schema
   `hyperai.compute-market.supply-comparison.v1`. Bind its body with a
   domain-separated digest. Include the state slice, fixed catalog and digest,
   buyer request, comparison time, deterministic eligible selection (or
   explicit no-eligible result), one assessment per signed catalog entry,
   advisory observations as a separate collection, freshness status
   `not_evaluated`, claim ceiling, nonclaims, and effects.
2. Derive each assessment exclusively from the verified signed offer and
   buyer request: resource model, region, capacity, asking-rate cap, checked
   total spend cap, and validity at the fixed comparison time. Preserve all
   applicable rejection reasons. Keep deterministic selection identical to
   the existing lowest-asking-rate then offer-ID rule. Make the existing
   selector share the same assessment logic so this view cannot drift from
   deal eligibility.
3. Keep asking rate and estimated market rate in separately named fields with
   their own source, unit, timestamp, and digest. Estimated availability stays
   advisory and never supplies seller capacity. No observation is accepted
   into the catalog or selection input.
4. Use a pure application projection over the existing fixed synthetic
   fixture. Do not call quote creation, approval, fulfillment, verifier,
   dispute, receipt, or settlement code to build the comparison.
5. Serve it through authenticated owner API `GET
   /api/v1/market/supply-comparison` and a separately scoped read-only alias
   `GET /api/v1/agent/tools/market/supply-comparison`. Append one new
   `hyperai_market_supply_comparison` entry to MCP V5; preserve V1–V4 names,
   ordering, and protocol version. The alias must not grant access to owner
   writes or other owner routes.
6. Add read-only CLI `market compare` and a TUI Market view, both loading the
   same owner API contract. The browser Market workspace renders the same
   response with clear hierarchy: buyer constraints; eligible offers and
   reasons; selected offer; seller asking terms; advisory analysis; provenance
   and timestamp; and visible synthetic-only/no-live-seller boundary. Include
   explicit loading, empty, stale/unknown, denied, malformed, and service-error
   states. Keep SCM captured listings in their separate explorer.

## Effects and exclusions

Allowed: local source edits, formatting, static checks, fixed synthetic
contract tests, bounded local API tests on ephemeral ports, and browser
qualification that uses only the fixed local fixture if its dedicated runner
is available and terminates cleanly. Record each command and exact result.

Not allowed: provider calls; public federation; registry fetch; seller
negotiation; paid evaluation; Fleet operation; model inference; reading or
joining ignored Bazaar data; connecting JustAI, HyperIntel runtime/provider,
or ESN-JustNet; wallet, RPC, HyperEVM, settlement, funds, external publication,
production service, real-user inputs, or persistence of comparison records.
Do not change the source's rights posture or make reuse claims for Bazaar.

## Proof gates and acceptance

- Demonstrate assessment parity with the unchanged buyer constraints and
  selection order, including negative cases for every eligibility reason,
  amount overflow, no eligible offer, and changed advisory values that leave
  eligibility and selected offer unchanged.
- Verify schema strictness, comparison-body digest, nested source digests,
  deterministic serialization, and output ordering.
- Verify the owner GET is authenticated; the agent alias is read-only and
  scoped; wrong methods and unrelated owner routes are denied; MCP V1–V4
  remain byte-for-byte/name-for-name compatible and V5 appends exactly one
  tool. ACP remains protocol v1 while its advertised read-only tool set is
  versioned to V5 and retains the exact V4 prefix; the fixed ACP integration
  peer verifies the new tool through the MCP sidecar.
- Verify API, CLI, TUI, MCP, and browser consume one response contract and
  expose the same eligibility, selection, advisory boundary, and errors.
- Verify no quote, approval, execution, evaluation, Fleet, provider, storage,
  settlement, or receipt operation occurs while generating the projection.
- Run focused crate and interface checks, `corepack pnpm run lint:fast`,
  `corepack pnpm run verify:contracts`, import verification, formatting,
  `git diff --check`, and the current fixed-fixture browser workflow if the
  runner's cleanup is conclusive. Do not repeat the known unresolved full
  `verify:full` HomeBusy failure as part of this slice.
- Recheck imported tree digests and pre-existing status entries, report exact
  revision/state slice, assurance, reviewer and reproduction statuses,
  custody, limitations, claim ceiling, and next authority gate.

## Recorded result (2026-10-01)

Revision is `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`; this
implementation remains uncommitted in the already-dirty working tree.

The browser, authenticated local service, read-only CLI, TUI, MCP V5, and ACP
V1-session projections now use the same deterministic, digest-bound synthetic
comparison. Advisory observations remain separate from offer eligibility and
selection. MCP V1–V4 names/order are preserved; V5 appends one read-only tool.
The fixed Simple Compute Market capture explorer remains separate.

Local proof results: `corepack pnpm run lint:fast` passed, including interface
syntax and 32 Node tests, installer checks, pinned `cargo fmt --check`, and
workspace Clippy. `corepack pnpm run verify:contracts` passed with 203 tests,
1 ignored Postgres test requiring `HSAI_TEST_POSTGRES_URL`, and 0 failures.
The focused no-eligible-offer test passed with pinned Rust 1.93.1. The pinned
workspace package check and the buyer-comparison, API/auth, CLI, TUI, MCP, and
ACP unit/contract checks passed. `python3 scripts/verify_imports.py` passed
for Simple Compute Market (2,189 files), HyperIntel (99), and JustAI (176);
`git diff --check` passed.

At this implementation packet's close, the fixed-fixture browser workflow was
not run because cleanup around the then-recorded port 4317 state was not
conclusive. A separate local continuation later completed the fixed
comparison workflow; see the [browser qualification packet](compute-bazaar-interface-market-supply-browser-packet.md).
The ACP synthetic-peer integration tests that use port 4317 were not run. The
32 Node renderer tests are not browser-rendering or accessibility
evidence. The source tree and imports remain local; no Bazaar code or data was
copied, no seller/provider was contacted, and no settlement or external write
occurred. Assurance is `INTERNAL_PASS` for the local synthetic contracts and
client projections actually checked; reviewer status is `NOT_REQUESTED`,
reproduction is `NOT_REPRODUCED`. The claim ceiling remains fixed local
synthetic comparison only. Screen-reader acceptance, cross-browser/platform
behavior, live-source rights and identity, provider execution, Fleet/Trade
runtime, settlement, and production authorization remain separate gates.

## Release and authority boundary

This packet authorizes local implementation and synthetic verification only.
Expected assurance is `INTERNAL_PASS` only if the stated local proof gates
pass; otherwise report the literal failed or inconclusive result. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. No independent
acceptance, live adapter, reuse of Bazaar code/data, deployment, publication,
paid execution, seller contact, testnet write, or production authorization
follows. The next gate for any live supply source remains source-specific
rights, identity/key governance, freshness/replay, privacy/retention, endpoint
policy, and a new explicitly authorized state slice.

### Browser qualification continuation (2026-10-01)

The separate browser-only continuation passed on Google Chrome
154.0.8037.92: one HTTP 200 comparison GET, no writes, keyboard and computed
accessibility-tree assertions, selected synthetic offer, separated advisory
observations, and 390px internal table scrolling. The owned service/browser
processes exited, ephemeral listeners closed, and the temporary home was
removed. This closes only the fixed local browser-rendering proof; see the
[continuation packet](compute-bazaar-interface-market-supply-browser-packet.md)
for exact output, runner corrections, custody, and remaining limitations.

### Installed ACP/MCP V5 continuation (2026-10-01)

Under [`hyperai.bazaar.buyer_supply_comparison_acp_digest_parity.v1`](state-slices.md#hyperai-bazaar-buyer_supply_comparison_acp_digest_parityv1),
the fixed synthetic ACP peer decodes the pinned MCP wrapper's text form,
checks schema `hyperai.compute-market.supply-comparison.v1` and selected offer
`offer-gpu-b`, and returns the comparison digest. The installed CLI/MCP
integration compares it with the authenticated owner API digest and passes.
The exact offline command was:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli --test cli_compat source_install_bundles_mcp_sidecar_and_connects_synthetic_peer -- --ignored --exact --nocapture
```

It passed on macOS arm64 (`1 passed, 0 failed`). Offline workspace build,
`corepack pnpm run lint:fast`, `corepack pnpm run verify:contracts` (203
passed, 1 ignored, 0 failed), pinned Rust formatting, and
`python3 scripts/verify_imports.py` also passed. Assurance is `INTERNAL_PASS`
for this installed local synthetic integration only; reviewer status is
`NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`. An unrelated runner
Data permission probe printed `PermissionDenied`; no Data worker behavior was
exercised. No fixed port, provider, seller, external ACP peer, or production
service was contacted. External ACP behavior, other platforms, clean
reproduction, live supply, computation correctness, payment entitlement, and
production authorization remain unverified. The separate Data worker Phase 2
remains `NOT_ASSIGNED`.
