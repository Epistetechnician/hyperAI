# HyperAI Local Service Route, Auth, and Effect Inventory v1

- **State slice:** `hyperai.bazaar.route_auth_effect_inventory.v1`
- **Governing overlay:** `global-experiment-assurance-lifecycle-v1`
- **Frozen source revision:** `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`
- **Pre-edit worktree status digest:** `5b08d339e4084c6f33246854aa5712ef0128c19ead04977120dac6e30bd9d3fd`

## Scope and assurance

This document is a static inventory of the local-service route registrations,
the shared authentication guard, the separately scoped agent-tool capability,
high-level local effects, and source-visible client call sites. The
2026-10-05 GPUDiff snapshot slice adds one owner-only Data route under
`hyperai.bazaar.gpudiff_live_offer_snapshot.v1`. The 2026-10-06 scheduled
refresh slice adds owner-only status/start/stop routes under
`hyperai.bazaar.gpudiff_owner_scheduled_refresh.v1`; neither changes the agent
allowlist.

The frozen baseline at `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` registered
50 literal `/api/v1` routes and 8 non-API routes. The current source adds four
direct GPUDiff routes, for 54 direct routes plus 15 allowlisted agent routes
(69 API registrations) and 8 static/health registrations. These are
registered route/method pairs; this count does not expand Axum-generated
`HEAD` behavior or other protocol methods.

- **Assurance:** `NOT_RUN` for complete route-by-route client compatibility.
  The bounded local tests below passed, but they do not exercise every client
  against every registered route.
- **Reviewer:** `NOT_REQUESTED`.
- **Reproduction:** `NOT_REPRODUCED`.
- **Custody:** source and documents are in the existing local checkout; this
  slice copies no imported product or reference-source content.
- **Claim ceiling:** source-level route/auth/effect and client-call-site
  inventory only. It does not establish runtime compatibility, complete
  schema/error parity, external review, platform support, or production
  readiness.

## Authentication boundary

`GET /api/v1/session` and `GET /api/v1/status` bypass the API authentication
middleware. Session bootstrap rotates a volatile browser cookie and CSRF
token. Every other `/api/v1/...` route accepts the process owner bearer; a
browser session cookie is also accepted, with a valid CSRF token and an
`Origin` header required for non-GET/HEAD requests. A process bearer is not
scoped by route.

The separate agent-tool capability is accepted only when both method and path
match one of the 15 entries in `route_policy.rs`. That capability is separate
from the process owner bearer. The browser, CLI, and TUI use ordinary
owner-authenticated service routes; MCP reaches the allowlisted agent-tool
routes; ACP exposes the bundled MCP tools. The scoped capability does not
include quote approval/completion, Data definition saves, Fleet simulation
transitions, Eval execution, or agent-session control.

## Effect labels

| Label | Meaning in this inventory |
| --- | --- |
| `READ` | Reads or returns a catalog, status, report, history page, comparison, or verification result. |
| `PREVIEW` | Computes a local/synthetic preview or projection without an owner-record write. Some workload-trace projections read the selected local Data snapshot. |
| `LOCAL_WRITE` | Changes local persistent or process-held owner state. This does not imply a remote action. |
| `LOCAL_EXEC` | Runs bounded local synthetic/paper computation and may retain its report in process memory. |
| `REMOTE_READ_LOCAL_WRITE` | Makes one bounded owner-triggered read from the fixed public GPUDiff endpoint and stores immutable source/normalized bytes plus a local receipt. |
| `AGENT_OP` | Starts/stops or prompts the configured local ACP/MCP agent path. Its downstream model/provider/network behavior is not qualified by this slice. |
| `SERVICE_OP` | Changes the local service process lifecycle. |

Client marks are source-visible call sites: `B` browser, `C` CLI, `T` TUI,
`MCP` MCP tool bridge, and `ACP` the MCP tools surfaced through ACP. A `—`
means no call site was found in those client sources; it is not a claim that
the route is unavailable. `C*` means the CLI composes that workload trace
through `hyperai-application` directly instead of calling the HTTP route.

## Static and health routes

These routes are outside the `/api/v1` authentication middleware.

