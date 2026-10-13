# HyperAI ACP Keyboard Cancellation Qualification Packet v4

Status: closed `INCONCLUSIVE`. V1, v2, and v3 also remain closed
`INCONCLUSIVE`; none of those identities may be repeated.

State slice: `hyperai.bazaar.interface.acp_cancel_keyboard.v4`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Prior outcome: [v3 packet](compute-bazaar-interface-acp-cancel-keyboard-v3-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and selected diagnostic

V3's runner observed the browser process group as empty during its wait, then
found the Chrome Crashpad helper in that same group during final cleanup. The
exact helper group was stopped after the run. The workflow report was also
suppressed by the cleanup exception, so its interaction evidence was lost.

The local source review considered these cleanup options:

1. Chrome DevTools Protocol `Browser.close` requests a graceful browser close;
   v3 added it, but the later helper observation shows it is insufficient as
   the only cleanup gate.
2. Signal the detached browser process group with `SIGTERM`, then `SIGKILL`;
   the existing runner already did this for v1/v2, whose cleanup remained
   inconclusive, so an immediate signal alone may race helper startup.
3. Independently enumerate process IDs and group IDs with macOS `ps`, require
   the group to remain absent through a bounded quiet interval, and signal the
   exact runner-owned group when any late member appears. This is the selected
   fix: it uses the system process list, needs no dependency, and lets the
   runner record interaction and cleanup outcomes separately.
4. Add macOS `libproc` descendant enumeration. This is more platform-specific
   and complex than needed for the observed exact process-group leak.
5. Add further Crashpad-disabling switches. Existing `--disable-breakpad` and
   `--disable-crash-reporter` flags did not prevent v3's helper, so this is not
   selected.

The protocol's Browser domain documents `Browser.close` as a graceful close;
Node documents that a detached child becomes a new process-group/session leader
on non-Windows hosts, and that process signaling is distinct from waiting for
the child. References: [Chrome DevTools Protocol Browser domain](https://chromedevtools.github.io/devtools-protocol/tot/Browser/#method-close),
[Node child process documentation](https://nodejs.org/api/child_process.html#optionsdetached),
and [Node process signaling](https://nodejs.org/api/process.html#processkillpid-signal).

Change only `scripts/verify-interface-browser.mjs` to: (a) retain v3's
browser-level graceful-close request; (b) use an independent `ps`-based group
membership check with a 750 ms continuously-empty interval; (c) send
`SIGTERM`, then `SIGKILL` after the fixed grace periods if any member remains;
(d) recheck group membership after service shutdown and before temporary-state
removal; and (e) emit the workflow report and cleanup report even when the
cleanup gate fails, without claiming a pass. No application, service, ACP,
fixture, or MCP behavior changes.

## Frozen baseline

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the complete current dirty and untracked worktree. Preserve all pre-existing
state, including v1-v3 records. V3's exact temporary root has been removed
after its leaked helper exited. Port `127.0.0.1:4317` had no listener at the
last read-only check; do not connect to, reserve, or modify port `4317`.

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `517f9e90f9d304065a382015d378d5646d3da2ee405c20d1c4c558f56a1f32d8` |
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

Build only the pinned offline `hyperai-ui`, `hyperai-mcp`, and
`acp-fixture-peer` binaries. Then use a disposable local service home, Chrome
profile, synthetic fixture process, and ephemeral IPv4 loopback ports. The
exact Chrome version is checked before service or browser startup. Allow only
same-origin `GET`/`HEAD` plus one HTTP 200 `POST` to each fixed route:

- `/api/v1/agent/connect`
- `/api/v1/agent/prompt`
- `/api/v1/agent/cancel`
- `/api/v1/agent/disconnect`

Do not connect to port `4317`, other local listeners, external origins,
providers, sellers, registries, chains, or reviewers. Do not launch the MCP
sidecar or call an MCP tool. Keep the browser, service, and temporary state
inside the disposable run. If cleanup fails, keep the exact temporary root,
report all workflow results already collected, and leave v4 `INCONCLUSIVE`.

## Proof gates and claim ceiling

1. JavaScript syntax and `package.json` parsing pass.
2. All fixed cancellation journey assertions pass.
3. Exactly four listed ACP POST requests occur once each with HTTP 200; there
   are zero MCP tool calls.
4. The runner independently confirms the browser group stayed absent for
   750 ms, the service exited, and the temporary state was removed only after
   those checks.
5. If the browser scope persists, exact runner-owned group membership and
   termination outcome are reported, the run is `INCONCLUSIVE`, and temporary
   state is preserved until manual cleanup.
6. `CARGO_NET_OFFLINE=true corepack pnpm run lint`, import preservation, and
   `git diff --check` are recorded. Preserve the existing JustAI
   import-inventory mismatch.

Maximum assurance is `INTERNAL_PASS` for this exact same-host Chrome synthetic
keyboard-cancellation workflow and its cleanup. Reviewer status starts
`NOT_REQUESTED`; reproducibility starts `NOT_REPRODUCED`. This does not
establish assistive technology, physical keyboard behavior, other browser or
platform behavior, broad ACP client-policy parity, Phase 8 completion, source
reuse rights, independent review, provider execution, paid execution,
federation, settlement, publication, or production readiness.

## Execution result

The exact command reached the browser runner after all pinned offline builds,
then exited 1 with `runner-owned browser process group 5830 or service PID
5525 remained active; temporary state was preserved`. The runner emitted no
workflow report, so cancellation assertions, request counts, and cleanup
details are not retained for this identity. Read-only process inspection after
the command found neither exact PID/scope active. The exact disposable
temporary root was removed only after confirming those scopes were gone; port
`4317` was not used. Assurance is `INCONCLUSIVE`; reviewer status is
`NOT_REQUESTED`, and reproduction is `NOT_REPRODUCED`.

`CARGO_NET_OFFLINE=true corepack pnpm run lint` exited 101. `lint:fast` and
`verify:contracts` passed; `verify:full` failed once in
`storage::restore_failure_tests::failed_row_copy_rolls_back_and_reports_the_safety_backup`
with `HomeBusy`. Do not retry that exact full-suite run without the required
error investigation. `python3 scripts/verify_imports.py` passed the pinned
Simple Compute Market snapshot (2,189 files), HyperIntel snapshot (99 files),
and JustAI snapshot (176 files). No import tree was changed.
