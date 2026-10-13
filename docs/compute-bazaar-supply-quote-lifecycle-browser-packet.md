# HyperAI Synthetic Supply-to-Receipt Browser Packet

Status: scoped local synthetic implementation and qualification slice.

State slice: `hyperai.bazaar.interface.supply_quote_lifecycle_browser.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Rust port subplan](compute-bazaar-rust-port-subplan.md).
Predecessor: [supply-to-quote preview packet](compute-bazaar-supply-to-quote-packet.md)
and its [final preview browser qualification](compute-bazaar-interface-quote-boundary-assertion-v1.md).

## Goal

Qualify the already implemented browser buyer path from the selected
HyperAI-authored synthetic offer, through its bound seller-quote preview, an
explicit synthetic buyer approval, mock fulfillment, and the resulting
receipt. The preview-only predecessor deliberately stopped before approval
and completion. This slice exercises those existing separate controls; it
does not change product or market behavior.

## Baseline and frozen implementation inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the complete existing dirty and untracked worktree. Preserve every existing
path. The source manifest before this slice contained 293 regular files /
4,522,620 bytes, SHA-256
`0ef8866db0e45bbf1741233d6400fc0bd3cc46e8f2c544d866bb915f0b73e53a`.
The pinned HyperIntel input remains its recorded 99-file snapshot. No
upstream Bazaar input is in the source manifest.

Pre-edit files relevant to this workflow:

| File | SHA-256 |
|---|---|
| `package.json` | `385100f1df4c314f5c391fffb6665da0c8fa346578bc65c4e4d7f12b95904e81` |
| `scripts/verify-interface-browser.mjs` | `cd48f833557cd8c563bb4f96921c36b894a4959e2e7ed75304f91268aaba228d` |
| `apps/hyperai-interface/app.js` | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `apps/hyperai-interface/app.css` | `e9f56117a9d8350c360331f5983f21c92a04b060b4f77acea2aaafc115ba0f83` |
| `crates/hyperai-local-service/src/server.rs` | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `crates/hyperai-application/src/lib.rs` | `dcb505ad95e25352434a2959d6143f00030766ff9c095c1c305c27e25e3b8c05` |
| `crates/hyperai-market/src/lib.rs` | `16b07ec36107d29182020abecd23b8cd23862e3c3203914790877409a65a44f8` |

Runtime versions: Node.js `v22.23.2`, pnpm `11.7.0`, pinned Rust/Cargo
`1.93.1`, and Google Chrome `154.0.8037.92`. Rust commands use
`scripts/with-pinned-rust.sh`; the host's unqualified Rust version is not used.

## Post-edit pre-run freeze — 2026-10-01

The implementation/test-harness source manifest is 293 regular files /
4,532,071 bytes, SHA-256
`15d17288df97a40a65a48f4376989e9c61b60355ca1d9f9f7af99f6d759ea2d1`.
Immediately before the browser workflow, full
`git status --porcelain=v1 --untracked-files=all` contains 2,670 lines /
229,821 bytes, SHA-256
`767954cdc25bf296475ef9c836305b029849dd4861545283b906027a7ad94ed5`.

The exact post-edit command inputs are:

| File | SHA-256 |
|---|---|
| `package.json` | `9b5096417011f6e15a53b5e2a028fa13ff10ad5a00041aa29a3cf1a2f0d7658a` |
| `scripts/verify-interface-browser.mjs` | `67d69e4b8f66c9f0431701cdbe41d9bf45c6a97c6b53aa228da95666b879121c` |
| `apps/hyperai-interface/app.js` | `78274da541a82e10fa0ba245c13d1fa9bca5eab260a7433e5ac31833540b4d03` |
| `crates/hyperai-local-service/src/server.rs` | `dbb238c7b09cc3cf6c76188d51316e169bbe27f101075d16273bf355ecfe0a47` |
| `crates/hyperai-application/src/lib.rs` | `dcb505ad95e25352434a2959d6143f00030766ff9c095c1c305c27e25e3b8c05` |
| `crates/hyperai-market/src/lib.rs` | `16b07ec36107d29182020abecd23b8cd23862e3c3203914790877409a65a44f8` |

Preflight passed: browser-runner syntax and package JSON parsing;
`CARGO_NET_OFFLINE=true corepack pnpm run lint:fast` (33 interface tests,
3 installer tests, pinned Rust formatting, and strict all-target/all-feature
Clippy); `CARGO_NET_OFFLINE=true corepack pnpm run verify:contracts`;
`python3 scripts/verify_imports.py`; and `git diff --check`. The pinned Rust
toolchain reports rustc 1.93.1 and Cargo 1.93.1; Chrome reports
`154.0.8037.92`.

Run exactly once after rechecking these inputs:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:supply-quote-lifecycle-browser
```

This package command builds the existing `hyperai-ui` binary offline and
executes only `--market-supply-quote-lifecycle-workflow` in the isolated
browser harness. It does not run the separate ACP cancellation identity.

## Execution result — 2026-10-01: `INTERNAL_PASS`

The exact command exited 0 on Google Chrome `154.0.8037.92`. The served
synthetic workflow keyboard-selected `offer-gpu-b`, previewed its quote,
explicitly approved that preview, then explicitly continued to mock
fulfillment. Computed accessible button names and visible focus passed at all
three action points; the interface restored focus to the approval and
completion headings after each state transition.

Digest binding passed end to end:

