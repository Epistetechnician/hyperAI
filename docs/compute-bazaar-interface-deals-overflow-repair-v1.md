# Deals Viewport Overflow Repair

Status: closed `INCONCLUSIVE` after the authorized post-change browser run
failed in browser evaluation before reporting route results. The pre-change
diagnostic identified the cause and the one-rule CSS repair was applied, but
the browser gate remains unverified. This packet authorizes no further run.

State slice: `hyperai.bazaar.interface.deals_overflow_repair.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Feature: [supply-to-quote](compute-bazaar-supply-to-quote-packet.md).
Prior diagnostic: [Deals overflow diagnostic](compute-bazaar-interface-deals-overflow-diagnostic-v1.md).

## Baseline and frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked work preserved. The NUL-delimited
`git status --porcelain=v1` digest is
`5f2fc67452348933febd1aa24d061b88a4974d9de11ec6ed47697e64db263ae2`.

Recheck the input hashes before editing; any mismatch requires a fresh packet.

| File | SHA-256 |
|---|---|
| `scripts/verify-interface-browser.mjs` | `e728755e093df9d411210f8e432d31340d94bfb9e6a9004ce45e4b9b23896460` |
| `apps/hyperai-interface/app.css` | `064c438a3a241ab369257e14b6f1458b47ff6adfeae738f1a1955f0e75f59ed7` |
| `apps/hyperai-interface/app.js` (read-only) | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `docs/state-slices.md` | `d7e5205743cf00df83512eb204247edb428afa996fcc2c46ea85f49984ca1fda` |
| `docs/bazaar-hyperai-interface-plan.md` | `7795abc3258e826acf20536da164cb561a0c62b3af04ef64db8134269acedd14` |
| `docs/compute-bazaar-interface-deals-overflow-diagnostic-v1.md` | `8e33d70dd6132f178325dc77b6cde449156540bfd097d4e8271ebe1d0865f2b7` |

## Contract and exact writable files

1. Extend `scripts/verify-interface-browser.mjs` to report bounded geometry
   and scroll dimensions for the root plus a small set of overflowing
   elements. Include tag, ID, classes, dimensions, overflow mode, and
   before/after pseudo-element geometry; exclude rendered text, field values,
   tokens, environment values, and local-home paths.
2. Run the fixed quote-preview browser flow once before a CSS change. If the
   output identifies one clear existing layout rule in `app.css`, change only
   that rule using an appropriate responsive constraint such as min-width,
   wrapping, or grid sizing. If the root is ambiguous or implicates multiple
   rules, stop without a CSS change.
3. Run the same browser flow once after the single CSS rule change. This is
   the only post-change browser attempt. Do not edit route behavior or feature
   logic to make the workflow pass.

Writable files:

- `scripts/verify-interface-browser.mjs` (diagnostic output only).
- `apps/hyperai-interface/app.css` (at most one responsive layout rule).
- This packet, `docs/state-slices.md`, and
  `docs/bazaar-hyperai-interface-plan.md` (scope and evidence only).

`apps/hyperai-interface/app.js`, HTML, Rust, manifests, source snapshots,
imported product trees, and the ignored Bazaar checkout are read-only.

## Allowed effects and stop conditions

- Run `corepack pnpm run verify:interface:supply-quote-browser` exactly once
  before the CSS edit and exactly once after it, only if the first report
  isolates one clear rule. Both attempts use the pinned local synthetic
  service and an isolated browser/profile on IPv4 loopback.
- Run `node --check scripts/verify-interface-browser.mjs`,
  `corepack pnpm run lint:fast`, `python3 scripts/verify_imports.py`, and
  `git diff --check`. Do not run root `verify:full` under this packet.
- Do not contact the user's listener on port 4317 or any external origin.
  Use no provider, seller, registry, model, fleet, chain, or remote service.
  Clean only runner-owned processes and temporary files.
- Stop on source drift, unexpected requests, ambiguous process ownership or
  cleanup, a diagnostic implicating multiple layout rules, or a failed or
  inconclusive post-change browser attempt. No third browser run is allowed.

## Non-goals and claim ceiling

No feature/API/route/record changes, new dependencies, storage changes,
external integrations, Bazaar source or data reuse, broader style redesign,
accessibility claim, platform claim, release, publication, or deployment.

Passing the second browser attempt can establish only that the fixed local
synthetic quote-preview path passed the existing browser route/viewport
checks, including the prior 720-by-500 Deals case, on this host. It does not
establish general usability, assistive-technology acceptance, other-browser
or native-platform support, seller intent, live availability, execution,
payment, Bazaar rights, or production readiness. Initial assurance is
`NOT_RUN`; reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Publish boundary: local source and documentation only.

## Execution record (2026-10-01)

The pre-change browser run isolated the cause: the full digest description
inside `#workload-trace-digest-value` uses a `<span hidden>`, while
`.trace-summary span { display: block; }` overrode the browser's hidden
behavior. At 720 CSS pixels, that span expanded the summary grid and document
scroll width to 852 pixels. No page text was captured in the diagnostic
report.

The one allowed CSS change narrowed that selector to
`.trace-summary span:not([hidden])`, preserving visible summary labels while
leaving the digest description hidden. `corepack pnpm run lint:fast` passed
(33 interface tests, 3 installer tests, pinned formatting, and strict Clippy).
`python3 scripts/verify_imports.py`, `node --check`, and `git diff --check`
also passed; the Simple Compute Market, HyperIntel, and JustAI recorded
snapshot digests matched.

The single post-change `corepack pnpm run
verify:interface:supply-quote-browser` attempt exited 1 with
`browser evaluation failed: SyntaxError: Invalid or unexpected token` before
returning a route or layout result. Therefore the fix and quote workflow are
not browser-verified. Read-only cleanup checks found no runner-owned
browser/service process and no `/tmp/hyperai-interface-browser-*` directory.
No external origin or port 4317 was contacted. This is `INCONCLUSIVE`; reviewer
status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. The packet's
post-change browser allowance is consumed. Any further browser work requires
a fresh packet that diagnoses the page-evaluation syntax failure and then
authorizes a new exact run.
