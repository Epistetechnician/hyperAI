# TUI Receipt Frame Completion Wait

State slice: `hyperai.bazaar.desk_runtime.tui_receipt_frame_wait.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.installed_tui_receipt_wrap.v1`,
`hyperai.bazaar.desk_runtime.receipt_digest_wrap_model.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed `INTERNAL_PASS` for local chunked-screen observer behavior; no
installed rerun was performed.

## Diagnosis and hypothesis

Two installed attempts failed at the same digest-visibility assertion. The
matcher-only repair did not change the result. The PTY flow presses `c`, waits
only until the receipt heading appears, and immediately samples the modeled
screen. Its marker wait returns on the first readable chunk containing that
heading; there is no completion-digest wait or post-heading drain. The
remaining frame could still be pending on the PTY when the snapshot is taken.
This is a plausible cause supported by the observer control flow, not proven
by retained PTY evidence. Raw terminal bytes from both attempts were not
retained.

The Python `select()` documentation defines readability as readiness for
reading and returns the descriptors ready; it does not assert that an entire
terminal frame has arrived. Python's `pty` documentation describes programmatic
reads from the child's controlling terminal. References:
[Python select documentation](https://docs.python.org/3/library/select.html)
and [Python 3.14 pty documentation](https://docs.python.org/3.14/library/pty.html).

## Candidate fixes

1. Wait until the exact receipt-digest predicate is true, continuing to read
   PTY output until the existing bounded timeout. Selected: it waits for the
   evidence being claimed and adds no fixed delay.
2. Wait for the success-message marker, then sample immediately. Not selected:
   the message precedes the digest and can arrive in the same partial frame.
3. Drain output for a fixed interval after the heading. Not selected: a fixed
   sleep is machine-load dependent and can still sample early.
4. Wait for the cursor position to remain stable over multiple reads. Not
   selected: Ratatui cursor stability does not establish frame completion, and
   redraws may continue after the cursor rests.
5. Reformat the product digest onto its own line. Not selected for this
   diagnosis: it could improve narrow-terminal readability, but it does not
   prevent the observer from returning after a partial frame.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved. Before edits, SHA-256:

| File | SHA-256 | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `9e4030c7ba5664de0594cff0ada2c38ebe54c9e7733ea5949294535153a2badd` | Writable PTY observer |
| `scripts/tests/test_tui_ansi_screen.py` | `008d9d031efa19d3540d8eca2bedcb7f4e3f81ebeb7b12ea5194a4b60d83a0f8` | Writable fixed observer fixtures |
| `crates/hyperai-tui/src/lib.rs` | `1f999f509d8e36e11b148808980e30d4ec489a24400d6ca627f127318ab71ec7` | Read-only TUI completion rendering |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only installed test caller |
| `docs/compute-bazaar-installed-tui-receipt-wrap-packet.md` | `012d739e5770e1d0233f63bd9148dd86729d6cefd4ca9fc55e781eb354bc13be` | Closed second installed failure |
| `docs/compute-bazaar-receipt-digest-wrap-packet.md` | `1b9b8e84cb4625e276a0e015627ba00a2bd904536bb5ab77ea405180b83e002c` | Completed matcher-only repair |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only root fast-lint build graph |

The NUL-delimited Git status digest after recording this packet and state
entry, and before edits, is recorded in the state-slice entry.

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: wait for the exact visible receipt
  digest predicate after `c`, reading further PTY output until the digest is
  present or the existing bounded timeout expires. Preserve fixed-token-only
  timeout diagnostics and never emit raw terminal content.
- `scripts/tests/test_tui_ansi_screen.py`: add deterministic chunked-screen
  cases proving that the receipt heading alone does not satisfy the completion
  predicate and that the later complete digest does.
- This packet and `docs/state-slices.md`: record scope and outcome.

Product source, integration call site, dependencies, service, API, permissions,
PTY dimensions, and terminal resize behavior are read-only.

## Allowed effects and proof gates

Allowed: fixed Python parser/observer tests, Python AST parsing, root
`corepack pnpm run lint:fast`, and `git diff --check`. No PTY launch, installed
integration, service launch, browser, provider, seller, registry, remote
worker, paid execution, live Fleet, financial Trade, wallet, chain,
publication, or deployment.

The local fixture must show that a heading-only chunk leaves digest visibility
false and a later digest chunk makes it true. Existing fixtures, Python AST,
root fast lint, and diff hygiene must pass. No raw PTY text is persisted.

Claim ceiling: a deterministic local observer waits for fixed synthetic screen
text. It does not prove the cause of either closed installed result or qualify
the installed quote lifecycle. Reviewer status is `NOT_REQUESTED`;
reproduction is `NOT_REPRODUCED`. Another installed attempt requires a
separate fresh packet after this repair.

## Result

The harness now waits for the exact visible 64-character receipt-digest
predicate after pressing `c`. It continues reading modeled terminal chunks
until the digest appears or the existing five-second deadline expires. The
timeout report adds fixed receipt-label and digest-presence booleans only; it
does not retain or print terminal text.

`python3 -B -m unittest scripts.tests.test_tui_ansi_screen` passed all 7
fixtures, including a heading-only chunk followed by the remaining digest.
Python AST parsing passed. `CARGO_NET_OFFLINE=true corepack pnpm run
lint:fast` passed 34 interface tests, 3 installer tests, pinned formatting,
and strict workspace Clippy. `git diff --check` passed. No PTY, installed
integration, service, or external action ran.

Post-change SHA-256: `scripts/verify_hyperai_tui_ctrl_c.py`
`564dfb676dc73854939c960e4c76fcabd292211a67fd185eeda970ac6ff2ccbd`;
`scripts/tests/test_tui_ansi_screen.py`
`517ca48f5ab04cf3007e4ca6683c96b0f04015f9536fe3f8218006b1b3532b51`.
Assurance is `INTERNAL_PASS` for fixed local observer behavior only.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The two
closed installed results remain unresolved; a separate new packet is required
for another attempt.
