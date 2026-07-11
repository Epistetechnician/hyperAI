# HSAI Verifiable Outcome Protocol

This repository is the greenfield implementation of the build contract in
[`HSAI_HIP4_END_TO_END_CODEX_PROMPT.md`](HSAI_HIP4_END_TO_END_CODEX_PROMPT.md).

The product is a settlement-backend-neutral protocol for freezing computational
claims, observing candidate submissions, running pinned validators, exporting
portable evidence, applying disputes and finality, and preparing settlement.
Hyperliquid HIP-4 is the first backend integration.

Current implementation status: hermetic vertical slice. Frozen claims,
cursor-aware local observation, structured verification, evidence export,
attestor quorum, finality, explicit HIP-4 binding, deterministic intents, and
idempotent mock reconciliation are executable and tested. Safe materialization
from an already-pinned local corpus checkout is tested, but the system does not
clone upstream repositories or resolve LFS/submodules/archives/images. No live
market has been created or settled. Testnet
publishing is `UNSUPPORTED_UNVERIFIED`; mainnet mutation is structurally
disabled.

## Five-minute local demo

Requires the pinned Rust toolchain and pnpm.

```sh
pnpm install --ignore-scripts --frozen-lockfile
pnpm run hsai outcomes demo computational-market --scenario true --artifact-root artifacts/true --json
pnpm run hsai outcomes evidence verify artifacts/true/evidence-bundle.json --blob-root artifacts/true
pnpm run hsai outcomes demo computational-market --scenario false --artifact-root artifacts/false --json
pnpm run hsai outcomes demo computational-market --scenario runner-error --artifact-root artifacts/error --json
pnpm run hsai outcomes settlement preflight --binding artifacts/true/market-binding.json --resolution artifacts/true/final-resolution.json --metadata fixtures/hip4/outcome-meta.json --capabilities artifacts/true/hip4-capabilities.json
pnpm run hsai outcomes settlement publish artifacts/true/settlement-intent.json --dry-run
pnpm run hsai outcomes settlement reconcile artifacts/true/mock-reconciliation.json
```

The TRUE demo performs one idempotent effect against the in-memory mock only.
The FALSE demo finalizes only after complete deadline/grace observation and does
not automatically publish. The runner-error demo remains `UNRESOLVED/NONE`.

## Repository gates

```sh
pnpm run lint:fast
pnpm run test:focused
pnpm run verify:contracts
pnpm run verify:full
pnpm run lint
```

See [architecture](docs/outcomes/architecture.md), [HIP-4 status](docs/outcomes/hyperliquid-hip4.md),
[limitations](docs/outcomes/limitations.md), and the [threat model](docs/threat-model.md).
