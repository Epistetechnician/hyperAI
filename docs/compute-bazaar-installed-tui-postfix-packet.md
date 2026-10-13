# Installed TUI Quote Visibility Post-Fix Continuation

State slice: `hyperai.bazaar.desk_runtime.installed_tui_quote_postfix.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.v1`,
`hyperai.bazaar.desk_runtime.market_quote_visible_stage.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed; the one installed integration failed its receipt-digest
visibility assertion.

## Purpose

The parent slice moved the active quote/deal lifecycle section ahead of long
offer and observation details. Its in-memory 120-by-40 TestBackend assertion
passes for the receipt heading, digest, effect-boundary identifier, and
section order. This slice verifies the installed TUI presents that state
after its synthetic completion action and that the enclosing existing
installed integration finishes its resize, restoration, CLI, and ACP checks.
No product or harness source changes are in scope.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `crates/hyperai-tui/src/lib.rs` | `1f999f509d8e36e11b148808980e30d4ec489a24400d6ca627f127318ab71ec7` | Read-only post-fix renderer |
| `scripts/verify_hyperai_tui_ctrl_c.py` | `ecd1850422b5364017cabbda8b52ff5b0c35f83350ab6ead117535c93c098541` | Read-only ANSI screen harness |
| `scripts/tests/test_tui_ansi_screen.py` | `7401312a1e6a80f787b9a3e8ec05da63a27791a3b851568afd66ba6790138c3f` | Read-only parser tests |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only enclosing integration |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` | Read-only offline installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline build |
| `docs/compute-bazaar-market-quote-visible-stage-packet.md` | `f6dc6b2dbeeb7119369eb179b521ff2f42dd98f63828c424748a1c9ac75cb90a` | Closed parent diagnosis and fix |
| `docs/compute-bazaar-installed-tui-screen-model-packet.md` | `47489229cd4b155d091a14079abcf1310f2470e4ec6af7a169981e0b87be307c` | Prior safe screen model |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited Git status baseline after recording this packet and state
slice, and before the installed run, is `fbaf89793ef4bfa2bbe5fcb01f5aaa5df58b7ac33a2c3121258ea9a9a0db4483`.

## Exact writable files

- This packet and `docs/state-slices.md`: record scope and result only.

Product source, PTY harness, installer, tests, dependencies, API, permission
model, and package contents are read-only.

## Allowed effects and one-run limit

Allowed: verify every frozen input hash and the disposable path/port boundary;
run exactly once:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The integration uses the offline installer, fixed synthetic quote and ACP
fixtures, a disposable install root/home, loopback service port `0`, and a
POSIX PTY. Temporary paths must resolve outside the user-owned HyperAI home.
Stop on frozen-input drift, port `4317`, or a temporary path inside that home.
Also allowed: root `corepack pnpm run lint:fast`, pinned formatting,
`git diff --check`, and scoped process/temp-directory/whitespace checks. No
provider, seller, registry, external agent, remote worker, paid execution,
live Fleet, financial Trade, wallet, chain, non-loopback network, publication,
or deployment.

The integration allowance is exactly one. If it fails or is inconclusive,
close the packet and require a fresh packet before another installed attempt.

## Proof gates and claim ceiling

The screen model observes Overview and Market before preview, approval, and
mock receipt; after completion it sees the receipt digest and fixed synthetic
effect boundary. The enclosing integration passes its existing installed
CLI, ACP, resize, Ctrl+C, terminal restoration, service shutdown, and cleanup
assertions. Root `lint:fast`, formatting, and diff hygiene pass. No raw PTY
text is retained.

At most, this proves the installed local synthetic TUI quote lifecycle and
the existing local installed integration on the tested Darwin arm64 host. It
does not prove computation correctness, real fulfillment/payment, native
cross-platform support, accessibility, independent review, or production
readiness. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Publish boundary: local source and documentation only; no
staging, commit, PR, release, publication, or deployment.

## Result

Executed 2026-10-02 from HyperAI base revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under this slice. All frozen input
hashes matched before execution. The locked offline integration exited 101
after 26.29 seconds (`0 passed, 1 failed`). The installed compatibility
entrypoints and CLI lifecycle returned successfully before the TUI check.

The PTY report identified `darwin arm64`, Python `3.14.5`, and installed
binary SHA-256
`ab4c4e6cea72aef80b5b3ad03976f5a90466f870887726c628db600ba5b7e948`. The
screen model observed the receipt heading and synthetic effect boundary, but
reported `receipt_digest_displayed: false`; the integration then failed the
digest assertion at `crates/outcomes-cli/tests/cli_compat.rs:291`. The safe
report recorded 23,561 transcript bytes with SHA-256
`db88731cafc9f7b7784f29f5008308bd24da0af5f410c84bbb3025cc1a824f25`. Raw PTY
text was not retained.

The child TUI report also passed its resize checks (48-by-10 and 100-by-30),
Ctrl+C exit, terminal restoration, alternate-screen exit, cursor restoration,
and temporary-home removal assertions. The enclosing integration therefore
did not reach its later synthetic ACP check. No provider, seller, remote
worker, paid execution, chain, or external effect occurred. No matching
`hyperai`, `hyperai-ui`, or `outcomes-cli` process and no `hyperai-ctrlc-*`
temporary directory remained after the run.

Test result is `FAILED` for the frozen digest-visibility assertion; assurance
is `INCONCLUSIVE` for whether the digest was visually absent versus missed by
the bounded screen model. Local renderer ordering remains `INTERNAL_PASS`
from the parent slice only. Reviewer status is `NOT_REQUESTED`; reproduction
is `NOT_REPRODUCED`. Custody is local source, fixed synthetic quote/ACP data,
and bounded in-memory diagnostics. The one-run allowance is consumed. The
next gate is a fresh named packet to diagnose the digest visibility contract
and qualify any corrected local screen/UI behavior before another installed
integration. Platform, accessibility, independent review, and production
gates remain open. No commit, PR, release, publication, or deployment
occurred.
