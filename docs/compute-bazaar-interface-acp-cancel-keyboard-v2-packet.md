# HyperAI ACP Keyboard Cancellation Qualification Packet v2

Status: closed `INCONCLUSIVE`. V1 remains closed `INCONCLUSIVE` because its
runner left a Crashpad helper after Chrome exited. V2 exercised the browser
journey, but its process-group cleanup gate also failed; this identity must not
be repeated.

State slice: `hyperai.bazaar.interface.acp_cancel_keyboard.v2`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Prior outcome: [v1 packet](compute-bazaar-interface-acp-cancel-keyboard-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and fixed scope

The local browser runner's Chrome process group may outlive the main Chrome
process through helper processes. Change only
`scripts/verify-interface-browser.mjs` so cleanup sends termination to the
entire owned browser process group, waits until that group is empty, escalates
to `SIGKILL` after the fixed grace period, and reports cleanup failure if the
group remains. Temporary state must still be removed after process cleanup.

Then run the existing `verify:interface:acp-cancel-keyboard` path once. It
connects the fixed synthetic peer, submits the fixed prompt, presses the
accessible Cancel control while the prompt is pending, observes the cancelled
state, and disconnects. Browser application, service/API, ACP session,
fixture, MCP implementation, and all imported workspaces remain read-only.
The MCP sidecar may be built because the service requires its adjacent binary,
but it must not be launched and no MCP tool may be invoked.

## Frozen baseline

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the dirty and untracked worktree after v1 implementation and result capture.
Preserve all unrelated work. Exact pre-change inputs:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `656060b6ab7a95bdb140b29c17057381d9732b75d8d03f2d4a81bfb83a3de90d` |
| `package.json` | `6a1eb1e47be05d8572cb964b997a7b555e82d83197fedf2e484b8b11fa865758` |
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

## Exact files, effects, and command

Implementation file: `scripts/verify-interface-browser.mjs`. Result files:
this packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-8-acp-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`. `package.json` and all runtime/source
inputs in the table above are read-only.

Run once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-cancel-keyboard
```

The command builds only the pinned offline `hyperai-ui`, `hyperai-mcp`, and
`acp-fixture-peer` binaries, then uses a disposable local service home, Chrome
profile, fixture process, and IPv4 loopback ports. The exact frozen browser
version is checked before starting the service or browser. The fixture profile
uses `--wait-for-cancel`, and the prompt is fixed synthetic text. Permit only
same-origin `GET`/`HEAD` plus exactly one HTTP 200 `POST` to each of:

- `/api/v1/agent/connect`
- `/api/v1/agent/prompt`
- `/api/v1/agent/cancel`
- `/api/v1/agent/disconnect`

Do not connect to port 4317, other local listeners, external origins,
providers, sellers, registries, chains, or reviewers. Do not launch the MCP
sidecar or call an MCP tool. The runner must confirm the Chrome process group
has exited before removing its temporary directory. After it exits, verify no
runner-owned service, Chrome/browser, Crashpad helper, fixture, MCP process,
or temporary browser directory remains, and confirm PID 85094 still owns
`127.0.0.1:4317`.

## Proof gates and claim ceiling

1. JavaScript syntax and package JSON checks pass.
2. The cancellation assertions match v1: keyboard route navigation and form
   entry, fixed pending prompt, role `button` / name `Cancel active request`,
   visible keyboard focus, cancelled terminal state, prompt focus restoration,
   and keyboard disconnect.
3. Exactly four listed ACP POST requests occur once each with HTTP 200; there
   are zero MCP tool calls.
4. The browser process group is confirmed empty, all runner-owned resources
   are absent, and the existing listener remains untouched.
5. `CARGO_NET_OFFLINE=true corepack pnpm run lint`, `git diff --check`, local
   packet-link checks, and import preservation are recorded. Existing import
   mismatches are preserved and reported.

Maximum assurance is `INTERNAL_PASS` for this exact same-host Chrome keyboard
cancellation workflow and its process-group cleanup. Reviewer starts
`NOT_REQUESTED`; reproduction starts `NOT_REPRODUCED`. This does not establish
OS screen-reader or physical-keyboard behavior, other browser/platform
behavior, broad ACP client-policy parity, Phase 8 completion, upstream rights,
independent review, provider execution, paid execution, federation,
settlement, publication, or production readiness.

## Execution result

The Chrome `154.0.8037.92` keyboard workflow passed: 33 route/viewport checks,
33 reduced-motion checks, 19,404 motion entries, one synthetic pending prompt,
one keyboard cancellation, the `cancelled` terminal state, returned prompt
focus, and keyboard disconnect. There were 38 same-origin requests and exactly
one HTTP 200 POST to each of connect, prompt, cancel, and disconnect; no MCP
tool was invoked.

The cleanup gate failed: after the runner returned, Chrome Crashpad PID `8401`
remained in process group `8400`. Process inspection confirmed the exact
Chrome `154.0.8037.92` `chrome_crashpad_handler` executable; that runner-owned
PID was terminated with `SIGTERM`. The process group was not confirmed empty
by the frozen runner before temporary-state removal, so this packet is
`INCONCLUSIVE` even though the interaction assertions passed. Do not repeat
v2. The existing service PID `85094` on `127.0.0.1:4317` was untouched.

Current harness SHA-256:
`6eec3fc8a2ccf45290fca5269071e56f90775446f001f20118f4af2d6f7f487`.
JavaScript syntax, `package.json` parsing, and `git diff --check` passed.
Import verification passed Simple Compute Market (2,189 files) and HyperIntel
(99 files), while preserving the existing JustAI mismatch (177 observed,
176 recorded). Root offline lint passed after the browser attempt. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. No source app,
service/API, ACP session, fixture, MCP behavior, or upstream workspace was
changed.
