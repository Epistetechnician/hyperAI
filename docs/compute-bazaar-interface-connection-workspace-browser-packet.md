# HyperAI Connection Workspace Browser Packet

Status: `INTERNAL_PASS` for the exact local pointer and keyboard navigation
paths recorded below.
The user-directed Rust compute-desk subplan and local interface work authorize
only this synthetic, loopback-only qualification. This packet grants no new
source, provider, seller, agent, Fleet, wallet, chain, release, or external
effect authority.

State slice: `hyperai.bazaar.interface.connection_workspace_browser.v1`.
Parent: [workspace-links packet](compute-bazaar-connection-workspace-links-packet.md),
[HyperAI interface plan](bazaar-hyperai-interface-plan.md), and
[Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

## Baseline and frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked working tree. Preserve all existing files
and changes. This slice writes only this packet, `docs/state-slices.md`, and
the two parent planning documents after evidence is recorded.

| Read-only input | SHA-256 |
|---|---|
| `apps/hyperai-interface/connection-inventory.js` | `77b595b757ac6caf1043e2b0f6629ae0a97b56bf9d923f6cff7ec19c51aa42fd` |
| `apps/hyperai-interface/connection-inventory.test.mjs` | `c01f01c04a844d5ab5d6ece541b36380422144dabaa4556f6addb39ee303961b` |
| `apps/hyperai-interface/app.css` | `cae429ce4f38abe89b05923853cfbc6f009f89315048a5cae1dc8ba178eea3b0` |
| `apps/hyperai-interface/app.js` | `fa6f6e6c46ddf72f786be9520fcede454137618c0608f08904c69b0e7bc3c142` |
| `apps/hyperai-interface/index.html` | `1447f2637de79aca50247665ee1355a938dac8870656019fbabcd9e758709a22` |

The current page handler delegates every `data-page` control to the existing
renderer, then focuses `#workspace-page-title` or the page root. The renderer
owns the fixed route map and displays buttons only for eligible local or
offline entries with descriptive capabilities. No source-owned route value
is executed.

## Question and procedure

Verify that the served browser interface can open an eligible workspace from
the Connections inventory by pointer and keyboard, that focus lands on the
destination page title/root, and that reference-only, disconnected, planned,
unknown, and empty-capability entries remain informational.

1. Start the current HyperAI interface from the locked, offline Rust workspace
   on an available IPv4 loopback port, using a fresh mode-0700 temporary home.
   Use the pinned Rust 1.93.1 toolchain. Do not print or copy the runtime bearer
   capability; browser access uses the local session flow.
2. Open a new hidden Codex in-app browser tab at the loopback URL. Do not
   navigate, reuse, or close the user's existing browser tabs.
3. Open Agents & Access / Connections. Confirm the source modes and visible
   labels. Activate one eligible workspace button by pointer and one by
   keyboard Enter after reaching it with Tab. Observe both the destination
   page and focused title/root using the browser accessibility state and, if
   supported by its documented browser API, the active element identity.
4. Confirm the Bazaar reference, disconnected JustAI, planned ESN-JustNet, and
   any empty-capability descriptor expose no workspace button. Do not invoke
   any action control or load a source payload.
5. Stop only the process started by this packet, close only the packet's new
   browser tab, verify the service/process scope is gone, and remove only its
   disposable home.

If the browser does not expose enough evidence to verify actual focus, record
that gate as `INCONCLUSIVE`; do not infer focus from markup or route content.
If any code change appears necessary, stop and open a separate code state
slice with its own packet before editing source.

## Exact effect and proof boundary

Writable files: this packet, `docs/state-slices.md`,
`docs/bazaar-hyperai-interface-plan.md`, and
`docs/compute-bazaar-rust-port-subplan.md`. App source, API/service contracts,
storage, CLI, TUI, MCP/ACP, source snapshots, manifests, and the ignored Bazaar
reference are read-only.

Allowed effects: one local synthetic service bound to IPv4 loopback, browser
requests to that loopback service, pointer/keyboard navigation among existing
local interface pages, and disposable service-home files. No external network
request, connection probe, caller payload, source import, provider/seller call,
agent run, Fleet operation, paid evaluation, funds movement, chain access, or
publication is allowed.

Proof gates:

1. Recheck frozen input hashes before the run; confirm no HyperAI interface
   service from another task owns the chosen port or temporary home.
2. Confirm the served page reports its local synthetic mode and renders the
   static connection inventory.
3. Confirm pointer activation reaches the fixed eligible destination and
   focus moves to its page title/root.
4. Confirm Tab reaches an eligible control, Enter activates it, and focus
   moves to the destination title/root.
5. Confirm the Bazaar, JustAI, ESN-JustNet, and other ineligible entries have
   no navigation control.
6. Confirm only the packet-owned process, browser tab, and temporary home are
   cleaned up; run `git diff --check` and explicit whitespace/link checks for
   the documentation update.

Claim ceiling: successful local browser navigation and focus for the exact
tested controls only. This does not establish broad accessibility, browser or
platform support, source rights, source health, external connectivity,
authorization, execution, payment, independent review, or production
readiness. Initial assurance: `NOT_RUN`. Reviewer status:
`NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Data custody: local
synthetic connection descriptors and a disposable service home. Publish
boundary: local documentation and ephemeral loopback interaction only; no
commit, PR, release, external circulation, or deployment.

## Execution record

The five frozen input hashes matched before execution and again after cleanup.
The service was launched with the pinned Rust 1.93.1 toolchain, locked offline
dependencies, and `--port 51895 --home
/tmp/hyperai-connection-workspace-browser.qIvw3N`; it reported local synthetic
mode. One hidden Codex in-app browser tab opened at `127.0.0.1:51895`.

From Agents & Access / Connections, pointer activation of **Open HyperIntel
workspace** navigated to Inference and placed focus on the `workspace-page-title`
heading. Returning to Connections, Tab focused **Open HyperAI Compute Market
workspace**; Enter navigated to Market and placed focus on that page's
`workspace-page-title` heading. The accessibility tree showed the Bazaar
reference, disconnected JustAI and HyperAI Outcomes, and planned ESN-JustNet
as descriptive entries without workspace controls. No action control or
source payload was invoked.

The packet-owned tab was closed and the service session was stopped with
Ctrl+C. No listener remained on port 51895; the exact disposable home was
removed. The frozen app input hashes remain unchanged. The browser interactions
and cleanup are `INTERNAL_PASS` for these two controls in this local browser
session. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Data custody is limited to static connection descriptors
and the disposable local service home. This does not qualify broader
accessibility, another browser or operating system, source health, any
external connection, or product release. The next gates remain broader
client-policy/accessibility parity and native-platform qualification.
