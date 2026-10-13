# HyperAI Browser Process-State Cleanup Diagnostic Packet

State slice: `hyperai.bazaar.interface.browser_process_state_diagnostic.v1`.
Status: completed bounded parser implementation and synthetic qualification.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and boundary

ACP cancellation browser packet v4 is closed `INCONCLUSIVE`. Its final
process-group check reported a live scope, but the packet retained no process
states, so it is not known whether a defunct process-table entry contributed.
This continuation makes the runner distinguish executing members from
defunct (`Z`) entries and reports the final runner-owned group PID/state pairs.
It does not diagnose v4 after the fact or repeat any ACP browser identity.

Only these code paths are changed:

- `scripts/browser-process-group.mjs` parses the fixed PID, PGID, and state
  columns from supplied `ps` text and marks a group active when any member is
  not defunct.
- `scripts/verify-interface-browser.mjs` uses that policy for its existing
  ACP cleanup checks and includes final PID/state pairs in the cleanup record.
- `scripts/tests/browser-process-group.test.mjs` tests fixed synthetic process
  table strings.
- `package.json` includes the parser tests in `verify:interface`.

The cleanup wait and signal intervals, browser workflow, service, ACP/MCP
contracts, process-group ownership, network rules, and temporary-state rules
are unchanged. A live or stopped process remains cleanup-active. A group that
contains only defunct entries is execution-empty; those entries have exited
and cannot execute or hold open profile files.

## Frozen baseline and allowed effects

Baseline is HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved. The runner before this
slice was 149,833 bytes at SHA-256
`70e43ba4ff7eecb7bb7aea97a27d42a3ac3b178213ceb152c78495e377d8c1bd`;
`package.json` was 6,653 bytes at SHA-256
`c83c2a132cecaa471e5fcd0d98bfa28f62855cd6b61af45d877d16051e4ba2c8`. The
read-only v4 packet was 8,848 bytes at SHA-256
`575abb7ff48ae7354c617f854270a14cc2c75f5e20a7ef0fd94865139aec7b06` and
must remain byte-identical.

Allowed effects are fixed-string parser tests, JavaScript syntax checks,
`corepack pnpm run lint:fast`, `corepack pnpm run verify:contracts`,
`python3 scripts/verify_imports.py`, Markdown-link validation, and
`git diff --check`. The tests do not invoke `ps`, spawn processes, open
sockets, launch the service or Chrome, inspect live process tables, signal
processes, or remove temporary state. Port `127.0.0.1:4317` is outside scope.
Do not rerun ACP cancellation browser identities v1-v4. Do not repeat the
prior full root test suite until the `HomeBusy` failure has a separate
supported diagnosis.

## Proof gates and claim ceiling

1. Darwin/Linux-style state strings `S+`, `Z+`, `T+`, and `?` are parsed into
   exact numeric PID/PGID and state fields.
2. Only a group with no non-defunct members is treated as execution-empty.
   Live, stopped, and unknown states remain active.
3. Malformed rows, unsafe numeric fields, and rows from another PGID cannot
   create matching members.
4. `verify:interface`, root fast lint, root contract tests, import
   preservation, Markdown-link validation, and `git diff --check` pass.
5. The v4 packet remains unchanged; no browser, service, socket, or process
   effect occurs.

Claim ceiling: `INTERNAL_PASS` for interpreting these fixed synthetic process
table rows if the listed local checks pass. This does not establish that v4
actually observed a zombie, that any Chrome process was cleaned up, or that
the ACP cancellation workflow passes. Reviewer status is `NOT_REQUESTED`;
reproduction is `NOT_REPRODUCED`. Browser cleanup remains `INCONCLUSIVE`
pending a fresh separately frozen browser workflow packet. Publish boundary:
local source and documentation only.

## Result (2026-10-01)

The parser and runner source matched these post-change digests:

| File | Bytes | SHA-256 |
|---|---:|---|
| `scripts/verify-interface-browser.mjs` | 150,103 | `837f73d3e14297611289f74e5643480d09417380e32bfe763a99374ba168cfe0` |
| `scripts/browser-process-group.mjs` | 1,744 | `540e8d516fb84dd4dba569987800c21083399466c7d25b3ce4daf6a024bed6ef` |
| `scripts/tests/browser-process-group.test.mjs` | 1,408 | `f930128679413b8f1881bc8fd206c745209f5a1b0e2702a3cbdee688349288b9` |
| `package.json` | 6,748 | `7a9ef6b1f4487190fe831e470d07ea1d9163d9f28a6a655d510657ba55f625c2` |

Node.js was `v22.23.2`; pnpm was `11.7.0`. The four fixed parser tests
passed. `CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` passed its 21
interface tests, 3 installer tests, pinned formatting, and strict all-target,
all-feature Clippy. `CARGO_NET_OFFLINE=true corepack pnpm run verify:contracts`
passed; 38 `hyperai-data` unit tests and the remaining selected contract tests
passed. `python3 scripts/verify_imports.py` passed all three recorded trees:
Simple Compute Market (2,189 files), HyperIntel (99), and JustAI (176).
`node --check scripts/verify-interface-browser.mjs`, linked-plan path checks,
and `git diff --check` passed. The v4 packet remains byte-identical at
SHA-256 `575abb7ff48ae7354c617f854270a14cc2c75f5e20a7ef0fd94865139aec7b06`.

No live process table was inspected, no process was spawned or signaled, and
no browser, service, socket, or temporary home was used. The v4 browser result
remains `INCONCLUSIVE`; this work does not establish that v4 observed a
zombie. The previous root `verify:full` failure `HomeBusy` was not rerun and
its cause is still unknown. Assurance is `INTERNAL_PASS` for the fixed
synthetic parser contract only. Reviewer status is `NOT_REQUESTED`,
reproduction is `NOT_REPRODUCED`, and browser cleanup qualification remains
open for a fresh separately frozen workflow.