| Method | Path | Owner / effect | Source-visible clients |
| --- | --- | --- | --- |
| `GET` | `/` | Interface shell; static | B |
| `GET` | `/app.css` | Interface stylesheet; static | B |
| `GET` | `/error-envelope.js` | Error-envelope client code; static | B |
| `GET` | `/workload-trace.js` | Workload-trace client code; static | B |
| `GET` | `/connection-inventory.js` | Connection-inventory client code; static | B |
| `GET` | `/market-supply-comparison.js` | Supply-comparison client code; static | B |
| `GET` | `/app.js` | Interface client code; static | B |
| `GET` | `/healthz` | Process liveness; read-only | Service/MCP health checks |

## Direct service API routes

All rows below use the common authentication boundary above. The scoped agent
capability does not authorize these paths; its parallel `/agent/tools/...`
routes are listed in the next section.

| Method | Path | Owner | Effect class and behavior | Source-visible clients |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/session` | Local service auth | `LOCAL_WRITE` — rotates volatile browser session and CSRF capability | B |
| `GET` | `/api/v1/status` | Local service | `READ` — returns interface status | B |
| `GET` | `/api/v1/connections` | Local service | `READ` — returns connection inventory | B, C |
| `POST` | `/api/v1/service/stop` | Local service | `SERVICE_OP` — requests service shutdown | C |
| `POST` | `/api/v1/agent/connect` | ACP session owner | `AGENT_OP` — starts configured local agent session and issues scoped capability | B, C, T |
| `GET` | `/api/v1/agent/status` | ACP session owner | `READ` — reads local agent-session snapshot | B, C, T |
| `POST` | `/api/v1/agent/prompt` | ACP session owner | `AGENT_OP` — sends prompt to configured agent session | B, C, T |
| `POST` | `/api/v1/agent/cancel` | ACP session owner | `AGENT_OP` — requests cancellation of active agent operation | B, C, T |
| `POST` | `/api/v1/agent/disconnect` | ACP session owner | `AGENT_OP` — revokes scoped capability and disconnects local session | B, C, T |
| `GET` | `/api/v1/market/workload-trace` | HyperAI Market | `PREVIEW` — composes local workload trace; may read selected local Data snapshot | B, T, C* |
| `GET` | `/api/v1/market/supply-comparison` | HyperAI Market | `READ` — returns local synthetic supply comparison | B, C, T |
| `GET` | `/api/v1/market/workload-trace/failure-preview` | HyperAI Market | `PREVIEW` — returns fixed seller-failure preview | B, T, C* |
| `GET` | `/api/v1/market/workload-trace/inconclusive-preview` | HyperAI Market | `PREVIEW` — returns fixed inconclusive preview | B, T, C* |
| `GET` | `/api/v1/market/workload-trace/data-status-preview/{status}` | HyperAI Market | `PREVIEW` — fixed Data-status presentation; no source read | B |
| `GET` | `/api/v1/market/workload-trace/duplicate-disposition-preview` | HyperAI Market | `PREVIEW` — returns duplicate-disposition preview | B |
| `POST` | `/api/v1/market/demo` | HyperAI Market | `PREVIEW` — runs local synthetic demo | B |
| `POST` | `/api/v1/market/quote-preview` | HyperAI Market | `PREVIEW` — computes local synthetic quote preview | B |
| `POST` | `/api/v1/market/quote-preview/scenario/{scenario_id}` | HyperAI Market | `PREVIEW` — computes named local quote scenario preview | — |
| `POST` | `/api/v1/market/supply-quote-preview` | HyperAI Market | `PREVIEW` — computes quote from selected local supply comparison | B, C, T |
| `POST` | `/api/v1/market/quote-approve` | HyperAI Market | `LOCAL_WRITE` — records synthetic quote approval in local market history | B, C, T |
| `POST` | `/api/v1/market/quote-complete` | HyperAI Market | `LOCAL_WRITE` — runs mock completion and records local receipt/history; no real execution or payment | B, C, T |
| `GET` | `/api/v1/market/deal-history` | HyperAI Market | `READ` — reads paginated local SQLite history | B, C, T |
| `POST` | `/api/v1/market/verify` | HyperAI Market | `READ` — verifies caller-supplied local receipt | B |
| `POST` | `/api/v1/market/registry-capture/verify` | Simple Compute Market adapter | `READ` — verifies caller-supplied offline capture | B |
| `POST` | `/api/v1/inference/verify-bundle` | HyperIntel adapter | `READ` — verifies caller-supplied bundle offline | B |
| `POST` | `/api/v1/inference/reconcile-execution` | HyperIntel adapter | `READ` — reconciles caller-supplied execution evidence offline | B |
| `GET` | `/api/v1/fleet/inventory` | HyperAI Fleet | `READ` — reads synthetic/local Fleet inventory | B, C, T |
| `GET` | `/api/v1/fleet/doctor` | HyperAI Fleet | `READ` — reads local Fleet diagnostic report | B, C, T |
| `POST` | `/api/v1/fleet/operation-preview` | HyperAI Fleet | `PREVIEW` — advisory operation preview; no resource is contacted or reserved | B, C, T |
| `GET` | `/api/v1/fleet/simulation` | HyperAI Fleet | `READ` — reads process-held simulation state | B, C, T |
| `POST` | `/api/v1/fleet/simulation/transition` | HyperAI Fleet | `LOCAL_WRITE` — advances process-held simulation state; no remote Fleet action | B, C, T |
| `GET` | `/api/v1/eval/catalog` | HyperAI Eval | `READ` — reads fixed evaluation catalog | B, C, T |
| `POST` | `/api/v1/eval/run` | HyperAI Eval | `LOCAL_EXEC` — bounded local synthetic scorer; caches reports in process memory | B, C, T |
| `GET` | `/api/v1/trade/catalog` | HyperAI Trade | `READ` — reads local synthetic instrument catalog | B, C, T |
| `GET` | `/api/v1/trade/series/{instrument_id}` | HyperAI Trade | `READ` — reads synthetic series | B, C, T |
| `POST` | `/api/v1/trade/scenario` | HyperAI Trade | `PREVIEW` — computes local scenario report | B, C, T |
| `POST` | `/api/v1/trade/paper` | HyperAI Trade | `PREVIEW` — computes local paper replay; no brokerage call | B, C, T |
| `GET` | `/api/v1/data/catalog` | HyperAI Data | `READ` — reads local dataset catalog | B, C, T |
| `GET` | `/api/v1/data/models` | HyperAI Data | `READ` — reads locally saved model definitions | B, C, T |
| `GET` | `/api/v1/data/views` | HyperAI Data | `READ` — reads locally saved view definitions | B, C, T |
| `GET` | `/api/v1/data/describe/{dataset_id}` | HyperAI Data | `READ` — reads local dataset manifest | C |
| `GET` | `/api/v1/data/synthetic-release` | HyperAI Data | `READ` — reads bounded synthetic release | B |
| `GET` | `/api/v1/data/synthetic/preview` | HyperAI Data | `PREVIEW` — previews synthetic local seed | B, C, T |
| `POST` | `/api/v1/data/synthetic/seed` | HyperAI Data | `LOCAL_WRITE` — seeds local synthetic snapshot using preview digest and idempotency key | B, C, T |
| `POST` | `/api/v1/data/gpudiff/refresh` | HyperAI Data | `REMOTE_READ_LOCAL_WRITE` — performs one fixed HTTPS GET to GPUDiff, then stores source bytes, normalized snapshot, and receipt locally; 10-second timeout, no redirect/retry, 4 MiB ceiling | B |
| `GET` | `/api/v1/data/gpudiff/schedule` | HyperAI Data | `READ` — returns in-memory status for the owner-controlled six-hour GPUDiff schedule | B |
| `POST` | `/api/v1/data/gpudiff/schedule` | HyperAI Data | `REMOTE_READ_LOCAL_WRITE` — starts a fixed six-hour schedule with one immediate GPUDiff GET, then bounded local snapshot and receipt writes | B |
| `POST` | `/api/v1/data/gpudiff/schedule/stop` | HyperAI Data | `LOCAL_WRITE` — stops the in-process schedule after any bounded in-flight source request | B |
| `POST` | `/api/v1/data/query` | HyperAI Data | `READ` — runs local DataFusion query in-process | B, C, T; also scoped tool route |
| `POST` | `/api/v1/data/query/{query_id}/cancel` | HyperAI Data | `LOCAL_WRITE` — requests cancellation of an active in-process query (transient state) | B |
| `POST` | `/api/v1/data/models/preview` | HyperAI Data | `PREVIEW` — previews local model definition | B, C |
| `POST` | `/api/v1/data/models/save` | HyperAI Data | `LOCAL_WRITE` — saves local model definition after preview/digest checks | B, C |
| `POST` | `/api/v1/data/views/preview` | HyperAI Data | `PREVIEW` — previews local view definition | B, C |
| `POST` | `/api/v1/data/views/save` | HyperAI Data | `LOCAL_WRITE` — saves local view definition after preview/digest checks | B, C |

## Scoped agent-tool routes

These 15 routes are created from `AgentToolRoute::ALL`; each path and method is
matched exactly (with the declared path parameter shape). MCP uses the scoped
capability for these routes. ACP surfaces the corresponding bundled MCP tools.
All tool effects remain within the endpoint behavior described below.

| Method | Path | Owner | Effect | Surface |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/agent/tools/data/catalog` | HyperAI Data | `READ` — reads local dataset catalog | MCP, ACP |
| `POST` | `/api/v1/agent/tools/data/query` | HyperAI Data | `READ` — runs read-only local DataFusion query | MCP, ACP |
| `GET` | `/api/v1/agent/tools/fleet/inventory` | HyperAI Fleet | `READ` — reads local inventory | MCP, ACP |
| `GET` | `/api/v1/agent/tools/fleet/doctor` | HyperAI Fleet | `READ` — reads local diagnostic report | MCP, ACP |
| `POST` | `/api/v1/agent/tools/fleet/operation-preview` | HyperAI Fleet | `PREVIEW` — computes advisory local preview | MCP, ACP |
| `GET` | `/api/v1/agent/tools/eval/catalog` | HyperAI Eval | `READ` — reads evaluation catalog | MCP, ACP |
| `GET` | `/api/v1/agent/tools/eval/reports` | HyperAI Eval | `READ` — reads process-held local reports | MCP, ACP |
| `GET` | `/api/v1/agent/tools/market/workload-trace` | HyperAI Market | `PREVIEW` — reads/composes local workload trace | MCP, ACP |
| `GET` | `/api/v1/agent/tools/market/workload-trace/failure-preview` | HyperAI Market | `PREVIEW` — returns fixed failure preview | MCP, ACP |
| `GET` | `/api/v1/agent/tools/market/workload-trace/inconclusive-preview` | HyperAI Market | `PREVIEW` — returns fixed inconclusive preview | MCP, ACP |
| `GET` | `/api/v1/agent/tools/trade/catalog` | HyperAI Trade | `READ` — reads synthetic instrument catalog | MCP, ACP |
| `GET` | `/api/v1/agent/tools/trade/series/{instrument_id}` | HyperAI Trade | `READ` — reads synthetic series | MCP, ACP |
| `POST` | `/api/v1/agent/tools/trade/scenario` | HyperAI Trade | `PREVIEW` — computes local scenario report | MCP, ACP |
| `GET` | `/api/v1/agent/tools/market/supply-comparison` | HyperAI Market | `READ` — reads local supply comparison | MCP, ACP |
| `POST` | `/api/v1/agent/tools/market/supply-quote-preview` | HyperAI Market | `PREVIEW` — computes local quote preview | MCP, ACP |

