# Buyer Supply to Quote Preview Packet

Status: accepted local synthetic implementation slice under the user's
direction to follow the Compute Bazaar Rust port subplan.

State slice: `hyperai.bazaar.supply_to_quote.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related contract: [buyer supply comparison](compute-bazaar-buyer-supply-comparison-packet.md).

## Goal and boundary

Connect the selected signed offer in the existing buyer supply comparison to
the existing seller-quote preview. The browser, authenticated service, CLI,
TUI, and read-only MCP tool exposed through ACP v1 must consume one
versioned, digest-bound response from the shared application facade.

The implementation uses only the fixed HyperAI synthetic market fixture. It
does not read or join the ignored Bazaar checkout or SCM captures. Advisory
price and availability observations remain separate from seller asking terms
and cannot affect offer eligibility or selection. The source-specific owners
retain their records and authority.

This is a preview-only connection. It creates no quote approval, deal,
fulfillment, evaluation, fleet operation, verifier decision, receipt,
settlement, persistent record, or external effect. Existing quote preview,
approval, completion, workload trace, V1 records, route behavior, ACP protocol
v1, MCP V1-V5, CLI commands, TUI keys, storage, and platform contracts remain
compatible. The new operation is additive and read-only even though its
authenticated local transport uses POST.

## Baseline and frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. The NUL-delimited
`git status --porcelain=v1` digest before this packet was
`896c3bd31ae72bda8909e7b05434bae9dc40f73663c7c4aed6fd75d7d2622e55`.

The source hashes below are the exact pre-implementation inputs. Recheck them
before editing; if any differs, stop and freeze a fresh packet.

| File | SHA-256 |
|---|---|
| `crates/hyperai-market/src/lib.rs` | `35b3dc1d8bbe92877c4ce421d53eff0c31a8253cce0753ff07b2f0420735591e` |
| `crates/hyperai-application/src/lib.rs` | `627e75c65d8a16db8be3bbc9b6df6d7e0e532f11d04682cfd2984288b53e7af0` |
| `crates/hyperai-local-service/src/server.rs` | `f45d9caac28e4ab5d5bfbb49d5277286811d37c367f0ec3c490c601aca67cfbe` |
| `crates/hyperai-local-service/src/route_policy.rs` | `0cd546dc17d4806c6bf24b5d862c6fbeafa32e6d16aa96a269f428c20e29b429` |
| `crates/hyperai-local-service/tests/api_contract.rs` | `a9973f63d99040d6cc76042e84cf3ecc55504e153b1743f294505e01298e2d87` |
| `crates/hyperai-local-service/tests/agent_api.rs` | `0c6fcb0c86f4fac13f09f3f1cfd09420841c03d316b5ed2721bc1f33e2348b05` |
| `crates/outcomes-cli/src/lib.rs` | `dbc55295d133f49b1503982db1be73588fe8b36b2e95052a05ded970eafb9c1e` |
| `crates/outcomes-cli/tests/cli_compat.rs` | `5cd5e587be2ce58aa6fd3241b9956b6477a83b825401481af4068c858402bfe2` |
| `crates/hyperai-tui/src/lib.rs` | `982729da2eb7948ed761497e4c96658eef6d9685ba484d7324fc57810c084a29` |
| `crates/hyperai-tui/tests/terminal_restore.rs` | `c427b2304cfd9580cc04a196ee71a177e95fb285479c413f98da5e7bf3930727` |
| `crates/hyperai-acp/src/tools.rs` | `75d84a5321ecba3b84f3737f7be6ae39055c2478a7d66ab3896618221588f65b` |
| `crates/hyperai-acp/src/session.rs` | `04d5fa3d3e0c40406bdcbb20025e7dcf6366c97c244b879ba70c755a84329fef` |
| `crates/hyperai-acp/tests/synthetic_peer.rs` | `1db4097c1210f49b9973d1d80149ba928b7fa1d2f6f968515c91cc6205a858d8` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | `bf614055152cbe6d85e58a8f6cb51fa20063d32e0ba9c36f1accc1cc798c9499` |
| `crates/hyperai-mcp/src/lib.rs` | `7edbdbcd9ef1ef1bba90488099d6904d0829da28738c554aa3050d0ade478d41` |
| `apps/hyperai-interface/app.js` | `8078c9779dcf6ca363eddc59cce78309c790df97c66baa2369a403423eee4555` |
| `apps/hyperai-interface/market-supply-comparison.js` | `1d4b673cf5d301efced8cb54da889c6ed8c014b21726415efe51c15e531c44a5` |
| `apps/hyperai-interface/market-supply-comparison.test.mjs` | `7bf249f31693a1ff4d0eba043e754918ed245d822a1ff3c49d4ca170dbd203d2` |
| `scripts/verify-interface-browser.mjs` | `1beacd753e48645d0f8abf5a1e166a0b9dd839d8ec9d51a00f0e54b7655db373` |
| `package.json` | `ef7e75130b7db670e42bae49559bad7d28ab204ae4353a0451e408fcda964d6e` |
| `docs/state-slices.md` | `d717175a41deaebfdda41a5c5c4f01b2a76b0472c60c6c74fc83b1e1fc441d52` |
| `docs/compute-bazaar-rust-port-subplan.md` | `cd6d120a1ec6f1a60b37b04e57f3c4b8f46a135202c1a54ac882bbbc320d6a15` |
| `docs/bazaar-hyperai-interface-plan.md` | `6ebdb74d65c89766467f848b1e7b13bbe2f6c85122b0c9aa6e2e9d204516dc08` |

## Contract

1. Add `SupplyQuotePreviewV1` with schema
   `hyperai.compute-market.supply-quote-preview.v1`. Its digest-bound body
   contains the state slice, input comparison digest, catalog digest, buyer
   request digest, selected offer ID and signed-offer digest, the existing
   `LocalMarketQuotePreviewV1`, claim ceiling, nonclaims, and effects.
2. The market builder recomputes and verifies the fixed supply comparison,
   rejects a caller digest that does not match it, and constructs the existing
   quote preview from the same fixture. Require exact catalog digest, buyer
   request, selected offer ID, and selected signed-offer digest parity before
   returning the wrapper. No advisory observation participates in quote terms.
3. Expose authenticated owner route
   `POST /api/v1/market/supply-quote-preview` with the exact input
   `{ "comparison_digest": "<sha256>" }`. Add the correspondingly scoped
   read-only agent route
   `POST /api/v1/agent/tools/market/supply-quote-preview`. Invalid input,
   stale/mismatched comparison digest, and no eligible offer return stable
   typed errors and no preview.
4. Add `hyperai market quote-preview --comparison-digest <sha256>` and a TUI
   Market-workspace Enter action. Add a browser action beside the selected
   offer. Each client displays the selected offer, signed-offer digest, quote
   amount/cap, quote-preview digest, and explicit synthetic/no-seller/no-funds
   boundary from the shared response.
5. Append exactly one read-only MCP V6 tool,
   `hyperai_market_supply_quote_preview`, retaining V1-V5 names and ordering.
   ACP remains protocol v1; its session advertises the exact V6 tool list under
   a new read-only tool-set label. The scoped route cannot reach owner actions
   or any quote approval/completion path.
6. Browser quote approval remains a distinct existing action over the nested
   quote-preview digest. This slice does not make comparison observations,
   client state, or MCP/ACP authority part of buyer approval.
7. Do not change persistence, dependencies, supported-platform claims, source
   imports, API version, or existing response schemas.

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
- `crates/hyperai-acp/tests/synthetic_peer.rs`
- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`
- `crates/hyperai-mcp/src/lib.rs`
- `apps/hyperai-interface/app.js`
- `apps/hyperai-interface/market-supply-comparison.js`
- `apps/hyperai-interface/market-supply-comparison.test.mjs`
- `scripts/verify-interface-browser.mjs`
- `package.json`
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`.

No Cargo manifest, lockfile, database/storage file, imported product tree,
ignored Bazaar file, source fixture, external asset, or generated artifact is
writable in this slice.

## Allowed effects and stop conditions

Allowed: edit the exact files above; run fixed HyperAI-owned synthetic tests;
start test-owned authenticated services only on kernel-assigned loopback
ports; run the dedicated served-browser flow once with an isolated temporary
home/profile; and remove only the processes and files created by those runs.
Recheck import-tree digests and preserve the pre-existing working-tree state.

Stop on a source-hash mismatch, unexpected route or HTTP method, owner route
reachable through the agent capability, external-origin request, persistent
write, mismatched selected offer/catalog/request digest, ambiguous process
ownership, or incomplete browser cleanup. Do not rerun after a failed or
inconclusive browser result without a fresh packet.

Non-goals: source-code reuse from Bazaar; registry or seller calls; SCM
capture joins; inference or model execution; live Fleet; paid Eval; financial
Trade; provider calls; data ingestion; job execution; approval/fulfillment
behavior changes; persistence; wallet/chain/escrow; publication; external
review submission; release; or deployment.

## Proof gates and claim ceiling

- Market tests verify exact selected offer/catalog/request/signed-offer binding,
  deterministic wrapper digest, invalid and stale digest rejection, and no
  eligible offer behavior.
- The owner API route is authenticated. The agent alias is limited to the
  exact read-only route and cannot call approval, completion, or unrelated
  owner routes. Wrong methods and extra fields are rejected.
- CLI, TUI, browser, MCP, and the ACP v1 session use the same owner response
  digest and selected offer. MCP V1-V5 are an exact prefix of V6.
- Browser interaction begins in Market, opens the quote preview for the
  displayed selected offer, preserves the existing separate approval step,
  and shows the binding digests and synthetic claim ceiling.
- Run focused market/application/service/CLI/TUI/ACP/MCP/interface checks,
  `corepack pnpm run lint:fast`, `corepack pnpm run verify:contracts`,
  `python3 scripts/verify_imports.py`, pinned Rust formatting, and
  `git diff --check`. Run the scoped browser flow once. Do not rerun root
  `verify:full` under this slice; the existing `HomeBusy` result remains
  inconclusive and separately scoped.
- Report exact revision and state slice, `INTERNAL_PASS` or literal failed/
  inconclusive result, reviewer status, reproduction status, custody,
  limitations, claim ceiling, and next blocked authority gate.

Maximum claim: the exact fixed local synthetic supply-comparison response can
be bound to its matching seller-quote preview through the tested clients. This
does not establish seller intent, live availability, quote validity outside
the fixture, execution, correct computation, payment entitlement,
independent review, native-platform support, source rights, or production
readiness. Initial assurance is `NOT_RUN`; reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Publish boundary is local
source and documentation only; no staging, commit, PR, release, publication,
or deployment.

## Execution record (2026-10-01)

Implementation and local contract checks completed on baseline revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under this named state slice. The
focused Rust suite, `corepack pnpm run lint:fast`,
`corepack pnpm run verify:contracts`, `python3 scripts/verify_imports.py`,
pinned formatting, and `git diff --check` passed. Import verification matched
the recorded Simple Compute Market, HyperIntel, and JustAI snapshots. Assurance
is `INTERNAL_PASS` for these fixed local synthetic contracts and checks only;
reviewer status is `NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`.

The one served-browser attempt exited 1 at the pre-workflow
`zoom-200-equivalent/deals` viewport check because the document had horizontal
overflow. The selected-offer quote-preview interaction assertion was not
reached, so the end-to-end browser path is not passed. The runner emitted no
cleanup error; subsequent read-only process and temporary-directory checks
found no runner-owned browser/service process or remaining
`hyperai-interface-browser-*` directory. No browser retry is authorized by
this frozen packet. A fresh packet must diagnose the 720-by-500 Deals overflow,
include any required layout changes, and authorize a new single browser run.

The fixed fixture remained the only market data used. No Bazaar source, data,
asset, or dependency was read or imported. The maximum claim remains the local
digest-bound supply-to-quote contract; seller intent, live availability,
execution correctness, payment entitlement, source rights, independent review,
native-platform support, and production readiness remain unestablished. The
next open proof gate is the browser layout and quote-action run; the next
external authority gate remains exact source/reuse rights before any Bazaar
material can enter the build.
