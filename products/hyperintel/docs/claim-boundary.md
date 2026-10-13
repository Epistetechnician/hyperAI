# Claim boundary

## Current supported claim

HyperIntel contains a local, pure-data implementation of one scoped
inference-market decision path. Its protocol tests and fixtures are hermetic and
can show that the implemented fixture obeys the named validation, routing,
arithmetic, digest, evidence-signature, signed-capacity-offer, typed
reliability, replay-transition, and deterministic reconstruction rules. The V2
path checks offer attribution against caller-supplied trust. The additive V3
path also checks that a caller-trusted verifier signed the exact typed
reliability artifact and that its provider, service, windows, counts, and
floor-PPM result match the offer.

The additive V4 path retains the full typed eligibility report for every
candidate and reconstructs either a selected route or a deterministic
all-rejected outcome. It binds both outcomes to a pure-data transition from the
exact caller-supplied buyer replay state to its deterministic successor.
Selected outcomes require metering and settlement records; denied outcomes
prohibit them. V4's evaluation profile distinguishes invalid assurance
evidence, policy rejection, and invalid offer authentication. V1, V2, and V3
remain compatible under their existing nonclaims and historical collapsed
assurance-rejection profile.

The formal contract test is an offline, environment-pinned local gate. With the
exact prerequisite `Z3 version 4.15.0 - 64 bit`, `tools/pinned-z3` checks the
reported solver version before running a handwritten two-candidate abstraction
bound to the recorded `routing.rs` source hash. Its supported result is `unsat`
for ineligible selection and `sat` for a deliberately weakened counterexample.
The resolver does not prove solver-binary provenance, and the model is not a
machine-checked Rust-source proof. No path grants external execution, replay,
provider, or settlement authority.

The separate R1 execution tracer verifies caller-trusted buyer-policy and
provider-receipt signatures. The local SQLite replay store verifies the
authenticated admission, serializes concurrent writers, atomically advances
the exact before-state, and retains an immutable consumption record.

The hermetic R1 execution harness then verifies an exact text-only
Chat Completions request commitment, consumes one new admission, and invokes
only the route-selected injected provider. Its local contracts exercise two
provider identities, deterministic streaming, cancellation, deadlines,
provider failure, signer failure, signed completion, metering reconstruction,
and a non-authoritative settlement intent. Exact retries do not execute again,
and provider substitution is rejected before replay consumption. These are
implemented local authentication, persistence, and in-process execution
claims. They grant no HTTP, external provider, credential, chain, settlement,
custody, funds, or production authority.

The authenticated reroute slice locally proves one narrower claim: a verified
zero-output provider-`UNAVAILABLE` attempt can link to and execute a separately
authenticated next-nonce admission for the same buyer, service, and prompt and
a different selected provider. It rejects cancellation, partial output,
same-provider reuse, prompt changes, stale replay lineage, and time regression
before invoking the next provider. It does not select a route, operate a buyer
signing key, or establish live failover.

The durable execution journal adds one local crash-classification claim. Its
metadata-only SQLite starts and terminals are immutable under exact identity,
concurrent exact retries are idempotent, and public recovery queries the actual
durable replay store before distinguishing an unconsumed start from a consumed
admission without a terminal. Existing starts never authorize implicit resume,
and cross-row, cross-store, timestamp, or identity mismatch fails closed. The
journal does not make its database atomic with replay consumption; a
consumed-without-terminal state requires a new authenticated nonce or explicit
operator reconciliation.

## Unsupported claims

The repository does not establish:

- model identity beyond supplied evidence;
- model or output correctness, truthfulness, safety, or alignment;
- confidential execution;
- provider authorization in the external world;
- capacity-offer signature verification in the V1 compatibility bundle, which
  retains its explicit unsigned-offer nonclaim;
- production offer-signing authority, key custody, or external provider
  authorization;
- production authentication or rollback-resistant, remotely replicated,
  tenant-isolated, independently anchored, or externally enforced replay
  state; V4 alone advances only caller-supplied pure data, while the separate
  R1 SQLite store provides local authenticated atomic consumption; the local
  harness releases a request only to an injected selected provider and grants
  no external prompt-release authority;
- full OpenAI API compatibility, an HTTP gateway, external provider transport,
  live cancellation propagation, automatic route selection/failover, provider
  health, or production provider-agent operation;
- atomic commit across replay and execution-journal databases, automatic crash
  replay or resume, rollback resistance, remote journal replication, prompt or
  response retention, or production recovery operations;
- truth, completeness, representative sampling, independence, or provenance of
  the source observations committed by a V3 reliability artifact;
- calibration, stationarity, causal validity, or future predictive accuracy of
  the artifact's historical success fraction;
- `AssuranceProperty::PolicyCompliance`; R0 rejects this assurance property
  because it has no policy-digest-bound assurance-evidence lane. Routing still
  evaluates the supplied policy's hard gates, but does not authenticate the
  buyer or release an external request;
- production authentication, availability, durability, or tenant isolation;
- independent reproduction or machine-checked Rust-source correspondence;
- HyperEVM contract correctness or deployment;
- HyperCore market availability;
- settlement finality, custody, payment, or value movement;
- legal, regulatory, or insurance status;
- production readiness or state of the art.

Digests prove byte commitments, not semantic truth. Signatures prove possession
of a signing key under the verifier's registry assumptions, not the truth of the
signed statement. Attestation, when added, must remain distinct from proof.
