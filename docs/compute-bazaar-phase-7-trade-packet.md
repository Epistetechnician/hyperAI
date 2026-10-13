# Phase 7 Local Trade Research Packet

State slice: `hyperai.bazaar.trade_research.v1`.
Parent plan: [HyperAI Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md).
Predecessors: [Phase 2 Local Data](compute-bazaar-phase-2-data-packet.md),
[Phase 3A synthetic derivation](compute-bazaar-phase-3-synthetic-transform-packet.md),
and [Phase 6 local Eval](compute-bazaar-phase-6-eval-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Objective and bounded task list

1. Add a versioned HyperAI-owned synthetic hourly asking-price series to the
   Data contracts, with provenance digest, an explicit sampling grid, one
   missing price interval, and unknown availability retained as unknown.
2. Add Trade research contracts for research instruments, series coverage,
   contiguous-step variability, regional asking-price basis, bounded scenario
   shocks, reference sensitivity, and deterministic paper procurement-cost
   replay.
3. Expose the same typed operations through the application facade, local
   authenticated service, CLI, TUI, and browser Trade workspace.
4. Validate deterministic outputs, source binding, unknown/missing behavior,
   input limits, adversarial sensitivity, and that all effects remain local
   and synthetic.

This phase is research and interface plumbing only. It does not create
tradable instruments, investment recommendations, price or availability
oracles, orders, positions, wallets, broker or chain connections, escrow,
settlement, or financial returns.

## Baseline and state boundary

Baseline revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty monorepo and Phases 1–6 preserved. No commit or frozen
worktree digest is claimed. The prior synthetic Data snapshot has one
cross-sectional timestamp and cannot support a time-series replay. This slice
adds a separate, independently authored in-memory Data release rather than
changing the existing snapshot or copying Bazaar data, code, schema, methods,
or fixtures.

Included files:

- `Cargo.toml`, `Cargo.lock`, and new `crates/hyperai-trade/`: typed research
  instruments, series analysis, regional basis, bounded shock scenarios,
  sensitivity calculations, and paper procurement-cost replay.
- `crates/hyperai-data/src/lib.rs`: the fixed eight-hour synthetic regional
  asking-price series, versioned source contract, and source digest.
- `crates/hyperai-application/Cargo.toml` and `src/lib.rs`: typed Trade
  facade functions composed over the Data-owned release.
- `crates/hyperai-local-service/src/server.rs` and new
  `crates/hyperai-local-service/tests/trade_api.rs`: bounded authenticated
  catalog, series, scenario, and paper-replay API paths.
- `crates/outcomes-cli/src/lib.rs`: `hyperai trade
  instruments|series|scenario|paper` as clients of the same local service.
- `crates/hyperai-tui/src/lib.rs` and `tests/terminal_restore.rs`: Trade
  workspace reads and deterministic scenario/replay summaries.
- `apps/hyperai-interface/index.html`, `app.js`, and `app.css`: enable the
  Trade navigation and provide source, method, coverage, scenario, sensitivity,
  and paper procurement-cost views.
- `README.md`, this packet, `docs/state-slices.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`.

No changes to `products/`, `references/`, HyperIntel, JustAI, Simple Compute
Market, outcome-resolution or settlement semantics, provider adapters, live
data ingestion, or payment code are included.

## Frozen Data source and methodology

- Dataset ID: `hyperai.synthetic.compute-hourly-series.v1`.
- Method/schema identity: `hyperai.data.synthetic-hourly-series.v1`.
- Source: HyperAI-authored fixed synthetic observations, compiled into the
  `hyperai-data` crate and returned as a typed immutable release. Its snapshot
  digest binds the exact ordered observations and sampling metadata.
- Sampling grid: eight UTC hourly slots from `2026-01-01T00:00:00Z` through
  `2026-01-01T07:00:00Z` for A100 observations in `us-east`, `us-west`, and
  `eu-west`. One east-region slot is absent. At least one observed asking
  price has unknown (`null`) availability; unknown is never zero.
- The source is synthetic and has no seller, provider, execution, or market
  freshness authority. The grid is a teaching fixture, not a historical
  market feed or a valid compute-price index.
- Variability method: mean absolute basis-point change between adjacent
  one-hour observations for the same instrument. A missing interval breaks
  the pair; the method does not bridge gaps or call the result forecast
  volatility.
- Basis method: same-GPU-model, same-timestamp regional asking-price
  difference in integer USD micros per GPU-hour. Report the latest common
  observed point and common-point coverage. A quote difference is not a
  tradable basis contract.
- Scenario method: apply an integer basis-point asking-price shock bounded to
  `[-5000, 5000]` and a bounded integer availability change to the primary
  instrument's latest observation. Reject overflow or negative resulting
  availability. Carry unknown availability through unchanged.
- Sensitivity method: compute the latest complete three-region median asking
  price, then recompute after increasing its highest input quote by 100%; also
  report the change when using the immediately prior complete interval. Label
  both as fixture-method sensitivity, not market-manipulation resistance or
  stale-feed detection.
- Concentration method: report each fixture instrument's share of observed
  rows and a Herfindahl-style concentration score for fixture coverage. Label
  this as sample composition, never provider or market concentration.
- Paper method: for each fixed slot, select the lowest asking price among
  regions with known availability at least equal to requested units and
  compare that asking-cost estimate with one named baseline region. Skip
  intervals with missing price data, unknown availability, or insufficient
  capacity and report each skip count. Ties break by instrument ID. The
  signed aggregate is a difference in hypothetical asking costs, not a fill,
  realized saving, trade P&L, or forecast.
- All arithmetic is checked integer arithmetic with fixed rounding rules.
  Floating-point model fitting, stochastic sampling, and wall-clock freshness
  checks are excluded.

## API and client contract

- `GET /api/v1/trade/catalog`: fixed research instrument catalog, dataset
  digest, method versions, limits, claim ceiling, and effect boundary.
- `GET /api/v1/trade/series/{instrument_id}`: fixed points, expected and
  observed slot counts, price coverage, availability known/unknown counts,
  and contiguous-step variability.
- `POST /api/v1/trade/scenario`: strict bounded body containing expected
  dataset digest, primary and comparison instrument IDs, price shock basis
  points, and availability delta.
- `POST /api/v1/trade/paper`: strict bounded body containing expected dataset
  digest, baseline instrument ID, and requested units.
- Existing loopback authentication, Host/Origin checks, CSRF protections,
  request IDs, fixed error envelopes, and service timeouts apply. Bodies are
  capped at 4 KiB. These pure local calculations write no durable state and
  trigger no external effects.
- Browser, CLI, and TUI call the shared service endpoints. They do not embed
  a separate calculation or data copy.

## Allowed effects and hard limits

Allowed effects are reading the compiled-in HyperAI synthetic release,
computing bounded summaries/scenarios in memory, and displaying typed results
through the authenticated local service and its clients. No result is stored
in the local metadata database or added to Activity as a durable record.

There are three instruments, eight expected hourly slots, at most 24 source
records, a 4 KiB request body, and a scenario price shock bounded to 50% in
either direction. The service binds to loopback under the existing Phase 1
lifecycle. There is no network, provider, wallet, key, or credential access.

## Proof gates

- Data source bytes and release digest are deterministic, versioned, and
  visibly synthetic; the existing Phase 2 snapshot digest remains unchanged.
- Series output preserves the missing interval and nullable availability;
  no operation converts unknown to zero or bridges the price gap.
- Basis rejects incompatible GPU models and computes only at common UTC slots.
- Scenario requests reject unknown fields, stale dataset digests, invalid IDs,
  out-of-range shocks, integer overflow, and negative availability.
- Paper replay is deterministic, uses only known sufficient capacity, has
  stable tie-breaking, and reports skipped/unknown intervals separately from
  its asking-cost difference.
- The one-quote manipulation and one-period-lag sensitivity outputs are
  deterministic and carry the fixture-only claim ceiling. Fixture record
  shares and concentration do not imply market participation.
- API authentication, CSRF, body limit, route contract, and no-persistence
  behavior pass local service tests. Browser, CLI, and TUI produce the same
  result schema and preserve the research labels.
- Focused tests, root lint, JavaScript syntax, diff hygiene, and responsive
  desktop/820px/390px Trade-workspace checks pass.
- Bounded adversarial review covers stale digests, mismatched instruments,
  missingness, capacity manipulation, price outliers, replay input bounds, and
  attempted financial or network authority.

## Assurance report

Exact source revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the
current uncommitted working tree; no frozen diff digest is claimed. State
slice: `hyperai.bazaar.trade_research.v1`. Assurance level:
`INTERNAL_PASS` for the bounded local scope only. Reviewer status:
`NOT_REQUESTED`. Reproducibility status: `REPRODUCED_LOCAL` for the exact
Trade-related Rust package suite recorded in
[`hyperai.bazaar.eval_trade_rust_reproduction.v1`](state-slices.md#hyperai-bazaar-eval_trade_rust_reproductionv1).
That same-owner Docker reproduction covers Rust tests and the fixed-seed
input sweep only. Full phase exit remains open for a separate-host reproduction
of the packet's root, JavaScript, browser, and responsive-view checks.

Local proof completed: `corepack pnpm run lint` passed formatting, workspace
Clippy, contract checks, all workspace tests, and doc tests. The focused Trade
API suite passed 3/3 and the Trade unit suite passed 7/7, including a
fixed-seed, 1,024-case scenario-input sweep. `node --check
apps/hyperai-interface/app.js` and `git diff --check` passed. Interactive
browser review exercised the authenticated local service and confirmed the
missing East interval and synthetic claim labels; desktop (1280 px), tablet
(820 px), and phone (390 px) layouts had no horizontal overflow.

Threat model: callers control request IDs, snapshot digests, instrument IDs,
price/capacity assumptions, and browser request origins. Assets are the
integrity of the compiled-in synthetic release, the bounded local service, and
the accuracy of the user-visible claim labels. The service stays on IPv4
loopback and uses its existing authentication, Host/Origin checks, CSRF token,
and 4 KiB body limit; request structs reject unknown fields; calculations are
in-memory and digest-bound. Stale digests, unknown IDs, invalid assumptions,
oversized bodies, unauthenticated access, and missing CSRF are rejected. The
Trade calculation crate and application facade contain no outbound-network or
filesystem calls; its API exposes only fixed catalog/series reads and bounded
in-memory scenario/paper calculations. Local host compromise and
bearer/session credential theft are outside this synthetic research slice.

The deterministic input-sweep harness is
`deterministic_scenario_input_sweep_stays_within_the_research_contract`, with
seed `0x4859504552414901`, 1,024 generated requests, and no persisted corpus or
external artifact. Its source, release identity, limits, and output are
regenerated by the pinned local test command.

Data and artifact custody: fixed HyperAI-authored synthetic observations and
bounded local outputs only. The clean-container Rust result, source manifest,
and output digests are recorded in the linked reproduction slice. Known
limitations: the price series is not empirical, variability is descriptive,
the input sweep is bounded rather than
an open-ended fuzz campaign, and paper cost comparisons do not prove
availability or execution. Claim ceiling: deterministic research plumbing
over the exact synthetic release. No live financial or operational claim is
eligible. The package-specific Rust reproduction is recorded, while the full
phase gate remains a separate-host reproduction of root, JavaScript, browser,
and responsive-view checks with toolchain, dependency, fixture, and output
digests. This remains separate from independent review and production
authorization. Real feeds, trading, oracle publication, funds movement, and
deployment require new source-specific plans, state slices, and owner/operator
authority.
