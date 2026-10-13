# HyperAI Current-Shell Contrast Repair Packet

State slice: `hyperai.bazaar.interface.current_shell_contrast_repair.v1`.
Diagnostic predecessor: [current-shell contrast audit](compute-bazaar-interface-current-shell-contrast-v1.md).
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Objective

Correct only the foreground colors identified by the one-run 2026-10-02
current-shell contrast audit, then re-run that exact shell-route audit once.
The correction reuses the existing readable muted foreground token
`#53645a` (CSS `var(--muted)`). The token measures 5.2844:1 against the
lightest reported failing background (`#f1f3ee`) and 5.9044:1 against white.

The diagnostic run measured these failures:

| Failing text selector | Foreground | Background | Ratio | Occurrences |
|---|---|---|---:|---:|
| `.supply-effects` | `#929c94` | `#f1f3ee` | 2.5381 | 3 |
| `.connection-module-id code` | `#8b978d` | `#ffffff` | 3.0395 | 33 |
| `.supply-table h3 span` | `#89958d` | `#ffffff` | 3.1124 | 3 |
| `.connection-capabilities h4` | `#89958c` | `#ffffff` | 3.1151 | 33 |
| `.market-history > .retention-note` | `#87928a` | `#ffffff` | 3.2264 | 3 |
| `.market-capture-section > .section-heading p:not(.eyebrow)` | `#7c8980` | `#f1f3ee` | 3.2704 | 3 |
| `.snapshot-time` | `#869188` | `#ffffff` | 3.2714 | 3 |
| `.connection-capabilities .connection-no-capabilities` | `#7c877e` | `#f7f9f6` | 3.5271 | 12 |
| `.trace-connection-state` | `#78867c` | `#ffffff` | 3.8184 | 15 |

The run reported 108 failing occurrences in total, across the 33 fixed
route/viewports. Its exact output is recorded in the diagnostic packet. The
current audit applies [WCAG 2.2 SC 1.4.3](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html):
4.5:1 for ordinary text and 3:1 for large text.

## Frozen baseline

- HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`, branch `main`.
- Pre-edit NUL-delimited worktree status SHA-256:
  `de1b33a21186a34c5163750a237add63af82252aa9fd28b29dbaec730707531f`.
- Host/toolchain: `Darwin 25.6.0 arm64`; Node.js `v22.23.2`; pnpm `11.7.0`;
  Rust/Cargo `1.93.1`; Google Chrome `154.0.8037.92`.
- Worktree is extensively dirty and untracked. Preserve unrelated changes.
- Exact pre-edit file identity (`bytes`, SHA-256):

| File | Bytes | SHA-256 |
|---|---:|---|
| `apps/hyperai-interface/index.html` | 4,978 | `311a2504aa252507cf3bf0b84838467bda0dbd6bd3a564b1ffa14e6720f6cd73` |
| `apps/hyperai-interface/app.js` | 161,096 | `3c960ee6022a76a1edc50739bc22854701a35344576b32d63e819bece3093f58` |
| `apps/hyperai-interface/error-envelope.js` | 1,344 | `16bd46b87e8ba899779f7c51dc9a477feebcb16277fcac3540579cc4cea21874` |
| `apps/hyperai-interface/workload-trace.js` | 13,992 | `9735ace26c0325bdfcae9fae7b20e7de8671fcdc6192a98878940edc40d4c40e` |
| `apps/hyperai-interface/connection-inventory.js` | 4,751 | `77b595b757ac6caf1043e2b0f6629ae0a97b56bf9d923f6cff7ec19c51aa42fd` |
| `apps/hyperai-interface/market-supply-comparison.js` | 8,636 | `0042f7eb6bf11447fc1a29317741edb98b489f1e9320b37d859a7c2571a29685` |
| `apps/hyperai-interface/app.css` | 95,193 | `3d0702db97e4b18d60585be7c802c1cc6d6a406e338da8519aa09bd4610ed8e0` |
| `scripts/verify-interface-browser.mjs` | 192,347 | `c2e26a7a7dd61fe6da26f7b75ba0195068ecdc21faf8f794ac3661bb1e0a3e5a` |
| `package.json` | 8,319 | `42e6796735cc5176feb5bb1e50ad2a908c35c56d5af6404c4be091efecb8faf7` |
| `Cargo.toml` | 3,449 | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | 152,785 | `af3a7e2b96c0c5b4b59d0c59305e61812f1fcf829ab4575900db09cdf2cb23e2` |
| `rust-toolchain.toml` | 86 | `7134cca955184c22a7374bda78be176b570e6d8b69ee92dc02d5efc3044212e8` |
| `pnpm-lock.yaml` | 114 | `17c814b167307942d3609c7b9d916ceddb85839573ab39baa114e30edb132a1a` |

The complete `index.html` script list is frozen above, including all four
helper modules omitted from the predecessor packet.

Post-edit CSS identity: `apps/hyperai-interface/app.css`, 95,318 bytes,
SHA-256 `0a8e9275370a867328e30005c6991d49f159f811e43f708e7940849e1332b8c6`.
All other frozen source identities remain unchanged at preflight.

## Exact files and allowed effects

Only `apps/hyperai-interface/app.css` may change as product source. Change
only the foreground colors for the nine measured selector groups. Where an
existing rule combines a failing selector with unrelated selectors, split or
override narrowly so the correction does not recolor unrelated text. Keep
layout, typography, markup, wording, JavaScript, API, and domain behavior
unchanged.

The packet and `docs/state-slices.md` may record source identities and results.
No other file is writable. Allowed effects are one locked offline `hyperai-ui`
build and one run of `corepack pnpm run verify:interface:contrast` using its
ephemeral IPv4-loopback service and fresh headless Chrome profile. The runner
may issue only its fixed same-origin read-only route requests and may create
temporary state that it removes on cleanup.

Do not access port 4317 or its home; do not invoke any workflow, mutation,
provider, agent, seller, network, or service-action control. Do not retain
screenshots or temporary state. Stop on the first unexpected request, browser
or cleanup error, or remaining contrast failure. The one-run allowance is
consumed in every case; a further correction requires a fresh packet.

## Proof gates and claim ceiling

- Only the nine listed foreground selectors change; all frozen read-only
  source hashes remain identical and the post-edit CSS hash is recorded.
- The single browser command passes all 33 route/viewports and all 33
  reduced-motion checks. Every eligible solid-background text sample meets
  its threshold; complex-background samples remain `review_required`.
- Requests remain same-origin `GET` only; no runtime or console error, no
  horizontal overflow, and complete runner/service/listener/temp-state cleanup
  are recorded.
- `corepack pnpm run verify:imports`, `node --check
  scripts/verify-interface-browser.mjs`, `git diff --check`, and local packet
  link checks pass. Record post-run source hashes and full worktree status
  digest.

Maximum claim: at most `INTERNAL_PASS` for the measured current empty-shell
solid-background text and fixed route/layout behavior on the recorded host.
Populated workflows, complex backgrounds, non-text contrast, screen-reader
or physical-keyboard use, cross-browser/platform support, full WCAG
conformance, independent review, and production authorization are excluded.
The prior measured `FAIL` remains preserved in its diagnostic packet.

## Execution record

The single authorized `corepack pnpm run verify:interface:contrast` run
returned exit code 0 with `status: passed`. Browser contrast reported
`passed-with-review-required`: 33 route/viewports, 3,025 samples, zero
threshold failures, and 213 samples on complex backgrounds retained as
`review_required`. The minimum eligible ratio was 4.5622:1 for ordinary text
in the desktop Deals route (`.market-history-empty`, foreground
`rgb(102, 116, 107)`, background `rgb(245, 247, 245)`). The nine corrected
selector groups no longer fail.

All 33 reduced-motion checks passed across 23,895 computed-style entries.
The runner observed 22 same-origin requests, all `GET`, with no workflow or
service action. Chrome exited with code 0 and no live process-group members;
the local service exited by `SIGTERM`; ephemeral ports 62607 and 62608 closed;
temporary state was removed. Port 4317 was not accessed.

Post-run hashes match the frozen preflight identities for every read-only
input. The CSS remains 95,318 bytes with SHA-256
`0a8e9275370a867328e30005c6991d49f159f811e43f708e7940849e1332b8c6`.
The NUL-delimited worktree status digest remains
`de1b33a21186a34c5163750a237add63af82252aa9fd28b29dbaec730707531f`.
Import verification, runner syntax validation, `git diff --check`, local
packet-link validation, and trailing-whitespace checks passed.

Assurance level: `INTERNAL_PASS` for the measured current empty-shell
solid-background text and fixed route/layout behavior on this host. Reviewer
status: `NOT_REQUESTED`. Reproduction status: `NOT_REPRODUCED`. Complex
backgrounds remain unqualified; populated workflows, non-text contrast,
assistive technology, other browsers/platforms, full WCAG conformance,
independent review, and production authorization remain outside the claim.
