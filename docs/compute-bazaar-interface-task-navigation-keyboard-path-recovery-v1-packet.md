# HyperAI Interface Task-Navigation Keyboard Path Recovery Packet v1

Status: consumed. The v2 workflow `FAILED` during its first browser assertion;
keyboard acceptance remains `INCONCLUSIVE`. Do not repeat this identity.

State slice: `hyperai.bazaar.interface.task_navigation_keyboard_path_recovery.v1`.
Predecessor: [`task-navigation keyboard packet v1`](compute-bazaar-interface-task-navigation-keyboard-v1-packet.md),
whose one-shot is consumed and must not be repeated.
Tool prerequisite: [`toolchain bootstrap packet v1`](compute-bazaar-interface-task-navigation-toolchain-bootstrap-v1-packet.md).
Interface direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. Before this packet was added,
`git status --porcelain=v1 -z` SHA-256 was
`38c574af683fe8c4c9b3f17fe649b328fb1e97956bd71f8ec2439d5677fbc510`; the
staged path list was empty.

## Purpose

The predecessor's offline build completed, then its browser runner failed
before browser assertions at `spawnSync lsof ENOENT`. The runner process did
not inherit `/usr/sbin`, where macOS provides `lsof`. This packet authorizes a
new one-run execution of the unchanged workflow with `/usr/sbin` explicitly
present in its `PATH`. No browser assertion, expected route, action, or product
source is changed or weakened.

## Frozen inputs

| Input | SHA-256 |
|---|---|
| `apps/hyperai-interface/index.html` | `026c82dbc0ea75b115b9428adf918e6800564425f5c19f939c334694783d38f1` |
| `apps/hyperai-interface/app.js` | `9764756b31e215e545d88ed60aa91e2ea75a281ebe1fa4601413c88061cb2ad1` |
| `apps/hyperai-interface/app.css` | `ded42d5f92c5e96495a30199bae36b8d86d2eb54f717f201f7a857c7d26f5251` |
| `scripts/verify-interface-browser.mjs` | `e8c81bfd0cd6b0f5bdad157d560dddb6a80f1f9569b7854fc606e0737af7633e` |
| `package.json` | `c7d2c372b17dabd81ec057077f847fdde0e15d32a90104cd79e7c190c5286354` |
| `Cargo.lock` | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `rust-toolchain.toml` | `7134cca955184c22a7374bda78be176b570e6d8b69ee92dc02d5efc3044212e8` |
| `scripts/with-pinned-rust.sh` | `d0bb19b6f621a31e10a7b46be852889a4f69bd0196ba7545abd9219b36f51171` |

Toolchain: macOS 26.6.1 arm64; Node.js 22.23.2; Corepack 0.33.0; pnpm
11.7.0; Rustup 1.29.0; Rust/Cargo 1.93.1 with `rustfmt` and `clippy`; and
Google Chrome 154.0.8037.92. The exact verified tools are in the existing
disposable root
`/tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e`.
The pre-existing `target/debug/hyperai-ui` binary was restored after v1 and
hashes to
`48bd6d3bb40f76f561e86e6f1607e9f0efa022dadfe94f1f3e701727993ca362`.

## Scope and inherited workflow

Exact writable repository files are this packet, `docs/state-slices.md`, the
predecessor keyboard packet, the bootstrap packet, `docs/compute-bazaar-phase-1-contract-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`. Browser HTML, JavaScript, CSS,
service/domain code, manifests, lockfiles, imported workspaces, and fixtures
are read-only. Add no dependency or source change.

This packet inherits the complete frozen workflow and proof assertions from
the predecessor: use browser-level Tab/Enter/Escape events for all 11 route
buttons and the 390-by-844 mobile navigation open/select/Escape paths; require
the exact role, accessible name, visible focus, active route, matching heading,
group order, and focus restoration; perform no DOM click, direct focus,
state mutation, form submit, or action activation; observe only same-origin
GET/HEAD requests and zero writes; and fail on browser/runtime errors,
unexpected requests, assertion mismatches, or cleanup failures. The complete
assertion text is frozen in the predecessor packet.

Allowed effects are one offline `hyperai-ui` build and one fresh isolated Chrome
run of the exact workflow mode, followed only on full workflow success by the
predecessor's `lint:fast` and import-verification commands. No live services,
external data, provider/registry/seller/agent/Fleet calls, paid Eval, financial
Trade, settlement, publication, commit, PR, or deployment.

