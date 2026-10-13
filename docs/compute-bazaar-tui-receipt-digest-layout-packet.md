# TUI Receipt-Digest Layout Continuation

State slice: `hyperai.bazaar.desk_runtime.tui_receipt_digest_layout.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.installed_tui_receipt_frame_wait.v1`,
`hyperai.bazaar.desk_runtime.tui_receipt_frame_wait.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed `INTERNAL_PASS` for the local digest layout and TestBackend
viewport contract; no installed rerun was performed.

## Diagnosis and selected change

Three installed attempts failed the same digest-visibility assertion. The
latest bounded diagnostic observed the completion message, receipt heading,
`receipt:` label, and effect-boundary identifier, but no digest. The raw PTY
text was not retained, so visual absence versus an ANSI model defect remains
unresolved.

The current TUI places both 64-character deal and receipt digests on one
146-character line. The detail panel at a 120-column terminal is 94 columns
wide after its border. Ratatui 0.30.2 wraps paragraph text, but the current
combined digest row is hard to inspect and makes the visibility contract
ambiguous. This slice puts the deal digest and receipt digest on separate
labeled lines and adds a TestBackend assertion that the complete receipt line
fits on one visible row in the existing 120-by-40 screen. This is a
readability and visibility improvement; it does not claim to diagnose the
closed PTY runs.

Candidates considered:

1. Render deal and receipt digests on separate labeled lines, then prove the
   full receipt line in a fixed 120-by-40 TestBackend frame. Selected: each
   value is independently readable and the receipt line is 73 columns, below
   the 94-column content width.
2. Use Paragraph scrolling or unstable `line_count` to bring the combined line
   into view. Not selected: it requires scrolling policy and can hide buyer
   action context.
3. Split Market into fixed lifecycle and detail panels. Not selected: broader
   interaction redesign than the local visibility change requires.
4. Increase installed PTY dimensions or add a delay. Not selected: it masks
   screen behavior without changing layout or establishing visibility.
5. Further amend the generic ANSI screen model. Not selected yet: safe
   diagnostics show the digest predicate false after a full wait, so establish
   the TUI's local rendered row first.

Reference: [Ratatui Paragraph](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Paragraph.html)
and [Ratatui Wrap](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Wrap.html).

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved. Before edits, SHA-256:

| File | SHA-256 | Use |
|---|---|---|
| `crates/hyperai-tui/src/lib.rs` | `1f999f509d8e36e11b148808980e30d4ec489a24400d6ca627f127318ab71ec7` | Writable Market lifecycle layout and TestBackend test |
| `scripts/verify_hyperai_tui_ctrl_c.py` | `564dfb676dc73854939c960e4c76fcabd292211a67fd185eeda970ac6ff2ccbd` | Writable digest observer label |
| `scripts/tests/test_tui_ansi_screen.py` | `517ca48f5ab04cf3007e4ca6683c96b0f04015f9536fe3f8218006b1b3532b51` | Writable fixed label predicate test |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only pinned Ratatui version and workspace gates |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Read-only dependency lock |
| `docs/compute-bazaar-installed-tui-complete-frame-packet.md` | `1f357761fc5d029698ec52d9c2cbe66e5383b2c02e5412092e365c43609c388d` | Closed installed failure evidence |
| `docs/compute-bazaar-tui-receipt-frame-wait-packet.md` | `6ee673def72490b0e21158aa60bb785f9c080c89424761474606310335d90fc4` | Completed local observer wait change |

The NUL-delimited Git status digest after recording this packet and state
entry, and before edits, is recorded in the state-slice entry.

## Exact writable files

- `crates/hyperai-tui/src/lib.rs`: render deal and receipt digests on separate
  lines; update the completed-state TestBackend fixture to use a fixed
  64-character digest and assert that the entire labeled receipt line appears
  within one row of the 120-by-40 buffer.
- `scripts/verify_hyperai_tui_ctrl_c.py`: update the fixed digest label used by
  the screen observer and timeout diagnostics.
- `scripts/tests/test_tui_ansi_screen.py`: update the local fixed label case.
- This packet and `docs/state-slices.md`: record scope and outcome.

No service/API/receipt contract, quote semantics, dependencies, screen size,
installer, or installed test source changes are authorized.

## Allowed effects and proof gates

Allowed: focused locked offline `hyperai-tui` TestBackend test, fixed Python
screen-model fixtures, Python AST parsing, root
`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast`, and `git diff --check`.
No PTY, installed integration, service launch, provider, seller, registry,
remote worker, paid execution, live Fleet, financial Trade, wallet, chain,
publication, or deployment.

The 64-character receipt digest and label must appear together on one visible
buffer row at 120-by-40. Existing TUI and Python screen tests, Python AST,
root `lint:fast`, and diff hygiene must pass.

Claim ceiling: readable digest layout and local TestBackend visibility only.
The prior installed failures remain `INCONCLUSIVE` about actual digest
visibility. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. A fresh separate packet is required before any installed
rerun.

## Result

The Market lifecycle now renders `Deal digest:` and `Receipt digest:` on
separate lines. Its TestBackend fixture uses an exact 64-character synthetic
digest and asserts that the entire `Receipt digest: <digest>` line fits on one
row in the 120-by-40 frame.

An initial focused Rust command included `--exact` and matched zero tests. The
corrected command
`CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked
--offline -p hyperai-tui
completed_market_quote_receipt_is_visible_with_comparison_details` passed
(`1 passed`). All 7 Python screen-model fixtures passed; Python AST parsing
passed. `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed 34
interface tests, 3 installer tests, pinned formatting, and strict workspace
Clippy. `git diff --check` passed. No PTY, installed integration, service, or
external action ran.

Post-change SHA-256: `crates/hyperai-tui/src/lib.rs`
`b45d17fb1a30b02ddc906986bf908b9b925c3c0d6306e936d503825e5aa97177`;
`scripts/verify_hyperai_tui_ctrl_c.py`
`ee9caf5978f6fca45e79d6f143df9237279e6ca9fee37e5aaba4e642caefd2f7`;
`scripts/tests/test_tui_ansi_screen.py`
`2ea06458415ade3ce66751f7d004d90aaf5941a5a8f5f21a58b0846e713c2971`.
Assurance is `INTERNAL_PASS` for local layout and fixed synthetic observer
fixtures only. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. The installed quote path remains unqualified; a separate
fresh packet is required before any installed run.
