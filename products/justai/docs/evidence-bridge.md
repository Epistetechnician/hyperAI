# Evidence bridge v1

`justai-evidence-bridge-v1` is the neutral receipt layer between open and restricted evidence. It does not ingest, copy, or expose restricted bytes. A public release contains metadata, hashes, bounded observations, uncertainty, nonclaims, review receipts, and disclosure decisions only.

## Receipt chain

```text
ArtifactManifest
  -> EvaluationReceipt
  -> ReviewReceipt + DisclosureDecision
  -> ClaimProjection
  -> PolicyTraceRecord
```

`ArtifactManifest` records the artifact kind, access class, custody mode, legal basis, custodian, acquisition time, content digest, permitted uses, and retention/disclosure rules. `public` with `reproducible` custody and `restricted` with `attested` custody use the same receipt shape but retain different claim ceilings.

`EvaluationReceipt` binds an artifact manifest to an authorization, evaluation scope, environment digest, runner digest, resource limits, observed-result digest, evidence basis, uncertainty, and nonclaims. `ClaimProjection` publishes only the strongest claim allowed by that evidence and its `DisclosureDecision`.

Every accepted review names an exact reviewed-record digest and a signed reviewer syndicate manifest. Two non-publisher reviewer organizations are required for publishable core records. A digest mismatch, unresolved reference, unsigned reviewer manifest, conflict, or missing disclosure decision fails closed.

## Policy boundary

The policy trace preserves:

```text
artifact -> evidence -> evaluation receipt -> review -> claim -> demand -> lawful target -> response
```

The bridge is not a private intake system, a universal safety adjudicator, a scientific replication authority, or a political authorization mechanism. Restricted material remains under its custodian's access and retention regime.

Real restricted handoffs must pass the separate [protected intake gate](protected-intake.md) before any digest-bound public receipt is considered for publication.

## Synthesis stages

The implementation gives the four synthesis stages concrete protocol duties:

- `solve`: separate observed results, projections, incentives, uncertainty, and power claims in the evaluation and projection records.
- `albedo`: expose custody, provenance, legal basis, missing information, and disclosure status without exposing restricted material.
- `coniunctio`: use one typed receipt chain for open reproducibility and restricted attestation while preserving distinct access and trust assumptions.
- `rubedo`: publish a bounded claim projection and connect it to a demand, lawful target, and response in the policy trace.

Symbols can generate questions, but only digest-bound evidence determines claims and only governance records determine permissible action.

## Synthetic fixtures

`fixtures/evidence-bridge/public/input` and `fixtures/evidence-bridge/restricted/input` contain metadata-only development fixtures. They produce structurally equivalent public receipts. The restricted fixture contains no raw restricted payload; its release exposes only an attested basis, aggregate summary, uncertainty, nonclaims, withholding decision, and contestation path.
