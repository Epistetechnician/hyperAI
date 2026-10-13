# HyperAI Session Activity Owner Links Packet

State slice: `hyperai.bazaar.interface.session_activity_owner_links.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

## Objective

Connect the existing browser Activity surface to the current page-memory
results from Data, Fleet, Eval, and Trade. Each card is a bounded summary with
an exact owner-result digest and a fixed link to the owning workspace. Preserve
the existing Simple Compute Market and HyperIntel cards. Keep Activity
session-only; do not make a shared ledger or attach unrelated results to a
Market deal.

## Frozen baseline

- Repository: HyperAI, branch `main`, base
  `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `cd95880b213c10c01f67b256da806cfb395f21334bab9b136cae7483763e2198`.
- The dirty and untracked worktree is pre-existing and must be preserved.
- Frozen implementation inputs:

| Path | Bytes | SHA-256 |
|---|---:|---|
| `apps/hyperai-interface/app.js` | 161096 | `3c960ee6022a76a1edc50739bc22854701a35344576b32d63e819bece3093f58` |
| `apps/hyperai-interface/app.css` | 95318 | `0a8e9275370a867328e30005c6991d49f159f811e43f708e7940849e1332b8c6` |
| `scripts/verify-interface-browser.mjs` | 198291 | `fcf1ea02d17fd2a056488a1b82f3166b20d8e38aebf479ca5fc7b37e28c2a905` |
| `package.json` | 8319 | `42e6796735cc5176feb5bb1e50ad2a908c35c56d5af6404c4be091efecb8faf7` |
| `docs/state-slices.md` | 1251959 | `c22558d49224d24cbab7c6497aa6387ca63ae0d4324eb95e171bccab00774aea` |
| `docs/bazaar-hyperai-interface-plan.md` | 173791 | `9378a05f4234cba302d04ebdc9d8c470ac296a53a9f526a4781c693ae9d0e1e2` |
| `docs/compute-bazaar-rust-port-subplan.md` | 232938 | `7659f0cb77b1b29a515255587cfe3b4038fcac1a1388cc72001046cad8864c70` |

## Exact writable files

- `apps/hyperai-interface/app.js`
- `scripts/verify-interface-browser.mjs`
- `package.json` (add one root verification command; no dependencies or lockfiles)
- `docs/compute-bazaar-interface-session-activity-links-packet.md`
- `docs/state-slices.md`
- `docs/bazaar-hyperai-interface-plan.md`
- `docs/compute-bazaar-rust-port-subplan.md`

All service, application, domain, storage, CLI, TUI, ACP/MCP, imported
workspace, and Bazaar reference files remain read-only. Existing card styles
in `app.css` are read-only; no visual-system or navigation behavior changes
are in scope.

## Contract and allowed effects

- Derive cards only from result objects already held in the browser's current
  page memory. Do not add polling, re-fetch-on-navigation, storage, or a
  cross-workspace query.
- Keep each artifact under its owner and label it by meaning:
  - Data uses `dataset_id`, `row_count`, and `snapshot_digest`; the digest is a
    source-snapshot identity, not a query-result digest. Do not repeat SQL or
    result cells in Activity.
  - Fleet may show an advisory preview identified by `preview_digest` and a
    virtual simulator transition identified by its `request_digest`. Preserve
    preview/simulation wording; no host operation is implied.
  - Eval uses `suite_digest` and `repeated_result_digest` from the fixed local
    scorer report. State that the result is a synthetic fixture comparison,
    not a model or agent run or quality claim.
  - Trade uses the owner's `replay_digest` and fixed paper-replay label. Do
    not turn asking-price assumptions into a fill, forecast, P&L, or payment
    claim.
- Keep Simple Compute Market captures and HyperIntel reports unchanged.
- Each card's destination comes from a renderer-owned constant for its source
  module and uses the existing `data-page` handler. Navigation must not fetch
  source data, trigger an action, or modify the current result.
- Escape all response-derived text. Omit a card if its required owner digest
  is absent or malformed; never substitute a digest from another object.
- Cards remain explicitly current-session, non-chronological summaries.
  Market history remains a separate durable Market-owned projection; do not
  auto-link Eval, Data, Fleet, or Trade results to a deal.
- Allowed effects are one fixed HyperAI-authored synthetic Data seed in the
  disposable test home followed by a bounded Data query, one Fleet advisory
  preview, one bounded local synthetic Eval run, and one Trade paper
  calculation, all through the existing authenticated loopback service. No
  other mutation route is allowed. Local rendering and deletion of the exact
  disposable test home are permitted. No external source, network, provider, seller,
  remote agent, real workload, paid execution, wallet, chain, publication, or
  production operation is permitted.

## Proof gates

1. The browser performs exactly one fixed synthetic Data seed and query, one
   Fleet operation preview, one Eval fixture run, and one Trade paper
   calculation, then opens Activity and verifies the exact digest and owner
   label for each summary.
2. Activating each owner link returns to that owner's existing page and
   focuses its heading. Navigation sends no additional service request and
   does not mutate the page-memory result.
3. Existing SCM and HyperIntel Activity cards continue to pass their exact
   digest/status/link assertions. Missing results remain absent; no default or
   fabricated records appear.
