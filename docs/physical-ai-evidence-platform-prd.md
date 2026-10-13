# HyperAI Physical AI Evidence Studio: Product Requirements

Status: draft; product and research plan only.
State slice: hyperai.physical_ai.evidence_platform_prd.v1.
Global assurance overlay: `global-experiment-assurance-lifecycle-v1`.
Planning baseline: HyperAI dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa on main,
with the pre-existing dirty and untracked working tree preserved.
Research scan: bounded public-source review performed 2026-09-29; commercial
and competitive sequencing refreshed in
docs/physical-ai-evidence-business-plan.md on 2026-10-01.

This document proposes an adjacent HyperAI product line. It does not change the
existing compute-market product decision or authorize implementation, capture
or transfer of real-world data, model training, simulation runs, provider
access, payment, chain activity, public release, or deployment.

## Executive summary

Build a B2B reliability-evidence workbench for robotics OEMs and systems
integrators releasing AI-enabled policies into repeated customer-facing tasks.
The first commercial workflow imports customer-controlled historical logs and
evaluation records, binds them to the robot, policy/software revision, task,
conditions, and exposure, then produces a reproducible release-evidence packet
with a data-readiness profile for the buyer's declared question. It exposes
lineage, synchronization and completeness limits, condition coverage, missing
data, and unresolved claims without asserting general training readiness.

The companion business plan defines the buyer, paid pilot, pricing hypotheses,
competitive position, and financial planning case. It changes the commercial
sequence: the first product reviews existing authorized records. Phone-first
contributor capture is deferred. The eventual product may add private
contributor capture, task evidence, visual replay, bounded prediction
evaluation, calibrated simulation, and cross-party contribution incentives
only as separate capabilities with their own rights and validation gates.

The long-term vision remains a cross-party physical-learning network. It may
help contributors, operators, robotics teams, evaluators, and eventually
capital providers fund underrepresented conditions and reduce uncertainty
about where a robot can work. Every derived dataset, simulator, model adapter,
quantized package, and evaluation remains linked to source evidence and its
limitations.

The MVP tests whether buyers will pay for a faster and more reproducible
release or failure-review workflow over existing records. It does not claim
that a log proves the physical event, a score certifies safety, a visual replay
is a faithful simulation, or an evidence packet authorizes deployment.

## Mission and product thesis

### Mission

Make physical-world learning accumulative, inspectable, and fairly
coordinated. A real episode should not disappear into an unlabeled video folder
or an untraceable training run. People and organizations should be able to
contribute evidence under clear rights, see how it was transformed and used,
and receive credit when it supports a validated improvement.

The system's durable asset is evidence about the relationship among task,
environment, machine or demonstrator, policy version, outcome, and uncertainty.
The initial product treats physical uncertainty as a research and economic
problem: it helps teams discover which task conditions lack evidence, gather
useful episodes safely, and test whether added evidence changes a real decision.

### Strategic thesis from the source conversation

- Frontier-scale training can remain centralized or tightly coupled where its
  communication and compute economics require it. The product does not depend
  on decentralizing ordinary backpropagation.
- The first product uses customer-controlled storage and ordinary contracts.
  Shared provenance, escrow, or other crypto coordination is considered only
  if a demonstrated multi-party workflow requires it.
- Data rewards should eventually reflect validated information value or
  operational risk reduction, not raw recording hours, novelty scores, or
  token activity alone.
- Measure data quality against the buyer's declared task and use: provenance,
  synchronization, completeness, outcome labels, and coverage across relevant
  conditions matter more than a raw count of hours or trajectories. The MVP
  reports these limits; it does not claim that coverage alone predicts model
  performance.
- Failures, near misses, interventions, aborts, and exposure denominators are
  first-class evidence. The product should preserve negative and inconclusive
  outcomes rather than making success the only rewarded result.
- Use observed failures to specify decision-relevant evidence gaps. The MVP may
  produce a human-reviewed, buyer-approved collection brief, but it does not
  source data, run a bounty market, or initiate retraining.
