# Product contract

HyperIntel will become an OpenAI-compatible market for policy-compliant,
SLA-backed, evidence-carrying inference.

The economic product is:

```text
service manifest x region x latency tier x privacy profile
x tool profile x assurance profile x availability window
```

Only equivalent and policy-eligible services compete directly.

## Buyer contract

The buyer declares permitted services, regions, privacy requirements, tool
requirements, assurance requirements, maximum cost, validity, and failover
rules. A missing or unsupported requirement denies routing.

## Provider contract

The provider declares authorization, service identity, capacity, price,
availability, operational risk inputs, settlement identity, and assurance
evidence. A signed offer is an attributable declaration, not proof that the
external service will perform.

The local V2 admission path verifies the exact offer digest, provider and offer
key identities, caller-assigned provider-key trust class, validity containment,
revocation state, and binding to verified provider-authorization evidence. It
does not supply, store, or operate a provider signing key.

Distinct, fully authenticated offers cannot reuse one provider-key logical
sequence. Sequence collisions are checked only after authentication so an
invalid wrapper cannot deny routing to unaffected valid providers.

## Router contract

The router first applies hard gates, then minimizes expected successful cost.
`RouteEvaluationRecordV1` emits one canonically ordered, self-digested
eligibility report per candidate, including typed rejection reasons and cost
when available. It records either a selected `RouteDecisionV1` or a
deterministic `NoEligibleOffer` outcome. A selected decision binds the complete
candidate-report digest set, scoring version, selected offer, and maximum
authorized cost. Offer-aware validation independently rejects a self-digested
selection that is not the deterministic minimum of the disclosed eligible
candidates. Rejection records distinguish invalid assurance evidence, verified
evidence that fails assurance policy, and invalid offer authentication instead
of treating those causes as interchangeable.

V4 binds the evaluation to a deterministic `ReplayStateTransitionV1`. The
transition accepts only the exact buyer, replay snapshot, and next nonce and
advances a full before-state to its successor. It is pure data: no buyer
signature, current-state lookup, database transaction, compare-and-swap,
rollback protection, or external nonce consumption is implemented.

That statement and the required V4 nonclaim below remain properties of the V4
bundle schema. The separate R1 execution tracer adds caller-trusted buyer
authentication, and `hyperintel-persistence` adds a local authenticated SQLite
compare-and-swap. Neither changes V4 bytes or causes a bundle verifier to read
external state. The local store grants no prompt-release authority and makes no
rollback-resistance, replication, tenant-isolation, or production claim.

The separate `hyperintel-execution` harness accepts only a text-only streaming
request whose exact digest equals the authenticated prompt commitment. It
preflights the selected provider and receipt signer, consumes the admission's
transition once, and only then invokes the injected in-process provider. An
exact retry is not re-executed. Provider substitution and a cancellation
already effective at consumption time leave the durable nonce untouched.

Every V4 bundle must carry these exact required nonclaim identifiers:

```text
no-inference-execution
no-buyer-policy-authentication-or-durable-atomic-replay-state-enforcement
no-observation-source-verification-or-future-reliability-accuracy
no-provider-authentication-beyond-supplied-evidence
no-model-or-output-correctness
no-confidential-execution-claim
no-settlement-or-value-movement
no-hyperevm-contract-claim
no-independent-reproduction
no-production-readiness
```

## Local execution contract

The implemented harness is a hermetic compatibility subset, not a gateway. It
emits OpenAI-shaped streaming chunks for one assistant choice and handles
deterministic cancellation and deadline boundaries. A completed path requires
a valid selected-provider plan, a configured provider receipt signature, exact
receipt reconstruction, and metering reconstruction before deriving the
existing non-authoritative settlement intent.

Cancelled, late, provider-failed, or receipt-signer-failed attempts contain no
metering receipt or settlement intent. Because the admission commits one
selected provider and offer, reroute requires a new policy decision and
next-nonce authenticated admission.

The implemented reroute lane accepts only the closed zero-output
`PROVIDER_FAILED/UNAVAILABLE` case. It verifies the prior attempt and admission,
the exact durable state succession, unchanged buyer/service/prompt bindings, a
different selected provider, and non-regressing time before passing the new
admission to the local harness. It rejects cancellation, deadline, partial
output, signer failure, same-provider reuse, prompt change, and stale lineage.
Route selection and buyer signing remain caller-owned.

The harness has no HTTP server, socket, ambient credential, external provider,
live cancellation propagation, wallet, settlement submission, custody, funds,
or production authority.

## Local receipt-reconciliation contract

The reconciler accepts only one fully authenticated selected admission, the
matching durable replay consumption, exact metadata-only journal lineage, and
one completed or incomplete local execution outcome. It deterministically
reconstructs an identity-only report and fails closed on any mismatch.

Completed reports reverify the provider receipt, exact stream commitment,
metering, selected route, and settlement intent. Incomplete reports verify the
closed status, stream shape, and time bounds and contain no metering or
settlement identity. The report retains no prompts, responses, signatures,
keys, credentials, or arbitrary errors. It proves internal local consistency,
not independent observation, payment, settlement finality, or authority.

## Settlement contract

The current pure-data settlement builder requires a successful route, fully
reconstructed metering, and buyer authorization. It enforces:

```text
authorized_max = provider_payout + platform_fee + buyer_refund
```

Buyer debit, provider/buyer co-signed receipts, bond remedies, and SLA credits
are future settlement-adapter behavior. When added, credits must be explicit
signed fields with their own conservation and collateral checks.

In V4, selected evaluations require both the reconstructed metering receipt and
settlement intent. All-rejected evaluations require both fields to be absent.
Neither branch grants execution or settlement authority.

No external effect is authorized in the current repository.
