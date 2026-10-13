# HyperAI ACP Keyboard Cancellation Qualification Packet v5

Status: closed `INCONCLUSIVE` after the one authorized local synthetic browser
run. ACP cancellation identities v1-v4 remain closed `INCONCLUSIVE` and will
not be repeated. Do not rerun v5.

State slice: `hyperai.bazaar.interface.acp_cancel_keyboard.v5`.
Parent: [HyperAI interface plan](bazaar-hyperai-interface-plan.md).
Implementation sequence: [Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md).
Prior result: [v4 packet](compute-bazaar-interface-acp-cancel-keyboard-v4-packet.md).
Cleanup diagnostic: [process-state packet](compute-bazaar-interface-cleanup-diagnostic-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Purpose and exact scope

Exercise one fixed synthetic ACP keyboard-cancellation journey with the
current runner's process-state-aware cleanup. The runner now records
runner-owned browser PID/state pairs and treats a defunct-only process group
as execution-empty; this packet measures that behavior in one new browser
identity. The previous v4 packet retained no process states, so this run does
not presume that v4 observed a zombie or that the parser resolves its cause.

The local service, browser UI, ACP session contract, synthetic fixture,
MCP implementation, and imported product workspaces are frozen inputs. The
MCP binary may be built because the established command requires it, but the
sidecar must not be launched and no MCP tool may be called. No app, service,
ACP, MCP, fixture, or product source is modified in this slice.

## Frozen inputs

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the complete current dirty and untracked worktree. Preserve every existing
path. Before the run, the direct input digests are:

| Input | SHA-256 |
|---|---|
| `package.json` | `7a9ef6b1f4487190fe831e470d07ea1d9163d9f28a6a655d510657ba55f625c2` |
| `Cargo.toml` | `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487` |
| `Cargo.lock` | `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a` |
| `scripts/with-pinned-rust.sh` | `d0bb19b6f621a31e10a7b46be852889a4f69bd0196ba7545abd9219b36f51171` |
| `scripts/verify-interface-browser.mjs` | `837f73d3e14297611289f74e5643480d09417380e32bfe763a99374ba168cfe0` |
| `scripts/browser-process-group.mjs` | `540e8d516fb84dd4dba569987800c21083399466c7d25b3ce4daf6a024bed6ef` |
| `scripts/tests/browser-process-group.test.mjs` | `f930128679413b8f1881bc8fd206c745209f5a1b0e2702a3cbdee688349288b9` |
| `apps/hyperai-interface/index.html` | `728054814d02dbe4f0758322dc0e00eea1b8cd90ec7af88182b7fd5e32314ade` |
| `apps/hyperai-interface/app.js` | `fbfdbc90fcfd7f12de22ff86bc2cc78a6e673df55d42355b4434b8dfa8cbd464` |
| `apps/hyperai-interface/app.css` | `769feaaf75333e99b3324fc683b6fe5967ebcb38892f870a8dc3499b478dddc3` |
| `crates/outcomes-cli/src/bin/hyperai-ui.rs` | `04da2c5f50f01c7a8b95bb4914725d0e9253018b55a9bdf520449df04f68be5d` |
| `crates/hyperai-acp/Cargo.toml` | `86db2fa85795ffcf7d89876526f8eb21948353fa5bdd48bc490aad14c75a726c` |
| `crates/hyperai-acp/src/session.rs` | `26e7492b4445c37611d7575128a134bcb4c74226fe9693d6c0a5978c49e68036` |
| `crates/hyperai-acp/src/bin/acp-fixture-peer.rs` | `0a2401b9d07e492aac398233d791a918b38a62ec8e4e125a4b6b15da5acdb556` |
| `crates/hyperai-mcp/Cargo.toml` | `6141d8f52750fab10aabdd33347a82176f3b238c6fbe0707947682046ea304bf` |
| `crates/hyperai-mcp/src/main.rs` | `60dd4b5a09b5403daf14607a46b7a5f51a7657ef230e31d4ad4c4ab33e78a8bc` |
| `crates/hyperai-mcp/src/lib.rs` | `73daab3f0e330d66518930d263257072fb74a540bd7aa35ba258f52f398a1022` |
| `crates/hyperai-local-service/src/server.rs` | `de1d5b6486110b6ed1859698ecee8db7e76149933d8ad84a1f5abca99e4fb950` |
| `crates/hyperai-local-service/src/auth.rs` | `2db8388d29b0f3e5e7951d7c9ffdb8b7c56451b5941a7b502cb51f8fc19f7af1` |

The complete source-tree input set `crates/`, `products/hyperintel/`, and
`apps/hyperai-interface/` contains 216 files. Its deterministic digest is
`e1842a829e71d1b5f95fb87da4ef00422e790f974fb46c9214a25a75335089de`, formed
by sorting repository-relative file paths and hashing each path, a NUL byte,
the eight-byte big-endian file length, and file contents. Exclude only
`target`, `node_modules`, and `.git` directories. Recompute before and after
the run.

Toolchain: macOS Darwin arm64; Node.js `v22.23.2`; pnpm `11.7.0`; pinned
Rust/Cargo `1.93.1`; Google Chrome
`154.0.8037.92` at
`/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

## Allowed effects and stop conditions

Before the browser journey, pass JavaScript syntax, root `lint:fast`, root
`verify:contracts`, source import verification, and the frozen-input checks.
Then run exactly once:

```sh
CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-cancel-keyboard
```

The command performs locked offline builds for `hyperai-ui`, `hyperai-mcp`,
and `acp-fixture-peer`, then runs the fixed local keyboard-cancellation
workflow using one disposable service home, Chrome profile, fixture process,
and kernel-assigned IPv4 loopback ports. Chrome version is checked before
service or browser startup. Chrome may access only same-origin local `GET`/
`HEAD` plus one HTTP 200 `POST` to each route:

- `/api/v1/agent/connect`
- `/api/v1/agent/prompt`
- `/api/v1/agent/cancel`
- `/api/v1/agent/disconnect`

The browser host-resolver policy denies other origins. Do not contact port
`127.0.0.1:4317`, any provider, seller, registry, chain, other local service,
or reviewer. Do not launch the MCP sidecar or call MCP tools. No external
publication, testnet write, paid execution, federation, deployment, or
production action is allowed.

If the command fails, close this identity with its literal result and do not
retry it. The runner may inspect and signal only its own recorded browser
process group and service PID. If ownership cannot be confirmed, stop and
preserve the exact temporary root. Remove temporary state only after the
runner confirms the browser execution group and service have exited. Any
post-run operator cleanup must use read-only process inspection for those
exact recorded IDs and may signal only a process whose command and profile
path identify it as this run's child.

## Proof gates and claim ceiling

1. The frozen source digests, toolchain, and exact Chrome version match.
2. Fixed cancellation assertions pass: keyboard route/form use, pending
   prompt, named cancel control, visible focus, cancelled terminal state,
   focus restoration, and keyboard disconnect.
3. Exactly four allowed ACP POSTs occur once each with status 200; no MCP
   tool is called and no external-origin request succeeds.
4. Cleanup reports the browser close result, signal fallback, final PID/state
   pairs, service exit, and temporary-state removal. A live non-defunct member
   or service is not treated as a pass.
5. The source-tree digest is unchanged after the run; root fast/contract
   checks, import preservation, and documentation hygiene are recorded.

Maximum claim: `INTERNAL_PASS` for this exact same-host Chrome synthetic
cancellation workflow and its recorded cleanup only. Reviewer status is
`NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. This does not establish
physical keyboard or screen-reader behavior, other browsers/platforms,
complete ACP policy parity, source rights, provider execution, paid execution,
federation, settlement, publication, Phase 8 completion, or production
readiness. The earlier root `verify:full` `HomeBusy` remains unresolved; this
packet excludes that full-suite rerun.

## Execution result (2026-10-01)

The frozen command
`CARGO_NET_OFFLINE=true corepack pnpm run verify:interface:acp-cancel-keyboard`
ran exactly once and exited 1. It completed the locked offline builds, then
reported: `runner-owned browser process group 22499 or service PID 22211
remained active; temporary state was preserved`. The browser workflow report
was not emitted. Therefore the cancellation assertions, request counts, HTTP
statuses, and final in-run process states were not retained and are not claimed.

Read-only post-run inspection used `ps -g 22499`, `ps -p 22211`, and the same
`ps -axo pid=,pgid=,stat=` listing/filter used by the runner. Neither exact
scope was present at inspection time. The runner did not record a browser
command/profile identity for either remaining scope, so the preserved
temporary root was not removed:
`/var/folders/qd/xlj1mz_x2t92gdst9km986p00000gn/T/hyperai-interface-browser-PJbGjW`.
No process was signaled. Port `127.0.0.1:4317` was not contacted, and no MCP
sidecar or tool was launched.

Preflight passed JavaScript syntax and root `lint:fast`, root
`verify:contracts`, `python3 scripts/verify_imports.py`, the frozen input
hashes, and `git diff --check`. The frozen packet records a 216-file
source-tree digest of
`e1842a829e71d1b5f95fb87da4ef00422e790f974fb46c9214a25a75335089de`. During
evidence closure, an independent recomputation using the packet's stated path,
length, and content procedure also found 216 files but produced
`e3d71d7de94e3b40ca1ba7c3e9f2ab7248a19e5c660d2bf60c7274044ceacbf8`. All 20
individually frozen input hashes match their packet values. The packet did
not retain a per-file manifest for the aggregate digest, so the difference
cannot be resolved as an algorithm discrepancy or source-set change; exact
source-tree equality is unconfirmed. Root `verify:full` was not run; its
earlier `HomeBusy` failure remains unresolved. Assurance is `INCONCLUSIVE`;
reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. The
only retained execution evidence is the command failure, frozen direct-hash
matches, and exact-ID read-only process inspection. Temporary state remains
locally preserved. Claim ceiling: no workflow or accessibility claim for v5.
The next gate is a fresh, separately named diagnostic slice that improves
cleanup-report retention and preserves a per-file digest manifest; v5 itself
is closed and must not be repeated.
