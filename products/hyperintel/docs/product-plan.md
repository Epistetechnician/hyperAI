# End-to-end product plan

## Executive decision

Build a verified inference-capacity marketplace, not a credential resale
exchange and not a prediction market.

> HyperIntel selects the lowest expected successful cost among offers that can
> satisfy the buyer's explicit service, privacy, assurance, and SLA policy.

Inference stays offchain. Evidence and settlement intent are deterministic and
portable. HyperEVM is the escrow, bond, registry, and settlement substrate.
HyperCore is reserved for a later, independently gated hedge over a mature
executed-price index. No outcome-market component belongs in the product.

## Initial customer

The first buyer is an agent platform, financial application, or regulated
software operator that needs all of the following:

- an OpenAI-compatible request path;
- price competition across authorized capacity;
- hard regional, privacy, model, tool, and assurance constraints;
- bounded maximum spend and machine-readable SLA remedies;
- evidence from which an independent checker can recompute routing and
  settlement without trusting the router's calculation. External provider,
  region, execution, model, and metering claims remain limited by their
  separately eligible evidence.

The first provider is an authorized self-hosted operator or official reseller
with a stable model build, declared service window, settlement identity, and a
bond. Buyer-owned provider keys may be supported as a separate direct mode.
Opaque credential resale is not accepted supply.

## Product surfaces

| Surface | Buyer result | Provider result | HyperIntel revenue |
|---|---|---|---|
| Immediate inference | Policy-eligible streamed response and receipts | Paid completed work | Execution fee |
| Reserved capacity | Contracted throughput window with measurable service levels and capped remedies | Forward demand and utilization | Reservation fee |
| Enterprise assurance | Private policies, on-prem checker, audit export | Higher-value compliant demand | Subscription |
| Verification services | Independent replay and evidence retention | Stronger trust tier | Verification fee |
| Future index licensing | Auditable executed-price benchmark | Market visibility | Data/API license |

## Non-negotiable product rules

1. Assurance requirements are hard eligibility gates, never compensable scores.
2. Routing minimizes expected successful cost only after economic equivalence.
3. Prompts, outputs, and provider credentials never enter public chain state.
4. Every charge binds to a route, offer, policy, service, usage, and payer cap.
5. Every settlement conserves value and has a full-payload idempotency key.
6. Missing, expired, revoked, unsupported, or ambiguous evidence fails closed.
7. Attestation can strengthen a declared boundary; it does not prove model
   semantics or output correctness.
8. A blockchain commitment proves inclusion and ordering, not external service
   delivery.
9. Inference delivery cannot depend on a derivatives market.
10. Mainnet, custody, or production claims require their own evidence gates.

## Target end-to-end service sequence

```text
buyer policy + maximum authorization
  -> equivalent-service discovery
  -> fail-closed evidence and availability checks
  -> expected-successful-cost route
  -> encrypted direct execution and streaming
  -> signed route + usage + provider + buyer receipts
  -> objective SLA evaluation
  -> deterministic settlement intent
  -> buyer refund + provider payout + platform fee + SLA credit
  -> portable evidence bundle and optional public commitment
```

The R0 slice ends at a non-authoritative settlement intent. Its V2 path
verifies capacity-offer signatures against caller-supplied trust. Its additive
V3 path also verifies verifier-signed typed reliability identity, bindings,
validity, and exact historical count arithmetic. The additive V4 path retains
complete causal rejection records and a pure-data replay transition. V1, V2,
and V3 retain their historical collapsed rejection semantics and narrower
explicit nonclaims. R0 does not verify the underlying observation truth, live
consumed-nonce state, bonds, SLA credits, chain commitments, or value movement.

The first implemented R1 tracer authenticates the exact buyer policy against a
caller-owned key registry and binds a provider-signed completed-execution
receipt to a reconstructed selected route from V4-grade signed-offer,
assurance, reliability, and replay inputs. Existing metering is then
reconstructed from the signed usage and output commitments. The tracer grants
no request-release authority and adds no transport, persistence, cancellation,
retry, custody, or value movement. It is implemented at `dbfbd48` and does not
by itself satisfy the R1 private-alpha gate.

The local durable replay store at `f5d3eff` verifies that authenticated
admission again, serializes SQLite writers, atomically advances only the exact
committed before-state, and retains an immutable consumption record. Exact
retry is idempotent and competing admissions fail stale. This closes the local
concurrent nonce-consumption gap, but it does not authorize request release or
establish rollback resistance, remote replication, tenant isolation, backup
and recovery operations, or production database readiness.

The hermetic execution harness at `4c4b17f` closes the next local dependency.
It binds a text-only streaming request to the policy prompt digest, verifies
the selected provider and configured receipt signer, consumes the admission
once, and invokes an injected in-process provider. The local contracts execute
both alpha and beta providers and cover exact-retry suppression, provider
substitution, cancellation, deadline, provider failure, signer failure, signed
completion, metering, and settlement-intent derivation. Incomplete attempts
cannot create metering or settlement artifacts.

This is a deterministic no-network exercise, not the private alpha. It has no
HTTP gateway, encrypted transport, external provider adapter, live stream or
cancellation propagation, provider health, safe reroute, tenant isolation,
credential operations, production persistence, funds, or settlement adapter.
The admission binds one provider; failover must route and authenticate a new
next-nonce admission rather than substitute providers under the old admission.

The authenticated reroute slice at `379ee98` now verifies and exercises that
exact local handoff for the safest case: provider `UNAVAILABLE` with zero
released chunks. A separately built and buyer-signed nonce-2 admission keeps
the buyer, service, and prompt commitment, selects the other provider, starts
from nonce 1's durable after-state, and completes through the existing harness.
Cancellation, partial output, same-provider reuse, changed prompt, and stale
lineage fail before invocation. The slice does not discover the next provider,
sign the policy, monitor health, or operate live transport.

