# HyperAI Interface Task-Navigation Toolchain Bootstrap Packet v1

Status: completed disposable toolchain bootstrap. The dependent v1 keyboard
qualification is consumed and failed before browser assertions; its separately
packeted path-recovery continuation may reuse the verified root.

State slice: `hyperai.bazaar.interface.task_navigation_keyboard_toolchain_bootstrap.v1`.
Parent state slice: `hyperai.bazaar.interface.task_navigation_keyboard.v1`.
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. Before this packet,
`git status --porcelain=v1 -z` SHA-256 was
`485008de90b4a313a9d19e1004037e406e0d4c33d092bcad54c4864fd7d09196`.

## Purpose

The keyboard workflow and its package-script entrypoint are already wired,
but its one-shot packet requires Node.js 22.23.2, pnpm 11.7.0, Rust/Cargo
1.93.1, and Google Chrome 154.0.8037.92. The host has the pinned Node and
Chrome, while pnpm 11.7.0 is not installed, Rustup's configured home is an
unavailable volume, and the installed compiler is Rust 1.87.0. This packet
permits acquiring only the exact missing local development tools into one
disposable directory, then running the original frozen workflow once.

The original keyboard packet was frozen when this prerequisite packet was
written. Its v1 one-shot is now consumed and must not be repeated. The
separately named path-recovery packet preserves its offline build, same-origin
request, read-only browser, and cleanup requirements.

## Recovery after unexpected Rustup self-update

The first Rust toolchain-install invocation unexpectedly upgraded the global
Rustup manager from 1.29.0 to 1.29.1 even though `RUSTUP_HOME` was temporary;
it also removed the previously present `rustup-init` helper. At that point, the
browser workflow had not started. Restore the previously observed 1.29.0 executables
before continuing: use Rustup's official update mechanism with
`RUSTUP_VERSION=1.29.0`, but set both `RUSTUP_HOME` and `CARGO_HOME` to
temporary paths first; verify that temporary binary reports 1.29.0; then
restore only the existing
`/Users/shaanp/.cargo/bin/rustup` and
`/Users/shaanp/.cargo/bin/rustup-init` paths from that exact artifact and
record their SHA-256 values. Do not install a default toolchain into the
global home. For the pinned Rust toolchain install, pass `--no-self-update`
and keep `RUSTUP_HOME` temporary. If restoration fails or reports another
version, stop before running the browser workflow.

## Frozen tool and source identities

- Repository package manager: pnpm 11.7.0, fixed by `package.json`.
- Node.js: 22.23.2 at
  `/Users/shaanp/.local/lib/qwen-code/node/bin/node`.
- Corepack: 0.33.0 at
  `/opt/homebrew/Cellar/node/24.3.0/lib/node_modules/corepack/dist/corepack.js`;
  it runs under the pinned Node.js executable. Do not use the installed
  Node-24 Corepack launcher or the incompatible Node-22 shim.
- Rustup: 1.29.0 at `/Users/shaanp/.cargo/bin/rustup`.
- Rust/Cargo toolchain: 1.93.1, with `rustfmt` and `clippy` components.
- Browser: Google Chrome 154.0.8037.92.
- Exact keyboard workflow inputs and their frozen hashes remain in the
  [parent packet](compute-bazaar-interface-task-navigation-keyboard-v1-packet.md).

## Scope and allowed effects

Exact writable repository files are this packet and `docs/state-slices.md`.
The parent keyboard packet additionally authorizes recording its result in
`docs/compute-bazaar-interface-task-navigation-keyboard-v1-packet.md`,
`docs/compute-bazaar-phase-1-contract-packet.md`,
`docs/compute-bazaar-rust-port-subplan.md`, and
`docs/bazaar-hyperai-interface-plan.md`.

The only permitted downloads are pnpm 11.7.0 through Corepack, Rust/Cargo
1.93.1 plus `rustfmt` and `clippy` from the official Rust distribution, and
the Rustup 1.29.0 recovery artifact described above. Use their
package-manager/toolchain integrity checks. Store Corepack data, Rustup state,
the temporary Corepack launcher, logs, and other bootstrap state only beneath:

`/tmp/hyperai-interface-toolchain-bootstrap-01a0d584-9ff8-73f0-aff7-7aa355e84b7e`

Use `COREPACK_HOME`, `RUSTUP_HOME`, and the recovery `CARGO_HOME` inside that
directory. Keep `CARGO_HOME=/Users/shaanp/.cargo` for the existing locked
offline crate cache during project commands; do not install packages there.
Do not change the default `.rustup` symlink, global Node/Corepack/pnpm
installations, project manifests, lockfiles, or Cargo configuration. Do not
run `pnpm install`, `cargo fetch`, or any workspace dependency update.

