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
idempotent mock reconciliation are executable and tested. A local Docker
candidate verifier is tested against a preloaded digest-pinned image, including
claim-bound environment identity, bounded output, timeout, cleanup, and the
complete TRUE-to-mock-settlement path; it is not production sandbox
qualification. Safe materialization from an already-pinned local corpus
checkout, inert special-object inspection, and reviewed role-partitioned
evaluation overlays are tested, but the system does not clone upstream
repositories, retrieve LFS or submodule objects, extract archives, or
materialize images. API liveness is process-only; readiness now fails closed
without fresh fixed-schema dependency evidence. No live market has been created
or settled. Testnet
publishing is `UNSUPPORTED_UNVERIFIED`; mainnet mutation is structurally
disabled.

## Five-minute local demo

Requires the pinned Rust toolchain and pnpm.

```sh
pnpm install --ignore-scripts --frozen-lockfile
pnpm run hsai outcomes demo computational-market --scenario true --artifact-root artifacts/true --json
pnpm run hsai outcomes evidence verify artifacts/true/evidence-bundle.json --blob-root artifacts/true
pnpm run hsai outcomes demo computational-market --scenario invalid-then-valid --artifact-root artifacts/invalid-then-valid --json
pnpm run hsai outcomes demo computational-market --scenario disputed-false-counterexample-true --artifact-root artifacts/disputed-counterexample --json
pnpm run hsai outcomes demo computational-market --scenario incomplete-observation --artifact-root artifacts/incomplete-observation --json
pnpm run hsai outcomes demo computational-market --scenario divergent-replays --artifact-root artifacts/divergent-replays --json
pnpm run hsai outcomes demo computational-market --scenario false --artifact-root artifacts/false --json
pnpm run hsai outcomes demo computational-market --scenario runner-error --artifact-root artifacts/error --json
pnpm run hsai outcomes settlement preflight --binding artifacts/true/market-binding.json --resolution artifacts/true/final-resolution.json --metadata fixtures/hip4/outcome-meta.json --capabilities artifacts/true/hip4-capabilities.json
pnpm run hsai outcomes settlement publish artifacts/true/settlement-intent.json --dry-run
pnpm run hsai outcomes settlement reconcile artifacts/true/mock-reconciliation.json
pnpm run hsai outcomes corpus profiles
```

The TRUE, invalid-then-valid, and disputed-counterexample demos each perform one
idempotent effect against the in-memory mock only. Invalid-then-valid proves
`[FAIL, PASS, PASS]`: the first candidate remains unresolved and non-qualifying,
while the later candidate reaches FINAL TRUE. The disputed-counterexample demo
retains a refused provisional FALSE, then uses a pre-cutoff candidate, immutable
decision, superseding quorum, and explicit edge to reach FINAL TRUE. The FALSE
demo finalizes only after complete deadline/grace observation and does not
automatically publish. The runner-error demo remains `UNRESOLVED/NONE`.
Incomplete-observation and divergent-replays remain `UNRESOLVED/NONE`, export
verified evidence plus typed settlement-ineligibility artifacts, and create no
resolution or external effect.

## Repository gates

```sh
pnpm run lint:fast
pnpm run test:focused
pnpm run verify:contracts
pnpm run verify:full
pnpm run lint
```

The opt-in local infrastructure gates require explicitly selected local
dependencies. For Docker, preload the image and export
`HSAI_TEST_OCI_IMAGE_REPOSITORY`, `HSAI_TEST_OCI_IMAGE_DIGEST`, and the explicit
`HSAI_TEST_OCI_PLATFORM` (`linux/arm64` or `linux/amd64`), then run `pnpm run
verify:infra-oci`. The gate never pulls an image.

## Controlled local beta

The trusted-operator, single-host beta is hermetic and mock-only. Use a fresh
run ID; the command refuses to overwrite an existing session:

```sh
pnpm run beta:local -- --run-id beta-001
```

The rehearsal runs all seven local scenarios serially, reload-verifies every
evidence bundle, requires the expected tamper rejection, checks exact effect and
finality ceilings, and writes `beta-run-manifest.json`. It performs no live
market write and does not qualify hosted operation or arbitrary hostile code.
Follow [the local beta runbook](docs/outcomes/local-beta-runbook.md).

See [architecture](docs/outcomes/architecture.md), [acceptance status](docs/outcomes/acceptance.md),
[HIP-4 status](docs/outcomes/hyperliquid-hip4.md), [limitations](docs/outcomes/limitations.md),
and the [threat model](docs/threat-model.md).
