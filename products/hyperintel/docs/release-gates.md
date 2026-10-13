# Release and evidence gates

## Evidence vocabulary

- **Implemented:** source exists in the named state slice.
- **Locally checked:** declared offline checks and their exact prerequisites
  passed at the recorded commit.
- **Independently reproduced:** a separate operator replayed the exact bundle.
- **Testnet exercised:** configured contracts and services produced retained
  testnet artifacts.
- **Audited:** a named external review covers an exact commit and deployment.
- **Production authorized:** technical, operational, financial, and legal gates
  for the named scope are all approved.

No lower state implies a higher one.

## Gate matrix

| Release | Required technical evidence | Required operating evidence | Prohibited claim |
|---|---|---|---|
| R0 local kernel | Full local gate, tamper fixtures, deterministic reconstruction, direct CLI-process fixture, source-bound abstract property | Source provenance and claim boundary | Network, funds, durable replay, provider, chain, or production readiness |
| R1 private alpha | End-to-end transport tests, durable replay, cancellation, tenant isolation | Two authorized providers, incident runbook, synthetic/no-funds soak | Confidential computing or external settlement |
| R2 testnet | Contract invariants, deployment provenance, indexer reconstruction, escape tests | Key ceremonies, RPC failover, testnet accounting reconciliation | Mainnet safety or custody authorization |
| R3 paid beta | External security review, bounded production telemetry, rollback | Legal approval, exposure caps, reserve policy, support and incident drills | General availability or insured SLA |
| R4 GA | Reproducible releases, independent checker, repeated tenant isolation and recovery tests | Published support matrix, SLOs, audits, provider concentration controls | Universal model identity or correctness |
| R5 index | Reproducible methodology and observations, manipulation monitors | Governance, revision, incident and licensing policy | Tradable benchmark readiness |
| R6 hedge | Oracle and market-operation controls, liquidity and failure simulations | HIP-3 stake, legal, treasury, slashing and 24/7 operations approval | Capacity delivery or portfolio-margin equivalence |

## R0 acceptance checklist

- All public identities are canonical and domain separated.
- Floats and ambiguous timestamps are rejected.
- Arithmetic is checked and settlement conserves value.
- Ineligible supply cannot win regardless of price.
- Route selection is permutation invariant with explicit tie-breaking.
- The V2 path verifies each exact capacity-offer digest against the configured
  provider offer key, caller trust class, provider-assurance binding, validity,
  and revocation state. V1 remains an explicitly unsigned compatibility schema.
- Verified same-key sequence equivocation blocks the signed batch; unverified
  spoofed sequence fields cannot poison fallback, and rejected claimed keys do
  not become caller-trusted disclosures.
- The additive V3 path verifies the exact reliability artifact referenced by
  each signed offer: canonical typed identity, caller-trusted verifier
  signature, provider and service bindings, observation and validity windows,
  nonzero source and policy commitments, and floor-rounded count-to-PPM
  equality. Invalid artifacts are candidate-scoped hard failures.
- Distinct verifier-authenticated reliability artifacts at one signer, key,
  and logical sequence block the batch; exact artifact reuse is allowed.
- V4 reconstructs one canonically ordered, self-digested eligibility report per
  candidate, retaining complete typed rejections for selected and all-rejected
  evaluations. Selected evaluations require metering and settlement records;
  all-rejected evaluations prohibit both.
- Offer-aware validation rejects a self-digested but suboptimal selected route,
  and V4's causal profile distinguishes invalid assurance evidence, valid
  evidence rejected by assurance policy, and invalid offer authentication.
- Frozen V1, V2, and V3 rejection/fallback goldens preserve their historical
  collapsed assurance-rejection report, route, and bundle commitments while
  an economically preferred rejected offer falls back to eligible supply.
- Route-evaluation identity commits to the buyer-supplied replay snapshot and
  either the selected reliability-artifact-qualified offer or the deterministic
  all-rejected result.
- The V4 replay transition checks the exact buyer, snapshot, next nonce,
  evaluation digest, validity window, and deterministic successor state. It
  advances caller-supplied pure data only and grants no buyer authentication,
  durable or atomic state enforcement, currentness, concurrency control,
  rollback resistance, or external nonce consumption.
- Evidence signatures, content bytes, validity, and policy are verified offline
  against trust keys supplied separately from the bundle.
- Route, metering, settlement, and evidence tampering fails.
- The compiled CLI process verifies committed V1–V4 fixtures with separate
  caller trust and rejects missing trust, unknown schemas, invalid argument
  counts, and missing files. Constructor-bound byte comparisons fail on fixture
  drift.
- The local Z3 4.15.0 gate binds the SHA-256 digest of `routing.rs`, returns
  `unsat` when asked whether its two-candidate abstract selector can select an
  ineligible offer, and returns `sat` for a deliberately weakened selector.
  This is a handwritten abstraction, not Rust-source correspondence or runtime
  authority.
- Reliability verification does not establish that committed source
  observations are true or predictive of future delivery.
- R0 kernel and CLI sources import no HTTP, wallet, EVM, database, or
  secret-handling runtime. The separately scoped R1 persistence crate imports
  only local SQLx/SQLite and opens no remote database or provider connection.
  The separately scoped R1 execution crate invokes only injected in-process
  providers and opens no network or credential source.
- `pnpm run verify:full` passes on the pinned toolchain.

## R0 status

The documented R0 implementation scope is complete in the local lineage through
`6887ac1`: schemas, canonicalization, deterministic route
simulation and reconstruction, V1–V4 offline verification, direct process
coverage, and one bounded formal property are present. This status means only
**Implemented** and, when the full gate is rerun at the exact working-tree
commit, **Locally checked**. It does not imply independent reproduction, R1
private-alpha readiness, production authorization, or any external effect.

