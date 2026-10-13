# HyperAI Physical AI Evidence Studio: Business Plan

Status: working business plan; market demand and pricing are unvalidated.
State slice: hyperai.physical_ai.evidence_business_plan.v1.
Date: 2026-10-01.
Stage: concept validation; no signed pilots, verified pipeline, or revenue
evidence is represented here.
Planning baseline: HyperAI dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa on main,
with the pre-existing dirty and untracked working tree preserved.

This plan describes a proposed business and its first commercial wedge. It is
not evidence of customer demand, a financing commitment, a product capability,
or an authorization to collect data, run models, conduct a pilot, or publish
results. The accompanying product requirements document remains the technical
vision; this plan changes the commercial sequence to start from existing
robot-operation evidence rather than phone-first data collection.

## Executive decision

Build a focused B2B data-readiness and reliability-evidence workbench for
teams developing or releasing AI-enabled robot policies. Keep the first
commercial segment narrow: robot OEMs and systems integrators deploying
vision-guided manipulation in warehouse or industrial cells. Sell a paid,
fixed-scope pilot that turns customer-controlled historical records into a
reproducible, version-bound evidence packet and identifies whether the data
support the buyer's declared release or failure-review question.

The first product helps a robotics engineering or reliability team answer:

> For this robot, policy and software revision, task, and operating
> conditions, what evidence supports the proposed release, what changed from
> the baseline, and where is the evidence incomplete?

The product makes data lineage, synchronization, completeness, and condition
coverage visible before evidence is used in a buyer's decision. It does not
promise that a dataset is generally training-ready or that more data improves a
model. It does not certify safety, recommend deployment, control a robot,
underwrite insurance, or prove that a sensor record is true. The customer
retains those authorities.

The initial offer is service-assisted B2B software. Data-marketplace features
are deferred until this workflow demonstrates value and customers establish a
need for cross-party exchange. Test whether buyers will pay for a faster,
repeatable release and failure-review workflow before investing in broad
capture, simulation, shared datasets, tokens, or automated risk pricing. The
first offer tests both reduced manual data preparation and a more reproducible
release or failure review.

## Problem and buyer

### Initial customer profile

The first target account is a robotics OEM or systems integrator that:

- deploys a learned or vision-guided policy for a repeatable manipulation task;
- has more than one customer deployment or test cell;
- retains task outcomes, failures, interventions, and software/policy version
  information in logs or test records;
- must investigate field failures or compare releases using a mixture of
  dashboards, spreadsheets, and specialist engineering time; and
- can authorize a read-only, bounded review of historical evidence.

These are qualification criteria, not claims about how many such accounts
exist. The first user is a reliability, quality, or robotics engineering lead.
The budget owner is likely the VP or director accountable for deployment
quality, customer acceptance, or field-support cost; discovery must establish
who actually controls the budget.

An adjacent hypothesis is that robotics model-development teams may own a
separate budget for data cleaning, synchronization, and coverage analysis.
Interview this buyer type only to compare the paid problem and purchase path;
do not treat it as a second launch segment until a buyer demonstrates budget
ownership and a repeatable use case.

### Job to be done

When an engineering team changes a policy or deployment configuration, help it
assemble the evidence needed to investigate failures and review the change
without manually joining unrelated logs, test sheets, and version histories.

The paid outcome is a more reproducible decision process: less analyst time
spent reconstructing what happened, faster isolation of a repeated failure
condition, and a report that customer engineering teams can inspect. A
data-readiness profile may also expose the specific conditions or outcomes
missing from the evidence. A reduction in incidents, liability, insurance
premiums, or model-training cost is not promised.

### Why a buyer might act now

