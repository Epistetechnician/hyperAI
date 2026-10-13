# Market Supply Comparison Browser Qualification

State slice: hyperai.bazaar.buyer_supply_comparison_browser.v1.
Parent implementation: compute-bazaar-buyer-supply-comparison-packet.md.
Product sequence: bazaar-hyperai-interface-plan.md.
Engineering sequence: compute-bazaar-rust-port-subplan.md.

Status: bounded local browser qualification packet. It adds a dedicated
browser assertion for the already implemented buyer supply comparison and
uses the existing process-state helper to make cleanup evidence accurate.
It changes no product API, projection, or rendered interface.

Baseline: HyperAI dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa on main, with the
preserved dirty and untracked working tree. The NUL-delimited Git-status
digest at freeze was
5bdd8d5cb61a088b40440b133848cc4ce512d3d9c989ddec482dc75d6174771a.

## Frozen basis

| File | Bytes | SHA-256 |
|---|---:|---|
| scripts/verify-interface-browser.mjs | 150,103 | 837f73d3e14297611289f74e5643480d09417380e32bfe763a99374ba168cfe0 |
| package.json | 6,988 | 8a849ccdc8918adc7e7f30d802cc1cbaf1a253817e8bfd5cfbaa69d752a0098a |
| apps/hyperai-interface/app.js | 150,131 | 8078c9779dcf6ca363eddc59cce78309c790df97c66baa2369a403423eee4555 |
| apps/hyperai-interface/market-supply-comparison.js | 7,737 | 1d4b673cf5d301efced8cb54da889c6ed8c014b21726415efe51c15e531c44a5 |
| apps/hyperai-interface/app.css | 93,275 | 064c438a3a241ab369257e14b6f1458b47ff6adfeae738f1a1955f0e75f59ed7 |
| apps/hyperai-interface/index.html | 4,978 | 311a2504aa252507cf3bf0b84838467bda0dbd6bd3a564b1ffa14e6720f6cd73 |
| crates/hyperai-market/src/lib.rs | 119,650 | 35b3dc1d8bbe92877c4ce421d53eff0c31a8253cce0753ff07b2f0420735591e |
| crates/hyperai-local-service/src/server.rs | 83,331 | f45d9caac28e4ab5d5bfbb49d5277286811d37c367f0ec3c490c601aca67cfbe |
| docs/compute-bazaar-buyer-supply-comparison-packet.md | 12,268 | 0418a71bb17009bb79e26ffcb89d2c5bff73d0332be3488edc9e82f4e2f3d184 |
| docs/bazaar-hyperai-interface-plan.md | 151,832 | 201956041b52134e9f1a5a9e4ca73037f0b1629822b7d82b7c1bfb32402e655a |
| docs/compute-bazaar-rust-port-subplan.md | 217,780 | 9e0219b803aa0c729b21b1e50f95caf3374e61a59e0fea3a0b3ae506d5a9512f |
| docs/state-slices.md | 1,102,766 | 1afc6c5225edaf532e273f48658b2ad3c6715c82c67c44e157ecafa70d3bad16 |

The product renderer and service sources are read-only. The comparison uses
the existing fixed HyperAI synthetic catalog; this slice does not read SCM
captures, Bazaar source or data, non-synthetic inputs, or provider endpoints.

## Exact writable files

- scripts/verify-interface-browser.mjs
- package.json
- docs/compute-bazaar-interface-market-supply-browser-packet.md
- docs/compute-bazaar-buyer-supply-comparison-packet.md
- docs/bazaar-hyperai-interface-plan.md
- docs/compute-bazaar-rust-port-subplan.md
- docs/state-slices.md

If the browser proof requires a product source change, stop and create a
separate implementation slice. Do not silently widen this qualification
packet.

## Browser contract

The browser starts the existing local service with a disposable home and
ephemeral loopback ports, then loads the bundled fixed synthetic comparison.
The workflow navigates to Market by keyboard and verifies:

- the comparison is populated rather than loading, missing, or failed;
- the buyer constraints, signed catalog, deterministic selected offer,
  seller asking terms, separate advisory observations, freshness status,
  provenance digest, and synthetic-only claim are visible together;
- the fixed fixture shows two assessed offers and selects offer-gpu-b;
- at least one advisory observation is shown as not an offer and remains
  outside the selected-offer assessment;
- the Simple Compute Market capture explorer remains a separate section;
- Chromium exposes the comparison panel as a named region and the offer list
  as a named table with its caption;
- keyboard navigation reaches Market with a visible focus indicator and
  returns visible focus to the workspace heading after navigation;
- at 390 CSS pixels, the page has no horizontal overflow and the offer table
  scrolls inside its own table container.

The network audit must observe exactly one successful GET of
/api/v1/market/supply-comparison from this browser initialization, no
non-GET/HEAD requests, and no non-loopback request. It must not call quote,
approval, fulfillment, verifier, receipt, settlement, provider, or registry
operations.

The runner must report the browser and service process identities, the
final process-group member states, whether any member can still execute,
whether ephemeral listeners closed, and whether the temporary home was
removed. A zombie-only process group is execution-empty but must remain
visible in cleanup evidence; a live member makes the run inconclusive and
preserves temporary state for diagnosis.

## Allowed effects and proof gates

