# HyperAI Connection Inventory Packet

Status: authorized local, static integration-surface inventory under the
user-directed HyperAI interface refactor and Rust compute-desk subplan.

State slice: `hyperai.bazaar.connection_inventory.v1`.
Parents: [HyperAI interface plan](bazaar-hyperai-interface-plan.md),
[Rust port subplan](compute-bazaar-rust-port-subplan.md), and the accepted
[Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md).

## Baseline and input identity

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated changes.
The files below are frozen inputs for this continuation; the hashes bind the
pre-change checkout bytes, not a clean commit.

| File | Bytes | Pre-change SHA-256 |
|---|---:|---|
| `crates/hyperai-application/src/lib.rs` | 95,768 | `53f9a1edc5e8c4cc1e710e94c227143650ad45052ef68a2ffd2e1cc6f43d5ecb` |
| `crates/hyperai-local-service/src/server.rs` | 80,398 | `75e918d2d08ebdf88c5b4df8db566e1d8e1bb24146d103ef159f004141f09a74` |
| `crates/hyperai-local-service/tests/api_contract.rs` | 9,765 | `f8528a2ccb98296b09005c6bcdfb94466a4de822cbb8265a8559843a1304bdd6` |
| `crates/outcomes-cli/src/lib.rs` | 146,661 | `04d2efbc5cf4de554a048a9d5a6bf274bd49bc4a59ef1fcf021e974632474864` |
| `crates/outcomes-cli/tests/cli_compat.rs` | 60,332 | `924e929e375f23c93f27f88802ea3211ededc5ca92a646a6a688dd281efad4ff` |
| `crates/hyperai-tui/src/lib.rs` | 91,956 | `236e5142e137be31c02e2029291907bbffa65b31c6753b4832cb0b44a98ca803` |
| `apps/hyperai-interface/app.js` | 152,997 | `fbfdbc90fcfd7f12de22ff86bc2cc78a6e673df55d42355b4434b8dfa8cbd464` |
| `apps/hyperai-interface/app.css` | 87,164 | `1a06e6696b65cd7abca3da4ea50db417c55865dce830eca90b7c38d4bf6a1a6d` |
| `apps/hyperai-interface/index.html` | 4,856 | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `package.json` | 6,748 | `7a9ef6b1f4487190fe831e470d07ea1d9163d9f28a6a655d510657ba55f625c2` |
| `crates/hyperai-local-service/src/route_policy.rs` (read-only) | 7,537 | `137cddf675449fcae9531bc9245dfd09b2d3dc8c0b7faa226a15909e1f22806a` |
| `crates/hyperai-acp/src/tools.rs` (read-only) | 5,573 | `e8406f28137afeb26854a77758a09469dae77868c49feb72fc8753c074aac1ac` |
| `crates/hyperai-mcp/src/lib.rs` (read-only) | 33,894 | `e5da96be4c3c161ece1299353ebd99e817c87bf6901d4048fbdeb513d7ffc40b` |
| `crates/hyperai-tui/tests/terminal_restore.rs` (read-only) | 31,745 | `10154a84953a6a7c105e2537d95cacf1a886c86020592d840e83ac6c0c9705f7` |
| `docs/provenance.md` (read-only) | 7,849 | `6e6b0ef176f996b1cafb2e0bf91264fc4dd2f11fda687d949466b2183b966ade` |
| `docs/state-slices.md` | 1,077,027 | `764485facc55033af9023b9902a2ab2c253b79d3245fbb70bcb274e2e92ae0dd` |
| `docs/bazaar-hyperai-interface-plan.md` | 148,834 | `fe16cb3dd85ffab72b5f8dee3214797d3bc81ba306420374dbd2fa000f5ea896` |
| `docs/compute-bazaar-rust-port-subplan.md` | 213,026 | `d102b290e9b6a23d788b4c0fd226d263ad6eeab8241ac43c467e6a20232217f7` |

## Decision and contract

