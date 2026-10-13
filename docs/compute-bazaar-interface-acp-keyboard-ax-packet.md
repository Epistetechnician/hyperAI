# HyperAI ACP Keyboard and Accessibility Packet v1

Status: frozen local qualification continuation. No interface or service
behavior changes are authorized under this state slice.

State slice: `hyperai.bazaar.interface.acp_keyboard_ax_fixture.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Existing local contract: [Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Preserve all pre-existing state.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and boundary

Qualify the existing Agents & access browser flow with keyboard navigation,
computed Chromium roles and names, visible focus, focus restoration, and one
synthetic ACP fixture response. This is a local browser and contract check. It
does not qualify OS screen readers, a physical keyboard, another browser or
platform, broad client-policy parity, or the whole Phase 8 acceptance gate.

The measured journey uses the fixed `acp-fixture-peer` binary with no fault
flags, a disposable service home and browser profile, one fixed synthetic
profile, and the prompt `synthetic keyboard flow check`. The fixture returns
`Synthetic ACP response.`. The service issues its volatile session capability
and advertises the fixed adjacent `hyperai-mcp` stdio configuration in the ACP
session handshake. The fixture validates that configuration but does not
launch the sidecar or invoke an MCP tool. The prompt and profile are not
retained after the temporary service home is removed.

## Baseline and exact files

Exact tracked scope:

- `scripts/verify-interface-browser.mjs`: add an opt-in keyboard and AX
  workflow; preserve every existing mode.
- `package.json`: add `verify:interface:acp-keyboard-ax` with no dependencies.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-phase-2-data-packet.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record the frozen scope and result.

Application, service, ACP, fixture, MCP, and contract files are read-only.
Their baseline SHA-256 values are:

| File | SHA-256 |
|---|---|
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd` |
| `apps/hyperai-interface/app.css` | `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `crates/outcomes-cli/Cargo.toml` | `8a0ccfcd67e8e232d655cb9b02104b75a04259df2a4b412df8a26a9d2e4fc144` |
| `crates/hyperai-local-service/Cargo.toml` | `b5f6831cf1b75c19aa035e7f785a81519ebcf33092aea4a04c72cf3fb60d02af` |
| `crates/hyperai-local-service/src/server.rs` | `005d2e9f685b25fcaf75196fdfe63791f9c89ec75b91349491081773f4b68df3` |
| `crates/hyperai-acp/Cargo.toml` | `86db2fa85795ffcf7d89876526f8eb21948353fa5bdd48bc490aad14c75a726c` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | `0a2401b9d07e492aac398233d791a918b38a62ec8e4e125a4b6b15da5acdb556` |
| `crates/hyperai-acp/src/session.rs` | `26e7492b4445c37611d7575128a134bcb4c74226fe9693d6c0a5978c49e68036` |
| `crates/hyperai-acp/src/profile.rs` | `02ea68a66c45e229d5cb93149a1cead4db1268e75726a32a1346ea551bc15840` |
| `crates/hyperai-acp/src/tools.rs` | `7ac1b411282ab5fe03f3139b76ec6fbd89485fccf96e9d0b1c740b26ea6c78b6` |
| `crates/hyperai-mcp/Cargo.toml` | `6141d8f52750fab10aabdd33347a82176f3b238c6fbe0707947682046ea304bf` |
| `crates/hyperai-mcp/src/main.rs` | `60dd4b5a09b5403daf14607a46b7a5f51a7657ef230e31d4ad4c4ab33e78a8bc` |

Harness baseline SHA-256:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `48e6bf7a09233663905340bada278bca4395476ba9b9113998e37a7311918d00` |
| `package.json` | `85fc2a382acd45ba044b93a8b90ef8aa555b913fe73612977afaa8b907084c13` |

Toolchain freeze: macOS 26.6.1 arm64, Node.js 22.23.2, pnpm 11.7.0, pinned
Rust/Cargo 1.93.1, and Google Chrome 154.0.8037.58. Use the pinned Rust
wrapper, not system Rust. Chrome executable:
`/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

## Frozen accessible-name expectations

Names are compared after NFC normalization, Unicode-whitespace collapse and
trimming, then ASCII case folding. Keep the raw Chromium role and name in the
workflow output. Roles remain exact and case-sensitive; normalized names
remain exact for words and punctuation.

