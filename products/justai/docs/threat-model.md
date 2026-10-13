# JustAI Threat Model

## Assets

- Integrity of signed public releases and source provenance.
- Safety and confidentiality of protected submissions.
- Privacy and safety of submitters, reviewers, affected people, and staff.
- Independence of publication, review, federation, and advocacy decisions.
- Availability and recoverability of public release artifacts.

## Threats and controls

| Threat | Required control |
| --- | --- |
| False, duplicated, or manipulated evidence | Exact snapshots, duplicate-key rejection, content digests, provenance, quarantine, independent review |
| Defamation, doxxing, or personal-data exposure | Data minimization, redaction, affected-party response, protected vault, publication review |
| Leaked credentials or exploit details | Secret scanning, no raw secrets in releases, coordinated disclosure, immediate release freeze |
| Reviewer or publisher conflict | Role separation, conflict declarations, recusal, independent review board |
| Key compromise or federation capture | Key registry, rotation, revocation, multi-signer support, independent mirrors, two-key emergency boundary |
| Prompt injection or autonomous model behavior | No model in the public renderer, bounded read-only intake, no public write path, sandboxed evaluation only |
| Insider misuse of protected intake | Least privilege, two-person access, immutable access logs, retention and legal-hold rules |
| Takedown, censorship, or outage | Content-addressed bundles, reproducible builds, independent mirrors, restore and rollback procedures |

## Freeze conditions

Freeze publication when a digest, signature, custody record, identity assertion, or disclosure boundary fails. Preserve the affected hashes, revoke compromised keys, notify the responsible reviewers and counsel, and require independent review before resuming. No emergency exception permits unauthorized access or disruption.