The metadata-only execution journal at `6a2d9d5` closes the next local crash
inspection dependency. It serializes exact start and terminal writes, binds
terminal and recovery decisions to the actual replay database, rejects
implicit resume from an existing start, and exposes consumed-without-terminal
as a non-retriable state. It retains only canonical identities and timestamps,
never prompt or response bodies. Because replay and journal writes remain
separate SQLite transactions, this is fail-closed recovery classification, not
cross-database atomicity, automatic retry, production backup, or operator
reconciliation automation.

The strict local runtime configuration at `ec46411` closes the configuration
shape dependency for later deployable candidates. It validates a bounded
explicit document, canonical identity, distinct absolute lexical SQLite paths,
bounded pool sizes, and opaque immutable-version secret references. It has no
ambient configuration, default credential, resolver, database open, socket,
provider call, or deployment authority.

The pure-data tenant IAM verifier at `f8a43dd`, hardened through `70a344b`, closes
the local authorization-decision shape. Dedicated tenant-namespaced keys,
one-action 300-second grants, caller-owned resource and operation context, and
signed rollback-pinned revocation state fail closed under exact current trust.
This is authorization separation only; token issuance, real identity, replay
consumption, gateway enforcement, runtime tenant isolation, and production IAM
remain open.

The local secret-injection seam at `22ddd2c` closes only the atomic
authorization-to-injected-resolution shape. Dedicated signed operator trust,
fresh rollback-pinned revocation, and exact runtime, consumer, selected
provider, immutable reference, purpose, and operation bindings precede one
resolver invocation. Material is bounded, redacted, nonserializable, and
zeroize-on-drop. No ambient credential source, real adapter, network request,
cache, retry, deployment, or production credential authority is present.

The hermetic provider-agent control layer at `1944478` closes the local signed
manifest/readiness-to-harness preflight shape. Purpose-separated trust,
revocation, exact deployment/route bindings, equal evaluation and replay-
consumption time, and manifest/readiness/revocation rollback checkpoints are
rechecked atomically before one existing-harness call. This authenticates a
provider/operator assertion; it does not observe health, reserve capacity,
resolve credentials, contact a provider, retry, fail over, or deploy a service.

## System boundary

The marketplace is split so no component receives authority merely because it
can calculate a result:

- **Buyer SDK/gateway:** authentication, policy, maximum authorization,
  streaming, cancellation, and local receipt verification.
- **Provider agent:** signed offers, encrypted execution, metering, health,
  capacity accounting, and receipt signing.
- **Router:** eligibility and deterministic selection; never holds provider
  credentials and never settles funds by itself.
- **Assurance verifier:** validates manifests, signatures, validity, revocation,
  attestations, and evidence policy; replaceable and customer-runnable.
- **Settlement operator:** submits already-authorized intents and monitors
  confirmations; cannot change price, usage, or destination.
- **HyperEVM contracts:** provider registry, buyer session escrow, provider
  bonds, cumulative settlement, public commitments, pause, and escape path.
- **HyperCore integration:** future hedge only after index and listing gates.

## Initial market definition

Launch exactly one economically coherent testnet service:

- one pinned open-weight model build;
- one region and latency tier;
- one request/response schema and tokenizer version;
- two separately administered authorized providers, with distinct settlement
  keys, infrastructure failure domains, disclosed control, and disclosed
  beneficial ownership;
- one settlement asset selected only after verifying its exact address,
  bytecode and proxy/admin controls, decimals, transfer behavior, supply
  backing, and allowlist status; Core/EVM linkage is verified separately only
  if used;
- immediate requests plus explicit availability windows;
- deterministic token metering and maximum-cost authorization;
- provider bond against narrow objective non-performance;
- no transferable capacity token, leverage, HIP-3 market, or mainnet funds.

## Objective SLA vocabulary

V1 remedies may cover only values available from attributable receipts:

- admitted request not started before a deadline;
- stream terminated before a declared completion condition;
- metered units exceed the signed authorization;
- service, provider, or route binding differs from the authorization;
- region differs only when eligible provider or hardware evidence binds the
  observed execution region;
- provider failed to submit a required signed receipt;
- reserved availability below the declared window only when a declared
  observation protocol supplies attributable admission records, heartbeats, or
  challenge observations.

When the required evidence is absent or ambiguous, the result is `unsupported`,
`inconclusive`, or manual review—not automatic slashing.

Semantic quality, hallucination, truthfulness, usefulness, and subjective output
quality are excluded from automatic slashing.

## Twelve-month sequence

| Phase | Deliverable | Exit gate |
|---|---|---|
| 0 | Local market kernel | Portable bundle reconstructs route, metering, settlement, and tamper rejection |
| 1 | Private execution alpha | Two providers, encrypted streaming, cancellation, durable replay protection, no funds |
| 2 | HyperEVM testnet | Session escrow, bonds, cumulative vouchers, objective dispute fixtures, escape path |
| 3 | Paid design-partner beta | Bounded real settlement, production IAM, incident drills, independent security review |
| 4 | Reserved-capacity product | Availability windows, reservation accounting, SLA remedies, enterprise tenancy |
| 5 | Executed-price index | Published method, diversity and manipulation controls, reproducible observations |
| 6 | Optional HIP-3 hedge | Separate legal, oracle, liquidity, stake, operations, and basis-risk approval |

## Explicitly removed

HyperIntel contains no prediction-market resolution, binary claim truth,
outcome shares, HIP-4 dependency, general dispute voting, or oracle for
subjective inference quality. Disputes are limited to objective signed service
facts and contract state.
