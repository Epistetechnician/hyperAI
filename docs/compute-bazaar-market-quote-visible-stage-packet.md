# Market Quote Stage Visibility Continuation

State slice: `hyperai.bazaar.desk_runtime.market_quote_visible_stage.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.v1`,
`hyperai.bazaar.desk_runtime.installed_tui_screen_model.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed `INCONCLUSIVE` for its one installed synthetic quote-flow
attempt; local renderer ordering is `INTERNAL_PASS` after the selected fix.

## Purpose and hypothesis

The installed screen-model run saw Overview, Market, quote preview, and
approval, but did not see the receipt after `c`. The display helper appends
quote lifecycle text after signed-offer and advisory rows, while the TUI
renders the whole text in a non-scrolling paragraph. Receipt clipping is a
plausible cause, not a confirmed one. This slice first adds a TestBackend
render assertion for a completion state with realistic offer and observation
rows. If the receipt is outside the 120-by-40 frame, move the active quote
stage above those long details so the current state stays visible. Add fixed
screen diagnostic booleans for completion-success and completion-error copy.

## Diagnosis and fix selection

The TestBackend case passed with two offer rows, two observation rows, long
synthetic claim text, and a completed receipt. In the installed run, the
completion-success message was visible after `c`, while the receipt heading
and effect-boundary identifier were absent. Source confirms that the success
message is set only on a successful completion response, then rendered before
the lifecycle receipt lines. The renderer wraps the whole market summary in a
bounded non-scrolling `Paragraph`, so the screen evidence points to trailing
lifecycle lines falling below the visible viewport. This remains an
inference: raw terminal bytes were not retained, and the screen model is a
bounded parser rather than a full terminal emulator.

Candidate fixes considered:

1. Move the latest quote/deal lifecycle section directly after the buyer
   summary and controls, before signed-offer and advisory detail. This keeps
   the current action result high in the screen, preserves all data, needs no
   dependency, and is selected.
2. Dynamically scroll the `Paragraph` to its final lines. Ratatui documents
   `Paragraph::scroll`, but calculating wrapped line count uses an unstable
   optional API or duplicates wrapping logic. It can also hide the buyer
   request and action context, so it is not selected.
3. Split Market into a fixed lifecycle panel and a separate detail viewport.
   This offers stronger persistent visibility but is a larger interaction
   redesign than the observed issue requires.
4. Increase PTY dimensions or add a fixed delay. This would mask the user's
   current-screen problem and would not change the client layout, so reject.
5. Change the ANSI parser or key delivery. The parser observed the success
   message, which source sets only after completion succeeds; that makes
   missing input or a service failure less likely, though not impossible.

Reference: [Ratatui Paragraph wrapping, scrolling, and line-count stability](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Paragraph.html).

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `crates/hyperai-tui/src/lib.rs` | `74cc744b085abd8bec62323b624881ea3808a1090c85e9f1231201d20e71c64d` | Writable renderer, layout, and TestBackend fixture |
| `scripts/verify_hyperai_tui_ctrl_c.py` | `e15bfe659ed3cc06651661c41cef25efa730f2c510fc429ecd1661341563bc3f` | Writable fixed PTY diagnostics |
| `scripts/tests/test_tui_ansi_screen.py` | `7401312a1e6a80f787b9a3e8ec05da63a27791a3b851568afd66ba6790138c3f` | Read-only parser tests |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only installed integration |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53ceebd44a7b9ddf49b3` | Read-only offline installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline build |
| `docs/compute-bazaar-installed-tui-screen-model-packet.md` | `47489229cd4b155d091a14079abcf1310f2470e4ec6af7a169981e0b87be307c` | Closed PTY result |
| `docs/compute-bazaar-tui-overview-render-packet.md` | `8935091657ef418936094683e1b279571d41bbd65ac4ad7a1fe7ed07215bb5c1` | Read-only renderer evidence |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited `git status --porcelain` SHA-256 after recording this packet
and state slice, and before changing TUI or harness source, is recorded below.

## Exact writable files

- `crates/hyperai-tui/src/lib.rs`: add a TestBackend test with fixed synthetic
  comparison/preview/approval/completion records and two signed-offer plus two
  advisory rows. If the completed receipt is clipped, place the quote lifecycle
  before the long listing and observations.
