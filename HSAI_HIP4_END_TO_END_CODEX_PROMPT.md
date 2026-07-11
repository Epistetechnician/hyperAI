# Codex Master Build Prompt — HSAI × Hyperliquid HIP-4

Replace `<ABSOLUTE_PATH_TO_HSAI_REPOSITORY>` with the real repository path, then give this entire prompt to Codex.

---

You are GPT-5.5/Codex acting as a principal distributed-systems engineer, applied cryptography engineer, protocol integration engineer, security engineer, SRE, and product-minded technical lead.

You are working in this repository:

`<ABSOLUTE_PATH_TO_HSAI_REPOSITORY>`

The current date is July 10, 2026.

## Mission

Build, integrate, test, and document an end-to-end **HSAI Verifiable Outcome Protocol** whose first domain is objectively verifiable computational challenges and whose first settlement backend is Hyperliquid HIP-4.

The system must let an operator:

1. Define a precise computational claim before trading begins.
2. Freeze and content-address every resolution-critical input.
3. Observe eligible challenge submissions without silently missing or reordering them.
4. Execute an untrusted validator in a controlled and reproducible environment.
5. Distinguish execution failure from a false claim.
6. Produce a portable, independently verifiable evidence bundle and signed resolution attestation.
7. Apply an explicit dispute and finality policy.
8. Bind the frozen claim to an external outcome market without relying on name matching.
9. Prepare and, only where officially supported and explicitly enabled, publish settlement through a Hyperliquid HIP-4 adapter.
10. Reconcile the external settlement state with HSAI and retain a complete audit trail.

The intended MVP wedge is computational challenge markets such as zk.golf, ecdsa.fail, ZK Hack, Paradigm CTF, and deterministic autoresearch or benchmark challenges. The initial canonical claim shape is:

> “Did at least one eligible submission satisfy the pinned validator before the pinned deadline under the frozen claim specification?”

The wider product is not “a HIP-4 plugin.” It is a settlement-backend-neutral verification and evidence protocol. HIP-4 is the first integration.

Do not merely produce another design document. Inspect the existing codebase, form a concrete plan, implement the system, run the tests, exercise the end-to-end demo, and leave the repository in a professional handoff state.

## Product context to preserve

HSAI may already contain some or all of the following:

- structured admission decisions;
- policy evaluation and execution gates;
- evidence packets and execution traces;
- combined proof packets;
- cryptographic digests;
- replayable or deterministic execution;
- repository inspection and patch validation;
- benchmark execution, measurement, scoring, and evidence recording;
- safety, execution, and admission policy layers;
- Mesh integration or state slices such as `mesh.hsai_admission_bridge.v1`, `mesh.run_admission.v1`, and `mesh.run_export.v1`.

Treat those statements as hypotheses until verified in the repository. Reuse mature primitives and extend compatible schema versions where possible. Do not duplicate a working registry, evidence, signing, replay, policy, or API subsystem under a new name. Do not force a rewrite into a different language or framework just to match this prompt.

If the repository is greenfield or lacks a suitable implementation stack, use Rust for the domain core, canonicalization, orchestration, replay verification, and CLI. Use PostgreSQL for durable relational state, an S3-compatible content-addressed blob store for larger immutable artifacts, and the repository’s existing service framework for HTTP APIs if one exists. A local development profile may use SQLite and a filesystem blob store only if behavior remains compatible with the production interfaces.

## Required working method

### 1. Inspect before editing

Start by reading, in this order:

- every applicable `AGENTS.md` and repository instruction file;
- `README` and architecture documents;
- package/workspace manifests and lockfiles;
- existing admission, policy, runner, proof/evidence, replay, registry, API, persistence, and Hyperliquid-related code;
- relevant schemas, migrations, fixtures, and tests;
- `git status` and the existing diff.

Map what exists to the target architecture. Preserve unrelated user changes and do not use destructive git commands.

### 2. Create an implementation plan

Before material changes, publish a concise plan tied to repository paths and maintain it as work progresses. Include:

- what can be reused unchanged;
- what must be extended;
- what is genuinely missing;
- migration and compatibility risks;
- any external capability that is not yet documented or available.

Then implement the plan. Do not stop after reconnaissance unless blocked by a missing authority or secret that cannot safely be mocked.

### 3. Verify current protocol facts

Before coding against HIP-4 or any challenge platform, consult current primary sources and record the access date and relevant interface version in `docs/references.md` or the repository’s equivalent. Prefer official Hyperliquid documentation, official APIs, official SDK repositories, and the challenge platform’s official repositories or specifications.

At minimum verify these current official sources:

- HIP-4 outcome markets: `https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-4-outcome-markets`
- Hyperliquid Asset IDs: `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/asset-ids`
- Hyperliquid Info endpoint: `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint`
- Hyperliquid Exchange endpoint and signing documentation, if implementing any write path.

As of this prompt’s date, official documentation describes outcome contracts as fully collateralized and settling in a bounded range, exposes outcome metadata through `outcomeMeta`, and documents binary side encoding as:

```text
encoding = 10 * outcome_id + side_index
coin = "#" + encoding
token = "+" + encoding
action_asset_id = 100_000_000 + encoding
```

Only side indices documented as valid by the current official specification may be encoded. Parse `sideSpecs` from metadata and do not assume display labels are always literally `YES` and `NO`.

Documentation and rollout state can disagree across testnet, mainnet, public API pages, and SDKs. Treat observed discrepancies as capability uncertainty, not permission to guess. Capture a dated capability matrix and make unsupported mutations fail closed.

### 4. Keep risky external actions disabled

Do not:

- submit a mainnet transaction;
- deploy or register a live market;
- trade, split, merge, negate, or move user funds;
- settle a live market;
- generate, request, or log a production private key;
- scrape around access controls or violate a challenge platform’s terms;
- assert that HSAI is the authoritative HIP-4 broadcaster without verified authorization.

All external writes must be behind explicit feature and runtime gates, default to dry-run, and target a mock or official testnet only. Mainnet mutation must remain structurally disabled in this implementation. Read-only public API calls are acceptable when permitted, but tests must not depend on live services.

## Non-negotiable truth and claim boundaries

Encode these boundaries in types, docs, tests, and user-facing terminology:

1. **HSAI can determine and attest to a claim under a frozen policy; it does not automatically become the onchain settlement authority.** HIP-4 settlement still depends on the deployed market’s authorized mechanism and available official actions.
2. **A digest is not a proof.** A hash proves integrity relative to bytes, and a signature proves that a key attested to bytes. Call an artifact a cryptographic proof only if verification of the artifact actually establishes the claimed statement under a documented proof system.
3. **A validator process error is not `false`.** Timeouts, crashes, source outages, missing artifacts, runner errors, equivocation, and insufficient quorum produce an unresolved or inconclusive state unless the frozen policy explicitly and safely defines another result.
4. **A failed candidate does not make an existential claim false.** For “at least one valid solution before deadline,” the system must continue observing other eligible candidates. `false` requires the deadline to pass, the grace period to complete, and the source-observation completeness policy to be satisfied.
5. **Proof generation may be nondeterministic while proof verification is deterministic.** Prefer settlement predicates based on deterministic verification of a pinned artifact rather than reproducing nondeterministic search or proof generation byte-for-byte.
6. **Reproducibility is scoped.** Distinguish byte-identical replay, normalized semantic replay, and quorum agreement. Never promise universal determinism across unspecified CPU architectures, kernels, clocks, networks, or concurrency settings.
7. **Frozen market semantics never change in place.** A changed specification, verifier, deadline, eligibility rule, fallback, dispute policy, or outcome mapping creates a new claim identity and requires a new market binding.
8. **External market metadata is not the claim specification.** Bind a market to a frozen HSAI claim by explicit identifiers and digests. Names, descriptions, and labels are presentation metadata, never the sole binding.
9. **Public evidence must be sufficient to validate the resolution without leaking prohibited secrets.** If an exploit or submission cannot be public immediately, freeze and attest to the private artifact, publish a separately hashed disclosure policy, and mark the public-verifiability limitation explicitly.
10. **No settlement on ambiguity.** Invalid specs, conflicting runs, stale bindings, insufficient signatures, incomplete observation, active disputes, and unsupported backend capabilities must fail closed.

## MVP scope

### In scope

