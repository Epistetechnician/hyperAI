# State slices

Every mutation names one slice. A slice authorizes only the files and behavior
listed here.

## `hyperintel-repository-foundation-v1`

Root Git, Cargo, pnpm, agent, documentation, lint, and validation surfaces.
No product runtime, network access, credentials, contracts, or value movement.

## `hyperintel-inference-domain-v1`

`crates/hyperintel-core/**`: canonicalization, strict identifiers and timestamps,
service specifications, provider identities, capacity offers, request policies,
and exact integer money/rate types.

## `hyperintel-assurance-evidence-v1`

`crates/hyperintel-assurance/**`: typed assurance guarantees, assumptions,
exclusions, trust roots, validity, acceptance policy, Ed25519 signed-statement
verification, content-addressed evidence references, and generic validated
assurance evidence sets. This crate knows no routing or settlement types.

## `hyperintel-route-receipt-vertical-slice-v1`

`crates/hyperintel-market/**` and `crates/hyperintel-cli/**`: fail-closed offer
eligibility, provider-key signed-capacity-offer admission,
expected-successful-cost routing, route receipts, metering-to-cost calculation,
settlement conservation, deterministic idempotency, compatible unsigned V1 and
signed V2 market-evidence assembly and reconstruction, typed reliability-artifact
identity, caller-trusted verifier signatures, binding, validity, and exact-ratio
verification through an additive V3 market-evidence profile, offline verification,
portable full eligibility and rejection reports, pure-data buyer replay-state
transitions, selected-or-denied reconstruction through an additive V4
market-evidence profile, direct CLI-process verification, and an offline
environment-pinned local formal demonstration. Reliability-artifact and
replay-transition verification grant no authority and make no claim about
observation authenticity, representative sampling, calibration, stationarity,
future provider performance, durable state, state currentness, atomic
compare-and-swap, rollback resistance, buyer authentication, external nonce
consumption, or concurrent request serialization.

## `hyperintel-authenticated-execution-receipt-v1`

`crates/hyperintel-market/src/execution.rs`, the corresponding exports in
`crates/hyperintel-market/src/lib.rs`, and execution contract coverage in
`crates/hyperintel-market/tests/vertical_slice.rs`: caller-trusted
buyer-policy signatures, non-authoritative admission bound to a reconstructed
selected route from V4-grade signed-offer, assurance, reliability, and replay
inputs, provider-receipt-key signatures over completed execution and usage,
and reconstruction of the existing metering receipt from that verified
provider receipt. This slice is pure data. It authorizes no socket, provider
call, prompt transport, credential storage, persistent replay state,
cancellation runtime, retry, settlement submission, or value movement.

## `hyperintel-durable-replay-store-v1`

