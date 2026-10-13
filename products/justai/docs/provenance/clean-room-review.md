# Clean-room and adversarial review record

State slice: `justai-governed-discovery-monorepo-v1`.

Review class: local read-only clean-room review against the pinned source commits and the migration manifest, followed by a separate local adversarial boundary review. This is a mechanical implementation review. It is not external independent review, scientific assessment, production certification, hosted enforcement, or authorization.

## Review basis

- `manifests/source-repositories/manifest.json` records the audited branch, pinned commit, current follow-up head when a source advanced, working-tree condition, validation commands, license/provenance status, allowlisted reference paths, and excluded material for all eight source repositories.
- `manifests/migration/source-file-provenance.json` records the exact SHA-256 of every source file used by a migration capability `source_ref` at its pinned commit. The source inventory's additional `reference_paths` are audit pointers only and are not represented as imported migration inputs unless they appear in this manifest.
- `tools/verify_monorepo.py` checks branch identity, pinned Git object availability, observed-head metadata, reference existence, exact hashes for capability source references, unique capability owners, and capability reference coverage. A later source checkout head is reported as drift and is not a migration input.
- `manifests/migration/freeze.json` freezes the target slice and preserves unrelated dirty and concurrent work.

## Findings and dispositions

| Finding | Disposition in this slice |
| --- | --- |
| External seal could trust a self-supplied key | Handoff verification now requires externally supplied reviewer identity and authorized public key, binds the result and seal context, checks the seal reference, hashes the exact payload, and verifies Ed25519. |
| Local loop could emit a higher claim ceiling | Python and Rust governed paths reject ceilings above `local-replay`; higher ceilings remain separate gates. |
| Fast path could ignore a human veto | Any human disposition forces the slow path and is persisted in the intuition record. |
| Safety values and confidence could be coerced | Booleans and confidence are exact typed values with bounded ranges in Python, Rust, and the JSON schema. |
| Validation or alternatives could be unrelated to the proposal | Validation receipts bind the proposal digest; sampled candidate IDs and candidate digests bind the alternative set and order. |
| Unknown fields and malformed records could pass typed mapping | Recursive duplicate-key rejection, exact field allowlists, required-field checks, strict types, and schema constraints are applied before acceptance. |
| Persistence and rollback could accept tampered state | Result persistence is immutable; recovery requires a verified journal, an exact receipt, and a digest-bound known baseline. Ambiguity returns quarantine. |
| Subprocess evidence could be accepted or lose rejected outcomes | Fresh-directory, stdout/stderr digest, manifest digest, observable outcome, and `accepted_evidence_write=false` are required. |
| Hosted runtime could be constructed as authorizing | Authorization capability fields are fixed closed and constructor injection of `true` is rejected. |
| Declared discovery budgets could be advisory only | Proposal and recovery usage counters are checked against frozen maxima; alternative sampling also enforces the declared candidate limit. |
| Project export could disclose local record payloads | Platform export is aggregate-only and `create()` does not return mutable internal project state. |
| Candidate monoculture or self-validation could pass locally | At least two distinct candidate IDs are required, and an asserted independent validator must differ from the intuition reviewer. Organizational independence remains external. |

## Residual boundaries

Cross-record identity resolution, organizational independence, external reviewer custody, source rights, deployment state, provider behavior, live execution, paid execution, and scientific assessment remain caller- or externally governed. The current local policy permits bounded same-operator local quorum for the local decision-loop lane, but that mode does not manufacture external authority or raise its claim ceiling.

Source repositories remain in their original locations and are neither deleted, archived, modified, nor vendored by this migration. Dirty source checkouts and the pre-existing target change remain preserved.
