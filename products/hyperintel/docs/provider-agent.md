# Provider agent V1

State slice: `hyperintel-provider-agent-v1`.

This slice adds a hermetic signed control layer around the existing local chat-completion
harness. It does not add a provider transport, health observer, secret resolver, credential
source, deployment runtime, route selector, retry policy, receipt-key custody, or settlement
authority.

## Exact control contract

The caller supplies three purpose-separated Ed25519 trust records: service-manifest,
readiness-assertion, and revocation-snapshot verification keys. Registration is keyed by exact
provider, key ID, and purpose. Duplicate registration, purpose substitution, malformed keys,
invalid validity windows, and inclusive key revocation fail closed.

The signed service manifest binds the exact provider, agent instance, service-spec digest,
selected-offer digest, validated runtime-configuration digest, model artifact, tokenizer,
runtime image, complete sorted capability set, configured receipt-key ID, fixed text-stream
interface, immutable secret-reference digest, sequence, and validity interval. The secret
reference is represented only by its caller-pinned digest; this crate never resolves it.

The separately signed short-lived readiness assertion repeats the complete binding and commits
to the complete signed-manifest digest. Its status is closed: only `READY` is admitted;
`DRAINING` and `UNAVAILABLE` fail before execution. This is a provider/operator assertion, not
an observed health signal, liveness proof, availability proof, or independent attestation.

The freshness-bounded signed revocation snapshot is bound to the provider and exact runtime
configuration. Assertion and key revocations are inclusive. Entries must be strictly sorted and
logically unique, so the same semantic target cannot appear twice with different timestamps.
The caller checkpoint rejects sequence rollback and same-sequence digest forks. Verification is
performed at the current explicit evaluation time against the current registry, snapshot, and
checkpoint inside every wrapper invocation. The checkpoint also binds the exact agent instance
and a domain-separated digest of the complete operational binding. There is no reusable validated
object to mix with another agent or later context.

## Atomic execution boundary

`execute_provider_agent_v1` verifies all signed control state first. It then reconstructs the
authenticated route from the existing execution inputs and compares the selected provider,
offer, service, provider identity, receipt key, model/tokenizer/image artifacts, exact
capabilities, model request, and fixed streaming interface. Any invalid control or binding state
returns before the existing harness, durable replay consumption, provider invocation, or receipt
signer invocation.

The control evaluation time must exactly equal the existing harness replay-consumption time. A
control decision evaluated before an inclusive revocation boundary cannot be reused to authorize
consumption at or after that boundary.

After successful preflight, the wrapper enters `execute_local_chat_completion_v1` exactly once.
It performs no retry, fallback, failover, or alternative route selection. Once entered, the
existing harness owns its documented replay and terminal behavior; for example, invalid provider
output can be rejected after replay consumption because that validation necessarily follows the
single provider invocation.

Completed and incomplete outcomes retain the authenticated admission digest, reconstructed
selected-route digest, selected-offer digest, complete manifest/readiness/revocation digests,
the three exact trusted key-record digests, checkpoint digest, evaluation time, provider and
agent identities, and the original terminal local outcome. `authority_granted` is always false.
The wrapper's `Debug` representation redacts digest lineage and the complete terminal outcome;
this narrow formatter behavior is not a general logging or confidentiality guarantee.

## Evidence ceiling

These signatures, digests, checkpoints, fixtures, and local tests are scoped local evidence.
They do not establish observed availability, confidential execution, model correctness,
production identity, external provider connectivity, deployment readiness, settlement
authority, custody, or value movement.