- Test service-assisted delivery as the path into repeated customer workflows.
  Reusable mappings and methods may compound only under explicit data rights;
  raw customer data are not pooled by default. Operator acquisitions or
  ownership of robot operations are outside the current product and business
  case.
- Local adaptation, model-improving physical work, multi-party robot
  coordination, and performance-linked credit or insurance are later research
  directions. They depend on reproducible evidence, rights, evaluator
  quality, and appropriate human authority.
- The large model may retrieve evidence and propose experiments. It does not
  own the holdout, evaluator, promotion decision, or physical operator.

These are product hypotheses, not established market findings. The public
landscape already includes broad robot datasets, failure-data initiatives,
world-model platforms, simulation tools, and robot-risk products. RoboGate and
Run Robotics publicly describe workflows that overlap directly with failure
evaluation and reliability evidence. The proposed differentiation is a
buyer-owned release-evidence workflow across existing customer records and
tools; whether buyers see a material gap and will pay for it remains untested.

## Current landscape and differentiation

The research and product space is active. Open X-Embodiment describes
standardized data and models assembled from multiple robot datasets and
institutions. Oopsie Data describes a multi-lab effort to collect real
manipulation failures and suboptimal rollouts. Robometer describes a
large-scale trajectory-comparison reward-model dataset that includes
suboptimal and failed behavior. These projects establish that multi-source
robot data and failure evidence are active areas; they do not establish paid
adoption for this release-evidence workflow or prove this PRD's business
hypothesis.

NVIDIA's Cosmos materials describe video processing and world-foundation-model
workflows alongside simulation frameworks such as Isaac Sim. Google DeepMind
describes Genie as an interactive world model and also lists limitations on
perfectly simulating real-world locations. The PhysWorld paper studies
video-to-physics reconstruction for deformable objects. Together these are
useful technical precedents; they do not make a single phone recording a
general, calibrated, physics-accurate digital twin.

NIST's Physical AI program describes work on measurement, evaluation methods,
testbeds, and standards for AI-enabled robotics. Company materials from Rhytor
and Koop describe robotics risk-data and insurance products. This confirms
that evaluation and underwriting are adjacent workstreams. This scan is
bounded and does not prove the absence of competitors or validate company
claims about reach or results.

RoboGate and Run Robotics are direct visible alternatives in failure
evaluation, rollout lineage, and reliability reporting. RoboBlackbox describes
an independent record of fleet behavior. The companion business plan records
the bounded comparison and current public claims. No unique or unoccupied
category is asserted here.

The MVP must be useful without requiring customers to pool data or change
their robot operations:

1. Bind source records to robot, policy/software version, task, conditions,
   exposure, outcome, and evaluation context where those fields exist.
2. Preserve lineage from customer source through transformations and report.
3. Distinguish observed, human-annotated, model-inferred, simulated,
   physically reproduced, and independently accepted evidence.
4. Make missingness, failed runs, and inconclusive outcomes visible.
5. Export a portable, customer-controlled evidence packet.

Contributor capture, broad data pooling, and value-sharing mechanisms are
deferred until a paid workflow demonstrates a need for them.

## Users and jobs to be done

### Primary user: robotics reliability or quality engineering lead

Needs field and test records joined to exact robot, policy, software, task,
condition, and exposure context. Wants to investigate failures and review
releases without manually reconstructing evidence from separate logs, test
sheets, and version histories.

### Future contributor: operator, technician, teleoperator, or demonstrator

Needs a low-friction way to record an authorized task, explain what happened,
control what is shared, inspect proposed labels, and understand whether the
record may be used for research, training, publication, or commercial reuse.

### Evaluator or data steward

Needs versioned schemas, duplicate and quality checks, fixed evaluation
protocols, controlled access to sensitive footage, and a clear ability to
reject, quarantine, or mark a record inconclusive.

