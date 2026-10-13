# Installed TUI Receipt-Digest Wrap Qualification

State slice: `hyperai.bazaar.desk_runtime.installed_tui_receipt_wrap.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.installed_tui_quote_postfix.v1`,
`hyperai.bazaar.desk_runtime.receipt_digest_wrap_model.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed; the one installed integration failed its receipt-digest
visibility assertion again.

## Purpose

The prior installed integration failed because the screen model did not
recognize a receipt digest split by terminal row wrapping. The local
screen-model repair passes a fixed 120-column fixture and rejects short,
non-hex, and overlong values. This slice checks the repaired observer against
the installed local synthetic quote flow and then allows the enclosing test
to continue through its existing resize, terminal restoration, cleanup, and
synthetic ACP checks.

No product or harness source changes are in scope.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `9e4030c7ba5664de0594cff0ada2c38ebe54c9e7733ea5949294535153a2badd` | Read-only wrapped-digest screen model |
| `scripts/tests/test_tui_ansi_screen.py` | `008d9d031efa19d3540d8eca2bedcb7f4e3f81ebeb7b12ea5194a4b60d83a0f8` | Read-only local parser fixtures |
| `crates/hyperai-tui/src/lib.rs` | `1f999f509d8e36e11b148808980e30d4ec489a24400d6ca627f127318ab71ec7` | Read-only quote and receipt rendering |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only enclosing installed integration |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` | Read-only offline installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Locked workspace build |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline dependencies |
| `docs/compute-bazaar-receipt-digest-wrap-packet.md` | `1b9b8e84cb4625e276a0e015627ba00a2bd904536bb5ab77ea405180b83e002c` | Local parser repair result |
| `docs/compute-bazaar-installed-tui-postfix-packet.md` | `a5cc644bee3df830f054464f86b521704885551a0681b6ea81c1d7b4a428c4d1` | Closed prior installed failure |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited Git status digest after recording this packet and state
slice, and before the run, is recorded in the state-slice entry.

## Exact writable files

- This packet and `docs/state-slices.md`: record preflight and outcome only.

All application source, screen-model harness, fixtures, installer,
dependencies, API, permissions, and package contents are read-only.

## Allowed effects and one-run limit

Allowed: verify all frozen hashes and temporary-path/port boundaries; run
exactly once:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The integration uses an offline installer, fixed synthetic quote and ACP
fixtures, a disposable install root and home, ephemeral loopback service
ports, and a POSIX PTY. Temporary paths must resolve outside the user-owned
HyperAI home. Stop on hash drift, port `4317`, or a temporary path inside that
home. No provider, seller, registry, external agent, remote worker, paid
execution, live Fleet, financial Trade, wallet, chain, non-loopback network,
publication, or deployment. Raw PTY text must not be retained.

The integration allowance is exactly one. Any failure or inconclusive result
closes this packet; another installed attempt requires a new named packet.

## Proof gates and claim ceiling

The installed screen report must observe the receipt heading, an exact
64-character receipt digest across any terminal row wrap, and the fixed
synthetic effect boundary after preview and approval. The enclosing test must
finish its compatibility, CLI, resize, Ctrl+C, terminal-restoration, cleanup,
and synthetic ACP assertions. No external or paid action occurs.

At most, a passing run establishes the installed local synthetic quote flow
and the named enclosing integration on the tested Darwin arm64 host. It does
not prove correct computation, real fulfillment/payment, general platform
support, accessibility, independent review, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
Publish boundary: local source and documentation only; no staging, commit,
PR, release, publication, or deployment.

## Result

Executed 2026-10-02 from HyperAI base revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under this slice. Every frozen
input hash matched, and the temporary root resolved outside the user-owned
HyperAI home. The locked offline integration exited 101 after 14.95 seconds
(`0 passed, 1 failed`).

The installed compatibility entrypoints and CLI lifecycle returned
successfully before the TUI check. The TUI screen model reported the receipt
heading and synthetic effect-boundary identifier, but still reported
`receipt_digest_displayed: false`; the assertion failed at
`crates/outcomes-cli/tests/cli_compat.rs:291`. Its safe report identified
`darwin arm64`, Python `3.14.5`, installed binary SHA-256
`ab4c4e6cea72aef80b5b3ad03976f5a90466f870887726c628db600ba5b7e948`, 23,580
transcript bytes, and transcript SHA-256
`9dc4ff3b4958e82a28fe4b38ecb3ac844fc0e3e581041fc843ebd19899d4719d`. Raw PTY
text was not retained. Resize, Ctrl+C exit, terminal restoration,
alternate-screen exit, cursor restoration, and temporary-home removal checks
passed. The later synthetic ACP check was not reached.

No matching `hyperai`, `hyperai-ui`, or `outcomes-cli` process and no
`hyperai-ctrlc-*` temporary directory remained after the run. No provider,
seller, remote worker, paid execution, chain, or external effect occurred.
Test result is `FAILED` for the frozen digest-visibility assertion; assurance
is `INCONCLUSIVE` about the installed visual state because the raw transcript
was not retained. The matcher-only repair passed its local synthetic tests
but did not resolve the installed assertion. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. This packet's one-run
allowance is consumed. A new named diagnostic packet is required before any
further installed TUI attempt. No commit, PR, release, publication, or
deployment occurred.
