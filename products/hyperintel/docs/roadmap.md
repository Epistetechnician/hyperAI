# Delivery roadmap

## R0: local market kernel

Complete local implementation target: canonical domain objects, assurance
envelopes, provider-key signed-offer admission, reliability-qualified routing,
receipts, settlement intent, compatible V1/V2/V3 evidence, and additive V4
evidence. V4 retains every typed eligibility report for selected and
all-rejected evaluations, binds the result to a deterministic caller-supplied
replay-state transition, and is exercised by committed fixtures through the
compiled CLI process for every V1–V4 schema. A source-hash-bound Z3 4.15.0 test
checks one bounded two-candidate policy-non-widening property and a weakened
counterexample.

R0 completion is a local implementation claim only. The replay transition does
not authenticate the buyer or provide current, durable, atomic,
rollback-resistant, concurrent, or externally consumed state. The formal model
is handwritten and does not prove Rust-source correspondence. No R0 artifact
authorizes inference execution, networking, persistence, custody, chain action,
funds, or production use.

## R1: private execution alpha

Future state slices: HTTP gateway, external provider adapters, encrypted and
live streaming transport, live cancellation, passive health, automatic route
selection, direct-provider mode, production persistence, and production IAM. No
chain dependency.

The next MVP tranche is authenticated buyer admission, durable atomic replay,
two provider agents, request deadlines, streaming and cancellation, health and
failover, signed execution and metering receipts, tenant isolation, receipt
reconciliation, fault injection, and a synthetic no-funds soak.

The implemented first tracer is pure data: verify the exact buyer policy under a
caller-trusted `BuyerPolicy` key, bind a non-authoritative admission to a fully
reconstructed selected route from V4-grade signed-offer, assurance,
reliability, and replay inputs, verify one completed execution under the
selected provider's configured receipt key, and reconstruct metering from that
signed receipt. This landed at `dbfbd48`.

The local durable replay store landed at `f5d3eff`: SQLite schema checksum,
exact initial state, authenticated admission verification, serialized atomic
compare-and-swap, immutable consumption, exact-retry idempotency, competing
writer rejection, and close/reopen coverage. It has no prompt-release
authority and no rollback resistance, replication, production backup policy,
tenant isolation, or production database operations. Gateway/provider sockets,
live cancellation, automatic reroute, and external effects remain
separately gated; R1 is not complete.

The hermetic execution harness landed at `4c4b17f`: text-only OpenAI-shaped
requests and streaming chunks, exact prompt commitment, selected-provider and
receipt-key preflight, newly applied replay consumption before provider
invocation, exact-retry suppression, deterministic cancellation and deadline
boundaries, signed completion, metering reconstruction, and a
non-authoritative settlement intent. Its contracts exercise separately
selected alpha and beta providers plus provider, signer, and substitution
failures without sockets, credentials, funds, or value movement.

This closes only the in-process R1 logic path. The gateway, external provider
transport, live streaming/cancellation, health, automatic reroute, tenant
isolation, production operations, sustained soak, and operator acceptance
remain open. Reroute cannot reuse an admission bound to another provider; it
requires a new route and next-nonce buyer-authenticated admission.

The narrow authenticated reroute landed at `379ee98`. A zero-output beta
`UNAVAILABLE` attempt at nonce 1 can link to a separately signed alpha admission
at nonce 2 and complete on the same durable cursor. The lineage freezes buyer,
service, prompt, state succession, provider change, and time ordering.
Cancellation/partial output, same-provider reuse, prompt change, and stale
lineage are rejected. Route selection, policy signing, live failover, health,
and transport remain future work.

The metadata-only execution journal landed at `6a2d9d5`: separate SQLite start
and terminal records, serialized exact retries, conflict and tamper rejection,
actual replay-store-bound recovery, no implicit resume, and restart coverage
for consumed-without-terminal crash states. It stores no prompt or response
body. Replay and journal writes are not one atomic transaction; cross-database
repair, rollback resistance, replication, production backup, and automatic
recovery remain future operations work.

The local receipt reconciler landed at `f2b12a2`: deterministic identity-only
reports bind authenticated admission, durable replay consumption, journal
lineage, completed or incomplete execution, stream shape and timing, provider
receipt, metering, and non-authoritative settlement intent. Tamper and
self-consistent impossible states fail closed. External observation,
production persistence, payment, settlement finality, and production
reconciliation remain open.

The strict local runtime configuration landed at `ec46411`: one bounded
explicit `LOCAL_HERMETIC` JSON document, canonical identity, distinct absolute
lexical SQLite paths, bounded connection counts, and opaque immutable-version
secret references. Unknown fields and unsafe path shapes fail closed. It does
not merge ambient state, resolve secrets, create paths, open databases, bind
sockets, or authorize deployment. Tenant IAM, secret resolution, gateway and
provider transport, production persistence operations, and soak remain open.

The pure-data tenant IAM verifier landed at `f8a43dd` and was hardened through
`70a344b`: fixed roles, one-action maximum-300-second grants, exact caller-owned
audience/runtime/policy/resource/operation bindings, dedicated tenant-
namespaced IAM keys, and signed monotonic revocation snapshots rechecked
against current trust and caller-pinned rollback checkpoints. It provides
local authorization separation only. Token issuance, durable token replay
prevention, gateway enforcement, runtime tenant isolation, and production IAM
remain open.

The local secret-injection seam landed at `22ddd2c`: dedicated signed
operator-secret trust, exact runtime/consumer/provider/reference/purpose/
operation binding, freshness-bounded revocation and rollback checkpoint, one
injected resolver invocation, and bounded redacted zeroize-on-drop material.
It has no ambient source, real resolver adapter, network, provider call, cache,
retry, fallback, persistence, deployment, or production credential authority.
The gateway and external provider transport remain open.

The hermetic provider-agent control layer landed at `1944478`: signed service
manifests and short-lived readiness assertions, dedicated trust, signed
revocation, exact service/offer/runtime/artifact/capability/key/reference
bindings, and manifest/readiness/revocation rollback checkpoints are verified
before one existing-harness call. It does not observe health, reserve capacity,
resolve credentials, open a socket, contact a provider, retry, fail over, or
authorize deployment. Real provider agents and transport remain open.

## R2: HyperEVM testnet

Future contracts: provider registry, buyer session escrow, cumulative vouchers,
bond manager, settlement hub, receipt commitments, objective disputes, multisig,
timelock, pause, and escape path.

The testnet exit gate includes measured fast/slow-block gas behavior, nonce
queue handling, indexed event reconstruction, RPC rate-limit and outage tests,
and a non-single-RPC production design. Current documented parameters such as
block cadence, gas limits, accepted future nonces, and official API rate limits
are deployment inputs to re-query, not protocol constants embedded in code.

## R3: capped mainnet beta

Requires external audits, verified settlement asset, exposure caps, practiced
incident recovery, legal approval, paying design partners, and a sustained
testnet soak.

## R4: reserved-capacity GA

Adds availability windows, reservations, enterprise tenancy, on-prem verifier,
release provenance, and repeated proof-to-enforcement customer runs.

## R5: reference index

Publishes a non-tradable executed-fill index only after diversity,
concentration, manipulation, staleness, and reproducibility gates pass.

## R6: optional HIP-3 hedge

Requires a separate legal, oracle, operational, liquidity, stake, slashing, and
basis-risk decision. It is never a dependency of inference authorization or
physical settlement.