The application already exposes a static `/api/v1/status` inventory, and the
browser and TUI use it to label module states. That frozen status shape does
not carry recorded source revisions, an explicit connection mode, named
exposed surfaces, or the next gate. Do not add fields to or otherwise change
`hyperai.interface.status.v1`. Add a separate authenticated, read-only
`GET /api/v1/connections` contract:

- Envelope schema: `hyperai.interface.connection-inventory.v1`.
- Each descriptor has `module_id`, `name`, `owner`, optional
  `source_revision`, `connection_mode`, the existing static `state`, the
  existing descriptive `boundary`, `exposed_capability_ids`, and a
  `next_blocked_gate` with stable ID and user-readable summary.
- `connection_mode` is one of `local_synthetic`,
  `offline_caller_selected`, `reference_only`, `workspace_not_connected`, or
  `planned`.
- Entries have deterministic order and no generated timestamp. This is
  build-time inventory metadata, not a readiness probe or health result.
- Capability IDs identify descriptive module surfaces. They are not tokens,
  grants, permissions, or proof that an operation is authorized.
- Upstream revisions come only from the pinned provenance record. A null
  revision means there is no applicable pinned source revision; it does not
  invite automatic discovery.
- `next_blocked_gate` describes a documented source, product, or operator
  gate. It does not assert that the listed owner has approved that gate.

The inventory covers the current eleven entries, in the existing
`interface_status()` order: HyperAI Compute Market, Simple Compute Market,
The Compute Bazaar, HyperIntel, HyperAI Fleet, HyperAI Eval, HyperAI Outcomes,
JustAI, ESN-JustNet, Trade, and HyperAI Data. Their source revision, modes,
surface IDs, and next gates must be declared from the current application
inventory and the interface plan. The Bazaar descriptor is reference-only,
has no capabilities, cites only its recorded pin, and names written rights for
the exact materials as its next gate.

The existing `hyperai.interface.status.v1` result must retain the same
top-level fields, connection count, order, and per-entry field values. Its
static data should be projected from the new application-owned descriptor
table so browser, CLI, and TUI do not maintain separate owner/mode tables.
The new authenticated route is additive. It performs no storage read/write,
network request, process probe, or connection attempt.

## Client behavior and compatibility

- Browser: make the existing Connections view and overview cards render the
  versioned inventory. Show owner, connection mode/state, pinned revision
  when present, exposed surface IDs, and the next blocked gate. Escape every
  source-provided string and retain responsive semantic markup.
- CLI: add `hyperai connections` as a read-only local-service command, with a
  stable JSON output option. Existing command names, flags, output, and exit
  codes do not change.
- TUI: add a Connections workspace using the same authenticated route and
  inventory. Preserve current navigation behavior, terminal restoration, and
  non-TTY output.
- ACP/MCP: keep ACP protocol v1, the scoped route allowlist, and MCP tool lists
  V1–V4 unchanged. This inventory is not added to the local agent capability
  in this slice.
- Storage and platform: do not change SQLite schema, home paths, permissions,
  Rust toolchain, dependency versions, or the declared platform matrix.

## Source and rights decisions

