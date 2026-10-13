# HyperAI Current-Shell Contrast Qualification Packet

State slice: `hyperai.bazaar.interface.current_shell_contrast.v1`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md) and
[Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Objective

Re-run the existing solid-background text-contrast and route-layout audit
against the current HyperAI shell source. The prior contrast packet measured
an earlier shell revision; later market-history and workspace changes have
changed the interface source. This packet covers the fixed, initially empty
shell routes only. It does not exercise populated workflows.

The method applies WCAG 2.2 Success Criterion 1.4.3 thresholds: at least 4.5:1
for ordinary text and 3:1 for large text. See the [W3C Understanding
document](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
Text on computed gradients/images or other complex backgrounds remains
`review_required`; this audit does not evaluate non-text contrast.

## Frozen baseline

- HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, branch `main`.
- Existing NUL-delimited `git status --porcelain=v1 -z` SHA-256:
  `40f403a55dab64bcf4b5b85890a9fa48aa4e5494d37bd804f7f5e4dc93f0cd54`.
  The checkout is extensively dirty and untracked; preserve every unrelated
  path.
- Pre-run status SHA-256 after adding this packet and slice record:
  `c40d4ae091d0c77c24a6298a2d11b4e290716a32ffd13f502fa0bcc99ee02b41`.
- Host: `Darwin 25.6.0 arm64`.
- Node.js `v22.23.2`; pnpm `11.7.0`; Rust/Cargo `1.93.1`; Google Chrome
  `154.0.8037.92`.
- Frozen file identity (`bytes`, SHA-256):

| File | Bytes | SHA-256 |
|---|---:|---|
| `apps/hyperai-interface/index.html` | 4,978 | `311a2504aa252507cf3bf0b84838467bda0dbd6bd3a564b1ffa14e6720f6cd73` |
| `apps/hyperai-interface/app.js` | 161,096 | `3c960ee6022a76a1edc50739bc22854701a35344576b32d63e819bece3093f58` |
| `apps/hyperai-interface/app.css` | 95,193 | `3d0702db97e4b18d60585be7c802c1cc6d6a406e338da8519aa09bd4610ed8e0` |
| `scripts/verify-interface-browser.mjs` | 192,347 | `c2e26a7a7dd61fe6da26f7b75ba0195068ecdc21faf8f794ac3661bb1e0a3e5a` |
| `package.json` | 8,319 | `42e6796735cc5176feb5bb1e50ad2a908c35c56d5af6404c4be091efecb8faf7` |
| `Cargo.toml` | 3,449 | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | 152,785 | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `rust-toolchain.toml` | 86 | `7134cca955184c22a7374bda78be176b570e6d8b69ee92dc02d5efc3044212e8` |
| `pnpm-lock.yaml` | 114 | `17c814b167307942d3609c7b9d916ceddb85839573ab39baa114e30edb132a1a` |

## Exact scope and effects

Writable files are this packet, `docs/state-slices.md`, and the current status
note under `docs/bazaar-hyperai-interface-plan.md`. The application, browser
runner, package manifests, imported products, and all other source files are
read-only. This slice authorizes no code or CSS correction; any proven failure
requires a fresh, exact correction packet before editing source.

Allowed effects are one offline build of `hyperai-ui`, one runner-owned local
service on an ephemeral IPv4-loopback port, one fresh headless Chrome profile,
and one execution of `corepack pnpm run verify:interface:contrast`. The runner
may visit its fixed 11 shell routes at 1440x1000, 720x500, and 390x844 CSS
pixels, perform only its existing read-only `GET`/`HEAD` requests, and create
temporary local browser state that it removes during cleanup.

Do not access the existing service or home on `127.0.0.1:4317`; invoke no
workflow, save, approval, evaluation, Trade, ACP-agent, provider, or service
action; and do not access external data or services. Do not retain screenshots
or temporary profiles. Stop on the first browser, route, request-boundary, or
cleanup failure. The one-run allowance is then consumed; diagnose under a new
slice.

## Proof gates and claim ceiling

- Confirm the command is the frozen root contrast command and record Chrome,
  source/binary identities, route results, eligible sample count, minimum
  ratio, failures, and `review_required` samples.
- Pass all 33 fixed route/viewport and reduced-motion checks; no eligible
  ordinary-text sample falls below 4.5:1 and no eligible large-text sample
  falls below 3:1.
- Confirm requests stay within the runner's same-origin read-only policy,
  with no browser/runtime error or horizontal overflow; confirm runner-owned
  processes and temporary state are removed and port 4317 remains untouched.
- Run `corepack pnpm run verify:imports`, `git diff --check`, and local
  Markdown-link validation. Recheck the frozen source hashes and the full
  worktree status digest.

Maximum claim: `INTERNAL_PASS` for measured text on computed solid-color
backgrounds and fixed route/layout behavior in the current shell on this
macOS/Chrome host. This is not a populated-workflow audit, complete WCAG
conformance, complex-background or non-text contrast acceptance, screen-reader
or physical-keyboard evidence, cross-browser/platform support, independent
review, or production authorization. Bazaar remains reference-only; no
external effects are authorized.

## Execution record (2026-10-02)

The one authorized `corepack pnpm run verify:interface:contrast` command
returned exit code 1 with `status: contrast-failed`. Route checks covered 33
route/viewports; the browser observed 22 requests, all `GET`. Reduced-motion
checks passed for all 33 route/viewports and 23,895 computed-style entries.
The contrast audit checked 3,025 samples, classified 213 as
`review_required`, and found 108 failing occurrences across nine selector
groups. The minimum was 2.5381:1 for `.supply-effects` on `#f1f3ee` on the
Market route. The complete grouped measurements are preserved in the
[correction packet](compute-bazaar-interface-current-shell-contrast-repair-v1.md).

Cleanup passed: Chrome exited with code 0 and no live process-group members;
the service exited on `SIGTERM`; both ephemeral listeners closed; temporary
state was removed. The runner used ports 62215 and 62216. It did not access
the pre-existing service on port 4317. The frozen file hashes listed above
were unchanged after the run.

Source-freeze limitation: the initial packet omitted four HTML-loaded helper
scripts. Their digests were captured after the run, so the full rendered-source
identity is `INCONCLUSIVE`. The browser output nevertheless directly records
the failing rendered foreground/background combinations. No product source
was changed under this one-run slice.

Audit result: `FAIL` for the measured solid-background text thresholds.
Assurance level: `NOT_ASSIGNED` for contrast acceptance. Reviewer status:
`NOT_REQUESTED`; reproduction status: `NOT_REPRODUCED`. The nine measured
foreground corrections are separately scoped in the linked fresh packet.
