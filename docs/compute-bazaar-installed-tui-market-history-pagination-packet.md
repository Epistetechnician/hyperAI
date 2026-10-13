# Installed TUI Market History Pagination Packet

Status: authorized local synthetic qualification slice under the user's
direction to follow the Compute Bazaar Rust port subplan. The existing
one-page PTY packet remains closed as written; this packet adds the missing
interactive second-page path without changing product code or contracts.

State slice:
`hyperai.bazaar.desk_runtime.tui_market_history_interactive_pagination.v1`.
Parents: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md),
[one-page TUI service packet](compute-bazaar-tui-market-history-service-packet.md),
and [CLI/TUI pagination packet](compute-bazaar-market-history-client-pagination-packet.md).

## Frozen baseline

- HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`; preserve all
  pre-existing dirty and untracked work.
- Pre-slice `git status --porcelain=v1 -z` SHA-256:
  `aaff109ee8a70f449eac31a7d084dc80719e3528535e0eff88b5b755c14c5dab`
  (178 records); the staged path list is empty.
- `scripts/verify_hyperai_tui_ctrl_c.py`:
  `971cf613f73ab6251b5398bcd37de45db864ac2f5da8f4b03ec0e3637ac316f9`.
- `package.json`:
  `aacf3d8785b1415319615a917f8c5bece4c1d3ddae0301af302e2c1a8a75b89b`.
- `docs/compute-bazaar-rust-port-subplan.md`:
  `0ad29e45044faf7aee7724fa48274cbd24877b8394eb4dd68273e02fe247f6e2`.
- `docs/bazaar-hyperai-interface-plan.md`:
  `aecdc05d5a7784e3eeb1d7ca48e5747757081ede2022f2364f134097a19d6cbc`.
- `docs/state-slices.md`:
  `b7d1d7ce98c468eb242464550c26ca3d3a27eccba5c70f1669f0765d73fd3bef`.

## Objective and exact writable files

Prove that the installed `hyperai desk` TUI requests, verifies, appends, and
renders a second authenticated Market history page after a user presses `n`.
Use exactly the existing `baseline` and `alternate` synthetic quote scenarios
to create two valid approved deals and mock receipts in a fresh disposable
home.

Writable files:

- `scripts/verify_hyperai_tui_ctrl_c.py`: add an opt-in pagination mode, seed
  both existing synthetic scenarios, and run a test-owned loopback proxy that
  rewrites only Market history `limit=20` requests to the supported
  `limit=1`. Forward the opaque cursor and all other local requests unchanged.
  The proxy must not log or return bearer credentials, raw request bodies, or
  raw PTY output.
- `package.json`: add `verify:tui-market-history-pagination` using the pinned
  offline Rust build and the new harness mode.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-rust-port-subplan.md`, and
  `docs/bazaar-hyperai-interface-plan.md`: record scope, evidence, and limits.

The TUI, CLI, local service, Market domain, API schema, storage, page-size
behavior, imported products, and ignored Bazaar checkout are read-only. If
the run needs another source file or any product/API behavior change, stop and
amend the packet before proceeding.

## Allowed effects and proof gates

Allowed effects are two approvals and deterministic mock receipts for the
existing local synthetic scenarios in a disposable home; authenticated IPv4
loopback requests to that home's service and test-owned proxy on dynamic
ports; PTY input; ignored `target/` build output; and the validation commands
below. The proxy forwards only to the service address read from the same
test-owned home. It must preserve authentication and page digests. It changes
only the requested page limit from 20 to 1 so the existing two records form
two valid service-generated pages; it does not create, alter, reorder, or
duplicate any record or cursor. Do not contact port `4317` or any external
endpoint.

Proof gates:

1. The harness creates and verifies exactly one `baseline` and one `alternate`
   approved deal, each with its existing mock receipt. The owner service
   reports two unique records newest-first.
2. The test-owned proxy binds only to `127.0.0.1` on port `0`, forwards the
   owner's bearer without logging it, and changes only `limit=20` to
   `limit=1` on `GET /api/v1/market/deal-history`. The first service response
   carries one row and its service-issued opaque cursor; the second request
   forwards that exact cursor and receives the other row with a verified
   terminal page.
