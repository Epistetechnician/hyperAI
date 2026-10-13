# HyperAI Connection Inventory Workspace Links Packet

Status: local browser navigation refinement under the end-to-end HyperAI
interface refactor and Rust compute-desk subplan.

State slice: `hyperai.bazaar.connection_inventory_workspace_links.v1`.
Parent: [connection inventory](compute-bazaar-connection-inventory-packet.md),
[HyperAI interface plan](bazaar-hyperai-interface-plan.md), and
[Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

## Baseline and frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated changes. The
following sizes and SHA-256 values bind the pre-change working-tree bytes:

| File | Pre-change SHA-256 |
|---|---|
| `apps/hyperai-interface/connection-inventory.js` | `8c4570b67b95aa2a487113fca04923d87fff13de454ea9c139c63a05325aeb33` |
| `apps/hyperai-interface/connection-inventory.test.mjs` | `ce456e30de6a5f92d7d3ebbb721f0ddf585e35e1e58ee088b1d1ff89c8cbf892` |
| `apps/hyperai-interface/app.css` | `254d471c5bdcdba8ec3d7408bb3c6f66a4fb377aa6243d2c694ee598d00dd65b` |
| `apps/hyperai-interface/app.js` (read-only navigation handler) | `fa6f6e6c46ddf72f786be9520fcede454137618c0608f08904c69b0e7bc3c142` |
| `apps/hyperai-interface/index.html` (read-only page map) | `1447f2637de79aca50247665ee1355a938dac8870656019fbabcd9e758709a22` |
| `docs/bazaar-hyperai-interface-plan.md` | `604ac83a05f1fc89f3dfa2e0c6029b396a215d6b2a6342bd50ccffea1f76748f` |
| `docs/compute-bazaar-rust-port-subplan.md` | `02a172a09082c815dd49cd50bb9bb41683cef0e8d82d234ef09a7ad54daa6229` |
| `docs/state-slices.md` | `9f61da91d533c5b44471359bcafadea6689b23cb2823525dfbd5b974b61a85e3` |

These read-only hashes bind the existing navigation handler and page IDs;
neither file is writable in this slice. No app code or markup changes are
planned.

## Decision and behavior

Make the static Connections inventory a useful navigation surface into the
workspaces HyperAI already exposes. Render an **Open workspace** button only
when all of these conditions hold:

1. the `module_id` is present in a fixed renderer-owned allowlist;
2. `connection_mode` is `local_synthetic` or `offline_caller_selected`; and
3. the descriptor has at least one descriptive exposed capability ID.

The allowlist maps `compute_market` and `simple_compute_market` to `market`,
`hyperintel` to `inference`, and the existing `data`, `fleet`, `eval`, and
`trade` entries to their same-named workspaces. Values from the inventory may
not select a route. Reference-only Bazaar, disconnected Outcomes/JustAI, and
planned ESN entries remain descriptive and have no workspace action, even if
a malformed descriptor claims capabilities. Unknown IDs never receive a
link. These are local page navigations only; they do not connect to or probe a
source, grant a capability, or authorize an operation.

Use the existing delegated `data-page` navigation, which already changes the
page and moves focus to its heading/root. Keep source-provided labels escaped
and use accessible, explicit button names. Keep the inventory schema, API,
CLI, TUI, MCP/ACP grants, permissions, storage, and source owners unchanged.

## Exact writable files

- `apps/hyperai-interface/connection-inventory.js`
- `apps/hyperai-interface/connection-inventory.test.mjs`
- `apps/hyperai-interface/app.css`
- This packet, `docs/state-slices.md`,
  `docs/bazaar-hyperai-interface-plan.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`.

Read-only inputs: the existing `data-page` handler in `app.js`, the page IDs
in `index.html`, and the application-owned static inventory contract. No
other files may change under this slice.

## Allowed effects and proof gates

Allowed effects: local renderer and style changes; deterministic Node tests;
JavaScript syntax checks; documentation and diff-hygiene checks; and offline
root validation. No browser, service, network, provider, seller, agent, Fleet,
wallet, or chain connection is needed. Do not retry the known inconclusive
headless-browser cleanup path in this slice.

Proof gates:

1. Assert the fixed allowlist opens only current `data-page` destinations for
   eligible local/offline descriptors.
2. Assert reference-only, disconnected, planned, empty-capability, and
   unknown descriptors do not render navigation controls, including when
   adversarial metadata supplies a fake capability.
3. Assert accessible labels and source text remain safely escaped in cards
   and the detailed inventory.
4. Assert existing page navigation remains the only route mechanism and no
   service/API contract, auth, permission, source, or side-effect behavior
   changes.
5. Run `corepack pnpm run verify:interface`, focused package-independent
   checks, `git diff --check`, and offline `corepack pnpm run lint` if the
   focused gates pass. Preserve all unrelated dirty files.

## Bounded threat review

Assets: the local page-navigation target and the integrity of source-provided
connection labels. The bounded attacker controls a malformed or future
connection descriptor, including `module_id`, mode, capability IDs, and name.
Abuse cases include injecting markup, choosing an arbitrary route, spoofing a
capability to make a restricted connection appear actionable, and exploiting
object-prototype property names. The renderer escapes labels; looks up only
own properties of a fixed module-to-page allowlist; requires a permitted
local/offline mode and non-empty descriptive capability list; and emits only
existing `data-page` destinations. Tests exercise hostile labels, `__proto__`,
`constructor`, fake capabilities, restricted modes, unknown IDs, and a bounded
128-ID unknown-module sweep. The remaining authority check stays in the
destination workspace's existing owner route. This does not assess malicious
same-origin script execution or establish end-to-end browser interaction.

## Claim ceiling and publish boundary

Maximum claim: for eligible static descriptors, the renderer emits an
accessible `data-page` button whose target is in the fixed local allowlist and
present in the existing shell navigation. Browser click/focus behavior was
not exercised in this slice. This establishes no external connectivity,
health, rights, trust, authorization, execution, payment, broad accessibility,
cross-platform support, independent review, or production readiness.

Assurance begins `INCONCLUSIVE`; a stronger label requires the slice-specific
static, adversarial, and validation gates to be recorded. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Custody is local
HyperAI source and deterministic synthetic tests. Publish boundary: local
files only; no commit, PR, release, external circulation, or deployment.

## Execution result (2026-10-01)

The Connections view and overview cards now render a workspace button only
for the allowlisted, already-exposed local synthetic or offline caller-selected
entries that also declare a surface. The route comes from a renderer-owned
map, dynamic names remain escaped, and ineligible entries remain informational
even when fixture metadata claims a capability. Navigation delegates to the
existing `data-page` handler; no application, API, service, auth, or permission
contract changed. The browser runner was not invoked, so click and focus
behavior are unverified here.

Validation:

- `CARGO_NET_OFFLINE=true corepack pnpm run verify:interface`: passed 29 tests,
  including syntax checks, safe allowlist destinations, source-label escaping,
  restricted-mode/fake-capability denial, prototype-key rejection, the
  128-ID unknown-module sweep, and destination presence in both shell
  navigation and the app page renderer.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint`: exited 0. The root gates
  included interface and installer tests, pinned formatting, strict all-feature
  Clippy, contract tests, all-feature workspace tests, and doc tests. Existing
  environment/staging-gated tests remain ignored by their declared conditions.
- `git diff --check` and explicit trailing-whitespace checks are recorded after
  the documentation update.

Final touched renderer bytes:

| File | Bytes | SHA-256 |
|---|---:|---|
| `apps/hyperai-interface/connection-inventory.js` | 4,751 | `77b595b757ac6caf1043e2b0f6629ae0a97b56bf9d923f6cff7ec19c51aa42fd` |
| `apps/hyperai-interface/connection-inventory.test.mjs` | 6,911 | `c01f01c04a844d5ab5d6ece541b36380422144dabaa4556f6addb39ee303961b` |
| `apps/hyperai-interface/app.css` | 90,133 | `cae429ce4f38abe89b05923853cfbc6f009f89315048a5cae1dc8ba178eea3b0` |

Assurance is `INTERNAL_PASS` for the fixed renderer-to-page-markup contract
only. The bounded threat cases above are covered by deterministic local tests;
reviewer status is `NOT_REQUESTED`, and reproducibility is `NOT_REPRODUCED`.
The claim ceiling remains safe static route-control generation, not successful
browser navigation or full interface integration. Custody is local HyperAI
source and synthetic tests; publish boundary remains local files only.