- Binary computational claims with explicit `true`, `false`, and unresolved/inconclusive operational states.
- Challenge registry and immutable claim-spec registry.
- A generic challenge-source adapter interface.
- Separate interfaces for challenge corpora, live submission observation, and agent/evaluation harnesses; never treat one as evidence supplied by another.
- A generic containerized validator adapter.
- At least one fully runnable local fixture adapter.
- Recorded-fixture adapters for zk.golf and ecdsa.fail, plus live read-only modes only where an official stable source interface is confirmed and legally usable.
- Structured corpus adapters for NYU CTF Bench, CTF Archives, the pwn.college CTF Archive, and the CryptoHack CTF Archive.
- A manifest-overlay Git corpus adapter exercised against the public challenge repositories maintained by josephsurin, maple3142, and alex-bellon without duplicating repository-specific business logic.
- An NYU CTF agent-harness integration for D-CIPHER and the NYU baseline, isolated from the claim-resolution core.
- A 247CTF manual/recorded-fixture integration unless the platform provides an official automation API and explicit authorization for the intended use.
- Candidate ingestion, deduplication, source cursoring, and completeness evidence.
- Sandboxed deterministic verification.
- Evidence manifests, content-addressed storage, replay, signatures, and public verification.
- Resolution state machine, disputes, finality, and quorum policy.
- Public read API, protected operator API, CLI, and SDK-quality schemas.
- Hyperliquid HIP-4 metadata discovery and explicit market binding.
- Mock HIP-4 settlement backend and deterministic unsigned settlement-intent generation.
- Official testnet publisher only if the exact current action, signer role, and authorization model are verified from primary sources and can be tested safely.
- Settlement reconciliation, audit log, observability, migrations, deployment examples, and runbooks.

### Out of scope for this build

- Mainnet settlement or trading.
- Custody of user funds.
- A consumer exchange or trading UI.
- Market making, liquidity incentives, or automated trading strategies.
- Subjective event resolution.
- LLM-as-judge settlement.
- Elections, sports, governance, legal outcomes, or macro events.
- Open-ended autonomous-agent tasks without a pinned deterministic evaluator.
- Token issuance, securities structuring, or legal/regulatory conclusions.
- Calling opaque model scores “objective truth.”
- Automated interaction with authenticated CTF services without explicit platform authorization.
- Attacking live targets, third-party infrastructure, or any system outside a locally rehosted or explicitly authorized challenge environment.

## Required system architecture

Implement the following logical boundaries. Match existing repository conventions, but preserve the separation of concerns.

```text
Pinned challenge corpus -> corpus adapter -> frozen challenge package + verifier
Frozen ClaimSpec ---------------------------> run/observation policy
Agent harness or external solver -> submission source + observation cursor
    -> candidate intake + admission/policy
    -> frozen package + isolated verification runner
    -> normalized run result
    -> immutable evidence bundle
    -> attestation/quorum engine
    -> provisional resolution
    -> dispute/finality engine
    -> final resolution
    -> settlement-backend adapter
    -> submission receipt + reconciliation
    -> permanent audit/evidence archive
```

Required modules or their repository-native equivalents:

1. `outcomes-domain`: versioned schemas, canonicalization, IDs, invariants, state machines.
2. `outcomes-registry`: claim, challenge, verifier, and market-binding registries.
3. `outcomes-corpus`: corpus snapshots, normalized packages, manifest overlays, solver views, and rights/provenance.
4. `outcomes-source`: submission-source adapter port, observation snapshots, cursors, and completeness.
5. `outcomes-harness`: agent/evaluation harness port, run plans, candidates, traces, and cost telemetry.
6. `outcomes-runner`: admission, sandbox configuration, execution, normalization, replay.
7. `outcomes-evidence`: manifests, content addressing, signatures, public verification.
8. `outcomes-resolution`: aggregation, quorum, disputes, finality, outcome mapping.
9. `outcomes-settlement`: backend-neutral port, intent generation, idempotency, reconciliation.
10. `hyperliquid-hip4`: metadata discovery, asset encoding, capability detection, binding validation, mock/dry-run/testnet implementation as available.
11. `outcomes-api`: public and operator endpoints plus OpenAPI.
12. `outcomes-cli`: local authoring, validation, replay, verification, evidence inspection, and safe settlement preflight.
13. `outcomes-worker`: durable corpus import, ingestion, verification, finalization, and settlement jobs if the repository uses workers.

Dependencies must point inward toward stable domain interfaces. The domain core must not import HTTP, PostgreSQL, cloud storage, Docker, Hyperliquid SDK, or challenge-specific code.

## Canonical domain model

Define versioned JSON Schemas and native types. Use repository-native schema tooling if already established. Reject unknown resolution-critical fields unless the schema explicitly allows an extension namespace.

### 1. `ChallengeDescriptorV1`

Required fields:

- `schema_version`
- `challenge_id`
- `source_platform`
- `source_namespace`
- `source_challenge_ref`
- `display_name`
- `canonical_spec_uri`
- `canonical_spec_digest`
- `source_terms_uri`
- `source_timezone`
- `source_adapter_kind`
- `source_adapter_version`
- `created_at`
- optional display-only metadata clearly separated from resolution-critical data.

### 2. `ClaimSpecV1`

This is the immutable semantic object frozen before market binding. It must include:

- `schema_version`
- `claim_id` derived from the canonical digest, not supplied arbitrarily;
- `challenge_descriptor_digest`
- human-readable `question`, marked non-authoritative;
- machine-readable `predicate`
- `eligible_submission_policy`
- `candidate_identity_policy`
- `observation_policy`
- `verification_policy`
- `resolution_policy`
- `deadline_policy`
- `dispute_policy`
- `attestation_policy`
- `evidence_publication_policy`
- `outcome_mapping`
- `created_at`
- `freeze_not_before`
- `extensions` under namespaced keys only.

For the MVP predicate, model the semantics directly instead of evaluating arbitrary code:

```text
exists candidate c such that
  eligible(c, frozen_policy)
  AND observed_source_timestamp(c) <= submission_deadline
  AND verify(c, pinned_verifier) == PASS
```

Do not implement a general `eval`, embedded shell command, JavaScript expression, SQL fragment, or unbounded policy language. If the repository already has a safe policy DSL, use a versioned subset with deterministic semantics.

The deadline policy must specify:

- authoritative time source;
- inclusive or exclusive boundary;
- canonical UTC timestamp;
- permitted source-clock skew;
- post-deadline ingestion grace period;
- required source finality or confirmation depth;
- behavior if the source clock or API is unavailable;
- no-solution finalization prerequisites.

The eligibility policy must pin:

- accepted source namespaces or repositories;
- author/team identity rules if applicable;
- allowed artifact types;
- duplicate and replacement semantics;
- whether withdrawn or edited submissions count;
- maximum artifact size;
- content license or disclosure constraints;
- policy for submissions observed before the deadline but fetched afterward.

The verification policy must pin:

- validator artifact digest;
- validator interface/ABI version;
- OCI image digest or reproducible environment digest;
- command/entrypoint as structured argv, never an interpolated shell string;
- CPU architecture and required instruction set;
- OS/runtime/toolchain versions;
- filesystem layout;
- network policy;
- environment allowlist;
- fixed locale and timezone;
- randomness policy and seed derivation;
- clock policy;
- resource limits;
- timeout and retry policy;
- output normalization rules;
- PASS/FAIL/INCONCLUSIVE/ERROR mapping;
- determinism tier and required replay count;
- required runner/attestor quorum.

The resolution policy must state when a provisional result can become final. It must never map infrastructure failure to `false` by default.

### 3. Canonicalization and identifiers

Use a documented, cross-language canonical representation. Prefer RFC 8785 JSON Canonicalization Scheme if compatible with the existing stack. If the repository already has a rigorously specified canonical format, extend it rather than introducing a competing one.

Requirements:

- no floating-point values in resolution-critical schemas;
- prices, resource quantities, and decimal values represented as bounded integers with units or canonical decimal strings;
- timestamps normalized to UTC with a specified precision;
- Unicode normalization policy documented and tested;
- maps ordered by the canonicalization algorithm;
- absent, `null`, and empty values semantically distinct only where the schema says so;
- integer bounds enforced identically across languages;
- domain separation for every digest.

Example identity construction:

```text
claim_spec_digest = SHA-256(
  "hsai.claim-spec.v1\0" || canonical_json(claim_spec_without_derived_id)
)
claim_id = "claim_" || lowercase_base32(claim_spec_digest)
```

