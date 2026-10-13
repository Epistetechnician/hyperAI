# Phase 8 ACP Integration Contract Packet

Planning slice: `hyperai.bazaar.acp_packet.v1`.
Proposed implementation slice: `hyperai.bazaar.acp.v1`.
Parent: [HyperAI Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Status and purpose

This packet scopes the final local interface phase for connecting a user-
selected ACP agent to HyperAI's existing application facade. It is a planning
artifact that is followed by the separately accepted
`hyperai.bazaar.acp.v1` implementation slice below. It does not authorize
starting a third-party agent, contacting a model/provider, forwarding real
user or market data, or enabling any Fleet, Eval, Trade, market, outcome,
custody, or payment action.

The intent is one ACP client integrated with the existing local service,
browser shell, CLI, and TUI. ACP is a presentation and request protocol; the
application facade remains the authority for data access and action policy.
The agent is never a policy engine, verifier, or approver.

## Bounded task list

1. Add an isolated `hyperai-acp` adapter that negotiates ACP v1, manages one
   volatile session, streams bounded updates, and cancels/disconnects cleanly.
2. Expose a fixed, typed, read-only HyperAI tool set through the shared facade:
   bounded Data catalog/query, Fleet inventory and preflight, Eval catalog and
   reports, and deterministic Trade research scenarios.
3. Connect the same typed operations to the browser, CLI, and TUI. Show the
   selected profile, negotiated protocol and capabilities, exact available
   tools, data scope, session state, and cancellation status.
4. Exercise the protocol with a HyperAI-authored synthetic peer and fixtures.
   Do not use a third-party agent or send a prompt to a model/provider in this
   implementation slice.

## Frozen protocol and dependency choices

- Use the official `agent-client-protocol` Rust crate at `=2.2.0`; its
  published manifest reports Apache-2.0 and Rust 1.88 MSRV, compatible with
  HyperAI's pinned Rust 1.93.1 toolchain. Its resolved schema dependency is
  `agent-client-protocol-schema = 1.9.1`; preserve the exact result in
  `Cargo.lock` and build with `--locked`.
- Negotiate `ProtocolVersion::V1` only. Do not enable
  `unstable_protocol_v2` or `unstable_mcp_over_acp`.
- Provide the typed read-only tools as a HyperAI-owned MCP stdio server named
  in the stable-v1 `session/new` request. This is the standard ACP v1
  client-provided MCP-server mechanism; do not use the SDK's experimental
  native MCP-over-ACP transport. Pin `agent-client-protocol-rmcp = 3.1.1`
  with default features disabled. Its published manifest declares the
  `rmcp` dependency with Cargo version requirement `2.1.0` and the
  `server`, `transport-io`, and `schemars` features. The current root
  `Cargo.lock` resolves that compatible requirement to `rmcp 2.2.0`; preserve
  the exact locked version and checksum, and do not add a direct `rmcp`
  dependency just to force the manifest's minimum version.
- Keep ACP types behind `hyperai-acp`; domain and UI crates must not depend on
  SDK wire types. The MCP adapter invokes application-facade methods and
  never accesses SQLite or product workspaces directly.

The official [ACP v1 overview](https://agentclientprotocol.com/protocol/v1/overview)
defines the client as controlling resource access and user interactions. The
[session setup contract](https://agentclientprotocol.com/protocol/v1/session-setup)
defines capability negotiation before session creation and supports client-
provided MCP servers. The [Rust SDK](https://docs.rs/agent-client-protocol/2.2.0/agent_client_protocol/)
documents stable-v1 client/session APIs. These sources inform the design; they
do not grant authority to run a real agent.

## Agent trust and process boundary

ACP restricts requests mediated by HyperAI. It does not sandbox the selected
agent executable or stop that process from using its own filesystem, tools,
network, or credentials. A profile marked `read-only` means only that
HyperAI's exposed tools are read-only; it is not a claim about the process as
a whole. Before a real profile can connect, the UI must identify the exact
executable and requested scope and state that the executable may have
independent capabilities outside HyperAI.

The `session/new` MCP configuration carries a random, per-session capability
that can call only `/api/v1/agent/tools/*` read-only routes. It is distinct
from the owner's full service bearer, remains in volatile memory, and is
revoked on disconnect. The selected agent necessarily receives this limited
capability as part of ACP's MCP-server configuration; it may use those
read-only routes directly or inspect its child MCP server. Never pass the
owner bearer, persist the scoped token, or log it. Show this access clearly
before connection. This is capability scoping, not OS process isolation.

The implementation must not use a shell, interpolate command strings, or
inherit the service's full environment. Profile commands use an absolute
executable path plus separately stored argument values. Launch with an empty
environment and a small explicit non-secret allowlist, bound stderr, and
terminate only the process tree owned by the session. Do not store secret
environment values in a profile. This limits accidental inheritance; it does
not create an OS sandbox. The actual third-party profile must remain disabled
for local qualification. Tests use only the local synthetic ACP peer.

## Platform scope

The Unix supervisor owns the child and descendants through a process group.
Windows process ownership is implemented behind the optional, default-off
`windows-acp` feature using a Windows
[Job Object](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
The adapter creates the job before the child, creates the child suspended,
assigns it to the job using
[`AssignProcessToJobObject`](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject),
and resumes it only after assignment succeeds. The job uses kill-on-close
without breakaway, and shutdown checks the owned job's process count after
bounded termination. This is process-tree ownership, not an OS sandbox or
evidence that a computation is correct.

The `windows-acp` feature is not enabled by default or by product
configuration. Builds for other unsupported targets fail closed with
`ACP_PLATFORM_UNSUPPORTED`; a Windows build with the feature disabled also
fails closed before starting the selected executable. Cross-compilation and
static checks do not qualify Windows runtime behavior. Native Windows tests
must cover connect, disconnect, failed handshake, cancellation, parent and
descendant termination, and cleanup after supervisor-task abort before any
product configuration enables the feature. Windows service, CLI, and TUI
compatibility does not imply ACP support. Current runtime evidence remains
limited to the tested Unix host and does not establish support on every Unix
platform.

The Windows-only implementation uses the checksum-locked MIT package
`subc-jobobject = 0.1.1` as an optional target dependency. The feature is
named `windows-acp` and remains disabled by default; no product crate enables
it. Its published package metadata does not state an MSRV or repository URL.
The adapter uses the lower-level create-job, create-suspended, assign, resume,
terminate, and process-count APIs. The convenience spawn helper is not used
because it creates the job after the child has already been spawned. Package
identity and checksum are recorded in [`docs/provenance.md`](provenance.md).

Local qualification on 2026-09-28 passed the Unix ACP package suite (18 tests),
host strict Clippy, formatting, and strict Clippy plus locked all-target
cross-checks for `x86_64-pc-windows-msvc` with `windows-acp` both disabled and
enabled. These checks provide `INTERNAL_PASS` evidence for the tested Unix
behavior and source compilation/static analysis only. No Windows binary was
executed; native connect, disconnect, cancellation, crash, descendant cleanup,
and supervisor-abort behavior remain unassessed. The `windows-acp` feature
must remain disabled in product configuration until that native gate passes.

The 2026-09-28 root workspace lint also passed on the corrected current source.
The exact locked offline source-install-to-ACP integration passed after its
synthetic peer's workload-trace guard was aligned with the current mock-verdict
and mock-disposition statuses. Its initial failing observation was a stale
test-fixture expectation; the service's workload-trace contract was unchanged.
See [`hyperai.bazaar.local_install_acp_bundle.v1`](state-slices.md#hyperai-bazaar-local_install_acp_bundlev1)
and [`hyperai.bazaar.acp_fixture_trace_contract.v1`](state-slices.md#hyperai-bazaar-acp_fixture_trace_contractv1).

Installed Workload Trace Data-state follow-up (2026-09-28): the same ignored
source-install integration now exercises an actual local filesystem
`PermissionDenied` for the fixed synthetic JSON source. The owner route, the
session-scoped ACP route, and the installed MCP sidecar agree on `denied`,
with no Data digest or buyer-intent Data reference. Permission is restored
before the follow-on integrity-conflict control. That control revealed an
obsolete test-only HTTP 409 expectation; it now checks the current HTTP 200
trace projection with Data status `conflicting`, no Data digest, and no intent
Data reference. No service, product, authentication, capability, or MCP tool
contract changed. The exact integration and full root lint pass on pinned Rust
1.93.1; the listener on `127.0.0.1:4317` remains untouched. See the [denied
parity packet](compute-bazaar-workload-trace-acp-denied-parity-packet.md), the
[conflict contract packet](compute-bazaar-workload-trace-acp-conflict-contract-packet.md),
and their named records in [state-slices.md](state-slices.md).

Linux cleanup checks distinguish a running process from a zombie. A terminated
orphan may remain visible in `/proc` until its parent or a subreaper waits for
it; Linux `wait(2)` documents this zombie state and reparenting behavior. The
synthetic Linux/amd64 test observed a killed descendant in state `Z`, reparented
to container PID 1 while still in the peer's process group. The supervisor had
terminated the descendant; the old `kill(pid, 0)` assertion incorrectly treated
the zombie as running. The test now treats `Z` and `X` as exited, and the same
process test passes with Docker `--init`. This does not establish that a
container running HyperAI directly as PID 1 will reap orphaned descendants;
such a deployment must provide a reaping init or a separately scoped process
supervisor. See [Linux wait(2)](https://man7.org/linux/man-pages/man2/waitpid.2.html),
[Linux child subreapers](https://www.man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html),
and [Docker init guidance](https://docs.docker.com/engine/containers/multi-service_container/).

The ACP session receives no filesystem or terminal capability. Its standard
MCP stdio server exposes only these operations:

- list and describe the fixed local Data catalog, then run the existing
  bounded read-only query contract against synthetic data;
- inspect fixed Fleet inventory, health, and advisory operation previews;
- read the fixed local Eval catalog and aggregate reports; and
- inspect Trade instruments/series and calculate bounded research or paper
  scenarios over the synthetic Data release.

Do not expose quote approval/completion, market fulfillment, arbitrary SQL
outside the existing query validator, Eval execution, Fleet transitions,
file access, terminal execution, credentials, provider routing, inference
execution, outcome resolution, storage writes, funds, or settlement. A tool
call returns only the typed facade result with its source, provenance,
unknowns, claim ceiling, and nonclaims. No tool call can authorize another
tool or widen a session's allowlist.

## Session and data contract

- A user explicitly selects a named profile and initiates each connection;
  there is no auto-start, reconnect, or background prompt delivery.
- Keep profiles to executable identity, argument values, and approved
  non-secret environment names/values. Session state is volatile. Never store
  raw prompts, model traces, full tool payloads, credentials, or private
  artifacts in the HyperAI metadata database or ordinary logs. The active
  snapshot may expose bounded assistant text for local display; this is
  volatile session content, not audit metadata or evidence. A confirmed
  disconnect clears the manager-held snapshot; its final response may still
  deliver the bounded text to the requesting local UI in memory. Tool raw
  input/output, thought updates, and stderr are discarded; only sanitized
  lifecycle error categories are returned.
- Use the existing authenticated IPv4-loopback service as the client
  transport for browser/CLI/TUI session control. Check the caller's local
  session on every connect, read, cancel, and disconnect request.
- Bind every session to a profile revision, ACP protocol version, negotiated
  capabilities, fixed tool-set version, data scope, and a random local session
  ID. A changed profile invalidates the session. Any durable record may retain
  only sanitized lifecycle metadata and content digests; do not claim a
  complete audit log. The bounded assistant-text buffer exists only in the
  active volatile snapshot and is not a durable record. A caller may retain
  the final disconnect response in its own volatile view until it clears or
  closes that view.
- Bound prompt and update sizes, concurrent sessions, event retention,
  aggregate ACP stdout to 8 MiB per session, stderr to 64 KiB, per-tool
  duration, query resources, and result bytes. A disconnect or
  timeout produces an explicit `inconclusive`/`cancelled` terminal state;
  it never implies that an external agent stopped or undid its own effects.
- The user may cancel the active ACP request and disconnect the owned child.
  The UI must show when process termination is unknown. Do not report a
  successful cancellation until the local process exit is confirmed.

## Proposed implementation file map

The later accepted implementation slice should be limited to:

| Files | Purpose |
|---|---|
| `Cargo.toml`, `Cargo.lock` | Add the pinned ACP SDK, MCP bridge dependencies, and `hyperai-acp` / `hyperai-mcp` workspace members; keep all experimental ACP features disabled. |
| `crates/hyperai-acp/Cargo.toml`, `src/lib.rs`, `src/profile.rs`, `src/session.rs`, `src/tools.rs`, `src/bin/acp-fixture-peer.rs`, and named unit/integration tests | Own ACP types, profile validation, v1 negotiation, MCP tool registration, bounded streaming, lifecycle, and the HyperAI-authored synthetic peer. |
| `crates/hyperai-mcp/Cargo.toml`, `src/lib.rs`, `src/main.rs`, and named tests | Own the stdio MCP server and its narrow authenticated calls through the HyperAI service facade, using the ACP crate's single fixed tool-name contract. Test-only local-service and temporary-home dependencies exercise its error adapter against the actual local server; no production dependency is added. |
| `crates/hyperai-application/Cargo.toml`, `src/lib.rs` | Provide the typed read-only agent facade and reuse existing module-owned limits. |
| `crates/hyperai-local-service/Cargo.toml`, `src/auth.rs`, `src/server.rs`, and new `tests/agent_api.rs` | Own one authenticated local ACP manager, short-lived read-only tool capability, and bounded connect/status/update/cancel/disconnect/tool routes. |
| `crates/outcomes-cli/Cargo.toml`, `src/lib.rs`, `tests/cli_compat.rs` | Add explicit `hyperai agent profile|connect|status|prompt|cancel|disconnect` commands over the same service. |
| `crates/hyperai-tui/Cargo.toml`, `src/lib.rs`, `tests/terminal_restore.rs` | Add the agent profile, prompt, cancel, disconnect, and session workspace while preserving terminal lifecycle behavior. Test-only local-service, temporary-home, and Tokio dependencies exercise error handling against the actual local server; no production dependency is added. |
| `apps/hyperai-interface/index.html`, `app.js`, `app.css` | Add the accessible Agents & Access workspace, profile disclosure, negotiated capability/tool display, bounded event view, and cancel/disconnect controls. |
| `README.md`, this packet, `docs/compute-bazaar-rust-port-subplan.md`, `docs/state-slices.md` | Document the interface and record exact scope/evidence. |

If implementation needs a file outside this map, amend the accepted state
slice before editing it. Do not modify `products/`, `references/`, HyperIntel,
JustAI, Simple Compute Market, outcome-resolution contracts, or any upstream
source tree.

## Acceptance and proof gates

1. **Protocol:** synthetic peer negotiates v1; unsupported/mismatched
   versions and capability combinations fail closed; protocol-v2 and native
   MCP-over-ACP features are absent; `session/new` includes only the fixed
   MCP stdio server; malformed requests, notifications, and JSON-RPC batches are
   bounded and produce the correct terminal/error status.
2. **Tools and policy:** only the listed tools are advertised; each maps to an
   existing typed facade call; malformed, extra, oversized, or unauthorized
   fields fail closed; mutation-shaped, path, shell, credential, and provider
   requests are denied; UI, CLI, and ACP return equivalent policy results.
3. **Streaming and lifecycle:** verify ordering, output/event caps, concurrent
   session caps, timeout, agent EOF, user cancellation, disconnect, process
   exit, and process-tree cleanup with the synthetic peer. A failed or
   ambiguous shutdown remains unknown.
4. **Privacy and storage:** prompts, tool contents, stderr, bearer tokens, and
   raw traces do not enter logs or persistent metadata. Verify that ACP does
   not add prompt/session tables to the metadata database. Only bounded
   redacted lifecycle metadata and digests may be retained durably; the
   bounded assistant-text display buffer is volatile session state, not an
   audit record. Confirmed disconnect clears the manager copy; a local client
   may still hold the final response in its volatile view.
5. **Interface:** verify keyboard-only access, focus restoration, assistive
   labels, reduced motion, narrow/tablet/desktop layouts, and complete
   loading, empty, disconnected, permission-denied, failed, cancelled, and
   inconclusive states.
6. **Validation:** run pinned-toolchain focused ACP/facade/service/CLI/TUI
   checks and `corepack pnpm run lint`; report root and nested workspaces
   separately. Exercise TUI and MCP stable-error handling against the actual
   local service on loopback with an isolated temporary home. The MCP test
   uses the temporary owner's bearer for its error-envelope check. Separately,
   run the ignored Unix CLI integration after `cargo build --workspace --bins`
   to qualify the profile-issued capability through the actual local service
   and HyperAI synthetic ACP peer. Seed only the fixed synthetic Data sample
   in the disposable service home, then require the peer to launch the exact
   packaged `hyperai-mcp` binary and use it over stdio. The peer must validate
   initialization, the active V3 list with preserved V1/V2 compatibility,
   successful Data and workload-trace tool calls including the fixed
   seller-failure preview, an owner-only Fleet route denial, and a denied Data
   call through the same sidecar after disconnect revokes the capability.
   Confirm clean sidecar exit and no capability echo. Do not start a
   third-party agent or contact a model/provider.

No `INDEPENDENT_ACCEPT` or production claim follows from these checks. The
local synthetic proof ceiling excludes sandboxing, provider/model behavior,
real-user data disclosure, external agent correctness, and all external
market, Fleet, Eval, Trade, settlement, or publication effects. A later
user-selected real-agent profile and any provider/data disclosure require a
separate exact authorization and, where needed, a real OS sandbox and owner
review.

Profile-issued capability and MCP stdio follow-up (2026-09-25): the dedicated
Unix integration test in `crates/outcomes-cli/tests/cli_compat.rs` passed after
`cargo build --locked --offline --workspace --bins`. It starts the packaged
`hyperai` service in a temporary home, seeds only the fixed synthetic Data
sample under the temporary owner credential, and connects only the HyperAI
fixture. The fixture reads the capability from the ACP `session/new` MCP
configuration, launches the packaged `hyperai-mcp` executable with only the
configured loopback endpoint and scoped capability, negotiates MCP over stdio,
checks the exact ten-tool allowlist, and calls the Data catalog tool to receive
the expected synthetic dataset. It also confirms an owner-only Fleet transition
is unauthorized. After disconnect revokes the capability, the still-running
sidecar repeats the Data tool call and receives an error without echoing the
capability; closing stdio lets the sidecar exit. A fixed result marker is
written only to the fixture's temporary working directory. This exercises the
HyperAI-owned MCP sidecar and synthetic peer, not a third-party MCP client or
agent. The explicit command is
`cargo test --locked --offline -p outcomes-cli --test cli_compat
service_issues_scoped_capability_to_synthetic_acp_profile -- --ignored
--nocapture`; it passed 1 test. The focused ACP and CLI packages passed with 9
ACP unit tests, 9 ACP integration tests, 26 CLI unit tests, 8 CLI compatibility
tests, 2 Eval API tests, and 1 market lifecycle test; this added test remains
ignored in the general suite because it requires staged workspace binaries.
`corepack pnpm run lint:fast` and `cargo fmt --all -- --check` passed. Assurance
is `INTERNAL_PASS` for this bounded local path only; reviewer status is
`NOT_REQUESTED`, reproducibility is `NOT_REPRODUCED`, and the temporary home,
capability, and fixture report were removed with the test directory.

Rejected-query parity follow-up (2026-09-25): the same ignored Unix
integration now sends a fixed rejected SQL query through the profile-issued
capability and packaged MCP sidecar before disconnect. The tool error preserves
HTTP 422 and `DATA_QUERY_REJECTED`, and contains neither the SQL text nor the
session capability. The disposable report records
`query_rejected=422:DATA_QUERY_REJECTED` before the existing revocation marker.
The focused ACP/CLI package suite, workspace binary build, explicit ignored
integration, and root `lint:fast` pass for this follow-up. This strengthens
the shared rejected-query case through ACP; full interactive UI/CLI/TUI/ACP
policy and error parity remains open. Assurance remains `INTERNAL_PASS` for
the local synthetic case only; reviewer status is `NOT_REQUESTED` and
reproducibility is `NOT_REPRODUCED`.

Packaged-sidecar target follow-up (2026-09-25): after building the workspace
binaries for `x86_64-apple-darwin`, the previously ignored
`service_issues_scoped_capability_to_synthetic_acp_profile` integration passed
when explicitly run for that target: 1 passed, 0 failed, 8 filtered out. It
used the local HyperAI service, synthetic ACP peer, disposable home, and
packaged `hyperai-mcp` sidecar to check the ten-tool allowlist, synthetic Data
read, owner-only Fleet denial, capability revocation, and sidecar exit. The
executables ran under Rosetta on macOS arm64; this is not native Intel
qualification or a clean second-environment reproduction. Build and test
commands, executable digests, output digest, and the source-inventory identity
are recorded in the
[`current-host validation record`](state-slices.md).
The bounded ACP result remains `INTERNAL_PASS`; reviewer status remains
`NOT_REQUESTED`, and reproducibility remains `NOT_REPRODUCED`.

ACP browser focus follow-up (2026-09-27): `app.js` SHA-256
`616cfa968aba7cfc58693f87c3dba426be62054f520095fd8c9aec5f4d7b67af`
matched the served bytes from the rebuilt local UI binary, SHA-256
`212fe8073f4443c8fda2ca69497cf4e843167e1570b945e417b14494d2a08f35`. The
disposable browser session connected only the HyperAI synthetic ACP peer, sent
one fixed synthetic prompt, displayed the fixed response, and disconnected.
The accessibility tree confirmed focus at the session heading after connect,
the prompt field after response, and the connect heading after disconnect.
In-progress messages announced connection, prompt, cancellation, and
disconnect operations through the existing status toast. The temporary
service/home were removed, the synthetic peer exited, and no other process
remained. `corepack pnpm run lint` passed. This is `INTERNAL_PASS` for the
observed local focus/status path; browser engine/version and screen-reader
operation were not captured. It does not close full assistive accessibility,
reduced-motion emulation, complete session/error-state coverage,
UI/CLI/TUI/ACP policy parity, platform qualification, or clean reproduction.
Reviewer status remains `NOT_REQUESTED`; no external agent or provider was
contacted.

Current-source browser session follow-up (2026-09-26): on macOS 26.6.1 arm64,
the current `hyperai` binary served the current workspace shell from an
isolated temporary home on `127.0.0.1:4318`. The served `index.html`,
`app.js`, and `app.css` matched the workspace SHA-256 values
`8f15d9272533e8672e7d91ff1c00d364a82257175549cf787c7e858e7b1230b7`,
`750416e0fbe1adb878268c45174b4a3571ca43680ffe6969003c1e3b29ae9d5b`, and
`52a34558e3e1aa6eac20fd52a4c61ddd1163e8c9a30efd642cc2189bef6ffcf1`.
Submitting the blank required profile left focus on the missing executable
field and displayed the browser's required-field message. Source inspection
confirms the service validates the profile before it can spawn a process.

The browser then connected the HyperAI-authored
`target/debug/acp-fixture-peer` (SHA-256
`9140f55b5398889274a6a024cf6278f7041c03e50f1783112cb298e12c09c16f`) with no
arguments or environment entries. The page showed ACP v1 ready state, the
fixed `hyperai.agent-tools.readonly.v1` tool-set identifier, the ten named
read-only tools, and the process-boundary disclosure. One fixed synthetic
prompt returned `Synthetic ACP response.`; the UI then returned to the profile
form on disconnect. CLI `hyperai agent status` against the same temporary home
reported `disconnected`, null session/profile identity, no capabilities, and
an empty event list. It used `target/debug/hyperai` SHA-256
`154b4df154cb841fd898d4650d49434a9ab72431a70caa140f7d087ff26cabe4` and the
packaged `hyperai-mcp` SHA-256
`6b1ee0aeba2f2adfb1d964c40b7a616d4a71d5e4e4e39899be11a814eee59eca`.

The service reported `LOCAL_LOOPBACK_ONLY_NO_PROVIDER_NO_FUNDS`, stopped with
status `stopped`, and released port 4318. Process inspection found no fixture
or MCP child; the pre-existing service on port 4317 remained running. The
temporary home and browser tab were removed. No third-party agent, provider,
user data, or external endpoint was used. Browser engine/version was not
captured. This adds a bounded browser-to-local-ACP lifecycle observation and a
CLI status check. It does not prove screen-reader operation, reduced-motion
emulation, full UI/CLI/TUI/ACP policy parity, platform support, or clean
reproduction. No overall assurance level is assigned; reviewer status is
`NOT_REQUESTED`, and reproducibility is `NOT_REPRODUCED`.

Linux/amd64 packaged-sidecar follow-up (2026-09-26): using the same
270-file source snapshot
(`5b13e723110353b871797a4c028d18086bdbb40ee9ce5bacb3ce69707fafad50`),
the workspace binaries built offline and the previously ignored
`service_issues_scoped_capability_to_synthetic_acp_profile` test passed: 1
passed, 0 failed, 8 filtered out. The local Docker image was
`rust:1.93.1-bookworm` with image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`;
the container ran `linux/amd64` under Rosetta on Apple Silicon, with no
network, read-only source/cache mounts, and Docker `--init`. The output SHA-256
is `3aea221b6c85825bee2bfc36f43d360dc722d9d28764bc8271dc9494c2c0c9b4`.
The ELF x86-64 outputs were:

- `hyperai`: `6f0fd21a03c0a200d13810121e7e60d4961ca62807c5584bdec7964d78f845b2`
- `hyperai-mcp`: `b72bd4b6f12c3d1938031bf35121d57d54450a10f9079d0a6bfb0831a319f215`
- `acp-fixture-peer`: `c067ef85e720ee4f8301a89b73f1d9c0636312b07784cb93791a897a97b0f0d4`

The test used the local synthetic peer and a disposable service home. It did
not contact a third-party agent, provider, registry, Fleet, or chain. This is
target-specific emulated-container evidence; it is not native Linux
qualification and does not close the full accessibility, policy-parity, or
packet-level reproduction gates. The earlier Linux run did not record whether
`--init` was enabled, so no `REPRODUCED_LOCAL` label is assigned to this
follow-up.

ACP browser error-code follow-up (2026-09-27): the local browser submitted a
profile with a relative executable and received the stable
`AGENT_PROFILE_INVALID` code alongside the generic service message. The
submitted path was not displayed, focus returned to the connect heading, and
service validation rejected the profile before process launch. Current
workspace `app.js` SHA-256 is
`d495e77587e681dd66e734a3553bd121e4228bd4d9b7b84d046528d650ab9bf9`. The root
`corepack pnpm run lint`, browser JavaScript syntax check, and three focused
error-envelope tests pass. This is `INTERNAL_PASS` for the observed
synthetic validation case only. It does not establish screen-reader behavior,
full error-state coverage, cross-client policy parity, platform support, or
clean reproduction of the current browser bundle. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The disposable service
and home were removed; no profile process was started and no external agent or
provider was contacted.

Current-source packaged-sidecar reproduction follow-up (2026-09-27): the
exact current 268-file manifest and verified HyperIntel snapshot were mounted
read-only into the pinned Rust 1.93.1 `linux/amd64` image with networking
disabled. After an offline four-job workspace-bin build, the explicitly
invoked `service_issues_scoped_capability_to_synthetic_acp_profile` test passed
1 test, failed 0, with 8 filtered out. The workspace-bin output SHA-256 is
`60e70ed13dfb0a4f692711e32f686be368e78819eede841cecda21fdc93898d5`; the
test output SHA-256 is
`a080f8bba9052b34d1d36451547154d6f2c84df0e0d55e75208a99b4079571ca`. The
manifest digest, import verification, source-unchanged comparison, image
identity, custody, and cleanup are in
[`hyperai.bazaar.rust_port_current_acp_sidecar_reproduction.v1`](state-slices.md#hyperai-bazaar-rust-port-current-acp-sidecar-reproductionv1).
Assurance is `REPRODUCED_LOCAL` for this exact synthetic integration only;
reviewer status is `NOT_REQUESTED`. Docker Desktop emulated the target on
Apple Silicon. This does not establish native platform support, third-party
ACP compatibility, broad accessibility, cross-client policy parity,
independent review, or production readiness.

Fleet preview parity follow-up (2026-09-27): the ignored packaged-sidecar
integration now calls `hyperai_fleet_operation_preview` through the profile-
issued capability for `fixture-node-atlas` / `launch`. The synthetic peer
checks the versioned preview schema, `preview_ready`, `not_authorized`, the
advisory-only effect boundary, target/operation identity, and a canonical
64-character digest, with no command or reservation fields. The test compares
that digest with the owner route's preview for the same request and confirms
the Fleet simulator JSON is identical before and after. Existing Data denial,
owner-only Fleet transition denial, capability revocation, sanitized report,
and sidecar-exit checks still pass. The pinned bridge returns JSON text as a
JSON-encoded MCP string; the fixture unwraps this transport representation
before checking fields, without changing a production contract.

On macOS 26.6.1 arm64 with Rust 1.93.1, the offline workspace-bin build and
the scoped `outcomes-cli`, `hyperai-acp`, and `hyperai-mcp` all-target suite
passed (63 passed, 0 failed, 1 ignored). The ignored integration passed when
run explicitly (1 passed). The binary digests and exact commands are in
[`hyperai.bazaar.fleet_preview_acp.v1`](state-slices.md#hyperai-bazaar-fleet_preview_acpv1).
This is `INTERNAL_PASS` for one local synthetic ACP Fleet preview path only;
reviewer status is `NOT_REQUESTED` and reproducibility is `NOT_REPRODUCED`.
It does not close full UI/CLI/TUI/ACP policy parity, accessibility, other
platforms, independent review, or release gates.

Installed-host follow-up (2026-09-27): the ignored source-install integration
passed on macOS 26.6.1 arm64 with Rust/Cargo 1.93.1. It runs the installed
`hyperai` binary from a disposable root, resolves the installed sibling
`hyperai-mcp`, connects only the HyperAI synthetic peer, checks the fixed
ten-tool surface and matching Fleet preview digest, then verifies capability
revocation and service/sidecar shutdown. The fixture binary is absent from the
product install. The exact invocation is
`cargo test --locked --offline -p outcomes-cli --test cli_compat
source_install_bundles_mcp_sidecar_and_connects_synthetic_peer -- --ignored
--exact --nocapture`; the test passed 1 case. Assurance is `INTERNAL_PASS` for
this installed local synthetic path only. Reviewer status is `NOT_REQUESTED`,
reproducibility is `NOT_REPRODUCED`, and temporary binaries, home, data, and
peer report were removed with the test directory. This does not qualify
third-party agents, other platforms, independent review, or release readiness.
The command and authority boundary are recorded in the
[local install ACP bundle state slice](state-slices.md#hyperai-bazaar-local_install_acp_bundlev1).

Seller-failure V3 parity follow-up (2026-09-28): the ignored packaged
`service_issues_scoped_capability_to_synthetic_acp_profile` integration now
passes with the V3 list (the exact V2 prefix plus the seller-failure tool) and
executes the fixed failure-preview tool over the real local stdio sidecar.
The fixture checks the seller-failure state slice, report-only claim ceiling,
digest presence, and exclusion of completion, verifier, and disposition
records. The service authorization test confirms the scoped capability can
GET only the fixed agent alias; the owner route remains owner-only. The CLI
test compares both default and opt-in output with the corresponding owner
routes, and the TUI shows mode and controls without clipping its narrow
failure details. Focused MCP/service/CLI/TUI tests, pinned formatting,
`python3 scripts/verify_imports.py`, and root `corepack pnpm run lint` passed
on macOS arm64 with Rust/Cargo 1.93.1. This is `INTERNAL_PASS` for the local
synthetic client-parity slice only; reviewer status is `NOT_REQUESTED` and
reproducibility is `NOT_REPRODUCED`. It does not qualify native Windows,
Ubuntu, or Intel macOS support; keyboard/accessibility breadth; live Fleet,
paid Eval, financial Trade; actual execution; or external/production actions.
Evidence and the exact state-slice boundary are in the
[seller-failure client-parity packet](compute-bazaar-workload-trace-seller-failure-clients-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-workload_trace_seller_failure_client_parityv1).

ACP cancel-keyboard browser continuation (2026-09-30): the frozen Chrome
154.0.8037.92 synthetic connect, pending prompt, keyboard cancel, cancelled
state, focus restoration, and disconnect assertions passed. The runner left
Crashpad PID `8401` in process group `8400`; it was identified as the exact
Chrome helper and terminated with `SIGTERM`. Since the runner did not confirm
the process group empty, both cancellation packets remain `INCONCLUSIVE` and
must not be repeated. This is a runner-cleanup failure, separate from the ACP
protocol lifecycle tests; broader browser accessibility and client-policy
parity remain open. See the [v2 packet](compute-bazaar-interface-acp-cancel-keyboard-v2-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-interface-acp_cancel_keyboardv2).

ACP cancel-keyboard v3 (2026-10-01) is also `INCONCLUSIVE`. The synthetic
browser command reached cleanup, but the final process-group check found the
Chrome Crashpad helper PID `42631` in group `42630`; the cleanup exception
prevented the runner from emitting its workflow report, so interaction and
request-count evidence are not retained. The exact helper group was terminated
afterward and the exact temporary root removed. Port `4317` was not used. Do
not repeat v3; use a new packet for any further harness investigation. See the
[v3 packet](compute-bazaar-interface-acp-cancel-keyboard-v3-packet.md) and
[state slice](state-slices.md#hyperai-bazaar-interface-acp_cancel_keyboardv3).

ACP cancel-keyboard v4 (2026-10-01) is `INCONCLUSIVE`. The process-list
continuation reached browser cleanup but exited 1 with browser group `5830` or
service PID `5525` reported active; the runner did not emit its workflow
report. Both exact process scopes were absent on later read-only checks, and
the exact temporary root was removed afterward. Do not repeat v4. Root lint
also failed once in a storage restore test with `HomeBusy`; investigate before
any retry. See the [v4 packet](compute-bazaar-interface-acp-cancel-keyboard-v4-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-interface-acp_cancel_keyboardv4).

ACP cancel-keyboard v5 (2026-10-01) is closed `INCONCLUSIVE`. Its one frozen
run exited 1 because cleanup reported browser group `22499` or service PID
`22211` still active; the runner emitted no workflow report. Read-only
post-run checks found neither exact scope present, but did not retain a
command/profile identity. No process was signaled and the exact temporary root
was preserved. The request counts and interaction assertions are unknown; do
not repeat v5. Preflight `lint:fast`, `verify:contracts`, import verification,
and all individually frozen input hashes passed. The recorded aggregate
source-tree digest could not be independently reproduced from the packet's
216-file set, so exact source-tree equality is unconfirmed. The earlier root
`verify:full` `HomeBusy` issue remains unresolved. See the [v5
packet](compute-bazaar-interface-acp-cancel-keyboard-v5-packet.md) and [state
slice](state-slices.md#hyperai-bazaar-interface-acp_cancel_keyboardv5).

Installed ACP V4 fixture alignment (2026-10-01): the offline workspace-bin
build, ACP synthetic-peer tests (9/9), exact ignored source-install integration
(1/1), pinned formatting, three-source import verification, and full root
`corepack pnpm run lint` passed. The installed current-source path verified
both allowed V4 preview GETs, owner-route and invalid-method/unknown-route
denials, ACP v1, capability revocation, TUI restoration, and cleanup. Test
services used port `0`; port `4317` was not contacted. The test fixture was
aligned with the existing V4 grants and digestless mock-disposition receipt
link; no product tool list, route, or permission changed. Assurance remains
`INCONCLUSIVE` because slice-specific fuzzing, threat modeling, and bounded
red-team review were not performed. Reviewer status is `NOT_REQUESTED` and
reproduction is `NOT_REPRODUCED`. This does not close Phase 8, Phase 2,
cross-platform, accessibility, rights, independent-review, or release gates.
See the [alignment packet](compute-bazaar-acp-v4-capability-alignment-packet.md)
and [state slice](state-slices.md#hyperai-bazaar-acp_v4_capability_fixture_alignmentv1).

### Current-source installed V6 tool-list alignment (2026-10-01)

A fresh offline source-install integration initially reached the installed
TUI and synthetic ACP session, then failed because its expected list still
named `MCP_TOOL_NAMES_V5` while the service and synthetic peer already used
the append-only `MCP_TOOL_NAMES_V6` list. The V6 list contains 15 read-only
tools, including the existing supply-quote-preview tool. Only the expected
constant in `crates/outcomes-cli/tests/cli_compat.rs` changed; production
ACP/MCP tools, schemas, permissions, routes, and install behavior did not.

The pinned offline fixture build passed, followed by the exact ignored
`source_install_bundles_mcp_sidecar_and_connects_synthetic_peer` integration
(`1 passed, 0 failed`). It installed to a disposable root, connected only
the HyperAI synthetic peer through the installed sibling sidecar, matched
the session tool list to V6, exercised the fixed synthetic read-only path
and Data denial boundary, then shut down the service and sidecar. The
installed TUI PTY check passed resize at 48x10 and 100x30, Ctrl+C, and
terminal restoration. Temporary homes and install files were removed;
post-run checks found no test-owned processes and no listener on port 4317.
Pinned import verification passed for Simple Compute Market, HyperIntel,
and JustAI.

The test source SHA-256 changed from pre-change
`b10474af0e6a77583dea41f99c0b6db51c81ea71c93d41543aed2820903e7869` to
`5902d02a83375b37f102010eeb34f1037638bd845f91442e0530356e64456891`; the
279-file source/harness inventory changed from
`a45ee047570ca7e27474fa494257c64b90d4559e3a00b8d7144b04ea80fc88ad` to
`2fa404d79ea8b7547e2bdd5de106e8b23247fb047e4d271d2f545b3043800116`.
Assurance is `INTERNAL_PASS` for this exact installed local synthetic V6
session only. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. No third-party agent, provider, seller, registry, remote
Fleet host, chain, or public service was contacted. This does not establish
native platform support, third-party-agent behavior, independent review,
clean reproduction, or release readiness. See the [V6 install alignment
state slice](state-slices.md#hyperai-bazaar-local_install_acp_tool_list_v6v1).
