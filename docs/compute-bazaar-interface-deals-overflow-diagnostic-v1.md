# Deals Viewport Overflow Diagnostic

Status: completed one-run local diagnostic under the user's direction to
continue the HyperAI interface and Compute Bazaar refactor. The failure was
reproduced, but the cause remains inconclusive because no ordinary element
box crossed the viewport edge. This slice authorizes no layout fix or second
browser run.

State slice: `hyperai.bazaar.interface.deals_overflow_diagnostic.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Related feature: [supply-to-quote packet](compute-bazaar-supply-to-quote-packet.md).
Prior result: the supply-to-quote browser run stopped at the
`zoom-200-equivalent/deals` 720-by-500 viewport check before reaching its
feature workflow.

## Baseline and frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. The NUL-delimited
`git status --porcelain=v1` digest is
`ee4eefbc22e0399f77192aba094cdd9a6d0359251df0fed830c648ebe546a109`.

Recheck these source hashes before editing; any mismatch requires a fresh
packet.

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `f2cc46bbe039973024c953904a0febf90277d311f5fa25f6f342d4d72f6ed008` |
| `apps/hyperai-interface/app.css` (read-only) | `064c438a3a241ab369257e14b6f1458b47ff6adfeae738f1a1955f0e75f59ed7` |

The current browser predicate only reports that the document is wider than
the viewport. This diagnostic adds a bounded list of rendered elements whose
right edge exceeds the viewport, including their tag, selector-like identity,
box coordinates, and computed minimum width. Do not include page text, form
values, tokens, environment values, or local-home paths in the report.

## Exact writable files

- `scripts/verify-interface-browser.mjs`: add overflow-node details to the
  existing viewport failure only.
- This packet and `docs/state-slices.md`: record the diagnostic scope/result.
- `docs/bazaar-hyperai-interface-plan.md`: link the diagnostic and its next
  gate after the run.

`apps/hyperai-interface/app.css`, `app.js`, Rust sources, manifests,
imported-product trees, and the ignored Bazaar checkout are read-only.

## Allowed effects

- Run `corepack pnpm run verify:interface:supply-quote-browser` exactly once.
  It builds the already-pinned local interface binary, starts the runner-owned
  local service and isolated browser/profile, traverses its frozen routes and
  viewports, and attempts the fixed synthetic quote-preview workflow.
- The browser may contact only the test-owned IPv4 loopback service. Use no
  external origins, providers, seller endpoints, registries, models, chains,
  or pre-existing service. Do not contact the user's listener on port 4317.
- Clean only processes, profile data, and temporary home created by this run.
- Run `node --check scripts/verify-interface-browser.mjs` and
  `git diff --check` after the harness edit.

## Stop conditions and next slice

Stop if the input hash changes, the browser runner cannot establish clear
process ownership, an external-origin request is observed, the existing
service is contacted, or cleanup is ambiguous. A failed or inconclusive
diagnostic consumes this slice; do not retry it.

This slice makes no CSS/layout change. Use the diagnostic result to prepare a
separate exact implementation packet naming the offending rule, writable
layout file, one validation run, and any additional responsive checks. Do not
run the browser again until that packet exists.

Non-goals: changing route behavior, markup, content, accessibility semantics,
the supply/quote contract, dependencies, storage, APIs, source imports,
platform claims, or any external integration. No Bazaar files, schemas,
fixtures, datasets, or assets may be read or copied.

## Proof and claim ceiling

Proof: the edited harness reports bounded element geometry for this exact
overflow; the single run records the literal route, viewport, outcome, process
cleanup, and whether the quote-preview interaction was reached. This is
diagnostic evidence only. It does not qualify the layout, feature workflow,
general accessibility, other platforms, Bazaar source rights, execution,
payment, or production readiness.

Assurance begins `NOT_RUN`; if the exact local diagnostic and cleanup checks
pass, report `INTERNAL_PASS` for the diagnostic harness only. Reviewer status
is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Publish boundary: local
source and documentation only; no commit, PR, release, publication, or
deployment.

## Execution record (2026-10-01)

`node --check scripts/verify-interface-browser.mjs` and `git diff --check`
passed. The one `corepack pnpm run
verify:interface:supply-quote-browser` attempt exited 1 at
`zoom-200-equivalent/deals`: the viewport width was 720 CSS pixels and the
document scroll width was 852, while the bounded list of ordinary rendered
element boxes extending beyond the viewport was empty. The quote-preview
workflow was not reached. This is `INCONCLUSIVE` for identifying the cause;
the harness-only diagnostic itself ran and produced its bounded report.

Cleanup checks found no runner-owned browser/service process and no
`/tmp/hyperai-interface-browser-*` directory. No user listener or external
origin was contacted. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. A fresh packet must inspect internal scroll dimensions and
generated pseudo-element geometry before authorizing a CSS repair and one
post-fix browser verification.
