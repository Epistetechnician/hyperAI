# Phase 6 Local Eval Harness Qualification Packet

State slice: `hyperai.bazaar.eval_local.v1`.
Parent plan: [HyperAI Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md).
Predecessor: [Phase 5 Fleet simulation](compute-bazaar-phase-5-fleet-simulation-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Objective and bounded task list

1. Define six HyperAI-authored synthetic evaluation cases and a fixed paired
   fixture comparison.
2. Implement a separately launched scorer process, bounded authenticated API,
   CLI and TUI views, and a clear browser Eval workspace.
3. Validate scorer isolation boundaries, deterministic reports, resource
   limits, and all client paths; record local evidence and nonclaims.

This phase qualifies the local report and scoring plumbing only. It does not
run an agent or model. The two compared records are pre-authored synthetic
answer fixtures, not candidates, executions, or evidence about a model.

## Baseline and state boundary

Baseline revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty monorepo and Phases 1–5 preserved. No commit or frozen
worktree digest is claimed. No Bazaar benchmark, answer key, corpus, prompt,
schema, code, or asset is reused.

Included files:

- `Cargo.toml`, `Cargo.lock`, and new `crates/hyperai-eval/`: owned case
  catalog, fixed answer fixtures, scorer protocol, paired report types, and
  contract tests.
- `crates/hyperai-application/Cargo.toml` and `src/lib.rs`: typed Eval facade.
- `crates/hyperai-local-service/Cargo.toml` and `src/server.rs`: scorer
  subprocess, request limits, and authenticated routes.
- `crates/outcomes-cli/Cargo.toml`, `crates/outcomes-cli/src/lib.rs`,
  `src/main.rs`, `src/bin/hsai.rs`, `src/bin/hyperai-ui.rs`,
  `crates/outcomes-cli/tests/cli_compat.rs`, and new
  `crates/outcomes-cli/tests/eval_api.rs`: `hyperai eval catalog` and
  `hyperai eval run` over the shared service, including process-boundary
  contract tests.
- `crates/hyperai-tui/src/lib.rs` and
  `crates/hyperai-tui/tests/terminal_restore.rs`: view the fixed catalog and
  run the named harness qualification fixture from Eval.
- `apps/hyperai-interface/app.js` and `apps/hyperai-interface/app.css`: the
  Eval catalog, explicit run action, paired report, cost/resource bounds, and
  claim limits.
- `README.md`, this packet, `docs/state-slices.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`: usage and validation record.

No changes to `products/`, `references/`, HyperIntel, JustAI, Simple Compute
Market, data adapters, outcome-resolution semantics, routing, or settlement
are included.

## Frozen measurement design

- Metric: exact agreement between the scorer's six case labels and the
  fixture contract's six expected labels, direction higher, target `6/6` for
  each fixed fixture. The smallest meaningful difference is one case (`1/6`);
  it is only a harness-correctness unit, not a statistical effect size.
- Cases: deal constraints, source attribution, market-data limits, risk
  reasoning, financing boundaries, and operational safety. Each task and
  answer is HyperAI-authored, deterministic, synthetic, and versioned.
- Pairing: `synthetic-pair-v1` binds both fixture identities to the same six
  case IDs, scorer version, suite digest, and run request. The two fixed
  responses are scored side by side; this does not estimate a causal model
  improvement.
- Baseline and control: the immutable `control-fixture-v1` and
  `alternate-fixture-v1` records, compared under the same scorer and cases.
  Neither record represents a model or agent.
- Repeats: three sequential scorer-process invocations per request, all using
  fixed inputs and seed label `synthetic-seed-v1`. Every repeat must be byte-
  identical in scored content. Stop on the first mismatch, timeout, malformed
  response, budget violation, or child-process failure.
- Variance: no sampling, temperature, provider, adaptive selection, or
  concurrency is used. Remaining measured variation may come from process
  startup and host scheduling; wall time is descriptive and is not an
  effectiveness metric.
- Resource bounds: six cases, two fixtures, three repeats; each scorer child
  receives at most 16 KiB and must return at most 32 KiB within one second.
  The complete API request is at most 4 KiB, one run is active per service,
  and the volatile idempotency cache retains at most 32 run keys without
  eviction. The API reports child wall time and serialized byte counts.
- Full cost boundary: no provider tokens, paid service, model runtime, remote
  worker, or external compute. Record scorer process time and serialized
  input/output bytes. Energy and human-review cost are unmeasured, not zero.
  No candidate-generation or training cost is included because none occurs.

## Scorer and data boundary

The browser, CLI, and TUI send only the fixed suite/pair IDs, expected suite
digest, and an idempotency key. They cannot submit prompts, arbitrary task
bytes, answers, executable paths, tool settings, or candidate code. The API
starts only the current service executable (`hyperai`, `hsai`, or the
compatibility `hyperai-ui` launcher) in its hidden `__internal_eval_scorer`
mode with one fixed argument, no shell, a clean
bounded JSON stdin/stdout protocol, and a one-second per-child deadline. The
child receives synthetic response codes and returns case labels plus
aggregates; the API does not return the answer fixture or scorer's
expected-label table. Timeout kills and reaps only this scorer child.

The child process is a software separation boundary, not an operating-system
sandbox or a defense against the local machine owner, source reader, debugger,
or administrator. The cases are public synthetic fixtures, not a protected
holdout. No candidate process exists in this slice. Any model, agent, prompt,
tool, memory, live scorer key, sealed holdout, or benchmark with restricted
rights requires a fresh packet and separate custodian authorization.

Reports remain in the local service's volatile memory and are returned only
to the authenticated caller. State resets when the service exits. No result is
eligible for routing, promotion, publication, procurement, settlement, or
production decisions.

## Allowed effects and hard limits

Allowed effects are reading the compiled-in HyperAI synthetic catalog,
starting the fixed scorer child, retaining at most 32 digest-bound reports in
one service process, and rendering the same report through the API, CLI, TUI,
and browser. Tests may use temporary local homes and the bundled scorer.

This slice does not invoke models, agents, providers, external evaluators,
training, external network services, remote workers, local arbitrary
executables, or user-supplied tasks; read holdouts or private data; persist benchmark outputs; change
HyperIntel routing; execute compute jobs; or move funds. It does not claim
benchmark validity, candidate quality, generalization, fairness, safety,
statistical significance, usefulness per cost, or independent verification.

## Proof gates

- The six case IDs, categories, suite digest, pair identity, and fixed
  synthetic fixtures are deterministic and reject unknown fields or IDs.
- The three sequential scorer outputs agree byte-for-byte on case labels and
  aggregates. Missing, unexpected, duplicate, or malformed labels fail
  closed and remain explicit; they never become zero-cost success.
- The server returns aggregates and provenance only, never fixture answer
  codes or the scorer's expected-label table. It launches no caller-selected
  executable and uses no shell. Replay, changed-key conflict, stale digest,
  and 32-key capacity paths are covered. Fault-injected service-path tests
  cover timeout, oversized child output, and nonzero child exit; timeout and
  output overflow kill and reap the child and return fixed error codes.
- Eval routes require local authentication and existing CSRF protections.
  Malformed and unknown fields fail closed. The process boundary and lack of
  sandboxing are visible in every report.
- Browser, CLI, and TUI use the same catalog, run request, and report schema.
  Each makes clear that the result compares fixed fixtures, not agents.
- Focused Eval/service/CLI/TUI tests, root lint, JavaScript syntax, browser
  desktop/820px/390px checks, and diff hygiene pass. A bounded property-based
  input run passed 128 generated byte strings of lengths 0–16,416 with fixed
  seed `20260924`; accepted outputs remained valid and within the 32 KiB cap.
- A bounded adversarial review covered malformed and oversized requests,
  stale suite/pair/seed values, reordered and inconsistent child results,
  executable/environment/shell selection, child timeout/output/exit failures,
  idempotency-key reuse, cache saturation, and unintended external effects.
  No issue was found within this slice. The review was performed by the author
  against the local implementation and tests; it is not independent review.

## Assurance report

Exact source revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the
current uncommitted working tree; no frozen diff digest is claimed. State
slice: `hyperai.bazaar.eval_local.v1`. Assurance level: `INTERNAL_PASS` for the
local synthetic contract and bounded checks in this packet. Reviewer status:
`NOT_REQUESTED`; no independent decision is claimed. Reproducibility status:
`REPRODUCED_LOCAL` for the exact Eval-related Rust package suite recorded in
[`hyperai.bazaar.eval_trade_rust_reproduction.v1`](state-slices.md#hyperai-bazaar-eval_trade_rust_reproductionv1).
That same-owner Docker reproduction covers Rust tests only. Full phase exit
remains open for a separate-host reproduction of the packet's root, JavaScript,
browser, and responsive-view checks.
Local evidence: focused Eval/service/CLI/TUI tests passed, including fault
injection; `corepack pnpm run lint` passed; `node --check` and `git diff
--check` passed; the browser showed the same synthetic catalog and report at
the default, 820px, and 390px widths; the 128-case property run passed with
fixed seed `20260924`; and the bounded adversarial review described above
found no issue within scope. Data and artifact custody: HyperAI-authored
synthetic case and answer fixtures; test reports remained in volatile local
memory and the disposable service home was removed. Known limitations: the
child is not an OS sandbox; no model, agent, holdout, external reviewer, or
second host was used. The claim ceiling is scorer and report plumbing for the
exact fixed fixture suite. The next phase-exit gate is clean second-host
reproduction with recorded toolchain, dependencies, fixtures, and outputs.
Any model/agent run or operational use of scores needs a fresh packet and
named evaluator, data, and holdout custodians.

## Local fault-fixture follow-up (2026-09-25)

The injected scorer child now drains its piped synthetic request before
triggering timeout, overflow, or nonzero-exit behavior. This removes a test
race where the child could exit before the parent finished writing stdin. The
four focused fault tests passed on the pinned macOS toolchain, and the fixed
source passed the 61-test Linux x86_64 container package run recorded in the
[Phase 1 state-slice entry](state-slices.md). That container used Debian 12
under Apple Silicon Rosetta emulation, so it is not a clean native Ubuntu
reproduction. The same fixed-source package suites also passed as
`x86_64-apple-darwin` binaries under Rosetta on macOS, with native Intel-host
reproduction still open. This follow-up changes test setup only; the scorer
runtime contract is unchanged. The `hyperai.bazaar.eval_local.v1` claim ceiling
and `NOT_REPRODUCED` phase-exit status remain in force.