### Future sponsor: OEM, fleet operator, buyer, insurer, or lender

Needs evidence about a declared task and operating envelope, including
exposure and interventions, before deciding whether to fund exploration,
expand a deployment, change a maintenance plan, or request underwriting review.
The product supplies evidence; the sponsor retains the relevant decision
authority.

## Product goals

- Import a customer-selected historical evidence export through a read-only
  path and produce a report scoped to one release or failure-review question.
- Bind source references, timestamps, robot/policy versions, task context,
  outcomes, exposure, uncertainty, and customer-defined rights status.
- Make incomplete lineage, missing denominators, and unknown outcomes visible.
- Compare a frozen baseline and candidate only when the customer has authorized
  the artifacts and evaluation protocol.
- Create a versioned evidence graph linking source, derived data, processing,
  training, simulation, evaluation, and promotion.
- Allow a larger model to retrieve approved episode knowledge and propose
  additional tests while preserving evaluator and operator authority.
- Establish a path to future contributor capture, local adaptation, and
  cross-party incentives only after the initial paid workflow is validated.

## Non-goals

The MVP will not:

- Control, dispatch, or authorize a physical robot.
- Train or update a frontier foundation model or autonomously promote model
  changes.
- Treat generic generated video as a physics engine or safety test.
- Claim universal physical understanding, general robot competence, safe
  autonomy, or production readiness.
- Assume that a human phone video contains robot actions, forces, joint states,
  or embodiment-specific control labels.
- Require phone capture or a contributor network for the first commercial
  release.
- Put raw video, personal data, credentials, or private traces on a public
  blockchain.
- Launch a token, open a permissionless data market, automate insurance
  underwriting, or represent a model score as an actuarial or safety decision.
- Collect or transfer real contributor data under this documentation-only
  authorization.
- Import source code, data, schemas, assets, or generated outputs from
  references whose reuse rights are unresolved.

## MVP decision

### Initial research and product domain

Start with one repeated manipulation task performed by an AI-enabled robot
policy in a warehouse or industrial cell. Vision-guided pick-and-place is a
candidate, not a fixed commitment. Select the task, buyer, robot/policy family,
source fields, and evaluation question only after customer discovery and
data-schema review. Use customer-controlled historical exports; do not make a
phone recording a substitute for robot state, action, intervention, or
exposure records.

Before a pilot, freeze the task, allowed data, record mapping, outcome and
exposure definitions, evaluation protocol, budget, retention, and stop
conditions in a separate packet. Quantitative tolerances remain open until
agreed with the buyer and evaluator.

### MVP user experience

1. **Declare the question.** Select the task, robot/policy family, release
   revision, site or cell, and buyer-owned decision under review.
2. **Select an export.** A customer operator chooses a historical source
   package and authorizes a read-only import under a bounded purpose.
3. **Map and qualify records.** Bind available versions, conditions,
   timestamps, outcomes, and exposure; check synchronization and completeness
   only where source fields support those checks; compare observed conditions
   with a buyer-declared coverage matrix.
4. **Review evidence.** A human checks duplicates, anomalous records, and
   proposed annotations. Unknown values remain unknown, and a readiness status
   is not inferred from data volume.
5. **Run or attach an evaluation.** Compare releases only under the
   pre-agreed protocol and authorized data; report controls, holdouts, and
   evaluator identity.
6. **Specify the next evidence need.** State the smallest decision-relevant
   gap and, if requested, draft a buyer-approved collection specification.
7. **Export the evidence packet.** Show source lineage, data-readiness limits,
   coverage, outcomes, differences, uncertainty, open issues, and the human
   decision owner.

The first workflow is a private engineering report over historical evidence.
It has no live robot connection, command path, public showcase, or automatic
release decision.

### MVP boundaries

