# Frozen claim specification

`ClaimSpecV1` is the immutable semantic object frozen before market binding.
The human-readable question is presentation metadata; the authoritative
predicate is `EXISTS_ELIGIBLE_VALIDATOR_PASS` under the pinned eligibility,
observation, verification, deadline, resolution, dispute, attestation,
publication, and outcome-mapping policies.

The claim ID is derived, never supplied arbitrarily:

```text
digest = SHA-256("hsai.claim-spec.v1\0" || RFC8785(identity_payload))
claim_id = "claim_" || lowercase_base32_no_padding(digest)
```

The identity payload contains every field except `claim_id`. Resolution-critical
JSON rejects floating-point numbers and non-NFC strings. Protocol timestamps use
whole-second `YYYY-MM-DDTHH:MM:SSZ` form. Decimal settlement values use canonical
strings. Unknown fields are rejected; extensions are allowed only under
namespaced keys.

Changing a verifier, environment, deadline, eligibility rule, observation
completeness rule, fallback, dispute policy, attestation policy, publication
policy, or outcome mapping creates a new digest and claim ID. Progress is stored
in append-only operational events, never written into the frozen claim.

Generated protocol assets:

- `schemas/claim-spec-v1.schema.json`
- `fixtures/claims/local-demo-claim-spec-v1.json`
- `fixtures/claims/local-demo-claim-spec-v1.canonical.json`
- `fixtures/claims/local-demo-claim-identity-v1.json`
