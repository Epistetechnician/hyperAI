# Controlled local beta signoff

- Template state slice: `outcomes.beta_signoff_template.v1`
- Record state slice: `outcomes.beta_signoff.v1`
- Manifest schema: `hsai.local-beta-run-manifest.v4`
- Profile schema: `hsai.local-beta-profile.v3`
- Inventory schema: `hsai.local-beta-artifact-inventory.v1`
- Provenance schema: `hsai.local-beta-run-provenance.v1`
- Run ID:
- Run started at (UTC):
- Run completed at (UTC):
- Operator preflight approval, name, and UTC time:
- Independent reviewer preflight approval, name, and UTC time:
- Repository commit:
- Worktree clean before run: yes / no
- Profile ID:
- Profile digest (`beta-run-manifest.json.profile_digest`, domain `hsai.local-beta-profile.v3`):
- Artifact root:
- Manifest raw-file SHA-256 (`beta-artifact-inventory.json.manifest_digest`):
- Inventory digest (`beta-artifact-inventory.json.inventory_digest`, domain `hsai.local-beta-artifact-inventory.v1`):
- Provenance digest (`beta-run-manifest.json.provenance_digest`, domain `hsai.local-beta-run-provenance.v1`):
- Source artifact root:
- Retention parent:
- Retained run root:
- Retention end date or policy:
- Deletion approvers:
- Disk budget:
- Observed retained run size:
- Free disk before copy:
- Free disk after copy:
- Incident owner:
- Stop conditions agreed before run: yes / no
- Stop-condition policy revision:

## Manifest record

- Manifest status:
- External effect:
- Artifact classification:
- Tamper check status and expected result:
- Exact manifest claim ceiling:

## Gate record

- `corepack pnpm run lint` exit code:
- Root gate completed at (UTC):
- Pinned Rust version observed:
- Pinned Cargo version observed:
- pnpm version observed:
- PostgreSQL infrastructure gate: not run / pass / fail
- OCI infrastructure gate: not run / pass / fail
- Tool and host inventory retained: yes / no
- Secrets and private material absent: yes / no

## Scenario record

| Scenario | Expected truth/finality | Observed truth/finality | Expected/observed effects | Execution outcomes | Reconciliation | Bundle paths/digests | Verifier | Result |
|---|---|---|---|---|---|---|---|---|
| `true` | `TRUE/FINAL` | | `1 /` | | | | | |
| `false` | `FALSE/FINAL` | | `0 /` | | | | | |
| `invalid-then-valid` | `TRUE/FINAL` | | `1 /` | | | | | |
| `disputed-false-counterexample-true` | `TRUE/FINAL` | | `1 /` | | | original + replacement | | |
| `incomplete-observation` | `UNRESOLVED/NONE` | | `0 /` | | | | | |
| `divergent-replays` | `UNRESOLVED/NONE` | | `0 /` | | | | | |
| `runner-error` | `UNRESOLVED/NONE` | | `0 /` | | | | | |

## Operational checks

| Order | Check | Status | Report path/digest | Database path/digest | Retained verifier |
|---:|---|---|---|---|---|
| 1 | `sqlite-settlement-recovery` | | | | |
| 2 | `authorization-denial` | | | | |

## Incidents and deviations

List incident IDs and deviations. A scenario mismatch cannot be waived into a
passing run.

- Incident IDs:
- Deviations:
- Retention decision:

## Artifact review

- Exact-set inventory verified after retention copy: yes / no
- Artifact classification reviewed: yes / no
- Credentials, private evidence, personal data, and unsafe paths absent: yes / no
- Run directory made read-only: yes / no
- Retained-copy verifier command, UTC time, and exit code:
- Retained observed inventory digest:
- Retained inventory digest matches source: yes / no

## Final disposition

- Candidate disposition before human signatures: `PASS` / `QUARANTINED` / `INVALID`
- Signed final disposition: `PASS` / `QUARANTINED` / `INVALID`
- Operator name, signature, and UTC time:
- Independent reviewer name, signature, and UTC time:

The signers attest to the exact manifest claim ceiling recorded above. An
unsigned packet remains pending and cannot represent a completed `PASS`
signoff. Automation and agent review cannot substitute for either human
signature.
