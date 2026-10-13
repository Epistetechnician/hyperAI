# HyperAI ACP Keyboard Cancellation Qualification Packet

Status: frozen local browser qualification. This packet authorizes one
keyboard-driven cancellation workflow through the existing HyperAI browser
interface and synthetic ACP fixture.

State slice: `hyperai.bazaar.interface.acp_cancel_keyboard.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and boundary

Qualify the existing local interface path for connecting the fixed synthetic
ACP peer, starting a pending prompt, requesting cancellation from the
keyboard, observing the cancelled prompt state, and disconnecting. The peer
already supports `--wait-for-cancel`; the browser path has not yet qualified
that behavior.

This is a harness and documentation slice only. Do not change browser
application behavior, local-service routes, the ACP session contract, the
fixture peer, MCP tools, source records, or upstream snapshots. The synthetic
peer advertises the fixed read-only MCP configuration but must not launch the
sidecar or invoke a tool.

## Frozen baseline

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve all pre-existing state.
The exact pre-change implementation inputs are:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `10e10e0307b1a5371b84ccaddc58abecaac235727e9aada5cc969ee88aa96a3f` |
| `package.json` | `9da756e0d480f580f34df376f9e70e9a8bc91996de5548277ca240d2b5eaf4c6` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `crates/hyperai-acp/Cargo.toml` | `86db2fa85795ffcf7d89876526f8eb21948353fa5bdd48bc490aad14c75a726c` |
| `crates/hyperai-acp/src/session.rs` | `26e7492b4445c37611d7575128a134bcb4c74226fe9693d6c0a5978c49e68036` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | `0a2401b9d07e492aac398233d791a918b38a62ec8e4e125a4b6b15da5acdb556` |
| `crates/hyperai-mcp/Cargo.toml` | `6141d8f52750fab10aabdd33347a82176f3b238c6fbe0707947682046ea304bf` |
| `crates/hyperai-mcp/src/main.rs` | `60dd4b5a09b5403daf14607a46b7a5f51a7657ef230e31d4ad4c4ab33e78a8bc` |
| `crates/hyperai-mcp/src/lib.rs` | `73daab3f0e330d66518930d263257072fb74a540bd7aa35ba258f52f398a1022` |
| `crates/hyperai-local-service/src/server.rs` | `005d2e9f685b25fcaf75196fdfe63791f9c89ec75b91349491081773f4b68df3` |
| `crates/hyperai-local-service/src/auth.rs` | `2db8388d29b0f3e5e7951d7c9ffdb8b7c56451b5941a7b502cb51f8fc19f7af1` |

Toolchain: macOS 26.6.1 arm64, Node.js `v22.23.2`, pnpm `11.7.0`, pinned
Rust/Cargo `1.93.1`, and Google Chrome `154.0.8037.92` at
`/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

## Exact scope and allowed effects

Implementation files: `scripts/verify-interface-browser.mjs` and
`package.json`. Evidence files: this packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-8-acp-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`.

Allowed effects are locked offline builds of `hyperai-ui`, `hyperai-mcp`, and
`acp-fixture-peer`; one disposable local service home and Chrome profile;
ephemeral IPv4 loopback ports; read-only CDP role/name and focus inspection;
keyboard navigation, field entry, and Enter activation; and one fixed synthetic
connect/prompt/cancel/disconnect journey. The browser may make same-origin
`GET`/`HEAD` requests and exactly one HTTP 200 `POST` to each of:

- `/api/v1/agent/connect`
- `/api/v1/agent/prompt`
- `/api/v1/agent/cancel`
- `/api/v1/agent/disconnect`

The profile uses the test-built fixture with `--wait-for-cancel`; the prompt
text is fixed synthetic data. The `hyperai-mcp` binary is built because the
local service validates its adjacent executable at connection time, but it
must not be launched. Do not contact `127.0.0.1:4317`, other local
listeners, external origins, providers, sellers, registries, upstream
repositories, chain endpoints, or reviewers. Do not launch the MCP sidecar or
invoke an MCP tool. Stop on any unexpected request, status, response, focus,
accessible name, or cleanup result. Remove the runner-owned service, browser,
fixture, profile, and temporary directory.

## Proof gates

1. The exact Chrome version guard runs before service/browser startup.
2. JavaScript syntax and package JSON parse checks pass.
3. Keyboard navigation reaches Agents & access, fills the fixed profile and
   `--wait-for-cancel` argument, and submits the prompt through visible focused
   controls.
4. The pending fixture message is visible after cancellation completes; the
   final prompt state is `cancelled`, and the ACP session can disconnect.
5. Chromium's computed role/name for the cancel control matches the frozen
   expectation; keyboard focus remains visible and returns to a meaningful
   control after cancellation and disconnect.
6. Exactly the four listed agent POSTs occur once each and return HTTP 200;
   no MCP call occurs.
7. Runner-owned process and temporary-state cleanup succeeds, and the existing
   listener on `127.0.0.1:4317` remains untouched.
8. `CARGO_NET_OFFLINE=true corepack pnpm run lint`, `git diff --check`, local
   packet-link checks, and import preservation are recorded. Existing import
   mismatches remain unchanged and must be reported literally.

## Claim ceiling

Maximum assurance is `INTERNAL_PASS` for this exact same-host Chrome keyboard
cancellation journey and fixed local synthetic peer. Reviewer status starts
`NOT_REQUESTED`; reproducibility starts `NOT_REPRODUCED`. This does not
establish operating-system screen-reader or physical-keyboard behavior,
cross-browser/platform behavior, broad ACP policy parity, Phase 8 completion,
source reuse rights, independent review, provider execution, paid execution,
federation, settlement, publication, or production readiness. It grants no
application or operational authority.

## Execution result

Closed `INCONCLUSIVE`. The Chrome `154.0.8037.92` browser flow passed the
keyboard connect, pending prompt, cancellation, cancelled-state, and
disconnect assertions. It inspected 33 route/viewport pairs and 19,404
reduced-motion entries, observed 38 same-origin requests, checked the cancel
button as role `button` / name `Cancel active request`, and returned focus to
`#agent-prompt`. The runner asserted one HTTP 200 POST each to connect,
prompt, cancel, and disconnect, with zero MCP tool calls.

The runner removed the service, fixture, temporary profile, and temporary
directory, but the post-run process audit found Chrome Crashpad helper PID
`57607` in runner process group `57606`. The helper was explicitly terminated
and a follow-up check found no runner-owned service, fixture, sidecar, browser
profile, or process group. Because the frozen packet required cleanup to
complete under the runner, the result remains `INCONCLUSIVE`; this exact v1
identity is closed and must not be repeated.

`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0. The contract and
workspace suites passed; environment-gated Postgres, Docker, and installed
sidecar cases remained ignored. `verify:imports` exited 1 for the unchanged
JustAI inventory mismatch (177 observed, 176 recorded); Simple Compute Market
and HyperIntel matched their recorded revisions. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The existing HyperAI
listener PID 85094 on `127.0.0.1:4317` remained untouched.

The v2 continuation freezes a corrected runner cleanup boundary in
[`compute-bazaar-interface-acp-cancel-keyboard-v2-packet.md`](compute-bazaar-interface-acp-cancel-keyboard-v2-packet.md).