Use constant-time digest comparisons where security-relevant. Provide golden canonicalization vectors and a standalone verification command.

### 4. Operational state is separate from frozen semantics

Do not mutate `ClaimSpecV1` to record progress. Model operational state with append-only events and projected views.

At minimum define:

#### Verification execution outcome

- `PASS`
- `FAIL`
- `INCONCLUSIVE`
- `ERROR`

#### Claim truth value

- `TRUE`
- `FALSE`
- `UNRESOLVED`

#### Resolution finality

- `NONE`
- `PROVISIONAL`
- `DISPUTED`
- `FINAL`

#### Settlement lifecycle

- `UNBOUND`
- `BOUND`
- `NOT_READY`
- `READY`
- `INTENT_BUILT`
- `SUBMITTED`
- `CONFIRMED`
- `RECONCILIATION_FAILED`
- `PAUSED`

Use an explicit transition table enforced in code and database constraints where possible. Illegal transitions must produce typed errors and audit events.

Important invariants:

- A frozen claim cannot be edited or deleted through normal APIs.
- A final resolution cannot be replaced. A correction is a separately signed superseding record that cannot silently drive the original market.
- `FALSE` for an existential challenge cannot be finalized before the deadline, grace period, and observation-completeness conditions are satisfied.
- `TRUE` requires at least one eligible PASS run satisfying the configured quorum.
- `ERROR` and `INCONCLUSIVE` cannot directly satisfy either outcome.
- An active dispute prevents settlement readiness.
- Settlement readiness requires a final resolution, valid signatures, an exact market binding, supported backend capability, and no pause.
- The same final resolution and market binding produce the same settlement idempotency key.
- No claim can bind to two sides of the same market, and a market side cannot be ambiguously bound to conflicting claims.

## Persistence and event model

Use transactional persistence and an append-only audit/event history. If the repository already uses event sourcing, follow it. Otherwise, use normalized current-state tables plus immutable events and a transactional outbox.

Provide migrations for the repository’s supported database. Suggested logical tables:

- `challenge_descriptors`
- `corpus_descriptors`
- `corpus_snapshots`
- `challenge_packages`
- `runtime_manifests`
- `verifier_manifests`
- `solver_view_manifests`
- `rights_provenance_assessments`
- `adapter_capability_snapshots`
- `agent_harness_descriptors`
- `agent_run_plans`
- `agent_runs`
- `agent_trace_bundles`
- `claim_specs`
- `claim_events`
- `observation_snapshots`
- `observation_cursors`
- `candidates`
- `candidate_artifacts`
- `verification_runs`
- `run_attempts`
- `evidence_manifests`
- `evidence_blobs`
- `attestations`
- `resolution_records`
- `disputes`
- `market_bindings`
- `settlement_intents`
- `settlement_attempts`
- `settlement_receipts`
- `idempotency_keys`
- `outbox_events`
- `audit_events`
- `operator_pauses`

Database requirements:

- primary and foreign keys with explicit deletion behavior;
- uniqueness on all content digests and external identities;
- optimistic concurrency or row locking for state transitions;
- append-only protection for frozen specs, evidence, attestations, and audit events;
- idempotent ingestion and job execution;
- no plaintext secrets;
- migrations tested from an empty database and from the repository’s current schema;
- timestamps for receipt and source time stored separately;
- external raw payloads retained by digest, with size limits;
- durable outbox delivery rather than fire-and-forget cross-system events.

Every material transition must record:

- event ID;
- aggregate ID and previous version;
- event type and schema version;
- actor type and actor ID;
- request/correlation/trace ID;
- canonical payload digest;
- receipt time;
- source time if applicable;
- code/build version;
- previous-event digest if the existing evidence model supports hash chaining.

## Challenge, corpus, submission, and agent adapter contracts

Do not force static challenge archives, live submission feeds, and agent frameworks through one misleading abstraction. Implement three small versioned ports that share provenance and artifact primitives.

### 1. Challenge corpus adapter

A corpus adapter imports a pinned challenge definition and the materials needed to rehost or verify it. It does **not** prove that every candidate submission to a live platform was observed.

```text
ChallengeCorpusAdapter
  describe_corpus(ref) -> CorpusDescriptor
  snapshot_corpus(ref) -> CorpusSnapshot
  list_challenges(snapshot, cursor) -> ChallengePage
  describe_challenge(snapshot, challenge_ref) -> ChallengeDescriptor
  materialize_challenge(snapshot, challenge_ref) -> ChallengePackage
  derive_runtime_manifest(package) -> RuntimeManifestResult
  derive_verifier_manifest(package) -> VerifierManifestResult
  assess_license_and_provenance(package) -> RightsAndProvenanceAssessment
  health() -> AdapterHealth
```

A `CorpusSnapshot` must pin the resolved Git commit, release digest, dataset version, submodules, Git LFS objects, OCI images, external downloads, and every transitive artifact needed for replay. Mutable branches and tags may be accepted only as discovery inputs and must resolve to immutable identities before a claim is frozen.

A corpus import must distinguish:

- public/release files visible to the solver;
- private/server files visible only to the isolated challenge runtime;
- verifier or flag material visible only to the result checker;
- organizer solutions and writeups, which must be excluded from the solver view and recorded as contamination-sensitive artifacts;
- build inputs and remotely referenced base images;
- challenge metadata that is descriptive versus resolution-critical.

The adapter must never expose a stored flag, solution, server secret, hidden test, or organizer writeup to an evaluated agent. Add an explicit solver-view manifest and assert in tests that secret digests cannot be reached from that view.

### 2. Submission-source adapter

This adapter observes candidates from a source that can support the frozen claim’s deadline and completeness semantics:

```text
SubmissionSourceAdapter
  describe_challenge(ref) -> ChallengeDescriptor
  snapshot(cursor, cutoff) -> ObservationSnapshot
  list_candidates(snapshot) -> [CandidateEnvelope]
  fetch_candidate(candidate_ref) -> CandidateArtifact
  verify_source_provenance(candidate) -> ProvenanceResult
  assess_completeness(snapshot, deadline_policy) -> CompletenessAssessment
  health() -> AdapterHealth
```

An `ObservationSnapshot` must contain the source cursor or block/commit/page boundary, request parameters, raw-response digests, received-at times, source times, pagination completeness, retry history, and adapter version. Never claim “no solution” from a single empty page or a partially failed enumeration.

A corpus adapter may be paired with an HSAI-owned submission ledger for locally run agent challenges. In that mode, HSAI is the submission source: it timestamps candidate receipt, freezes the run identity, and can assess completeness over its own append-only ledger. Do not infer live-platform submission completeness from a Git archive.

### 3. Agent harness adapter

An agent harness launches a solver against a permitted, isolated challenge and returns a candidate plus evaluation telemetry. It is not a truth oracle and its score never bypasses the pinned challenge verifier.

```text
AgentHarnessAdapter
  describe_harness(ref) -> AgentHarnessDescriptor
  prepare_run(challenge, solver_view, policy) -> AgentRunPlan
  execute(run_plan) -> AgentRunResult
  extract_candidates(run_result) -> [CandidateEnvelope]
  export_trace(run_result) -> AgentTraceBundle
  health() -> AdapterHealth
```

Pin the harness commit, configuration, model/provider identity, prompt digests, tool allowlist, container digest, budgets, and termination policy. Keep nondeterministic agent generation distinct from deterministic candidate verification. Capture token/tool/runtime cost data for research, but do not make a model-reported success or harness reward the settlement predicate.

Candidate ingestion must be:

- idempotent by source identity plus content digest;
- resilient to pagination, reordering, edits, duplicates, and eventual consistency;
- bounded by size and count limits;
- safe against path traversal, symlink attacks, archive bombs, malicious filenames, and content-type confusion;
- explicit about source authenticity and transport authenticity;
- replayable from recorded fixtures.

### Required adapter implementations

#### Core adapters

1. `local-fixture-corpus` and `local-submission-ledger`: fully runnable, deterministic, and used by end-to-end tests.
2. `generic-git-corpus`: accepts an allowlisted repository URL and ref, resolves it to a commit digest, fetches submodules/LFS only under policy, inventories files, applies a versioned manifest overlay, and materializes safely.
3. `generic-container-validator`: invokes a pinned local verifier against a candidate through the verification-runner contract.
4. `zk-golf-submission-source`: implement the documented public read/validation path if an official stable interface, license, completeness model, and authorization are confirmed. Otherwise implement a complete recorded-fixture adapter and capability report.
5. `ecdsa-fail-submission-source`: same requirement as zk.golf.