No upstream code, data, schemas, fixtures, benchmarks, visual assets, or
dependencies are copied or queried for this inventory. It reads the committed
HyperAI provenance record only. Exact current recorded pins are SCM
`a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`, Bazaar
`d1809e5cfa843c087ebc4de9f00561bf0be67d55`, HyperIntel
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`, and JustAI
`9a0e56536b968cbb2cb7173e142068b53c4f9fe3`.

The SCM pin has an MIT license at its exact upstream revision; preserve its
notice and keep listing-content rights, capture retention, and live registry
access separate. The pinned Bazaar revision's root `LICENSE` path and the
current upstream `main` path both returned 404 in the official raw GitHub
check on 2026-10-01. A license-looking file inside the ignored local reference
cannot be attributed to the pinned revision, so Bazaar remains reference-only;
no code or data reuse is authorized. The HyperIntel and JustAI snapshots have
no verified root redistribution license; use their already-recorded committed
local snapshots only, and do not broaden their reuse or publish them. These
decisions match [provenance](provenance.md) and the existing
[rights discrepancy record](state-slices.md#hyperai-bazaar-source_rights_local_license_discrepancyv1).

## Exact writable files and proof gates

Allowed source files:

- `crates/hyperai-application/src/lib.rs`.
- `crates/hyperai-local-service/src/server.rs`.
- `crates/hyperai-local-service/tests/api_contract.rs`.
- `crates/outcomes-cli/src/lib.rs` and
  `crates/outcomes-cli/tests/cli_compat.rs`.
- `crates/hyperai-tui/src/lib.rs`.
- `apps/hyperai-interface/app.js`, `app.css`, and `index.html`.
- New `apps/hyperai-interface/connection-inventory.js` and
  `connection-inventory.test.mjs`.
- Root `package.json` only to include the new dependency-free browser test and
  syntax check in `verify:interface`.
- This packet, `docs/state-slices.md`,
  `docs/bazaar-hyperai-interface-plan.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`.

Allowed effects: source-only local implementation; authenticated requests to
the test-owned IPv4 loopback service on port `0`; fixed static inventory
metadata from the local provenance record; focused tests, browser smoke,
root validation, and cleanup of test-owned homes and processes.

Non-goals: changes to `/api/v1/status`, the database, environment or platform
support, dependency graph, ACP/MCP grants, outcome/HIP-4 records, provider or
seller requests, Bazaar checkout, source imports, paid evaluation, live Fleet,
financial Trade, custody, chain, reviewer, publication, commit, PR, or
deployment. Do not contact port `127.0.0.1:4317` or user data.

Proof gates:

1. Application tests assert the exact connection inventory schema, fixed
   ordering, all eleven owners/modes/pins/surface IDs/gates, and that static
   generation performs no external probe.
2. Service tests prove authenticated owner access, unauthenticated denial,
   exact response schema, and unchanged `/api/v1/status` field names, values,
   count, and order.
3. CLI, TUI, and browser consume the same inventory values. Browser tests cover
   escaping, absent revision, all connection modes, next-gate rendering, and
   unavailable/error state. TUI tests cover wide and narrow panes.
4. `corepack pnpm run verify:interface`, focused Rust tests for the changed
   crates, pinned formatting, strict Clippy, `python3 scripts/verify_imports.py`,
   `git diff --check`, and `CARGO_NET_OFFLINE=true corepack pnpm run lint` pass.
5. Verify no test-owned processes or temporary homes remain and port 4317 was
   not contacted.

Maximum claim: current local HyperAI-owned connection metadata is rendered
from one static, versioned contract in browser, CLI, and TUI on the tested
host. It does not establish source health, live reachability, rights, identity
trust, capability authorization, external readiness, cross-platform support,
full accessibility, independent review, clean reproduction, or production
readiness. Assurance remains `INCONCLUSIVE` unless slice-specific tests,
static analysis, threat-model coverage, fuzzing where input requires it, and a
bounded adversarial review support a stronger status. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Data/artifact custody is
checked-out HyperAI source and disposable synthetic tests. Publish boundary:
local files only; no commit, PR, release, external circulation, or deployment.

## Execution result (2026-10-01)

Implemented the application-owned inventory and `/api/v1/connections` route;
the unchanged `/api/v1/status` response is projected from the same catalog.
The browser overview cards and Connections surface, `hyperai connections
--json`, and a new TUI Connections workspace read this route. The Bazaar entry
shows only the recorded revision, `reference_only` mode, no exposed surfaces,
and its written-rights gate. No upstream code, data, fixture, asset, or
dependency was imported; no ACP/MCP grant, storage schema, or package
dependency changed.

Validation on macOS arm64, pinned Rust 1.93.1, at baseline revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` plus the dirty worktree:

- `CARGO_NET_OFFLINE=true corepack pnpm run verify:interface`: passed 25 tests.
- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test -p hyperai-application -p hyperai-local-service -p outcomes-cli -p hyperai-tui`: passed; includes API auth/compatibility, installed CLI command, and wide/narrow TUI rendering.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint`: passed, including formatting, strict Clippy, workspace contract tests, and all-feature workspace tests.
- `python3 scripts/verify_imports.py`: passed; SCM, HyperIntel, and JustAI inventories match their recorded pins.
- `git diff --check`: passed for tracked changes. Read-only hashes for route policy, ACP tools, MCP, TUI terminal integration, and provenance still match the frozen packet inputs.
- `corepack pnpm run verify:interface:browser`: the browser runner reached and completed its configured route, viewport, focus, control-name, overflow, and reduced-motion assertions, but the command exited 1 at cleanup after reporting a runner-owned Chrome crashpad helper or service as still active. Later process and handle checks found the service and tracked browser group gone, while one isolated crashpad helper still had this run's temporary directory as its working directory. That helper was terminated and this run's temporary home/profile was removed. Treat the browser command as `INCONCLUSIVE`, not passed. An older pre-existing browser-runner temp root was left untouched. The run used a dynamically reserved loopback port; `127.0.0.1:4317` was not contacted.

Final source hashes for the tested worktree:

| File | SHA-256 |
|---|---|
| `crates/hyperai-application/src/lib.rs` | `1cb64bdea3843140d522334ca3c607eb7955e705bf834f6c3e9a0365198d7f41` |
| `crates/hyperai-local-service/src/server.rs` | `b9cc9e8decb2a341ae4dab63a5ef3a0b1a8ac6714bbb1dc32355c590640cfca1` |
| `crates/hyperai-local-service/tests/api_contract.rs` | `36b9ba02ea16daeb69ff924fa1fc754c7c50d46389d8584358d63855d4ab909e` |
| `crates/outcomes-cli/src/lib.rs` | `7a9f112f1c67e13ae542646ca0f0dbf72cdfe65da0be98804d879d5f16e79a93` |
| `crates/outcomes-cli/tests/cli_compat.rs` | `7b315515113432ce0dce73b9b5dd0229e843313e77313e434c8d815ce0a8b27c` |
| `crates/hyperai-tui/src/lib.rs` | `bf9b2f124345f869b79b30c9d7b8cbc908994fe5dbcdb68e6bdae5e5ac551ff5` |
| `apps/hyperai-interface/app.js` | `fa6f6e6c46ddf72f786be9520fcede454137618c0608f08904c69b0e7bc3c142` |
| `apps/hyperai-interface/app.css` | `254d471c5bdcdba8ec3d7408bb3c6f66a4fb377aa6243d2c694ee598d00dd65b` |
| `apps/hyperai-interface/index.html` | `1447f2637de79aca50247665ee1355a938dac8870656019fbabcd9e758709a22` |
| `apps/hyperai-interface/connection-inventory.js` | `8c4570b67b95aa2a487113fca04923d87fff13de454ea9c139c63a05325aeb33` |
| `apps/hyperai-interface/connection-inventory.test.mjs` | `ce456e30de6a5f92d7d3ebbb721f0ddf585e35e1e58ee088b1d1ff89c8cbf892` |
| `package.json` | `f5bcb21883d20894bfd58c8438bc87895df3cb64f4fbe9033dce03302ca3f1d6` |

Assurance remains `INCONCLUSIVE`: focused and root checks pass, but this slice
did not include a dedicated threat model, fuzz campaign, or bounded adversarial
review, and the browser runner's cleanup contract did not pass. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Data/artifact
custody is the dirty local HyperAI worktree and disposable synthetic tests.
The claim ceiling is one static local inventory shared by browser, CLI, and
TUI on the tested host; it establishes no source health, live rights, trust,
authorization, execution, payment, cross-platform support, or production
readiness. The next gates are browser-runner cleanup qualification, broader
workflow accessibility/client parity, and the remaining native platform and
owner-authorization slices in the linked plans.