## Client wiring and compatibility status

The source call-site crosswalk shows one loopback service API with multiple
clients, plus intentional local paths that do not use HTTP:

| Surface | Wiring observed in source | Limits of this inventory |
| --- | --- | --- |
| Browser | Same-origin `requestJson` calls with session/CSRF behavior; includes manual GPUDiff import and owner-only schedule start/stop/status controls | Source call sites are enumerated above; this does not exercise browser behavior or every response/error state |
| CLI | Reads the owner bearer file and calls the loopback service for Data, Fleet, Eval, Trade, Market history/quote lifecycle, agent control, connection inventory, and shutdown; workload-trace composition also calls `hyperai-application` directly | It is not a route-for-route wrapper; direct-application trace commands do not prove HTTP compatibility |
| TUI | Reads the owner bearer and calls the same loopback service for its Data, Fleet, Eval, Trade, Market, history, and agent workflows | It does not expose every browser/API route; call-site presence does not prove schema/error parity |
| MCP | Uses the separate capability and only the 15 allowlisted tool routes | No approval, completion, save, transition, or agent-session-control authority |
| ACP | Starts/uses the bundled MCP bridge and its tool surface | ACP tool parity is the MCP allowlist, not the full owner API |

Selected client packets document bounded local lifecycles (including browser,
CLI, and TUI paths), but they do not constitute a complete route-by-route
schema, denial, stale-digest, unsupported-version, or normalized-error parity
suite. The source-visible route rows above are not runtime compatibility
evidence by themselves. The bounded executable checks and their exact limits
are listed below. No machine-readable route/schema manifest or shared typed
HTTP client is established by this slice. Keep Stage 1 open for
request/response schema-version mapping, stable error mapping, and complete
route/client negative-case coverage. Do not widen the MCP/ACP allowlist to
close a client coverage gap.

