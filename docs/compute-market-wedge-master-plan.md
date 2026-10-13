# HyperAI Compute Market Wedge: Master Plan

- Date: 2026-09-24
- State slice: `hyperai.compute_market.wedge_plan.v1`
- Status: planning only. This document does not authorize implementation,
  provider access, paid execution, remote workers, data transfer, federation,
  settlement, publication, or deployment.

## Decision

Build HyperAI toward a buyer-controlled workload exchange that can compare and
eventually dispatch eligible AI jobs across three supply classes:

1. Owner-opted-in local machines and workstations.
2. Independent datacenters and GPU clouds, including capacity marketplaces.
3. Mainstream hosted model and AI service APIs.

The buyer describes a workload, budget, deadline, data boundary, and acceptance
checks. HyperAI discovers eligible offers, produces a workload-specific route
and quote, executes only under the applicable authorization, and returns a
portable record of cost, performance, provenance, and evaluation results.

The product sells **lower, more predictable fully loaded cost per
customer-accepted workload**, with buyer control over where and how work runs.
Provider listing price, GPU-hours, and token prices remain useful inputs; they
are not interchangeable measures of completed work.

## First customer and wedge

Target AI product and research teams with repeatable, bursty batch inference
and model-evaluation jobs that already move between local GPUs, rented
datacenter capacity, and hosted model APIs.

Start with **batch inference and evaluation**. A customer supplies a bounded
job description, data location or authorized artifact reference, task-specific
evaluator, and policy. The service compares eligible execution paths and
reports the measured fully loaded cost per accepted result. The first useful
experience can recommend a route before any external execution is enabled.

This wedge is portable, queueable, measurable, and compatible with both spare
local capacity and rented or managed services. After the economics and
reliability are demonstrated, expand to fine-tuning, recurring inference, and
interactive serving under separately qualified workload contracts.

## Customer experience

1. **Describe the job.** Select a workload type and version, required model or
   runtime properties, data handling, region, deadline, spend cap, and
   customer-owned acceptance checks.
2. **Choose eligible supply.** Include or exclude owner devices, datacenter
   offers, hosted model APIs, and named providers. Local supply is off until
   its owner explicitly grants a bounded resource and time window.
3. **Compare a quote.** See price basis, known capacity, expected latency and
   throughput, measurement age, data movement, failure/retry terms, and the
   provenance and limits of each estimate. Missing values stay unknown.
4. **Approve and dispatch.** The buyer approves the exact job and quote under
   the spend and data policies. Any material change to the job, price, provider,
   or execution boundary requires a new preview and approval.
5. **Evaluate and reconcile.** Run the customer-selected evaluator against the
   returned result. Record accepted, rejected, incomplete, disputed, and
   unknown outcomes distinctly.
6. **Export the receipt.** Provide a portable record binding the job, offer,
   quote, approval, execution identity, metering basis, result commitment, and
   evaluation evidence. Keep private payloads outside public listings and
   receipts.

The service can later automate routing within a buyer's explicit policy.
Retries, fallback, provider substitution, and added spend must be visible and
bounded by that policy.

## Why this can stand out

