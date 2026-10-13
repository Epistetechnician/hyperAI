# Protected intake gate v1

`justai-protected-intake-v1` is a metadata-only handoff gate for real restricted or protected evidence. It does not ingest raw bytes, private locators, credentials, or restricted payloads. The packet carries only exact digests, bounded scope, timestamps, resource identity, signatures, and references to externally controlled records.

The verifier requires:

- an externally controlled identity registry and custody receipt bound to the artifact digest;
- one externally signed authorization with an active scope and expiry;
- one externally signed custodian attestation;
- two distinct externally signed witness attestations bound to the witnessed evaluation;
- two accepted independent review attestations from distinct organizations, each bound to a signed reviewer-manifest digest and exact review-record digest;
- a disclosure decision bound to an exact decision record, plus a public projection digest when the decision is `public` or `aggregate_only`.

Run the verifier with:

```text
python3 tools/protected_intake.py /external/metadata-only/intake-001.json
```

The external path is illustrative. The verifier must be run against a packet delivered by the external custodian through its approved custody channel. A local user authorization is not a substitute for external custody, an external identity registry, a witness, or independent review. Synthetic test keys prove only contract behavior and have no authority.

`publication_eligible=true` means the packet has passed this local cryptographic and structural gate and the disclosure decision allows a public or aggregate projection. It does not establish scientific validity, legal authorization, safety, production readiness, or political authority. The subsequent JustAI release validator must still resolve every referenced record and publish only the digest-bound public receipt.
