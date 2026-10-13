# TUI Overview Render Contract Packet

State slice: `hyperai.bazaar.desk_runtime.tui_overview_render.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.v1` and
`global-experiment-assurance-lifecycle-v1`.

Status: `INTERNAL_PASS` for the exact in-memory TUI renderer assertion only.

## Purpose

Add a focused in-memory assertion that the existing Ratatui renderer puts the
default Overview workspace's defining content in the visible 120-by-40 frame.
This separates renderer content from the three closed installed-PTY quote
attempts, whose raw output was not retained. It does not diagnose or retry
those PTY attempts.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `crates/hyperai-tui/src/lib.rs` | `346b9c1f68d1bbc8cba70e9e1e49e7ad62a6d2d2d0288cbd92a2d2bac9deb203` | Writable focused test only |
| `crates/hyperai-tui/Cargo.toml` | `df4e070e2588b487edd6428a21d8e5a39c44adf5e2b162b262e4421f8e601ac3` | Read-only package manifest |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline validation |
| `docs/compute-bazaar-installed-tui-marker-window-packet.md` | `d9aefbc7f6eaae1f669a67a3710b6b03f53b83947a44db8af991912e1ee4f9a3` | Closed PTY attempt; read-only |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited `git status --porcelain` SHA-256 after recording this packet
and state slice, and before changing Rust source, is recorded in the state
slice below.

## Exact writable files

- `crates/hyperai-tui/src/lib.rs`: add one test that renders selected workspace
  index 0 with `ratatui::backend::TestBackend` at 120-by-40 and asserts the
  visible frame contains `Workspace: Overview`, the advisory-observation
  boundary, and the disabled Fleet/Eval statement.
- This packet and `docs/state-slices.md`: scope and result only.

Do not change the renderer, terminal event loop, CLI, service, API, package
dependencies, runtime behavior, permissions, domain contracts, or other tests.
Do not change the Python PTY harness. The three installed quote-flow packets
remain closed and consumed; no installed integration or PTY is authorized by
this slice.

## Allowed effects and proof gates

Allowed: the focused locked offline `hyperai-tui` test; root
`corepack pnpm run lint:fast`; pinned Rust formatting; `git diff --check`; and
scoped trailing-whitespace and worktree checks. No network, service process,
PTY, user-home access, provider, seller, registry, agent, remote worker, paid
execution, Fleet action, financial Trade, wallet, chain, publication, or
deployment.

Proof gates:

1. The TestBackend buffer at 120-by-40 includes the default Overview title and
   both fixed authority-boundary statements.
2. The focused test, `lint:fast`, pinned formatting, and diff hygiene pass.
3. No runtime file other than the TUI test module changes; no dependencies or
   package contents change.

## Claim ceiling

At most, the local TUI renderer places the specified Overview content in an
in-memory 120-by-40 Ratatui frame. This does not establish PTY byte rendering,
keyboard navigation, the installed TUI quote lifecycle, user comprehension,
native cross-platform support, independent review, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Publish
boundary: local source and documentation only; no staging, commit, PR,
release, publication, or deployment.

## Result

Executed 2026-10-02 from HyperAI base revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under
`hyperai.bazaar.desk_runtime.tui_overview_render.v1`.

The focused command
`CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-tui --lib tests::overview_workspace_renders_default_boundaries_in_wide_frame -- --exact`
passed (`1 passed, 0 failed`). The 120-by-40 TestBackend frame contained
`Workspace: Overview`, `Market observations are advisory.`, and `Fleet
operations and paid evaluations are not enabled.` Root
`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed: 34 interface
checks, 3 installer checks, pinned formatting, and strict all-target/all-feature
workspace Clippy. `git diff --check` passed and the scoped whitespace scan
found no trailing whitespace.

Assurance is `INTERNAL_PASS` for this in-memory render contract. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Custody is
local source and synthetic empty status only; no external data or generated
artifact was retained. Known limitations: this does not explain the missing
raw PTY marker and does not verify terminal rendering or input behavior. The
next blocked gate is a fresh, separately frozen PTY screen-observation
diagnostic before any installed TUI quote-flow attempt. Native platforms,
accessibility, independent review, and production authorization remain open.
No commit, PR, release, publication, or deployment occurred.
