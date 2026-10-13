# HyperAI Populated Data Browser Workflow Packet

State slice: `hyperai.bazaar.interface.data_workflow_browser.v1`.
Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md),
[interface plan](bazaar-hyperai-interface-plan.md), and
[Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Purpose

Make the existing manually qualified Data workspace journey repeatable in the
headless-browser runner. The test exercises only HyperAI-authored synthetic
data in a fresh temporary home. It does not change product behavior, reuse
Bazaar code or data, or close overall Phase 2 acceptance.

## Exact files

- `scripts/verify-interface-browser.mjs`: add the opt-in `--data-workflow`
  path, fixed local Data POST allowlist, assertions, and optional screenshot.
- `package.json`: add `verify:interface:data-browser`; do not add dependencies
  or include Chrome-dependent work in root lint.
- `docs/compute-bazaar-interface-data-browser-workflow-packet.md`: this scope
  and its evidence.
- `docs/compute-bazaar-phase-2-data-packet.md`: append the result and limits.
- `docs/compute-bazaar-rust-port-subplan.md`: update only Phase 2 progress.
- `docs/state-slices.md`: authorization and result record.

## Frozen browser workflow

1. Open the Data workspace on the runner-owned local service with an empty
   temporary HyperAI home.
2. Create the fixed HyperAI synthetic JSON and Parquet snapshot using its
   preview digest and idempotency key.
3. Run the workspace's fixed bounded query and verify its three deterministic
   rows, query status and focus, dataset/source label, and snapshot digest.
4. Submit a fixed query against an unknown table. Verify the SQL remains in the
   editor, the previous result rows disappear, an accessible error status is
   rendered and focused, and no result is attributed to the failed query.
5. Resubmit the same valid query and verify successful recovery.
6. Preview and explicitly confirm one named local query model. Verify the
   preview digest, saved definition digest, success status, and inventory row.
7. Preview and explicitly confirm one table view bound to the saved model.
   Verify its preview digest, saved definition digest, success status, and
   inventory row.

Model and view names are fixed but used only under the disposable home. The
runner checks that the workflow emits exactly these same-origin POST requests:

| Route | Count | Purpose |
|---|---:|---|
| `/api/v1/data/synthetic/seed` | 1 | Create the fixed local sample |
| `/api/v1/data/query` | 3 | Valid query, rejected query, valid recovery |
| `/api/v1/data/models/preview` | 1 | Bind model definition to source snapshot |
| `/api/v1/data/models/save` | 1 | Explicit local model save |
| `/api/v1/data/views/preview` | 1 | Bind view to model definition digest |
| `/api/v1/data/views/save` | 1 | Explicit local view save |

All other browser requests must be same-origin loopback `GET` or `HEAD`, apart
from the self-contained inline SVG icon. The runner must fail on any other
method/path, non-loopback origin, console/runtime error, or favicon fallback.

## Allowed effects and controls

- Build `hyperai-ui` with the pinned toolchain and locked offline dependencies.
- Start one service process on an ephemeral IPv4 loopback port with a new empty
  synthetic home. Do not connect to or inspect the existing service/home on
  `127.0.0.1:4317`.
- Start headless Chrome with a fresh temporary profile and loopback DevTools.
  Disable background networking, synchronization, component updates, and
  nonlocal name resolution.
- Use only the fixed synthetic fixture, SQL and definition names specified by
  the runner. The service writes only below its temporary home.
- Capture a populated Data screenshot only when the explicit retention option
  is used. Visually inspect it, then remove its temporary directory.
- Stop only runner-owned processes and remove its temporary home, profile,
  logs, and unretained screenshots.

No upstream source, non-synthetic data, provider, seller, remote worker,
credential, paid execution, model run, Fleet action, settlement, publication,
or deployment is allowed.

## Proof gates

- `corepack pnpm run verify:interface:data-browser -- --retain-screenshots`
  passes and reports the observed request methods and counts.
- Query success renders three rows and snapshot provenance; query failure
  retains the entered SQL, clears the old rows, and focuses an accessible
  failure status; retry restores the result.
- Model and view each use preview plus separate explicit confirmation and
  show valid 64-character definition digests in their local inventories.
- Only the six frozen POST counts above are observed, with no other
  non-read request, browser error, console error, or missing-resource fallback.
- Visually inspect the populated screenshot and remove the returned temporary
  screenshot directory.
- Confirm service, Chrome, DevTools, temporary home/profile, and screenshots
  are cleaned up and the pre-existing listener on `127.0.0.1:4317` is unchanged.
- Run `CARGO_NET_OFFLINE=true corepack pnpm run lint`,
  `python3 scripts/verify_imports.py`, `git diff --check`, and scoped
  documentation/link hygiene. Preserve and report the known JustAI import
  inventory discrepancy if still present.

## Claim ceiling and open gates

Assurance can be `INTERNAL_PASS` only for this same-host headless-Chrome
workflow over the fixed synthetic snapshot and disposable local home. Reviewer
status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. This does not
establish screen-reader or physical-keyboard acceptance, cross-browser or
native-platform behavior, DataFusion allocation isolation, exhaustive SQL
safety, overall Phase 2 acceptance, clean second-host reproduction,
independent review, upstream source rights, release readiness, or production
authorization.

Phase 2 remains `NOT_ASSIGNED`. Bazaar rights and non-synthetic source
authorization remain separate. No commit, pull request, external circulation,
or deployment is authorized.

## Execution result

Completed on 2026-09-30 on macOS 26.6.1 arm64 with Rust 1.93.1, Node.js
22.23.2, pnpm 11.7.0, and Google Chrome 154.0.8037.58. The browser run used
the Data model/view pattern correction recorded separately under
[`hyperai.bazaar.interface.data_form_pattern.v1`](state-slices.md#hyperai-bazaar-interface-data_form_patternv1).

The first invocation built the UI target, then stopped at argument parsing
because pnpm forwarded an additional `--` separator before the screenshot
option. It stopped before starting the service or browser. The runner parser
was adjusted to discard forwarded separators. The next browser invocation
reached the expected query HTTP 422, which Chrome reported as a network log
error, and also exposed that both Data name patterns were invalid under the
current HTML pattern rules. The work stayed local; the pattern issue was
handled under the separate UI slice above, and the runner now exempts only the
exact observed query 422 after asserting its status and count.

`CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:data-browser --
--retain-screenshots` passed. The existing 33 route/viewport checks passed at
1440x1000, 720x500, and 390x844; the 33 reduced-motion checks covered 19,404
computed-style entries. The browser observed 39 requests, only same-origin
loopback `GET`/`POST` plus the inline icon. It verified exactly one seed, three
query, one model-preview, one model-save, one view-preview, and one view-save
POST. There were no unexpected requests, console/runtime errors, invalid
pattern errors, or favicon fallback.

The fixed synthetic dataset was
`hyperai.synthetic_market_observations_json.v1`, snapshot digest
`3e4d6abe5f967c8f60ebdf124851df7794dc1f812e0acde79c2a9c9913756335`. The
successful query returned three rows. The unknown-table query rendered HTTP
422, retained the entered SQL, cleared the previous result, and focused the
accessible status; repeating the valid query restored three rows from the same
snapshot. Both form patterns accepted their valid hyphenated names and rejected
their fixed invalid values. The model preview and definition digests were
`569b6a21d7944b455d34c5ab85bb18808baefce927e0399fcd55025ba343cb58` and
`554d383c8b5203c49ecf0c7ca09a555e2f8d5fc1d6af5622b263e49303cf3607`. The view
preview and definition digests were
`9ba05f425cc619a51ae0cad7c44265aad89c85715605b6f4a2695377eee8c12a` and
`4600b530ea971dc97595e5ad190a7c84d7b2b3f5db7c3f46e829ed3b7a32fc8b`.

The desktop populated Data, desktop Overview, 720x500 Data, and mobile Deals
screenshots were visually inspected and removed. The runner stopped its
service and Chrome processes and removed its temporary home/profile; the
retained screenshot directory was removed after inspection. The listener on
`127.0.0.1:4317` remained owned by PID 85094 and was not contacted. No upstream
code/data, provider, seller, or user data was used.

Runner source SHA-256:
`8096549f7871044df48add71ad7b50b6260f14b836f869b3f34da9d09d8b751f`.
Interface source SHA-256:
`f2bd8e181a4034c51cecd301928193d8a9788c8ceed421ca6761e337cd5925bd`.
The optional package script adds no dependency.

Final validation: `CARGO_NET_OFFLINE=true corepack pnpm run lint` passed,
including the root fast, contract, and full-workspace gates. The full gate
reported only its declared environment-gated skips for Docker, PostgreSQL, and
staged sidecar integration. `git diff --check`, scoped whitespace and final
newline checks, local Markdown link-target checks, and
`node --check scripts/verify-interface-browser.mjs` passed. Import preservation
passed for Simple Compute Market (2,189 files) and HyperIntel (99 files); the
known JustAI inventory mismatch remains at 177 observed files versus 176
recorded files, so `python3 scripts/verify_imports.py` exits 1. No imported
source tree was changed. The existing listener on `127.0.0.1:4317` remained
owned by PID 85094, and no browser screenshot directory remained after cleanup.

Assurance is `INTERNAL_PASS` for this same-host synthetic browser workflow
only. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Overall Phase 2 remains `NOT_ASSIGNED`.