R1 begins at authenticated buyer and provider edges, durable atomic replay
state, OpenAI-compatible streaming and cancellation through two provider
agents, receipt reconciliation, tenant isolation, fault injection, and a
synthetic no-funds soak.

## R1 status

The pure-data authenticated execution receipt tracer is **Implemented** at
`dbfbd48` and locally checked by its contract coverage and the full repository
gate. It verifies caller-trusted buyer-policy signatures, reconstructs one
selected route and replay transition, verifies the configured provider receipt
key over a completed execution receipt, and reconstructs metering from that
receipt. Every emitted admission and receipt has `authority_granted = false`.

The local durable replay store is **Implemented** at `f5d3eff` and locally
checked by focused concurrency/restart coverage and the full repository gate.
It verifies the authenticated admission, atomically compares and advances the
exact buyer state, persists one immutable consumption, makes exact retry
idempotent, and rejects a competing writer as stale.

The hermetic execution harness is **Implemented** at `4c4b17f` and locally
checked by four direct contracts plus the full repository gate. It binds a
text-only OpenAI-shaped streaming request to the buyer prompt commitment,
preflights the selected provider and receipt signer, requires a newly applied
durable replay consume before provider invocation, and rejects exact retry
without re-execution. Its completed path verifies the provider receipt,
reconstructs metering, and derives a non-authoritative settlement intent. Its
incomplete paths cover deterministic cancellation, deadline, provider failure,
and signer failure without metering or settlement artifacts. Separate alpha
and beta selections are exercised with no sockets, credentials, funds, or
value movement.

Authenticated reroute is **Implemented** at `379ee98` and locally checked by
three direct contracts plus the full repository gate. Only a zero-output
provider-`UNAVAILABLE` attempt can link to the separately routed,
buyer-authenticated next nonce for an unchanged buyer, service, and prompt and
a different provider. The successful contract advances one durable cursor from
beta failure at nonce 1 to alpha completion at nonce 2. Cancellation/partial
output, same-provider reuse, changed prompt, and stale lineage fail before the
next provider is invoked.

The metadata-only execution journal is **Implemented** at `6a2d9d5` and locally
checked by concurrent exact-retry, conflict, close/reopen, no-content,
pre-existing-consumption, consumed-without-terminal, actual replay-store
binding, and cross-row/cross-store/timestamp tamper contracts plus the full
repository gate. Existing starts do not authorize implicit execution. The
journal persists no prompt or response body and grants no authority.

Local receipt reconciliation is **Implemented** at `f2b12a2` and locally
checked by five direct contracts plus the full repository gate. It rebuilds a
stable identity-only report across authenticated admission, durable
consumption, journal lineage, completed or incomplete execution, stream,
provider receipt, metering, and settlement intent. Completed/incomplete shape,
self-consistent impossible times, tamper, and report reconstruction fail
closed. This is internal local consistency, not independent metering or
production reconciliation.

Local runtime configuration is **Implemented** at `ec46411` and locally checked
by twelve direct contracts plus the full repository gate. It validates a
bounded explicit `LOCAL_HERMETIC` document, canonical identity, distinct
absolute lexical SQLite paths, bounded pool sizes, strict nested schemas, and
opaque immutable-version secret references. It does not resolve credentials,
open databases, bind sockets, contact providers, or establish production
configuration acceptance.

Local tenant IAM verification is **Implemented** at `f8a43dd` and hardened
through `70a344b`. Twelve direct contracts plus the full repository gate bind one fixed
action to caller-owned tenant, principal, audience, runtime, IAM policy,
resource ownership, and operation context under dedicated tenant-scoped keys
and signed revocation evidence rechecked against current trust and rollback
state. This establishes local authorization separation only—not identity
proof, token issuance, durable token consumption, gateway enforcement, runtime
tenant isolation, or production IAM.

Local operator-secret injection is **Implemented** at `22ddd2c` and locally
checked by ten direct contracts plus the full repository gate. It verifies one
short-lived signed authorization under dedicated trust and rollback-pinned
revocation, then invokes one injected resolver with exact runtime, consumer,
selected-provider, immutable-reference, purpose, and operation bindings.
Returned material is bounded, redacted, nonserializable, and zeroize-on-drop.
This is an in-process seam only—not an ambient credential source, real
provider/KMS adapter, network path, deployment, or production credential
acceptance.

Hermetic provider-agent control is **Implemented** at `1944478` and checked by
fourteen direct contracts plus the full repository gate. Signed manifests,
readiness, revocation, and full-binding checkpoints fail closed before replay
consumption or port invocation; valid control enters the existing harness once.
This is authenticated provider/operator assertion handling—not observed health,
capacity reservation, external transport, real credential use, live provider
operation, deployment, or private-alpha acceptance.

R1 remains open. The local store is not rollback-resistant, replicated,
tenant-isolated, encrypted by this crate, backed up, independently anchored, or
production operated. The harness is not an HTTP gateway, external provider
runtime, encrypted transport, live stream/cancellation implementation, health
system, or automatic failover path. The reroute slice does not select or sign
the next route. No production IAM, tenant isolation, external/production
receipt reconciliation, sustained two-provider traffic, incident runbook,
synthetic no-funds soak, independent reproduction, or private-alpha operator
acceptance has been implemented or exercised.
The replay and journal databases also have no cross-database atomic commit,
automatic crash resume, or production reconciliation workflow.

## Change control

Every mutation names a state slice. Schema changes require compatibility and
golden-vector tests. Routing or fee changes require before/after fixture replay.
Settlement changes require conservation, authorization, replay, and destination
tests. Assurance changes require explicit claim-ceiling review. Contract changes
require a new deployment identity and migration/escape analysis.
