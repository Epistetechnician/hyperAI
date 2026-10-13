# Local secret injection

`hyperintel-secret-provider` implements the authorized
`hyperintel-local-secret-injection-v1` boundary. It verifies one short-lived
operator-secret authorization and, only after every preflight check succeeds,
performs one synchronous material-resolution call through a caller-injected
in-process resolver.

This is local dependency injection, not a credential backend. The crate ships
no environment, file, keychain, network, KMS, HSM, cloud-secret-manager, or
model-provider adapter. Normal tests use one hermetic fixture resolver and
synthetic bytes only.

## Dedicated trust and authorization

Operator-secret trust is independent of tenant IAM, buyer-policy,
provider-receipt, assurance, wallet, and settlement keys. A registered V1 key
is identified by operator, key ID, and the closed
`SECRET_RESOLUTION_AUTHORIZATION` key purpose. Duplicate registration fails
without overwriting the original key. Public-key material is redacted from
debug output.

The only V1 material purpose is `PROVIDER_API_AUTHENTICATION`. There is no
custom, wildcard, receipt-signing, wallet, or settlement purpose.

One signed authorization binds all of the following:

- its exact V1 wrapper and claims schemas, nonzero sequence, operator, signer,
  dedicated key, and Ed25519 algorithm;
- the validated runtime-configuration digest;
- the exact consumer and selected inference provider;
- the domain-separated digest of the complete existing `SecretReferenceV1`;
- the `PROVIDER_API_AUTHENTICATION` purpose;
- a nonzero operation-context digest and a signed maximum byte count;
- issuance and expiration, limited to at most 300 seconds.

`SecretReferenceV1` remains unchanged. Its provider identifies the injected
secret backend; its handle is opaque; and its nonzero version digest identifies
immutable provider-issued version metadata. The version digest must never be
derived from secret bytes.

Verification uses a caller-supplied expectation, explicit evaluation time,
dedicated key registry, current closed revocation state, and caller-pinned
rollback/fork checkpoint. Evaluation is inclusive at authorization and key
revocation instants. The revocation state is freshness bounded, monotonic, and
rejects rollback, same-sequence forks, unsorted entries, duplicate logical
authorization revocations, and duplicate logical key revocations.

The opaque verified authorization retains the exact evaluation time, trusted
key-record digest, revocation-state digest, and checkpoint digest. It is
created and consumed only inside the atomic verify-and-resolve function, so a
caller cannot mix a prevalidated artifact with later trust or revocation
context. The pre-material artifact reports no external authority.

## Resolver boundary

`resolve_authorized_secret_v1` performs authorization, expectation, time,
trust, signature, revocation, and resolver-identity preflight before material
resolution. Resolver identity inspection is a separate preflight metadata
query; after it passes, the function calls `InjectedSecretResolverV1::resolve`
exactly once. It performs no automatic retry, fallback, alias lookup, cache, or
alternate-provider attempt.

The request carries the exact runtime, consumer, selected provider, reference,
purpose, operation, byte limit, evaluation, complete signed-authorization,
trusted-key record, revocation-state, and checkpoint identities. The returned
candidate must repeat the exact secret-backend provider, handle, immutable
version digest, and purpose. Every post-resolution mismatch fails closed and
drops the returned material.

The resolver is caller supplied. This crate validates its declared identity
and returned metadata but does not establish or authorize the implementation's
effects, freshness, honesty, or retention behavior. Any real adapter or
external effect requires a separate state slice and operator acknowledgement.

## Material handling

Constructor-rejected bytes, returned-candidate metadata mismatches, signed-
limit mismatches, explicit zeroization, and successful-result drop all operate
on crate-owned zeroize-on-drop material. When the injected resolver returns an
error, no material reaches this crate; any cleanup remains the resolver's
responsibility and is outside this boundary.

Material is wrapped before empty and size validation. It must be nonempty, no
larger than the signed per-call limit, and never larger than 65,536 bytes.

`SecretMaterialV1`, resolver candidates, and successful results are not
cloneable or serializable and have redacted debug implementations. Secret
bytes are available only through an explicitly named borrowed accessor. They
are never hashed, formatted, added to errors, logged, cached, or persisted by
this crate.

Zeroization is best-effort memory hygiene for the current owned vector,
including its current capacity. It does not prove erasure of prior
reallocations, copies retained by a resolver or consumer, allocator state,
registers, swap, crash dumps, or physical memory. It is not process, tenant,
filesystem, or log isolation.

## Replay and authority ceiling

Each atomic call performs at most one material-resolution invocation. This
slice has no durable authorization consumption or replay store. A caller can
submit the same still-valid signed authorization in a later atomic call; that
causes a new resolver call rather than a cache hit. One-time durable use needs
a separately authorized composition slice.

A successful result proves only that dedicated caller-supplied trust accepted
one exact signed authorization under the supplied current revocation context,
and that one injected resolver returned matching nonsecret metadata with
bounded bytes placed in a redacted zeroizing container. The material-bearing
result deliberately makes no `external_authority_granted = false` claim:
possession of provider-authentication material may itself confer capability.

This slice grants no permission to call a model provider, release a request,
sign a receipt, access an ambient credential, contact an external secret
system, use a wallet or chain, submit settlement, hold custody, move funds,
deploy a service, or operate a production credential boundary.
