# HyperAI Intelligence-per-Dollar PRD

Status: draft product requirements; documentation only.
State slice: `hyperai.intelligence_per_dollar.prd.v1`.
Supersedes nothing. This PRD is the umbrella above the
[wedge master plan](compute-market-wedge-master-plan.md) and the
[explorer PRD](compute-market-explorer-prd.md), which keep their own scope.
Execution plan: [PRD-E](hyperai-intelligence-per-dollar-prd-e.md).

This document authorizes no code, provider call, paid execution, rental,
publication, or claim beyond the ceilings below.

## 1. Product statement

Anyone with an AI workload that needs rented compute brings HyperAI the
workload and a quality bar. HyperAI returns the cheapest verified way to run
it, and proves the result with a receipt.

"Way to run it" means the whole configuration: model, quantization, serving
engine, kernel and batch settings, hardware, provider, region, and timing.
The unit sold is cost per unit of acceptable work, not GPU-hours.

Primary metric (extends the wedge plan's fully loaded cost per accepted job):

```text
intelligence per dollar (IPD)
  = customer-accepted work units, at or above the frozen quality bar
    -----------------------------------------------------------------
    fully loaded cost (compute + retries + data movement + storage
                       + orchestration + evaluation + search/tuning spend)
```

Report IPD only against a baseline run of the same frozen workload and
evaluator. If nothing is accepted, IPD is undefined; show spend and failures
instead. Missing prices or telemetry are unknown, never zero.

## 2. Does this exist

Pieces exist; the whole objective does not.

| Layer | Examples | Gap |
|---|---|---|
| GPU price finding and launch | SkyPilot, Shadeform, Vast.ai, RunPod | Optimizes price per GPU-hour; ignores quality and serving efficiency |
| Hosted model routing | OpenRouter | Routes API calls by price/latency; does not choose hardware or tune kernels |
| Price indices and futures | Silicon Data, CME compute futures | Hedging and benchmarks, not workload execution |
| Cluster cost tools | Cast AI, Kubecost | Trim spend on infrastructure you already run |
| Kernel autotuning | Triton autotune, TVM/Ansor, CUTLASS profiler, KernelBench | Per-kernel or per-model tools; no cross-run memory tied to sourcing |

This is a bounded scan from general knowledge and the wedge plan's
references, not proof that no one is building the whole stack. Re-verify
before any external claim.

## 3. Why it is valuable

- Savings multiply across layers: provider and GPU-class dispersion, model
  right-sizing and quantization, and serving and kernel tuning. A tool that
  shops only for GPUs captures a fraction. Magnitudes are hypotheses until
  measured (see section 9).
- A neutral broker paid on verified savings is aligned with the buyer. Labs
  and clouds sell their own models and hardware.
- Every run adds measured throughput, cost, and quality for a workload on
  specific hardware. That ledger and the kernel memory improve each later
  recommendation and are hard to copy.
- Faster configurations change which hardware is cheapest per unit of work,
  so tuning and sourcing improve each other.

## 4. Users and wedge

First customer: AI product and research teams with repeatable, bursty batch
inference, fine-tuning, or media jobs (the wedge plan's first customer).

Reference workloads, in build order:

| # | Workload | Quality gate | IPD unit |
|---|---|---|---|
| 1 | Batch LLM inference (classify, extract, summarize, embed) | Accuracy or agreement on a held-out labeled sample of the buyer's data | Cost per 1M tokens at the bar; cost per accepted record |
| 2 | LoRA/QLoRA fine-tune of a small or mid model | Target eval score reached | Dollars to reach quality X |
| 3 | Image/video generation or transcription | Fixed score (aesthetic/CLIP, or word error rate) | Cost per 1,000 images or per audio hour |

Workload 1 first: cleanest metric, widest option spread, fully runnable as a
synthetic mock under current constraints.

## 5. Core requirements

### R1. Workload contract
A versioned workload spec: type, model or runtime constraints, data boundary,
region, deadline, spend cap, evaluator identity, and quality bar. Builds on
`WorkloadIntentV1` in `hyperai-market`; general workload identity outside
procurement stays open.

### R2. Supply and observations
Typed offers from local, datacenter, and hosted classes with source, unit,
price basis, observed time, and freshness. Listing price is advisory and never
equals a firm quote. Already built: GPUDiff snapshot ingestion and a read-only
price board. See the explorer PRD for coverage policy.

### R3. Configuration search
Given a workload and bar, enumerate candidate configurations (model,
quantization, engine, hardware, provider) and rank by predicted IPD with
uncertainty. Start with a read-only advisor that never dispatches.

### R4. Kernel and configuration memory
A store keyed by (op or fused graph, shapes, dtype, GPU arch, driver and engine
versions) holding best-known configs, measured throughput, variance, thermal
state, and correctness evidence. Entries expire when keys change.

### R5. Guided optimization loop
On each run: profile, warm-start from nearest memory entries, search
(Bayesian or evolutionary over tile sizes, warps/stages, fusion, batch, KV
cache, quantization; model-proposed kernel variants where search cannot reach),
steered by the bottleneck class (memory, compute, launch bound).

### R6. Correctness and quality gates
A candidate enters memory only after: numerical equivalence against a
reference (dtype tolerances, randomized and adversarial inputs, NaN and edge
shapes), a stable speedup over repeated runs, and an end-to-end re-measure of
workload throughput and IPD at the quality bar. A fast wrong kernel is a
product failure. Search is allowed only when expected savings over remaining
runtime exceed the search cost, and search cost is included in IPD.

### R7. Proof and receipts
Each run emits a portable receipt: frozen workload and evaluator identity,
baseline and chosen configuration, price actually paid, runtime, hardware
identity, eval results, and the IPD comparison with denominator. Provider
signatures and telemetry attribute statements; they do not prove correct
computation or payment entitlement.

### R8. Buyer control
Provider accounts, credentials, and billing stay with the buyer. Any change
to job, price, provider, or boundary needs a new preview and approval. Local
supply is off until an owner grants a bounded resource and window.

## 6. Non-goals

- Universal model router, public GPU network, arbitrary-code execution
  platform, futures venue, or custody of money, credentials, or fleets.
- Universal quality, alignment, or "effective compute" scores.
- Cross-unit conversion of GPU-hours, tokens, and requests into one number.
- Any use of The Compute Bazaar source or data (rights unresolved; see
  AGENTS.md).

## 7. Success measures

Primary: IPD improvement versus the same-workload baseline, with denominator.
Also: eligible-route coverage, acceptance rate, latency, throughput,
availability, retry rate, spend-cap adherence, reproducibility of receipts
(re-run reproduces within tolerance), kernel-memory hit rate, search ROI
(savings over search cost), and repeat buyer use.

Targets are not set. They need baseline measurements from stage S2 in the PRD-E
before numbers are committed.

## 8. What we have

Verified in the repo at planning time. "Implemented" means local and
synthetic unless stated.

| Capability | Where | Ceiling |
|---|---|---|
| Market contracts: offer, catalog, request, quote, approval, receipt, verifier decision, mock settlement; offline receipt verify; bounded journal | `crates/hyperai-market`, [architecture](compute-market-architecture.md) | `INTERNAL_PASS`, synthetic |
| Bounded `WorkloadIntentV1` and read-only synthetic workload trace | `hyperai-market`, `apps/hyperai-interface/workload-trace.js` | Synthetic |
| Data module with GPUDiff snapshot, catalog, query, derived-data contracts | `crates/hyperai-data` | Public list prices, advisory |
| Fleet and eval crates (simulation, eval API) | `crates/hyperai-fleet`, `crates/hyperai-eval` | Simulation only |
| Local service, CLI, TUI, MCP, ACP surfaces | `crates/hyperai-local-service`, `outcomes-cli`, `hyperai-tui`, `hyperai-mcp`, `hyperai-acp` | Local |
| Browser interface with market supply comparison and quote lifecycle | `apps/hyperai-interface` | Local, partial accessibility coverage |
| Live read-only price board, static export, Pi 5 deploy kit | `demo/live-board`, `deploy/rpi5` | Operator-run; 53 valid GPU hourly rows in a manual refresh |
| Outcome protocol, evidence, settlement, HIP-4 (separate record families) | `crates/outcomes-*`, `hyperliquid-hip4` | Mock settlement only |
| Provenance, state slices, threat model, CI | `docs/`, `.github/workflows/ci.yml` | Process |

## 9. What we are missing

Gaps, ordered by how much they block the objective.

| # | Missing | Why it matters | Type |
|---|---|---|---|
| G1 | Any real workload measurement: tokens/sec, cost, and quality for one workload on at least two hardware classes | Every IPD claim is currently a hypothesis | Evidence |
| G2 | Quality evaluators for the three reference workloads, with frozen datasets and thresholds | IPD is meaningless without a quality bar | Evidence + code |
| G3 | Candidate-configuration model (model x quantization x engine x hardware) and an IPD predictor with uncertainty | The core of R3 | Code |
| G4 | Kernel/configuration memory schema, store, expiry rules | R4 | Code |
| G5 | Numerical-equivalence and stability gate | R6, the main safety feature | Code |
| G6 | Search loop and bottleneck profiler integration; choice of reuse targets (Triton autotune, CUTLASS profiler, TVM) | R5 | Code + decision |
| G7 | Search-cost accounting inside IPD | Prevents optimization that loses money | Code |
| G8 | Offer data beyond a list-price aggregator: firm quotes, availability, interruption rates, egress and storage fees | Fully loaded cost needs these | Data + rights |
| G9 | A provider adapter for any real rental, with credentials and spend-cap handling | Needed to prove anything on real hardware | Authority |
| G10 | Willingness-to-pay and buyer interviews; a fee model | Commercial hypothesis is untested | Evidence |
| G11 | Reproducibility protocol: pinned images, seeds, driver versions, thermal and noisy-neighbor controls | Receipts must re-run | Process |
| G12 | Baseline definition: what counts as the naive route for each workload | Savings claims need a fair baseline | Decision |
| G13 | Rights review of data sources and benchmark datasets | Redistribution and licensing | Authority |
| G14 | Independent review of the methodology | Local green checks are not independent review | Authority |

Open decisions carried from the wedge plan: evaluator trust roots and
ownership, refund and re-run rules for metering errors, evidence threshold for
high-stakes eligibility, and supported operating systems for local supply.

## 10. Risks

- Price routing alone is easy to copy. Defensibility is the workload-to-config
  optimizer plus the measured ledger and kernel memory.
- Savings are noisy: spot prices move, hardware varies by host, and benchmarks
  drift. Report uncertainty; do not average away variance.
- Kernel entries rot with driver and engine updates. Key tightly and expire.
- Search spend can exceed savings on short jobs. Gate on expected ROI.
- Self-reported provider or telemetry data can mislead. Treat as advisory.
- A quality bar chosen after seeing results is a hidden degree of freedom.
  Freeze evaluators before any comparison.

## 11. Assurance and claim ceiling

This is a product requirements document, not implementation evidence. Section
8 reflects repository contents at planning time and local synthetic checks
only. No cost-saving, throughput, or quality claim is supported until the
PRD-E stages produce frozen-workload, paired-baseline evidence with named
revision and slice. A receipt records attributed facts; it does not establish
correct computation, benchmark validity, independent review, production
readiness, or payment entitlement.

Publish boundary: repository documentation only.
