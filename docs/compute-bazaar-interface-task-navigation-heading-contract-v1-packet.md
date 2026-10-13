# HyperAI Interface Task-Navigation Heading Contract Packet v1

Status: closed `INTERNAL_PASS` for the authorized narrow browser-harness
correction, one fresh synthetic keyboard qualification, two authorized local
follow-ups, and cleanup. Source diagnosis proved the Overview heading
expectation impossible.

State slice: `hyperai.bazaar.interface.task_navigation_keyboard_heading_contract.v1`.
Source diagnosis: [`task-navigation diagnostic v1`](compute-bazaar-interface-task-navigation-diagnostic-v1-packet.md).
Previous executions: keyboard v1 and path-recovery v1 are consumed. Never
repeat either identity.
Interface direction: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the dirty and untracked work preserved. Before this packet,
`git status --porcelain=v1 -z` SHA-256 was
`7630689470f737ab1fc3963bac1d62088f49957e82f620efb210764c286ce996`; no paths
were staged.

## Frozen inputs

| Input | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `e8c81bfd0cd6b0f5bdad157d560dddb6a80f1f9569b7854fc606e0737af7633e` |
| `apps/hyperai-interface/index.html` | `026c82dbc0ea75b115b9428adf918e6800564425f5c19f939c334694783d38f1` |
| `apps/hyperai-interface/app.js` | `9764756b31e215e545d88ed60aa91e2ea75a281ebe1fa4601413c88061cb2ad1` |
| `apps/hyperai-interface/app.css` | `ded42d5f92c5e96495a30199bae36b8d86d2eb54f717f201f7a857c7d26f5251` |
| `package.json` | `c7d2c372b17dabd81ec057077f847fdde0e15d32a90104cd79e7c190c5286354` |
| `Cargo.lock` | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `rust-toolchain.toml` | `7134cca955184c22a7374bda78be176b570e6d8b69ee92dc02d5efc3044212e8` |
| `scripts/with-pinned-rust.sh` | `d0bb19b6f621a31e10a7b46be852889a4f69bd0196ba7545abd9219b36f51171` |

Toolchain targets: macOS 26.6.1 arm64; Node.js 22.23.2; Corepack 0.33.0;
pnpm 11.7.0; Rustup 1.29.0; Rust/Cargo 1.93.1 with `rustfmt` and `clippy`;
Google Chrome 154.0.8037.92. The restored global Rustup executable currently
hashes to
`aeb4105778ca1bd3c6b0e75768f581c656633cd51368fa61289b6a71696ac7e1`.

## Exact correction

The navigation control name and workspace heading are different user-facing
contracts. Keep the route button's exact accessible name `Overview`; its
workspace H1 is `Compute, with context.`. Change only the task-navigation
runner data/predicate so this route has a route-specific expected H1, while the
other current empty-workspace route headings retain their existing exact
expectations. The measured browser checks must still require the correct active
route, one matching H1, exact computed heading name, visible keyboard focus,
all 11 named route buttons, Work/Research/Operate grouping, mobile open/select/
Escape behavior, and GET/HEAD-only same-origin traffic. Do not change product
HTML/CSS/JavaScript, route/action behavior, or any assertion other than
separating Overview's nav name from its real heading.

## Scope and allowed effects

Exact writable repository files are:

- `scripts/verify-interface-browser.mjs`: the single expected-heading mapping
  and the corresponding exact heading checks for this workflow mode.
- this packet and `docs/state-slices.md`.
- the predecessor v1 and path-recovery packets, the Phase 1 contract packet,
  the Rust port subplan, and the HyperAI interface plan, solely to record the
  result and link the new identity.

All browser product assets, service/domain source, package manifests, lockfiles,
imported workspaces, fixtures, datasets, and Bazaar reference files are
read-only. No domain/API/storage change or dependency edit is allowed.

### Disposable toolchain

Use only this unique temporary root for toolchain/cache/bootstrap files:

`/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e`

Use the already installed Node.js 22.23.2, Corepack 0.33.0, Chrome
154.0.8037.92, and restored Rustup 1.29.0. Download only pnpm 11.7.0 through
Corepack and Rust/Cargo 1.93.1 plus `rustfmt` and `clippy` from the official
Rust distribution. Store Corepack, Rustup, and temporary bootstrap state under
the unique root. During Rustup installation set both `RUSTUP_HOME` and
`CARGO_HOME` to temporary directories under that root and pass
`--no-self-update`; verify the global Rustup file keeps the frozen SHA-256.
For project commands, use the existing `CARGO_HOME=/Users/shaanp/.cargo`
offline crate cache with the temporary `RUSTUP_HOME`. Do not change the
`.rustup` symlink, global tools, manifests, or Cargo cache. Do not run
`pnpm install` or `cargo fetch`.

Enable Corepack networking only to fetch the exact pnpm release. Disable
Corepack networking after acquisition and disable Cargo networking for all
project commands. Include `/usr/sbin` in the browser runner's `PATH`. Before
building, save the current `target/debug/hyperai-ui` bytes, SHA-256, size, and
mode under the temporary root; restore it if changed and verify the final
attributes. Preserve unrelated build outputs and dirty work.

For project commands, set `PATH` in this order so the browser finds `lsof`,
Node resolves to 22.23.2, the temporary Corepack launcher is available, and
`with-pinned-rust.sh` can prepend the pinned Rustup toolchain:

```text
/usr/sbin:/usr/bin:/bin:/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin:/Users/shaanp/.local/lib/qwen-code/node/bin:/Users/shaanp/.cargo/bin:/opt/homebrew/bin
```