4. The only successful POST routes are `/api/v1/data/synthetic/seed`,
   `/api/v1/data/query`, `/api/v1/fleet/operation-preview`,
   `/api/v1/eval/run`, `/api/v1/trade/scenario`, and `/api/v1/trade/paper`.
   No Market history, deal, authorization, connection-inventory, service/API,
   ACP/MCP, or imported-workspace contract changes.
5. Run the focused interface suite,
   `corepack pnpm run verify:interface:activity-owner-links-browser`,
   `corepack pnpm run lint:fast`, `git diff --check`, and cleanup checks.
   Record each result separately and recompute exact source hashes and the
   full NUL-delimited worktree status digest.

## Execution result (2026-10-02)

The exact local synthetic browser workflow passed. Pinned Chrome 154.0.8037.92
checked 33 routes and 35 requests at 1440x1000, 720x500, and 390x844; the
reduced-motion run checked 33 route/viewport combinations. Activity contained
the four expected owner summaries:

| Owner | Identity label and digest |
|---|---|
| Data | Source snapshot digest `3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335` |
| Fleet | Preview digest `c629f88308388c0f59fdb5807dfa39c93f2457783a7176833991b9b5ad310356` |
| Eval | Repeated fixture result digest `ce9e41c9dbe250d592ce9ac77700eacbfa661ab40aaeb34263fd3329cca240fd` |
| Trade | Paper replay digest `c839965a60ced5cd9ba1c46448cab1ebe7a3bab24bc4ac8a81c3342d656b6dae` |

All four owner links returned focus to the destination heading, preserved the
displayed digest, and issued no service request during navigation. The cards
fit a 390-pixel viewport without horizontal document overflow; SQL was absent.
Exactly the six allowed synthetic Data/Fleet/Eval/Trade POST routes returned
HTTP 200. Existing SCM and HyperIntel Activity checks remained in the browser
workflow. The disposable home was removed and the service, browser group, and
listeners closed.

Validation results:

- `corepack pnpm run verify:interface:activity-owner-links-browser`: passed;
  pinned Rust build and the fixed synthetic Chrome workflow both exited 0.
- `corepack pnpm run verify:interface`: passed, 34/34 tests.
- `corepack pnpm run lint:fast`: passed, including 34 interface tests, 3
  installer tests, Rust formatting, and strict workspace Clippy.
- `git diff --check`: passed after documentation close-out; the authored files
  also have no trailing spaces or tabs.
- Cleanup: browser workflow reported removal of the disposable state and
  closure of its service, browser group, and listeners.

Assurance is `INTERNAL_PASS` for this exact local synthetic path only. Reviewer
status is `NOT_REQUESTED`; separate-environment reproduction is
`NOT_REPRODUCED`. No import verification, root full suite, independent review,
or production gate is claimed for this slice.

Final source identity (SHA-256; packet excluded from its own manifest):

| Path | Bytes | SHA-256 |
|---|---:|---|
| `apps/hyperai-interface/app.js` | 164943 | `67e7c85a52014aa2501eaa711b28e53d9114fbbfd3b9f03a2d347e4913f748c0` |
| `apps/hyperai-interface/app.css` (unchanged, read-only) | 95318 | `0a8e9275370a867328e30005c6991d49f159f811e43f708e7940849e1332b8c6` |
| `scripts/verify-interface-browser.mjs` | 212035 | `f05bcd2a832659967e761d0f1f23a5f0db12ef866b61c8bd543eea03143b10e0` |
| `package.json` | 8608 | `5240bc37e3fbbc1fb99b44bbc351efb1b1eed789fd58bfd6fa27d0ba5bcd07c9` |
| `docs/state-slices.md` | 1255146 | `9519c1f2cdc2b7eba8c1bc4e760639cd32fddff3996bfbf7298b2e32f5b5205f` |
| `docs/bazaar-hyperai-interface-plan.md` | 174139 | `cabb459317d01e50df27468d7894281f89fd9a492cfd655ab624a07f8c2a499f` |
| `docs/compute-bazaar-rust-port-subplan.md` | 234016 | `02fecaa74bfbc09ad1522d23f6df5e959919d8018e197718345705a951a6eb2a` |

The full NUL-delimited `git status --porcelain=v1 -z` SHA-256 after the scoped
changes is
`627ae6ce51f49e5f08976dd85b5eaacb5f2f2e13a08d616a8401ee94d202b20c`.
This status digest records path/status identity, not file contents; the exact
authored file content digests are listed above. The packet itself is excluded
from the content manifest, and no staging or commit was performed.

## Claim ceiling and non-goals

At most, the tested local browser can present the exact already-loaded Data,
Fleet, Eval, and Trade synthetic result objects as source-labelled session
cards and navigate to their owner pages. Activity is not chronological,
durable, cross-product, or a shared audit record. The source identities do not
prove fresh data, real capacity, correct computation, agent/model quality,
provider access, a trade fill, payment, or settlement. This slice establishes
no live connection, cross-platform behavior, broad accessibility, independent
review, or production readiness. Bazaar code, schemas, fixtures, data,
dependencies, and assets remain excluded.

Assurance target: `INTERNAL_PASS` for the exact local synthetic browser path.
Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED` unless a
separate environment is recorded. Data and artifact custody: local HyperAI
source and disposable synthetic test homes. Publish boundary: local source and
docs only; no commit, PR, external circulation, or deployment.
