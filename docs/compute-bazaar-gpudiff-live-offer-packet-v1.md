# GPUDiff Public GPU Offer Snapshot

Status: implemented and locally qualified under
[`hyperai.bazaar.gpudiff_live_offer_snapshot.v1`](state-slices.md#hyperai-bazaar-gpudiff-live-offer-snapshotv1).

## Source choice

The first real-data source is GPUDiff's public GPU offer JSON endpoint:

`GET https://gpudiff.com/api/v1/offers.json`

GPUDiff publishes this dataset as CC BY 4.0, asks consumers to cite
`gpudiff.com`, provides versioned public history, and states that every datum
links to its source. The endpoint is static JSON on a CDN, refreshed by its
publisher; individual records carry a provider/source URL and `observed_at`.
This is an aggregator feed, not a direct provider connection. Fields such as
`attrs.metric`, `attrs.stock`, and `attrs.sample_size` must be retained as
publisher-reported metadata; they are not interchangeable or independently
verified.

The first response preview exposed records with `id`, `provider`, `sku`,
`price`, `unit`, `pricing_type`, `region`, `attrs`, `provenance.url`,
`provenance.observed_at`, and `fixture`. The client accepts only the exact V1
shape and excludes/rejects any row with `fixture: true`. Source content is
hashed before parsing. The response `ETag` is a cache validator, not a
cryptographic digest.

At source selection on 2026-10-05, a read-only HEAD returned HTTP 200,
`Content-Length: 1591997`, ETag `6ac3ebb2-184abd`, and
`Last-Modified: Mon, 05 Oct 2026 18:25:54 GMT`. The implementation allows a
maximum 4 MiB response because the endpoint can change. The current HyperAI
DataFusion file ceiling remains 16 MiB. A read-only check found the regional
AWS EC2 bulk JSON file to be 481,532,363 bytes, so that file is unsuitable for
this bounded first path. Runpod's API was not selected because its current
terms prohibit automated data collection without written permission.

Source references: [GPUDiff API and data license](https://gpudiff.com/api/),
[current offer feed](https://gpudiff.com/api/v1/offers.json), and
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

## Import contract

An owner action in the local HyperAI browser calls one fixed local refresh
route. That handler performs at most one HTTPS GET to the endpoint above,
without credentials, request payload, redirect, or automatic retry. It checks
status, content type, declared and observed body length, schema, record count,
and the fixture marker before writing anything. On fetch or parse failure,
the currently cataloged snapshot remains readable.

The import stores the exact response bytes under a path derived from their
SHA-256, and stores normalized JSON Lines for the existing manifest-verified
DataFusion reader. Every normalized row retains the source record ID,
provider, SKU, model/spec fields, original price and unit, pricing type,
region, attributes, provenance URL and observed timestamp, and raw feed hash.
Price-to-micros conversion uses decimal text and integer arithmetic. Missing
and unknown values remain missing or source-labelled; they are never filled
with zero or a guessed value.

The Data catalog exposes one current GPUDiff dataset ID. Each new feed digest
creates a new immutable normalized path and source receipt, then atomically
advances the manifest pointer. Previous captures remain in the selected local
HyperAI home. At most 16 distinct raw captures are retained; a seventeenth
unique capture fails closed until a separately authorized retention change.
Identical bytes are idempotent. The standard catalog, saved model freshness
checks, read-only SQL limits, browser/CLI/TUI query paths, and service
authentication remain in force. ACP/MCP receives no refresh capability.

Refresh is manual and on demand. It is not a persistent streaming connection,
scheduled poller, provider offer validator, market index, or trade signal.

## Validation and claim ceiling

Local parser tests use a small fixed contract fixture and do not call the
network. The live end-to-end check uses one real request through the owner
route in a temporary HyperAI home, then lists and queries that exact dataset
through the existing Data API. Retain the response status, source and
normalized digests, row count, observation-time range, and bounded query
count; do not write the live body to the repository or test logs.

Successful local checks support only `INTERNAL_PASS` for this exact snapshot
import/query scope. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. The claim ceiling is GPUDiff-attributed public price data
with source-provided timestamps. It does not establish direct provider access,
an executable offer, current or guaranteed capacity, seller identity,
computation correctness, buyer eligibility, settlement, payment entitlement,
or production readiness. Keep this dataset advisory and separate from
`hyperai-market` deal and outcome records.

## Implementation and live result

Assessed revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the scoped,
uncommitted working-tree changes named in
[`docs/state-slices.md`](state-slices.md#hyperai-bazaar-gpudiff-live-offer-snapshotv1).
Assurance: `INTERNAL_PASS` for this import/catalog/query path only. Reviewer:
`NOT_REQUESTED`. Reproduction: `NOT_REPRODUCED`. The exact source bytes and
temporary HyperAI home were removed with the ignored live test's temporary
directory; no live payload was written into the repository or test output.

The final ignored live E2E test made one authenticated owner-route source
request and then queried the same snapshot through the catalog and Data API:

- Refresh route: HTTP 200; unauthenticated route request: HTTP 401.
- Raw response: 1,591,997 bytes; SHA-256
  `cac8685e605e0c5fa3f39506b7a024fdb8f6590658329b701dc55b360b16a932`.
- Normalized DataFusion snapshot SHA-256
  `1f9e6eb56fd92c805015ebc6c9df5080e8f417aa6aff58051176c961b6f10c09`.
- 2,673 non-fixture rows; source timestamps ranged from
  `2026-10-03T06:31:07Z` to `2026-10-05T18:25:39Z`.
- Authenticated catalog digest matched the import receipt; DataFusion returned
  5 bounded rows with HTTP 200 and the same normalized digest.

The first query attempt hit the existing maximum-row contract because the SQL
did not include an explicit `LIMIT`; the final UI default and E2E SQL both
include a limit. The existing two-second, 1,000-row, and 256 KiB query bounds
remain unchanged. The source refresh is a manual snapshot read, not a
continuous stream. The next authority gate is any scheduled refresh, direct
provider connection, use as a market/quote signal, paid execution, publication,
or production deployment; each remains outside this slice.

## Owner-controlled scheduled refresh

The follow-on state slice is
`hyperai.bazaar.gpudiff_owner_scheduled_refresh.v1`. An explicit owner action
starts one immediate refresh and then one fixed-source request every six hours
while the local HyperAI service remains running. A separate owner action
stops the schedule; service shutdown also stops it, and restart does not
resume it. Status is held in memory and is available only through the
authenticated owner route.

Every attempt keeps the existing ten-second deadline, fixed HTTPS URL,
no-redirect/no-retry policy, 4 MiB response ceiling, and digest-bound local
import. Transient failures leave the current catalog entry intact and wait
for the next six-hour tick. At the existing 16-distinct-capture limit, the
schedule stops and reports capacity; it does not delete or overwrite prior
source bodies, receipts, or snapshots. The owner may still use manual refresh.
The three schedule routes are not exposed through ACP/MCP.

This schedule reads public publisher data and uses local CPU; it does not
dispatch or benchmark GPU workloads. No H100/Gman resource is required for
this slice. The data remains advisory and separate from Market offers, quotes,
provider execution, settlement, public publication, and production.

## Validation and current local stream (2026-10-06)

Baseline: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
uncommitted working-tree changes for
`hyperai.bazaar.gpudiff_owner_scheduled_refresh.v1`. Assurance:
`INTERNAL_PASS`. Reviewer: `NOT_REQUESTED`. Reproduction:
`NOT_REPRODUCED`.

Focused qualification passed: GPUDiff data contract tests 3/3, local-service
data API tests 8/8, owner-route denial test, local-service compilation,
formatting, 34 interface checks, import-manifest verification, and whitespace
checks. The two additional live tests each used a disposable local home,
started and stopped the owner schedule, and removed the temporary home. The
API path queried 5 rows; the Chromium Data workspace path queried 100 rows and
verified the visible `LOCAL · GPUDIFF SNAPSHOT` indicator and CC BY 4.0
attribution. The browser audit returned HTTP 200 for its start, query, and stop
POSTs, checked 33 route/viewport combinations, and closed its temporary
service, browser, ports, and state.

Both live test homes imported 2,687 rows. Publisher observation timestamps
ran from `2026-10-03T06:31:07Z` through `2026-10-06T07:33:04Z`. The raw source
SHA-256 was
`0608188132bf8ac530e5f2d6ee734d9e93066e20dc8c9920891cdb789be5f4c4`; the
normalized snapshot SHA-256 was
`32afab390c6788aae8f127f6472186dead60dd46e7a0f605e35029a914244401`.

After those temporary runs, the schedule was started in the owner's
platform-default HyperAI home and remains active as of 2026-10-06 13:49 UTC.
It is configured for a 21,600-second interval and currently retains one
distinct capture. The local raw and normalized files were re-hashed in place
and matched the digests above; a bounded five-row catalog query returned
HTTP 200. Data/artifact custody is the owner's local HyperAI home. No raw feed
body was copied into this repository or printed. This feed and the query use
local CPU; no H100 or Gman resource was used. This run verified the immediate
fetch and active schedule state but did not wait a full six-hour interval for
a second scheduled fetch. Service-shutdown cleanup and scheduler behavior at
the 16-capture limit were not exercised against the persistent owner home.

The evidence ceiling remains GPUDiff-attributed public snapshots with local
bounded queries. It does not establish direct provider access, capacity,
price representativeness, computation correctness, payment entitlement, or
production readiness. The next authority gate is any direct provider,
executable-market, compute, payment, publication, or production use, each of
which needs a separate named slice and authority.
