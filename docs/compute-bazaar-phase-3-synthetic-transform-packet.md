# HyperAI Compute Desk Phase 3A: Synthetic Data Derivation

Status: accepted local implementation packet under state slice
`hyperai.bazaar.data_transform_synthetic.v1`.

Parent: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Product framing: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
This is the synthetic-only opening of Phase 3. It adds no upstream source
adapter and does not change the separate source-rights gates.

## Outcome

Build an in-memory Bronze/Silver/Gold projection from the existing,
HyperAI-authored synthetic JSON snapshot. Bronze refers to its verified
manifest and exact input digest; it does not duplicate the raw file. Silver
uses explicit integer price units and preserves unknown availability as null.
Gold groups the sample by GPU model and region, reports asking-price range and
availability coverage, and binds its output digest to the input snapshot and
method version. The authenticated service and browser Data workspace show the
same versioned result with provenance and its claim ceiling.

This is a deterministic interface and transformation contract exercise. It
does not claim to be a live market feed, an upstream Bazaar adapter, a price
index, seller inventory, quote, fill, compute-performance measure, or payment
input. The response is computed on demand and is not persisted.

## Exact scope

State slice: `hyperai.bazaar.data_transform_synthetic.v1`.

Exact files:

- `crates/hyperai-data/src/lib.rs`.
- `crates/hyperai-data/tests/derived_data_contract.rs`.
- `crates/hyperai-application/src/lib.rs`.
- `crates/hyperai-local-service/src/server.rs`.
- `crates/hyperai-local-service/tests/data_api.rs`.
- `apps/hyperai-interface/app.js` and `apps/hyperai-interface/app.css`.
- `docs/state-slices.md`, this packet, and
  `docs/compute-bazaar-rust-port-subplan.md`.

