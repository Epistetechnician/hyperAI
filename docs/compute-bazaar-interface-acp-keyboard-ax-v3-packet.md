# HyperAI ACP Keyboard and Accessibility Packet v3

Status: frozen local harness continuation. V1 and v2 remain `INCONCLUSIVE`;
this packet authorizes one run with an exact Chrome version guard.

State slice: `hyperai.bazaar.interface.acp_keyboard_ax_fixture.v3`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Prior outcomes: [v1](compute-bazaar-interface-acp-keyboard-ax-packet.md) and
[v2](compute-bazaar-interface-acp-keyboard-ax-v2-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6b253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Preserve all pre-existing state.
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and frozen scope

V1 failed in harness setup because a workflow flag was missing from the main
binding. V2 corrected that binding and passed its browser workflow, but used
Google Chrome `154.0.8037.92` while the packet had frozen `154.0.8037.58`.
Neither identity establishes acceptance. V3 adds a mode-specific equality
check before service or browser startup and freezes the currently observed
Chrome `154.0.8037.92`.

The only implementation change is in `scripts/verify-interface-browser.mjs`:
when `--acp-keyboard-ax-workflow` is selected, require the exact browser
version string `Google Chrome 154.0.8037.92` before reserving ports or starting
the service. Preserve every other mode and v2 behavior. On mismatch, stop
before starting external processes and do not retry this identity.

The measured flow is inherited exactly from v1: route/reduced-motion prelude;
keyboard navigation to Agents & access; the 12 role/name AX expectations;
visible keyboard focus; focus restoration after connect, prompt, and
disconnect; one fixed synthetic ACP fixture response; and exactly one HTTP
200 POST to each of connect, prompt, and disconnect. The default fixture has
no fault flags. The service issues its volatile capability and advertises the
fixed adjacent MCP stdio configuration in the ACP session handshake. The
fixture validates this configuration but does not launch the sidecar or
invoke an MCP tool. Prompt and profile values are not retained. No application
behavior or contract changes are authorized.

## Baseline and exact files

Pre-change harness baseline:

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `284e84fac7d25b91f0f9cc7ae50d16fe026dd7359e3e5539bc4d108871fea137` |
| `package.json` | `9da756e0d480f580f34df376f9e70e9a8bc91996de5548277ca240d2b5eaf4c6` |

Read-only application SHA-256 values remain HTML
`728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade`,
JavaScript `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd`,
and CSS `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046`.
All locked manifests and read-only runtime, service, ACP, fixture, and MCP
digests remain as frozen in the v1 packet.

Exact tracked scope: `scripts/verify-interface-browser.mjs`, `package.json`,
the v1, v2, and v3 packets, `docs/state-slices.md`,
`docs/compute-bazaar-phase-2-data-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`. No app/runtime/service/fixture/MCP
source, external repository, or generated asset may be changed or imported.

Toolchain: macOS 26.6.1 arm64, Node.js 22.23.2, pnpm 11.7.0, pinned
Rust/Cargo 1.93.1, and exact Google Chrome `154.0.8037.92`. The Chrome binary
is `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

## Exact command, effects, and gates

Run once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-keyboard-ax
```

The package command builds only the locked offline `hyperai-ui`,
`hyperai-mcp`, and `acp-fixture-peer`, then runs the browser. The ACP-mode
Chrome equality guard must run before reserving ports or starting processes.
Use only ephemeral loopback ports, disposable service/browser state, the fixed
synthetic profile and prompt, read-only CDP DOM/AX inspection, and keyboard
navigation/activation. CDP text insertion is allowed only into keyboard-
focused fields. Do not use DOM clicks, direct API setup, or direct API POSTs.

Allowed network methods are same-origin `GET`/`HEAD` and exactly one HTTP 200
each for `POST /api/v1/agent/connect`, `POST /api/v1/agent/prompt`, and
`POST /api/v1/agent/disconnect`. Any other POST, nonlocal request, non-200
status, AX or focus mismatch, browser error, wrong fixture response, failed
disconnect, or cleanup failure stops the run. Do not connect to `127.0.0.1:4317`,
external services, providers, sellers, upstream repositories, or public
origins. Remove all runner-owned processes and temporary files; retain no
screenshots, profile/prompt values, capability, or browser artifacts.

Proof gates: exact version preflight; syntax and package JSON checks; 12 exact
computed roles/names with raw AX values; keyboard navigation/activation,
visible focus and restoration; exact fixture response; exact ACP POST paths,
counts, and HTTP 200 statuses; cleanup; `git diff --check`; root
`CARGO_NET_OFFLINE=true corepack pnpm run lint`; and import preservation with
pre-existing mismatches reported unchanged.

Maximum claim: `INTERNAL_PASS` only for the exact frozen same-host Chrome
keyboard/AX workflow and one fixed local ACP fixture response. Reviewer starts
`NOT_REQUESTED`; reproduction starts `NOT_REPRODUCED`. This does not establish
OS screen-reader or physical-keyboard behavior, another browser/platform,
general ACP client-policy parity, Phase 8 completion, Phase 2 acceptance,
independent review, upstream rights, paid execution, federation, or production
readiness. No external action or product-authority expansion is authorized.

## Execution result (2026-09-30)

The exact Chrome preflight passed for Google Chrome `154.0.8037.92`, and the
one-shot command exited 0. All 33 route/viewport checks and 33 reduced-motion
checks passed; 19,404 motion entries were inspected. All 12 frozen ACP
role/name expectations passed with raw Chromium names retained. Keyboard focus
was visible on navigation, form fields, and actions. Connect restored focus to
the active profile heading, prompt completion restored focus to the prompt
field, and disconnect restored focus to the connect heading. The fixture
returned `Synthetic ACP response.`. Exactly three ACP POSTs were observed, one
each to connect, prompt, and disconnect, all HTTP 200. The fixture validated
the advertised fixed MCP stdio configuration but did not launch the sidecar
or invoke a tool. The harness counted 37 local requests; all were same-origin
GET/POST traffic.

`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 0, including
`lint:fast`, workspace contract tests, and `verify:full`. Environment-gated
Postgres and OCI/Docker cases remained ignored. `git diff --check` and local
Markdown-link checks for the ACP packets passed. `verify:imports` exited 1:
Simple Compute Market and HyperIntel matched their pins, while JustAI retained
the existing 177-observed versus 176-recorded mismatch; no snapshot was
modified.

Assurance is `INTERNAL_PASS` for this exact same-host browser/fixture path.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Application SHA-256 values remained HTML
`728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade`,
JavaScript `f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd`,
and CSS `5063756e0114e24d231eaeb055667d61c02289f9a76d05c0ba0deb76c1002046`.
Post-run harness SHA-256 values are runner
`10e10e0307b1a5371b84ccaddc58abecaac235727e9aada5cc969ee88aa96a3f` and
package `9da756e0d480f580f34df376f9e70e9a8bc91996de5548277ca240d2b5eaf4c6`.
Temporary service, browser profile, and fixture process were removed. The MCP
sidecar was not launched. The existing service PID 85094 on `127.0.0.1:4317`
remained untouched.

This does not establish OS screen-reader or physical-keyboard behavior,
cross-browser/platform behavior, full ACP client-policy parity, overall
Phase 8 completion, Phase 2 acceptance, independent review, upstream rights,
or production readiness. The JustAI import mismatch remains a separate open
preservation issue.
