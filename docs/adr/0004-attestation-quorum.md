# ADR 0004: attestation and quorum

Status: accepted for implementation.

Sign domain-separated canonical resolution attestations through a signer port.
Development policy is visibly 1-of-1. The production policy default is 2-of-3
distinct valid attestors with activation and revocation windows. Duplicate
logical signers and mixed claim, policy, evidence, or verdict digests are
rejected.