#### Structured benchmark and archive adapters

6. `nyu-ctf-bench-corpus`
   - Source: `https://github.com/NYU-LLM-CTF/NYU_CTF_Bench`
   - Support the documented `development` and `test` splits, `<year>/<event>/<category>/<challenge>` hierarchy, `challenge.json`, challenge files, Docker images, and `docker-compose.yaml` where present.
   - Pin an immutable commit or release and record the GPL-2.0 repository license plus per-challenge provenance.
   - Use the development split for adapter development. Treat the test split as contamination-sensitive and never expose flags or organizer-only data to the solver.
   - Normalize the six documented categories without discarding original labels.
   - Add a hermetic smoke subset spanning at least crypto, pwn, web, reverse, forensics, and misc where local runtime support permits.

7. `nyu-ctf-agents-harness`
   - Source: `https://github.com/NYU-LLM-CTF/nyuctf_agents`
   - Integrate the documented D-CIPHER, single-executor ablation, and NYU baseline entry points behind `AgentHarnessAdapter` rather than importing their orchestration into the HSAI domain core.
   - Pin the harness commit/release, configuration, Docker environment, Python/runtime dependencies, prompts, model settings, and tool policy.
   - Default to offline fake-model or recorded-trace conformance tests. Live model use must be opt-in and must never be required for CI or settlement correctness.
   - Import candidates and traces, then use HSAI’s pinned challenge verifier as the only success authority.

8. `ctf-archives-corpus`
   - Source: `https://github.com/sajjadium/ctf-archives`
   - Index the event/year/challenge hierarchy at a pinned commit, retain upstream attribution and writeup references, and use per-challenge overlays because packaging is heterogeneous.
   - Treat the repository’s top-level MIT license as one provenance fact, not automatic proof that every nested third-party artifact can be redistributed or commercially rehosted. Record an item-level rights status.
   - Import only an allowlisted, reproducible subset into executable test fixtures; catalog-only entries remain non-settlement-capable.

9. `pwn-college-ctf-archive-corpus`
   - Source: `https://github.com/pwncollege/ctf-archive`
   - Parse challenge-local README and `REHOST.md` materials, preserve original-author attribution, and prefer local rehosting instructions over interaction with the hosted pwn.college service.
   - Convert only reviewed challenge packages into structured runtime/verifier manifests.
   - Never automate the hosted platform or use its scoreboard as a completeness oracle without an official API and explicit authorization.
   - Because archive-wide rights may vary by original challenge, require item-level license/provenance classification before redistribution or market use.

10. `cryptohack-ctf-archive-corpus`
    - Sources: `https://github.com/cryptohack/ctf_archive` and `https://cryptohack.org/challenges/ctf-archive/`
    - Support the documented `description.yml`, `release_files`, optional `server_files`, pinned Dockerfile, symlinked release files, port-1337 runtime convention, and flag injection without exposing the flag to the solver.
    - Record the repository’s stated MIT terms and upstream author/original-event attribution.
    - Prefer local Docker rehosting from the pinned repository. The hosted archive page is discovery/read-only unless a documented API and authorization exist.

#### Heterogeneous public author repositories

11. `public-author-repo-corpus` implemented as profiles over `generic-git-corpus`, with committed manifest overlays and conformance fixtures for:
    - `https://github.com/josephsurin/my-ctf-challenges`
    - `https://github.com/maple3142/My-CTF-Challenges`
    - `https://github.com/alex-bellon/ctf-challenges`

Do not create three copy-pasted adapters. Each overlay must pin paths, challenge taxonomy, build/runtime commands, verifier/flag handling, excluded solutions/writeups, license status, and reproducibility status. A missing or ambiguous license must yield `CATALOG_ONLY` or require operator-supplied permission; public Git availability alone is not redistribution authorization. Preserve upstream difficulty labels as subjective metadata only.

#### Authenticated practice platform

12. `247ctf-manual-source`
    - Source: `https://247ctf.com/dashboard`
    - Implement metadata entered through a reviewed manifest and sanitized recorded fixtures only.
    - Do not automate login, challenge fetching, flag submission, scoreboard access, or account activity unless 247CTF publishes an official API/automation policy and the operator supplies explicit authorization.
    - This adapter cannot attest to submission completeness or drive financial settlement in manual mode.

### Normalized source catalog

Account for the supplied 20-item research list without implementing duplicates:

- NYU CTF Bench, the CSAW-derived dataset, and “Dockerized challenge suites” map to `nyu-ctf-bench-corpus`.
- NYU LLM CTF agents maps to `nyu-ctf-agents-harness`.
- CTF Archives and its repeated GitHub seed-corpus entry map to `ctf-archives-corpus`.
- pwn.college CTF Archive and the repeated pwn.college ecosystem entry map to `pwn-college-ctf-archive-corpus`.
- CryptoHack’s repository, hosted archive, and repeated archive entry map to `cryptohack-ctf-archive-corpus` with separate repository and hosted-service capabilities.
- josephsurin, maple3142, alex-bellon, and the generic “public author repositories” class map to manifest profiles over `public-author-repo-corpus`.
- 247CTF maps to `247ctf-manual-source` until an authorized API exists.
- “Autonomous LLM Agents & CTFs: A Second Look,” the NYU CTF Bench paper, “CTF Archive: Capture, Curate, Learn Forever,” and secondary benchmark surveys are research references, not executable adapters.

Add these research references with access dates to `docs/references.md`:

- `https://arxiv.org/abs/2605.21497`
- `https://papers.nips.cc/paper_files/paper/2024/hash/69d97a6493fbf016fff0a751f253ad18-Abstract-Datasets_and_Benchmarks_Track.html`
- `https://arxiv.org/abs/2512.01233`
- `https://www.emergentmind.com/topics/nyu-ctf-bench` as a secondary discovery source only; do not use it over primary artifacts for implementation claims.

### Adapter capability and readiness matrix

Generate a machine-readable and documented matrix for every adapter/profile with:

- discovery/catalog support;
- immutable snapshot support;
- local artifact materialization;
- local rehosting;
- verifier availability and determinism tier;
- secret/flag isolation;
- candidate ingestion;
- live submission observation;
- deadline/source-time evidence;
- completeness attestation;
- authenticated access requirements;
- license and redistribution status;
- supported categories/architectures;
- external network requirements;
- CI fixture availability;
- settlement readiness: `SETTLEMENT_READY`, `EVALUATION_ONLY`, `CATALOG_ONLY`, `MANUAL_ONLY`, or `UNSUPPORTED`;
- exact blocking reason and last verification date.

Only adapters with a pinned verifier, deterministic candidate verification, reliable deadline evidence, and a valid submission-completeness model may be `SETTLEMENT_READY`. A replayable challenge corpus is valuable for agent evaluation but is not automatically a live market oracle.

### Adapter conformance requirements

Build shared contract tests for corpus, submission-source, and harness adapters. At minimum verify:

- stable identity from the same pinned snapshot;
- failure on mutable or unresolved refs at freeze time;
- complete pagination/cursor behavior where applicable;
- solver-view exclusion of flags, solutions, writeups, and server secrets;
- safe handling of symlinks, Git submodules, LFS, nested archives, executable bits, and hostile filenames;
- deterministic normalized manifests;
- item-level provenance/license status;
- identical PASS/FAIL semantics after local replay;
- truthful capability and settlement-readiness reporting;
- zero network use in fixture tests;
- no interaction with hosted challenge services during CI.

Do not fabricate an API, scrape login-only pages, attack hosted services, or make CI depend on live network responses. Store sanitized recorded fixtures with provenance and retrieval dates.

## Admission and policy reuse

Pass every candidate and execution request through HSAI’s existing admission/policy boundary if one exists. The outcome subsystem should reference, not bypass, existing admission decisions.

The execution admission record must cover:

- claim and candidate digests;
- validator/environment digest;
- source provenance status;
- artifact size/type checks;
- network and filesystem policy;
- resource budget;
- secrets policy;
- code-execution risk classification;
- allow/deny/manual-review decision;
- policy version and decision digest.

An admission `deny` blocks execution and produces evidence of the denial, but it does not make the market claim false. A manual-review state cannot be auto-settled.

## Verification runner

Treat all candidate content and challenge validators as untrusted.

Implement a runner abstraction with a production-capable isolated backend and a deterministic fake for tests. Prefer the repository’s existing sandbox. If none exists, define an OCI-based runner boundary and implement the safest locally available backend without pretending a container alone is a complete hostile-code sandbox.

Minimum controls:

- image pinned by immutable digest, never a mutable tag;
- non-root user and dropped capabilities;
- read-only root filesystem;
- isolated scratch directory with quota;
- no host socket mounts;
- no Docker socket;
- no ambient credentials;
- network disabled by default;
- explicit egress allowlist only when the frozen verifier policy requires it;
- seccomp/AppArmor/SELinux or equivalent profile where available;
- CPU, memory, process, file-descriptor, output, disk, and wall-clock limits;
- structured argv execution without a shell;
- fixed locale, timezone, `umask`, and environment allowlist;
- deterministic seed derivation where randomness is allowed;
- stable input mount names and ordering;
- stdout/stderr truncation with full-stream digesting;
- cancellation and orphan cleanup;
- runner heartbeat and lost-worker recovery;
- sensitive-log redaction that preserves the original private digest and creates a separate public representation.

Capture an `ExecutionManifestV1` with:

- claim, candidate, admission, verifier, and environment digests;
- runner implementation and build version;
- host compatibility facts that affect determinism;
- normalized argv and environment;
- sandbox policy digest;
- input and output artifact manifests;
- resource limits and measured usage;
- start/end receipt times and injected logical time;
- raw exit status, signal, timeout, OOM, and infrastructure classification;
- stdout/stderr digests and storage references;
- normalized result and normalization-policy version;
- replay command/instructions;
- any declared nondeterminism.

### Result normalization

Implement explicit typed mapping. Do not infer PASS from a log substring unless the adapter contract pins that exact behavior and tests it.

Recommended semantics:

- `PASS`: validator completed successfully and emitted the exact accepted structured result.
- `FAIL`: validator completed correctly and deterministically rejected the candidate.
- `INCONCLUSIVE`: validator completed but its result cannot safely decide the predicate, including permitted semantic divergence.
- `ERROR`: runner, adapter, environment, source, timeout, OOM, malformed output, or internal failure.

If a validator has only an exit-code interface, pin the complete exit-code map and reject unspecified codes.

### Reproducibility tiers

Model and report at least:

- `BYTE_IDENTICAL`: canonical outputs and resolution-relevant artifact digests are identical.
- `SEMANTIC_IDENTICAL`: normalized result and specified semantic outputs are identical even if non-critical logs or timings differ.
- `QUORUM_ONLY`: independent allowed implementations agree on the result, but deterministic replay is not claimed.

The frozen policy states which tier is required. Production settlement for the MVP should require at least semantic identity across configured replays, plus the configured attestor quorum.

## Evidence bundle and attestations

Extend HSAI’s existing evidence/proof packet if present. Avoid a parallel format.

Define an immutable `OutcomeEvidenceBundleV1` containing or referencing:

- frozen claim spec and digest;
- challenge descriptor and source-spec digest;
- observation snapshot(s) and completeness assessment;
- candidate envelope and artifact digest;
- admission decision;
- execution manifest(s);
- normalized result(s);
- verifier and environment materials or reproducible retrieval instructions;
- resolution aggregation record;
- dispute records if any;
- code/build provenance;
- evidence publication policy;
- content-addressed blob manifest;
- canonical bundle digest;
- signatures/attestations;
- standalone verification instructions.

Use a Merkle or flat content-addressed manifest consistent with existing HSAI primitives. Every referenced blob must specify digest algorithm, digest, byte length, media type, confidentiality classification, and storage locator. A locator is not authoritative; the digest is.

Sign a domain-separated canonical `ResolutionAttestationV1`, not an arbitrary serialized API response. It should contain:

- claim ID and claim-spec digest;
- truth value and finality;
- resolution policy version/digest;
- evidence bundle digest;
- qualifying candidate/run digests for `TRUE`, or completeness evidence for `FALSE`;
- dispute-window start/end;
- issued-at time and logical sequence;
- signer/attestor ID, key ID, algorithm, and signature;
- supersession field if this is a non-settling correction record.

Prefer the repository’s established signing system. Otherwise start with Ed25519 for local/test environments behind a signer interface suitable for KMS/HSM-backed production keys. Never place raw private key bytes in application config, logs, fixtures, database rows, or command history.

Support configurable quorum:

- development: explicit `1-of-1`, visibly non-production;
- production policy default: at least `2-of-3` distinct registered attestors/runners;
- no duplicate logical signer counting;
- reject mixed claim, policy, evidence, or verdict digests;
- key activation/revocation windows and rotation metadata;
- deterministic attestation ordering.

Provide a standalone command that verifies a downloaded public bundle without database access. It must validate schemas, canonical digests, references, signatures, quorum, policy linkage, and final resolution. It must report precisely what cannot be independently verified.

## Resolution, disputes, and finality

Implement resolution as a durable state machine, not a single boolean database update.

### Positive resolution

A provisional `TRUE` may be proposed only when:

- the candidate is eligible under the frozen policy;
- source provenance and deadline checks pass;
- required verification replays agree at the required tier;
- the execution result is PASS;
- the evidence bundle is complete;
- the configured attestation quorum is satisfied.

### Negative resolution

A provisional `FALSE` may be proposed only when:

- the submission deadline has passed under the authoritative time policy;
- the ingestion grace period has passed;
- all required source pages/cursors/blocks are finalized and recorded;
- the completeness assessment satisfies the frozen policy;
- no eligible PASS candidate exists;
- pending eligible candidates have reached terminal verification states;
- no source outage or unresolved observation gap remains;
- required attestations cover the negative-resolution evidence.

### Disputes

Implement a bounded dispute process with:

- deterministic opening and closing times;
- authenticated dispute submission;
- reason codes and evidence references;
- duplicate suppression;
- optional counterexample candidate;
- replay/reevaluation jobs;
- conflict detection across attestors;
- operator pause without silent resolution mutation;
- append-only decisions and rationale;
- explicit terminal outcomes: rejected, accepted/superseded-before-finality, or unresolved/escalated.

Do not invent decentralized governance. The MVP dispute authority and policy must be explicit and configurable. A dispute that cannot be mechanically resolved must prevent automatic settlement.

### Finality

A provisional resolution becomes final only after its configured dispute window closes, no blocking dispute remains, signatures remain valid, and the claim has not been paused. Finalization emits an immutable event and an outbox message.

## Settlement-backend interface

Create a backend-neutral port:

```text
SettlementBackend
  capabilities(network) -> CapabilityMatrix
  discover_markets(query) -> [ExternalMarket]
  validate_binding(claim, market, mapping) -> BindingValidation
  preflight(binding, final_resolution) -> SettlementPreflight
  build_intent(binding, final_resolution) -> UnsignedSettlementIntent
  submit(intent, signer_ref, idempotency_key) -> SubmissionReceipt
  get_status(receipt_or_binding) -> ExternalSettlementStatus
  reconcile(binding, final_resolution) -> ReconciliationResult
```

All methods return typed errors. Capability absence is not an internal error.

A `MarketBindingV1` must include:

- backend and network;
- external market/outcome/question ID;
- exact side IDs and parsed side specs;
- mapping from HSAI `TRUE`/`FALSE` to backend settlement values or sides;
- external market metadata digest and raw-payload reference;
- frozen HSAI claim ID and claim-spec digest;
- external description or commitment to the HSAI digest, if the backend supports it;
- settlement authority/broadcaster identity expected by the external market;
- backend capability snapshot and version;
- creation actor, timestamp, and signature;
- binding status and invalidation reason.

Binding validation must reject:

- wrong network;
- stale or changed market metadata;
- ambiguous or duplicate sides;
- outcome mapping outside the backend’s allowed range;
- deadline or description incompatible with the frozen claim;
- missing settlement authority;
- an already-settled market;
- unsupported market class;
- a digest mismatch;
- any heuristic-only association.

Build settlement intents deterministically. The idempotency key should commit to backend, network, market binding digest, final resolution digest, action type, and canonical payload. Persist intent before submission. On retry, query external status before resubmitting.

## Hyperliquid HIP-4 integration

Implement HIP-4 as an adapter, not as domain logic.

### Required implementations

