# HyperAI ACP Keyboard and Accessibility Packet v2

Status: frozen local harness continuation. This identity preserves the v1
`INCONCLUSIVE` result and authorizes one fresh run after the runner-binding
correction below. It does not reopen or repeat v1.

State slice: `hyperai.bazaar.interface.acp_keyboard_ax_fixture.v2`.
Parent and frozen workflow: [v1 ACP keyboard/AX packet](compute-bazaar-interface-acp-keyboard-ax-packet.md).
Product direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Preserve all pre-existing state.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## v1 outcome and v2 change

The v1 packet is closed as `INCONCLUSIVE`: after all three locked offline
binary builds succeeded, the runner failed in its network-policy callback
because `acpKeyboardAXWorkflow` was not destructured in `main`. The measured
ACP journey never started and no ACP POST occurred. The exact orphan Chrome
process group and temporary profile were cleaned; the existing listener on
`127.0.0.1:4317` was left untouched.

The only implementation change authorized here is runner flag wiring:
`parseArguments` must return one `acpKeyboardAXWorkflow` value, and `main` must
destructure it before the browser request callback uses it. Preserve all
other workflow flags, v1 behavior, and application files. The measured
journey, names, request allowlist, exact command, and claim ceiling are
inherited unchanged from v1.

## Baseline and exact files

The v2 pre-change harness baseline is:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `33ea9b131eb94082a1d7f2185a1b26efe5fd65ca47da3bf3ede8e114b7f85cbb` |
| `package.json` | `9da756e0d480f580f34df376f9e70e9a8bc91996de5548277ca240d2b5eaf4c6` |

Read-only application baseline remains frozen at the v1 hashes: HTML
`728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade`,
JavaScript `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd`,
and CSS `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046`.
Read-only runtime, service, fixture, ACP, MCP, manifest, and lockfile digests
are listed in the [v1 packet](compute-bazaar-interface-acp-keyboard-ax-packet.md#baseline-and-exact-files).

Exact tracked files: `scripts/verify-interface-browser.mjs`,
`package.json`, this packet, the v1 packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-2-data-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`. Runtime, service, ACP, fixture, MCP,
application, contract, and upstream snapshot source files are read-only.

Toolchain freeze is inherited from v1: macOS 26.6.1 arm64, Node.js 22.23.2,
pnpm 11.7.0, pinned Rust/Cargo 1.93.1, and Google Chrome 154.0.8037.58.

## Run and proof gates

Run once under this new identity:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-keyboard-ax
```

This builds the locked offline `hyperai-ui`, `hyperai-mcp`, and
`acp-fixture-peer`, then runs the full frozen v1 route/reduced-motion prelude
and ACP keyboard journey. The only effect authorized beyond the v1 read-only
prelude is the single fixed synthetic connect/prompt/disconnect flow. Network,
focus, AX, fixture, cleanup, lint, and import-preservation gates remain exactly
as specified in v1.

Stop and preserve the literal outcome if the flag is absent in `main`, any
runner error occurs, any v1 stop condition occurs, or cleanup fails. Do not
retry this identity or adjust expectations. Any additional fix requires a new
packet and state slice.

Maximum claim remains `INTERNAL_PASS` only for the frozen same-host Chromium
keyboard/AX path and one fixed local ACP fixture response. Reviewer status
starts `NOT_REQUESTED`; reproduction starts `NOT_REPRODUCED`. This does not
establish OS screen-reader or physical-keyboard behavior, browser/platform
parity, general ACP policy parity, Phase 8 completion, Phase 2 acceptance,
independent review, source rights, paid execution, federation, or production
readiness.

## Execution result (2026-09-30)

The frozen command exited 0 and the complete behavior passed on the observed
Google Chrome `154.0.8037.92`: all 33 route/viewport checks, all 33
reduced-motion checks, 12 frozen role/name targets, visible keyboard focus,
connect/prompt/disconnect focus restoration, and the exact synthetic fixture
response. The three permitted ACP POSTs returned HTTP 200 and no MCP tool was
invoked. The runner removed its service/browser processes and temporary home.
Existing PID 85094 on `127.0.0.1:4317` remained untouched.

The v2 packet froze Google Chrome `154.0.8037.58`, so its exact toolchain gate
did not match the observed `.92` binary. Preserve these browser results as
observations, but close v2 as `INCONCLUSIVE`; do not claim v2 `INTERNAL_PASS`.
The application hashes remained unchanged. Root lint and import-preservation
gates were not run. A new [v3 packet](compute-bazaar-interface-acp-keyboard-ax-v3-packet.md)
freezes `.92` and adds a preflight equality check before the runner starts the
service or browser.