- `scripts/verify_hyperai_tui_ctrl_c.py`: add only fixed, non-sensitive marker
  booleans for successful and failed mock completion to timeout diagnostics.
- This packet and `docs/state-slices.md`: record scope and outcome.

Do not change API/domain semantics, lifecycle requests, permissions,
dependencies, installer, package contents, CLI, service, or quote authority.
The one installed test remains local, offline, synthetic, and mock-only.

## Allowed effects and one-run limit

Allowed: the focused locked offline `hyperai-tui` renderer test; four Python
screen-model fixtures; Python AST parsing; exactly one locked offline ignored
installed integration:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The integration uses the offline installer, a disposable install root/home,
fixed synthetic quote and ACP fixtures, loopback port `0`, and a POSIX PTY.
Before execution, verify all frozen read-only hashes, port-0 paths, and that
temporary paths are outside the user-owned HyperAI home. Stop on drift, port
`4317`, or a temporary path inside that home. Also allowed: root
`corepack pnpm run lint:fast`, pinned Rust formatting, `git diff --check`, and
scoped whitespace/process/temp-directory cleanup checks. No provider, seller,
registry, remote worker, external agent, paid execution, live Fleet, financial
Trade, wallet, chain, non-loopback network, publication, or deployment.

The installed integration allowance is exactly one. Any failure or
inconclusive result consumes it; a further attempt requires a fresh packet.

## Proof gates and claim ceiling

The completed receipt heading, receipt digest, and fixed synthetic
effect-boundary identifier must be present in the 120-by-40 TestBackend
viewport and in the installed TUI screen after preview and approval. The
installed integration must pass its existing resize, Ctrl+C, restoration,
cleanup, CLI, and ACP assertions. Parser fixtures, `lint:fast`, formatting,
and diff hygiene must pass.

At most, this establishes visible local synthetic quote completion in the TUI
and the enclosing installed integration on this Darwin arm64 host. It does not
establish computation correctness, real fulfillment/payment, native platform
support, accessibility, independent review, or production readiness. Reviewer
status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Publish boundary:
local source and documentation only; no staging, commit, PR, release,
publication, or deployment.

## Result

Executed 2026-10-02 from HyperAI base revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under
`hyperai.bazaar.desk_runtime.market_quote_visible_stage.v1`.

The one locked offline installed integration exited 101 (`0 passed, 1
failed`) after 17.32 seconds. It observed Overview, Market, preview, and
approval in order, then failed to see the receipt heading after `c`. Safe
diagnostics showed `Mock fulfillment and receipt completed.` present and
`Mock completion failed:` absent, while the receipt heading and effect-boundary
identifier were absent. This indicates the completion request returned
successfully but the receipt text was not visible in the modeled screen. It
reported 635 CSI sequences, one alternate-screen entry, 8,015 transcript
bytes, and SHA-256
`156dc50bddaae91099792534cb8e18009b97faf26021cf4a4707a51a25d4d001`; no raw
terminal text was retained. The test stopped before resize, Ctrl+C, and later
ACP checks.

The TestBackend fixture with two offer rows, two observations, and long
synthetic claim text passed at 120-by-40. The source places the success message
before the quote lifecycle section and renders the full summary in a
non-scrolling paragraph. Research considered moving lifecycle details,
dynamic paragraph scrolling, a split lifecycle/detail layout, increasing PTY
dimensions, and changing parser/input behavior. The selected low-scope fix
moved the current quote lifecycle directly after buyer controls and before
long details. The updated TestBackend test asserts the visible receipt,
receipt digest, effect-boundary identifier, and that this section precedes
`SIGNED OFFERS`; it passed (`1 passed, 0 failed`).

The four parser fixtures and Python AST checks passed. Root
`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed 34 interface checks,
3 installer checks, pinned formatting, and strict workspace Clippy.
`git diff --check` and scoped whitespace checks passed. The installed attempt predates
the layout change; this packet's one-run allowance is consumed and was not
repeated. Assurance is `INCONCLUSIVE` for installed quote visibility and
`INTERNAL_PASS` for the local renderer ordering. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Custody is local source,
fixed synthetic quote/ACP data, and bounded in-memory diagnostics only. No
provider, seller, paid execution, chain, or external effect occurred. The next
blocked gate is a fresh post-fix installed integration packet. Platform,
accessibility, independent review, and production gates remain open. No
commit, PR, release, publication, or deployment occurred.
