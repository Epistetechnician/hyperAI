# Controlled local beta signoff

- Run ID:
- Run started at (UTC):
- Run completed at (UTC):
- Repository commit:
- Worktree clean before run: yes / no
- Profile ID:
- Profile SHA-256:
- Artifact root:
- `SHA256SUMS` SHA-256:
- Retention location:
- Retention end date or policy:
- Deletion approvers:

## Gate record

- `pnpm run lint` exit code:
- PostgreSQL infrastructure gate: not run / pass / fail
- OCI infrastructure gate: not run / pass / fail
- Tool and host inventory retained: yes / no
- Secrets and private material absent: yes / no

## Scenario record

| Scenario | Expected truth/finality | Observed truth/finality | Expected effects | Observed effects | Evidence verified | Result |
|---|---|---|---:|---:|---|---|
| `true` | `TRUE/FINAL` | | 1 | | | |
| `false` | `FALSE/FINAL` | | 0 | | | |
| `invalid-then-valid` | `TRUE/FINAL` | | 1 | | | |
| `disputed-false-counterexample-true` | `TRUE/FINAL` | | 1 | | original + replacement | |
| `incomplete-observation` | `UNRESOLVED/NONE` | | 0 | | | |
| `divergent-replays` | `UNRESOLVED/NONE` | | 0 | | | |
| `runner-error` | `UNRESOLVED/NONE` | | 0 | | | |

## Incidents and deviations

List incident IDs and deviations. A scenario mismatch cannot be waived into a
passing run.

## Artifact review

- Hash inventory verified after retention copy: yes / no
- Artifact classification reviewed: yes / no
- Credentials, private evidence, personal data, and unsafe paths absent: yes / no
- Run directory made read-only: yes / no

## Final disposition

- Disposition: `PASS` / `QUARANTINED` / `INVALID`
- Operator name, signature, and UTC time:
- Independent reviewer name, signature, and UTC time:

The signers attest only to a controlled, trusted-operator, single-host local
beta of a hermetic verifiable-outcome protocol with offline-verifiable evidence
and idempotent in-memory mock settlement. This signoff makes no live market
creation or settlement, production sandbox, production IAM, HA/DR, live-source
completeness, cross-host determinism, or exactly-once external delivery claim.
