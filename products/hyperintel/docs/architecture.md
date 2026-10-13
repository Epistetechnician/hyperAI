# Architecture

HyperIntel separates five planes:

1. **Execution**: a current hermetic selected-provider harness, followed later
   by provider discovery, encrypted transport, live streaming, cancellation,
   metering, and failover.
2. **Market**: signed offers, hard eligibility, equivalent-product comparison,
   and expected-successful-cost routing.
3. **Assurance**: canonical manifests, signatures, claim envelopes, evidence,
   revocation, and customer-runnable verification.
4. **Settlement**: deterministic preauthorization, payout, fee, credit, refund,
   and later HyperEVM effects.
5. **Financial risk**: a future executed-price index and separately gated HIP-3
   hedge, never inference delivery.

The current implementation covers pure-data portions of planes 1–4 plus one
in-process execution harness backed by the local replay store and a separate
metadata-only execution journal. It verifies and consumes an admission before
invoking an injected scripted provider, and it can classify local crash states
without retaining request or response bodies. A separate pure-data reconciler
then checks the complete local identity graph without retaining content. None
of these components runs or authorizes a networked or production execution
runtime.

A separate runtime-configuration crate validates one bounded explicit
local-hermetic JSON document. It canonicalizes the configuration identity,
requires distinct absolute lexical SQLite file paths and bounded pool sizes,
and represents secrets only as opaque immutable-version references. It does
not open either database, resolve a secret, read ambient configuration, or
grant deployment authority.

## First vertical slice

```text
ServiceSpecV1 + ProviderIdentityV1 + CapacityOfferV1
  + SignedCapacityOfferV1
  + SignedReliabilityArtifactV1
  + InferenceRequestPolicyV1 + AssuranceAcceptancePolicyV1
  -> provider-key offer authentication
  -> verifier-key reliability identity, binding, validity, and arithmetic checks
  -> fail-closed eligibility
  -> expected-successful-cost selection
  -> RouteEvaluationRecordV1 with every typed candidate report
  -> selected RouteDecisionV1 or deterministic NoEligibleOffer
  -> ReplayStateTransitionV1 over caller-supplied buyer state
  -> optional MeteringReceiptV1 + SettlementIntentV1 for selected routes only
  -> portable ReplayProtectedMarketEvidenceBundleV4
  -> offline verification
```

`SignedCapacityOfferV1` binds the canonical offer digest to the provider's
configured offer key, a caller-classified `ProviderKey`, the verified provider
authorization envelope, and contained validity windows. Invalid offers remain
ineligible regardless of price. `SignedMarketEvidenceBundleV2` reconstructs
this check. `MarketEvidenceBundleV1` remains a compatibility schema and keeps
its unsigned-offer nonclaim.

Two distinct offers that both authenticate under one provider key and logical
sequence are treated as verified equivocation and block the signed batch.
Unverified spoofed sequence fields remain ordinary ineligible offers and cannot
poison fallback to valid supply. Rejected attacker-claimed keys are retained in
the signed wrapper for reconstruction but are never promoted into caller trust.

`SignedReliabilityArtifactV1` binds exact observed-success and total counts,
their floor-PPM result, provider, service, source and observation-policy
commitments, observation window, and validity to a caller-classified
`VerifierKey`. Missing, stale, untrusted, tampered, or inconsistent artifacts
are candidate-scoped hard failures. `ReliabilityArtifactVerifiedMarketEvidenceBundleV3`
reconstructs this path while artifacts and trust remain separate inputs. V1 and
V2 remain compatible under their narrower explicit nonclaims.

`RouteEvaluationRecordV1` retains one canonically ordered
`EligibilityReportV1` per candidate, including typed failures and available
cost decomposition. Its outcome is either a selected `RouteDecisionV1` or
`NoEligibleOffer`. V4 therefore reconstructs both fills and complete denials;
metering and settlement records must both be present for a selection and both
absent for a denial. V4 uses an additive causal-rejection profile that
distinguishes invalid assurance evidence, valid evidence rejected by policy,
and invalid offer authentication. The V1, V2, and V3 selectors retain their
historical collapsed rejection profile so their eligibility-report, route, and
bundle commitments do not change.