1. `Hip4MockBackend`
   - deterministic in-memory or fixture-backed metadata;
   - supports discovery, binding, preflight, intent generation, submission, confirmation, failure injection, and reconciliation;
   - powers all end-to-end CI tests.

2. `Hip4ReadOnlyBackend`
   - calls the official Info endpoint for the selected network;
   - supports `outcomeMeta` only where the current official API exposes it;
   - parses exact documented response types with raw-payload retention;
   - computes and round-trips documented coin/token/action IDs;
   - rate limits, times out, retries boundedly, and records response freshness;
   - never signs or posts an exchange action.

3. `Hip4TestnetPublisher`, conditionally
   - implement only if primary documentation confirms the exact current settlement action, payload, signing domain, authorized broadcaster role, replay protection, and testnet availability;
   - use official signing conventions or a verified official SDK implementation;
   - require a signer reference supplied at runtime;
   - default to `dry_run = true`;
   - require a second explicit runtime acknowledgement for mutation;
   - reject mainnet unconditionally in this build;
   - record request digest, response/transaction hash, external sequence/nonce, and confirmation evidence;
   - never log the signature preimage if it contains secret material.

If any required write detail is undocumented or unavailable, do not guess. Implement the complete port, unsigned intent schema, mock, manual handoff artifact, and capability status `UNSUPPORTED_UNVERIFIED`. Document the exact missing external dependency.

### HIP-4 metadata handling

- Treat outcome IDs as network-scoped.
- Use checked integer arithmetic for `10 * outcome + side` and `100_000_000 + encoding`.
- Validate side indices against the current official spec and payload.
- Preserve unknown display metadata without allowing it to affect settlement.
- Do not hardcode collateral currency; capture what the official market/API actually specifies or mark it unknown.
- Do not parse an opaque human description as authoritative unless its grammar is formally documented and validated. Preserve both raw and parsed forms.
- Cache metadata only with freshness and digest checks.
- Detect market rotation/recurrence and never roll a binding to a new outcome ID automatically.
- Re-read and compare metadata during preflight and reconciliation.

### Manual settlement handoff

When testnet publishing is unavailable, generate a signed `SettlementHandoffV1` containing:

- backend/network;
- binding and resolution digests;
- expected settlement mapping;
- unsigned canonical backend intent;
- evidence bundle URI/digest;
- required authority/signer role;
- operator preflight checklist;
- expiry/replay-protection fields;
- HSAI handoff signature.

Producing a handoff is not settlement. Keep states and wording distinct.

## Public and operator APIs

Follow the repository’s API conventions and generate an OpenAPI specification. Version all routes and payloads.

Suggested public read endpoints:

- `GET /v1/corpora`
- `GET /v1/corpora/{corpus_id}`
- `GET /v1/corpora/{corpus_id}/snapshots/{digest}`
- `GET /v1/adapters/capabilities`
- `GET /v1/challenges`
- `GET /v1/challenges/{challenge_id}`
- `GET /v1/claims`
- `GET /v1/claims/{claim_id}`
- `GET /v1/claims/{claim_id}/spec`
- `GET /v1/claims/{claim_id}/resolution`
- `GET /v1/claims/{claim_id}/evidence`
- `GET /v1/claims/{claim_id}/attestations`
- `GET /v1/claims/{claim_id}/market-bindings`
- `GET /v1/claims/{claim_id}/settlement`
- `GET /v1/evidence/{digest}` with safe publication rules.

Suggested protected operator endpoints:

- `POST /v1/corpora/validate`
- `POST /v1/corpora/snapshots`
- `POST /v1/corpora/{corpus_id}/challenges/{challenge_ref}/materialize`
- `POST /v1/agent-runs/plan`
- `POST /v1/agent-runs/{run_id}/execute`
- `POST /v1/agent-runs/{run_id}/import-candidates`
- `POST /v1/claims/validate`
- `POST /v1/claims`
- `POST /v1/claims/{claim_id}/freeze`
- `POST /v1/claims/{claim_id}/observe`
- `POST /v1/claims/{claim_id}/candidates/ingest`
- `POST /v1/claims/{claim_id}/verify`
- `POST /v1/claims/{claim_id}/resolve`
- `POST /v1/claims/{claim_id}/disputes`
- `POST /v1/claims/{claim_id}/finalize`
- `POST /v1/market-bindings`
- `POST /v1/settlements/preflight`
- `POST /v1/settlements/intents`
- `POST /v1/settlements/{intent_id}/submit`
- `POST /v1/settlements/{intent_id}/reconcile`
- `POST /v1/claims/{claim_id}/pause`
- `POST /v1/claims/{claim_id}/unpause` with elevated authorization and reason.

API requirements:

- explicit authentication and role checks for mutations;
- public endpoints reveal only public evidence;
- structured error codes and correlation IDs;
- idempotency keys for all mutating requests;
- optimistic concurrency/version preconditions;
- pagination and bounded filters;
- request-size limits;
- rate limiting;
- safe caching headers for immutable artifacts;
- no internal stack traces, filesystem paths, secrets, or raw private evidence;
- health, readiness, and dependency status separated;
- OpenAPI and schema examples validated in CI.

## CLI requirements

Provide repository-native equivalents of:

```text
hsai outcomes adapter capabilities [--adapter <id>]
hsai outcomes corpus validate <corpus-manifest.yaml>
hsai outcomes corpus snapshot <corpus-id> --ref <commit-or-release>
hsai outcomes corpus list-challenges <snapshot-digest>
hsai outcomes corpus materialize <snapshot-digest> <challenge-ref>
hsai outcomes corpus verify-solver-view <challenge-package>
hsai outcomes agent plan <harness-id> <challenge-package> --config <file>
hsai outcomes agent run <agent-run-plan> --offline|--live-authorized
hsai outcomes agent import-candidates <agent-run-id>
hsai outcomes claim validate <claim.yaml>
hsai outcomes claim freeze <claim.yaml>
hsai outcomes claim show <claim_id>
hsai outcomes observe <claim_id> --fixture|--live
hsai outcomes verify <claim_id> [--candidate <id>]
hsai outcomes replay <evidence-bundle>
hsai outcomes resolve <claim_id>
hsai outcomes dispute open <claim_id> --reason ... --evidence ...
hsai outcomes finalize <claim_id>
hsai outcomes evidence verify <bundle>
hsai outcomes hip4 discover --network <testnet|mainnet-readonly>
hsai outcomes hip4 bind <claim_id> <outcome_id> --mapping <file>
hsai outcomes settlement preflight <claim_id>
hsai outcomes settlement build-intent <claim_id>
hsai outcomes settlement publish <intent_id> --dry-run
hsai outcomes settlement reconcile <claim_id>
hsai outcomes demo computational-market
```

Commands must support machine-readable JSON output, typed exit codes, and `--help`. Dangerous commands must state the network and dry-run status clearly and require explicit flags. No command should accept a private key directly on the command line.

## Jobs, reliability, and exactly-once effects

Use durable jobs or the repository’s existing worker mechanism for corpus snapshotting/materialization, agent runs, observation, fetching, verification, replay, aggregation, finalization, settlement, and reconciliation.

Requirements:

- at-least-once job delivery with idempotent handlers;
- leases/heartbeats for long verification jobs;
- bounded retries with classified retryability;
- exponential backoff plus jitter for external reads;
- dead-letter/quarantine state with operator visibility;
- no duplicate verification-run identity for the same policy-defined attempt;
- no duplicate external settlement effect;
- transactional outbox for cross-process events;
- restart recovery tests between intent persistence and submission/receipt persistence;
- clock-skew-safe scheduling;
- dependency circuit breakers and operator pause.

External effects are not “exactly once” merely because a database row is unique. Reconcile using the backend’s authoritative status and deterministic idempotency identity.

## Security and abuse model

Create `docs/threat-model.md` and cover at least:

- malicious challenge author or compromised challenge source;
- forged, edited, backdated, withdrawn, duplicated, or oversized submissions;
- source pagination gaps and equivocation;
- compromised validator artifact or dependency;
- candidate-triggered code execution and sandbox escape;
- path traversal, symlink races, archive bombs, decompression bombs, and parser exploits;
- SSRF and unintended network access;
- nondeterminism, time manipulation, randomness, race conditions, architecture drift, and flaky validators;
- runner/attestor collusion or equivocation;
- signer theft, misuse, rotation, revocation, and replay;
- evidence deletion, substitution, withholding, and privacy leakage;
- database rollback or audit-log tampering;
- stale/incorrect HIP-4 metadata;
- wrong network, side, outcome ID, mapping, or broadcaster;
- duplicate settlement submission and ambiguous external response;
- chain/API reorganization or delayed finality where applicable;
- denial of service and expensive-candidate griefing;
- operator error and emergency pause abuse.