**Included in the planned MVP:** bounded customer-selected historical import;
source and version lineage; schema mapping; explicit unknown and missingness
handling; field-supported synchronization and completeness checks; coverage
against a buyer-declared condition matrix; human review; one
release/failure-review comparison; and a versioned, exportable evidence
report. The report is scoped to its declared downstream question and makes no
general model-training-readiness claim.

**Stretch capability:** one repeatable read-only source adapter and retrieval
of approved episode summaries. These require customer schema evidence and
separate implementation authority.

**Later capability:** phone capture, visual replay, contributor network,
physics simulation, model adaptation, and cross-party value sharing. Simulation
passes remain separate from physical robot acceptance.

## Functional requirements

### Customer-controlled historical evidence import (MVP)

- Import only a customer-selected source export for an agreed review purpose.
- Use read-only processing; do not issue robot, fleet, or production-system
  commands.
- Preserve source identity, version, time, units, task context, and outcome
  fields when available; record the exact fields that are absent.
- Bind exposure denominators to the declared task and observation window.
  Never infer a failure rate from an unknown or incomplete denominator.
- Preserve failure, near-miss, intervention, abort, partial, and unknown
  outcomes without converting missing fields to success.
- Profile completeness, synchronization, duplicates, quarantine, and coverage
  only to the degree supported by the source fields and predeclared question.
- Show which declared task, environment, object, and perturbation conditions
  are present, absent, or unknown. Do not reduce the profile to a universal
  quality score or predict model uplift from data volume.
- Permit a human-reviewed, buyer-approved gap specification as an output;
  acquisition, contributor compensation, and model training remain outside
  the MVP.
- Keep raw artifacts in customer-controlled custody by default. Any transfer,
  retention, or processing outside that boundary requires separate written
  authorization.

### Future capture and contributor rights

- Provide a mobile-first PWA with task-specific capture guidance and explicit
  start, pause, resume, preview, and submit controls.
- Support video, optional narration, task conditions, outcome labels, and
  contributor corrections. Additional sensors are optional and must declare
  device and timestamp limitations.
- Show a clear consent and data-use summary before upload. Distinguish storage,
  annotation, retrieval, model training, commercial reuse, research sharing,
  and public publication.
- Default to private, access-controlled storage. The contributor must be able
  to preview and omit segments before submission.
- Provide redaction review for faces, screens, identifying documents, and
  sensitive areas before any wider sharing.
- Record rights grants and withdrawal requests against each source and
  derivative. Product policy must explain that removing data from an already
  trained model may not be technically possible; training permission therefore
  needs its own explicit grant and retention terms.
- Preserve outcomes as success, failure, partial, intervention, abort, or
  unknown. Never convert missing telemetry into a success or a zero-cost event.

### Episode processing and evidence

- Every derived claim links to source timestamps or an explicit external
  annotation record.
- Distinguish observed media, contributor statements, human-reviewed labels,
  machine inferences, simulation outputs, and physical test results.
- Store model identity, prompt or processing configuration, runtime, dataset
  revision, and transformation version for each derived artifact.
- Support duplicate detection and quarantine. Quarantine must not discard the
  original source or silently admit it to a training dataset.
- Preserve original bytes under the owner's retention policy; use digests and
  access-controlled references to connect artifacts.
- Model-generated text, OCR, signs, and audio are untrusted data. They must not
  be executed as instructions by the processing or research agents.
- Provide explicit provenance states for rights unknown, pending review,
  accepted for specified use, revoked for future use, rejected, and disputed.

### Scene reconstruction and simulation

- Generate a visual replay for the selected task, with links back to source
  frames and model-generated components.
- Represent geometry, object identity, motion, contact, and inferred dynamics
  with confidence and unknown states.
- Keep visual similarity and physical validity as separate measurements.
- A physics-simulator adapter must record simulator version, scene assets,
  units, object parameters, calibration data, initial conditions, action
  representation, and scenario seed.
- No simulation result can be labeled physically validated without comparison
  to a predeclared physical test set.
- Do not use a generated environment to authorize robot operation or replace
  hardware safety controls.