`BuyerReplayStateV1` and `ReplayStateTransitionV1` bind the buyer, exact next
nonce, policy, evaluation digest, evaluation time, and before/after state. The
transition is a deterministic pure-data result for selected and denied
evaluations. It does not authenticate the buyer or establish currentness,
durable storage, atomic consumption, concurrency control, rollback resistance,
or external enforcement.

## Authenticated execution receipt tracer

```text
SignedInferenceRequestPolicyV1
  + V4-grade signed offer, assurance, reliability, and replay inputs
  -> caller-trusted BuyerPolicy signature verification
  -> reconstructed selected route and replay transition
  -> AuthenticatedExecutionAdmissionV1 (authority_granted = false)
  -> ProviderExecutionReceiptV1
  -> configured provider receipt-key signature verification
  -> MeteringReceiptV1 reconstructed from signed usage and output commitments
```

The tracer is implemented at `dbfbd48`. Admission is available only for a
reconstructed selected route. The buyer-policy statement binds the exact
policy digest, replay snapshot, buyer, nonce, and validity window. The provider
statement binds the exact completed receipt, admission, provider, configured
receipt key, completion time, deadline, and nonzero sequence. Receipt usage is
bounded by the policy, service, and selected offer; its runtime commitment must
equal the selected service runtime.

These checks create local authenticated evidence, not execution authority.
There is no request release, socket, prompt transport, durable nonce consume,
cancellation, retry, partial-output accounting, persistence, provider call,
settlement submission, custody, or value movement.

## Durable replay store

`hyperintel-persistence` implements the next local R1 dependency at `f5d3eff`.
It validates the full authenticated admission before opening an immediate
SQLite transaction. The transaction first resolves an exact immutable retry;
otherwise it updates the buyer cursor only when the stored buyer, state digest,
nonce, and canonical payload all equal the transition's before-state. It then
inserts the immutable admission-to-transition consumption record in the same
transaction.

SQLite WAL, a bounded busy timeout, `BEGIN IMMEDIATE`, schema checksums, and
validated duplicated columns make concurrent local writers fail closed. The
contract test sends two different authenticated admissions against one
before-state: exactly one applies and the other is stale. It also covers exact
retry, invalid buyer signature, conflicting initialization, and close/reopen.

This is local durability and atomicity, not rollback resistance, replication,
backup policy, high availability, tenant isolation, database encryption,
operator authentication, prompt release, or production operation. Replacing
the database file with an older copy can restore older state unless a future
independent monotonic anchor detects it.

## Hermetic execution harness

```text
text-only ChatCompletionRequestV1
  + AuthenticatedExecutionAdmissionV1
  + SqliteReplayStoreV1
  + selected LocalProviderAgentV1
  + configured ProviderReceiptSignerV1
  -> exact request/prompt-commitment check
  -> provider and receipt-key preflight
  -> newly applied durable replay consumption
  -> deterministic local stream, deadline, or cancellation result
  -> completed provider receipt + configured signature verification
  -> MeteringReceiptV1
  -> SettlementIntentV1 (authority_granted = false)
```

