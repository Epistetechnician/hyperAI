# Evidence and attestations

`OutcomeEvidenceBundleV1` is an immutable manifest of frozen claim semantics,
observation/completeness evidence, candidates, admission decisions, execution
manifests, aggregation, disputes, build provenance, publication policy, and
content-addressed blobs. Each blob records algorithm, digest, length, media
type, classification, and a non-authoritative locator.

A digest is an integrity commitment to bytes, not a proof of the semantic
claim. The standalone verifier recomputes the bundle identity and every supplied
blob digest and length without database access. Redaction creates a new derived
artifact and never changes the original digest.

`ResolutionAttestationV1` signs canonical, domain-separated fields rather than
an API response. It binds claim ID/spec digest, truth, finality, resolution
policy, evidence bundle, qualifying runs or completeness evidence, dispute
window, issuance sequence, signer, key, and algorithm. Development signers are
generated from OS randomness at runtime. Private seed bytes are never accepted
in config or CLI arguments, logged, stored, or committed.

Quorum verification rejects invalid signatures, inactive/revoked keys, duplicate
logical signers, and mixed claim, policy, evidence, verdict, or finality payloads.
The local acceptance policy uses 2-of-3 semantics with two runtime-generated
signers; this is a test boundary, not a production independent-attestor claim.