### Dataset, model, and evidence graph

The conceptual graph contains versioned nodes for:

- Contribution grant and source episode.
- Capture application and device metadata.
- Human annotations and model-derived observations.
- Episode collection and immutable dataset snapshot.
- World or scene hypothesis and simulator scenario.
- Training or adaptation run and complete model-harness configuration.
- Full-precision model, quantized model, and serving package.
- Evaluation protocol, holdout identity, result, review, and promotion
  decision.
- Optional sponsor request, accepted evidence, contributor allocation, and
  payment receipt in a separately authorized market phase.

Edges identify derivation, membership, evaluation, rights inheritance, and
approval. The graph is a provenance and access-control system; it is not one
undifferentiated memory store. Raw video remains under controlled custody and
does not go onchain.

### Retrieval and model improvement

- Initial integration is retrieval of approved episode cards and evidence
  summaries into a general model's context or a typed tool.
- The general model may identify coverage gaps and propose candidate
  experiments. It may not modify the evaluator, holdout, claim ceiling, or
  promotion authority.
- Parameter tuning occurs only as a frozen, versioned training run over an
  approved dataset snapshot.
- Quantization is a separate candidate transformation and must be evaluated
  against its full-precision source under the same task protocol.
- The product retains failed, rejected, unknown, and inconclusive runs with
  reasons and exact artifact identities.
- Promotion requires an authorized human decision after held-out evaluation;
  shadow, canary, deployment, and rollback require separate operational
  authority.

## Evidence and verification model

The product must display the evidence level beside every output:

1. **Captured:** bytes and metadata were received and bound to a source record.
   This proves record integrity, not event truth.
2. **Annotated:** a contributor or reviewer labeled a segment. Attribution
   does not prove the label is correct.
3. **Model-inferred:** a model estimated an object, action, state, or outcome.
   The confidence and model version are visible.
4. **Replay-generated:** a model rendered a visual reconstruction. Visual
   plausibility is not physical simulation.
5. **Simulation-evaluated:** a declared simulator and scenario produced a
   recorded result. This does not establish sim-to-real validity.
6. **Physically reproduced:** a bounded test was run on named hardware and
   environment with measured conditions. Same-owner repetition is not
   independent review.
7. **Independently accepted:** only available after the configured independent
   authority returns a signed decision bound to the exact packet. This PRD
   creates no such result.

The product must never collapse these levels into a single “verified” badge.
Cryptographic signatures and hashes bind records to keys and bytes. They do
not prove the identity of a contributor, the truth of a recording, the
correctness of a simulation, or the safety of a robot.

## Experimental design and RSI governance

Every material model or harness change is a hypothesis test. Before an
experiment, a separate packet must freeze:

- Exact model, checkpoint, adapter, quantization, prompt, preprocessing,
  harness, tools, memory, evaluator, runtime, and authority configuration.
- Target metric and direction, smallest meaningful effect, baseline, controls,
  variance sources, paired units or seeds, repeat count, stopping rule, total
  resource budget, retention, claim ceiling, and nonclaims.
- Dataset identity, rights, custody, source grouping, deduplication, training
  split, and sealed holdout policy.
- Safety controls for any physical capture or test and the operator who can
  stop it.

The initial benchmark hierarchy is synthetic or hermetic checks, representative
local checks, then separately authorized held-out physical or external
evaluation. Splits should group by contributor, scene, task instance, or time
where needed to prevent near-duplicate leakage. Cheap checks are useful only
after their relationship to the representative evaluation is measured.

Keep proposer, evaluator, red-team, replicator, governor, and operator
responsibilities separate. The loop is:

**observe → record → verify → curate → retrieve or train → evaluate on sealed
data → shadow → authorized promotion or rollback.**

The base model, evaluator, holdouts, authority policy, and governance kernel
remain immutable within a candidate run. Reversible retrieval, prompt, and
adapter experiments precede any broader parameter update. No autonomous
recursive self-improvement claim is made.

