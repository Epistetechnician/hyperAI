# Economics and profit flywheel

## Revenue model

Revenue must follow delivered customer value rather than token issuance.

1. **Execution fee:** a disclosed fee on successfully settled inference.
2. **Reservation fee:** a fee on committed capacity windows, independent of
   variable execution fees.
3. **Enterprise subscription:** policy management, tenant controls, on-prem
   checking, audit retention, support, and private provider pools.
4. **Assurance fee:** pass-through verification cost plus margin for premium
   evidence and independent reproduction.
5. **Index license:** future paid access to an independently reproducible
   executed-price data product; no trading dependency.

Provider price and HyperIntel fee are separate signed fields. Hidden spread is
prohibited. Refunds and SLA credits do not generate fees.

## Unit-economics identity

For a settled request:

```text
gross revenue
  = execution fee + assurance fee + allocated subscription revenue

contribution margin
  = gross revenue
  - payment and gas cost
  - verification and evidence retention cost
  - router and gateway compute
  - customer support and SLA credit provision
  - provider-acquisition incentive
```

The router optimizes buyer expected successful cost. It does not optimize
platform revenue. Commercial policy may set a disclosed minimum fee before
routing, but cannot reorder eligible providers based on undisclosed margin.

## Compounding loop

```text
more verified buyer demand
  -> higher provider utilization and more predictable revenue
  -> more authorized providers and sharper capacity offers
  -> lower expected successful cost and better regional coverage
  -> more executed, attributable observations
  -> better reliability estimates, assurance, and purchasing forecasts
  -> more verified buyer demand
```

Two secondary loops reinforce it:

- Reservations convert recurring demand into provider capacity planning,
  improving both utilization and SLA performance.
- Portable evidence reduces buyer audit cost, increasing retention and making
  higher-assurance supply economically valuable.

## Defensibility

The moat is not a settlement chain. It is the accumulated network of authorized
capacity, normalized service manifests, attributable execution history,
policy integrations, independently replayable evidence, enterprise workflow,
and reservation liquidity.

Executed history may influence routing only through a versioned reliability
artifact whose typed identity is committed by the signed offer and whose
evaluation is reconstructed by the route and evidence bundle. No unpublished
live database state may silently change a historical route decision. The
current V3 artifact verifies supplied identity, signature, bindings, validity,
and arithmetic; it does not establish that the committed observations occurred.

## Provider and buyer incentives

- Providers earn more by improving measured successful delivery, capacity
  availability, assurance, and price—not by buying a governance token.
- Buyers receive price competition, bounded authorization, return of unused
  authorization, timeout escape, and deterministic objective remedies.
- Bonds collateralize scheduled remedies only up to posted, unencumbered
  collateral. They do not cover every loss and are not insurance or proof of
  provider quality.
- New-provider exploration budgets must be explicit, capped, and excluded from
  high-assurance traffic until evidence thresholds are met.

## Financial controls

- Separate customer funds, company operating funds, provider bonds, and fee
  revenue in accounting and contract state.
- Cap per-buyer, per-provider, per-session, and total unsettled exposure.
- Never pay twice for the same cumulative authorization digest.
- Maintain reserves for refunds, credits, chargebacks, gas, and incident costs.
- Report gross volume, settled volume, fees, credits, bad debt, concentration,
  and contribution margin separately.
- Do not describe escrow balances, bonds, or prepaid inference as deposits,
  yield, insurance, or principal-protected products without applicable approval.

## Index and hedge boundary

The first index is non-tradable and uses completed eligible fills for one fixed
service definition. Publication requires provider diversity, buyer diversity,
minimum observation depth, concentration caps, outlier policy, staleness rules,
revisions, reproducibility, and manipulation monitoring.

A HIP-3 product is a later independent business line. Its deployer stake,
oracle, liquidity, market operations, legal status, and basis risk are separate
cost centers and cannot be subsidized invisibly by customer escrow.

Official HIP-3 guidance warns that most price indices are unsuitable as perp
oracles. Feasibility review must cover roughly three-second oracle operations,
minimum update spacing, the ten-second stale-price behavior without relying on
it, manipulation resistance, liquidity, `haltTrading` settlement at the current
mark, and the irreversible consequences of cross-margin enablement. HyperCore
portfolio margin is alpha; no documented rule recognizes HyperEVM escrow,
provider bonds, or physical inference liabilities as portfolio collateral.

Primary references: [HIP-3](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-3-builder-deployed-perpetuals),
[HIP-3 deployer actions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions-1),
and [portfolio margin](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/portfolio-margin).
