# Synthetic ACP Test Isolation Packet

State slice: `hyperai.bazaar.acp.synthetic_peer_test_isolation.v1`.
Parent: [Rust compute-bazaar subplan](compute-bazaar-rust-port-subplan.md)
and [Phase 8 ACP packet](compute-bazaar-phase-8-acp-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Preserve unrelated state.

## Trigger and evidence

Two `corepack pnpm run lint` executions failed at
`synthetic_peer_checks_scoped_mcp_capability_allow_and_deny_routes` with the
same `ACP connection closed; local route result: Ok(Ok(Ok(())))` error. The
test passed when run by itself; all nine tests in its target also passed with
`--test-threads=1`. This points to test concurrency or process pressure as a
possible cause; the exact shared resource has not been established.

Cargo's test harness runs test functions on multiple threads by default. The
synthetic-peer target contains nine tests that launch owned child processes
and local loopback servers. Serialize only this target's test bodies with a
test-local async mutex. Do not change ACP session behavior, test expectations,
root test-thread defaults, or dependencies.

## Alternatives considered

1. **Test-local mutex around this target — selected.** Narrowly removes
   process/loopback fixture concurrency while leaving other workspace tests
   parallel. The serialized target already passed before this change.
2. **Set `--test-threads=1` or `RUST_TEST_THREADS=1` globally.** Cargo documents
   this as the way to disable harness parallelism, but it serializes unrelated
   suites and increases total validation time.
3. **Increase ACP connect timeout or retry `connect`.** The observed failure
   is an early disconnect, not a timeout; retrying a closed child session
   would conceal the failure and duplicate side effects within a test.
4. **Change `AgentSessionManager` or the ACP SDK close path.** The pinned ACP
   2.2.0 docs say `connect_with` stays active until its client closure
   returns. No evidence currently ties the intermittent failure to production
   session semantics, so runtime changes are out of scope.
5. **Add fixture failure-file diagnostics.** This could improve future
   diagnosis but would not remove the observed concurrency sensitivity.

Primary references: [Cargo test execution and test threads](https://doc.rust-lang.org/cargo/commands/cargo-test.html), [Tokio 1.52.3 `Mutex`](https://docs.rs/tokio/1.52.3/tokio/sync/struct.Mutex.html), [ACP SDK 2.2.0 `Builder::connect_with`](https://docs.rs/agent-client-protocol/2.2.0/agent_client_protocol/struct.Builder.html), and [Tokio 1.52.3 `Child`](https://docs.rs/tokio/1.52.3/tokio/process/struct.Child.html).

## Exact files and allowed effects

- `crates/hyperai-acp/tests/synthetic_peer.rs`: hold one target-local async
  mutex for each test body.
- `docs/compute-bazaar-acp-test-isolation-packet.md`: record evidence and
  limits.
- `docs/state-slices.md`: record the named slice, verification, and claim
  ceiling.
- `docs/compute-bazaar-rust-port-subplan.md`: update the root-gate status.

Only locked, offline local synthetic tests and the repository root lint are
allowed. The test peers use disposable directories and IPv4 loopback only.
No provider, model, remote worker, user data, paid execution, live Fleet,
financial Trade, chain, publication, or production system is in scope.

## Proof gates and claim ceiling

- All nine synthetic-peer tests acquire the shared test-only mutex before
  starting their process or loopback fixture.
- The target passes under the normal parallel libtest invocation.
- `corepack pnpm run lint`, pinned formatting, and `git diff --check` pass.
- Preserve explicit evidence that the earlier default-parallel root gate
  failed twice and the serial target passed; do not represent serialization as
  proof of the precise OS-level race.

Maximum claim: the ACP synthetic-peer integration target passes with its
fixtures serialized on this host. This does not establish that the production
ACP runtime caused the earlier failure or qualify external ACP peers, Windows,
remote workers, or production behavior. Reviewer status is
`NOT_REQUESTED`; reproduction remains `NOT_REPRODUCED`.

## Validation report

All nine tests passed under the normal parallel test harness after the
test-local mutex was added. The exact target also passed under root lint:
`corepack pnpm run lint` exited 0 after the test and geometry-strengthened TUI
changes. Focused strict Clippy and pinned Rust formatting passed, as did
`git diff --check`. The test source SHA-256 at this validation is
`a5eef98d64966dcdc71bc6cd06c793b0ef88c49fe79c90d1032cc3a5c3ecf4f8`.

The two earlier root-lint failures, isolated-test pass, and serialized-target
pass remain part of the evidence. The exact OS-level source of the original
connection close is still unknown; this is `INTERNAL_PASS` for the serialized
synthetic-peer target on this host only. Reviewer status is `NOT_REQUESTED`,
reproducibility is `NOT_REPRODUCED`, and production ACP behavior or external
peer compatibility is not claimed. The next blocked gates are a clean
second-environment reproduction and the separately scoped native Windows ACP
lifecycle qualification.

## Failure-marker diagnostic continuation (2026-09-30)

State slice: [`hyperai.bazaar.acp.synthetic_peer_failure_diagnostic.v1`](state-slices.md#hyperai-bazaar-acp-synthetic_peer_failure_diagnosticv1).

Revision: HyperAI baseline `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on
`main` plus the dirty local worktree. Assurance is `INTERNAL_PASS` for the
focused target only; reviewer status is `NOT_REQUESTED`, and reproducibility
is `NOT_REPRODUCED`.

After a later root all-feature run reproduced the default-parallel ACP
disconnect, the test failure path was extended to read at most 512 bytes from
the fixture's existing `mcp-capability-probe.failure` marker. It replaces
control characters and redacts the test's fixed synthetic capability before
including the marker in transient failure context. The test retains no marker
or process output, and no ACP runtime code changed.

The locked/offline `hyperai-acp --test synthetic_peer` target passed all 9
tests under its normal parallel harness after the change. The marker path was
not exercised. The final offline root lint passed formatting, strict Clippy,
contract checks, the all-feature workspace suite, and doc tests; it exited 0.
Two earlier root invocations stopped at formatting and strict Clippy; both
findings were fixed before the final run. Final test
source SHA-256:
`8e1249370a3fc91b6f174aaecc96441dac1d2f7c7cb8ba091f82b37d19386622`.

Custody stayed in the local test's disposable directory and transient output;
the failure marker was not read in the passing run. The claim ceiling is the
focused synthetic-peer target and root lint on this host. This does not
identify the original disconnect's OS-level cause, establish production ACP
behavior, or qualify external peers or Windows. Clean second-environment
reproduction and native Windows lifecycle remain open.

## Loopback request-body drain (2026-09-30)

State slice: [`hyperai.bazaar.acp.synthetic_http_body_drain.v1`](state-slices.md#hyperai-bazaar-acp-synthetic_http_body_drainv1).

The root `pnpm run lint` run reached the workspace all-feature tests and
failed in the existing scoped MCP capability test. The fixture marker reported
`Connection reset by peer (os error 54)`, while both local route checks
completed. The request helper sends a POST with a two-byte `{}` body. The
fixture server reads only until `\r\n\r\n`, then writes its response and drops
the socket without checking whether those body bytes have arrived or been
consumed. This leaves a plausible framing race at the test-owned TCP boundary.

RFC 9293 §3.6 describes that a half-duplex close with received data still
pending may send RST to signal data loss. Tokio's `TcpStream` docs describe
stream shutdown and the EOF indication on the peer. These support draining the
declared body before close; they do not by themselves prove this was the
observed macOS reset's cause. See [RFC 9293 §3.6](https://www.rfc-editor.org/rfc/rfc9293.html)
and [Tokio 1.52.3 `TcpStream`](https://docs.rs/tokio/1.52.3/tokio/net/struct.TcpStream.html).

Alternatives: drain the exact `Content-Length` after headers (selected); use a
full Hyper/Axum fixture server (larger change and direct test dependency);
remove the POST body or change the method (weakens the intended route check);
retry or increase connect timeouts (masks the reset); or serialize all Cargo
tests globally (broader than needed, and test-local serialization already
exists). The new slice permits only the local fixture to consume a bounded,
complete body before preserving its current method/path/bearer checks and
fixed response.

Execution evidence: the fixture now reads exactly the bounded request length
declared by `Content-Length` before checking the method, path, bearer header,
and returning the fixed response. The locked/offline
`cargo test --locked --offline -p hyperai-acp --test synthetic_peer` command
passed 9 tests, 0 failed, and 0 ignored under normal parallel test execution.
The final offline root `pnpm run lint` passed interface checks, installer
checks, pinned formatting, strict Clippy, workspace contract tests, the
all-feature workspace suite, and doc tests; environment-gated cases remained
ignored. Its included Data API target passed 7/7. `git diff --check` passed.
The resulting test source SHA-256 is
`9e8b7d121ae332322e8a8837c7df77c2aaaae5ffdda9320b2431848a4ea550b2`.

The root failure is not counted as a failure of the Data API probe; that
probe's separate state slice remains bounded to the local Data query. The
passing result supports the selected fixture correction, but it does not
prove this was the cause of every earlier reset or alter ACP runtime
behavior. Assurance is `INTERNAL_PASS` for the local test target and root
commands only; reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Clean second-environment reproduction and native Windows
ACP lifecycle qualification remain open.
