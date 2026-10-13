# HyperAI Compute Market Explorer PRD

Status: draft; product and implementation plan only.
State slice: hyperai.compute_market.explorer_prd.v1.
Planning baseline: HyperAI dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa on main,
with the pre-existing dirty and untracked working tree preserved.
Upstream Desk reference: gustofied/desk at 2ac5d9b24377b21e0d3f928f1f09323dc5785fff,
the main-branch revision inspected on 2026-09-25.

This PRD proposes a compute-market discovery and analysis workspace inside
HyperAI. It does not authorize a code fork, source import, data ingestion,
provider connection, external publication, or production use.

## Executive summary

Build a Hyperliquid/Hyperdash-inspired explorer for compute supply and
market context. Users should be able to move from a market overview to a GPU
or service detail page, compare source-attributed observations, inspect
coverage and methodology, and carry a shortlist into HyperAI's existing local
synthetic quote flow.

The product borrows the market-universe, detail-page, watchlist, and
provenance-inspector interaction patterns. It does not assume compute has
crypto-style trades, funding, open interest, liquidations, or an order book.
Compute offers are heterogeneous listings and snapshots. They become
comparable only when their hardware, region, capacity, term, price basis, and
observation time are compatible.

The user's full source list is the initial entity directory, not a claim that
every entry has a live connector. The directory spans venues and marketplaces,
provider clouds, research publishers, data-center and power infrastructure,
software products, aggregators, and crypto compute networks. Each entity
must show its type and connection status.

## Product decision

Make the HyperAI Market workspace the buyer-facing explorer and use the
existing Data contracts for bounded, source-labelled datasets. Keep the
compute-market deal lifecycle authoritative for buyer requests, quotes,
approvals, and receipts. Market observations remain advisory and must not
silently become seller offers, buyer approvals, completion evidence, or
payment claims.

