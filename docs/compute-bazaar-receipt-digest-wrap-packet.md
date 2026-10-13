# TUI Receipt Digest Screen-Model Continuation

State slice: `hyperai.bazaar.desk_runtime.receipt_digest_wrap_model.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.market_quote_visible_stage.v1`,
`hyperai.bazaar.desk_runtime.installed_tui_quote_postfix.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed `INTERNAL_PASS` for the fixed local screen-model predicate;
no installed rerun was performed.

## Purpose and hypothesis

The closed installed attempt displayed the receipt heading and effect-boundary
identifier but the screen model did not detect the digest. The TUI formats the
deal and receipt digests on one line. At the frozen 120-column screen width,
that line is longer than the viewport, so terminal row wrapping can place
whitespace inside the digest as represented by `visible_text()`. The harness
currently collapses whitespace to a single space before matching 64
contiguous hexadecimal characters. This is a specific, testable explanation,
not proof that the prior digest was visible or absent.

This slice repairs and tests only the bounded screen-model predicate so a
64-character lowercase SHA-256 digest remains recognizable when a terminal
wrap inserts whitespace within its visible characters. A fresh packet is
required before any installed integration attempt.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved. Before edits, SHA-256:

| File | SHA-256 | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `ecd1850422b5364017cabbda8b52ff5b0c35f83350ab6ead117535c93c098541` | Writable screen-model predicate and installed-report producer |
| `scripts/tests/test_tui_ansi_screen.py` | `7401312a1e6a80f787b9a3e8ec05da63a27791a3b851568afd66ba6790138c3f` | Writable fixed parser-contract tests |
| `crates/hyperai-tui/src/lib.rs` | `1f999f509d8e36e11b148808980e30d4ec489a24400d6ca627f127318ab71ec7` | Read-only receipt text formatting |
| `package.json` | `42e6796735cc5176feb5bb1e50ad2a908c35c56d5af6404c4be091efecb8faf7` | Read-only root fast-lint command |
| `docs/compute-bazaar-installed-tui-postfix-packet.md` | `a5cc644bee3df830f054464f86b521704885551a0681b6ea81c1d7b4a428c4d1` | Closed installed failure evidence |

After recording this packet and state entry, and before edits, the NUL-delimited
Git status SHA-256 is recorded in the state-slice entry.

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: recognize exactly 64 lowercase
  hexadecimal digest characters after the fixed `receipt:` label while
  ignoring only terminal whitespace between visible characters.
- `scripts/tests/test_tui_ansi_screen.py`: add fixed screen-model cases for a
  digest split by viewport wrapping, plus rejection of short or malformed
  values.
- This packet and `docs/state-slices.md`: record contract and evidence.

TUI product source, installed integration, dependencies, API, permissions,
service behavior, and screen dimensions are read-only.

## Allowed effects and proof gates

Allowed: run the fixed Python parser tests, Python AST parsing, root
`corepack pnpm run lint:fast`, and `git diff --check`. No PTY, installed
integration, service launch, browser, provider, seller, registry, remote
worker, paid execution, live Fleet, financial Trade, wallet, chain,
publication, or deployment.

The wrapped fixed fixture must prove the exact 64-character digest is
recognized across rows. Short and non-hex values must be rejected. Existing
screen parser fixtures remain green; the Python harness parses; root
`lint:fast` and diff hygiene pass. No raw PTY content is used or retained.

Claim ceiling: the fixed local predicate recognizes a synthetic digest in a
synthetic terminal screen model. It does not establish what was visible during
the closed PTY run, real computation, fulfillment, payment, installed quote
visibility, accessibility, independent review, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
Publish boundary: local source and documentation only. A separate frozen
packet is required for a post-repair installed integration.

## Result

The local screen-model predicate now removes terminal whitespace before
matching the exact `receipt:` label and exactly 64 lowercase hexadecimal
characters. A fixed 120-column fixture places the 64-character receipt digest
across a terminal row wrap; the previous contiguous-text matcher rejects it,
and the repaired matcher recognizes it. Fixed short, non-hex, and overlong
values are rejected.

`python3 -B -m unittest scripts.tests.test_tui_ansi_screen` passed all 6
fixtures. Python AST parsing passed. `CARGO_NET_OFFLINE=true corepack pnpm run
lint:fast` passed 34 interface tests, 3 installer tests, pinned formatting,
and strict workspace Clippy. `git diff --check` passed. No PTY, installed
integration, service, or external action ran.

Post-change SHA-256: `scripts/verify_hyperai_tui_ctrl_c.py`
`9e4030c7ba5664de0594cff0ada2c38ebe54c9e7733ea5949294535153a2badd`;
`scripts/tests/test_tui_ansi_screen.py`
`008d9d031efa19d3540d8eca2bedcb7f4e3f81ebeb7b12ea5194a4b60d83a0f8`.
Assurance is `INTERNAL_PASS` for the fixed synthetic predicate only.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The
cause of the closed PTY result remains unverified because its raw transcript
was not retained. A new frozen packet is required before an installed rerun.
