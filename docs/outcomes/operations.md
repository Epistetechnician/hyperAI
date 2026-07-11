# Operations and runbooks

## Operational signals

Bound metric labels to adapter/version/status; do not label with claim IDs,
candidate IDs, wallets, or raw errors. Required counters and histograms cover
claim truth/finality/settlement states, observation lag, duplicate/rejected
candidates, queue depth, verifier outcomes, replay divergence, evidence failures,
quorum latency/conflicts, disputes, settlement lifecycle, API freshness/rate
limits, outbox backlog, dead letters, pauses, and denied operator requests.
Telemetry exporters are future production work; the state records needed to
derive these signals are modeled now.

## Key rotation

Add the new verifying key with an activation time, retain the old public key,
switch the external signer reference, confirm quorum, then revoke the old key at
an explicit time. Never rewrite prior attestations. Pause finalization if quorum
cannot be established.

## Source outage

Record the cursor, gap, first failure, and freshness. Stop negative finalization;
do not convert outage to FALSE. Resume from the last durable cursor, prove page
continuity, and issue a new completeness assessment.

## Nondeterminism incident

Quarantine the verifier/environment tuple, retain all divergent manifests, mark
the claim unresolved or disputed, compare host compatibility facts, and require
a new claim identity if the frozen verification policy changes.

## Disputed resolution

Pause settlement, validate counterevidence under the original frozen policy,
append the dispute decision, recompute before finality where permitted, and never
edit the provisional record in place.

## Erroneous preflight or submission

Engage operator pause, preserve intent/request/response digests, query the
backend’s authoritative status before retry, and reconcile by deterministic
idempotency identity. Escalate ambiguous acceptance; never blind-resubmit.

## Evidence-store outage and recovery

Stop evidence publication and finalization when required blobs are unavailable.
Restore versioned objects, recompute every content digest, run standalone bundle
verification, then clear readiness. A database row is not evidence availability.

## Database migration and rollback

Record migration checksums. Apply forward migrations before workers. Roll back
application binaries only when schema compatibility is documented; never delete
append-only evidence/audit rows. Restore testing must cover database plus object
store consistency.
