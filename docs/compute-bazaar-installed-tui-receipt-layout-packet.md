# Installed TUI Receipt-Digest Layout Qualification

State slice: `hyperai.bazaar.desk_runtime.installed_tui_receipt_layout.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.tui_receipt_digest_layout.v1`,
`hyperai.bazaar.desk_runtime.installed_tui_receipt_frame_wait.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: completed; the one installed synthetic integration allowance was used.

## Purpose

The local TUI now places the deal digest and receipt digest on separate lines.
A locked TestBackend fixture proves the complete 64-character receipt digest
and `Receipt digest:` label fit together on one row at 120-by-40. The observer
waits for that exact full digest. This one installed run qualifies the revised
layout and the enclosing compatibility, CLI, resize, terminal-restoration,
cleanup, and synthetic ACP checks.

The three earlier installed packets failed to detect the digest. This packet
allows one post-layout run only. No raw PTY text is retained.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `crates/hyperai-tui/src/lib.rs` | `b45d17fb1a30b02ddc906986bf908b9b925c3c0d6306e936d503825e5aa97177` | Read-only separate-line receipt layout and TestBackend test |
| `scripts/verify_hyperai_tui_ctrl_c.py` | `ee9caf5978f6fca45e79d6f143df9237279e6ca9fee37e5aaba4e642caefd2f7` | Read-only receipt-digest wait and safe diagnostics |
| `scripts/tests/test_tui_ansi_screen.py` | `2ea06458415ade3ce66751f7d004d90aaf5941a5a8f5f21a58b0846e713c2971` | Read-only screen-model fixtures |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only installed integration caller |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3` | Read-only offline installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Locked workspace build |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline dependencies |
| `docs/compute-bazaar-tui-receipt-digest-layout-packet.md` | `793f7a01cc06ac3905d6044af6acb0319b1d1e907ba36f10d3b2b6a72b0c602f` | Local layout and viewport proof |
| `docs/compute-bazaar-installed-tui-complete-frame-packet.md` | `1f357761fc5d029698ec52d9c2cbe66e5383b2c02e5412092e365c43609c388d` | Closed installed failure evidence |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited Git status digest after recording this packet and state
slice, and before the run, is recorded in the state-slice entry.

## Exact writable files

- This packet and `docs/state-slices.md`: record preflight and outcome only.

All application source, observer harness, tests, installer, dependencies,
API, permissions, and package contents are read-only.

## Allowed effects and one-run limit

Allowed: verify every frozen input hash and the temporary-path/port boundary;
run exactly once:

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
closes this packet; any further installed attempt requires a new named packet.

## Proof gates and claim ceiling

The installed screen must show the receipt heading, exact 64-character
receipt digest on its separate row, and the fixed synthetic effect boundary.
The enclosing test must finish compatibility, CLI, resize, Ctrl+C,
terminal-restoration, cleanup, and synthetic ACP assertions.

At most, a passing run establishes this installed local synthetic lifecycle
and the named enclosing integration on the tested Darwin arm64 host. It does
not prove computation correctness, real fulfillment/payment, general
platform support, accessibility, independent review, or production
readiness. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Publish boundary: local source and documentation only; no
staging, commit, PR, release, publication, or deployment.

## Result

The exact frozen command completed with exit code 0 on Darwin arm64, using
Python 3.14.5. The integration result was `1 passed; 0 failed; 0 ignored`;
the test body took 16.41 seconds and the command took 24.63 seconds. The
installed binary SHA-256 was
`0ba5d51079ce398a7fb9f7e1a8a2edd3843da9c66a130a8eef7951a51c96db5d`.

The safe PTY report recorded all required quote markers, including the
completion message, receipt heading, exact receipt digest, and synthetic
effect boundary. The transcript was 23,571 bytes with SHA-256
`afd0dfe45374e5db821694b4393af41bf21c828bb826be16cc033202f0016eb4`; raw PTY
text was not retained. The test also reported alternate-screen entry/exit,
raw-mode and terminal restoration, cursor restoration, and Ctrl+C exit as
successful. Resize checks passed at 48-by-10 and 100-by-30. The synthetic ACP
permission probe returned `PermissionDenied`; the test exercised the denied
owner-workload trace path, restored fixture permissions, and completed the
remaining assertions. The temporary installed home was removed.

Post-run cleanup found no `hyperai`, `hyperai-ui`, or `outcomes-cli` process,
no `hyperai-ctrlc-*` temporary directory in the test temp root, and
`git diff --check` passed. The NUL-delimited Git status digest remains
`a5aa1907d3b1d6a95ec2ce1d58418b36abf312e8bdc8c28c37908b54638a7441`; only
the already declared packet and state-slice documentation were changed for
this result.

Assurance: `INTERNAL_PASS` for this exact installed, local synthetic quote
lifecycle and its named enclosing integration on the tested Darwin arm64
host. Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`.
Custody: fixed synthetic quote/ACP fixtures and bounded in-memory PTY
diagnostics; only the summary and transcript digest above are retained. The
run does not establish provider behavior, computation correctness, real
fulfillment or payment, independent review, broader platform support, or
production readiness. The one-run allowance is consumed; another installed
attempt requires a fresh named packet.
