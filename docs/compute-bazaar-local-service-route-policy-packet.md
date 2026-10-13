# HyperAI Local-Service Route Policy Packet

Status: completed bounded local refactor; `INTERNAL_PASS` for the proof gates
recorded below under `hyperai.bazaar.local_service_route_policy.v1`.

Parent: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related contracts: [Phase 1 runtime packet](compute-bazaar-phase-1-contract-packet.md)
and [Phase 8 ACP packet](compute-bazaar-phase-8-acp-packet.md).

## Objective

Give the local service one typed definition for its scoped agent-tool routes.
That definition must drive Axum route registration, restricted-capability
authorization, and policy tests. Preserve the existing paths, methods,
handlers, request and response schemas, owner bearer behavior, and exact
restricted capability grant. This is an internal locality improvement; it
adds no route or authority.

## Frozen capability grant

The restricted per-session capability permits exactly these operations:

| Method | Route |
|---|---|
| GET | `/api/v1/agent/tools/data/catalog` |
| POST | `/api/v1/agent/tools/data/query` |
| GET | `/api/v1/agent/tools/fleet/inventory` |
| GET | `/api/v1/agent/tools/fleet/doctor` |
| POST | `/api/v1/agent/tools/fleet/operation-preview` |
| GET | `/api/v1/agent/tools/eval/catalog` |
| GET | `/api/v1/agent/tools/eval/reports` |
| GET | `/api/v1/agent/tools/market/workload-trace` |
| GET | `/api/v1/agent/tools/market/workload-trace/failure-preview` |
| GET | `/api/v1/agent/tools/market/workload-trace/inconclusive-preview` |
| GET | `/api/v1/agent/tools/trade/catalog` |
| GET | `/api/v1/agent/tools/trade/series/{instrument_id}` |
| POST | `/api/v1/agent/tools/trade/scenario` |

The `instrument_id` segment must be present and contain no additional path
segment, matching current behavior. All other paths and method/path pairs are
denied to this capability. The owner bearer continues to authorize the
existing owner routes under the current service policy.

## Baseline and writable files

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the preserved dirty and untracked worktree. Verify these current source bytes
before implementation; if either differs, refresh this packet first.

| File | Bytes | SHA-256 |
|---|---:|---|
| `crates/hyperai-local-service/src/lib.rs` | 184 | `73da914bb4a3946effe5ec58af6f2f7f66f25ed686631dff17df57f6a0748c60` |
| `crates/hyperai-local-service/src/server.rs` | 81,302 | `8b40261e216cc9c287833969e983fc472226bb3800d4409dd334950258188d20` |

Exact writable files:

- `crates/hyperai-local-service/src/lib.rs`: declare the private policy module.
- `crates/hyperai-local-service/src/route_policy.rs`: define typed agent-tool
  route identities, paths, methods, matching, route registration, and focused
  policy tests.
- `crates/hyperai-local-service/src/server.rs`: construct agent-tool routes
  from the typed definitions and delegate capability checks to the policy
  module. Keep the independent owner-route denial inventory and its HTTP
  coverage.
- This packet and `docs/state-slices.md`: record scope, result, and evidence.

No manifest, lockfile, public response, browser, CLI, TUI, ACP/MCP tool list,
or other workspace file is in scope.

## Allowed effects and non-goals

Allowed effects: local Rust source and test changes; in-process tests and
test-owned loopback services on an ephemeral port; temporary synthetic data
owned by the tests; pinned offline package validation.

Non-goals: changing a route, method, response, schema, capability, owner
bearer rule, browser session/CSRF policy, or ACP/MCP protocol; adding a route,
credential, process, provider, seller, registry, external agent, remote
worker, wallet, chain, or network call; using Bazaar source or data; touching
port `4317`, user data, or nested product workspaces; committing, publishing,
opening a PR, or deploying.

## Proof gates

1. Recheck the frozen source sizes and digests before editing.
2. Verify each of the 13 declared route identities is registered once with
   its exact existing handler, path, and method; reject duplicate declarations.
3. Verify the restricted capability accepts every declared method/path and
   denies the wrong method, empty or multi-segment instrument IDs, unknown
   tool paths, and owner paths.
4. Preserve the existing 43-route owner denial checks and owner-bearer access
   checks, including their HTTP middleware coverage.
5. Run the focused local-service policy and agent API suites, pinned Rust
   formatting, strict local-service Clippy, root `corepack pnpm run lint`,
   and `git diff --check`.
6. Confirm no test-owned process remains and no request reached port `4317`.

Claim ceiling: the unchanged restricted-capability method/path policy is
defined once and used by route registration, authorization, and the local
policy tests. This demonstrates preservation of the listed authorization
behavior only to the extent those named tests cover it. It does not establish
complete product policy parity, independent review, separate-host
reproduction, platform support, or production readiness.

Reviewer status: `NOT_REQUESTED`. Reproducibility target:
`NOT_REPRODUCED`. Data/artifact custody: checked-out HyperAI source and
temporary HyperAI-authored synthetic tests only. Publish boundary: local
source and Markdown; no commit, PR, external write, or deployment.

## Execution result — 2026-10-01

The pre-edit source sizes and hashes matched the frozen baseline. The 13
typed route definitions are registered through an exhaustive match, so a new
route identity cannot compile until its existing handler mapping is declared.
Registration derives the path and GET/POST method from each route identity;
authorization and policy tests read the same definitions. The golden grant
test passed, including uniqueness and the parameter-route shape. The existing
43-route owner denial and owner-bearer checks remain unchanged and passed.

Validation passed:

- Local-service library: 14 tests; `agent_api`: 10 tests.
- Strict local-service Clippy for all targets and features.
- Pinned Rust formatting.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0, including
  `lint:fast`, workspace contract tests, all-feature workspace tests, and doc
  tests.
- `git diff --check` exited 0; a scoped trailing-whitespace scan found no
  matches in the packet's writable files.
- No test service remained and no listener was present on port `4317`. The
  scoped tests used in-process routers or test-owned ephemeral ports; no
  external services or Bazaar inputs were used.

The root run retained its declared environment-gated skips: PostgreSQL needs
`HSAI_TEST_POSTGRES_URL`; Docker cases need explicitly selected local images;
two CLI lifecycle cases need staged sidecars/installation; and browser
reconciliation needs a built UI binary plus Chrome or Chromium. These skips do
not affect the local-service route-policy proof gates. Root validation does
not validate the excluded HyperIntel, JustAI, or Simple Compute Market
nested workspaces.

Assurance: `INTERNAL_PASS` for the exact route-policy implementation and
listed local checks. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. Claim ceiling and publish boundary remain as above. Final
source bytes and SHA-256:

| File | Bytes | SHA-256 |
|---|---:|---|
| `crates/hyperai-local-service/src/lib.rs` | 202 | `806704fc3862d568af434ab54c428119b19dadb3ccfa9bce74765efa925d3180` |
| `crates/hyperai-local-service/src/server.rs` | 80,398 | `75e918d2d08ebdf88c5b4df8db566e1d8e1bb24146d103ef159f004141f09a74` |
| `crates/hyperai-local-service/src/route_policy.rs` | 7,537 | `137cddf675449fcae9531bc9245dfd09b2d3dc8c0b7faa226a15909e1f22806a` |