The International Federation of Robotics reports 5.079 million industrial
robots in operational use and 603,000 installations during 2025. This is
evidence of a large installed base, not a market-size estimate for this
software: most industrial robots are outside the initial learned-policy
segment. [IFR, World Robotics 2026 summary](https://ifr.org/ifr-press-releases/news/presidents-report-by-jane-heffner-3-2026)

The plan's commercial hypothesis is narrower: teams deploying AI-enabled
policies need stronger traceability across policy revisions, operating
conditions, exposure, interventions, and outcomes as they expand deployment.
Public research and products show that failure data, simulation evaluation,
deployment monitoring, and robot-risk records are active areas. They do not
show that this specific workflow is underserved or that buyers will pay for a
new product.

## Product and first offer

### Product promise

Create a customer-controlled evidence packet that binds:

- robot and relevant configuration;
- policy, model, firmware, and evaluation revisions where available;
- task, site or cell, and operating conditions;
- total exposure or tested episodes, including missingness in the denominator;
- successes, failures, near misses, interventions, aborts, and unknown outcomes;
- source records and their rights and custody status;
- the frozen evaluation protocol, controls, and holdout boundary; and
- the result, its uncertainty, limitations, and the human decision owner.

The product distinguishes captured records, human annotations, model
inferences, simulation results, physically reproduced results, and independent
acceptance. It never compresses them into one universal reliability or safety
score.

Each packet includes a data-readiness profile for the buyer-declared question:
source lineage and rights status; field and timestamp completeness; clock and
sensor synchronization where source fields permit the check; duplicate and
quarantine counts; and coverage across the task, environment, objects, and
perturbations declared in advance. Quality is use-specific: a record may be
adequate for one review and unusable for another. Hours and trajectory counts
are not a quality score, and coverage alone does not establish model uplift.

### MVP workflow

1. Select one task, robot/policy family, site or test cell, and release question.
2. Import a customer-selected historical export through a read-only,
   customer-controlled workflow. The first integration format is selected
   only after design-partner data-schema review; ROS 2 bags and structured
   CSV/JSON exports are candidates, not promised support.
3. Map the source fields to a narrow evidence schema. Preserve source
   references, versions, timestamps, outcome categories, units, missing values,
   and exposure counts; check synchronization and completeness only where
   source fields support those checks.
4. Compare observed coverage with a buyer-declared task/condition matrix and
   review duplicates, incomplete lineage, and anomalous or ambiguous records
   with a human. Unknown stays unknown; model readiness is not inferred.
5. State the smallest decision-relevant evidence gap and, if requested, draft
   a buyer-approved collection specification. Do not source or purchase data
   in the MVP.
6. Compare a frozen baseline and candidate release on a buyer-approved,
   bounded evaluation set, when the customer already has the necessary
   artifacts and authority.
7. Export a versioned report with evidence coverage, outcome counts,
   differences, unresolved questions, and exact limitations.

The first pilot can be manually assisted behind a private workspace. The
company should automate only the repeatable mappings and reports observed
across paid pilots. No live fleet connector, robot command path, data pooling,
data acquisition, or model training is required for this first sale.

### Explicitly deferred

- Phone-first contributor capture and an open contribution network.
- Generic video-to-physics reconstruction and claims of a digital twin.
- A new simulator or a universal cross-robot benchmark.
- Frontier-model training, autonomous policy promotion, or direct robot control.
- Cross-customer dataset pooling or data licensing without explicit rights.
- Tokens, on-chain settlement, performance-linked insurance, and automated
  credit or underwriting decisions.

Phone capture may later supplement a bounded test protocol where it adds
evidence that existing logs do not provide. It is not the first commercial
product because video alone lacks robot actions, forces, joint state, and
reliable exposure counts.

## Market, alternatives, and position

### Competitive reality

The product enters an active market and has direct competition:

| Alternative | Publicly described focus | Implication for this plan |
|---|---|---|
| RoboGate | Simulation and field-telemetry failure evidence, policy/robot/cell-scoped reports, and runtime drift monitoring. Its site also describes historical benchmark quarantine and says public automatic evaluation remains paused pending qualification. | Closest visible overlap. Do not compete on a generic reliability report; test for a specific gap in customer-controlled release evidence and data portability. Vendor statements are not independently verified. |
| Run Robotics | Rollout lineage, failure clustering, simulation evaluations, regression analysis, and robotics data quality. | Direct overlap in failure-to-evaluation workflow; validate interoperability and buyer preference before claiming differentiation. |
| RoboBlackbox | An independent record of autonomous fleet behavior, with public claims that raw fleet data stay on site and that it has no robot control path. | Overlaps on customer-controlled fleet evidence and observability. |
| Oopsie Data and Robometer | Real-robot failure collection and large trajectory/reward-learning datasets, respectively. | Establish that failure and suboptimal data are already recognized as valuable; they are not by themselves the same enterprise release-workflow product. |
| Rhytor and Koop | Robot fleet records, compliance/risk data, and insurance workflows. | Downstream risk use exists. Start with engineering evidence; do not sell underwriting or insurance outcomes. |
| NIST | Measurement, evaluation methods, testbeds, and standards work for physical AI and robotics. | Standards and measurement are important context, not a substitute for buyer-specific acceptance criteria. |

Sources: [RoboGate](https://www.robogate.io/), [Run Robotics](https://runrobotics.ai/), [RoboBlackbox](https://www.roboblackbox.com/), [Oopsie Data](https://oopsie-data.com/), [Robometer](https://robometer.github.io/), [Rhytor](https://rhytor.com/), [Koop's 2026 report](https://www.koop.ai/reports/in-bots-we-trust-report-2026), and [NIST Physical AI program](https://www.nist.gov/programs-projects/physical-ai-and-data-generation-robotics). Reviewed 2026-10-01. Company pages describe their own offerings; they do not independently establish product performance, customer adoption, or revenue.

### Differentiation hypothesis

The proposed wedge is a buyer-owned data-readiness and release-evidence
workflow that joins customer-selected field records, exposure denominators,
version history, synchronization checks, declared-condition coverage, and
bounded evaluations across the tools the customer already uses. The report
should travel with the release decision and preserve missing evidence and
negative results.

This is not yet a defensible moat. RoboGate and Run Robotics publicly describe
overlapping components. The business must earn differentiation through a
specific workflow that target buyers cannot already run with existing
products, internal tooling, or a systems integrator. If discovery finds no
material gap, narrow the offer to a repeatable integration/evidence service or
stop the product effort.

Potential longer-term defensibility comes from validated import mappings,
customer-specific evidence history, reproducible evaluation workflows, and
trusted integration into release and customer-acceptance processes. Raw data
remain customer controlled by default. A cross-customer benchmark or network
effect exists only if customers grant rights and the aggregated evidence is
representative; it must not be assumed in the business case.

### Strategic synthesis: data curation and service delivery

Two user-supplied sources suggest useful hypotheses, not validated market
facts. The [Physical AI Data Curation working note](https://madhavg.notion.site/Physical-AI-Data-curation-3c13d78aa2bf8082b4b2c60a06934e9b)
argues that billed hours are a poor proxy for data utility, that
synchronization and cleaning are costly, and that failures can expose specific
coverage gaps. The ["Intelligence Arbitrage" essay](https://sdan.net/blog/intelligence-arbitrage)
argues that service businesses may use AI to improve delivery economics while
building an intelligence advantage from operational work. Neither source
establishes buyer demand, achievable model uplift, robotics-service margins,
or a viable acquisition strategy for this business. The cited numerical
claims and research leads in the working note are excluded from the market
model pending primary-source review.

The practical implication is to test a service-assisted route to product:
enter through a paid data-readiness and release-evidence engagement, then
standardize the repeated mapping, synchronization, coverage, and review work.
A systems integrator or robot-service operator could later be a delivery
partner with recurring deployment workflows and an authorized data path. The
potential compounding asset is reusable processing and evidence methods, not
pooled raw customer data. Contracts must separately specify access to raw
records, derived artifacts, and any model-training or cross-customer use.

The scale sequence is gated: (1) sell a bounded diagnostic; (2) prove that
buyers pay for the resulting time savings or decision value; (3) convert
repeated manual work into a software workflow; and only then (4) test
buyer-authorized, gap-directed data collection or operator-embedded delivery.
No data marketplace, automatic bounty, robot-service acquisition, or model
training is included in the current base case. Consider an acquisition only
after separate financial, operational, liability, rights, and capital
diligence, and under a new authorization.

### Bottom-up market sizing

Do not multiply the global robot count by a software price and call it TAM.
The relevant serviceable market is the number of reachable organizations that
deploy learned policies, have usable evidence, and own a budget for this
workflow.

Management should build and qualify a named North American account list before
using a market-size figure in fundraising. As a planning sensitivity only, 100
qualified accounts at an eventual annual contract value of USD 60,000–120,000
would represent USD 6–12 million in possible recurring revenue. Both the
account count and price are unvalidated assumptions; this arithmetic is not a
published market estimate. Expand the account universe only after the first
segment converts and renews.

## Go-to-market

### Sales motion

Use founder-led, consultative sales to robotics OEMs and integrators. Lead with
a paid evidence review for one concrete release or recurring failure question.
The first sales process is:

1. Interview the engineering user and budget owner separately.
2. Map the current release-review process, systems, time spent, and decision
   delays without requesting sensitive media.
3. Review a customer-provided schema or synthetic export under the appropriate
   written authorization.
4. Agree on the task, source fields, denominators, acceptance criteria,
   retention, and success measure before quoting a pilot.
5. Deliver a fixed-scope pilot and ask for annual conversion only after the
   buyer reviews the evidence and measured value.

Likely routes to qualified introductions include robotics systems integrators,
robotics accelerators and applied-research programs, and safety/reliability
consultants. These are channel hypotheses, not existing partnerships.

### Design-partner program

Before substantial implementation, target 15–20 structured interviews and
five data-schema reviews. Use discovery to compare the value buyers place on
release evidence with the cost of data cleaning, synchronization, and coverage
analysis; keep robotics OEMs and integrators as the launch segment unless the
comparison shows a materially stronger, budgeted alternative. The initial
qualification gate is three independent organizations willing to define the
same core problem, provide a lawful path to representative historical
evidence, identify a budget owner, and consider a paid pilot. Interest, a
waitlist entry, or a nonbinding letter is not a paid validation.

Pilot scope: 8–10 weeks; one robot/policy family; one repeated task; one site
or test cell; historical, read-only exports; one baseline and one release
question; a buyer-declared data quality and condition-coverage profile; no
robot control; no training use; and no onward data use. A pilot
contract must define data purpose, access, retention/deletion, derived
artifacts, publication prohibition, liability allocation, and stop conditions.
Legal counsel must review customer terms and any handling of personal,
confidential, or safety-relevant data before a real pilot.

### Pilot success criteria

The buyer and operator pre-agree the baseline and metric before each pilot.
Candidate hypotheses for negotiation—not universal product guarantees—are:

- produce the first inspectable evidence packet within ten business days of
  receiving a validated export;
- reduce analyst time for the selected failure/release review by at least 30%
  against a measured baseline, or meet another buyer-selected economic outcome;
- preserve the source-to-report lineage and expose missing denominators rather
  than silently dropping records; and
- identify at least one buyer-confirmed data-quality or condition-coverage gap
  that changes the next review, evaluation, or authorized data request; and
- obtain a paid annual conversion or a second paid evaluation from at least
  two of the first three completed pilots.

If the required source fields or denominator are unavailable, report the
analysis as incomplete and use the pilot to test data readiness. Do not infer a
reliability rate from incomplete exposure records.

## Revenue model and pricing hypotheses

Charge for workflow value and repeatability, not submitted data volume:

| Offer | Proposed price | Scope and qualification |
|---|---:|---|
| Design-partner pilot | USD 20,000–35,000 fixed fee | 8–10 weeks, one task/release question and one site or test cell; price hypothesis to test with paid proposals. |
| Team license | USD 60,000–90,000 per year | One engineering team and a bounded number of task/policy workflows; final packaging depends on observed use and support burden. |
| Enterprise license | USD 120,000–240,000 per year | Multiple sites or teams, customer-managed deployment, audit controls, and agreed integrations. |
| Integration services | USD 15,000–50,000 one time | Only for repeatable, scoped connectors and migration work; avoid uncapped custom consulting. |

The deliverable unit is one customer-authorized dataset slice and one declared
release or failure-review question, with the preparation work, coverage, and
limitations visible. Package by decision scope and integration effort, not by
raw recording hours or trajectory count. Revisit usage pricing only after
measuring accepted records, preparation cost, and buyer-confirmed utility.
Do not price per successful outcome or promise model uplift.

There is no verified public competitor price benchmark in this scan. These are
initial willingness-to-pay hypotheses, not established market rates. Quote the
pilot against the customer's measured annual cost of engineering review,
delayed acceptance, and repeated integration work. Do not claim avoided
incidents or insurance savings as ROI without buyer-specific evidence.

For each pilot, estimate buyer value as:

**hours saved × buyer's loaded engineering cost + buyer-verified value of
shorter release/acceptance cycles − pilot and recurring fees.**

Treat every term as unknown until measured with the customer. Annual contracts
should renew because the workflow is used for successive releases and sites,
not because the customer expects a speculative token or data royalty.

## Operating plan and milestones

| Period | Company objective | Decision gate |
|---|---|---|
| Months 0–3 | Conduct discovery; qualify the first task and budget owner; review schemas; write the pilot contract, data map, and offline evidence-report specification. | Proceed only if three organizations confirm the same expensive workflow and at least two will consider a paid pilot at the proposed range. |
| Months 4–6 | Under a separately authorized implementation slice, build a synthetic end-to-end vertical slice and one customer-approved read-only import path. Offer the first paid pilot. | Stop or rescope if source data cannot bind policy/version, task context, outcomes, and exposure well enough for the buyer's decision. |
| Months 7–12 | Complete up to three paid pilots; standardize only recurring import and report steps; convert at least two accounts to annual contracts. | Seek a repeatable deployment under 4 weeks and a customer-verified value case. These are targets, not current capability. |
| Months 13–18 | Reach seven annual customers in the planning case, add a second validated source adapter, and establish renewal evidence. | Raise or expand only if paid conversion, gross margin, data rights, and referenceability support a repeatable business. Otherwise narrow to a profitable specialist service or stop. |

Initial team hypothesis: a commercial founder/product lead, two engineers
covering data/evidence systems and robotics integrations, and one
customer-facing reliability/evaluation lead. Use specialist contractors for
robotics hardware, security, and legal work until customer evidence justifies
full-time roles. The plan does not assume a large model-training team.

## Financial model

The following is a management planning case, not a forecast or valuation.
There is no validated pipeline, signed customer, observed sales cycle, or
customer-approved price in this record. Recognized revenue assumes annual
contracts begin throughout each year; recurring revenue is shown separately.
Amounts are USD.

| Year | Paid pilots | Annual customers at year end | Assumed annual contract value | Ending ARR | Illustrative recognized revenue |
|---|---:|---:|---:|---:|---:|
| 1 | 3 × USD 25k | 2 | USD 60k | USD 120k | USD 150k |
| 2 | 6 × USD 25k | 7 | USD 70k | USD 490k | USD 565k |
| 3 | 10 × USD 30k | 18 | USD 80k | USD 1.44m | USD 1.50m |

Recognized-revenue arithmetic uses an average of 1, 4.5, and 12.5 subscription
customers during Years 1–3, respectively, plus pilot and integration fees of
USD 15k, USD 100k, and USD 200k. The assumptions imply approximately 60%, 65%,
and 75% blended gross margin as delivery becomes more repeatable. These
figures need to be replaced with actual pilot labor, infrastructure, support,
renewal, and sales-cycle data. Data preparation may remain labor-intensive;
automation and improved gross margin are unproven assumptions.

Illustrative operating expense excluding direct delivery costs is USD 1.2m in
Year 1, USD 1.55m in Year 2, and USD 2.1m in Year 3. Under the revenue and
margin assumptions above, the business remains loss-making throughout these
three years. This software-led venture uses service-assisted delivery and must
prove repeatability and secure follow-on capital; the model does not imply
early break-even.

### Initial capital plan

Raise approximately USD 2.3m to fund an 18-month validation and product
period, conditional on founder compensation, hiring location, customer
integration needs, and insurance/legal quotes. This is a planning target, not
a financing commitment. The operating case is software and scoped services; it
excludes an owned robot fleet, operator acquisitions, revenue-share income, and
data royalties. Proposed allocation:

Amounts below are rounded planning estimates.

| Use | Share | Approximate amount |
|---|---:|---:|
| Core product, data, and reliability team | 48% | USD 1.10m |
| Customer integrations and pilot delivery | 13% | USD 0.30m |
| Compute, storage, and test equipment | 7% | USD 0.16m |
| Security, privacy, legal, and insurance | 10% | USD 0.23m |
| Customer discovery, sales, and travel | 8% | USD 0.18m |
| Contingency and working capital | 14% | USD 0.32m |
| **Total** | **100%** | **USD 2.30m** |

Do not spend the full plan before the paid-pilot gate. If three paid pilots
cannot be secured by month 6, freeze hiring and test a narrower evidence
service before raising or building a platform. The next financing decision
depends on real annual conversions and delivery economics, not robot-market
growth statistics.

## Risks and controls

| Risk | Business response |
|---|---|
| Existing competitors already cover the proposed workflow | Validate the exact unmet job with buyers; win on integration or customer-controlled evidence only if buyers show a material gap. |
| Robot logs omit policy identity, exposure, or outcomes | Make data-readiness review part of qualification; return an explicit incomplete result instead of inventing denominators. |
| Pilot becomes custom consulting | Limit each pilot to one task and release question; charge separately for integrations; productize only repeated work. |
| Data volume is mistaken for data utility or drives low-quality collection | Define the buyer's downstream question and coverage matrix first; report quality, missingness, cost, and rights; keep acquisition and bounty mechanisms out of the first offer. |
| Service-embedded strategy adds operational liability or capital intensity | Begin with commercial integration partners; do not own robot operations or acquire a service provider without a new diligence and authorization gate. |
| Customer data or site video is sensitive | Keep customer custody by default; use local/offline processing when required; obtain written purpose, access, retention, and deletion terms before transfer. |
| A report is mistaken for a safety certification | Put scope and evidence status on each report; do not issue safety or insurance decisions. |
| Model or simulation output looks more certain than evidence permits | Keep observed, inferred, simulated, physical, and independently accepted evidence separate; preserve unknowns and failed runs. |
| Contributor incentives encourage unsafe exploration or incident suppression | No open data bounty in the first product; later compensation must reward complete evidence, including negative and inconclusive outcomes, under a controlled protocol. |
| Long enterprise sales cycles consume capital | Require a named budget owner and paid pilot proposal early; use month-6 and month-12 stop gates. |
| Crypto adds friction without solving a real trust problem | Use ordinary contracts and payment first; consider neutral escrow or shared provenance only when a real multi-party workflow requires it. |

## Business validation questions

The first discovery cycle must establish:

1. Which role owns the release or failure-review budget?
2. What decision was delayed or made with incomplete evidence in the last
   quarter, and what did the current review process cost?
3. Which robot, policy, firmware, task, site, and exposure fields can be
   recovered from records today?
4. How much time and money does the buyer spend cleaning and synchronizing
   robot data, and which quality or coverage gaps actually change an
   evaluation or release decision?
5. Which product or internal tool already solves the problem, including
   RoboGate, Run Robotics, and customer-specific systems?
6. What data can remain in the customer's environment, and what minimum
   output can be shared with a vendor?
7. Will a buyer pay USD 20,000–35,000 for the first bounded pilot, and what
   conversion price and procurement path would follow?
8. Which operator or integrator could deliver the workflow repeatedly while
   preserving customer rights, and what economics would justify the channel?
9. What result would make the customer repeat or renew, including a result
   that narrows the operating envelope?

The business thesis is falsified or materially weakened if qualified buyers
will not pay for a bounded pilot, the necessary lineage and denominator do not
exist, current tools already satisfy the workflow, or measured savings are
small relative to delivery cost. The curation extension is weakened if
cleaning and synchronization costs are immaterial, buyers cannot define
decision-relevant coverage, or a data-readiness profile does not change a
paid review.

## Assurance, authority, and publish boundary

This business plan is documentation only under
hyperai.physical_ai.evidence_business_plan.v1. It grants no code changes, real
data access, data capture or transfer, provider calls, model or simulator
execution, payment, chain activity, contact with customers or reviewers,
physical testing, publication, or deployment. Any prototype, pilot, external
data, or public claim requires its own named state slice, rights and custody
packet, budget, controls, claim ceiling, and stop conditions.

The relevant current HyperAI repo work remains distinct: the compute-market
plan composes AI workloads and compute supply; this proposed physical-AI
product would organize robotics evidence. The two are adjacent product lines,
not one implemented marketplace.

Assurance level: no assurance level assigned; this is a planning artifact.
Reviewer status: NOT_REQUESTED. Reproducibility status: NOT_APPLICABLE.
Data and artifact custody: public-source research links and management
assumptions only; no customer or robot data was collected. Known limitations:
no buyer interviews, signed pilots, verified pricing, named account list,
competitive teardown, legal review, or product implementation. Claim ceiling:
business hypothesis and illustrative operating case only. Next blocked gate:
customer discovery and a separately authorized pilot packet.

Publish boundary: local planning only. No commit, PR, financing solicitation,
public release, or external circulation is included.

## Sources

- [Intelligence Arbitrage](https://sdan.net/blog/intelligence-arbitrage), strategy essay, 2024-12-04. Used as a service-delivery business-model hypothesis, not as robotics-market validation.
- [Physical AI Data Curation](https://madhavg.notion.site/Physical-AI-Data-curation-3c13d78aa2bf8082b4b2c60a06934e9b), user-supplied working note, accessed 2026-10-01. Its market estimates and linked performance claims are leads for primary-source review, not verified evidence in this plan.
- [International Federation of Robotics, 2026 World Robotics summary](https://ifr.org/ifr-press-releases/news/presidents-report-by-jane-heffner-3-2026).
- [RoboGate](https://www.robogate.io/), product description and current public qualification notice.
- [Run Robotics](https://runrobotics.ai/), product description.
- [RoboBlackbox](https://www.roboblackbox.com/), product description.
- [Oopsie Data](https://oopsie-data.com/), project description.
- [Robometer](https://robometer.github.io/), dataset and research description.
- [Rhytor](https://rhytor.com/), company product description.
- [Koop, IN BOTS WE TRUST 2026](https://www.koop.ai/reports/in-bots-we-trust-report-2026).
- [NIST Physical AI and Data Generation for Robotics](https://www.nist.gov/programs-projects/physical-ai-and-data-generation-robotics).
- [RAPT: Model-Predictive OOD Detection and Failure Diagnosis](https://arxiv.org/abs/2602.01515).

Public company pages are self-descriptions. They are cited to map visible
alternatives, not as independent verification of customer counts, performance,
revenue, or benchmark results.