| Target | Chromium role | Accessible name |
|---|---|---|
| Agents & access navigation | `button` | `Agents & access` |
| Connect heading | `heading` | `Connect a local agent` |
| Profile name | `textbox` | `Profile name` |
| Executable path | `textbox` | `Absolute executable path` |
| Working directory | `textbox` | `Working directory` |
| Arguments | `textbox` | `Arguments one argument per line` |
| Connect action | `button` | `Review and connect` |
| Connected synthetic profile heading | `heading` | `HyperAI ACP synthetic fixture` |
| Prompt field | `textbox` | `Send a prompt to this local process` |
| Send prompt action | `button` | `Send prompt` |
| Disconnect action | `button` | `Disconnect` |
| Disconnected connect heading | `heading` | `Connect a local agent` |

Every AX target must be present and not ignored. Check visible keyboard focus
on navigation, connect, prompt, and disconnect controls where they receive
keyboard focus. Check that connect returns focus to the active-session heading,
prompt completion returns focus to the prompt field, and disconnect returns
focus to the connect heading. Confirm the fixture response is visible in the
session-message list before disconnect.

## Exact command and effects

Run once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-keyboard-ax
```

The package command checks runner syntax, builds only `hyperai-ui`,
`hyperai-mcp`, and `acp-fixture-peer` with pinned Rust and the locked offline
workspace, then runs the browser harness. The harness retains the existing
route and reduced-motion prelude. The measured ACP path uses Tab and Enter for
navigation and activation, keyboard focus plus CDP text insertion for fixed
synthetic form values, and keyboard activation for connect, prompt, and
disconnect. CDP DOM and AX operations are reads only; there is no DOM click,
DOM value assignment, or direct API setup in the measured journey.

Permitted browser network behavior is same-origin `GET`/`HEAD`, plus exactly
one `200` response each for `POST /api/v1/agent/connect`,
`POST /api/v1/agent/prompt`, and `POST /api/v1/agent/disconnect`. Any other
POST, nonlocal request, failed status, browser error, or unexpected fixture
state stops the run. Use only fresh ephemeral loopback ports, a disposable
service home and Chrome profile. Do not connect to the existing listener on
`127.0.0.1:4317`, any upstream repository, provider, seller, external source,
or public service. Remove all runner-owned profiles, temporary data, and
processes at completion. Do not retain screenshots, prompt contents, profile
values, session capabilities, or browser artifacts.

## Stop conditions, proof gates, and claim ceiling

Stop on the first absent or ignored AX target, role mismatch, normalized-name
mismatch, invisible keyboard focus, focus-restoration failure, wrong fixture
response, unexpected request/status, browser error, port collision, failed
disconnect, or failed cleanup. Do not repeat a failed frozen run or alter its
expectations under this identity; make a new state slice if product behavior
needs correction.

Proof gates: exact roles and normalized names with raw AX evidence retained;
keyboard navigation and activation; visible focus and focus restoration;
connected/session/message/disconnected state checks; exactly three permitted
ACP POSTs with HTTP 200; browser and process cleanup; JavaScript syntax and
package JSON checks; `git diff --check`; root
`CARGO_NET_OFFLINE=true corepack pnpm run lint`; and import preservation with
any existing mismatch reported unchanged.

Maximum claim: `INTERNAL_PASS` for the listed computed Chromium names, keyboard
journey, and one fixed local ACP fixture response on this host. Reviewer
status starts `NOT_REQUESTED`; reproducibility starts `NOT_REPRODUCED`. This
does not establish VoiceOver or another OS screen reader, physical-keyboard
behavior, another browser/platform, general ACP client-policy parity, Phase 8
completion, Phase 2 acceptance, independent review, upstream source rights,
paid compute, external federation, or production readiness. No interface,
service, API, contract, persisted schema, upstream import, or external effect
is authorized.

## Execution result (2026-09-30)

The one-shot package command exited 1 after the three locked offline binary
builds completed. The browser runner exited from its request-policy callback
with `ReferenceError: acpKeyboardAXWorkflow is not defined`: the flag was
returned by argument parsing but omitted from `main`'s destructuring. The
measured ACP workflow did not start; no ACP connect, prompt, or disconnect
POST occurred. No application or contract file changed. This identity is
closed as `INCONCLUSIVE` and was not repeated.

The abrupt runner exit left its isolated Chrome process group (88347) and
profile under `/var/folders/qd/xlj1mz_x2t92gdst9km986p00000gn/T/` alive. The
process group was stopped and that exact temporary profile removed. The
disposable service was no longer listening. Existing PID 85094 on
`127.0.0.1:4317` remained running and untouched. A fresh continuation is
recorded in [v2](compute-bazaar-interface-acp-keyboard-ax-v2-packet.md); it
fixes the runner binding only and retains the v1 expectations and claim
ceiling.
