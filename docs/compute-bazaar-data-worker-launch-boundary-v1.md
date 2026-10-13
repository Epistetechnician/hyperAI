# Linux Data Worker Launch Boundary v1

State slice: `hyperai.bazaar.data_worker_launch_boundary_design.v1`.
Parent: [Phase 2 Data packet](compute-bazaar-phase-2-data-packet.md).
Worker protocol: [Data worker IPC contract](compute-bazaar-data-worker-ipc-contract-v1.md).
Platform order: [Data worker platform decision](compute-bazaar-data-worker-platform-decision-v1.md).
Implementation sequence: [Rust port subplan](compute-bazaar-rust-port-subplan.md).

Status: completed design-only launch-boundary assessment. It makes no runtime
change and does not authorize a worker, privileged helper, system service,
dependency, platform support claim, or external effect. Overall Phase 2
remains `NOT_ASSIGNED`.

## Frozen baseline

HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with the existing
dirty and untracked work preserved. The NUL-delimited Git status was 6,600
bytes with SHA-256
`a9e7a6af9e84f6b23332a63ec02df9e80195dfa3e3b2267dc688180beba30490`.
The named local inputs were read at these exact bytes:

| File | Bytes | SHA-256 |
|---|---:|---|
| `crates/hyperai-data/src/lib.rs` | 178,647 | `dd67c0073d00f448d4843bcce7e3406efe7a0dff7250f158bf61bb26cdb2a1f6` |
| `crates/hyperai-application/src/lib.rs` | 107,329 | `627e75c65d8a16db8be3bbc9b6df6d7e0e532f11d04682cfd2984288b53e7af0` |
| `crates/hyperai-local-service/src/server.rs` | 83,331 | `f45d9caac28e4ab5d5bfbb49d5277286811d37c367f0ec3c490c601aca67cfbe` |
| `crates/hyperai-data/Cargo.toml` | 546 | `0670ca8d428e3d8d827d9444d1de8d5e8cc86dfb4dcb306efa76ea1644143a14` |
| `crates/hyperai-local-service/Cargo.toml` | 1,724 | `b5f6831cf1b75c19aa035e7f785a81519ebcf33092aea4a04c72cf3fb60d02af` |
| `docs/compute-bazaar-phase-1-contract-packet.md` | 84,463 | `1e1fb6ba8479d6c462fffd3e57a120b48673dcb3dd1caaccc6e6b5fe7d370546` |
| `docs/compute-bazaar-phase-2-data-packet.md` | 173,715 | `b480690ec709f5c1228cec901e3fea3eb03ee56c0e3fffc9c8dd9fe78fccd05d` |
| `docs/compute-bazaar-data-worker-ipc-contract-v1.md` | 12,796 | `0b9bb5bfccea364042e4035ca62ca5d1c7f1ede704e222116e72b1acd7558903` |
| `docs/compute-bazaar-data-worker-platform-decision-v1.md` | 8,090 | `fcd7389f047127b3c9b25c54147d8a999956df2e1fe43988709cfa9a59a6ebc3` |
| `docs/compute-bazaar-rust-port-subplan.md` | 219,466 | `ed740d585c638e355caa172705c3abdf60bd9f6e0da884befc51718811dd4594` |
| `docs/state-slices.md` | 1,113,494 | `3f0629ed45c21ff3680df09f10de9c1e11d46fa0742143b7b471223a13907c07` |