## Executed checks and limits

At the frozen baseline, the exact route-set comparison passed for all 8
static/health rows, all 50 direct API rows, and all 15 agent-tool rows. The
GPUDiff continuation adds one direct owner route, records it in the owner-route
authorization test, and leaves the 15-route agent allowlist unchanged. The
baseline source hashes below are historical and predate this additive route.

`corepack pnpm run verify:interface` passed JavaScript syntax checks and 34
Node tests (`34 passed, 0 failed`). The locked/offline Rust package run below
passed all executed tests in the HyperAI local stack, adapter, interface, and
CLI packages:

```sh
sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p hyperai-foundation -p hyperai-market -p hyperai-acp \
  -p hyperai-application -p hyperai-data -p hyperai-eval \
  -p hyperai-fleet -p hyperai-hyperintel-adapter \
  -p hyperai-local-service -p hyperai-mcp -p hyperai-scm-adapter \
  -p hyperai-trade -p hyperai-tui -p outcomes-cli
```

Three existing tests were ignored by their own prerequisites: one browser
reconciliation test requires a built UI binary and local Chrome/Chromium; one
service/ACP test requires staged sidecar binaries; one source-install test
installs locked binaries into a disposable root. Thus these checks cover
route-policy exactness, owner-route denial, selected API contracts, MCP/ACP
capability behavior, and selected CLI/TUI/browser component paths. They do not
run the browser, prove every client/route/schema/error combination, run the
root `lint` or `test:focused` gates, or establish native platform, independent
review, provider, or production readiness. The imported-product verifier
passed separately for all three recorded product snapshots; that result is
snapshot-integrity evidence only.