Before execution, verify all table hashes, exact pinned tool versions, no
runner-owned service/browser process, no active listener on `127.0.0.1:4317`,
and no staged paths. Save the current `target/debug/hyperai-ui` bytes, hash,
length, and mode before the build; restore and verify them after the workflow.
Disable Corepack and Cargo networking. Keep the shared Cargo cache unchanged.

The child-process environment must include `/usr/sbin` in `PATH`; use this
ordered path prefix so both the pinned Node and `lsof` resolve:

```text
/usr/sbin:/usr/bin:/bin:/opt/homebrew/bin:/Users/shaanp/.local/lib/qwen-code/node/bin:/Users/shaanp/.cargo/bin:/tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin
```

Run this v2 identity once:

```sh
env COREPACK_HOME=/tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/corepack-home COREPACK_ENABLE_NETWORK=0 RUSTUP_HOME=/tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/rustup-home RUSTUP_TOOLCHAIN=1.93.1 CARGO_HOME=/Users/shaanp/.cargo CARGO_NET_OFFLINE=true PATH=/usr/sbin:/usr/bin:/bin:/opt/homebrew/bin:/Users/shaanp/.local/lib/qwen-code/node/bin:/Users/shaanp/.cargo/bin:/tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin /tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin/corepack pnpm run verify:interface:task-navigation-keyboard
```

If it succeeds, run under the same environment:

```sh
corepack pnpm run lint:fast
python3 scripts/verify_imports.py
```

Do not run `verify:full`. If the one-shot starts and fails or times out, do not
retry it. Record the literal result, stop, verify only runner-owned process and
port cleanup, and preserve unrelated processes and files. Remove the unique
bootstrap root only after the authorized follow-ups and all child processes
finish. Preserve the pre-existing Oct 1 browser temp directory because its
ownership is unresolved.

## Stop conditions and claim ceiling

Stop before execution on any input/tool mismatch, staged path, active service,
or inability to preserve the old binary. Stop after any failure without retry.
Claim `INTERNAL_PASS` only for this exact local synthetic keyboard workflow,
follow-ups, and cleanup if every gate passes. Otherwise report the execution
as `FAILED` or the unverified keyboard behavior as `INCONCLUSIVE`, whichever
the evidence supports. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. This does not establish physical-keyboard or screen-reader
acceptance, other browsers/platforms, full accessibility, source rights, live
integration, independent review, or production readiness. Publish boundary:
local documentation and generated local build outputs only; no commit, PR,
external write, or deployment.

## Execution record (2026-10-02)

Preflight passed: all frozen hashes and pinned versions matched; `/usr/sbin/lsof`
resolved on the child-process `PATH`; no runner process or service was active;
port 4317 was not listening; and no paths were staged. The pinned offline
build and JavaScript syntax check passed. The v2 one-shot then exited 1 during
its first keyboard browser assertion with this literal failure:

```text
Browser audit error before cleanup: keyboard navigation to Overview timed out ({"route":"overview","activeCount":1,"heading":"Compute, with context.","headingCount":1,"focus":"workspace-page-title","focusVisible":true})
HyperAI interface browser audit failed: keyboard navigation to Overview timed out ({"route":"overview","activeCount":1,"heading":"Compute, with context.","headingCount":1,"focus":"workspace-page-title","focusVisible":true})
```

The observed state already had the Overview route active, exactly one expected
heading, and visible focus on that heading, but the required keyboard
transition timed out. No assertion was weakened and the v2 identity was not
retried. `lint:fast` and import verification did not run because the main
workflow failed. The saved and current `target/debug/hyperai-ui` files both
hash to
`48bd6d3bb40f76f561e86e6f1607e9f0efa022dadfe94f1f3e701727993ca362`, with mode
755 and size 251893624 bytes.

Post-run cleanup found no runner-owned HyperAI service, Chrome remote-debugging
process, or browser runner, and no listener on port 4317. The only matching
browser temp directory remained the pre-existing one born 2026-10-01; it was
preserved. The unique bootstrap root had no open file handles and was removed.
No paths were staged. Execution is `FAILED`; keyboard behavior is
`INCONCLUSIVE`. Any further diagnosis or qualification requires another
explicitly named packet and must retain the exact browser assertions.

Follow-up record (2026-10-02): the separate
[`heading contract v1`](compute-bazaar-interface-task-navigation-heading-contract-v1-packet.md)
resolved the source-level predicate mismatch by keeping the `Overview` button
name and expecting the real `Compute, with context.` H1. Its fresh same-host
synthetic workflow passed with the route, mobile keyboard, and read-only
request assertions intact. The path-recovery execution remains `FAILED` and
its keyboard behavior remains `INCONCLUSIVE` for that consumed identity.