Writable files are this packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-2-data-packet.md`, and
`docs/compute-bazaar-rust-port-subplan.md`. The named Rust source/manifest
files and existing worker design packets are read-only inputs. The
reference-only Bazaar checkout, imported products, datasets, and assets are
outside this slice.

## Current execution path

The Phase 1 contract keeps `hyperai-ui` as a foreground compatibility launcher
for one authenticated loopback service and explicitly adds no OS auto-start
integration. The current service route acquires one of two Data admission
slots, registers cancellation, and calls `query_data_v1`; that facade calls
`hyperai_data::query_with_cancellation` in the service process. The Data crate
validates and plans the query, captures the verified snapshot, then executes
DataFusion in-process. No `hyperai-data-worker` binary or worker crate exists
in the current workspace.

The planning IPC contract already bounds a one-query, one-snapshot request and
keeps the public service/API, clients, schemas, errors, and query authority
unchanged. The platform decision selects Linux cgroup v2 as the first
qualification candidate, Windows Job Objects as a follow-on, and keeps
bounded-worker mode unavailable on macOS. Its Docker Desktop observation is
limited to that Linux VM container; it is not native Linux host evidence.

## Launch-boundary findings

1. A cgroup is a resource boundary, not the whole worker sandbox. Linux
   `memory.max` applies a hard memory limit to a non-root cgroup, but the
   kernel documentation says usage can temporarily exceed the limit and an
   unreclaimable limit can trigger the cgroup OOM killer. The implementation
   must observe the cgroup outcome and must not infer an exact memory cause
   from a generic worker exit.
2. A service-launched user process cannot assume it may create a cgroup or
   write the memory controller. systemd's current delegation documentation
   says `Delegate=` applies to service/scope subtrees, but an unprivileged
   unit has no access to controllers. The normal foreground HyperAI CLI
   launch has no such delegated-control contract. A user unit with
   `Delegate=yes` is therefore not enough evidence for `memory.max` writes.
3. Filesystem and network confinement need separate controls. Landlock can
   restrict a process's ambient filesystem rights, and newer ABIs add
   selected network restrictions; support and rights differ by kernel ABI,
   and inherited file descriptors and special filesystems need explicit
   treatment. Seccomp filters reduce allowed system calls but the kernel
   documentation explicitly says syscall filtering is not a sandbox and
   requires architecture validation.
4. The worker must receive only its private bounded IPC streams, the exact
   verified snapshot, and the validated query. It must not inherit service
   bearer/session credentials, home paths, open home/cgroup descriptors,
   network sockets, or unrelated environment. The parent must own timeout,
   cancellation, process-tree termination, complete-result validation,
   cgroup-empty verification, cleanup, and the two-slot lifecycle.

Primary references reviewed for this assessment:

- [Linux kernel cgroup v2 memory interface](https://docs.kernel.org/admin-guide/cgroup-v2.html#memory-interface)
- [systemd control-group interface](https://systemd.io/CONTROL_GROUP_INTERFACE/)
- [systemd cgroup delegation](https://systemd.io/CGROUP_DELEGATION/)
- [Linux kernel Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html)
- [Linux kernel seccomp filter documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html)

## Decision and implementation gate

Do not add a self-managed cgroup adapter to the current foreground service,
and do not label a worker bounded, isolated, or supported on Linux. No launch
authority model is selected here. Keep the current in-process Data path and
its existing claim ceiling unchanged; keep the future bounded-worker mode
unavailable until the service has an explicit, verified way to acquire the
memory controller and the child confinement is qualified on native Linux.

The next implementation packet must choose and freeze one of these launch
models before Rust changes:

| Candidate | Required authority and proof | Current disposition |
|---|---|---|
| Host-managed delegated cgroup | An operator-managed service context supplies a delegated subtree with the memory controller actually writable to the HyperAI supervisor. Prove the service's cgroup identity, controller availability, worker placement before request handling, descendant containment, `memory.max`/swap behavior, and cleanup on native Linux. | Possible only with a concrete host launch contract; ordinary user-launched `hyperai` has no such contract. |
| System-manager-owned worker unit | A system manager creates each worker scope/service and owns memory and process limits. Freeze D-Bus or helper authorization, exact unit properties, stdio/IPC transfer, start/cancel/timeout semantics, cleanup, installer/uninstaller behavior, and privilege boundary. | Not selected; adds an administrative/system-service integration absent from Phase 1. |
| Existing in-process path | Keep the current service and DataFusion execution model and report only its existing bounded-query contract. | Remains the current behavior; it does not close worker-isolation qualification. |

For any selected worker path, a separate sandbox packet must define and
qualify the filesystem policy, network-denial mechanism, inherited-FD and
environment allowlists, syscall architecture handling, process-tree control,
OOM/timeout/cancel reporting, and combined service-plus-two-worker capacity.
The configured DataFusion pool is not a process-memory guarantee. Resource
values must be frozen from measured worker behavior on the selected native
platform, not copied from the Docker probe or inferred from the pool size.

An absent controller, unsupported kernel confinement feature, failed sandbox
setup, or ambiguous worker exit must fail closed when worker mode is selected.
There is no silent fallback to in-process execution after an isolated-worker
request has begun. Existing local in-process queries remain unchanged while
the worker mode is unavailable.

## Scope and assurance

This slice authorizes only the named documentation changes and read-only
review of the frozen local inputs and official references. No test, build,
dependency change, service launch, cgroup write, VM start, installer change,
imported source edit, dataset read, provider call, external write, or
production action was performed or authorized. No Bazaar code or data was
copied.

Claim ceiling: `PLANNING_ONLY`. Runtime assurance is not assigned; reviewer
status is `NOT_REQUESTED`; reproduction is `NOT_APPLICABLE`. Custody is this
packet's local documentation inputs and linked public platform documentation.
The next blocked authority gate is selection of an acceptable Linux service
launch model with controller authority, followed by a native Linux host with
delegated cgroup v2 for exact worker qualification. Phase 2 remains
`NOT_ASSIGNED`.

Publish boundary: local documentation only; no staging, commit, PR, release,
deployment, API exposure, or external effect.
