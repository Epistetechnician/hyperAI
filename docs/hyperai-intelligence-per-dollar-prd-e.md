# HyperAI Intelligence-per-Dollar PRD-E (Execution)

Status: draft execution plan; documentation only.
State slice: `hyperai.intelligence_per_dollar.prd.v1`.
Parent: [Intelligence-per-Dollar PRD](hyperai-intelligence-per-dollar-prd.md).
Gap IDs (G1..G14) and requirement IDs (R1..R8) refer to that PRD.

This plan is strategic. Every stage that edits code or data needs its own
named state slice in `docs/state-slices.md` (slice, exact files, allowed
effects, proof gates, claim ceiling, publish boundary) before work starts.
Stages S4 and later involve external effects and need explicit authorization.
Nothing here authorizes a provider call, rental, spend, or publication.

## Ground rules

- Local, synthetic, mock-only until a stage says otherwise.
- Freeze workload, dataset, evaluator, baseline, and quality bar before
  comparing any configuration. Record the freeze digest.
- Use `pnpm`, never `npm`. Report root, HyperIntel, JustAI, and Simple Compute
  Market validation separately.
- Preserve existing dirty work. Do not touch the modified interface and demo
  files unless a stage names them.
- Keep compute-market records separate from outcome and HIP-4 records.
- Market data is advisory. A signature or telemetry record is not proof.

## Stage map

| Stage | Name | Effects | Closes gaps | Depends on |
|---|---|---|---|---|
| S0 | Freeze the contract | docs only | G12 (partly), decisions | none |
| S1 | Workload and IPD spec | schema + fixtures, local | G2 (spec), G12 | S0 |
| S2 | Synthetic IPD advisor | local code, mock data | G3, G7 (model) | S1 |
| S3 | Kernel memory and gates | local code, synthetic kernels | G4, G5, G7 | S1 |
| S4 | First real measurement | one rental, capped, authorized | G1, G2, G9, G11 | S2, S3 |
| S5 | Guided search on real hardware | capped rentals | G6, G1 | S4 |
| S6 | Shadow routing | no dispatch; paired comparisons | G8, G10 | S4 |
| S7 | Capped pilot | one adapter at a time | G9, G14 | S5, S6 |
| S8 | Broaden | more workloads and providers | G10 | S7 |

S2 and S3 can run in parallel; they touch different crates.

## S0. Freeze the contract (docs)

Deliverables:
1. Decide the baseline for workload 1: a single named route, for example the
   full-size model on an on-demand H100 through one named provider with the
   default serving engine. Record the choice and why.
2. Decide the reuse targets for search (Triton autotune, CUTLASS profiler,
   TVM) and the first engine (vLLM or SGLang).
3. Decide who owns evaluator trust roots for the pilot (default: the buyer,
   with HyperAI holding only the digest).
4. Add the state slice entry for S1.

Done when: the three decisions are written in this file under "Decisions
log" and S1's slice is recorded.

## S1. Workload and IPD spec (local)

Slice: `hyperai.intelligence_per_dollar.spec.v1` (to be recorded).
Files (proposed): `schemas/workload-spec.v1.json`,
`schemas/ipd-receipt.v1.json`, `fixtures/ipd/*`, new module in
`crates/hyperai-market` (generic behavior only) or a new
`crates/hyperai-ipd`; decide in S0.

Deliverables:
- Workload spec (R1) extending `WorkloadIntentV1`; configuration record
  (model, quantization, engine, hardware, provider, region, kernel config
  refs); IPD receipt (R7) with explicit denominator and unknown states.
- Fixture set for workload 1 using a public, license-clean dataset and a
  tiny open model. Dataset rights recorded in `docs/provenance.md` (G13).
- IPD calculation with checked integer arithmetic (micro-USD), reproducible
  and property-tested: unknown cost never becomes zero; zero accepted work
  yields undefined.

Proof gates: schema golden tests, property tests for the metric, `cargo test`
for touched crates, root validation script. Claim ceiling: `INTERNAL_PASS`,
synthetic.

## S2. Synthetic IPD advisor (local)

Deliverables:
- Candidate enumerator over (model x quantization x engine x hardware) from
  a declared catalog; rank by predicted IPD with an uncertainty band, using
  mock performance tables and the existing GPUDiff snapshot for price.
- Read-only: output is a ranked table with provenance per cell. No dispatch.
- Surface in the existing local service as one read-only route and in the
  interface as a new panel; reuse connection-status vocabulary.