| Record | SHA-256 |
|---|---|
| Supply comparison | `5036567a60199af6f6a9ea0fde9871854ce951b1d65ec0f1783ab483575ea5b6` |
| Supply-to-quote binding | `04c80223ae1b22df7232b889f1dec9508f12ac535de8e9a9c02541cf57c11848` |
| Quote preview | `7861ec3e388f66dc0adcacb7231d9476457c22d221e330a326cbcad871af384c` |
| Approved deal | `1b1bab4e42c3c4b149545fc48149b184866163c704b1cf0ff74f58edb6c06855` |
| Mock receipt | `bf29e0770f5e41616f4d58593286552d613433070c0b28f01870b050211403d7` |

The service observed exactly one comparison `GET` and one successful `POST` to
each of the three allowlisted routes. All four responses were HTTP 200, the
approval body bound the preview digest, and the completion body bound both
the same preview and approved deal. `otherWrites` was 0. The rendered receipt
identified the seller report, verifier, and disposition as synthetic mocks;
the UI stated that no funds moved.

All 33 workspace route/viewport checks passed at 1440x1000, 720x500, and
390x844 with no document overflow. All 33 reduced-motion checks passed across
23,760 inspected motion entries. The 390px offer table retained contained
horizontal scrolling. This is not OS-level screen-reader, physical-keyboard,
cross-browser, or native-platform evidence.

Cleanup was conclusive: Chrome exited 0 with no live process-group members;
the service exited after `SIGTERM`; both ephemeral listeners closed; and the
temporary home/profile directory was removed. The post-run source manifest
matched the 293-file / 4,532,071-byte digest
`15d17288df97a40a65a48f4376989e9c61b60355ca1d9f9f7af99f6d759ea2d1`. The
full Git-status output remained 2,670 lines / 229,821 bytes with SHA-256
`767954cdc25bf296475ef9c836305b029849dd4861545283b906027a7ad94ed5`; the
three import pins passed after the run and `git diff --check` passed.

Assurance is `INTERNAL_PASS` for this exact same-host synthetic browser path.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Data
and artifact custody was limited to HyperAI source, its fixed synthetic
fixture, and disposable local service/browser state. No Bazaar source or
data, seller/provider, external origin, live execution, funds movement, or
external service was used. Phase 2 remains `NOT_ASSIGNED` and all external
operation gates remain closed.

## Exact writable files

- `scripts/verify-interface-browser.mjs`
- `package.json`
- This packet
- `docs/state-slices.md`
- `docs/compute-bazaar-rust-port-subplan.md`
- `docs/bazaar-hyperai-interface-plan.md`

Product UI, API handlers, market contracts, storage formats, import trees,
source data, and runtime dependencies are read-only.

## Allowed effects and network boundary

Allowed: add a dedicated browser qualification mode and package command; run
JavaScript syntax and focused interface checks, root `lint:fast`, root
`verify:contracts`, import verification, pinned formatting, diff hygiene, and
exactly one dedicated browser workflow against a fresh temporary home and
Chrome profile. The workflow starts the already built local service and
Chrome on kernel-assigned IPv4 loopback ports and removes only its own
processes and temporary files.

The browser may issue same-origin `GET`/`HEAD` requests and exactly one `POST`
to each of:

- `/api/v1/market/supply-quote-preview`
- `/api/v1/market/quote-approve`
- `/api/v1/market/quote-complete`

The owner-only approval and completion routes use deterministic synthetic
keys and mock records. They create no seller/provider request, job, remote
execution, durable market/deal ledger, funds movement, or payment entitlement.
The disposable service home is removed after the run. All other non-read
requests and every non-loopback origin are denied by the runner.

Before the browser run, freeze the post-edit source manifest, relevant file
hashes, Git-status byte count and digest, browser version, and exact command.
Stop without another attempt if any frozen input drifts, a request falls
outside this allowlist, a digest is not bound across records, an unexpected
write occurs, or process cleanup is ambiguous. A failed or inconclusive run
closes this identity; a correction requires a fresh packet and state slice.

## Proof gates and claim ceiling

- Navigate by keyboard from Market and preview the selected `offer-gpu-b`.
- Confirm visible keyboard focus and computed button name at the quote-preview,
  approval, and mock-completion steps.
- Before approval, confirm no approval or completion record exists. After
  approval, verify the approval binds the exact quote-preview digest and
  exposes its deal digest. Verify completion is absent until this action.
- After mock completion, verify its preview and deal digests match the prior
  records, its receipt digest is present, and visible copy states that the
  seller report/verifier/disposition are synthetic and no funds move.
- Verify exactly one HTTP 200 response for each allowed POST; submitted
  approval and completion digest fields match the displayed records; no other
  write request or browser error occurs.
- Retain the existing route, desktop/720px/mobile, reduced-motion, and
  contained mobile-table checks. This workflow does not claim OS screen-reader,
  physical-keyboard, cross-browser, or native-platform accessibility.
- Run `node --check scripts/verify-interface-browser.mjs`,
  `corepack pnpm run lint:fast`, `corepack pnpm run verify:contracts`,
  `python3 scripts/verify_imports.py`, pinned Rust formatting, and
  `git diff --check`; then run the dedicated browser command once.

Maximum claim: `INTERNAL_PASS` for the exact local synthetic browser path from
selected offer to mock receipt, and only the listed browser/keyboard checks.
Reviewer status is `NOT_REQUESTED`; reproduction begins as
`NOT_REPRODUCED`. This does not establish seller intent, real availability,
execution correctness, payment entitlement, independent review, source rights,
native-platform support, production readiness, or overall Phase 2 acceptance.
Phase 2 remains `NOT_ASSIGNED`; live Fleet, paid Eval, financial Trade,
provider access, settlement, publication, and deployment stay closed.

Publish boundary: local source and documentation only. No commit, PR,
publication, release, deployment, or external review submission.