`crates/hyperintel-persistence/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, and durable-replay contract
coverage: local SQLite schema initialization, exact initial buyer-state
registration, caller-trusted authenticated-admission verification, and an
atomic compare-and-swap from the admission's before-state to after-state with
immutable consumption records and exact-retry idempotency. The state slice may
open only an operator-selected local filesystem path or an in-memory test
database. It authorizes no remote database, socket, prompt release, provider
call, credential storage, request body, cancellation, retry, settlement
submission, custody, or value movement.

## `hyperintel-local-execution-harness-v1`

`crates/hyperintel-execution/**`, its workspace membership and exact dependency
pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in
`package.json`, and direct execution-harness contract coverage: a text-only,
in-process OpenAI Chat Completions compatibility shape; an exact prompt
commitment; authenticated admission verification; one durable replay consume;
strict selected-provider binding; deterministic scripted streaming, deadline,
and cancellation semantics; a configured provider-receipt signer; completed
execution receipt verification; metering reconstruction; and derivation of the
existing non-authoritative settlement intent. Only a newly applied replay
consumption may invoke the selected provider. An exact retry fails closed and
does not execute again. Failed, cancelled, or late attempts cannot produce a
completed receipt, metering receipt, or settlement intent. This slice may use
only injected in-process provider and signer implementations. It authorizes no
socket, HTTP server, external provider call, ambient credential, environment
secret, tool call, image or audio request, live cancellation propagation,
automatic reroute, settlement submission, custody, funds, or value movement.

## `hyperintel-authenticated-reroute-v1`

`crates/hyperintel-execution/src/reroute.rs`, its module declaration and public
exports in `crates/hyperintel-execution/src/lib.rs`, direct reroute contract
coverage in `crates/hyperintel-execution/tests/authenticated_reroute.rs`, and
the focused-test command in `package.json`: verification and execution of one
caller-supplied reroute admission after a prior local provider-unavailable
attempt. The prior attempt must be self-consistent, have status
`PROVIDER_FAILED`, carry the closed `UNAVAILABLE` error, and have released no
chunks. The next authenticated admission must keep the same buyer, service,
and prompt commitment; begin from the prior consumption's exact after-state;
consume the next sequential nonce; select a different provider; and not precede
the prior terminal time. Only then may the existing local harness consume and
invoke the caller-supplied next provider. This slice may emit a self-digested,
non-authoritative reroute-lineage record. It does not select a route, construct
or sign buyer policy, retry an admission, reroute partial output, treat user
cancellation as failure, open a socket, contact an external provider, read a
credential, submit settlement, hold custody, move funds, or grant production
authority.

## `hyperintel-durable-execution-journal-v1`

`crates/hyperintel-execution-journal/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, and the focused-test command
in `package.json`: a separate operator-selected local SQLite journal for
metadata-only execution starts and terminal records. It may persist only
canonical admission, buyer, provider, prompt-commitment, replay-consumption,
outcome, status, and timestamp identities; it must never persist request or
response bodies, credentials, keys, arbitrary error text, or settlement
authority. Exact start and terminal retries are idempotent; conflicting reuse
fails closed. Restart inspection must distinguish a start with no replay
consumption from a consumed admission with no terminal record. The integrated
wrapper may start the journal, invoke the existing local harness, and retain a
terminal identity, but it cannot make journal and replay databases one atomic
transaction. A consumed-without-terminal state is non-retriable and requires a
new authenticated nonce or operator reconciliation. This slice authorizes no
remote database, socket, external provider, credential source, prompt log,
response log, automatic retry, settlement submission, custody, funds, or
production authority.

## `hyperintel-receipt-reconciliation-v1`

`crates/hyperintel-reconciliation/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, and the focused-test command
in `package.json`: deterministic, pure-data reconciliation of one authenticated
execution admission, its exact durable replay consumption, metadata-only
journal start and terminal records, and the existing completed or incomplete
local execution outcome. A completed report must reverify and bind the selected
route, provider receipt, stream commitment, metering receipt, and
non-authoritative settlement intent. An incomplete report must reverify its
closed status and stream shape and must contain no metering or settlement
identity. Rejected-before-consumption, failed-after-consumption, missing,
conflicting, or mismatched records fail closed and produce no positive report.
Reports persist only canonical identities, closed statuses, and digests; they
must never retain prompts, responses, signatures, keys, credentials, arbitrary
error text, or settlement authority. This slice authorizes no independent
metering observation, database mutation, socket, provider call, credential
source, payment, settlement submission, custody, funds, or production
authority.

## `hyperintel-runtime-configuration-v1`

`crates/hyperintel-runtime-config/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in
`package.json`, `config/examples/local-hermetic.json`, and
`docs/runtime-configuration.md`: strict loading and validation of one bounded
JSON runtime-configuration document for a local hermetic profile; canonical
runtime identity; absolute, distinct local SQLite replay and execution-journal
paths; bounded connection counts; a domain-separated configuration digest;
and an opaque, version-pinned secret-reference type for later service-specific
configuration. The slice may read only one operator-selected local
configuration file. It authorizes no defaults, environment merge,
interpolation, hot reload, remote configuration, secret material, secret
resolution, credential access, directory creation, database opening, socket,
provider call, wallet, chain, settlement, custody, funds, or production
authority.

## `hyperintel-tenant-iam-v1`

`crates/hyperintel-tenant-iam/**`, its workspace membership and exact dependency
pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in `package.json`,
and `docs/tenant-iam.md`: deterministic pure-data buyer-tenant, principal,
fixed-role, fixed-scope, signed access-token, revocation, and resource-ownership
contracts; caller-supplied IAM snapshots and Ed25519 verification keys; exact
schema, tenant, principal, audience, validity, role, scope, signature,
revocation, and ownership checks; and an opaque non-authoritative verified-
access result.

This slice issues no token, generates or stores no key, reads no ambient key,
secret, environment, configuration file, or credential, opens no filesystem,
database, socket, provider, wallet, or chain connection, consumes no durable
replay state, and grants no request-release, execution, settlement, custody,
funds, production-IAM, or production authority.

## `hyperintel-local-secret-injection-v1`

`crates/hyperintel-secret-provider/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in
`package.json`, and `docs/secret-provider.md`: dedicated caller-supplied
operator-secret Ed25519 trust; one short-lived signed authorization bound to an
exact runtime configuration, consumer, selected inference provider, immutable-
version `SecretReferenceV1`, provider-API-authentication purpose, operation
context, explicit evaluation time, and closed revocation state; one
preflighted invocation of an injected in-process resolver; exact returned
reference-metadata validation; and bounded, non-cloneable, nonserializable,
redacted, zeroize-on-drop secret material.

Normal tests may use only hermetic fixture resolvers and synthetic bytes. The
crate may invoke one caller-supplied resolver after authorization, but does not
establish or authorize the resolver implementation's effects or trust. It
authorizes no tenant-IAM key reuse, ambient environment/file/keychain access,
network, KMS/HSM/cloud-secret-manager adapter, model-provider call, receipt-
signing key, wallet, chain, settlement, cache, retry, fallback, persistence,
logging, custody, funds, deployment, production credential, or production
authority. Any real adapter, credential access, or external effect requires a
separate state slice and operator acknowledgement.

## `hyperintel-provider-agent-v1`

`crates/hyperintel-provider-agent/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in
`package.json`, and `docs/provider-agent.md`: dedicated caller-supplied
provider-agent Ed25519 trust; a signed service manifest and short-lived signed
readiness assertion bound to the exact provider, agent instance, service,
selected offer, runtime configuration, model artifact, tokenizer, runtime
image, capability set, configured receipt key, text-stream interface,
immutable secret-reference digest, validity window, revocation state, and
caller-pinned rollback checkpoint;
and one atomic preflight-and-execute wrapper around the existing local
execution harness. Logical duplicate revocations, sequence rollback, and
same-sequence forks must fail closed. Invalid control state must fail before
durable replay consumption or injected provider invocation. A valid call may
enter the existing harness exactly once with no retry or fallback and must
retain the exact admission, selected-route, manifest, and readiness lineage in
its local result.

The manifest and readiness record are provider/operator assertions, not
observed liveness, availability proof, independent attestation, or routing
authority. The slice may reuse existing canonical identity/time/digest,
Ed25519, local execution, and dependency-injection patterns, but its keys must
have dedicated provider-agent purposes and may not reuse offer, receipt,
tenant-IAM, operator-secret, assurance, wallet, or settlement authority.
Normal tests remain hermetic. The slice authorizes no socket, HTTP, TLS,
ambient credential, secret resolution, external provider call, live stream or
cancellation transport, health observation, automatic route selection or
failover, new durable replay mechanism, receipt-signing key custody,
settlement, funds, deployment, production identity, or production authority.

## `hyperintel-hermetic-runtime-composition-v1`

`crates/hyperintel-runtime-composition/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in
`package.json`, `docs/runtime-composition.md`, one exact journal-owned provider-
agent attempt seam in `crates/hyperintel-execution-journal/Cargo.toml` and
`crates/hyperintel-execution-journal/src/lib.rs` with direct coverage in
`crates/hyperintel-execution-journal/tests/durable_journal.rs`, and one atomic
post-control/pre-harness activation seam in
`crates/hyperintel-provider-agent/src/execution.rs` with its export in
`crates/hyperintel-provider-agent/src/lib.rs` and direct coverage in
`crates/hyperintel-provider-agent/tests/provider_agent.rs`, plus one canonical
public read-only harness-preflight seam in
`crates/hyperintel-execution/src/lib.rs` with parity coverage in
`crates/hyperintel-execution/tests/local_execution.rs`, and one canonical
read-only authenticated-admission consumption preflight in
`crates/hyperintel-persistence/src/lib.rs` with direct coverage in
`crates/hyperintel-persistence/tests/durable_replay.rs`: deterministic
composition of one validated local runtime configuration, one exact tenant
`SUBMIT_INFERENCE` authorization, one signed operator-secret authorization and
injected synthetic resolution, one signed provider-agent control lineage, one
durable replay consumption through the existing harness, one metadata-only
journal start and terminal, one receipt reconciliation, and one opaque
identity-only composition report.

The journal-owned provider-agent seam must exclusively own start/recovery
inspection, one provider-agent activation/execution attempt, actual replay
inspection, and terminalization. It must derive status, consumption, outcome,
and time from the opaque provider-agent result and closed error variants; no
public API may accept caller-asserted consumption ownership or arbitrary
terminal metadata. A concurrent or pre-existing consumption not mechanically
owned by this attempt receives no synthesized terminal. The seam must not
retry, resume, or duplicate execution, and the existing journaled harness must
retain its public behavior. The provider-
agent activation hook must run only after current signed control and exact
execution bindings plus the canonical harness preflight pass, but before
harness entry or replay consumption. The public harness preflight must remain
the single implementation used by both the harness and provider-agent; no
validation logic may be copied. It must call the persistence preflight, and the
mutating replay-store consume must call that same persistence preflight, so
buyer-admission authentication and the inclusive consumption-time window are
identical before activation and mutation. It
must invoke at most one activation closure and must not expose a reusable
verified-control token. The composition uses that hook to resolve one synthetic
secret into a private nonserializable secret-bound injected provider adapter;
activation denial fails before replay, provider, or signer invocation. The
exact runtime/configuration, tenant, principal, audience, operation, admission,
request/prompt, selected route/provider/offer, agent, receipt key, secret
reference/purpose, evaluation time, trust, revocation, and checkpoint
identities must agree. Evaluation time equals replay-consumption time.

Normal tests may use only validated local-hermetic configuration, in-memory or
temporary local SQLite databases, fixture keys, fixture resolvers, synthetic
secret bytes, and injected provider/signer implementations. The slice
authorizes no socket, HTTP, TLS, ambient environment/file/keychain credential,
real KMS/HSM/cloud secret manager, external provider, live streaming or
cancellation transport, automatic routing or reroute, health observation,
tenant-isolation claim, remote database, cross-database atomicity, automatic
crash resume, settlement submission, wallet, chain, custody, funds,
deployment, production credential, production identity, or production
authority.

## Reserved future slices

The following names reserve no implementation authority:

- `hyperintel-openai-gateway-v1`
- `hyperintel-private-transport-v1`
- `hyperintel-hyperevm-contracts-v1`
- `hyperintel-operator-settlement-v1`
- `hyperintel-capacity-reservation-v1`
- `hyperintel-index-publication-v1`
- `hyperintel-hip3-hedge-v1`
