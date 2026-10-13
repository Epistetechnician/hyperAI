# HyperAI Interface Task-Navigation Source Diagnostic Packet v1

Status: completed `INTERNAL_PASS` for the source-grounded cause of the consumed
keyboard path-recovery workflow's first Overview transition timeout.

State slice: `hyperai.bazaar.interface.task_navigation_diagnostic.v1`.
Failed execution: [`task-navigation keyboard path recovery v1`](compute-bazaar-interface-task-navigation-keyboard-path-recovery-v1-packet.md).
Interface direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. Before this packet,
`git status --porcelain=v1 -z` SHA-256 was
`508c787de0057f19f9d0cb4f889f857fb3ec48cbe9b8893effc4bee98c775388`; no paths
were staged.

## Frozen read-only inputs

| Input | SHA-256 |
|---|---|
| `apps/hyperai-interface/index.html` | `026c82dbc0ea75b115b9428adf918e6800564425f5c19f939c334694783d38f1` |
| `apps/hyperai-interface/app.js` | `9764756b31e215e545d88ed60aa91e2ea75a281ebe1fa4601413c88061cb2ad1` |
| `apps/hyperai-interface/app.css` | `ded42d5f92c5e96495a30199bae36b8d86d2eb54f717f201f7a857c7d26f5251` |
| `scripts/verify-interface-browser.mjs` | `e8c81bfd0cd6b0f5bdad157d560dddb6a80f1f9569b7854fc606e0737af7633e` |
| `package.json` | `c7d2c372b17dabd81ec057077f847fdde0e15d32a90104cd79e7c190c5286354` |

The observed failure was `keyboard navigation to Overview timed out` with
`route=overview`, `activeCount=1`, heading `Compute, with context.`, one
matching heading, focus on `workspace-page-title`, and visible focus. The
predecessor packet contains the unchanged route and mobile-menu assertions.

## Scope and allowed effects

Exact writable files are this packet and `docs/state-slices.md`. Read the
frozen inputs above, the predecessor v1/v2 packets, the route definitions and
browser-facing service contracts needed to explain the observed state. Do not
edit product code, browser runner, package scripts, manifests, lockfiles,
fixtures, imported workspaces, or other documentation.

Allowed effects: inspect source and existing test/runner control flow; record
line-specific evidence and a source-grounded diagnosis in this packet and its
state-slice record; run read-only hash, whitespace, and link checks. Do not
start a browser, service, test, or package command; make no network or external
calls; write no runtime data; and make no product-source or harness change.

## Proof gates and claim ceiling

Trace the initial route and focus, the runner's keyboard-event sequence, its
route transition wait predicate and timeout, the application's route-change
and focus behavior, and the fields included in the failure report. Determine
whether the observed state is inconsistent with the frozen assertion, whether
the runner expects a non-idempotent navigation for the already-active
Overview route, or whether the source leaves the cause unresolved. Cite exact
file and line ranges. If source review cannot prove a single cause, record
`INCONCLUSIVE` and specify only the smallest new runtime observation needed;
do not infer behavior from tests that this packet does not run.

This packet grants no runtime/browser acceptance, accessibility, platform,
source-rights, independent-review, or production claim. Assurance is
`INTERNAL_PASS` for this exact source diagnosis only; reviewer status is
`NOT_REQUESTED` and reproduction is `NOT_REPRODUCED`. Any code repair, browser rerun, or accessibility claim needs
a new named state slice and packet. Publish boundary: local documentation only;
no commit, PR, external write, or deployment.

## Source finding (2026-10-02)

The runner uses the navigation label as both the button's accessible name and
the expected page H1. In `scripts/verify-interface-browser.mjs:2815`, the
Overview entry is `[overview, Overview]`; lines 2848-2854 bind the second
value to `expectedName` and correctly use it for the navigation button.
However, lines 2872 and 2882 also require the rendered H1 and its accessible
heading name to equal `expectedName`.

The UI intentionally gives Overview a descriptive H1. The shared heading
renderer is at `apps/hyperai-interface/app.js:1043-1044`, and
`renderOverview()` supplies `Compute, with context.` at line 1057. The initial
HTML independently confirms the nav label `Overview` at
`apps/hyperai-interface/index.html:33`. The route click handler updates the
route, renders, and focuses `#workspace-page-title` at
`apps/hyperai-interface/app.js:1883-1887`.

Thus the v2 failure's exact state—active route `overview`, heading
`Compute, with context.`, and visible focus on `workspace-page-title`—matches
the application's intended route behavior. The wait predicate can never pass
because it compares that H1 with `Overview`. This is a test-harness expectation
bug, not evidence of a product navigation failure. The smallest correction is
to keep the nav accessible name `Overview` and use the actual descriptive H1 as
the expected workspace heading. All other route and mobile assertions remain
unchanged. This diagnosis is source-grounded; no runtime test was executed by
this packet.
