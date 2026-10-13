# Local tenant IAM V1

`hyperintel-tenant-iam` is a pure-data authorization verifier for the future
buyer-facing gateway. It verifies one exact, short-lived action under
caller-supplied tenant policy, dedicated IAM keys, caller-owned resource
ownership, explicit operation context, and authenticated revocation evidence.

## Closed authorization model

V1 has three fixed roles and no wildcard or custom scope:

| Role | Actions |
| --- | --- |
| `INFERENCE_CALLER` | submit, cancel own request, read own request, read own evidence |
| `TENANT_ADMINISTRATOR` | read tenant audit metadata, manage principals, manage IAM keys |
| `TENANT_AUDITOR` | read tenant audit metadata |

Every grant carries one action. It binds the tenant, principal, issuer,
audience, runtime-configuration digest, caller-owned IAM snapshot digest,
resource kind and purpose, resource-owner tenant and principal, resource
digest, operation-context digest, issuance, expiration, and nonzero sequence.
The maximum lifetime is 300 seconds with no implicit clock skew.

The caller supplies the expected values independently. Token assertions never
establish tenant ownership, resource ownership, trust, audience, or operation
context by themselves. Principal-owned operations require the exact principal;
administrator and auditor roles cannot override body ownership.

## Dedicated trust and revocation

The crate owns a tenant-namespaced IAM key registry. Keys are indexed by exact
tenant, issuer, key ID, and one closed purpose: `ACCESS_GRANT` or
`REVOCATION_SNAPSHOT`. There is no fallback across tenants, issuers, keys, or
purposes, and IAM keys do not reuse assurance or buyer-policy trust classes.

Revocation evidence is a separately signed snapshot bound to the tenant,
issuer, audience, runtime configuration, issue/effective/expiry times, and a
nonzero monotonic sequence. The caller supplies a minimum accepted sequence
and the required digest at that sequence. Older snapshots and same-sequence
forks fail closed. Grant and key revocations take effect inclusively at their
recorded instant. Missing authenticated revocation evidence cannot produce a
verified access result.

The verified result has private fields, no public constructor, and no
deserialization path. It retains identities and digests only, redacts bearer
metadata in debug output, and always reports `external_authority_granted` as
false. Verification is repeatable and does not consume a token.

## Claim boundary

Successful verification establishes only that, under caller-supplied local
trust, policy, expected context, explicit time, and a signed non-rolled-back
revocation snapshot, an exact Ed25519-signed grant authorizes one exact local
action and resource binding.

It does not authenticate a real person or organization, issue tokens, generate
or store keys, establish wall-clock authenticity or globally current
revocation, provide logout or durable replay prevention, enforce process,
filesystem, database, cache, network, encryption-key, or log isolation, resolve
secrets, release a request, call a provider, authorize settlement, move funds,
or establish production IAM or deployment readiness. Fixture signatures and
local tests remain scoped local evidence only.
