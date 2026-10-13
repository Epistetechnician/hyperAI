# HyperAI Data Worker: Linux Launch Model Decision v1

Status: documentation-only architecture decision under
`hyperai.bazaar.data_worker_linux_launch_model.v1`. It selects a launch model
for the Linux qualification candidate; it does not authorize worker code,
process launch, cgroup mutation, or platform support.

Parent: [Phase 2 Data contract](compute-bazaar-phase-2-data-packet.md).
Protocol: [Data worker IPC contract](compute-bazaar-data-worker-ipc-contract-v1.md).
Prior assessment: [launch-boundary assessment](compute-bazaar-data-worker-launch-boundary-v1.md).
Platform order: [worker platform decision](compute-bazaar-data-worker-platform-decision-v1.md).

## Decision

Use an **operator-managed delegated cgroup v2 subtree** as the first Linux
worker launch model. HyperAI's local service remains the only query authority
and may manage per-query child cgroups only beneath an explicitly supplied,
already delegated subtree. HyperAI does not create the delegation, enable
controllers in an ancestor, change systemd or host configuration, install a
privileged helper, or manage the root cgroup.

The system-manager-owned worker-unit alternative is deferred. It would add a
system service or helper, an administrative authorization path, installer and
uninstaller effects, and a separate process-control protocol. Those effects
are outside the first local Data worker slice.

If no cgroup v2 subtree is supplied, any required controller is unavailable,
or required control files cannot be read and written within that subtree,
worker mode is unavailable. The service must report a stable existing error
and must not silently retry the query in-process after worker mode was
selected. The current in-process Data path remains the default and its
existing claim ceiling is unchanged.

## Required launch sequence

1. The service authenticates the caller, authorizes the request, validates the
   SQL and manifest, reads and verifies the immutable snapshot, and enforces
   the existing input and output limits. Those responsibilities stay in the
   parent service.
2. The parent creates a unique leaf cgroup below the supplied delegation and
   applies the implementation packet's measured memory, swap, and process
   limits before admitting query bytes. It must verify the available
   memory and PID controllers and the leaf's `memory.max`, `memory.swap.max`,
   `pids.max`, `cgroup.procs`, `cgroup.events`, and `cgroup.kill` files, as
   well as the effective values. Missing controls fail closed. It must not
   write outside the supplied subtree.
3. The parent starts the pinned worker binary with a cleared environment and
   bounded stdin, stdout, and stderr. Before reading protocol data, the
   single-threaded worker bootstrap applies `no_new_privs`, Landlock, and the
   seccomp policy. It then emits a bounded ready frame and waits. It must not
   initialize Tokio, DataFusion, or accept a query before the parent confirms
   cgroup placement. If confinement setup fails, it exits and the parent
   removes the empty leaf cgroup.
4. The parent moves the waiting worker into the leaf cgroup, verifies the
   process membership, then sends the versioned IPC request and exact snapshot
   bytes. The request carries no dataset path, credential, general home path,
   provider endpoint, or caller-controlled executable.
5. The parent owns the existing two-second deadline, cancellation, bounded
   output parsing, typed error mapping, process-tree termination, and cleanup
   of the per-query cgroup. Timeout, cancellation, crash, malformed output,
   OOM, and cleanup failure produce no partial query result.

Linux cgroup v2 documents `memory.max` as its primary memory limit and
`cgroup.kill` as a way to kill a cgroup and its descendants; the kernel
documentation notes that memory use may temporarily exceed `memory.max` under
certain circumstances. These controls establish neither filesystem nor network
confinement. Landlock and seccomp must be evaluated as separate controls;
seccomp filtering alone is not a sandbox. The implementation packet must name
the required kernel ABIs and syscall policy, test them against the exact worker
build, and fail closed if they are unavailable. See the
[kernel cgroup v2 documentation](https://docs.kernel.org/admin-guide/cgroup-v2.html),
[Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html),
and [seccomp filter documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html).

## Platform and capacity boundary

- First candidate: native Ubuntu x86_64 with cgroup v2 and an operator-provided
  delegation. This is a qualification target, not current support.
- macOS: keep the existing in-process query path; bounded worker mode remains
  unavailable.
- Windows: keep the existing behavior; Job Objects remain a separate future
  platform slice.
- Do not freeze a numeric cgroup memory value from the macOS RSS baseline or
  the DataFusion pool setting. A native Linux implementation packet must
  measure worker startup and query peaks, set the exact per-worker cap, and
  measure service-plus-two-worker capacity before code is authorized.
- Docker Desktop observations are not native Linux qualification. The current
  host is Darwin arm64 and does not provide the required native Linux evidence.

## State, data, and compatibility

The worker receives one already-verified immutable snapshot through the
versioned IPC contract. It creates no persistent records and uses no external
source. The parent preserves the current `/api/v1` routes, request and response
schemas, SQL policy, stable API errors, CLI/TUI/browser/ACP behavior, and
two-query admission contract. No client launches or addresses the worker.

Only HyperAI-authored synthetic data is in scope. Bazaar code, schemas,
datasets, fixtures, generated output, assets, and dependencies remain excluded
while exact source rights are unresolved. SCM captures and HyperIntel records
remain caller-selected offline inputs and do not feed query dispatch or deal
approval.

## Next gate

Before implementation, create a separate native-Linux implementation packet
that freezes exact source files, worker identity, IPC framing, configuration
surface, kernel/architecture requirements, confinement rules, numeric resource
budgets, cgroup lifecycle, API error mapping, capacity, test fixtures, and
cleanup proof. It must include a native Linux host with delegated cgroup v2;
the current Mac host cannot close that gate. No testnet, provider, seller,
federation, paid execution, live Fleet, financial Trade, external publication,
or production activity is part of this decision.

Claim ceiling: `PLANNING_ONLY`. Assurance for runtime is not assigned;
reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_APPLICABLE`.
Custody is local documentation only. No code, dependency, build, test,
service, cgroup, VM, install, API, host configuration, or external effect was
changed by this slice. Publish boundary: no commit, PR, release, deployment,
or production support claim.
