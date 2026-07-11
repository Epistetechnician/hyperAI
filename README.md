# HSAI Verifiable Outcome Protocol

This repository is the greenfield implementation of the build contract in
[`HSAI_HIP4_END_TO_END_CODEX_PROMPT.md`](HSAI_HIP4_END_TO_END_CODEX_PROMPT.md).

The product is a settlement-backend-neutral protocol for freezing computational
claims, observing candidate submissions, running pinned validators, exporting
portable evidence, applying disputes and finality, and preparing settlement.
Hyperliquid HIP-4 is the first backend integration.

Current implementation status: workspace bootstrap. No live market has been
created or settled. Mainnet mutation is out of scope and will remain
structurally disabled.

## Repository gates

```sh
pnpm run lint:fast
pnpm run test:focused
pnpm run verify:contracts
pnpm run verify:full
pnpm run lint
```

The complete local demo and trust boundaries will be documented as the
vertical implementation becomes executable.