For every material threat, document trust assumption, preventive control, detection, response, residual risk, and whether it blocks MVP production readiness.

Add security tests for malformed schemas, canonicalization collisions, overflow, path traversal, malicious archives, output floods, timeout/OOM, signature tampering, quorum duplication, stale bindings, wrong-network publishing, idempotency races, and unauthorized API calls.

## Privacy and disclosure

Computational challenge submissions may contain undisclosed vulnerabilities or personal identifiers. Implement evidence classification:

- `PUBLIC`
- `EMBARGOED`
- `OPERATOR_ONLY`
- `SECRET_REFERENCE_ONLY`

The public evidence manifest may contain a commitment to a private artifact without exposing it. It must state what is hidden, why, who can access it, the disclosure condition/date, and how the hidden artifact’s digest participates in the attestation. Redaction creates a new derived artifact and never overwrites the original digest.

Never expose environment secrets, bearer tokens, signer material, internal host paths, or unrelated source data.

## Observability and operations

Add structured logs, traces, and metrics using existing repository conventions.

At minimum measure:

- claims by truth/finality/settlement state;
- observation lag from source and deadline;
- candidate counts, duplicates, and rejected artifacts;
- verification queue depth and duration;
- PASS/FAIL/INCONCLUSIVE/ERROR rates by adapter and version;
- replay agreement and nondeterminism incidents;
- evidence generation failures and storage availability;
- attestation quorum latency and conflicts;
- open disputes and time to resolution;
- settlement preflight/submission/confirmation/reconciliation counts and failures;
- external API latency, freshness, rate-limit, and circuit-breaker state;
- outbox backlog and dead-letter jobs;
- operator pauses and unauthorized attempts.

Do not use claim IDs, candidate IDs, wallet addresses, or raw error payloads as unbounded metric labels.

Provide:

- local development configuration;
- deployment configuration examples;
- liveness/readiness checks;
- backup and restore notes;
- database migration and rollback procedure;
- key rotation runbook;
- source outage runbook;
- nondeterminism incident runbook;
- disputed resolution runbook;
- erroneous preflight or submission runbook;
- evidence-store outage and recovery runbook.

## Required tests

Tests must be deterministic, hermetic, and runnable without production credentials or live network access.

### Unit tests

Cover:

- schema validation and unknown-field behavior;
- canonicalization and domain-separated IDs;
- time/deadline boundary semantics;
- eligibility and duplicate policies;
- execution result mapping;
- state-transition legality;
- resolution aggregation;
- quorum and key validity;
- HIP-4 checked encoding/decoding;
- binding validation;
- intent/idempotency generation;
- public/private evidence projections.
- corpus snapshot and manifest-overlay normalization;
- solver-view secret exclusion;
- adapter capability/readiness classification.

### Property and fuzz tests

Cover:

- canonicalization stability under object key order;
- encode/decode round trips and integer boundaries;
- state-machine invariants across random event sequences;
- no illegal finalization or settlement readiness;
- parsers for source and HIP-4 payloads;
- archive/path safety;
- signature and evidence mutation rejection.

### Golden tests

Commit stable fixtures for:

- claim-spec canonical JSON and digest;
- challenge descriptor;
- observation snapshot;
- candidate envelope;
- admission decision;
- execution manifest;
- evidence bundle;
- resolution attestation;
- market binding;
- HIP-4 metadata and asset IDs;
- unsigned settlement intent and manual handoff.

### Integration tests

Use real persistence and blob-store interfaces in local containers or repository-native test services. Cover migrations, transaction boundaries, job retries, outbox delivery, storage integrity, API auth, and restart recovery.

Also run the shared adapter conformance suite against:

- local fixture corpus/submission ledger;
- NYU CTF Bench recorded subset;
- CryptoHack recorded subset;
- CTF Archives recorded subset;
- pwn.college recorded subset;
- all three public-author manifest overlays;
- 247CTF manual fixture without any authenticated or hosted interaction;
- NYU agent harness with a fake model or recorded trace.

### End-to-end acceptance scenarios

Automate all of these:

1. **Valid solution before deadline**
   - freeze claim;
   - bind mock HIP-4 market;
   - ingest eligible candidate;
   - run required replays;
   - produce complete evidence and quorum;
   - pass dispute window under an injected clock;
   - finalize `TRUE`;
   - build and submit mock settlement exactly once;
   - reconcile confirmed state;
   - independently verify exported evidence.

2. **Invalid candidate followed by valid candidate**
   - first candidate yields FAIL but claim stays unresolved;
   - second eligible candidate yields PASS;
   - final result is `TRUE`.

3. **No solution**
   - empty but complete source snapshots through the deadline and grace period;
   - final result becomes `FALSE` only after completeness and quorum requirements.

4. **Incomplete observation**
   - pagination/source outage leaves a gap;
   - claim remains unresolved and settlement preflight fails.

5. **Runner error**
   - timeout/OOM/crash produces ERROR;
   - it never maps to `FALSE` or settlement readiness.

6. **Nondeterministic validator**
   - replays diverge;
   - resolution becomes inconclusive or disputed according to policy.

7. **Dispute with counterexample**
   - provisional negative resolution receives a valid eligible candidate observed before cutoff;
   - finalization is blocked and resolution is recomputed.

8. **Tampered evidence**
   - one byte is modified;
   - standalone verifier rejects the bundle and identifies the failed check.

9. **Stale market binding**
   - external metadata digest changes or a recurring outcome rotates;
   - preflight fails without automatic rebinding.

10. **Settlement crash recovery**
    - process crashes after mock backend accepts submission but before receipt persistence;
    - restart reconciles rather than duplicating the effect.

11. **Wrong network/mainnet mutation**
    - any mainnet publish attempt fails closed even with mutation flags.

12. **Unauthorized operation**
    - public reader attempts freeze/finalize/publish;
    - request is denied and audited without leaking sensitive data.

13. **Backward compatibility**
    - existing HSAI admission, proof/evidence, replay, and Mesh tests still pass.

14. **NYU agent-to-verifier path**
    - pin a development-split NYU CTF Bench challenge and harness fixture;
    - expose only the solver-view manifest to the fake or recorded agent run;
    - import the resulting candidate through the HSAI-owned submission ledger;
    - verify with the hidden pinned checker;
    - preserve the agent trace as evaluation evidence without using the harness reward as truth.

15. **Corpus update isolation**
    - move an upstream fixture branch/tag or add a new archive challenge;
    - confirm an already frozen claim remains bound to its original commit and artifacts;
    - require a new claim identity to adopt the new corpus snapshot.

16. **Rights and capability gating**
    - mark a public author repository fixture with missing/ambiguous license metadata;
    - confirm cataloging can succeed while redistribution, live execution, and settlement readiness remain blocked.

### Optional live smoke tests

Provide opt-in, read-only tests for official challenge and HIP-4 metadata endpoints. Mark them ignored or separately tagged by default. They must have strict timeouts, no secrets, no writes, no CI dependency, and sanitized recorded-fixture refresh tooling.

## Documentation and developer experience

Create or update:

- top-level README section with product scope and a five-minute local demo;
- `docs/outcomes/architecture.md`;
- `docs/outcomes/claim-spec.md`;
- `docs/outcomes/lifecycle.md`;
- `docs/outcomes/evidence-and-attestations.md`;
- `docs/outcomes/challenge-adapters.md`;
- `docs/outcomes/corpus-adapters.md`;
- `docs/outcomes/agent-harnesses.md`;
- `docs/outcomes/adapter-capability-matrix.md` plus a machine-readable equivalent;
- `docs/outcomes/corpus-rights-and-provenance.md`;
- `docs/outcomes/hyperliquid-hip4.md`;
- `docs/outcomes/security.md` or the repository’s threat-model location;
- `docs/outcomes/deployment.md`;
- `docs/outcomes/operations.md`;
- `docs/outcomes/limitations.md`;
- `docs/references.md` with dated primary sources;
- OpenAPI spec and example requests;
- configuration reference and `.env.example` containing placeholders only;
- ADRs for canonicalization, persistence/eventing, runner isolation, attestation/quorum, dispute/finality, and HIP-4 capability gating.

