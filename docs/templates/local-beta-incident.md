# Controlled local beta incident

- Incident ID:
- Run ID:
- Opened at (UTC):
- Incident owner:
- Repository commit:
- Profile ID and SHA-256:
- Affected scenario:
- Affected state slices:
- Classification: source / runner / nondeterminism / evidence / authorization / persistence / settlement / host
- External effects observed:
- Run directory made read-only at:

## Trigger and expected invariant

Describe the exact stop condition, command, exit code, expected result, and
observed result.

## Timeline

Record UTC timestamps without editing earlier entries.

## Preserved evidence

List artifact paths and SHA-256 digests. Do not include credentials, private
keys, tokens, or populated environment files.

## Containment

Record paused work, quarantined claim/verifier/environment tuples, blocked
effects, source-finality restrictions, and authoritative backend lookups. State
explicitly whether any retry or resubmission occurred.

## Cause and affected claims

State the verified cause or `UNKNOWN`. List affected truth, finality, evidence,
authorization, persistence, delivery, and settlement boundaries without raising
the repository claim ceiling.

## Recovery evidence

Record the corrective commit or environmental recovery, successful root gate,
restored digest verification, effect reconciliation, and the fresh replacement
run ID. Never resume inside this incident's run directory.

## Resume decision

- Disposition: `REMAIN_QUARANTINED` / `INVALID` / `APPROVED_FOR_FRESH_RUN`
- Operator name and UTC approval:
- Independent reviewer name and UTC approval:
- Residual limitations:
