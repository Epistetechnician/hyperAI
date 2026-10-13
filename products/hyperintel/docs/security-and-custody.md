# Security, privacy, and custody contract

## Protected assets

- prompts, outputs, embeddings, tool calls, and customer metadata;
- provider credentials, model weights, capacity schedules, and private offers;
- buyer authorizations, settlement keys, provider signing keys, and bond funds;
- service manifests, evidence, revocation state, receipts, and audit history;
- route integrity, availability, and deterministic replay.

## Trust boundaries

| Boundary | Required control |
|---|---|
| Buyer to gateway | Tenant authentication, request policy signature, maximum authorization, replay protection |
| Router to provider | Mutual authentication, encrypted direct transport, route binding, deadline and cancellation |
| Provider execution | Isolated workload, scoped credentials, signed metering, declared assurance and limitations |
| Evidence verifier | Pinned policy and keys, freshness, revocation, content digests, fail-closed unsupported cases |
| Operator to HyperEVM | Restricted submitter role, no authority to raise cap or change destinations |
| Contract upgrade | Multisig, timelock, bytecode provenance, pause and buyer escape |

The current R1 persistence slice covers only a local SQLite replay cursor and
immutable consumption record. Immediate transactions serialize local writers,
but the database is not encrypted by HyperIntel, replicated, backed up,
tenant-isolated, independently anchored, or protected against restoration of
an older file. It stores no prompt, response, provider credential, wallet key,
or settlement authorization.

The current execution harness invokes only an injected in-process provider
after exact request commitment, selected-provider, receipt-key, buyer
authentication, and newly applied replay-consumption checks. Exact retry is
not executed again. Provider substitution and already-effective cancellation
fail before nonce consumption. Deterministic cancellation, deadline, provider
failure, and signer failure cannot create metering or settlement artifacts.

The harness keeps request and response text in process and returns response
chunks to its caller; it does not persist those bodies. This is not a
confidentiality, secure erasure, tenant isolation, or no-retention proof. The
test receipt keys are fixtures, not an approved key-management boundary. No
HTTP, TLS, external provider, ambient credential, live cancellation, wallet,
chain, funds, or production authority is present.

The tenant IAM verifier at `f8a43dd`, hardened through `70a344b`, adds local
authorization separation, not runtime tenant isolation. Dedicated tenant-
namespaced keys, exact resource-owner bindings, short-lived one-action grants,
and signed rollback-pinned revocation evidence are checked as pure data. No
process, filesystem, database row, cache, network, encryption-key, or log
partition is implemented or claimed.

The local secret-injection seam at `22ddd2c` moves resolver-returned bytes into
bounded, non-cloneable, nonserializable, redacted zeroize-on-drop storage after
an exact signed operator authorization. Constructor rejection, returned-
metadata mismatch, signed-limit mismatch, explicit zeroization, and successful
result drop are crate-owned cleanup paths. Resolver errors return no material
to the crate, so their cleanup remains resolver-owned. No ambient credential
source or real adapter exists; a successful material-bearing result may itself
confer external capability.

The provider-agent control layer at `1944478` uses dedicated manifest,
readiness, and revocation keys and binds caller checkpoints to the full agent
deployment identity. Control evaluation and replay consumption use the same
explicit time, preventing a pre-revocation assertion from authorizing a later
consume. Invalid control cannot consume replay or invoke the provider/signer.
The records are provider/operator assertions, not independent health or
availability evidence, and the crate holds no signing key or credential and
opens no external connection.

## Custody posture

The target design avoids custody of provider API credentials and minimizes
custody of buyer funds:

- provider connections terminate at provider-controlled agents;
- buyer-owned credentials stay in buyer-controlled direct mode;
- the router receives opaque route/session references, not reusable secrets;
- buyer funds reside in explicit onchain session escrow with caps and expiry;
- company treasury, fee revenue, provider bonds, and buyer escrow remain
  segregated;
- operators can relay authorized settlement but cannot rewrite it.

Before production, counsel must classify the actual payment, escrow, routing,
fee, bond, data, sanctions, export-control, consumer, and derivatives activities
for each launch jurisdiction. Architecture does not substitute for legal
authorization.

## Privacy rules

- Never put prompt, output, credential, raw private evidence, or stable customer
  identifiers onchain.
- Commit only domain-separated digests with a documented retention and deletion
  policy for the corresponding offchain content.
- Default logs to structural metadata; payload logging requires tenant policy.
- Encrypt in transit and at rest with tenant-scoped keys and explicit rotation.
- Make retention duration and evidence classification part of service policy.
- A digest may still be personal or sensitive metadata when linkable; do not
  describe hashing as anonymization.

## Key lifecycle

Every signing or verification key has a purpose, owner, activation time,
expiration, revocation state, and allowed statement domains. Provider receipt,
buyer authorization, assurance verifier, contract upgrade, pause, deployer, and
treasury keys remain distinct. Rotation and compromised-key recovery must be
tested before funds are enabled.

## Failure policy

- Unknown or expired policy: deny.
- Missing or stale assurance: deny or route only to a separately declared lower
  assurance tier; never silently downgrade.
- Provider disconnect before completion: stop billing at verified usage. The
  current local reroute lane permits only a closed provider-`UNAVAILABLE`
  failure with zero released chunks and a separately signed, next-nonce route
  to a different provider. Partial output, cancellation, deadline, and signer
  failure remain non-reroutable. Live policy and transport remain future work.
- Local process crash: inspect the metadata-only execution journal against the
  actual durable replay store. An existing start never implicitly resumes, and
  consumed-without-terminal requires a new authenticated nonce or explicit
  operator reconciliation. The two SQLite databases are not atomically
  committed together.
- Metering disagreement: withhold disputed delta, preserve evidence, and apply
  the bounded dispute contract.
- Chain or RPC unavailability: inference may continue only within a buyer's
  still-valid offchain authorization and operator exposure cap; settlement
  queues without changing intent.
- Verifier disagreement: block stronger assurance promotion.
- Upgrade or key compromise: pause new exposure while preserving buyer refunds
  and escape.

## Required pre-production reviews

- independent application and protocol threat model;
- cryptography and canonicalization review;
- smart-contract audit plus invariant and differential testing;
- infrastructure, IAM, tenant-isolation, privacy, and incident-response review;
- economic manipulation and provider-collusion analysis;
- legal and regulatory analysis for the exact jurisdictions and asset;
- rehearsed key compromise, provider default, RPC outage, contract pause,
  refund, and migration exercises.
