# HyperAI Monorepo Contributor Guide

This repository has one Git root and several independently owned build graphs.
Monorepo membership does not combine product authority, persistence, runtime,
or claim semantics.

## Workspace ownership

- The root Cargo workspace owns HyperAI outcome protocol crates and
  `crates/hyperai-market/`.
- `products/hyperintel/` is the committed HyperIntel snapshot at its recorded
  source revision. Its local `AGENTS.md`, state slices, provenance, and gates
  apply to files in that tree.
- `products/justai/` is the JustAI workspace snapshot. Its charter, status,
  custody, disclosure, and evidence rules remain authoritative for those files.
- `products/simple-compute-market/` is the MIT-licensed upstream workspace.
  Its local `AGENTS.md`, OpenSpec workflow, license, and package-layer rules
  apply to files in that tree.
- `references/the-compute-bazaar-main/` is an ignored local-only checkout while
  code and data rights remain unresolved. Do not add it to a build graph or
  copy its source, schema, fixture, dataset, generated output, or asset into
  tracked HyperAI files.

## Change discipline

- Before editing, name the state slice, exact files, allowed effects, proof
  gates, claim ceiling, and publish boundary in `docs/state-slices.md` or the
  owning product's state-slice record.
- Preserve pre-existing dirty work. Do not copy uncommitted HyperIntel source
  content; import only its recorded committed snapshot unless a later explicit
  source revision is separately approved.
- Keep outcome resolution and HIP-4 records independent from compute-market
  deal and escrow records. Shared modules must contain generic behavior only.
- Keep market data advisory. A provider signature, telemetry record, local
  receipt, asking price, or model output does not prove correct computation or
  payment entitlement.
- Keep the first compute-market path local, synthetic, and mock-only. Provider
  calls, paid execution, federation, custody, external publication, testnet
  writes, and production deployment require separate named slices and their
  respective authority.
- Keep exact upstream revisions, licenses, included files, excluded artifacts,
  and source-tree digests in `docs/provenance.md` and the import manifest.
- Use `pnpm`, never `npm`, for JavaScript workspaces. Preserve each nested
  workspace's own toolchain and lockfile.

## Root validation

The root package scripts are the monorepo entrypoints. A passing subworkspace
gate proves only that subworkspace's declared scope. Report root, HyperIntel,
JustAI, and Simple Compute Market validation separately; do not infer an
independent review or production authorization from local green checks.

## Public release readiness

Explicit public-release readiness is authorized under named state slice
`public-release-readiness-v1`: additive `LICENSE`, `NOTICE`, `SECURITY.md`,
`CONTRIBUTING.md`, `RESEARCH.md`, README navigation, and a CI workflow. It
changes no source, import digests, product behavior, or claim ceilings and
authorizes no provider call, paid execution, fleet action, or live market write.
Every mutation in this phase names state slice `public-release-readiness-v1`.

Explicit public README documentation is authorized under named state slice
`public-release-readme-v2`: documentation-only rewrite of `README.md` (summary of
findings, verification and CI/CD acceptance record, quick start) with the
original documentation preserved. It adds no CI file, source change, evidence,
or claim-ceiling change and authorizes no execution, provider call, spend,
accepted Evidence Ledger mutation, or Level2+ claim. Every mutation in this
phase names state slice `public-release-readme-v2`.