`hyperintel-execution` implements this local R1 harness at `4c4b17f`. Its
request and chunks intentionally cover only text messages, streaming, one
choice, assistant deltas, and the normal `stop` finish reason from the official
[Chat Completions API shape](https://developers.openai.com/api/reference/resources/chat).
It is not an HTTP endpoint or a claim of full API compatibility.

The request digest must exactly equal the buyer policy's prompt commitment.
The provider and signer must match the route-selected provider and configured
receipt key before durable state changes. Only `Applied` replay consumption
invokes the provider; `ExistingIdentical` is rejected, so an exact retry cannot
execute twice. Provider substitution and cancellation at or before consumption
leave the nonce untouched.

The injected provider emits explicit timestamps, text frames, token deltas,
and a runtime commitment. Completed plans are bounded by the policy, service,
selected offer, and admission deadline before the receipt chain is accepted.
Cancellation, deadline, provider failure, and signer failure return typed
non-completed evidence without metering or settlement artifacts. The
cancellation signal and stream are deterministic in-memory records, not live
transport controls.

The contract suite exercises independently selected alpha and beta providers,
exact-retry suppression, provider-substitution rejection, cancellation,
deadline, provider failure, and signer failure. It uses no socket, HTTP server,
provider credential, wallet, chain, custody, funds, or value movement.

`hyperintel-authenticated-reroute-v1` at `379ee98` implements the narrow safe
lineage to a different provider. It accepts only a self-consistent prior
`PROVIDER_FAILED` record with the closed `UNAVAILABLE` error and zero released
chunks. The caller must supply a separately routed and buyer-signed admission
that preserves buyer, service, and prompt commitment, starts from the prior
durable after-state, consumes the next sequential nonce, selects a different
provider, and does not regress evaluation or admission time. The existing
harness then performs the second durable consume and provider invocation.

Cancellation, deadline, partial output, signer failure, same-provider reuse,
prompt change, and stale lineage do not qualify. The code neither selects the
fallback route nor signs the next policy. Automatic/live failover, provider
health, and external transport remain unimplemented.

## Durable execution journal

`hyperintel-execution-journal` at `6a2d9d5` adds a separate operator-selected
SQLite database for canonical start and terminal identities. Start and
terminal compare-and-insert operations use `BEGIN IMMEDIATE`; concurrent exact
retries return an idempotent result and same-admission semantic conflicts fail
closed. Terminal writes and public restart inspection query the actual durable
replay store and bind admission, buyer, replay-consumption digest, start digest,
and non-regressing time.

The integrated wrapper writes a new start, rejects any existing start rather
than implicitly resuming it, checks durable consumption before provider
invocation, runs the existing hermetic harness, and retains a closed terminal
identity. A pre-existing or concurrently detected consumption without a
terminal is non-retriable. Recovery distinguishes not-started,
consumed-without-start, started-not-consumed, consumed-without-terminal, and
terminal states. Cross-row, cross-store, digest, and timestamp tampering fails
closed.

The journal stores metadata only: no prompt or response bodies, credentials,
keys, arbitrary error text, or settlement authority. The journal and replay
store remain two local databases with no atomic transaction across them. A
consumed-without-terminal result requires a new authenticated nonce or explicit
operator reconciliation; the code supplies no automatic resume, rollback
resistance, remote replication, network transport, funds, or production
authority.

## Local receipt reconciliation

`hyperintel-reconciliation` at `f2b12a2` reconstructs one positive,
identity-only consistency report from the authenticated admission, its exact
durable replay consumption, journal start and terminal, and the completed or
incomplete local execution outcome. Completed reports reverify the selected
route, provider-signed execution receipt, stream commitment, metering, and
non-authoritative settlement intent. Incomplete reports reverify the closed
status, stream shape, and time bounds and contain no metering or settlement
identity.

The five direct contracts cover stable alpha and beta reports, every incomplete
status, admission/replay/journal/chunk/receipt/metering/settlement/authority
tamper, completed/incomplete terminal swaps, self-consistent impossible times,
report reconstruction, and serialized no-content behavior. An optional buyer
observation remains an opaque identity-bound commitment; reconciliation makes
no claim that it is authentic or independent. The crate opens no database or
socket, mutates no state, and grants no execution, payment, settlement, or
production authority.

## Local runtime configuration

`hyperintel-runtime-config` at `ec46411` validates at most 65,536 bytes from an
explicit caller-selected file. The V1 schema accepts only `LOCAL_HERMETIC`,
rejects unknown fields at every nesting level, requires canonical identifiers,
distinct absolute lexical SQLite file paths, and connection bounds of 1–16.
Its domain-separated digest is stable across JSON field order.

`SecretReferenceV1` contains a provider identity, a logical handle, and a
nonzero digest of an immutable provider-issued version identifier or metadata
record. That digest must never be derived from secret bytes. The crate has no
resolver and does not inspect environment variables, create paths, open a
database or socket, contact a provider, or grant credential authority.

## Local tenant authorization separation

`hyperintel-tenant-iam` at `f8a43dd`, hardened through `70a344b`, verifies one exact
buyer-tenant action under caller-supplied policy and expected context. Signed
grants bind tenant, principal, issuer, audience, runtime and IAM snapshot
identities, resource ownership, operation context, and a maximum 300-second
validity window.

IAM keys are tenant-namespaced and purpose-separated between access grants and
revocation snapshots. Revocation evidence is independently signed, bounded in
time, monotonic, and checked against a current caller-pinned sequence and
digest plus the exact revocation trust record. Grant, key, and self-revocation
boundaries are inclusive. The success type is opaque and explicitly non-
authoritative. This is local authorization separation, not process, storage,
cache, network, encryption-key, or log isolation and not production IAM.

## Local operator-secret injection

`hyperintel-secret-provider` at `22ddd2c` atomically verifies one short-lived
signed operator authorization and invokes one caller-injected resolver. The
authorization and result bind the exact runtime, consumer, selected provider,
immutable `SecretReferenceV1`, provider-API purpose, operation context,
evaluation time, trust record, revocation state, and rollback checkpoint.
Preflight failure invokes no resolver; a successful preflight invokes exactly
once with no retry or fallback.

Returned material is non-cloneable, nonserializable, redacted, bounded, and
zeroize-on-drop. The crate contains no environment, file, keychain, KMS, cloud,
or provider adapter; it performs no network request and does not establish the
authenticity or cleanup behavior of an injected resolver. Possession of a
successful result may itself confer external capability.

## Hermetic provider-agent control

`hyperintel-provider-agent` at `1944478` verifies separate provider-signed
service-manifest and short-lived readiness assertions under purpose-separated
keys and signed revocation state. The complete control lineage binds the exact
provider, agent instance, selected service and offer, runtime configuration,
model/tokenizer/runtime artifacts, capability set, receipt key, text-stream
interface, and immutable secret-reference digest. Caller checkpoints pin the
manifest, readiness, and revocation sequences and same-sequence digests to the
full binding.

Control evaluation time must equal replay-consumption time. Invalid, stale,
revoked, forked, or context-mixed control fails before the existing harness is
entered. Valid control permits one harness call with no retry or fallback; the
existing harness remains the sole replay consumer. These records authenticate
provider/operator assertions only. They do not observe health, prove
availability or model identity, resolve a secret, open a socket, or contact an
external provider.

V1, V2, and V3 schemas and reconstruction behavior remain available unchanged,
with selected-path and rejection/fallback golden commitments enforcing that
compatibility boundary.

Assurance requirements are hard gates. They are never converted into a scalar
that a cheaper offer can offset.

## Initial trusted computing base

- Pinned Rust toolchain and dependencies.
- Canonical JSON implementation.
- SHA-256 implementation.
- Ed25519 implementation.
- HyperIntel validation and routing code.
- Caller-controlled provider and verifier key registry, supplied separately
  from the evidence bundle. Bundle-disclosed keys must match it exactly.
- Caller-controlled buyer-policy and provider-receipt keys for the execution
  tracer. The provider receipt key must match the selected provider identity.
- Key-validity and revocation-aware Ed25519 verification for domain-specific
  canonical messages. The assurance crate checks cryptography and caller trust;
  the owning protocol remains responsible for its message schema and bindings.
- Test fixture declarations.
- Z3 4.15.0 for the separately scoped local abstract routing-property gate. Z3
  is resolved through `tools/pinned-z3`; its reported version is pinned, but
  its installed binary provenance is an environment assumption. It is not a
  runtime dependency or an authority source.
- SQLx 0.8.6, SQLite, the local filesystem, and operator path selection for the
  separately scoped durable replay store and metadata-only execution journal.
- Caller-injected local provider and receipt-signer implementations for the
  hermetic execution harness. Test signers are fixtures, not production key
  custody or signing authority.

No blockchain, external provider, attestation service, or network is in the
current trusted computing base because none is called.

## Dependency direction

```text
hyperintel-core <- hyperintel-assurance <- hyperintel-market <- hyperintel-cli
                                                    ^
                                                    |
                                      hyperintel-persistence
                                                    ^
                                                    |
                                        hyperintel-execution
                                                    ^
                                                    |
                                hyperintel-execution-journal
                                                    ^
                                                    |
                                  hyperintel-reconciliation
```

`AssuranceEvidenceSetV1` is generic and contains no market-derived records.
Market evidence bundles belong to `hyperintel-market`, where reliability,
route, metering, and settlement objects can be reconstructed without a
dependency cycle.

## Formal property boundary

The local formal gate models two candidates and proves that the abstract
selector cannot select an ineligible candidate. A deliberately weakened
selector is satisfiable under a fixed counterexample. The test pins the exact
Z3 version and the SHA-256 digest of `routing.rs`; source drift therefore fails
the gate before the SMT result is accepted. The SMT model remains handwritten
and does not establish semantic correspondence to Rust, serialization,
concurrency, networking, or any external runtime. Exact inputs and claims are
recorded in [`formal-verification.md`](formal-verification.md).