Allowed effects are a focused browser-runner and package-script change,
offline build of the existing local interface binary, one browser run against
the fixed synthetic comparison, and local documentation updates. The browser
runner reserves service and DevTools ports dynamically. It does not use port
4317 or an existing user service.

Run the dedicated command:

    corepack pnpm run verify:interface:market-supply-browser

This command must use the pinned Rust toolchain, build outcomes-cli's
hyperai-ui binary with Cargo locked and offline, then invoke the browser
runner's market-supply-comparison workflow. It must not retain screenshots.
Record the literal browser version, workflow assertions, API request counts
and statuses, process cleanup evidence, exit status, and removed temporary
home.

The focused run proves only the rendered fixed comparison and runner cleanup
on this host/browser version. It does not establish screen-reader acceptance,
cross-browser or cross-platform behavior, external market correctness,
availability, seller intent, execution, payment, source rights, independent
review, or production readiness.

## Boundaries and report

Non-goals: changing the product renderer, service/API contract, projection,
offer assessment, selection, capture explorer, local storage, or any imported
workspace; adding seller or provider calls; using Bazaar material; exercising
ACP's separate fixed-port integration; or changing the financial, Fleet,
Eval, settlement, custody, or authority gates.

Assurance target: INTERNAL_PASS only for the exact browser and cleanup
assertions if every proof gate passes. Reviewer status is NOT_REQUESTED;
reproduction starts as NOT_REPRODUCED. Data and artifact custody is limited
to HyperAI-authored synthetic records, a temporary local service home, and
volatile browser memory. Claim ceiling: the exact fixed local synthetic
comparison rendered through one local browser on the recorded host.

Publish boundary: local code and documentation only; no commit, PR, release,
deployment, API expansion, provider call, seller contact, external write, or
production effect.

## Recorded execution (2026-10-01)

Baseline revision is `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`,
with the pre-existing dirty worktree preserved. Two initial browser attempts
are retained as `INCONCLUSIVE`: the first cleanup exception masked its earlier
assertion; the second exposed a stale expected phrase (`Synthetic comparison
only`) and showed that the runner treated a signal-terminated service as live
because it checked `exitCode` without `signalCode`. Their disposable homes
were removed after their recorded service processes and ports were confirmed
gone; no source product files were changed.

The runner now treats either a normal exit code or a received signal as a
completed child process, while retaining process-group membership and listener
closure as separate cleanup evidence. The Node.js child-process contract
specifies that signal termination leaves `exitCode` null and sets
`signalCode`; its `killed` flag only confirms a signal was sent, not that the
child terminated. See the [Node.js child process
documentation](https://nodejs.org/download/release/v26.8.2/docs/api/child_process.html#subprocesssignalcode).

Final command: `corepack pnpm run verify:interface:market-supply-browser`.
`node --check scripts/verify-interface-browser.mjs` and
`node --test scripts/tests/browser-process-group.test.mjs` passed. The pinned
offline `hyperai-ui` build passed. The browser command exited 0 with Google
Chrome `154.0.8037.92`.

The keyboard workflow reached Market with visible focus and restored visible
focus to the workspace heading. Chromium exposed the supply panel as a named
region and the offers as a named table. Two offers were assessed and
`offer-gpu-b` was selected by asking price. Both advisory cards remained
separate and labeled “Not an offer”; freshness remained `not_evaluated`; the
Simple Compute Market capture explorer remained a separate section. At 390px
the document width was 390px while the offers table scrolled internally
(1153px content width, 354px viewport, `overflow-x: auto`).

The browser observed exactly one
`GET /api/v1/market/supply-comparison` with HTTP 200 and zero write requests.
The runner used service PID 37445, browser PID/process-group 37730, service
port 57664, and DevTools port 57665. The browser exited with code 0; the
service exited after SIGTERM; both loopback listeners were closed; the
temporary home and profile were removed.

Assurance is `INTERNAL_PASS` for these assertions on this host/browser only.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.
Custody was limited to the fixed HyperAI synthetic records, volatile browser
memory, and a disposable local service home. The claim ceiling is the exact
fixed local synthetic comparison rendered through one local Chrome version on
the recorded host. The run does not establish screen-reader acceptance,
cross-browser/platform behavior, live supply, upstream rights, seller intent,
execution, payment, settlement, independent review, or production readiness.
Phase 2 remains `NOT_ASSIGNED`; the next blocked authority
gates are platform qualification and source-specific rights, identity, and
authorization for any live supply.

Final writable-file hashes: `scripts/verify-interface-browser.mjs`
`1beacd753e48645d0f8abf5a1e166a0b9dd839d8ec9d51a00f0e54b7655db373`;
`package.json` `ef7e75130b7db670e42bae49559bad7d28ab204ae4353a0451e408fcda964d6e`.
The product renderer, API, projection, fixture, and imported trees retained
their frozen hashes.
`python3 scripts/verify_imports.py` also passed: Simple Compute Market
2189 files (SHA-256 `afe6d9363a24fa627d9530240c190b6e68cdd1b7d6425d54d0b7547d829207ab`),
HyperIntel 99 files (SHA-256 `62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`),
and JustAI 176 files (SHA-256 `e853b7319aa9c1e1f4a49d5717b48e634ac2d39cd551621aa0324c0708e290c4`).
