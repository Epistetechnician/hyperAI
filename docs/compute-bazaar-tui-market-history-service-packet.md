# Populated TUI Market History Service Packet

State slice: `hyperai.bazaar.desk_runtime.tui_market_history_service_pty.v1`.
Parent: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Related contracts: [Phase 1 service and compatibility packet](compute-bazaar-phase-1-contract-packet.md), [Market history client pagination packet](compute-bazaar-market-history-client-pagination-packet.md), and [provenance](provenance.md).

Status: accepted local synthetic integration packet. It authorizes only the
exact disposable-home PTY flow below. It does not authorize changes to the
TUI's API, market domain, service, storage, page size, imported products, or
external operations.

## Frozen baseline

- Repository: HyperAI, branch `main`, base
  `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `2e790a26c7de5857efdf4659a4e9f9f54fb9890cb7ce57cc12528f0784081b00`
  (172 existing status records). Preserve all unrelated dirty and untracked
  work.
- Read-only runtime/API inputs:

| Path | SHA-256 |
|---|---|
| `crates/hyperai-tui/src/lib.rs` | `41a44749ed6e136a72d8554d6dbf99e4ed506527a75bcc2933e56968883b1a94` |
| `crates/hyperai-local-service/src/server.rs` | `0e42d27e5a58d82bba04296740bc3e35411f640f1ddf78122b66b816a1562602` |
| `crates/outcomes-cli/src/lib.rs` | `b615e25e29f1569b8d3ed0dbd1914d6418f27a2dd75e6c85b7f9c77f083b1945` |
| `docs/compute-bazaar-phase-1-contract-packet.md` | `bae05fbeaed2431322aba4207fe4f23867c485158b38d7c5fdbdd40e74263993` |
| `docs/provenance.md` | `6e6b0ef176f996b1cafb2e0bf91264fc4dd2f11fda687d949466b2183b966ade` |

- Initial harness and command inputs:

| Path | SHA-256 |
|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `ee9caf5978f6fca45e79d6f143df9237279e6ca9fee37e5aaba4e642caefd2f7` |
| `package.json` | `5240bc37e3fbbc1fb99b44bbc351efb1b1eed789fd58bfd6fa27d0ba5bcd07c9` |

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: add one opt-in populated-history
  PTY mode and its synthetic CLI setup. Preserve existing default, panic, and
  quote-lifecycle modes.
- `package.json`: add `verify:tui-market-history`, building the pinned
  `hyperai` binary offline and invoking only the named PTY mode.
- `docs/compute-bazaar-tui-market-history-service-packet.md`: this packet and
  exact evidence.
- `docs/state-slices.md`: this slice and result.
- `docs/compute-bazaar-rust-port-subplan.md`: concise status and limitation.

The TUI library, local service, Market/application contracts, SQLite schema,
CLI implementation, browser, ACP/MCP, imported products, and Bazaar reference
are read-only. No dependency, lockfile, route, schema, or package behavior
change is permitted. If the scope needs another file or contract change, stop
and amend this packet first.

## Objective and allowed effects

Prove one populated interactive TUI Deals view reads the real authenticated
local service and displays a valid Market-owned history entry. Use the existing
`baseline` synthetic comparison, quote preview, buyer approval, and
deterministic mock completion in a fresh temporary HyperAI home. Start the
existing service on port `0`, invoke only its existing CLI/API routes, run the
installed `hyperai desk` binary in a POSIX PTY, navigate from Overview to
Deals, and press `n` after the service has returned the single terminal page.

The TUI's production page limit remains 20. This fixture contains one valid
deal and therefore proves populated first-page rendering and terminal-page
interaction only. It is not evidence of multi-page TUI-to-service traversal;
the current two closed synthetic quote scenarios cannot create more than two
unique valid deal digests. Do not forge rows, duplicate approvals, alter the
service page size, or expand the fixture set in this slice.

Effects are restricted to the exact synthetic approval and mock receipt in
the test-owned temporary home, authenticated IPv4 loopback requests to that
home's service, PTY key input, and temporary build/runtime files under the
existing ignored `target/` directory. Do not contact the pre-existing service
on port `4317` or any external endpoint.

## Proof gates

1. The test setup creates exactly one valid baseline deal through the existing
   authenticated CLI and confirms its completion receipt before launching the
   TUI.
2. The actual `hyperai desk` loop enters raw mode and the alternate screen;
   PTY key events navigate to Deals and activate `n`.
3. The visible Deals screen contains the exact seeded deal and preview
   digests, the mock-completed label, the receipt digest, and the terminal-page
   message. After `n`, it shows the no-additional-page message. No write,
   provider, seller, payment, settlement, or chain action occurs.
4. The TUI exits cleanly, restores termios, alternate screen, and cursor; the
   service stops and its temporary home is removed.
5. Run the named `corepack pnpm run verify:tui-market-history`, existing
   `corepack pnpm run verify:tui-ctrl-c`, `corepack pnpm run lint:fast`,
   `python3 scripts/verify_imports.py`, and `git diff --check`. Do not run root
   `verify:full` or `pnpm run lint` under the root focused-coverage state slice.
   Check authored untracked files for trailing whitespace separately.
6. Record exact command output, host/tool versions, binary digest, transcript
   digest, changed paths, final worktree status digest, and cleanup. Preserve
   all unrelated work and stage nothing.

## Execution evidence (2026-10-02)

The first PTY attempt seeded and rendered the synthetic deal, then failed
because the harness checked for the terminal-page line immediately after the
deal row and digests first appeared in the captured screen. No product source
changed. The harness was corrected to wait for the explicit terminal-page
marker and to report only fixed-marker presence on timeout; the fresh run
passed.

`corepack pnpm run verify:tui-market-history` exited 0 on macOS Darwin arm64
with Python 3.14.5. The pinned offline build produced `target/debug/hyperai`
with SHA-256
`5e3de9899940d760b1cb0f4da7e7e7813cfc5f85966b7521a8819daee94ccd25`. The
PTY run displayed the exact seeded deal and preview digests, `MOCK COMPLETED`,
the mock receipt digest, the synthetic no-execution/payment claim boundary,
and terminal-page feedback. After `n`, the no-additional-page message was
visible. Raw mode and alternate screen were entered; Ctrl+C exited 0 and
restored termios, alternate screen, and cursor. Both existing geometry checks
passed: 48x10 after 11 navigation events (maximum cursor row 9), and 100x30
after one navigation event (maximum cursor row 29). The 17,597-byte PTY
transcript SHA-256 is
`11c6f6c0fa24613dcebcc3324670a15f8f2e5696574453afdbc13784073e619d`.
The temporary home and service were removed.

The existing `corepack pnpm run verify:tui-ctrl-c` mode also exited 0 with the
same binary. It restored terminal state and passed both geometry checks; its
11,873-byte transcript SHA-256 is
`b8ea531ce194b6ef73ebce5d7c698e9461f2ecf87b820680ff0e0e525013fcbe`. Its
temporary home and service were removed.

`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed: 34 interface
tests, 3 installer tests, pinned Rust formatting, and strict workspace Clippy.
`python3 scripts/verify_imports.py` passed the Simple Compute Market (2,189
files), HyperIntel (99 files), and JustAI (176 files) recorded inventories.
`git diff --check` passed, and the five authored paths had zero trailing
whitespace lines. The final NUL-delimited worktree status SHA-256 is
`0b5bec7e4d285a13f0b24971522373158baaf9473c2042392919e95e8d461263` across
173 records: the 172-record pre-slice status plus this packet as one new path;
all other existing dirty/untracked work was preserved. No changes were
staged.

Assurance is `INTERNAL_PASS` for this populated local TUI first-page service
projection and the named local PTY checks only. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The complete interactive
multi-page traversal remains open because the fixed synthetic product has
only two unique valid deals and the TUI page limit is 20.

## Claim ceiling and non-goals

Assurance target: `INTERNAL_PASS` for this one local synthetic macOS POSIX PTY
path only. Reviewer status: `NOT_REQUESTED`. Reproducibility remains
`NOT_REPRODUCED` unless a clean second environment runs the exact packet.
Custody is the HyperAI source and one disposable synthetic home.

This does not prove multi-page interactive TUI traversal, general history
availability, browser/CLI/TUI/ACP parity, broad keyboard or assistive
accessibility, native Linux/Windows/Intel macOS support, correct computation,
real fulfillment, payment, settlement, source rights, independent review, or
production readiness.

Publish boundary: local test harness and documentation only; no commit, PR,
external circulation, release, deployment, or production operation.
