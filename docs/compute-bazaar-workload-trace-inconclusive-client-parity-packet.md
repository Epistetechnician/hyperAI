# Inconclusive Workload Trace Client Parity Packet

Status: completed bounded local synthetic client integration; `INTERNAL_PASS`
for the local proof gates recorded below.

State slice: `hyperai.bazaar.workload_trace_inconclusive_client_parity.v1`.
Parent: [Rust port subplan](compute-bazaar-rust-port-subplan.md).
Related contracts: [inconclusive trace](compute-bazaar-workload-inconclusive-packet.md),
[seller-failure client parity](compute-bazaar-workload-trace-seller-failure-clients-packet.md),
and [ACP Phase 8](compute-bazaar-phase-8-acp-packet.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the preserved dirty and untracked worktree. These exact working-tree source
bytes bind the implementation target; the Git revision alone is insufficient.

## Objective and contract

Expose the existing fixed synthetic inconclusive-verifier / mock
hold-for-dispute `WorkloadTraceV1` consistently through the CLI, TUI, and
read-only ACP/MCP clients. The owner route already exists at
`GET /api/v1/market/workload-trace/inconclusive-preview`; reuse its existing
application function and preserve its response bytes and claim ceiling.

- Add opt-in `hyperai market trace --inconclusive-preview`. The existing
  default and `--seller-failure-preview` output remain unchanged. The two
  preview flags conflict and cannot select two projections at once.
- Add the TUI `i` control for the fixed inconclusive preview; retain `f` for
  seller failure and `r` for the default trace. Clear the previous projection
  before loading a new one. Render the inconclusive verdict and mock
  dispute-hold status without implying an actual verifier decision, dispute,
  funds hold, or payment.
- Add exactly one scoped agent alias,
  `GET /api/v1/agent/tools/market/workload-trace/inconclusive-preview`, mapped
  to the existing fixed preview handler. The scoped capability must not gain
  access to the owner route or any other preview route.
- Add append-only `MCP_TOOL_NAMES_V4`, equal to V3 plus exactly
  `hyperai_market_workload_trace_inconclusive_preview`. Keep V1, V2, and V3
  arrays byte-for-byte unchanged and keep ACP protocol v1. The MCP tool takes
  empty input and performs only the fixed GET above.

This adds no durable record, caller-selected data, dispute mechanism, verifier,
execution, or settlement behavior. The existing browser and owner route remain
unchanged.

## Frozen source files

Verify every size and SHA-256 before the first code edit. If any source differs,
stop and freeze a new packet before editing.

| File | Bytes | SHA-256 |
|---|---:|---|
| `crates/outcomes-cli/src/lib.rs` | 146,354 | `3cac92010eb3f20fd3f2b0778182804b9128e415064dc863c747c0ddc76414c1` |
| `crates/outcomes-cli/tests/cli_compat.rs` | 58,428 | `20aa56550a5ea6213ba6d5880cac544f9b9021de54d57bb68581f475a77318ce` |
| `crates/hyperai-tui/src/lib.rs` | 89,747 | `885aac5e13eee79998b743c622a8fc9232d4fccd1d6f421dec433f343aaa8dfa` |
| `crates/hyperai-tui/tests/terminal_restore.rs` | 27,487 | `c1fe2d4dd021e96a162c9b4aba737520ee4240afeb888458dea9f32f54d04c55` |
| `crates/hyperai-acp/src/lib.rs` | 585 | `85af4799c1f06317c5a2d3c275201d798340d99a06316a06ea9feb6465ec6b73` |
| `crates/hyperai-acp/src/tools.rs` | 4,971 | `7ac1b411282ab5fe03f3139b76ec6fbd89485fccf96e9d0b1c740b26ea6c78b6` |
| `crates/hyperai-acp/src/session.rs` | 38,079 | `26e7492b4445c37611d7575128a134bcb4c74226fe9693d6c0a5978c49e68036` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | 40,799 | `0a2401b9d07e492aac398233d791a918b38a62ec8e4e125a4b6b15da5acdb556` |
| `crates/hyperai-acp/tests/synthetic_peer.rs` | 21,092 | `9e8b7d121ae332322e8a8837c7df77c2aaaae5ffdda9320b2431848a4ea550b2` |
| `crates/hyperai-mcp/src/lib.rs` | 31,151 | `73daab3f0e330d66518930d263257072fb74a540bd7aa35ba258f52f398a1022` |
| `crates/hyperai-local-service/src/server.rs` | 80,499 | `de1d5b6486110b6ed1859698ecee8db7e76149933d8ad84a1f5abca99e4fb950` |
| `crates/hyperai-local-service/tests/agent_api.rs` | 27,758 | `3d6be03836ffdb23ad2564173a682ad25820f072d1e6c14f0528f6a5713add6e` |

## Exact writable files

- `crates/outcomes-cli/src/lib.rs` and
  `crates/outcomes-cli/tests/cli_compat.rs`: add the mutually exclusive
  inconclusive-preview option and compare its JSON with the owner route.
- `crates/hyperai-tui/src/lib.rs` and
  `crates/hyperai-tui/tests/terminal_restore.rs`: add the fixed `i` route,
  correct stale-result clearing, and claim-bounded wide/narrow rendering.
- `crates/hyperai-acp/src/lib.rs`, `crates/hyperai-acp/src/tools.rs`,
  `crates/hyperai-acp/src/session.rs`,
  `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`, and
  `crates/hyperai-acp/tests/synthetic_peer.rs`: append V4 while preserving
  V1–V3, and verify the same read-only tool through ACP.
- `crates/hyperai-mcp/src/lib.rs`: serve the empty-input V4 tool and compare
  its structured result with the owner route.
- `crates/hyperai-local-service/src/server.rs` and
  `crates/hyperai-local-service/tests/agent_api.rs`: add the exact GET alias,
  capability allowlist entry, authorization assertions, and route parity.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record the scope and results.

## Allowed effects and stop conditions

Allowed: reconstruct the existing HyperAI-authored fixed synthetic trace;
make local loopback requests only to test-owned services bound to port `0`;
render it in the TUI; expose it as one empty-input, read-only MCP tool; and run
the focused local tests and root validation listed below. Use disposable test
homes, stop only test-owned services, and remove test-owned artifacts. Preserve
the existing service on port `4317`.

Stop if the slice would require changing the existing owner response, reading
user or upstream data, contacting a provider or seller, running an agent/model,
enabling another write route, creating a durable dispute or settlement record,
using credentials outside test-owned capabilities, accessing a chain, or
publishing/deploying anything. Those effects require their own state slices
and authority.

## Proof gates

1. The CLI inconclusive-preview JSON equals the owner-route JSON; the default
   and seller-failure outputs remain unchanged; selecting both preview flags
   is rejected.
2. TUI `i`, `f`, and `r` select only their exact fixed routes. Success and
   request-failure cases show no stale prior preview. Wide and narrow terminal
   renders distinguish an inconclusive mock verdict from the dispute-hold
   projection and retain the no-real-decision/no-funds claim limits.
3. The scoped agent capability can GET the new alias and receives the exact
   owner-route projection. It cannot GET the owner route or other preview
   routes, and POST is denied on the new alias.
4. MCP V4 returns that same projection using empty input. V1/V2/V3 arrays are
   unchanged, V4 has V3 as its exact prefix and one appended tool, and the ACP
   fixture exercises it without widening the protocol or capability.
5. Run focused tests for the changed CLI, TUI, local-service, ACP, and MCP
   crates; pinned formatting; `python3 scripts/verify_imports.py`;
   `corepack pnpm run lint`; and `git diff --check`. Confirm test-owned
   processes and homes are cleaned up and port `4317` was not contacted.

Claim ceiling: local cross-client projection parity for this exact fixed
synthetic inconclusive-verifier/mock-dispute-hold trace. It proves no actual
verifier decision, dispute, funds hold, payment entitlement, computation,
independent review, accessibility acceptance, native platform support, or
production readiness. Expected assurance is `INTERNAL_PASS` only for the exact
local gates above; reviewer status is `NOT_REQUESTED` and reproduction is
`NOT_REPRODUCED`. Data and artifact custody is checked-out HyperAI source and
removed HyperAI-authored test fixtures. Publish boundary: local source and
documentation only; no commit, PR, release, external circulation, or
deployment.

## Execution result (2026-10-01)

The CLI, TUI, scoped service route, ACP fixture, and MCP bridge now expose the
same fixed synthetic inconclusive-verifier/mock-dispute-hold projection. The
CLI flag is opt-in and conflicts with the seller-failure flag. The TUI `i`,
`f`, and `r` keys select the three fixed routes, clear stale results before a
request, wrap narrow-pane text, and label the mock verdict and dispute-hold
projection without asserting a real decision or funds hold. The scoped alias
reuses the existing owner handler; V4 appends only the inconclusive MCP tool.
ACP stays protocol v1, and V1/V2/V3 tool arrays remain unchanged.

Focused results: CLI integration 13 passed and 2 ignored (binary staging or
disposable install prerequisites); TUI 10 passed; local-service library 11
passed and API 10 passed; ACP synthetic peer 9 passed; MCP 8 passed. Pinned
format checking, `python3 scripts/verify_imports.py`, and the full
`corepack pnpm run lint` gate passed. The import check matched Simple Compute
Market `a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`, HyperIntel
`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`, and JustAI
`9a0e56536b968cbb2cb7173e142068b53c4f9fe3` at their recorded digests.
`git diff --check` and the scoped source trailing-whitespace scan returned no
diagnostics. The process check found no remaining HyperAI/MCP/ACP fixture
process; the port-4317 listener check was empty. Test-owned services used
ephemeral ports and disposable homes; no request was sent to port 4317.

Assurance is `INTERNAL_PASS` for this exact local synthetic client-parity
slice. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Source/data custody remained in the checked-out HyperAI
workspace and disposable HyperAI-authored test fixtures. No Bazaar source or
data was copied or read. This does not establish independent acceptance,
accessibility or platform qualification, actual computation, live dispute,
funds movement, payment entitlement, paid execution, or production readiness.
The overall Data qualification remains `NOT_ASSIGNED`; live provider, Fleet,
paid Eval, financial Trade, chain, and production gates remain separately
authorized work.

Final source SHA-256 values (the target is the post-edit working tree above
the unchanged baseline Git revision):

| File | SHA-256 |
|---|---|
| `crates/outcomes-cli/src/lib.rs` | `04d2efbc5cf4de554a048a9d5a6bf274bd49bc4a59ef1fcf021e974632474864` |
| `crates/outcomes-cli/tests/cli_compat.rs` | `8909939a0d92b56b015f3b26f20fa93151449e60aee946427808c3722a4a60f2` |
| `crates/hyperai-tui/src/lib.rs` | `236e5142e137be31c02e2029291907bbffa65b31c6753b4832cb0b44a98ca803` |
| `crates/hyperai-tui/tests/terminal_restore.rs` | `2e6c2524b7470810d30289f1ffe989469f2cd7c4c8c582fd22d97f5cf08fc249` |
| `crates/hyperai-acp/src/lib.rs` | `e689a71031b44733d339bd0f32de17bd7b44d9ca7f20eb8d33d80042391665ff` |
| `crates/hyperai-acp/src/tools.rs` | `e8406f28137afeb26854a77758a09469dae77868c49feb72fc8753c074aac1ac` |
| `crates/hyperai-acp/src/session.rs` | `8cf6488518039de57fa52c7f94148875014abc8f40feb96b8a23b7ebed5ba5c5` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | `3a29e5fcf1455f3d9892aef6072fbe3923a93ed09d3610521add627b00bc03ee` |
| `crates/hyperai-acp/tests/synthetic_peer.rs` | `dc886c9f4bdd886b7c7636d3d6b6c0c41b1c0701751a07fd2104a9c8d8ccfc6c` |
| `crates/hyperai-mcp/src/lib.rs` | `ac7da19dcda0f252fbd91329d18f67dd00cbea458f8d08ce0627ca6204bc5633` |
| `crates/hyperai-local-service/src/server.rs` | `8b40261e216cc9c287833969e983fc472226bb3800d4409dd334950258188d20` |
| `crates/hyperai-local-service/tests/agent_api.rs` | `a5c483395ed527c73ef9d8aba6c6d3f8f89d140970db5fcaec2ca19a20402bde` |