3. The installed TUI PTY opens Deals, renders only the first page and its
   continuation state, advances on `n`, renders both exact records and page
   digests, and shows the terminal state. A further `n` shows the no-more-page
   message without a third history request.
4. The installed TUI exits cleanly and restores raw mode, alternate screen,
   cursor, and terminal settings. The proxy, service, child processes, and
   temporary home are removed. Restore the original service locator before
   requesting service shutdown.
5. Run `corepack pnpm run verify:tui-market-history-pagination`, the existing
   `corepack pnpm run verify:tui-market-history`,
   `corepack pnpm run verify:tui-ctrl-c`, `corepack pnpm run lint:fast`,
   `python3 scripts/verify_imports.py`, and `git diff --check`. Inspect all
   authored untracked files for trailing whitespace, confirm the staged list
   is empty, and record the final worktree status digest.

## Execution record

The first pagination command stopped during setup before the PTY opened:
HTTP 400 from the synthetic preview route. The route-security middleware
requires `Content-Type: application/json` for API `POST` requests. The harness
was corrected to send `{}` with that content type, matching the existing
service client contract. The subsequent run passed all pagination assertions.

Successful run on 2026-10-02, macOS arm64, Python 3.14.5; binary SHA-256
`7455559c025359e867ba8c1fa34a574ce414bae397755d143d8af00d46019a90`:

- `corepack pnpm run verify:tui-market-history-pagination`: passed. The TUI
  rendered the alternate record on page one, both records and both service
  page digests after `n`, terminal and no-more-page states, and exactly two
  authenticated history requests. The proxy forwarded the exact service
  cursor; both pages contained one service-generated row and had no proxy
  error. The terminal, alternate screen, cursor, service locator, proxy,
  service, and temporary home were restored or removed.
- `corepack pnpm run verify:tui-market-history`: passed; populated first-page
  service flow and terminal cleanup retained.
- `corepack pnpm run verify:tui-ctrl-c`: passed; terminal cleanup and resize
  checks retained.
- `corepack pnpm run lint:fast`: passed 34 interface tests, 3 installer tests,
  Rust formatting, and strict workspace Clippy.
- `python3 scripts/verify_imports.py`: passed for Simple Compute Market
  (`a1c46d20b1b2f1aabe53e1b052921a87cf9d7360`), HyperIntel
  (`d52111a07baa5fc4a7f6bfb4407731e2b9f609dc`), and JustAI
  (`9a0e56536b968cbb2cb7173e142068b53c4f9fe3`) at their recorded source
  digests.
- `git diff --check` and the corrected scoped trailing-whitespace scan passed.
  The staged path list was empty. Final `git status --porcelain=v1 -z` has
  179 records and SHA-256
  `5dfab8730943d0b665d67ac98159766a0a546ed7b96361ae154ef078b88d26eb`.

Assurance: `INTERNAL_PASS` for this two-page installed-TUI local synthetic
POSIX path only. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. A Python 3.14 `DeprecationWarning` notes that the PTY harness
forks while its test proxy thread is active; the run completed and restored
the terminal. Other OSes, broad accessibility, arbitrary history volume,
live fulfillment, payment, and production remain outside the claim ceiling.
Next blocked authority gate: separate platform and accessibility qualification
is required before making broader TUI compatibility claims. Independent
review, live providers, paid execution, testnet writes, and production are not
requested or authorized by this local packet.

## Non-goals and claim ceiling

Do not change the production TUI page size, route, API contract, storage, or
Market scenario set. Do not create extra or forged deals, repeat approvals,
contact sellers/providers, run Fleet, call a model, perform paid Eval or
financial Trade, access a wallet or chain, publish, deploy, or touch user-home
state.

The maximum claim is interactive installed-TUI traversal across two
service-generated local synthetic history pages on the tested POSIX host,
with the loopback proxy constraining each valid service request to one record.
This is not evidence for other platforms, arbitrary history volumes, broad
accessibility, live fulfillment, correct computation, payment, settlement,
source rights, independent review, or production readiness. Assurance target:
`INTERNAL_PASS`. Reviewer status: `NOT_REQUESTED`. Reproducibility:
`NOT_REPRODUCED`. Data custody is limited to HyperAI-authored synthetic
records and disposable local test state. Publish boundary: local test harness
and documentation only; no staging, commit, PR, external circulation,
release, deployment, or production operation.