Generic routing and raw GPU supply already exist. OpenRouter routes hosted
model calls using price, throughput, latency, provider health, and fallback
behavior. GPU platforms such as RunPod offer cloud GPUs and serverless
inference. HyperAI's opening is to combine buyer-controlled local capacity,
datacenter compute, and hosted services around a portable workload contract,
then compare their **observed cost for the buyer's accepted job** rather than
presenting another provider list. See [OpenRouter latency and performance
routing](https://openrouter.ai/docs/guides/best-practices/latency-and-performance)
and [RunPod Serverless](https://www.runpod.io/product/serverless).

Other adjacent work includes CME/Silicon Data rental-price futures and an SSRN
proposal for benchmark-derived AI service units. These address price hedging or
market design; they do not establish the complete local-to-datacenter-to-hosted
workload exchange described here. This is a bounded scan of public examples,
not evidence that no other team is building a broader aggregator. See [CME
Compute Futures](https://www.cmegroup.com/markets/energy/power/compute-futures.html)
and [Financializing Compute](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=6737898).

The differentiator is the control and evidence path across supply types:

- Owner policy and revocation for local resources.
- Workload-specific price and performance observations, with units, source,
  timestamp, uncertainty, and missing data visible.
- A clear separation between raw capacity offers and managed AI services.
- Buyer-specified task acceptance and evidence-bound execution receipts.
- Portable policy and records that reduce provider lock-in.

The commercial hypothesis is a subscription for control, cost analysis, and
workflow management, with a transaction fee for successfully coordinated jobs.
Validate willingness to pay and marketplace liquidity before fixing fees.
Keep the first service a software aggregator: providers retain execution and
billing, buyers retain their provider accounts and policy, and HyperAI does not
need to custody credentials, money, or a GPU fleet. Consider integrated
settlement only after the routing service proves value and its own authority,
dispute, and reconciliation gates pass.

## Market and research evidence

### Compute-depth claims

Treat depth as an architecture-conditioned research variable, not an established
universal scaling rule. The Q Labs September 2026 paper reports improved
compute-loss scaling from model growth and boundary operators, and its post
reports loss reduction from 4 to 128 layers in a fixed-width, 1B-token run that
uses a boundary operator. Its larger-scale conclusions include extrapolation.
The January 2026 *Depth Delusion* arXiv preprint reports the opposite tendency
for its tested depth/width family, including a 7B comparison where its
64-layer model loses to a similar-size 32-layer model. These findings are
regime- and architecture-specific and motivate matched experiments rather than
a general “deeper” or “shallower” rule.

References: [Q Labs paper](https://arxiv.org/abs/2609.19107), [Q Labs
summary](https://qlabs.sh/depth), and [The Depth Delusion](https://arxiv.org/abs/2601.20994).

Neither line of work establishes that deeper models improve alignment or that
they break most interpretability methods. Latent serial computation may reduce
what a chain-of-thought-only monitor can observe. Mechanistic interpretability,
causal intervention, monitorability, and behavioral alignment require separate
measurements. The Q Labs post itself presents chain-of-thought monitorability
as a separate axis; its computational-depth proxy does not measure circuit
interpretability or deceptive-alignment detection.

Use the AHCL research distinction between representation, circuit,
intervention, and behavior measures: stable task behavior does not establish
mechanistic continuity or preserved alignment. Use the Composed ZK Benchmark
OS discipline to freeze evaluator identity, controls, custody, and claim scope;
local replay or a signed receipt is not scientific proof or independent
acceptance.

### Required depth and oversight experiment

Before architecture-specific quality signals affect routing, compare width-
heavy and depth-heavy baselines with boundary-operator, tied-loop, and
model-growth variants under matched compute and architecture-specific tuning.
Measure training FLOPs, wall-clock time, energy, memory, and serving latency.
Report the following outcome families separately:

- Capability and customer-task acceptance.
- Cost, throughput, latency, availability, and failure rate.
- Chain-of-thought coverage, legibility, and adversarial evasion.
- Representation, circuit, and causal-intervention continuity.
- Behavior on held-out, adversarial alignment and reliability evaluations.

Use frozen model, data, tokenizer, evaluator, budget, and control identities.
Keep held-out evaluations sealed where required. Do not collapse results into a
single alignment or “effective compute” score. Until measured and reviewed,
depth/interpretability findings are research context only and do not change
provider eligibility or pricing.

## Offer, evidence, and ranking model

Keep two offer families distinct:

| Offer family | Examples | Comparable fields |
|---|---|---|
| Raw compute | Local GPU reservation, datacenter GPU-hours, cluster job | Hardware identity, memory, interconnect, region, reservation window, price basis, utilization/availability evidence, workload benchmark results |
| Managed AI service | Hosted model API, serverless inference endpoint, managed evaluation service | Model/service identity, input/output price basis, limits, region/data policy, measured latency/throughput/availability, task-specific evaluation evidence |

Normalize only at the workload comparison boundary. If both offers can run the
same authorized job and evaluator, compare total cost, task outcome, latency,
data movement, and constraints. Do not convert unrelated raw hardware and API
offers into a single generic compute unit.

Treat alignment and reliability as attributed, task- and model-specific
evidence with a measurement date, protocol, uncertainty, and scope. Buyers may
apply explicit thresholds or filters. HyperAI must not issue a universal
alignment score or claim that hardware capacity itself is aligned. A seller
signature or telemetry record attributes a statement; it does not prove
correct execution, model safety, or payment entitlement.

Dynamic pricing should distinguish live seller terms, observed asking prices,
completed quotes and fills, metered costs, and model-estimated future prices. A
time-sensitive price or availability index is advisory; it cannot silently
change a binding quote, buyer approval, or settlement amount. Compare older and
newer accelerators through versioned, workload-specific measured outcomes and
their full costs, including utilization, power, networking, and data movement.
Do not equate peak FLOPs, GPU generation, or posted hourly price with useful
completed work.

## Portfolio ownership

- **HyperAI Market** owns cross-provider job/offer composition, buyer policy,
  quote and approval records, deal state, and portable receipts. Compute-market
  records stay separate from outcome and HIP-4 settlement records.
- **Simple Compute Market** retains its registry, buyer, seller storefront,
  negotiation, and seller-owned fulfillment contracts. Integrate through
  versioned adapters; preserve its independent workspace and license boundary.
- **HyperIntel** retains inference-specific policy, routing, execution lineage,
  and reconciliation. HyperAI composes through reviewed contracts and adapters
  without duplicating or taking over HyperIntel's execution authority.
- **Local provider agent** reports opted-in hardware and accepts only bounded,
  owner-authorized jobs. It has separate sandbox, privacy, resource, and
  cancellation contracts.
- **Datacenter and hosted-service adapters** retain their provider-specific
  identity, terms, metering, data-retention, and failure semantics.
- **JustAI** may receive an optional metadata-only evidence projection after its
  owner, charter, disclosure, retention, and review gates. It is not the market
  ledger, execution authority, payment authority, or evidence custodian.
- **ESN-JustNet** may later provide buyer-owned consent, preferences, spend
  limits, revocation, and portable receipt references through an owner-approved
  adapter. It does not take over HyperIntel execution or evaluator authority.
- **Outcome protocol** is used only when an objective job-completion claim maps
  to its frozen verifier contract. Subjective quality remains in the market's
  own evaluation and dispute records.

Keep The Compute Bazaar ignored and reference-only until rights are resolved.
The wedge can proceed with independently authored contracts and synthetic
data.

## Delivery plan and gates

The sequence below is strategic. Every implementation or external effect still
requires its own named state slice and exact authorization.

| Stage | Deliverable | Exit condition |
|---|---|---|
| 0. Validate the wedge | Buyer/job interviews and a frozen batch-inference/evaluation job contract; synthetic walkthrough across typed local, datacenter, and hosted offers | Buyer pain, evaluator ownership, data boundary, target workload, and fully loaded cost metric are explicit. No provider calls or real payloads. |
| 1. Local proof of flow | Reuse the current synthetic quote, buyer approval, mock fulfillment, and receipt; add workload-specific route comparison only under a new approved local slice | Deterministic inputs; missing values preserved; policy/quote/job digests bound; no external effects. |
| 2. Catalog and observations | Add separately approved read-only adapters for one source in each offer family; record terms, provenance, units, timestamps, and freshness | Exact endpoint, key/trust pins, schema, retention, replay/freshness, and network policy are reviewed per adapter. Listings remain advisory until a deal contract admits them. |
| 3. Shadow routing | Compare suggested routes against a fixed workload and evaluator without dispatching jobs | Paired workload results, observed costs, uncertainty, failure paths, privacy boundary, and route changes are independently auditable. No hidden provider call or spend. |
| 4. Capped execution pilot | Separately authorize one local execution adapter, one datacenter adapter, and one hosted-service adapter, one at a time | Exact code/runtime/provider, job/data scope, credentials, hard spend cap, cancellation/retry behavior, receipt, and incident path are frozen and reviewed. User approval binds to each exact quote. |
| 5. Broader exchange | Add supplier onboarding, multi-provider routing, reconciliation, dispute, and settlement adapters where justified | Demonstrated demand and liquidity; seller identity and revocation; metering and dispute evidence; legal/privacy/security review; independent review where required; distinct production authorization. |

The current repository already has local synthetic market contracts and
receipts, an offline Simple Compute Market capture adapter, and a Rust compute
desk plan for Data, Fleet, Eval, and Trade. This wedge plan selects their
product focus; it does not mark those later workstreams complete or authorize
their external gates. These wedge stages do not replace detailed-plan phases:
paid execution must satisfy the existing paid-beta/production gate, testnet
settlement its exact contract and write gate, and every provider adapter its
own state slice. See the [compute-market refactor plan](compute-market-refactor-plan.md)
and [Rust compute desk subplan](compute-bazaar-rust-port-subplan.md).

## Success measures

Primary buyer value:

```text
fully loaded cost per accepted job
  = provider charges + retries + data movement + storage + orchestration + evaluation
    ---------------------------------------------------------------------------
                          customer-accepted outputs
```

Always report the denominator and incomplete, failed, disputed, and unknown
jobs. Missing prices or telemetry remain unknown, never zero. Track alongside
the primary measure: eligible-route coverage, acceptance rate, latency,
throughput, availability, retry rate, policy-compliant data location, spend-cap
adherence, and repeat buyer use. Any claim of cost savings needs the same frozen
workload and evaluator on a baseline route. If no outputs are accepted, report
the cost-per-accepted-job metric as undefined and show spend and failures
separately.

Scientific outcomes are separate from commercial acceptance. A customer
accepting a result does not establish general model correctness, safety, or
alignment.

## Non-goals and open decisions

This wedge does not make the first release a permissionless public GPU network,
an arbitrary-code execution platform, a universal model router, a certified
alignment marketplace, an outcome/prediction market, or a financial compute
futures venue. It does not authorize copying Bazaar source or data, training or
publishing models, provider execution, paid workloads, federation, remote fleet
operations, custody, chain writes, or production deployment.

Resolve before implementation of each dependent stage:

- Which first buyer and one or two batch job templates have verified demand?
- Which workload manifest and evaluator contract can safely span raw compute
  and hosted model APIs?
- What local runtime isolation and owner-grant model is acceptable on the first
  supported operating systems?
- Which offer sources expose enough data to compare actual, fully loaded costs?
- Who controls evaluation trust roots, evaluator rights, result privacy,
  retention, and dispute decisions?
- Which metering errors, provider changes, failures, retries, or partial results
  should qualify for a buyer refund or re-run?
- What evidence threshold makes a route eligible for a high-stakes workload?

## Assurance and claim ceiling

This is a product and research plan, not implementation evidence. Existing
local checks apply only to their recorded synthetic slices. Provider and model
claims remain unverified until their exact adapters and evaluation packets are
run and reviewed. A marketplace receipt records attributed execution facts;
it does not on its own establish execution correctness, benchmark validity,
alignment, independent review, production readiness, or payment entitlement.
Evidence-bearing experiment reports must identify the exact revision and state
slice, assurance and reviewer status, reproducibility status, data/artifact
custody, limitations, claim ceiling, and next blocked authority gate.

## References

- [HyperAI compute-market architecture](compute-market-architecture.md)
- [HyperAI compute-market refactor plan](compute-market-refactor-plan.md)
- [HyperAI Rust compute desk subplan](compute-bazaar-rust-port-subplan.md)
- [OpenRouter latency and performance routing](https://openrouter.ai/docs/guides/best-practices/latency-and-performance)
- [RunPod Serverless](https://www.runpod.io/product/serverless)
- [Q Labs depth-scaling paper](https://arxiv.org/abs/2609.19107)
- [The Depth Delusion preprint](https://arxiv.org/abs/2601.20994)
