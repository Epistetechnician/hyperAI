# Migration provenance

HyperIntel is a new inference-native repository. It does not preserve either
source repository's outcome-market or benchmark-specific domain model.

## Source A: HyperAI

- Repository: `/Users/shaanp/Documents/GitHub/hyperAI`
- Commit: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`
- Working tree at migration: clean.

Generalized foundations:

- `crates/outcomes-domain/src/canonical.rs` -> canonical JSON validation and
  domain-separated digesting.
- `crates/outcomes-domain/src/identifiers.rs` -> strict SHA-256 representation.
- `crates/outcomes-domain/src/timestamp.rs` -> canonical UTC whole-second time.
- `crates/outcomes-evidence/src/lib.rs` -> offline bundle verification,
  activation/revocation-aware signature patterns, and tamper rejection.
  HyperIntel's signed-capacity-offer V2 path reuses only this already
  generalized caller-owned registry and canonical Ed25519 verification pattern;
  it adds inference-native offer/provider/envelope binding without copying an
  outcome schema or development signer.
- `crates/outcomes-settlement/src/lib.rs` -> deterministic intent identity,
  canonical payloads, idempotency, and fail-closed preflight patterns.
- `crates/outcomes-domain/src/authorization.rs`,
  `crates/outcomes-domain/src/receipt_identity.rs`, and the injected boundary
  pattern in `crates/outcomes-api/src/lib.rs` -> bounded authenticated-request
  failures, canonical receipt identity, and caller-supplied verification
  dependencies consulted for the inference-native authenticated execution
  tracer. No operator role header, HTTP route, outcome record, development
  authority, or persistence adapter is copied.
- `crates/outcomes-persistence-sqlx/src/lib.rs` -> exact SQLx 0.8.6 and SQLite
  WAL configuration, busy-timeout, explicit transaction, immutable replay,
  and fail-closed idempotency patterns consulted for the inference-native
  durable replay store. Only the pinned dependency versions and generic
  transaction pattern are reused; no claim, outcome, resolution, market,
  dispute, settlement, event, outbox, or authorization-denial schema is copied.
- `crates/outcomes-api/src/readiness.rs` and
  `crates/outcomes-runner/src/lib.rs` -> injected clock/provider boundaries, a
  versioned execution port, and a deterministic scripted implementation
  consulted for the authorized local execution harness. The harness may reuse
  only these generic dependency-injection and hermetic-test patterns. No HTTP
  server, outcome request, validator manifest, process runner, OCI runner,
  development authority, operator header, or runtime adapter is copied.

Explicitly excluded:

- `ClaimSpecV1`, `ClaimTruth`, resolution aggregation, disputes over binary
  truth, `MarketBindingV1`, external sides, and all HIP-4 code;
- outcome-specific API, persistence, worker, and settlement schemas;
- development authentication and development-only signing authority.

## Source B: composed-zk-benchmark-os

- Repository: `/Users/shaanp/Documents/GitHub/composed-zk-benchmark-os`
- Commit: `26ac4eed8e9e698d45596167a279c92f19972e62`
- Working tree at migration: dirty; only committed Git objects were consulted.

Generalized foundations:

- `crates/hsai-claim-envelope/src/lib.rs` -> non-compensable guarantees,
  assumptions, exclusions, maturity, trust-root classes, validity, and
  acceptance-policy algebra.
- `crates/statebook-core/**` -> exact integer/rational discipline, canonical
  identities, semantic separation, and fail-closed validation patterns.
- `crates/statebook-settlement/src/completeness.rs` -> independent completeness
  dimensions plus bounded, closed, fail-closed validation patterns consulted
  for the inference-native typed reliability artifact. No Statebook subject,
  capital, settlement, recovery, or payoff type was copied.
- `crates/hsai-gateway-digest-checker/**` -> implementation-diverse checking as
  a future verification pattern.
- formal-workbench source-binding practice -> the local Z3 contract test pins
  the exact `routing.rs` digest, solver version, and expected verdict sequence.
  Only the process and claim-discipline pattern was reused. The
  `policy_non_widening_v1.smt2` model and all routing semantics are new,
  inference-native HyperIntel work; no benchmark-OS domain or formal-model code
  was copied.

Explicitly excluded:

- benchmark DSL, scoring, evidence promotion, external execution, economy,
  membrane, agent identity, payoff/residual market semantics, live attestation
  provider integrations, and all uncommitted admission/P01B work;
- any claim that local replay constitutes accepted evidence or production
authority.

## Local R0 extension lineage

- `4f8b640` adds the inference-native V4 full route-evaluation record,
  selected-or-all-rejected evidence reconstruction, pure-data buyer
  replay-state transition, and direct compiled-CLI fixtures. It preserves the
  V1, V2, and V3 schemas and their existing reconstruction behavior.
- `661cdc3` adds the local two-candidate policy-non-widening SMT model and its
  source-bound process test.
- `9703256` adds strict offer-aware winner validation, distinct assurance
  evidence/policy/signature rejection records, direct V1–V4 process fixtures,
  and the repository-controlled exact-version Z3 resolver.
- `6887ac1` scopes the distinct assurance evidence/policy/signature rejection
  taxonomy to the additive V4 causal-evaluation profile, restores the legacy
  collapsed rejection semantics used by V1, V2, and V3, and freezes exact
  rejection/fallback report and bundle commitments for all three legacy
  versions. The formal test now binds `routing.rs` SHA-256
  `a0ea4bc9129b509692ac3d4b55e215ba3514056cb09c270cdbaf95142d1bb425`
  and the expected `unsat` then `sat` verdicts. The resolver accepts only the
  reported version `Z3 version 4.15.0 - 64 bit`; it does not prove solver-binary
  provenance.

These local extensions create no provenance claim over runtime observations,
provider operations, buyer authentication, durable state, independent formal
correspondence, external settlement, or production authority.

## Local R1 extension lineage

- `dbfbd48` adds the inference-native authenticated execution receipt tracer:
  caller-trusted buyer-policy signatures, non-authoritative admission bound to
  a reconstructed selected route and replay transition, configured
  provider-receipt-key signatures over completed execution and usage, and
  metering reconstruction from the verified receipt. The implementation
  consulted only the pinned HyperAI patterns named above and copied no outcome,
  HTTP, operator-header, development-authority, or persistence code.

This tracer has no provider runtime, request-release authority, durable replay
consume, transport, secrets, cancellation, retry, settlement submission,
custody, funds, or production authority.

- `f5d3eff` adds the local durable replay store. It pins SQLx 0.8.6, uses SQLite
  WAL plus `BEGIN IMMEDIATE`, checksums its inference-native schema, verifies
  the full authenticated admission before mutation, atomically advances only
  an exact canonical before-state, and stores one immutable consumption record.
  The contract covers invalid buyer authentication, conflicting initialization,
  two competing admissions, exact retry, validated reads, and close/reopen.

The store reuses only the generic transaction and immutable-replay patterns
recorded under Source A. It copies no HyperAI schema. It has no remote database,
rollback-resistant anchor, provider runtime, prompt-release authority,
credential storage, settlement, custody, funds, or production authority.

- `4c4b17f` adds `hyperintel-local-execution-harness-v1`: a text-only
  OpenAI-shaped request and stream subset, exact prompt commitment, one newly
  applied durable replay consumption before provider invocation, strict
  selected-provider and receipt-key binding, deterministic in-process
  streaming, cancellation and deadline boundaries, completed receipt
  verification, metering reconstruction, and derivation of the existing
  non-authoritative settlement intent. The contract exercises separately
  selected alpha and beta providers, exact-retry suppression, provider
  substitution, provider failure, and signer failure without network,
  credentials, wallets, chain access, custody, funds, or value movement.

The implementation reuses only the pinned generic dependency-injection,
versioned execution-port, and scripted-runner patterns recorded under Source
A. Its domain objects and execution semantics are inference-native. It copies
no outcome request, validator manifest, HTTP route, process/OCI runner,
operator header, development authority, credential adapter, or source schema.

- `379ee98` adds `hyperintel-authenticated-reroute-v1`. It verifies a
  self-consistent provider-`UNAVAILABLE`, zero-output attempt and its original
  authenticated admission, then requires a caller-supplied admission with the
  same buyer, service, and prompt, the prior durable after-state, the next
  sequential nonce, a different selected provider, and non-regressing time.
  Only after that lineage passes does the existing local harness consume and
  invoke the next provider. Contracts cover beta-unavailable to alpha-complete
  on one durable cursor plus rejection of cancellation/partial output,
  same-provider reuse, prompt change, and stale lineage.

This slice reuses only existing HyperIntel records and no additional source
repository code. It adds no route selection, buyer signing authority, HTTP or
provider transport, health monitor, credential custody, settlement, funds, or
production authority.

`6a2d9d5` adds `hyperintel-durable-execution-journal-v1`. It reuses the existing
local SQLx and checksum patterns in a separate metadata-only SQLite journal
around the hermetic harness without changing the replay schema or copying
additional source code. Contracts cover concurrent exact-retry idempotency,
same-admission start and terminal conflicts, close/reopen, no-content records,
pre-existing consumption, consumed-without-terminal recovery, actual replay-
store binding, and cross-row, cross-store, and timestamp tampering.

The implementation creates no cross-database atomicity claim. Existing starts
and consumed-without-terminal states fail closed rather than implicitly
resuming. It adds no prompt/response retention, network, credentials,
settlement, funds, or production authority.

`f2b12a2` adds `hyperintel-receipt-reconciliation-v1`. It composes only existing
HyperIntel admission, replay-consumption, journal, execution, metering, and
settlement-intent records into a deterministic pure-data consistency report.
Five contracts cover stable completed alpha/beta reports, every incomplete
status, tamper and terminal-shape failures, impossible stream times, report
reconstruction, and no-content serialization. No additional source code or
runtime dependency was copied.

The report grants no authority and creates no claim of independent metering,
external observation authenticity, payment, settlement finality, custody,
networking, or production readiness.

`ec46411` adds `hyperintel-runtime-configuration-v1`. It reuses only the
repository's canonical JSON, strict identifier, digest, serialization, error,
and local-file test patterns. No source code or runtime dependency was copied.
The implementation validates a bounded local-hermetic document and opaque
immutable-version secret references; it does not resolve secrets, merge
ambient configuration, create paths, open databases, bind a socket, contact a
provider, or add settlement authority. Size, schema, canonical digest, path,
collision, connection-bound, secret-redaction, no-side-effect, strict Clippy,
and full repository gates passed on the committed candidate.

`f8a43dd` adds `hyperintel-tenant-iam-v1`; `1ddfee2` binds each validated
revocation proof to its exact trust record, evaluation time, and current
caller checkpoint; `70a344b` freezes the complete role/action/resource matrix,
revocation tamper boundary, and redacted validated wrappers. The slice reuses
only existing HyperIntel canonical JSON,
strict identifier and timestamp, Ed25519, digest, and fail-closed validation
patterns. Its tenant policy, dedicated purpose-separated IAM trust registry,
one-action grant, exact caller expectation, resource ownership, signed
revocation snapshot, rollback checkpoint, and opaque result are new inference-
native records; no source code or runtime dependency was copied. It issues no
token, generates or stores no key, reads no ambient configuration or
credential, opens no filesystem, database, or socket, contacts no provider,
consumes no durable replay state, releases no request, and grants no
settlement, custody, funds, production-IAM, or production authority.

`22ddd2c` adds `hyperintel-local-secret-injection-v1`. It reuses only the
repository's canonical JSON, strict identity/time/digest, Ed25519,
fail-closed verification, opaque-result, runtime-config secret-reference, and
dependency-injection patterns; no source code was copied. New dedicated
operator-secret trust authorizes one exact runtime, consumer, selected
provider, immutable reference, provider-API purpose, and operation context
under signed freshness-bounded revocation and a caller-pinned checkpoint. One
injected resolver call returns bounded, redacted, nonserializable,
zeroize-on-drop material. Hermetic tests use synthetic bytes only. The slice
adds no ambient credential adapter, environment/file/keychain access, network,
KMS/HSM/cloud secret manager, provider call, cache, retry, fallback,
persistence, logging, funds, deployment, production credential, or production
authority.

`1944478` adds `hyperintel-provider-agent-v1`. It reuses only the repository's
canonical identity/time/digest, Ed25519, strict fail-closed validation, local-
execution, and dependency-injection patterns; no source code was copied. New
purpose-separated provider-agent trust binds signed service and readiness
assertions to exact provider, agent, service, selected offer, runtime
configuration, model, tokenizer, runtime image, capabilities, configured
receipt key, text-stream interface, immutable secret-reference, time,
revocation, and full-binding checkpoint identities. One atomic wrapper enters
the existing local harness at most once after preflight, with no retry or
fallback. Assertions are not described as observed liveness, availability
proof, independent attestation, or routing authority. No socket, HTTP, TLS,
ambient credential, secret resolution, external provider, live transport,
health observation, automatic routing/failover, new replay authority, signing-
key custody, settlement, funds, deployment, production identity, or production
authority was added.

Queued after hermetic provider-agent control is
`hyperintel-hermetic-runtime-composition-v1`. It may reuse only the committed
runtime-configuration, tenant-IAM, operator-secret, provider-agent, local
execution, replay, journal, and reconciliation public contracts. A narrow
journal-owned provider-agent attempt seam must retain sole ownership of start/
recovery, single invocation, actual replay inspection, and terminalization,
deriving terminal metadata from the opaque provider-agent result and closed
errors rather than caller assertions. A narrow
provider-agent activation seam must run one closure only after current control
and execution bindings pass and before harness entry. One public read-only
harness-preflight seam may be added to `hyperintel-execution` only so the
existing harness and provider-agent use the same validation implementation;
copying that logic is forbidden. One public read-only persistence preflight may
likewise be added only so both execution preflight and the mutating replay
consume share authenticated-admission and consumption-window validation. The
new composition may
then bind one exact local runtime, tenant `SUBMIT_INFERENCE` authorization,
synthetic secret resolution, provider-agent lineage, replay-protected local
execution, journal terminal, and reconciliation report. No existing source may
be copied and no wrapper may be bypassed or invoked twice. Only fixture keys,
synthetic bytes, injected ports, and in-memory or temporary local SQLite are
authorized in normal tests. No socket, ambient credential, real secret
adapter, external provider, remote database, live transport, automatic
routing/reroute, cross-database atomicity, settlement, funds, deployment, or
production authority may be added.

## Licensing boundary

Neither source checkout exposed a root license file during the migration audit.
HyperIntel therefore records provenance but does not assert a public
redistribution license. A license must be selected by the repository owner
before external distribution.