- Search-cost accounting: predicted savings versus tuning spend (G7).

Proof gates: deterministic output for fixed inputs; missing inputs render as
unknown; route is owner-only like the others; browser test for the panel.
Claim ceiling: advisory ranking from synthetic performance tables.

## S3. Kernel memory and gates (local)

Deliverables:
- Memory store (R4): key (op/graph, shapes, dtype, arch, driver, engine
  versions), value (config, throughput, variance, thermal note, correctness
  evidence digest), expiry on key change, bounded and no-overwrite journal
  following the market crate's pattern.
- Equivalence gate (R6): reference-vs-candidate comparator with dtype
  tolerances, randomized and adversarial inputs, NaN and edge shapes; plus a
  stability check over repeated timing runs.
- Synthetic kernels (CPU reference implementations with injected faults) so
  the gate is proven to reject wrong-but-fast candidates.

Proof gates: fault-injection tests where the gate must reject every injected
bug; expiry tests; replay determinism. Claim ceiling: the gate works on
synthetic kernels; no GPU claim.

## S4. First real measurement (external effect)

Requires separate authorization naming provider, instance type, hard spend
cap (suggest a few tens of dollars), credentials handling (buyer-held,
never stored in the repo), data scope, and stop conditions. Slice:
`hyperai.intelligence_per_dollar.first_measurement.v1`.

Deliverables:
- Run workload 1 on the baseline route and at least one alternative (for
  example an L40S or 4090 class with a quantized model) with the frozen
  evaluator.
- Capture price paid, wall time, throughput, accepted-work count, hardware
  identity, driver and engine versions, and noisy-neighbor indicators; repeat
  runs to estimate variance (G11).
- Emit IPD receipts and a short report with the baseline and denominator.

Done when: two or more configurations have repeatable receipts, and the
report states the measured IPD ratio with variance. If the ratio is not
favorable, report that; do not retune the evaluator.

Claim ceiling: one workload, one dataset, named hardware and dates; not a
general savings claim.

## S5. Guided search on real hardware (external effect)

Deliverables: profiler integration (G6), the search loop (R5) wrapping the
chosen reuse target, write-back into kernel memory through the S3 gate, and
end-to-end re-measure. Track search ROI: savings over search spend.
Stop rule: abandon search when marginal improvement per dollar of search
falls below a recorded threshold.

Done when: at least one memory entry produced by search improves end-to-end
IPD on the same frozen workload after passing the gate, and a second run
reuses it (hit) without new search spend.

## S6. Shadow routing (no dispatch)

Deliverables: for a fixed workload set, record what the advisor would pick
versus what a human or baseline picks, then measure both on paired runs
within the authorized budget. Add offer fields beyond list price where a
source and rights allow: firm quotes, availability, interruption rates,
egress and storage (G8). Run buyer interviews and test willingness to pay
(G10); do not fix fees before this.

## S7. Capped pilot

One adapter at a time (local, datacenter, hosted), each with its own slice,
hard cap, kill switch, and reconciliation. Commission an independent review of
the methodology and receipts (G14) before any external claim.

## S8. Broaden

Workloads 2 and 3 (fine-tune; media), more providers, multi-provider routing,
and settlement only if demand and the authority, dispute, and reconciliation
gates are satisfied. Follows the wedge plan's stage 5.

## Verification matrix

| Claim | Evidence required | First stage |
|---|---|---|
| IPD arithmetic is correct and handles unknowns | Property tests, goldens | S1 |
| Advisor ranks deterministically from stated inputs | Replay tests | S2 |
| Gate rejects wrong-but-fast kernels | Fault-injection suite | S3 |
| Config X beats baseline on workload 1 | Paired real runs with variance | S4 |
| Search pays for itself | Search ROI on real runs | S5 |
| Advisor picks well without dispatch | Shadow vs paired outcomes | S6 |
| Buyers will pay | Interview and pilot data | S6-S7 |
| Method is sound | Independent review | S7 |

## Decisions log

Fill in during S0.

| Decision | Choice | Date | Reason |
|---|---|---|---|
| Baseline route for workload 1 | open | | |
| First serving engine | open | | |
| Search reuse target | open | | |
| Evaluator trust owner | open (default buyer) | | |
| Schema home (`hyperai-market` or new crate) | open | | |

## Reporting

Each stage's report names the exact revision and state slice, assurance and
reviewer status, reproducibility status, data and artifact custody,
limitations, claim ceiling, and the next blocked authority gate. Root and
nested workspace validation are reported separately.
