# Research report and findings

HyperAI is a product-and-protocol monorepo. Its findings are what the hermetic
builds established about buyer-controlled compute markets and a
settlement-backend-neutral outcome protocol, together with the limits recorded
for each. Authoritative detail: `docs/outcomes/acceptance.md`,
`docs/outcomes/limitations.md`, `docs/threat-model.md`, `docs/state-slices.md`.

## Questions

1. Can a computational claim be frozen, observed, verified by pinned validators,
   disputed, finalised, and prepared for settlement as portable, replayable
   evidence, independent of the settlement backend (Hyperliquid HIP-4 is the
   first)?
2. Can a buyer-controlled compute market (offers, buyer constraints, deals,
   fulfilment receipts, mock settlement) be modelled locally and synthetically
   before any provider, payment, or fleet action exists?

## Findings

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

## What this does not establish

No live market has been created or settled; HIP-4 testnet publishing is
`UNSUPPORTED_UNVERIFIED` and mainnet mutation is structurally disabled. The
development role header is not authenticated identity; the local Docker verifier
trusts the host and daemon and is not hostile-code sandbox qualification;
determinism across hosts is not shown; production HA, backup, IAM and hosted
operation are untested. Nothing here is a benchmark result, security guarantee,
financial return, or independent verification.

Local result: `cargo test --workspace` on the pinned Rust 1.93.1 gave 469 passed, 0 failed, 9 ignored (opt-in infrastructure gates).

## Reproduce

```sh
corepack pnpm install --ignore-scripts --frozen-lockfile
corepack pnpm run hsai outcomes demo computational-market --scenario true --artifact-root artifacts/true --json
corepack pnpm run hyperai market demo
corepack pnpm run lint:fast && corepack pnpm run test:focused
```

Requires rustup (the toolchain is pinned in `rust-toolchain.toml`), Corepack and
pnpm. Docker-based verifier gates are opt-in and never pull images.
