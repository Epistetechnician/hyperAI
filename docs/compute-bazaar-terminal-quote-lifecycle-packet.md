# Terminal Buyer Quote Lifecycle Packet

State slice: `hyperai.bazaar.terminal_quote_lifecycle.v1`.
Parent: [HyperAI end-to-end interface plan](bazaar-hyperai-interface-plan.md).
Market owner contract: `hyperai.bazaar.supply_to_quote.v1` and the existing
local quote-preview, buyer-approval, and mock-completion contracts.
Source rights: this slice uses only the HyperAI-owned fixed synthetic fixture;
it reads no Bazaar source or data.

## Purpose and boundary

Close the local terminal-client gap in the buyer journey. The browser already
previews a fixed synthetic quote, asks for explicit approval, and separately
requests mock completion. The CLI currently stops after quote preview, and the
TUI displays a preview without advancing the lifecycle. Add explicit CLI
commands and TUI key actions through the existing authenticated owner routes.

The route, application, market-domain, receipt, persistence, and storage
contracts remain unchanged. `hyperai-acp` and its delegated MCP tools remain
read-only: agent capabilities may inspect the comparison and request the
quote-preview projection, but cannot approve or complete it. This is a
separate local synthetic lifecycle slice, not a grant inferred from general
client parity.

## Exact interaction contract

- CLI keeps existing command output and adds `market quote-approve
  --preview-digest DIGEST` and `market quote-complete --preview-digest DIGEST
  --deal-digest DIGEST`, each with the existing optional `--home` selector.
- CLI commands use the local owner's bearer capability and POST only to
  `/api/v1/market/quote-approve` and `/api/v1/market/quote-complete`.
- TUI `Market` keeps Enter for the existing supply-to-quote preview. After it
  displays the exact nested quote-preview digest, `a` requests the synthetic
  buyer approval. After it displays the returned deal digest, `c` explicitly
  requests deterministic mock completion. The TUI checks the response schema
  and both digest bindings before advancing each stage.
- Each TUI action is disabled once its result is present in the current
  session. A preview refresh clears the approval and completion projections.
  Failures remain visible, preserve the last valid stage, and never trigger an
  automatic retry.
- The rendered result says that the buyer signature, seller completion, local
  verifier, and disposition are synthetic/mock records. It states that no
  seller or provider was contacted, no workload ran, and no funds moved.
- All records remain volatile responses in the local process. No receipt,
  approval, event, or activity is persisted by this slice.

## Frozen inputs

Baseline revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved. The exact client inputs
at packet freeze were:

| File | SHA-256 |
|---|---|
| `crates/outcomes-cli/src/lib.rs` | `d3c3096b8ade2c00959b89158dbedb0f28ac269d9d3a5daa80f1e825a6020d36` |
| `crates/outcomes-cli/tests/cli_compat.rs` | `5902d02a83375b37f102010eeb34f1037638bd845f91442e0530356e64456891` |
| `crates/hyperai-tui/src/lib.rs` | `2aafa64e3048a9af386f07532a2ef4c109bb39e504bb6d137ecb1cab22d82f94` |
| `crates/hyperai-application/src/lib.rs` (read-only) | `dcb505ad95e25352434a2959d6143f00030766ff9c095c1c305c27e25e3b8c05` |
| `crates/hyperai-market/src/lib.rs` (read-only) | `16b07ec36107d29182020abecd23b8cd23862e3c3203914790877409a65a44f8` |
| `crates/hyperai-local-service/src/server.rs` (read-only) | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `docs/bazaar-hyperai-interface-plan.md` | `df49cd6292daed85fb0850e8f1b17261f332120934b9a1ed699f090a99c8be79` |
| `docs/state-slices.md` | `03f412a6626185315518fe9a846e88420b34937033d0f949fea667e0eb6f22d4` |

## Writable files and allowed effects

