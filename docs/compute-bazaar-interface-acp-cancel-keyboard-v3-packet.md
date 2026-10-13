# HyperAI ACP Keyboard Cancellation Qualification Packet v3

Status: closed `INCONCLUSIVE`. V1 and v2 also remain closed `INCONCLUSIVE`;
none of those identities may be repeated.

State slice: `hyperai.bazaar.interface.acp_cancel_keyboard.v3`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Prior outcome: [v2 packet](compute-bazaar-interface-acp-cancel-keyboard-v2-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and exact scope

The v1 and v2 browser journeys passed their synthetic connect, pending prompt,
keyboard cancellation, cancelled-state, focus-restoration, and disconnect
assertions. Their runner did not prove that Chrome's Crashpad helper had
exited. This continuation changes only `scripts/verify-interface-browser.mjs`:
connect to the browser-level DevTools endpoint, request the protocol's graceful
`Browser.close`, retain process-group termination as a fallback, confirm the
runner-owned browser scope and service have exited, and remove temporary state
only after those cleanup checks pass. If a process remains, preserve the
temporary state and fail the run.

Then run the existing synthetic cancellation path exactly once. The browser
application, service/API, ACP session contract, fixture, MCP implementation,
and all imported product workspaces remain read-only. `hyperai-mcp` may be
built because the existing package script requires its adjacent binary, but it
must not be launched and no MCP tool may be invoked.

## Frozen baseline

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the complete dirty and untracked worktree observed before this packet. Preserve
all of that pre-existing state. The local inspection found no listener on
`127.0.0.1:4317`; do not connect to, reserve, or modify port `4317`.

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `a35fe21b8686f914ebc0ac1dd40e006c11c28445338f33d5be7d8af540c38561` |
| `package.json` | `c83c2a132cecaa471e5fcd0d98bfa28f62855cd6b61af45d877d16051e4ba2c8` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `fbfdbc90fcfd7f12de22ff86bc2cc78a6e673df55d42355b4434b8dfa8cbd464` |
| `apps/hyperai-interface/app.css` | `769feaaf75333e99b3324fc683b6fe5967ebcb38892f870a8dc3499b478dddc3` |
| `crates/hyperai-acp/Cargo.toml` | `86db2fa85795ffcf7d89876526f8eb21948353fa5bdd48bc490aad14c75a726c` |
| `crates/hyperai-acp/src/session.rs` | `26e7492b4445c37611d7575128a134bcb4c74226fe9693d6c0a5978c49e68036` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | `0a2401b9d07e492aac398233d791a918b38a62ec8e4e125a4b6b15da5acdb556` |
| `crates/hyperai-mcp/Cargo.toml` | `6141d8f52750fab10aabdd33347a82176f3b238c6fbe0707947682046ea304bf` |
| `crates/hyperai-mcp/src/main.rs` | `60dd4b5a09b5403daf14607a46b7a5f51a7657ef230e31d4ad4c4ab33e78a8bc` |
| `crates/hyperai-mcp/src/lib.rs` | `73daab3f0e330d66518930d263257072fb74a540bd7aa35ba258f52f398a1022` |
| `crates/hyperai-local-service/src/server.rs` | `de1d5b6486110b6ed1859698ecee8db7e76149933d8ad84a1f5abca99e4fb950` |
| `crates/hyperai-local-service/src/auth.rs` | `2db8388d29b0f3e5e7951d7c9ffdb8b7c56451b5941a7b502cb51f8fc19f7af1` |

Toolchain: macOS 26.6.1 arm64, Node.js `v22.23.2`, pnpm `11.7.0`, pinned
Rust/Cargo `1.93.1`, and Google Chrome `154.0.8037.92` at
`/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

## Exact command and effects

Run once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-cancel-keyboard
```

The command syntax-checks the runner, builds only the pinned offline
`hyperai-ui`, `hyperai-mcp`, and `acp-fixture-peer` binaries, then uses a
disposable local service home, Chrome profile, fixture process, and ephemeral
IPv4 loopback ports. The browser version is checked before the service or
browser starts. The fixed fixture waits for cancellation; the prompt is fixed
synthetic text. Permit only same-origin `GET`/`HEAD` plus exactly one HTTP 200
`POST` to each of:

- `/api/v1/agent/connect`
- `/api/v1/agent/prompt`
- `/api/v1/agent/cancel`
- `/api/v1/agent/disconnect`

Do not connect to port `4317`, other local listeners, external origins,
providers, sellers, registries, chains, or reviewers. Do not launch the MCP
sidecar or call an MCP tool. Confirm the owned browser process scope and local
service have exited before deleting their temporary home/profile. Preserve the
temporary state and fail closed if either scope does not exit.

## Proof gates and claim ceiling

1. JavaScript syntax and `package.json` parsing pass.
2. The cancellation assertions match v1: keyboard route navigation and form
   entry, fixed pending prompt, role `button` / name `Cancel active request`,
   visible keyboard focus, cancelled terminal state, prompt focus restoration,
   and keyboard disconnect.
3. Exactly four listed ACP POST requests occur once each with HTTP 200; there
   are zero MCP tool calls.
4. Browser and service cleanup are confirmed by the runner before temporary
   state removal; no runner-owned service, browser, Crashpad helper, fixture,
   MCP process, or temporary browser directory remains.
5. `CARGO_NET_OFFLINE=true corepack pnpm run lint`, import preservation, and
   `git diff --check` are recorded. Preserve and report the existing JustAI
   import-inventory mismatch.

Maximum assurance is `INTERNAL_PASS` for this exact same-host Chrome synthetic
keyboard-cancellation workflow and its cleanup. Reviewer status starts
`NOT_REQUESTED`; reproducibility starts `NOT_REPRODUCED`. This does not
establish operating-system screen-reader or physical-keyboard behavior, other
browser/platform behavior, broad ACP client-policy parity, Phase 8 completion,
upstream rights, independent review, provider execution, paid execution,
federation, settlement, publication, or production readiness.

## Execution result

The exact command reached the browser runner after the pinned offline build
steps. The runner exited with status 1 during cleanup and emitted no workflow
report because its final cleanup exception prevented report output; the
interaction assertions and request counts are therefore not retained for this
identity. The final check found an active browser process group and preserved
the temporary root. The local service had exited. Process inspection identified
Chrome Crashpad PID `42631` in process group `42630`, started at
`2026-10-01 07:26:25` while this run's disposable profile was created. That
group contained only the helper. The exact group was terminated with `SIGTERM`,
confirmed empty, and the run's exact temporary root was then removed. Port
`4317` was not used. Assurance is `INCONCLUSIVE`; reviewer status is
`NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`. Root lint and import
preservation were not run for this packet.