## Architecture and trust boundaries

The PRD is technology-neutral. The proposed logical components are:

1. **Customer-controlled evidence import and review:** bounded read-only
   selection of historical exports, task/version mapping, and report review.
2. **Ingestion and custody boundary:** explicit authorization, private or
   customer-managed processing, retention, access logs, and provenance receipt.
3. **Evidence registry:** versioned source, rights, dataset, model, simulator,
   evaluation, and review records with controlled references to artifacts.
4. **Evaluation workbench:** fixed baselines, task metrics, sealed data
   handling, comparison reports, regression tracking, and candidate status.
5. **Optional episode processor:** media segmentation, multimodal annotation,
   confidence and unknown handling, human review, duplicate detection, and
   quarantine for a later capture phase.
6. **World and simulation adapters:** separate APIs for video prediction,
   scene reconstruction, physical simulation, and robot telemetry. Provider
   or simulator selection is an open later-stage decision.
7. **Future PWA capture client:** local recording control, task form, rights
   preview, redaction review, resumable submission, and contributor access.
8. **Research assistant and governance boundary:** retrieve only authorized
   records and propose experiments; keep evaluation, promotion, physical
   execution, external sharing, and deployment under their respective human
   authorities.

The current HyperAI compute-market product owns compute-offer and
buyer-workload composition. This product would own physical episode and
learning-evidence records. They may later connect through versioned adapters
for buyer-approved computation, while keeping data rights, training
decisions, physical operations, and compute deals separately authoritative.

## Privacy, safety, and rights

- Obtain explicit permission from the person recording and rights-cleared
  authority for the environment, people, objects, and any third-party material.
- The upload flow must warn about bystanders, faces, voices, screens, addresses,
  location, proprietary processes, and safety-sensitive information.
- Geolocation and device identifiers are off unless the task requires them and
  the contributor opts in.
- Access to raw media is purpose-bound, logged, revocable for future access,
  and limited to the minimum roles required.
- Training and commercial reuse require separate, understandable grants.
  Terms must distinguish source media from derived annotations, weights, and
  evaluation artifacts.
- Do not promise that deleting a source can remove its influence from an
  already-trained model. Define retention and retraining or unlearning options
  before collecting training-authorized data.
- No data, private prompt, full video, or precise location is included in a
  public-chain transaction. Any future public commitment is limited to
  approved digests and non-sensitive metadata.
- No generated model output controls a robot in the MVP. Physical testing
  requires a separate equipment owner, safety protocol, emergency stop, test
  boundary, and operator authorization.
- Any future reward system must avoid incentives to stage hazards, omit
  incidents, overstate sensor authenticity, or collect unsafe edge cases in
  public environments.

## Success measures and experiment hypotheses

### Product north star

**Full cost and time to produce a buyer-accepted, reproducible evidence packet
for a bounded release or failure-review decision.** Report data preparation,
engineering review, compute, storage, integration, physical test time, and
rejected or inconclusive work separately. More records are not a success if
their lineage, exposure, rights, or decision value are unknown.

### MVP product measures

- Time from validated export to first inspectable report.
- Time spent cleaning and synchronizing the selected records, and the share of
  records accepted, quarantined, rejected, or left unknown under the declared
  use.
- Coverage of the buyer-declared condition matrix, with missing and unknown
  conditions reported separately.
- Fraction of source records with policy/version, task, time, outcome, and
  exposure fields required by the agreed question.
- Analyst minutes and elapsed time for the selected review before and during
  the pilot.
- Human correction, rejection, and unknown rates for any generated
  annotations.
- Full cost per report, including integration and human review.
- Buyer acceptance, repeat-pilot intent, and paid annual conversion.
- Provenance completeness from source export through report.

### Model and simulation measures

- Event, object, and state extraction compared with independently double-
  annotated episodes.
- Outcome or next-state prediction against a frozen zero-shot baseline and a
  retrieval-only control.