- `crates/outcomes-cli/src/lib.rs`
- `crates/outcomes-cli/tests/cli_compat.rs`
- `crates/hyperai-tui/src/lib.rs` (including its unit tests)
- This packet, `docs/state-slices.md`, and the interface plan's phase-status
  row.

Allowed effects: implement the two additive CLI commands and TUI state/actions;
call the existing owner routes against test-owned services on kernel-assigned
IPv4 loopback ports; use disposable synthetic homes; run the focused Rust
workspace suite, `lint:fast`, formatting, and diff checks. No dependency or API
contract changes are required.

## Non-goals

No service/application/domain-route changes; new agent or ACP tools; credential
or permission changes; persistent lifecycle state; new receipt schema; live
registry, seller, provider, Fleet, Eval, Trade, escrow, settlement, payment,
wallet, chain, network ingestion, public release, or deployment; Bazaar source
or data reuse; or broad human usability, accessibility, or platform claims.

## Proof gates

1. CLI integration starts the existing local service in a disposable home,
   compares the authenticated owner response, obtains the nested quote preview,
   approves its exact digest, and completes only with the returned deal digest.
2. CLI output binds preview, approval, deal, and receipt digests to the same
   synthetic records. Invalid digests retain the existing stable error
   envelope and do not report success.
3. TUI state uses the same two owner routes, refuses to advance without the
   current preview/approval, rejects mismatched returned digests, renders each
   stage and its effect boundary, and does not issue duplicate actions after a
   stage succeeds.
4. Existing agent-capability tests continue to deny the owner approval and
   completion routes; ACP/MCP tool lists and permissions do not change.
5. Existing CLI aliases, market commands, TUI terminal restoration, market
   schemas, and all unrelated dirty/untracked work remain intact.

## Validation record

Final working-tree assurance: `INTERNAL_PASS` for this exact local synthetic
CLI/TUI lifecycle and the repository fast gates. The focused root suite and
`lint:fast` passed. The focused suite includes the CLI compare/preview/approve/
complete journey, TUI preview/approve/mock-completion journey, stale digest
rejections, duplicate-action suppression, and the existing agent capability
denial coverage. The CLI lifecycle test was also run directly after its final
helper extraction and passed (1 test).

Commands and final checks:

- `CARGO_NET_OFFLINE=true corepack pnpm run test:focused`: passed.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast`: passed, including
  interface and installer checks, pinned formatting, and workspace Clippy.
- `git diff --check`: passed for the tracked diff; a scoped whitespace scan
  found no trailing whitespace in the source and documentation files above.
- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli --test cli_compat market_compare_cli_matches_the_authenticated_service_projection`:
  passed (1 test).

Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`; checks ran
in one local environment. Custody: HyperAI-owned synthetic responses, fixed
fixtures, and disposable local homes only. Claim ceiling: the exact current CLI
and TUI implementations can advance the fixed local synthetic lifecycle
through existing owner APIs with checked digest bindings. This proves no real
compute, seller intent, independent verification, correctness, payment, or
settlement. Native platform support and broad human usability/accessibility
remain unqualified. The next external gate is the exact Bazaar source/data
rights decision before any Bazaar material is reused; live integrations and
production actions still require separate owner-authorized slices.

Final client source SHA-256:

| File | SHA-256 |
|---|---|
| `crates/outcomes-cli/src/lib.rs` | `9367ccb2e2c1ac1d5d968e32d8e932938f97c8d82c6bee5903aed88db919354c` |
| `crates/outcomes-cli/tests/cli_compat.rs` | `3f0a86eb70eab4d9250e0aaa57e5679c66cdb6f7b6234f4abd870cba95754465` |
| `crates/hyperai-tui/src/lib.rs` | `346b9c1f68d1bbc8cba70e9e1e49e7ad62a6d2d2d0288cbd92a2d2bac9deb203` |

Publish boundary: local source/documentation only; no stage, commit, PR,
release, publication, or deployment.