After the exact tools are installed, set the temporary Corepack launcher to
invoke the pinned Node.js executable and installed Corepack JS module. Disable
Corepack network access and Cargo network access for all project commands.
Before the build, if `target/debug/hyperai-ui` exists, save a temporary copy
under the bootstrap root and record its SHA-256, byte length, and mode. After
the workflow and follow-up checks, restore that exact binary and verify those
three attributes. Leave other Cargo build-cache outputs intact; they are
generated by the packet-authorized build and are not source inputs.

Run exactly once:

```sh
corepack pnpm run verify:interface:task-navigation-keyboard
```

Only if that one-shot completes, run the parent packet's follow-up commands:

```sh
corepack pnpm run lint:fast
python3 scripts/verify_imports.py
```

Record exact tool versions, source/input digests, command results, and cleanup
in the named local documents. Remove the unique temporary bootstrap directory
after all child processes exit. Preserve all pre-existing dirty or untracked
work. Stage nothing.

## Stop conditions and non-goals

Stop before the browser workflow if a downloaded tool reports a different
version, integrity validation fails, a frozen browser/Cargo input differs,
the original workflow has already been consumed, or a pre-existing service
on `127.0.0.1:4317` would be contacted. Do not retry the workflow after it
starts, including after a timeout or cleanup failure; preserve the literal
result and open a fresh state slice for any continuation.

No source changes, dependency or lockfile changes, Bazaar/SCM data access,
provider or registry calls, external ACP agents, seller calls, Fleet
operations, paid Eval, financial Trade, settlement, publication, commit, PR,
or deployment are in scope. Fleet live operations, paid evaluations, and
financial Trade remain under their separate authority gates.

## Proof gates and claim ceiling

Before the one-shot, verify its original task packet is still unconsumed and
its read-only browser assets and locked Rust inputs match. During it, preserve
the original same-origin GET/HEAD-only policy, keyboard-only workflow,
disposable service home, and Chrome process cleanup. Afterwards, record
`INTERNAL_PASS` only if every parent-packet assertion, follow-up command,
import inventory, process/port cleanup, temp-directory cleanup, and unstaged
worktree check passes. Otherwise record the exact failure as `INCONCLUSIVE`
or `FAILED` without rerunning the one-shot.

This packet's claim ceiling is the exact local toolchain bootstrap and the
parent packet's same-host macOS 26.6.1 arm64 Chrome workflow. It does not
establish physical keyboard or screen-reader acceptance, another browser or
OS/architecture, native platform support, clean reproduction, independent
review, Bazaar reuse rights, live integrations, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
Publish boundary is local documentation and disposable local toolchain state;
no commit, PR, external write, or deployment.

## Execution and recovery record (2026-10-02)

The isolated bootstrap completed after recovery. Corepack/pnpm reported
`0.33.0` / `11.7.0`; Rustup reported `1.29.0`; the isolated toolchain reported
`rustc 1.93.1 (01f6ddf75 2026-02-11)` and Cargo `1.93.1`, with `rustfmt` and
`clippy` installed. Project commands used the existing Cargo cache without
installing packages. The restored global `rustup` and `rustup-init` paths both
hash to
`aeb4105778ca1bd3c6b0e75768f581c656633cd51368fa61289b6a71696ac7e1`; the
global manager again reports `1.29.0`. The pinned Rustup invocation thereafter
used `--no-self-update`.

The parent v1 workflow then ran once and returned exit status 1 after the
offline binary build, before any browser assertion. Its literal failure was:
`Browser audit error before cleanup: spawnSync lsof ENOENT`, followed by
`HyperAI interface browser audit failed: spawnSync lsof ENOENT`. The execution
environment's `PATH` omitted `/usr/sbin`, where macOS provides `lsof`. This
consumed the v1 one-shot; it was not retried. No `lint:fast` or import-inventory
follow-up ran because the one-shot did not complete. The preserved
`target/debug/hyperai-ui` was restored byte-for-byte; both its saved copy and
the restored binary hash to
`48bd6d3bb40f76f561e86e6f1607e9f0efa022dadfe94f1f3e701727993ca362`.

After the failed run, process inspection found no runner-owned HyperAI service,
Chrome remote-debugging process, or browser runner; TCP port 4317 was not
listening. The only matching browser temp directory found was
`/var/folders/qd/xlj1mz_x2t92gdst9km986p00000gn/T/hyperai-interface-browser-PJbGjW`,
whose recorded birth and modification times are 2026-10-01 08:19:33 and
08:19:38. Because it predates this run and ownership is unproven, it was
preserved. Do not remove it under this packet.

The verified bootstrap root was reused only by the separately named v2
continuation packet with `/usr/sbin` added to the child-process `PATH`. That
workflow failed during its first browser assertion and was not retried. No
`lint:fast` or import follow-up ran. The previous binary remained unchanged;
no runner-owned process, service, or port remained. After confirming there
were no open files, the bootstrap root was removed. The v1 and v2 workflow
identities are both consumed and must not be run again.