- Calibration and abstention quality on ambiguous or out-of-distribution
  episodes.
- Visual reconstruction quality and physical outcome agreement reported as
  separate axes.
- Candidate model utility, regression/retention, data efficiency, and
  full-cost-per-verified-improvement.
- For later robot policies: task success, failure severity, intervention
  rate, and operating-envelope coverage on held-out physical tests.

Numeric thresholds, smallest meaningful effect, sample size, repeats, and
stopping rules are open. They must be pre-registered in the relevant
experiment packet before real data or paid compute is used. No performance
result is claimed by this PRD.

### Commercial and network measures

- Number of qualified design partners willing to share a bounded task and
  define a buyer-owned evaluation.
- Paid conversion or signed pilot commitments, tracked separately from
  interest or waitlist volume.
- Share of contributions with explicit reuse rights and intact attribution.
- Cost and time to resolve provenance, quality, and rights disputes.
- In later network phases, sponsor renewal and reward allocation tied to
  independently accepted utility rather than submitted volume.

## Delivery phases and decision gates

### Phase 0: design and synthetic contract

Define the episode schema, rights vocabulary, provenance graph, trust
boundaries, task template, evaluation protocol, and synthetic walkthrough.
No real capture or model training. Gate: reviewers agree that observed,
inferred, simulated, and physically tested claims are distinct and the schema
preserves unknowns.

### Phase 1: historical-evidence-to-report prototype

Build a local, synthetic vertical slice for bounded record import, schema
mapping, evidence lineage, missingness reporting, and one release-evidence
report. A customer-data pilot requires a separate state slice and data-rights
packet. Gate: demonstrate the report on synthetic records and establish that a
qualified buyer will pay for a bounded pilot before expanding the product.

### Phase 2: episode retrieval and prediction evaluation

Create a curated, versioned dataset and compare a fixed general model,
retrieval-only model, and candidate task adapter on a sealed evaluation set.
Gate: predeclared metrics, costs, data custody, and an evaluator separate from
the proposer.

### Phase 3: estimated scene and calibrated simulator

Produce an estimated scene from approved recordings, then calibrate a bounded
physics simulation with known dimensions, repeated outcomes, and physical
checks. Gate: simulator claims remain scoped to the validated task and
environment; failed calibration is retained.

### Phase 4: robot/teleoperation data and task adapters

Add robot action, state, or teleoperation streams through a distinct
embodiment-specific contract. Train a separate candidate adapter, evaluate
against held-out physical tasks, test quantization independently, and retain
rollback artifacts. Gate: hardware owner and operator authorize the exact
physical test.

### Phase 5: cross-party information-value network

Test sponsor-funded collection for predeclared coverage gaps. Use a base
contribution reward for complete, rights-cleared evidence and a later bonus
for independently replicated, actionable findings. Negative and inconclusive
results remain eligible under the contract. Gate: rights, evaluator
independence, dispute resolution, allocation, and full economics reviewed.

### Phase 6: risk and capital integration

Explore whether evidence can inform maintenance, operational policy, warranty,
credit, or licensed-insurance decisions for one narrowly defined asset class.
HyperAI supplies evidence and workflow; the appropriate human, operator, or
regulated institution retains decision authority. This phase requires its
own legal, actuarial, safety, and deployment authorization.

### Phase 7: local learning and multi-party coordination research

