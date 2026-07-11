# Adapter capability matrix

Snapshot date: 2026-07-10. The machine-readable source is
`config/adapter-capabilities.json`.

| Adapter | Mode | Readiness | Exact boundary |
|---|---|---|---|
| local fixture corpus and ledger | Hermetic fixture | SETTLEMENT_READY | Only for the frozen local demo policy |
| generic Git corpus | Pinned allowlisted snapshots | EVALUATION_ONLY | Settlement requires item verifier, time, completeness, and rights evidence |
| zk.golf source | Recorded fixture | EVALUATION_ONLY | Live completeness and authorization unverified |
| ecdsa.fail source | Recorded fixture | EVALUATION_ONLY | Live completeness and authorization unverified |
| NYU CTF Bench | Recorded pinned corpus | EVALUATION_ONLY | Corpus is not a live submission oracle |
| NYU agent harness | Offline trace or fake model | EVALUATION_ONLY | Harness score never decides truth |
| CTF Archives | Recorded allowlisted subset | CATALOG_ONLY | Item-level rights and reproducibility required |
| pwn.college archive | Recorded allowlisted subset | CATALOG_ONLY | Hosted service is not automated |
| CryptoHack archive | Recorded allowlisted subset | EVALUATION_ONLY | Hosted page is discovery only |
| public author profiles | Manifest overlays | CATALOG_ONLY | Ambiguous rights block execution and settlement |
| 247CTF manual source | Reviewed metadata and fixture | MANUAL_ONLY | No authenticated automation or completeness claim |
| HIP-4 read-only | Opt-in bounded Info API and recorded fixture | EVALUATION_ONLY | Discovery only; no write authority |
| HIP-4 mock | Hermetic fixture | SETTLEMENT_READY | Mock effect only; never external settlement |
| HIP-4 resolution publisher | Disabled | UNSUPPORTED_UNVERIFIED | Official action, authority, replay protection, and signing model undocumented |

No corpus is settlement-ready merely because it can be replayed. Readiness
requires a pinned deterministic verifier, reliable deadline evidence, a valid
completeness model, rights, and authorization.
