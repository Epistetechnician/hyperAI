# Monorepo Import Provenance

This document records source identity and import limits for HyperAI's modular
monorepo. A copied source tree is not an endorsement of its claims or a transfer
of execution, settlement, custody, reviewer, or production authority.

| Source | Source identity | Imported path | Import evidence and limits |
|---|---|---|---|
| Simple Compute Market | `arkhai-io/simple-compute-market` at `a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`; MIT | `products/simple-compute-market/` | The source import contains 2,189 files matching the pinned Git tree after excluding `.DS_Store` and `.pyc` artifacts. The MIT `LICENSE` is retained. `crates/hyperai-scm-adapter/` is independently implemented against its v2 signed `/listings` envelopes; its EIP-191 and Ed25519 test vectors are generated with the pinned source identity package. The root runtime does not load that Python package. Build and settlement roles remain separately gated. |
| HyperIntel | Local repository at `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc` | `products/hyperintel/` and read-only use from `crates/hyperintel-adapter/` | Imported from `git archive` of that exact commit: 99 files, 0 missing, 0 extra, 0 different. Its dirty source working tree was not copied. The local federated-catalog candidate described by a separate untracked note was absent from the committed source and is not imported. The HyperAI adapter calls source-owned V1–V4 market-evidence verification and pure-data receipt reconciliation from the pinned `hyperintel-assurance`, `hyperintel-core`, `hyperintel-market`, `hyperintel-execution`, `hyperintel-execution-journal`, and `hyperintel-reconciliation` crates; it copies and modifies none of those crates. Reconciliation consumes caller-selected complete lineage in memory and returns identity digests only. No root license file was identified; public redistribution is unverified, so this local integration is not a publication or redistribution grant. |
| JustAI | Local repository at `9a0e56536b968cbb2cb7173e142068b53c4f9fe3`, branch `codex/exploratory-local-quorum-v1`; `docs/local-quorum.md` was modified | `products/justai/` | 176 source files match the selected local tree after excluding `.git`, `.dist`, `.venv`, build caches, `target`, `node_modules`, `.DS_Store`, `__pycache__`, and `.pyc`. Its metadata-only local-quorum record is included as source-project evidence and remains same-owner self-verification, not independent review. Its current charter/status remain in force. No root license file was identified; public redistribution is unverified. |
| The Compute Bazaar | `gustofied/the-compute-bazaar` at `d1809e5cfa843c087ebc4de9f00561bf0be67d55`; `main` resolved to this same commit on 2026-10-02; the root `LICENSE` path returned HTTP 404 on 2026-09-30, 2026-10-01, and 2026-10-02 | `references/the-compute-bazaar-main/` | Local-only ignored reference. No source code, schema, fixture, dataset, generated output, asset, or runtime dependency is imported into tracked HyperAI files. Code and data rights remain unverified; keep it reference-only until written rights for exact materials and uses are established. See [`hyperai.bazaar.source_rights_recheck.v1`](state-slices.md#hyperai-bazaar-source_rights_recheckv1) and [`hyperai.bazaar.source_rights_local_license_discrepancy.v1`](state-slices.md#hyperai-bazaar-source_rights_local_license_discrepancyv1). |
| Desk | `gustofied/desk` at `2ac5d9b24377b21e0d3f928f1f09323dc5785fff` (main, inspected 2026-09-25); root `LICENSE` path returned 404 | None | Public README and package metadata inspected for planning. No code, schema, fixture, dataset, generated output, asset, or runtime dependency is imported. No reuse or redistribution license was recorded; a written rights grant or explicit license for the exact materials is required before forking or copying. Conditional integration is planned in `docs/compute-market-explorer-prd.md`. |
| [`subc-jobobject` 0.1.1](https://crates.io/crates/subc-jobobject/0.1.1) | crates.io package; checksum `129ca2f4dd4d310d9ccab92f8369206510920d0207060ef70a03a953d13f78b4`; MIT | Optional target-specific dependency of `crates/hyperai-acp/` | Used only on Windows when the default-off `windows-acp` feature is enabled. The checksum is recorded in `Cargo.lock`. The published metadata does not report an MSRV or repository URL. The registry archive API used by the adapter was inspected; no dependency source is copied into HyperAI. Windows runtime support remains unqualified pending native lifecycle tests. |

### Pinned-revision and local-reference distinction

The ignored local `references/the-compute-bazaar-main/LICENSE` contains MIT
license text, but this reference directory has no nested Git metadata and the
file cannot be tied to the pinned revision above. Its SHA-256 is
`d8a406d540569e52a0eb1924f6c8575850ed81c1092bf3dcaba33223b6971377`. The
root license path at the pinned commit returns HTTP 404, reconfirmed on
2026-10-01. On 2026-10-02, a read-only `git ls-remote` check resolved the
public `main` ref to the same pinned SHA, and the raw `main/LICENSE` path also
returned HTTP 404. The current [GitHub repository root](https://github.com/gustofied/the-compute-bazaar)
lists no root license file. Therefore the ignored local file does not change
the recorded rights decision. Bazaar code remains excluded pending written
rights for the exact materials and uses; rights to provider-derived data,
released datasets, benchmarks, and assets require separate source-specific
terms and remain unverified. See
[`hyperai.bazaar.source_rights_local_license_discrepancy.v1`](state-slices.md#hyperai-bazaar-source_rights_local_license_discrepancyv1).

The import-tree verifier and digest manifest are the byte-level check for the
imported directories. Recompute only when an explicitly reviewed source update
changes the imported bytes; do not refresh a manifest merely to make an
unexplained mismatch pass. `corepack pnpm run verify:monorepo` invokes this
check before each independently reported workspace gate.

The bounded Simple Compute Market gate in `scripts/verify-simple-market.mjs`
runs the pinned source's core, registry, and provisioning-IaC suites in a
disposable copy, so uv lockfile rewrites and generated files cannot alter the
tracked import. It builds only the required local wheels. The gate does not
invoke the upstream aggregate packaging target,
hosted release verifier, hosted settlement tests, or the API-credit
cross-language middleware target that invokes npm. Those omitted suites retain
their source-project gates and do not pass by inheritance from the monorepo
check.

## Authority and evidence limits

- HyperIntel remains inference-native and separate from outcome resolution and
  HIP-4. Its local contracts do not establish provider truth, paid execution,
  independent metering, or production authority.
- The HyperAI Inference workspace may verify a caller-selected portable
  HyperIntel market-evidence bundle and reconcile a complete caller-selected
  execution lineage offline through pinned owner contracts. The caller
  supplies the public-key trust registry; the adapter performs no key
  resolution, routing, execution, provider call, or settlement. Lineage may
  contain stream text, so only synthetic or explicitly authorized
  non-sensitive input is allowed by the integration slice; results contain
  identity digests and counts, not execution payloads.
- JustAI's evidence bridge is metadata-only. Its custody package does not
  choose a root or signer. Local-quorum results remain same-owner
  self-verification.
- Simple Compute Market's MIT license permits copying under its terms; it does
  not authorize activating hosted Stripe, Alkahest, seller provisioning, or
  real settlement in HyperAI. The local HyperAI adapter verifies only
  caller-supplied offline listing captures; it does not contact the registry,
  use source credentials, or call sellers.
- HyperIntel and JustAI are staged from the user's local repositories for this
  local monorepo task. Their lack of a root license file means public
  redistribution remains unverified; do not publish these imported sources
  until their rights are confirmed.
- The Compute Bazaar remains reference-only until code and data rights are
  independently established.
