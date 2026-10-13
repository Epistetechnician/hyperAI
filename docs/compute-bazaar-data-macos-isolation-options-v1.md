# macOS Data Worker Isolation Options v1

State slice: [`hyperai.bazaar.data_macos_isolation_options.v1`](state-slices.md#hyperai-bazaar-data_macos_isolation_optionsv1).

Status: documentation-only platform assessment. It recommends a separately
scoped Virtualization.framework feasibility spike; it does not select or
authorize a runtime implementation.

## Question and decision

Phase 2 needs a platform-specific resource boundary before HyperAI can claim
a bounded Data worker on macOS. The current DataFusion memory pool, API
concurrency limit, query timeout, and input/output byte caps serve different
purposes; they do not collectively prove an operating-system-enforced
whole-worker memory ceiling.

Carry one Linux guest under Apple's Virtualization.framework as the leading
macOS feasibility candidate. Use a fixed guest memory size and, if needed,
Linux cgroup v2 `memory.max` for the worker process. This would bound memory
inside the guest, not establish a host-process RSS limit. The guest kernel,
worker, and runtime share the guest budget; the complete host-plus-guest
capacity and failure behavior still need measurement. A fixed guest image,
entitlement and signing path, no-network device configuration, bounded
digest-bound IPC, cancellation, and cleanup must be proven in a separate
authorized spike before this option is selected.

If the entitlement or packaging path is not feasible, keep macOS
bounded-worker support disabled until another enforceable option qualifies.
The current local query path may not claim process isolation or a hard
process-memory bound from its in-process controls.

## Candidate comparison

| Candidate | What current evidence says | Assessment |
|---|---|---|
| DataFusion pool plus bounded inputs, outputs, time, and API concurrency | HyperAI already configures an engine pool and two-slot API admission. These controls do not account for every allocation in the host process. | Retain as query-engine and service controls; do not call them OS isolation. |
| Native `RLIMIT_AS` | The v3 macOS probe accepted a limit 64 MiB above the measured virtual-size baseline and refused one 128 MiB allocation. Raw `ru_maxrss` did not establish an RSS cap or useful worker budget. | Not selected as the worker memory budget. |
| Native `RLIMIT_DATA` | The one authorized probe failed to set 64 MiB with `EINVAL` before any allocation. Current public XNU source documents this error when the new data limit is below current usage, but the helper did not record that baseline. | Inconclusive; no retry under that slice. A later baseline-aware probe would need a fresh slice and still would not prove DataFusion coverage by itself. |
| `os_proc_available_memory()` | Apple documents this as advisory app memory information; it is not a limit setter and returns zero for a caller that is not an app or has exceeded its limit. | Useful only for diagnostics where applicable; not enforcement. |
| Virtualization.framework Linux guest | Apple documents `VZVirtualMachineConfiguration.memorySize` as the physical memory the guest recognizes and requires the `com.apple.security.virtualization` entitlement. Linux can provide cgroup v2 controls inside the guest. | Leading candidate for a bounded macOS worker feasibility spike. It adds VM image, entitlement, boot, IPC, and capacity costs. |

## Proposed feasibility-spike gates

A future implementation packet must freeze the target macOS version,
entitlement and signing model, kernel/rootfs/toolchain and their digests,
guest memory size, worker cgroup limit if used, IPC schema, snapshot handling,
budgets, retention, failure contract, and claim ceiling before any VM starts.

The bounded synthetic spike should then demonstrate:

1. The signed host process has the required entitlement and the host reports
   Virtualization.framework support; absent entitlement or support fails
   closed.
2. The guest has no network device and receives only one synthetic,
   digest-bound read-only snapshot and one bounded query through a versioned
   IPC channel; it cannot read the HyperAI home or credentials.
3. The fixed guest memory and worker cgroup limits are read back and bound to
   the run identity. A controlled over-budget allocation produces a literal
   guest/worker failure without a partial query success.
4. Cancellation, timeout, guest crash, OOM, host shutdown, and cleanup leave
   no live worker, retained private bytes, or ambiguous success receipt.
5. Host service plus guest memory, startup time, query latency, and one-slot
   capacity are measured from exact source and image digests. No throughput
   or user-facing budget is inferred before those measurements.

Claim ceiling for any such future spike must remain the exact synthetic guest
boundary, lifecycle, and resource measurements. It would not establish
hostile-code safety, correct computation, production support, or a host RSS
ceiling. This document authorizes none of those tests.

## Sources and limits

- Apple [Virtualization framework memory size](https://developer.apple.com/documentation/virtualization/vzvirtualmachineconfiguration/memorysize): guest-visible physical memory; it may be backed by host virtual memory reserved without immediate physical allocation.
- Apple [Virtualization entitlement setup](https://developer.apple.com/documentation/virtualization/adding-the-virtualization-entitlement-to-your-project): processes using the framework require `com.apple.security.virtualization`.
- Apple [`os_proc_available_memory`](https://developer.apple.com/documentation/os/os_proc_available_memory): advisory app memory information.
- Apple [public XNU `RLIMIT_DATA` handling](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c): one documented `EINVAL` condition is that the requested data limit cannot be lowered below current usage. The exact shipped kernel and this probe's current usage were not recorded.
- Linux kernel [cgroup v2 memory controller](https://docs.kernel.org/admin-guide/cgroup-v2.html): candidate worker-level control inside a Linux guest.

The references inform design only. No HyperAI code, dependency, entitlement,
guest image, dataset, fixture, or runtime behavior changes under this packet.
