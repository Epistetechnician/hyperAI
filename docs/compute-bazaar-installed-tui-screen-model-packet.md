# Installed TUI Screen Model Continuation

State slice: `hyperai.bazaar.desk_runtime.installed_tui_screen_model.v1`.
Parent slices: `hyperai.bazaar.desk_runtime.v1`,
`hyperai.bazaar.desk_runtime.tui_overview_render.v1`, and
`global-experiment-assurance-lifecycle-v1`.

Status: closed `INCONCLUSIVE`; its one local installed synthetic quote-flow
integration attempt is consumed.

## Purpose and evidence

The three preceding installed quote-flow packets closed `INCONCLUSIVE` before
any quote keys because raw transcript matching did not observe
`Workspace: Overview`. Their raw PTY bytes were not retained. The separate
TestBackend check in the parent renderer slice confirms that Overview content
exists in the in-memory Ratatui frame, but it does not explain terminal byte
rendering.

Use a bounded stateful ANSI screen model in the Python PTY harness to apply
cursor-addressed Ratatui diffs to an in-memory 120-by-40 screen. Assert the
visible text at each synthetic quote stage. Keep the screen contents and PTY
transcript in memory only. On failure, report only fixed-token booleans,
control-sequence counts, byte count, and transcript digest; never print or
persist raw terminal text.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
all pre-existing dirty and untracked work preserved.

| File | SHA-256 at freeze | Use |
|---|---|---|
| `scripts/verify_hyperai_tui_ctrl_c.py` | `b0ab0472ec2592556d1c002397672c3fcfe4ee36ce6c6209cb6032aa3a1caa53` | Writable PTY harness |
| `crates/outcomes-cli/tests/cli_compat.rs` | `114befb5e069f0e9265e1ce241ab2aec410768a30e5c3e6afb580d2fd884736c` | Read-only installed integration |
| `crates/hyperai-tui/src/lib.rs` | `74cc744b085abd8bec62323b624881ea3808a1090c85e9f1231201d20e71c64d` | Read-only product renderer and TestBackend assertion |
| `scripts/install-hyperai-local.mjs` | `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53ceebd44a7b9ddf49b3` | Read-only offline installer |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` | Read-only workspace manifest |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` | Locked offline build |
| `docs/compute-bazaar-installed-tui-quote-lifecycle-packet.md` | `5173a5cb67e2575c5c758ccf29c53456e84d2b71cf195d203271c2795b1388d0` | Closed first attempt |
| `docs/compute-bazaar-installed-tui-quote-startup-sync-packet.md` | `cc6e63ef0873864244bd97ffc6e3046d5e6daaa61bf494654cde407c082598a9` | Closed second attempt |
| `docs/compute-bazaar-installed-tui-marker-window-packet.md` | `d9aefbc7f6eaae1f669a67a3710b6b03f53b83947a44db8af991912e1ee4f9a3` | Closed third attempt |
| `docs/compute-bazaar-tui-overview-render-packet.md` | `8935091657ef418936094683e1b279571d41bbd65ac4ad7a1fe7ed07215bb5c1` | Read-only renderer evidence |
| `docs/compute-bazaar-rust-port-subplan.md` | `dc39608cfd04295ebab03b9e02b9e8de355af3bc7e664c94edba0daedcf7d6f7` | Read-only overall plan |

The NUL-delimited `git status --porcelain` SHA-256 after recording this packet
and state slice, and before changing the PTY harness or creating the parser
fixture file, is recorded in the state-slice entry.

## Exact writable files

- `scripts/verify_hyperai_tui_ctrl_c.py`: add the minimal ANSI screen model,
  feed it initial and subsequent PTY bytes, assert visible stage markers, and
  emit only fixed diagnostics on failure.
- `scripts/tests/test_tui_ansi_screen.py`: add deterministic screen-model
  fixtures for split CSI sequences, cursor-positioned updates, clear-line
  behavior, and split UTF-8 input.
- This packet and `docs/state-slices.md`: scope and final result only.

