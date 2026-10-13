# Installed TUI Complete-Frame Receipt Qualification

State slice: `hyperai.bazaar.desk_runtime.installed_tui_receipt_frame_wait.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.tui_receipt_frame_wait.v1`,
`hyperai.bazaar.desk_runtime.installed_tui_receipt_wrap.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed; the one installed integration timed out waiting for the
receipt-digest predicate.

## Purpose

The current observer waits for the receipt heading and exact 64-character
digest predicate, reading further PTY chunks until the digest appears or its
five-second deadline expires. Fixed local chunked-screen fixtures and root
`lint:fast` pass. This one installed run checks the amended observer and then
allows the enclosing existing integration to continue through resize,
terminal restoration, cleanup, and synthetic ACP assertions.

The two previous installed packets failed at the digest-visibility
assertion. This attempt has a distinct, named observer change and is limited
to one run. No raw PTY text is retained.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `564dfb676dc73854939c960e4c76fcabd292211a67fd185eeda970ac6ff2ccbd` | Read-only completion-aware PTY observer |
| `scripts/tests/test_tui_ansi_screen.py` | `517ca48f5ab04cf3007e4ca6683c96b0f04015f9536fe3f8218006b1b3532b51` | Read-only observer unit fixtures |
| `crates/hyperai-tui/src/lib.rs` | `1f999f509d8e36e11b148808980e30d4ec489a24400d6ca627f127318ab71ec7` | Read-only quote and receipt renderer |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only installed integration caller |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` | Read-only offline installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Locked workspace build |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline dependencies |
| `docs/compute-bazaar-tui-receipt-frame-wait-packet.md` | `6ee673def72490b0e21158aa60bb785f9c080c89424761474606310335d90fc4` | Local observer correction and proof |
| `docs/compute-bazaar-installed-tui-receipt-wrap-packet.md` | `012d739e5770e1d0233f63bd9148dd86729d6cefd4ca9fc55e781eb354bc13be` | Closed matcher-only installed failure |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited Git status digest after recording this packet and state
slice, and before the run, is recorded in the state-slice entry.

## Exact writable files

- This packet and `docs/state-slices.md`: record preflight and outcome only.

All application source, observer harness, tests, installer, dependencies,
API, permissions, and package contents are read-only.

## Allowed effects and one-run limit

Allowed: verify the frozen hashes and temporary-path/port boundary; run
exactly once:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The integration uses offline build and install, fixed synthetic quote and ACP
fixtures, disposable install/home paths, ephemeral loopback ports, and a POSIX
PTY. Temporary paths must resolve outside the user-owned HyperAI home. Stop on
hash drift, port `4317`, or a temporary path inside that home. No provider,
seller, registry, external agent, remote worker, paid execution, live Fleet,
financial Trade, wallet, chain, non-loopback network, publication, or
deployment.

The installed allowance is exactly one. Any failure or inconclusive result
closes this packet; another installed attempt requires a new named packet.

## Proof gates and claim ceiling

The final modeled screen after the completion action must contain the receipt
heading, exact 64-character receipt digest, and fixed synthetic effect
boundary. The enclosing test must finish its compatibility, CLI, resize,
Ctrl+C, terminal-restoration, cleanup, and synthetic ACP assertions.

At most, a passing run establishes this installed local synthetic flow and
the named enclosing integration on the tested Darwin arm64 host. It does not
prove computation correctness, real fulfillment/payment, general platform
support, accessibility, independent review, or production readiness.
Reviewer status is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`.
Publish boundary: local source and documentation only; no staging, commit,
PR, release, publication, or deployment.

## Result

Executed 2026-10-02 from HyperAI base revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under this slice. All frozen
hashes matched and the temporary root resolved outside the user-owned
HyperAI home. The locked offline integration exited 101 after 17.70 seconds
(`0 passed, 1 failed`). The observer waited the full five-second predicate
deadline, then failed because its modeled screen contained no receipt digest.

Safe timeout diagnostics recorded 910 CSI sequences, one alternate-screen
entry, 13,711 transcript bytes, and SHA-256
`3c0ac8efacb86086ad74c479ebfa34719a6e224021ce896a7abfc62f43401ad3`. Fixed
screen markers reported the receipt heading, `receipt:` label, successful
mock-completion message, and synthetic effect-boundary identifier present;
`receipt_digest_visible` was false. Raw PTY text was not retained. The
installed compatibility entrypoints and CLI lifecycle had returned
successfully before the PTY check; later resize, Ctrl+C restoration, and ACP
assertions were not reached in the enclosing test. The harness cleanup
removed its temporary home. No matching `hyperai`, `hyperai-ui`, or
`outcomes-cli` process and no `hyperai-ctrlc-*` directory remained.

Test result is `FAILED` for the digest-wait assertion; assurance is
`INCONCLUSIVE` about whether the receipt digest is absent from the rendered
screen or missed by the modeled screen. Reviewer status is `NOT_REQUESTED`;
reproduction is `NOT_REPRODUCED`. The one-run allowance is consumed. The
next gate is local TestBackend coverage with a full-length digest and a
bounded visual-token diagnostic plan; no further installed attempt is
authorized by this packet. No provider, seller, paid execution, chain, or
external effect occurred. No commit, PR, release, publication, or deployment
occurred.
