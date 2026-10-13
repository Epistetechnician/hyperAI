# Data Worker Platform Decision v1

State slice: hyperai.bazaar.data_worker_platform_decision.v1.
Parent: compute-bazaar-phase-2-data-packet.md.
IPC boundary: compute-bazaar-data-worker-ipc-contract-v1.md.
Implementation sequence: compute-bazaar-rust-port-subplan.md.

Status: planning only. This record selects a staged qualification order for a
future supervised query worker. It changes no runtime, product support claim,
API, or dependency.

Baseline: HyperAI dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa on main, with the
preserved dirty and untracked worktree. The NUL-delimited Git status digest at
freeze was
8199950202cc81576d8788955f1f7241f6c95096cc176d821b4ed62dad560ff6.

## Frozen inputs

| File | Bytes | SHA-256 |
|---|---:|---|
| docs/compute-bazaar-phase-2-data-packet.md | 173,326 | 36fcd6ed367ff1c378adaec704294f6604625877c5b2e7d2ef55ec3f227568e0 |
| docs/compute-bazaar-data-worker-ipc-contract-v1.md | 12,796 | 0b9bb5bfccea364042e4035ca62ca5d1c7f1ede704e222116e72b1acd7558903 |
| docs/compute-bazaar-data-docker-cgroup-probe-v1.md | 8,439 | ee4b44f761ac154dafd8bccf82a8349ea226ed20f93af801c768029deb533cce |
| docs/compute-bazaar-data-macos-resource-limit-probe-v3.md | 6,487 | fa29c5b286a46bca4f8434daabe3d46ea7141504882dca6fda22b1b60ad98695 |
| docs/compute-bazaar-data-macos-isolation-options-v1.md | 6,228 | 2f5e175d902b0668dca5b4800f6a3d87f4a290875b4c343a583fc7c40b0158ac |
| docs/compute-bazaar-data-macos-guest-start-instrumented-v1.md | 11,795 | 4b6d64c970ad266769358034bca01e39427b88b4e7ee6cb4b0b6611e9cd86499 |
| docs/compute-bazaar-rust-port-subplan.md | 217,250 | 549c4c594db0a5c452ddf63c9b184b2e17040a79695b5bda20bfeece02311111 |
| docs/state-slices.md | 1,099,267 | 2c50ade47e1fe099416902c1d8a90fea5de3fddd76c2e722f6fefbd2ce84b94a |

The source packets are local design and probe records. No Bazaar code or data
is used. The exact platform references reviewed are the Linux kernel cgroup
v2 memory controller documentation, Microsoft Job Objects and
JOB_OBJECT_LIMIT_JOB_MEMORY documentation, Apple's os_proc_available_memory
documentation, and Apple's XNU resource-limit header:

- https://docs.kernel.org/admin-guide/cgroup-v2.html#memory-interface
- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information
- https://developer.apple.com/documentation/os/os_proc_available_memory
- https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/resource.h

## Decision

Keep the existing local Data API and query behavior unchanged. Do not enable
worker mode on any platform until the exact worker, transport, OS boundary,
cleanup, and capacity gates pass on that platform. The existing DataFusion
pool remains an engine query budget; it is not a whole-process memory claim.

For a future worker implementation, qualify native Linux cgroup v2 first.
Require an available delegated cgroup v2 subtree, a per-attempt cgroup,
memory.max, memory.swap.max, process-tree ownership, and verified cleanup.
Fail worker startup closed if those controls cannot be installed. Do not add
Docker as a product dependency. The Docker Desktop probe is evidence for its
exact emulated Linux VM container only, not native Linux host support.

Treat Windows Job Objects as the next platform candidate. A Windows adapter
must assign the suspended worker to its job before resuming it, apply the
job-wide committed-memory limit, own the whole child tree, terminate on
timeout or cancellation, and verify cleanup. This boundary limits committed
memory, not resident set size. Microsoft documentation establishes the API
shape, not HyperAI behavior or platform qualification.

Keep bounded worker mode unavailable on macOS. The local RLIMIT_AS probe
started from roughly 415 GiB of virtual address space and refused one
128 MiB allocation when its limit was only 64 MiB above that baseline. That
does not establish an RSS or useful DataFusion process-memory limit.
Apple documents os_proc_available_memory as advisory and returns zero when
the caller is not an app. The recorded Virtualization.framework start was
INCONCLUSIVE; this packet authorizes no VM retry. A different macOS mechanism
requires current source research, a fresh exact packet, and any required
explicit authorization before execution.

This staged decision leaves existing local query behavior available under
its current claim ceiling. It does not silently fall back from a future
worker error to in-process query execution. When worker mode is configured,
an unsupported or unenforceable platform must fail closed through the
existing stable service error mapping; do not add a public error code in
this planning slice.

## Platform disposition

| Platform | Worker disposition | Evidence and next gate |
|---|---|---|
| Linux | First implementation candidate; not yet qualified. | Docker Desktop Linux/aarch64 VM probe observed cgroup v2 enforcement in its exact container. Implement and qualify against a native Linux host with delegated cgroup v2 controllers. |
| Windows | Follow-on candidate; not yet qualified. | Job Objects provide process grouping and a job-wide committed-memory limit. Implement suspended assignment, whole-tree termination, and native Windows lifecycle and limit tests. |
| macOS | Worker unavailable. | RLIMIT_AS did not establish a useful resident-memory budget; available-memory reporting is advisory; the recorded guest start is inconclusive. Keep the current in-process path and its narrower claim ceiling until a new mechanism is approved and qualified. |

No row in this matrix is a production support claim. The first implementation
packet must choose the exact worker binary, IPC transport, OS adapter,
resource values, supported targets, spawn protocol, no-network/no-home
controls, failure mapping, cleanup, and total service-plus-worker capacity.
It must retain unsupported-platform behavior as fail closed and preserve the
current service as the only query authority.

## Required implementation gates

1. Freeze one platform and exact worker/service identities. Do not bundle
   Linux, Windows, and macOS qualification into one claim.
2. Prove that a query receives one immutable digest-bound snapshot, that the
   worker has no credentials, network, or general HyperAI home access, and
   that only the authenticated service can admit work.
3. Prove the OS limit with an over-budget child, collect platform-native
   evidence, terminate the whole process tree, and remove temporary cgroups
   or jobs on success, cancellation, timeout, crash, and parent shutdown.
4. Measure combined service and worker capacity under two admitted queries.
   Keep the existing two-second end-to-end deadline and no-partial-result
   contract.
5. Exercise the same authenticated service route and stable errors through
   browser, CLI, TUI, and ACP. No client may spawn or address the worker.
6. Keep local synthetic inputs only. Provider calls, external data,
   federation, paid execution, Fleet operations, settlement, public release,
   and production deployment remain separately gated.

## Assurance and publish boundary

This is PLANNING_ONLY. Assurance for runtime is not assigned; reviewer status
is NOT_REQUESTED; reproducibility is NOT_APPLICABLE. Custody is limited to
the local documentation and named probe records. No product source, code,
test, binary, VM, dataset, provider endpoint, or imported workspace was
accessed or changed. Network research was limited to the official
documentation pages listed above.

Claim ceiling: platform qualification order and fail-closed worker policy
only. This record does not prove Linux or Windows worker enforcement, select
a macOS mechanism, establish Phase 2 acceptance, or authorize independent
review or production use. The next gate is a separately scoped native Linux
worker implementation packet; native Linux execution and delegated cgroup
availability must be secured for qualification. Publish boundary: local
documentation only; no commit, PR, release, deployment, API exposure, or
external effect.