The browser proof is extended separately under
[`hyperai.bazaar.data_transform_browser.v1`](state-slices.md#hyperai-bazaar-data_transform_browserv1).
That continuation may edit only `scripts/verify-interface-browser.mjs` and
its named state/packet/subplan records; it asserts the existing projection's
rendered provenance and nonclaims without changing production behavior.

If implementation needs another file, amend the packet and state-slice record
before touching it. Keep the Phase 2 dataset, API, CLI, TUI, and model/view
contracts compatible.

## Source and layer contracts

- Input is only `hyperai.synthetic_market_observations_json.v1`, opened through
  the existing manifest, path, size, and digest checks. No caller-selected
  file, browser upload, network request, credential, or ignored Bazaar
  checkout is read.
- Bronze output is metadata-only: dataset ID, snapshot digest, schema digest,
  source label, row count, freshness, and claim ceiling. It contains no raw
  rows or bytes.
- Silver records use schema `hyperai.data.normalized_observation.v1`. The
  asking price remains an unsigned integer in USD micros per hour. GPU model,
  region, observation time, and source instance ID are preserved. Availability
  is `Option<u64>`; unknown remains unknown through every layer.
- Gold records use schema `hyperai.data.synthetic_group_summary.v1`, grouped
  by exact GPU model and region. They contain record count, minimum and maximum
  asking USD micros per hour, known and unknown availability record counts,
  and a known-unit total only when all grouped values are known. No currency
  conversion, freshness inference, forecast, benchmark, or live index is
  calculated.
- The release digest is domain-separated and binds method version, the exact
  Bronze snapshot, Silver records, and Gold records. Identical source bytes
  produce identical layer records and digest.
- Source freshness remains `synthetic_not_market_data`; display the
  non-claim next to results. Empty, missing, stale, or tampered input returns
  a typed error and no partial layer result.

## Allowed effects and proof gates

Allowed effects are local CPU and memory use for one verified synthetic
snapshot; one authenticated read-only API request; and rendering the compact
result in browser memory. No local or external writes occur.

Acceptance requires:

1. Stable, typed Bronze/Silver/Gold schemas and deterministic release digest.
2. Unknown availability remains null and forces the corresponding Gold total
   to remain null; known and unknown record counts remain explicit.
3. Tampered, absent, unsupported, or over-limit input fails without returning
   a partial result.
4. The service route requires existing local authentication and returns the
   same facade result; no second transformation implementation is added in JS.
5. Browser Data shows provenance, input/output digests, layer names, sample
   counts, units, and nonclaims, including on narrow viewports.
6. Focused tests and `corepack pnpm run lint` pass; JavaScript syntax and
   `git diff --check` pass.

## Boundaries and report

Non-goals: upstream code/data import, market/provider feeds, persistence of
derived layers, caller file import, changing DataFusion SQL policy, market
offer construction, connecting a derived estimate to seller quotes, Fleet
operation, evaluation, Trade, custody, escrow, payment, or publication.

Claim ceiling: deterministic derivation over one verified HyperAI synthetic
snapshot on the tested host. It does not establish source accuracy, current
availability, independent compute capacity, provider quality, market price,
correct computation, or settlement entitlement.

## Validation snapshot

Snapshot date: 2026-09-24. Baseline revision:
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the exact slice
files in the existing dirty working tree. No commit or frozen working-tree
digest was created.

- `corepack pnpm run lint` passed with the pinned Rust 1.93.1 toolchain:
  formatting, workspace Clippy, contract tests, and full workspace tests.
- Focused `hyperai-data` and `hyperai-local-service` suites passed. Tests
  cover deterministic repeat output, input tamper refusal, authenticated API
  access, Bronze/input digest binding, row counts, and null availability in
  Silver and Gold.
- `node --check apps/hyperai-interface/app.js` and `git diff --check`
  passed. Browser interaction rendered the pipeline from the authenticated
  local API. At 390 px, document/body/main widths were all 390 px, the table
  retained horizontal scrolling inside its card, unknown availability showed
  as `Unknown`, and browser console logs were empty.
- The browser service, synthetic fixture, and saved outputs used a disposable
  local home; the service was stopped, its port was confirmed closed, and the
  home was removed. No upstream files or data were read or copied.

Assurance level: `NOT_ASSIGNED`; no bounded red-team record, external review,
or clean second-environment reproduction was performed. Reviewer status:
`NOT_REQUESTED`. Reproduction: `NOT_REPRODUCED`. Fixture and output custody:
the existing HyperAI synthetic fixture plus disposable local homes and browser
memory only. The next rights gate remains an exact per-source decision for any
upstream data adapter. The current Bazaar checkout remains ignored and
reference-only.

## Current local qualification addendum (2026-09-30)

The `hyperai.bazaar.data_transform_browser.v1` continuation adds a direct
assertion for the populated Bronze/Silver/Gold view to the existing served
browser workflow. On Chrome `154.0.8037.92`, all 33 route/viewport checks
passed. Three authenticated synthetic-release GETs returned HTTP 200; the
browser retained the existing eight Data POST requests. The Bronze digest
matched the queried snapshot, all layer labels and counts rendered, Gold had
three rows and a valid output digest, unknown availability remained visibly
`Unknown`, and the accessible table and exact synthetic-only claim were
present. See the [state-slice execution record](state-slices.md#hyperai-bazaar-data_transform_browserv1)
for digests, test counts, the preserved first assertion mismatch, cleanup,
and import verification.

The locked/offline Data, application, and local-service suites passed 120
tests. The root offline `corepack pnpm run lint` passed, including the full
Rust workspace checks and tests. The bounded local synthetic implementation
and browser contract are `INTERNAL_PASS`; reviewer status is
`NOT_REQUESTED`, and clean second-environment reproduction remains
`NOT_REPRODUCED`. This result does not qualify the full Phase 2 Data slice,
screen-reader or physical-keyboard behavior, other browsers or platforms,
external sources, or production. The next source-adapter gate remains
per-source rights, classification, freshness, retention, and explicit
authorization.