## Evidence and frozen inputs

The route and effect map was read from these exact files at the frozen source
revision. SHA-256 digests bind this inventory's source snapshot:

| File | SHA-256 |
| --- | --- |
| `crates/hyperai-local-service/src/server.rs` | `0e42d27e5a58d82bba04296740bc3e35411f640f1ddf78122b66b816a1562602` |
| `crates/hyperai-local-service/src/route_policy.rs` | `294763c1c5f45044da17dc02681600c3a039395969e57743e1647f45af85032c` |
| `crates/hyperai-local-service/src/auth.rs` | `2db8388d29b0f3e5e7951d7c9ffdb8b7c56451b5941a7b502cb51f8fc19f7af1` |
| `crates/hyperai-application/src/lib.rs` | `cc0417bd98139560c907f13dd77695febad59e6ba910ca0dbbc96bc4798cfe24` |
| `crates/hyperai-market/src/lib.rs` | `9c44ee40323072c9ffd36e2a60124247c07ab471b893102a0b88b3f33abf9e93` |
| `crates/outcomes-cli/src/lib.rs` | `a8989acf5ca479363ba6d63eda60002826c43e1346c4bf09d471e7ee081250bc` |
| `crates/hyperai-tui/src/lib.rs` | `41a44749ed6e136a72d8554d6dbf99e4ed506527a75bcc2933e56968883b1a94` |
| `crates/hyperai-mcp/src/lib.rs` | `a59b65680dd126a98a3ed830c03199521fd8a28650c259e1bf1c03eca2cabf5f` |
| `crates/hyperai-acp/src/tools.rs` | `01b748d8a621ab0eec42086c51e97a2bd28d5becb372bb473759c817f4596028` |
| `apps/hyperai-interface/app.js` | `9764756b31e215e545d88ed60aa91e2ea75a281ebe1fa4601413c88061cb2ad1` |

The root source inventory does not change the nested product import boundaries.
The pre-edit imported-product verification passed with the recorded SCM,
HyperIntel, and JustAI source digests; it is an integrity check only, not a
client or product acceptance result. The separate [interface plan](bazaar-hyperai-interface-plan.md)
and [local service route policy packet](compute-bazaar-local-service-route-policy-packet.md)
remain authoritative for their existing scoped work.

## Next blocked gate

Complete exact call-site/schema/version/error cross-checks, then add bounded
compatibility checks for route registration versus ordinary clients and the
15-route MCP/ACP capability. Include denied access, stale digest, malformed or
unsupported schema, and failure behavior. Preserve the current route and
capability bytes until a separate implementation slice is defined. Data worker
isolation remains `NOT_ASSIGNED`; DataFusion is currently in-process.