Use Desk as a conditional UI-code fork candidate, not as a data authority or
runtime dependency. Its README describes a composable compute-data workspace
with comparison and market-depth/history views. At the pinned revision above,
the root LICENSE path returns 404 and package.json declares a private Node 22
ES-module application with D3, GSAP, Motion, esbuild, and npm-based scripts.
No source code, generated bundles, visual assets, datasets, provider refresh
scripts, or Desk schemas may be copied until exact reuse and redistribution
rights are documented. See the [Desk repository](https://github.com/gustofied/desk),
[pinned README](https://github.com/gustofied/desk/blob/2ac5d9b24377b21e0d3f928f1f09323dc5785fff/README.md),
and [pinned package manifest](https://github.com/gustofied/desk/blob/2ac5d9b24377b21e0d3f928f1f09323dc5785fff/package.json).

HyperAI already has a local synthetic Market/Data path and an independently
authored interface. This PRD extends that direction and does not supersede
the existing [Compute Desk interface plan](bazaar-hyperai-interface-plan.md),
[market architecture](compute-market-architecture.md), or their state slices.
The Compute Bazaar is a separate source with separate rights; its current
reference-only boundary is unchanged.

## Users and jobs

| User | Job to be done |
|---|---|
| Compute buyer or ML platform engineer | Find and compare suitable capacity for a workload, region, time window, and budget. |
| Market analyst or researcher | Understand price dispersion, advertised supply, source coverage, and changes over time without confusing estimates with transactions. |
| Infrastructure or investment researcher | Follow providers, facilities, power, and research references while seeing which facts are direct observations and which are editorial context. |

## Goals

- Catalog every company, venue, network, or reference source in the supplied list with a stable identity, category, canonical URL when known, and explicit connection status.
- Explore compute instruments such as H100 GPU-hour by region, provider, configuration, availability class, and billing term.
- Compare compatible observations with visible units, timestamps, source attribution, sample counts, and missing-data state.
- Show stale, incomplete, conflicting, or unsupported inputs rather than hiding them behind a single score.
- Provide provider/source detail pages and a provenance inspector for every displayed metric.
- Let a buyer save a local shortlist and open the existing synthetic quote/deal review flow without promoting an observation into an offer.
- Keep synthetic/offline behavior useful while no live source is authorized.

## Non-goals

- Trading, copy trading, wallet connection, token execution, derivatives, or
  financial advice.
- Calling every listed website, scraping gated data, or claiming all entries
  are connected.
- Presenting a synthetic or modeled capacity curve as a live order book.
- Making price forecasts, universal provider rankings, quality scores, or
  compute-equivalence claims across incompatible billing units.
- Provider signup, live quotes, job dispatch, SSH, paid compute, custody,
  escrow, settlement, or production deployment in the explorer slice.
- Reusing Desk code, data, schemas, generated output, or assets before the
  rights gate passes.

## Information architecture

1. **Market overview.** GPU and service instruments, recent comparable price
   bands, observed supply, number of contributing sources, freshness, and
   coverage gaps. Every card links to its calculation method and included
   observations.
2. **Instrument detail.** Price history and dispersion, provider/region
   comparison, terms and availability filters, advertised capacity where
   supported, and a list of source observations. Label price bands as modeled
   or observed and show the sample count.
3. **Provider or venue detail.** Identity aliases, supported hardware and
   regions, offer types, observed terms, source status, and source-specific
   freshness and limitations.
4. **Research and company directory.** Company facts, reports, facility
   announcements, and methodology references remain typed reference records;
   they do not enter GPU offer calculations without an explicit structured
   measurement contract.
5. **Coverage directory.** Search and filter the full user-supplied entity
   list by category, source type, rights status, connection maturity, last
   checked time, and available instruments.
6. **Compare and shortlist.** Compare only offers with compatible dimensions,
   retain incomparable fields as unknown, and pass a user-selected shortlist
   into a local synthetic request/quote review.
7. **Provenance inspector.** Reveal source URL or capture identity, retrieval
   and observation times, exact snapshot digest, parser/normalizer version,
   calculation version, units, exclusions, and claim limits.

Use the existing HyperAI navigation and local connection-status vocabulary.
The visual design may adopt a dense terminal layout, keyboard navigation,
saved views, and configurable panels after the base workflows are usable.
Chart motion and decorative treatment are optional; source readability,
accessibility, and data clarity take priority.

## Data model and calculation rules

Use distinct versioned records. Initial conceptual types:

- **DirectoryEntity:** canonical entity ID, display name, aliases, category,
  entity kind, canonical URL, owner, rights/access status, source status, and
  last-checked time.
- **ComputeInstrument:** accelerator model and variant, memory/configuration,
  region, supply class, billing unit, and compatibility dimensions.
- **OfferObservation:** source and seller identity as reported, instrument
  reference, price and currency, price basis, capacity and capacity unit,
  term, region, availability class, observed_at, retrieved_at, exact source
  snapshot digest, and parse status.
- **MarketStatistic:** metric and unit, compatible input set, method/version,
  time window, sample count, exclusions, calculated_at, and uncertainty or
  explicit unknown state.
- **ResearchReference:** publisher, title, publication time, URL or
  caller-supplied capture identity, topic tags, and any permitted excerpt or
  digest. Research claims remain attributed to the source.
- **CompanyFact / NetworkMarketState:** separately typed records for
  infrastructure facts and crypto-network/token observations. Neither may be
  coerced into an OfferObservation.

Never merge GPU-hour, host-hour, serverless request, token price, or
million-token API pricing into one unit. Partition derived price distributions
by hardware/configuration, region, billing basis, availability class, and
term. Label any remaining mismatch. Use median and percentile bands only when
the sample is large enough under a documented method; otherwise show raw
observations and an insufficient-sample state. Count duplicate and excluded
rows. Do not interpolate missing observations into apparent source data.

Treat seller asking price, Desk-derived or HyperAI-derived index, a firm quote,
an executed fill, and a metered cost as separate record classes. Advertised
capacity is not confirmed availability. A provider signature or telemetry
record attributes a statement but does not independently prove hardware,
correct computation, or payment entitlement.

## Source and coverage policy

Initialize the directory from the user-provided list and classify entries
before attempting ingestion. Examples of categories include:

- Marketplaces and exchanges: Compute Exchange, Ornn, Squaretower, FPX, Stoa
  Exchange, and related venues.
- GPU providers and neoclouds: CoreWeave, Lambda, Crusoe, Nebius, RunPod,
  Vast.ai, OVHcloud, and related providers.
- Research and market intelligence: Epoch AI, SemiAnalysis, Artificial
  Analysis, Synergy Research, and other publishers.
- Aggregators and directories: GPUs.io, GetDeploying, Deploybase, and similar
  discovery surfaces.
- Data-center, energy, and infrastructure: Nscale, Giga Energy, WhiteFiber,
  and related operators.
- Inference and software: Parasail, DeepInfra, Modal, BentoML, dstack, and
  related services.
- Crypto or decentralized compute: Bittensor, io.net, Aethir, Fluence,
  Aleph Cloud, and related networks.

The seed directory must preserve every submitted name, even when a canonical
identity or URL is unresolved. Do not collapse similarly named companies or
products without evidence. Connection states should distinguish at least:
reference only, rights review, user-supplied capture, synthetic fixture,
read-only connected, stale, unavailable, and retired. “Listed in the
directory” never means “data connected.”

Every live connector requires an individual authorization and source record:
approved endpoint/access method, data and redistribution rights, fields,
schema, rate/freshness limits, retention, privacy, exact source revision or
capture digest, parser version, and stop conditions. Prefer public APIs,
official downloads, and user-provided captures where permitted. No connector
may crawl, evade access controls, or copy a source's published dataset into
HyperAI by default.

## Architecture and fork strategy

### HyperAI boundary

The browser should read through the existing HyperAI application facade and
authenticated loopback service. The Data workspace owns bounded local
catalog/query operations and source-labelled derived datasets. The Market
domain owns buyer requests, seller quotes, approvals, and deal records.
Connectors and ingestion are new, separately scoped work; do not add network
fetching to the current synthetic Data or Market modules.

Use the following flow:

Entity directory -> authorized source adapters or caller captures ->
immutable source snapshots -> normalized typed observations -> versioned
statistics -> explorer pages -> user shortlist -> existing local synthetic
quote/deal flow.

The selected observation IDs and digests may be attached as advisory context
to a local request. Existing market eligibility must still consume a valid
seller offer under its own contract. A market statistic must never satisfy
seller signature, buyer approval, fulfillment, verifier, or settlement fields.

### Conditional Desk fork

| Option | Benefit | Constraint | Decision |
|---|---|---|---|
| Maintain an owner-controlled Desk fork | Reuses the existing market-view and chart implementation and keeps upstream changes reviewable. | Blocked until code, asset, data, and dependency rights are explicit; also needs a pnpm and HyperAI API integration spike. | Preferred only if the rights gate passes and the spike is maintainable. |
| Independently implement the selected interaction patterns | Fits the current shell, facade, local service, and HyperAI data contracts directly. | Recreates some presentation and chart behavior. | Safe fallback; proceed with synthetic data if Desk rights remain unresolved. |
| Link to or embed hosted Desk | Lowest initial integration effort. | No demonstrated HyperAI API, identity, provenance, or data contract; hosting and access remain outside HyperAI control. | Not the product integration path. |

**Recommendation: fork only after a source-rights gate, and keep the fork
isolated from HyperAI's domain contracts.**

1. Obtain written permission or an explicit license covering the exact Desk
   source revision and intended modification, redistribution, hosting, and
   commercial use. Confirm separate rights for bundled data, generated
   snapshots, artwork, fonts, brand marks, and third-party contributions.
   Record the grant and exclusions before copying any bytes.
2. Audit the exact tree and dependency licenses. Pin the accepted upstream
   commit, enumerate imported and excluded files, retain required notices, and
   produce a source digest and dependency inventory. Do not assume the
   repository being public grants a reuse license.
3. If rights pass, create an owner-controlled GitHub fork with an upstream
   remote and a documented patch/update policy. Import the approved UI code as
   a separate frontend workspace under products/compute-market-explorer/.
   Keep its package/build graph separate from root Rust crates and from
   apps/hyperai-interface until the integration spike succeeds.
4. Adapt the fork behind a narrow read-only HyperAI API client. Preserve the
   upstream commit and HyperAI patch history. Use pnpm and a pnpm lockfile in
   accordance with HyperAI repository rules; do not run the upstream npm
   commands. Retain only dependencies and UI modules needed by the explorer.
5. Exclude Desk's CLI, provider-refresh jobs, raw and generated datasets,
   example analytics, deploy workflow, and unreviewed assets from the initial
   integration. Re-add any item only after its rights, behavior, and data
   boundary are reviewed.
6. Make upstream sync a deliberate reviewed change. Each update must recheck
   the source license/permission, dependency tree, changed source files,
   datasets, generated outputs, and build/deployment effects.

If the rights gate does not pass, implement the selected interactions
independently in the existing HyperAI interface using synthetic fixtures.
Do not copy Desk source, assets, or distinctive expression as a workaround.
The product can proceed without the fork; the fork is an implementation
option, not a dependency for the market-data contract.

## Delivery plan

| Phase | State slice proposal | Deliverable and exit gate |
|---|---|---|
| 0. Rights and fit | hyperai.compute_market.explorer_rights_review.v1 | Resolve source license/permission, permitted files and data, dependency rights, fork owner, and integration form. Until it passes, no Desk source bytes enter a branch or workspace. |
| 1. Directory and synthetic prototype | hyperai.compute_market.explorer_synthetic.v1 | Catalog the entire submitted universe with types and honest connection states. Build one synthetic H100 overview/detail/compare flow and provenance inspector in the current local workspace. No network. |
| 2. Fork integration spike, conditional | hyperai.compute_market.explorer_desk_fork.v1 | If Phase 0 passes, build the isolated fork workspace against synthetic HyperAI API responses; demonstrate pnpm build, keyboard/mobile use, license notices, no provider calls, and a maintainable upstream update path. Otherwise use the independently authored prototype. |
| 3. First read-only source | Separate hyperai.compute_market.explorer_source_<name>.v1 slice | Connect one rights-cleared source for one instrument. Freeze endpoint/schema, source snapshot, transform, freshness, retention, and stop conditions. Keep results advisory; no seller eligibility or live quote. |
| 4. Comparable coverage | Separate source slices | Add sources one at a time; qualify unit mapping, aliases, duplicate handling, staleness, outliers, and conflicting claims. Publish coverage and limitations before adding more instruments. |
| 5. Workload shortlist | hyperai.compute_market.explorer_shortlist.v1 | Map a buyer's hardware/region/time/budget constraints to explainable candidates and open the existing local synthetic quote flow. No dispatch or spend. |
| 6. Live quote or execution | New authority-specific slices | Only after separate owner/operator approval, provider contracts, identity, spend limits, privacy, cancel/retry, and receipt controls are frozen. This PRD does not authorize these effects. |

## Acceptance criteria for the first usable slice

- Every supplied entity is present in the directory or explicitly recorded as
  unresolved; no unconnected entity is shown as a live source.
- A user can search/filter the directory and distinguish providers, venues,
  research, infrastructure, software, aggregators, and networks.
- A synthetic H100 instrument page shows comparable price basis, region,
  provider, capacity unit, observation time, source count, and provenance.
- Incompatible price bases are separated or labeled incomparable. Missing,
  stale, duplicate, and unsupported records remain visible.
- A chart can be traced to exact observation IDs, snapshot digests, and a
  versioned calculation method.
- The compare/shortlist flow creates no external request and hands off only to
  the existing local synthetic market path.
- The interface has keyboard-operable search, filters, tables, details, and
  provenance; the desktop and narrow viewport retain readable data labels.
- No provider, registry, Bazaar, model, agent, wallet, chain, or public-network
  request occurs in the synthetic slice.
- If a Desk fork is used, the approved commit, license/permission evidence,
  file manifest, third-party notices, dependency inventory, and pnpm build
  contract are recorded before merge.

## Success measures

- Coverage: fraction of directory entities with validated canonical identity,
  category, URL, and known access/rights status.
- Data quality: fraction of displayed observations with known units, source,
  observation time, and parse status.
- Comparability: fraction of instrument rows included in a metric after
  explicit compatibility filters, with excluded counts visible.
- Freshness: age distribution of displayed observations by source and
  instrument, not only a provider's self-reported status label.
- Workflow: users can find an instrument, understand why two offers compare
  or do not compare, inspect the underlying provenance, and produce a local
  shortlist without losing source identity.

Do not optimize for a larger connected-provider count if rights, freshness,
normalization, or provenance quality are unresolved.

## Risks and unresolved decisions

- Who can grant reuse rights for Desk code, assets, bundled data, and
  third-party contributions, and what uses the grant covers?
- Is the desired integration a separately maintained frontend workspace or
  should approved modules be ported into the current HyperAI shell after the
  spike?
- Which first read-only source offers the clearest rights, stable schema, and
  useful H100/H200 coverage?
- Which workload and procurement dimensions are required for the initial
  shortlist (region, term, data location, interconnect, minimum capacity)?
- Which source categories are directory-only versus candidates for structured
  market data?
- What freshness thresholds are meaningful per source and instrument?

## Authority and publication boundary

This PRD is a local planning artifact. It does not grant a Desk license, create
a GitHub fork, authorize a source import, connect any data feed, permit paid
execution, or authorize public release. The next decision gate is written
rights/permission for exact Desk materials. Any code, data, provider, external
publication, and production work needs its own named state slice and
owner/operator authority.