Evaluate small-fleet adaptation methods, sample efficiency, retention of
existing skills, robustness to distribution shift, and coordination across
owners or shared physical resources. Compare against appropriate centralized
baselines. Do not claim decentralized training is viable until a matched
experiment demonstrates it for the selected task.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Customer exports lack policy identity, exposure, outcomes, or physical context | Qualify data readiness before a pilot; preserve unknowns and do not infer failure rates or event truth from missing fields. |
| Photorealistic replay is mistaken for physics | Label each output level, expose unknown parameters, and require physical validation for dynamics claims. |
| Contributor footage exposes people or proprietary spaces | Private-by-default capture, preview/redaction, granular rights, minimum retention, and explicit upload. |
| Training data contaminates evaluation | Immutable dataset snapshots, grouped splits, sealed holdouts, and independent evaluation role. |
| Data volume rewards low-value or staged records | Pay for complete, rights-cleared evidence and validated utility; include failures and denominator data; audit incentives. |
| One operator controls evidence, model, evaluator, and promotion | Separate proposer, evaluator, governor, and physical operator authority; preserve versioned records. |
| Token or insurance narrative outruns utility | No token or underwriting claim in MVP; validate a paying buyer and one useful decision first. |
| Broad RSI ambition produces unbounded compute or unsafe updates | Fixed budgets, frozen evaluator/holdouts/base policy, sandbox-only candidates, explicit stopping and rollback. |
| Open-world/generalization claims outrun a narrow benchmark | Report task, embodiment, site, and environment scope with each result; test new scopes separately. |

## Open decisions

- Which single task family, user, and budget owner provide the strongest
  repeatable first release-evidence workflow?
- Which source export can bind policy version, task, exposure, outcomes, and
  conditions without requiring live robot access?
- Which later capture, visual reconstruction, simulation, and model providers
  meet data custody, license, cost, and reproducibility requirements?
- Who controls raw media, annotations, model-derived features, trained
  adapters, and downstream commercial rights?
- What data can be deleted after training, and what retention/withdrawal terms
  can be honored technically and contractually?
- What constitutes an accepted episode and a meaningful information-value
  result for the first task?
- Who funds collection, evaluation, and ongoing contribution rewards?
- Which multi-party trust failure would require crypto settlement instead of
  signed off-chain records and ordinary payment?
- Which research artifacts remain isolated from any promoted product model?
- What resource budget, retention period, and stop conditions bound each
  experiment?

## Authority, assurance, and publication boundary

This PRD authorizes documentation only. It does not authorize source code
changes, a runnable PWA, real-world capture, data transfer, provider calls,
model or simulator execution, paid activity, chain writes, publication, or
deployment. Development may begin only under a new implementation slice with
frozen source and data rights, synthetic or explicitly authorized inputs,
privacy controls, budgets, evaluator and holdout identities, claim ceiling,
and stop conditions.

Below independent acceptance, local work may qualify only its stated synthetic
or explicitly authorized scope. `INTERNAL_PASS`, same-owner reproduction,
model signatures, hashes, local simulation, and successful ingestion do not
establish independent acceptance or production authorization. Independent
review and deployment approval are separate future gates.

Publish boundary: local planning document only. No commit, pull request,
public data release, public claim, product launch, token launch, or external
circulation is included.

## References

### Research and public systems

- [Open X-Embodiment: Robotic Learning Datasets and RT-X Models](https://arxiv.org/abs/2310.08864)
- [Oopsie Data](https://oopsie-data.com/), project description for real
  manipulation failures.
- [Robometer](https://robometer.github.io/), project description for reward
  learning from trajectory comparisons.
- [NVIDIA Cosmos documentation](https://docs.nvidia.com/cosmos/latest/introduction.html),
  world foundation models and physical AI.
- [Google DeepMind Genie 3](https://deepmind.google/models/genie/),
  interactive world-model description and limitations.
- [PhysWorld](https://arxiv.org/abs/2510.21447), physics-aware reconstruction
  from real videos.
- [NIST Physical AI and Data Generation for Robotics](https://www.nist.gov/programs-projects/physical-ai-and-data-generation-robotics).
- [Rhytor](https://rhytor.com/), robotics fleet risk-data product description.
- [Koop IN BOTS WE TRUST 2026 report](https://www.koop.ai/reports/in-bots-we-trust-report-2026).

These references establish adjacent public work and research questions, not
license grants, independent validation of vendor claims, market demand,
technical feasibility for this product, or acceptance of any future
experiment.