Include one compact architecture diagram and one state-machine diagram. Keep source-of-truth semantics in prose and tables as well so docs remain accessible.

Document the exact trust model:

- challenge source;
- verifier authorship and pinned artifact;
- runner isolation;
- observation completeness;
- attestor quorum;
- evidence availability;
- HSAI operator/dispute authority;
- Hyperliquid broadcaster/settlement authority;
- external network finality.

Do not market the system as “trustless,” “fully decentralized,” “proof-based,” or “automatic” unless the implemented path actually justifies that precise claim.

## Implementation sequence

Work in vertical, testable increments. Adapt paths to the repository.

### Phase 0 — Reconnaissance and claim-boundary audit

- Map current HSAI modules to the target architecture.
- Run the existing test suite and record the baseline.
- Verify official external interfaces.
- Create the HIP-4 capability matrix.
- Add ADR skeletons and an implementation checklist.

Exit criteria: no unresolved architectural duplication; external unknowns are explicit; baseline is known.

### Phase 1 — Domain contracts and canonicalization

- Implement versioned types and JSON Schemas.
- Implement canonicalization, digest IDs, validation, and transition rules.
- Add golden/property tests.
- Extend existing HSAI evidence/proof contracts compatibly.

Exit criteria: cross-language-stable fixtures and all invariants tested.

### Phase 2 — Persistence and durable events

- Add migrations, repositories, append-only audit events, idempotency, and outbox.
- Test concurrency and migration paths.

Exit criteria: frozen data cannot be silently mutated; job/event recovery works.

### Phase 3 — Source intake, admission, and runner

- Implement source port, local fixture, generic validator, observation completeness, and safe artifact materialization.
- Connect existing admission/policy.
- Implement runner, result normalization, replay, and failure classification.

Exit criteria: local fixture candidates execute hermetically and runner errors never become truth values.

### Phase 4 — Evidence, attestations, resolution, disputes

- Produce portable evidence bundles.
- Implement signing/quorum and standalone verification.
- Implement positive/negative resolution, dispute window, finalization, and pause.

Exit criteria: acceptance scenarios 1–8 pass without a settlement backend.

### Phase 5 — API, CLI, and workers

- Add versioned public/operator APIs, OpenAPI, RBAC, idempotent mutation endpoints, CLI, jobs, and observability.

Exit criteria: complete local workflow is operable through CLI and API after restart.

### Phase 6 — Challenge adapters

- Implement the three adapter ports and shared conformance suites.
- Implement recorded-fixture zk.golf and ecdsa.fail submission-source adapters and official live read-only modes only where verified.
- Implement the NYU CTF Bench corpus adapter and NYU CTF agent harness first.
- Implement CryptoHack, pwn.college, and CTF Archives corpus adapters over pinned local snapshots.
- Implement the generic public-author repository adapter with josephsurin, maple3142, and alex-bellon manifest profiles.
- Implement 247CTF as manual metadata plus sanitized fixtures only unless an official authorized API is verified.
- Generate the capability/readiness matrix, rights/provenance inventory, and fixture refresh tooling.

Exit criteria: every adapter passes its applicable conformance suite; the NYU agent-to-verifier acceptance path works; flags/solutions remain outside solver view; all live, legal, completeness, and settlement limitations are explicit.

### Phase 7 — HIP-4 read-only integration and binding

- Implement official metadata reads, exact types, asset encoding, explicit bindings, freshness, and stale-binding checks.
- Add mock backend and fixtures.

Exit criteria: discovery/binding works read-only; acceptance scenario 9 passes.

### Phase 8 — Safe settlement boundary

- Implement deterministic settlement intents, preflight, manual handoff, idempotency, mock submission, and reconciliation.
- Implement testnet publisher only if official capabilities are verified.
- Hard-block mainnet mutations.

Exit criteria: acceptance scenarios 1, 10, and 11 pass through mock; no unsupported write path is implied.

### Phase 9 — Hardening and handoff

- Run formatting, linting, unit, property, integration, E2E, security, documentation, and existing regression tests.
- Run the local demo from a clean checkout/configuration.
- Review threat model and operational failure modes.
- Remove accidental placeholders, dead code, and misleading claims.

Exit criteria: definition of done below is satisfied or each external blocker is isolated, tested, and precisely documented.

## Definition of done

The build is complete only when:

1. A frozen claim has a stable, independently recomputable content identity.
2. Resolution-critical semantics cannot mutate after freezing.
3. Candidate observation is cursor-based, replayable, and completeness-aware.
4. Untrusted verification is isolated, resource-bounded, and classified correctly.
5. Required replays and attestor quorum are policy-enforced.
6. Errors and inconclusive runs cannot settle the claim as false.
7. Positive and negative claims follow correct existential/deadline semantics.
8. Evidence can be exported and verified independently.
9. Disputes and finality are explicit and settlement-blocking where required.
10. Market bindings are explicit and digest-bound, never heuristic-only.
11. HIP-4 metadata and asset IDs are handled from current official contracts.
12. Unsupported HIP-4 write capabilities fail closed.
13. Mock settlement is idempotent and reconciles after crash recovery.
14. Mainnet mutation is structurally disabled.
15. Public/operator APIs and CLI exercise the complete workflow.
16. Migrations, observability, security controls, and runbooks exist.
17. All new tests and all relevant existing HSAI tests pass.
18. A one-command local demo proves the full true-resolution path and a second scenario proves the unresolved/failure boundary.
19. Documentation states exact trust assumptions and limitations.
20. The final handoff distinguishes implemented, tested, externally blocked, and future work without exaggeration.
21. NYU CTF Bench, CryptoHack, pwn.college, CTF Archives, and the three author-repository profiles have pinned, normalized, fixture-tested corpus paths.
22. NYU D-CIPHER/baseline integration is isolated behind an agent-harness port, and its output must pass the HSAI verifier.
23. 247CTF performs no automated authenticated interaction without an official API and explicit authorization.
24. Every adapter publishes a truthful, machine-readable capability, rights/provenance, and settlement-readiness record.

## Quality bar

- Prefer small, typed, composable modules over a monolithic service.
- Prefer state machines and schema validation over scattered booleans.
- Prefer structured payloads over log parsing.
- Prefer content-addressed immutable artifacts over mutable URLs.
- Prefer deterministic fixtures over live-test flakiness.
- Prefer a clearly disabled capability over a guessed integration.
- Do not add dependencies when the standard library or an existing vetted dependency suffices.
- Pin new dependencies and run the repository’s audit/licensing checks where available.
- Keep APIs compatible or provide explicit migrations/deprecations.
- Avoid broad refactors unrelated to this feature.
- No TODO may stand in for a safety-critical invariant. An externally blocked adapter may be explicitly disabled with tests and a documented capability gap.

## Required final verification

Before reporting completion:

1. Inspect the full diff for secrets, generated junk, accidental binaries, and unrelated changes.
2. Run repository formatters and linters.
3. Run focused tests after each phase and the complete relevant suite at the end.
4. Run migration tests.
5. Run the end-to-end demo from clean local state.
6. Export an evidence bundle and verify it with the standalone verifier.
7. Exercise mock settlement crash recovery and mainnet refusal.
8. Confirm existing admission/proof/replay/Mesh behavior remains intact.
9. Check docs and OpenAPI examples against real code.
10. Record any test not run and the exact reason.

## Final response format

When finished, provide:

1. **Outcome:** what now works end to end.
2. **Architecture:** the important components and how existing HSAI primitives were reused.
3. **Files changed:** grouped by subsystem, with concise purposes.
4. **External capability status:** separate HIP-4 read-only, mock, manual handoff, testnet publish, and mainnet publish status.
5. **Verification:** exact commands run and results.
6. **Demo:** exact commands to reproduce the true, false, and unresolved/error paths.
7. **Security and trust boundaries:** material residual risks.
8. **Blockers:** only genuine external dependencies or missing authority, with the implemented safe boundary.
9. **Next production steps:** ordered and concrete.

Do not claim a live HIP-4 market was created or settled unless that actually occurred with explicit user authorization—which this prompt does not grant. Do not describe a dry run, unsigned intent, manual handoff, mock receipt, or testnet result as a mainnet settlement.

Begin now by inspecting the repository and reporting the phase plan, then implement the system through the definition of done.
