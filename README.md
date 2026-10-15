<!-- readme-v2:begin -->
# HyperAI Monorepo

[![CI](https://github.com/Epistetechnician/hyperAI/actions/workflows/ci.yml/badge.svg)](https://github.com/Epistetechnician/hyperAI/actions/workflows/ci.yml) [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE) ![visibility: private staging](https://img.shields.io/badge/release-private%20staging-lightgrey) ![claims: bounded](https://img.shields.io/badge/claims-bounded%20local-orange)

> HyperAI is a modular monorepo for buyer-controlled compute markets, inference infrastructure, outcome verification, and governed evidence. Modules share a Git root and operator entrypoints while retaining their own contracts, workspaces, custody rules, runtime boundaries, and external authority.

## Contents

- [At a glance](#at-a-glance)
- [Key findings](#key-findings)
- [Verification and CI/CD acceptance](#verification-and-cicd-acceptance)
- [Quick start](#quick-start)
- [Detailed documentation](#detailed-documentation)
- [Governance, claims, and non-claims](#governance-claims-and-non-claims)
- [Security, contributing, license](#security-contributing-license)

## At a glance

| | |
|---|---|
| Stack | Rust 1.93.1 (pinned) + Node tooling |
| Release stage | Private staging for research release; not yet public |
| License | Apache-2.0 (see [LICENSE](LICENSE), [NOTICE](NOTICE)) |
| Research report | [RESEARCH.md](RESEARCH.md) |
| Last full acceptance run | 2026-10-14 |
| Evidence ceiling | Bounded local replay. Nothing here is independent, benchmark, or production evidence. |

## Key findings

This is a summary of [RESEARCH.md](RESEARCH.md). Every result is a bounded local or synthetic observation, and negative results are retained rather than retuned away.

### Questions

1. Can a computational claim be frozen, observed, verified by pinned validators,
   disputed, finalised, and prepared for settlement as portable, replayable
   evidence, independent of the settlement backend (Hyperliquid HIP-4 is the
   first)?
2. Can a buyer-controlled compute market (offers, buyer constraints, deals,
   fulfilment receipts, mock settlement) be modelled locally and synthetically
   before any provider, payment, or fleet action exists?

### Findings

**Outcome protocol (hermetic vertical slice).** Of 16 end-to-end acceptance
scenarios, 9 are `COMPOSED_TESTED` (one automated path across all boundaries),
including: valid solution; invalid-then-valid candidate; no solution (FINAL
FALSE with no publication); incomplete observation (stays UNRESOLVED, no
effect); nondeterministic validator (UNRESOLVED with `ReplayDivergence`);
dispute with a counterexample (provisional FALSE refused, later FINAL TRUE with
one idempotent mock effect); tampered evidence (typed rejection); settlement
crash recovery (receipt recovered without resubmission); and unauthorized
operation (fixed denial before parsing). Five are `CONTRACT_TESTED`, one is
`PARTIAL_COMPOSED` (live Docker timeout/flood classification is runner-level
only), and one (backward compatibility) is `TESTED` by the workspace gates. The two named remaining gaps
are live Docker classification and composed stale-binding refusal.

**Design findings.** Fail-closed defaults carried the design: incomplete
observation, replay divergence, and runner error all resolve to UNRESOLVED with
no settlement artifact; evidence stays append-only so a corrected outcome
supersedes rather than rewrites; and settlement is idempotent with the intent
persisted before submission, so a lost receipt is recovered by authoritative
status lookup instead of resubmission. Binding the Docker image, platform, argv,
environment, mounts and resource controls into the frozen claim removes
operator-selected environment drift before the verifier runs.

**Compute market and interface.** Offers, buyer constraints, deals, fulfilment
receipts and mock settlement run end to end on synthetic data across CLI, TUI
and browser against one local service. Fleet, Eval, Trade and Data surfaces are
digest-bound, advisory and synthetic: Fleet previews are `not_authorized`,
simulator state is volatile, Eval compares two fixed answer fixtures on six
synthetic cases (no model is run), and Trade summaries are in-memory over a fixed
series and are not forecasts, orders or returns.

### What this does not establish

No live market has been created or settled; HIP-4 testnet publishing is
`UNSUPPORTED_UNVERIFIED` and mainnet mutation is structurally disabled. The
development role header is not authenticated identity; the local Docker verifier
trusts the host and daemon and is not hostile-code sandbox qualification;
determinism across hosts is not shown; production HA, backup, IAM and hosted
operation are untested. Nothing here is a benchmark result, security guarantee,
financial return, or independent verification.

Local result: `cargo test --workspace` on the pinned Rust 1.93.1 gave 469 passed, 0 failed, 9 ignored (opt-in infrastructure gates).

## Verification and CI/CD acceptance

GitHub Actions workflow `ci.yml` runs on every push and pull request with read-only repository permissions (`contents: read`).

| Job | Runner | Gate |
|---|---|---|
| test | ubuntu-latest | `cargo fmt --all -- --check` |
|  |  | `cargo test --workspace --locked` |

**Gate intent:** Format and locked workspace tests under the pinned toolchain.

**Local acceptance (2026-10-14):** fmt clean; 469 tests passed, 0 failed, 9 ignored.

Run the same gate locally:

```sh
sh scripts/with-pinned-rust.sh cargo fmt --all -- --check
sh scripts/with-pinned-rust.sh cargo test --workspace --locked
```

### Acceptance criteria

- The gate above exits 0 on a clean checkout of the default branch.
- No claim in this repository exceeds the evidence ceiling stated in [At a glance](#at-a-glance).
- A failing, skipped, or ignored check is reported as such; it is never reclassified as a pass.
- Commits to this repository are authored by the maintainer only.

## Quick start

```sh
git clone https://github.com/Epistetechnician/hyperAI.git
cd hyperAI
sh scripts/with-pinned-rust.sh cargo fmt --all -- --check
sh scripts/with-pinned-rust.sh cargo test --workspace --locked
```

## Detailed documentation

The sections below are the repository's original design and implementation documentation, preserved in full.

<!-- readme-v2:detail -->

HyperAI is a modular monorepo for buyer-controlled compute markets, inference
infrastructure, outcome verification, and governed evidence. Modules share a
Git root and operator entrypoints while retaining their own contracts,
workspaces, custody rules, runtime boundaries, and external authority.

## Product map

- `crates/outcomes-*` and `crates/hyperliquid-hip4/` implement the existing
  outcome verification and settlement protocol.
- `crates/hyperai-market/` owns compute offers, buyer constraints, deal
  records, fulfillment receipts, and mock settlement. The first implementation
  is local and synthetic.
- `products/simple-compute-market/` preserves the MIT-licensed buyer, registry,
  seller storefront, negotiation, and seller-owned fulfillment implementation
  as an independently built Python workspace.
- `products/hyperintel/` preserves the pinned inference-native offer, routing,
  execution receipt, replay, and reconciliation workspace.
- `products/justai/` preserves the JustAI governance, evidence, disclosure, and
  custody packages with its existing authority limits.
- `references/the-compute-bazaar-main/` is an ignored local-only reference
  until its code and data reuse rights are established. HyperAI-owned market
  observations and analysis do not depend on it.

`corepack pnpm run verify:monorepo` runs the import digest check and each
workspace's validation gates separately. Read [import provenance](docs/provenance.md),
[state slices](docs/state-slices.md), and the [compute market plan](docs/compute-market-refactor-plan.md)
before changing source ownership or enabling external behavior.

## HyperAI interface

Initialize a local home, start the shared service, inspect the connection, and
open the terminal desk with the primary `hyperai` command:

```sh
corepack pnpm run hyperai init
corepack pnpm run hyperai service start
corepack pnpm run hyperai doctor
corepack pnpm run hyperai desk
corepack pnpm run hyperai service stop
```

Use `--home PATH` to select a disposable or non-default home. The service binds
to IPv4 loopback on port `4317` by default. `service status` and `connect`
probe the recorded instance over loopback; a runtime record by itself does not
count as readiness. The local browser shell and TUI connect to this same
service. Browser sessions use an HttpOnly cookie, same-origin checks, and an
in-memory CSRF token; CLI/TUI access reads the owner-only bearer file from the
runtime directory. These local capabilities do not authorize provider calls,
paid execution, remote fleet actions, or financial operations.

Offline metadata recovery is limited to the schema-v1 local metadata table.
Run these commands:

    corepack pnpm run hyperai storage backup --home <path>
    corepack pnpm run hyperai storage restore --home <path> --backup-name <basename>

Backup can run while the service is active and creates a no-overwrite snapshot.
Restore requires the service to be stopped, accepts a generated backup from
the same home, and first creates a safety snapshot. These commands do not
export deal, outcome, provider, or nested-product records.

For a disposable source-built bundle containing `hyperai`, `hsai`,
`hyperai-ui`, and the sibling `hyperai-mcp` ACP helper, run:

```sh
corepack pnpm run install:local -- --root "$HOME/.local/opt/hyperai-dev" --offline
```

The root is required and must be a dedicated install directory. Add its `bin`
directory to `PATH` to use the installed commands. The install test connects
only HyperAI's synthetic ACP peer; the fixture peer is not included in the
bundle. This is a local development install path, not a signed installer,
cross-platform qualification, or public release. See the [Phase 1 packet](docs/compute-bazaar-phase-1-contract-packet.md)
and [Phase 8 ACP packet](docs/compute-bazaar-phase-8-acp-packet.md) for its
proof scope and remaining gates.

To run the existing browser shell in the foreground, use the compatibility
launcher:

```sh
corepack pnpm run interface:local
```

Open the loopback URL printed by the process (default port `4317`). The Deals
surface stages a synthetic quote preview, explicit approval, and a separate
mock-fulfillment step; approval uses a deterministic fixture key and simulated
clock, not the user's identity or purchasing authority. The Overview also
keeps a clearly labeled full synthetic lifecycle walkthrough. The interface
can verify caller-selected HyperIntel evidence bundles, reconcile a complete
caller-selected HyperIntel execution lineage under public keys the caller
supplies, and verify Simple Compute Market signed listing captures offline.
Reconciliation accepts only synthetic or otherwise explicitly authorized
non-sensitive data, including possible execution-stream text, and returns
identity digests only. These checks do not contact a registry or provider, call
a seller, or promote imported listings into deal selection. Fleet exposes a
fixed synthetic inventory, fixture-only doctor checks, digest-bound advisory
previews, and a volatile lifecycle simulator. The simulator binds state
digests and retry keys, demonstrates timeout reconciliation and partial
cleanup, and creates virtual leases and task IDs only. It does not probe a host
or execute an operation. The ACP session path is present for local
synthetic-peer qualification; this does not qualify a third-party agent or
authorize model/provider calls or real-data disclosure. Live fleet actions,
evidence publication, real settlement, and financial Trade remain unavailable.
See the [Bazaar-to-HyperAI interface plan](docs/bazaar-hyperai-interface-plan.md),
the [Rust compute desk subplan](docs/compute-bazaar-rust-port-subplan.md), and
the [Phase 1 contract packet](docs/compute-bazaar-phase-1-contract-packet.md).
ACP scope and remaining proof gates are listed in the
[Phase 8 ACP packet](docs/compute-bazaar-phase-8-acp-packet.md).
The accepted Phase 2 slice adds a separate synthetic-only local catalog and
bounded query workflow; its exact boundaries are in the
[Phase 2 Data contract packet](docs/compute-bazaar-phase-2-data-packet.md).
The Phase 3A local slice derives an in-memory Bronze/Silver/Gold view from that
HyperAI-owned synthetic snapshot; see the
[Phase 3A packet](docs/compute-bazaar-phase-3-synthetic-transform-packet.md).
It adds no Bazaar adapter or external data.
The [Phase 4 Fleet packet](docs/compute-bazaar-phase-4-fleet-readonly-packet.md)
records the synthetic inventory and preview-only operation surface. The
[Phase 5 Fleet simulation packet](docs/compute-bazaar-phase-5-fleet-simulation-packet.md)
records the service-owned simulator contract and its recovery limits.

With the local service running, inspect and query the owned synthetic sample
through the shared authenticated API:

```sh
corepack pnpm run hyperai data seed-synthetic
corepack pnpm run hyperai data tables
corepack pnpm run hyperai data describe hyperai.synthetic_market_observations_json.v1
corepack pnpm run hyperai data sql hyperai.synthetic_market_observations_json.v1 --query 'SELECT gpu_model, available_count FROM dataset ORDER BY gpu_model' --format csv
corepack pnpm run hyperai data model list
corepack pnpm run hyperai data view list
corepack pnpm run hyperai fleet --home <path> inventory
corepack pnpm run hyperai fleet --home <path> doctor
corepack pnpm run hyperai fleet --home <path> preview --node-id fixture-node-atlas --operation launch
corepack pnpm run hyperai fleet --home <path> simulation show
corepack pnpm run hyperai fleet --home <path> simulation step --operation attach --expected-state-digest <digest> --idempotency-key attach-once
```

The CLI, TUI, and browser read the same synthetic Fleet inventory and
fixture-only doctor. Their operation preview is digest-bound and advisory;
`not_authorized` means it cannot invoke or reserve a resource. Previewing does
not change the simulator. The separate CLI and TUI simulation controls require
an expected state digest and an idempotency key for each explicit step.
Repeating the same request returns its original transition; a changed request
or stale digest is rejected. Simulator state lives only in the local service
process and resets when that service exits. Virtual task IDs are not process
IDs, and virtual leases are not resource reservations.

The Eval workspace compares two fixed HyperAI-authored answer fixtures against
the same six synthetic cases. It does not run an agent or model. Inspect the
catalog and submit a digest-bound local harness run through the shared service:

```sh
corepack pnpm run hyperai eval --home <path> catalog
corepack pnpm run hyperai eval --home <path> run --expected-suite-digest <digest> --idempotency-key eval-once
```

Reports include three sequential scorer-process times and serialized protocol
sizes. They remain volatile in the service process and cannot authorize
routing, promotion, procurement, publication, settlement, or production use.
The [Phase 6 Eval packet](docs/compute-bazaar-phase-6-eval-packet.md) records
the scorer boundary and claim limits.

Trade is a local research surface over a fixed HyperAI-authored hourly series.
The CLI, TUI, and browser call the same bounded service methods; results are
in-memory summaries and are not market feeds, forecasts, orders, fills, or
financial returns:

```sh
corepack pnpm run hyperai trade --home <path> instruments
corepack pnpm run hyperai trade --home <path> series --instrument-id a100-us-east
corepack pnpm run hyperai trade --home <path> scenario --primary-instrument-id a100-us-east --comparison-instrument-id a100-us-west --price-shock-bps 500
corepack pnpm run hyperai trade --home <path> paper --baseline-instrument-id a100-us-west --requested-units 1
```

The [Phase 7 Trade packet](docs/compute-bazaar-phase-7-trade-packet.md)
records its source digest, methods, local-only proof gates, and nonclaims.

The Data workspace keeps the local catalog beside the separately verified SCM
capture explorer and shows the synthetic Bronze/Silver/Gold transform beside
its input and output digests. Query results are bounded and session-only;
saved models and views contain definitions and digests, not row payloads. The
TUI lets the user edit a bounded SQL query (`e`), run the saved query
(`Enter`), clear it while editing (`Ctrl+U`), and cancel editing (`Esc`). It
uses the authenticated local Data route and does not persist query text or
results. These Data slices add no Bazaar ingestion, network source, seller
call, provider execution, fleet operation, or settlement.

## Outcome protocol

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

Requires rustup, Corepack, and pnpm. Corepack resolves the exact `packageManager`
pin from `package.json`; use `corepack pnpm` if a global pnpm launcher is stale or
conflicts with an unrelated parent project. Every pnpm Rust entrypoint
uses `scripts/with-pinned-rust.sh`: it resolves the channel from
`rust-toolchain.toml`, prepends that installed toolchain's binary directory to
`PATH`, and exports the same selection to child processes. A Homebrew or system
Rust earlier in the operator's original `PATH` therefore cannot shadow the pin.

```sh
corepack pnpm install --ignore-scripts --frozen-lockfile
corepack pnpm run hsai outcomes demo computational-market --scenario true --artifact-root artifacts/true --json
corepack pnpm run hyperai market demo
corepack pnpm run hyperai market demo --output-dir artifacts/market-run
corepack pnpm run hyperai market verify --receipt artifacts/market-run/market-receipt.json
corepack pnpm run hyperai market recover --journal artifacts/market-run/market-journal.json --output artifacts/recovered-market-receipt.json
corepack pnpm run hsai outcomes evidence verify artifacts/true/evidence-bundle.json --blob-root artifacts/true
corepack pnpm run hsai outcomes demo computational-market --scenario invalid-then-valid --artifact-root artifacts/invalid-then-valid --json
corepack pnpm run hsai outcomes demo computational-market --scenario disputed-false-counterexample-true --artifact-root artifacts/disputed-counterexample --json
corepack pnpm run hsai outcomes demo computational-market --scenario incomplete-observation --artifact-root artifacts/incomplete-observation --json
corepack pnpm run hsai outcomes demo computational-market --scenario divergent-replays --artifact-root artifacts/divergent-replays --json
corepack pnpm run hsai outcomes demo computational-market --scenario false --artifact-root artifacts/false --json
corepack pnpm run hsai outcomes demo computational-market --scenario runner-error --artifact-root artifacts/error --json
corepack pnpm run hsai outcomes settlement preflight --binding artifacts/true/market-binding.json --resolution artifacts/true/final-resolution.json --metadata fixtures/hip4/outcome-meta.json --capabilities artifacts/true/hip4-capabilities.json
corepack pnpm run hsai outcomes settlement publish artifacts/true/settlement-intent.json --dry-run
corepack pnpm run hsai outcomes settlement reconcile artifacts/true/mock-reconciliation.json
corepack pnpm run hsai outcomes corpus profiles
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
corepack pnpm run lint:fast
corepack pnpm run test:focused
corepack pnpm run verify:contracts
corepack pnpm run verify:full
corepack pnpm run lint
```

`test:focused` covers the HyperAI foundation, compute-market, Data, Fleet,
Eval, Trade, application, local service, TUI, ACP/MCP, and offline adapter
crates alongside the focused outcomes packages. To keep Cargo offline for
this run, use `CARGO_NET_OFFLINE=true corepack pnpm run test:focused`.
`verify:full` remains the all-workspace, all-features gate.

The opt-in local infrastructure gates require explicitly selected local
dependencies. For Docker, preload the image and export
`HSAI_TEST_OCI_IMAGE_REPOSITORY`, `HSAI_TEST_OCI_IMAGE_DIGEST`, and the explicit
`HSAI_TEST_OCI_PLATFORM` (`linux/arm64` or `linux/amd64`), then run `corepack pnpm run
verify:infra-oci`. The gate never pulls an image.

## Controlled local beta

The trusted-operator, single-host beta is hermetic and mock-only. Use a fresh
run ID; the command refuses to overwrite an existing session:

```sh
corepack pnpm run beta:local --run-id beta-001
```

The rehearsal requires a clean Git tree, records bounded source/tool/input
provenance, runs all seven local scenarios serially, reload-verifies every
evidence bundle, requires the expected tamper rejection, checks exact effect and
finality ceilings, and exercises lost-receipt recovery across an application
restart using file-backed SQLite and the retained mock authority. It also sends
real unauthorized requests to every protected local API route and verifies
sanitized durable denial audits with zero protected effects. It writes manifest
v4 with raw report and database digests for both operational checks. Before
returning PASS it creates and verifies an exact-file
`beta-artifact-inventory.json` seal. Verify a retained run in a fresh process
with `corepack pnpm run beta:verify --run-id beta-005`.
It performs no live market write and does not qualify hosted operation,
production authentication or IAM, arbitrary hostile code, backend-process or
host-crash durability, or signed build provenance.
Follow [the local beta runbook](docs/outcomes/local-beta-runbook.md).

See [outcome architecture](docs/outcomes/architecture.md), [compute-market architecture](docs/compute-market-architecture.md), [acceptance status](docs/outcomes/acceptance.md),
[HIP-4 status](docs/outcomes/hyperliquid-hip4.md), [limitations](docs/outcomes/limitations.md),
and the [threat model](docs/threat-model.md). The full migration sequence is in
the [HyperAI Compute Market plan](docs/compute-market-refactor-plan.md).

## Research report and project policy

See [RESEARCH.md](RESEARCH.md) for findings, [SECURITY.md](SECURITY.md) to report
issues, and [CONTRIBUTING.md](CONTRIBUTING.md). The repository is licensed under
[Apache-2.0](LICENSE); `products/simple-compute-market` keeps its MIT license
(see [NOTICE](NOTICE)).

---
<!-- readme-v2:footer -->
## Governance, claims, and non-claims

Work proceeds under named state slices recorded in [AGENTS.md](AGENTS.md). The claim ceiling stated above is a hard limit: a local pass is not independent verification, a benchmark result, a proof of the whole system, or a statement about production readiness. See "What this does not establish" in [RESEARCH.md](RESEARCH.md).

## Security, contributing, license

- Report vulnerabilities per [SECURITY.md](SECURITY.md).
- Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.
- Licensed under [Apache-2.0](LICENSE); see [NOTICE](NOTICE).