Do not change TUI, CLI, service, API, installer, dependencies, permissions,
package contents, domain contracts, or other tests. Do not retain raw PTY text.

## Allowed effects and one-run limit

Allowed: the Python screen-model unittest; Python syntax parsing; one exact
locked offline ignored installed integration:

```sh
CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline \
  -p outcomes-cli --test cli_compat \
  source_install_bundles_mcp_sidecar_and_connects_synthetic_peer \
  -- --ignored --exact --nocapture
```

The integration uses the installer in `--offline` mode, a disposable install
root/home, fixed synthetic quote and ACP fixtures, local loopback services on
port `0`, and a POSIX PTY. Before execution, verify every frozen input hash,
the port-0 paths, and that temporary paths resolve outside the user-owned
HyperAI home. Stop on drift, port `4317`, or a temporary path inside that home.
Also allowed: root `corepack pnpm run lint:fast`, pinned Rust formatting,
`git diff --check`, and scoped whitespace/process/temp-directory cleanup
checks. No provider, seller, registry, remote worker, external agent, paid
execution, live Fleet, financial Trade, wallet, chain, network access beyond
loopback, publication, or deployment.

The installed integration allowance is exactly one. If it fails or is
inconclusive, close this packet with that result and require a new packet
before any subsequent installed integration attempt.

## Proof gates and claim ceiling

The visible Overview and Market selection must precede preview, synthetic
approval, and mock receipt in order. The visible receipt must include its
digest and fixed synthetic effect-boundary text. Existing resize, Ctrl+C,
terminal restoration, service shutdown, installed CLI/ACP checks, disposable
home cleanup, the Python fixtures, enclosing integration, `lint:fast`, pinned
formatting, and diff hygiene must pass.

At most, this proves the installed local synthetic TUI quote flow and existing
installed integration on this Darwin arm64 host. It does not prove computation
correctness, real fulfillment or payment, native cross-platform support,
accessibility, independent review, or production readiness. Reviewer status
is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Publish boundary: local
source and documentation only; no staging, commit, PR, release, publication,
or deployment.

## Result

Executed 2026-10-02 from HyperAI base revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` under
`hyperai.bazaar.desk_runtime.installed_tui_screen_model.v1`.

The exact locked offline installed integration exited 101 (`0 passed,
1 failed`) after 17.11 seconds. The screen model observed Overview, Market,
preview, and approval in order. After `c`, it did not observe the visible
`MOCK FULFILLMENT AND RECEIPT` stage within five seconds. Safe diagnostics
reported Overview, Market, preview, and approval present; receipt and the
synthetic effect-boundary identifier absent; 635 CSI sequences; one alternate
screen entry; 8,015 transcript bytes; SHA-256
`c29744d747c92ef230934bcfcc5625aeb792c2efe3823ac4b135eb9821ea8977`. No raw
terminal text was printed or retained. The run stopped before resize, Ctrl+C,
and the integration's later ACP step.

The PTY child, local service, disposable home, and temporary install root were
cleaned. Post-run checks found no `hyperai-ctrlc-*` temporary directory and no
matching HyperAI/outcomes CLI process. The four parser fixtures passed; Python
AST parsing passed for both files; root `lint:fast` passed 34 interface checks,
3 installer checks, pinned formatting, and strict workspace Clippy. Assurance
is `INCONCLUSIVE` for the installed TUI quote lifecycle; reviewer status is
`NOT_REQUESTED`, reproducibility is `NOT_REPRODUCED`. Custody is local source,
fixed synthetic quote/ACP data, and in-memory diagnostics only; no raw PTY
transcript or other output artifact was retained.

The absent receipt marker does not distinguish an unhandled completion key, a
completion error, or receipt content outside the visible viewport. The next
blocked gate is a new local renderer/viewport diagnostic packet before any
further installed quote-flow attempt. This result does not establish a TUI
product failure, computation correctness, real fulfillment or payment,
platform support, accessibility, independent review, or production readiness.
No commit, PR, release, publication, or deployment occurred.