### One-run workflow

After the source patch, verify all frozen hashes except the runner hash, the
exact tool versions, no staged paths, no runner-owned process, and no listener
on `127.0.0.1:4317`. Then run exactly once:

```sh
env COREPACK_HOME=/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/corepack-home COREPACK_ENABLE_NETWORK=0 RUSTUP_HOME=/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/rustup-home RUSTUP_TOOLCHAIN=1.93.1 CARGO_HOME=/Users/shaanp/.cargo CARGO_NET_OFFLINE=true PATH=/usr/sbin:/usr/bin:/bin:/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin:/Users/shaanp/.local/lib/qwen-code/node/bin:/Users/shaanp/.cargo/bin:/opt/homebrew/bin corepack pnpm run verify:interface:task-navigation-keyboard
```

Only if it completes successfully, run:

```sh
env COREPACK_HOME=/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/corepack-home COREPACK_ENABLE_NETWORK=0 RUSTUP_HOME=/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/rustup-home RUSTUP_TOOLCHAIN=1.93.1 CARGO_HOME=/Users/shaanp/.cargo CARGO_NET_OFFLINE=true PATH=/usr/sbin:/usr/bin:/bin:/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin:/Users/shaanp/.local/lib/qwen-code/node/bin:/Users/shaanp/.cargo/bin:/opt/homebrew/bin corepack pnpm run lint:fast
env COREPACK_HOME=/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/corepack-home COREPACK_ENABLE_NETWORK=0 RUSTUP_HOME=/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/rustup-home RUSTUP_TOOLCHAIN=1.93.1 CARGO_HOME=/Users/shaanp/.cargo CARGO_NET_OFFLINE=true PATH=/usr/sbin:/usr/bin:/bin:/tmp/hyperai-interface-heading-contract-01a0d584-9ff8-73f0-aff7-7aa355e84b7e/bin:/Users/shaanp/.local/lib/qwen-code/node/bin:/Users/shaanp/.cargo/bin:/opt/homebrew/bin python3 scripts/verify_imports.py
```

Do not run `verify:full`. Verify the runner-owned service/Chrome process group
exits, reserved ports close, generated browser state is removed, the previous
binary is preserved, and no paths are staged. Remove the unique temporary root
after the workflow, any authorized follow-ups, and all child processes finish.
Preserve the pre-existing browser temp directory born 2026-10-01 because its
ownership is unresolved. If this workflow starts and fails, times out, or has
ambiguous cleanup, record the literal result and do not retry this identity.

## Non-goals and claim ceiling

No product navigation, UI content, route/API/schema, storage, domain behavior,
source integration, dependency, or platform change. No external data, provider,
registry, seller, agent, live Fleet, paid Eval, financial Trade, settlement,
publication, commit, PR, or deployment.

Claim `INTERNAL_PASS` only for the exact same-host synthetic browser workflow,
authorized follow-ups, and cleanup if every gate passes. Otherwise report the
literal execution as `FAILED` or behavior as `INCONCLUSIVE`. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. This proves no physical
keyboard or OS screen-reader behavior, other browser/platform, full workflow
accessibility, source rights, independent review, live integration, or
production readiness. Publish boundary: local source and documentation only;
no commit, PR, external write, or deployment.

## Execution record (2026-10-02)

The one authorized run completed successfully with Node.js 22.23.2, Corepack
0.33.0, pnpm 11.7.0, Rustup 1.29.0, Rust/Cargo 1.93.1, and Chrome
154.0.8037.92. The corrected runner SHA-256 is
`05ca4ed3c7ba507e2e2b06d71bfb5f49b283aedd169e38b6463d8714bedcf11a`; the
frozen UI, package, Cargo, toolchain, and wrapper hashes above remained
unchanged. The global Rustup executable retained SHA-256
`aeb4105778ca1bd3c6b0e75768f581c656633cd51368fa61289b6a71696ac7e1`.
An initial toolchain probe resolved Homebrew Rust 1.87 before the pinned PATH
was applied; this was corrected before the workflow began. The first version
probe also triggered pinned-toolchain synchronization from the repository;
subsequent installation used the disposable Rustup/Cargo homes and disabled
self-update. No project workflow ran until the pinned versions resolved, and
no global Rustup change remained.

The offline `hyperai-ui` build and browser runner passed. All 11 route buttons
retained their exact names and route-specific headings; Overview exposed the
`Overview` button name and `Compute, with context.` H1. Keyboard route focus and
the 390x844 mobile open/select/Escape/focus-restoration path passed. The runner
observed 300 same-origin GET requests and zero writes. Its service and Chrome
processes exited, ports 4317, 62350, and 62351 were closed, and generated
browser state was removed.

The authorized follow-ups also passed: `corepack pnpm run lint:fast` and
`python3 scripts/verify_imports.py`. Import verification matched the recorded
Simple Compute Market revision `a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`,
HyperIntel revision `d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`, and JustAI
revision `9a0e56536b968cbb2cb7173e142068b53c4f9fe3`. The pre-existing
`target/debug/hyperai-ui` remained SHA-256
`48bd6d3bb40f76f561e86e6f1607e9f0efa022dadfe94f1f3e701727993ca362`, mode
755, size 251893624 bytes. No paths were staged.

Assurance is `INTERNAL_PASS` for this exact same-host synthetic browser flow,
the two authorized local follow-ups, and verified process/artifact cleanup.
Reviewer status remains `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
This does not establish broad accessibility, another browser or platform,
source rights, live integration, or production readiness. The separate Data
worker isolation/platform gate remains open; overall Phase 2 remains
`NOT_ASSIGNED`.
