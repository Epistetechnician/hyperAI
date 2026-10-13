# HyperAI Data Docker cgroup Probe v1

Status: `INTERNAL_PASS` for the exact Docker Desktop Linux/aarch64 cgroup v2
probe below. It advances one possible Data worker boundary; it does not select
a product runtime or authorize product code.

State slice: `hyperai.bazaar.data_docker_cgroup_probe.v1`.
Parent: [Phase 2 Data packet](compute-bazaar-phase-2-data-packet.md),
[Rust compute-desk subplan](compute-bazaar-rust-port-subplan.md), and
[macOS isolation options](compute-bazaar-data-macos-isolation-options-v1.md).

## Baseline and question

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked working tree. Preserve all existing changes.
The local Docker context is `desktop-linux`; the server reports Linux/aarch64,
cgroup v2, 12,525,842,432 bytes of memory, and `cgroupfs`. The locally cached
`rust:1.93.1-bookworm` image has image ID
`sha256:78a50754b62786434bc4ebfd874ef382348701b5f01b2c0d21b8e17cfad97969`
and repo digest
`rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`.
No container with the probe name prefix was present at preflight.

Question: does this exact local Docker Desktop Linux environment expose a
128 MiB cgroup v2 `memory.max` and contain a synthetic child allocation while
the parent process survives and can read the cgroup OOM counters?

## Frozen helper

The following dependency-free Rust program is the complete test payload. The
temporary source file must match the bytes in this code block; record its
SHA-256 before compiling. It reads cgroup controls, then starts a child that
touches up to 512 MiB in retained 8 MiB blocks. No project dataset or source
artifact is used.

```rust
use std::env;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const BLOCK_BYTES: usize = 8 * 1024 * 1024;
const BLOCK_COUNT: usize = 64;

fn read_control(name: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(format!("/sys/fs/cgroup/{name}"))?.trim().to_owned())
}

fn allocate() {
    let mut blocks = Vec::with_capacity(BLOCK_COUNT);
    for index in 0..BLOCK_COUNT {
        let block = vec![index as u8; BLOCK_BYTES];
        black_box(block.as_ptr());
        blocks.push(block);
    }
    println!("allocation_complete=true blocks={}", blocks.len());
}

fn run() -> Result<(), Box<dyn Error>> {
    if env::args().nth(1).as_deref() == Some("allocate") {
        allocate();
        return Ok(());
    }

    let memory_max = read_control("memory.max")?;
    let swap_max = read_control("memory.swap.max")?;
    let events_before = read_control("memory.events")?;
    println!("memory.max={memory_max}");
    println!("memory.swap.max={swap_max}");
    println!("events.before={events_before:?}");

    let status = Command::new(env::current_exe()?)
        .arg("allocate")
        .status()?;
    println!("child.exit_code={:?}", status.code());
    println!("child.signal={:?}", status.signal());
    println!("events.after={:?}", read_control("memory.events")?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("probe_error={error}");
        std::process::exit(2);
    }
}
```

Pre-run helper freeze: temporary `probe.rs` SHA-256
`2ca2eba84ae5d57341892abb8ccbdd6884b8d477c7c4fb0832f4529acc0ac1d4`.
The helper is HyperAI-authored probe code, not imported product or Bazaar
source. The image is `linux/amd64` while Docker Desktop's Linux VM is
`aarch64`; compile and execution therefore use explicit amd64 emulation. The
binary SHA-256 is
`3ef79a5739b14dae2239a38748f2ae069c1b0b9a9beb04b62b2bba4bd6d194c2`.

## Exact procedure and effects

Writable repository files: this packet and `docs/state-slices.md` before the
run; this packet, `docs/state-slices.md`,
`docs/compute-bazaar-phase-2-data-packet.md`, and
`docs/compute-bazaar-rust-port-subplan.md` when recording the result. No Rust
workspace, service, interface, dependency, lockfile, product snapshot, source
manifest, or ignored Bazaar file is writable.

Allowed effects: create one mode-0700 disposable temporary directory holding
the frozen helper source and compiled binary; use the cached image only with
`--pull=never`; run one compile container and one resource-capped probe
container, both with `--network=none`; and remove both containers and that
exact temporary directory. The probe container must have `--memory=128m`,
`--memory-swap=128m`, `--pids-limit=16`, a read-only root filesystem, a
read-only mount of only the probe binary, `--cap-drop=ALL`,
`--security-opt=no-new-privileges`, and user `65534:65534`. It receives no
HyperAI home, credentials, dataset, source tree, socket, or writable bind
mount. Do not change Docker daemon settings or pull an image.

Proof gates:

1. Reconfirm the exact cached image ID/digest, Docker context, cgroup version,
   and absence of a conflicting container name.
2. Freeze the temporary helper source hash and retain the compile command and
   output binary hash in this packet.
3. Confirm the run has no network, uses only the exact read-only binary mount,
   and reads `memory.max=134217728` and `memory.swap.max=0` from cgroup v2.
4. Confirm the child does not report successful completion, the parent remains
   alive, and `memory.events` shows an OOM kill increment attributable to the
   child. If any condition is missing or contradictory, record `INCONCLUSIVE`.
5. Confirm both containers are removed, the exact temporary directory is
   removed, and no project file changed.

## Claim ceiling and report

Even a passing result proves only that the exact probe child was contained by
the exact Docker Desktop Linux/aarch64 container's cgroup v2 limit. It does
not establish native Linux host behavior, native macOS process isolation,
Windows Job Object behavior, a HyperAI worker contract, snapshot/IPC safety,
cancellation or crash recovery, combined service-plus-worker capacity, or
overall Phase 2 acceptance. Docker is not added as a product dependency by
this packet. Bazaar rights and all live data, provider, Fleet, evaluation,
settlement, publication, and production gates remain closed.

Assurance: `INTERNAL_PASS` for the exact constrained container probe only.
Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Data
custody: the helper source, binary, and cgroup counters existed only in the
disposable local probe environment. Publish boundary: local evidence only; no
commit, PR, release, or deployment.

## Execution record

The source hash was
`2ca2eba84ae5d57341892abb8ccbdd6884b8d477c7c4fb0832f4529acc0ac1d4`; the
compiled binary hash was
`3ef79a5739b14dae2239a38748f2ae069c1b0b9a9beb04b62b2bba4bd6d194c2`. The
cached image identity matched the preflight. Docker Desktop reported an
`aarch64` Linux VM; the cached image and helper were `linux/amd64`, executed
under its emulation. The offline compile container used `--network=none`, a
read-only root, a 64 MiB temporary filesystem, and read-only source plus
writable output mounts. The compile produced the hashed ELF binary.

The run container used `--pull=never --platform=linux/amd64 --network=none
--memory=128m --memory-swap=128m --pids-limit=16 --read-only`, a single
read-only binary mount, user `65534:65534`, all capabilities dropped, and
`no-new-privileges`. It exited 0 after the parent observed:

```text
memory.max=134217728
memory.swap.max=0
events.before: max 0, oom 0, oom_kill 0
child.exit_code=None
child.signal=Some(9)
events.after: max 39, oom 1, oom_kill 1, oom_group_kill 0
```

The synthetic child was killed by signal 9 while the parent survived and read
the incremented cgroup counters. Both containers used automatic removal. A
post-run container listing had no matching probe container; the exact
temporary directory was removed. The helper and binary hashes were checked
before deletion, and no HyperAI product source was changed.

This is `INTERNAL_PASS` only for Docker Desktop's Linux/aarch64 cgroup v2
container boundary with an emulated amd64 process. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. It does not prove a
native macOS limit, native Linux host behavior, Windows enforcement, a
HyperAI worker contract, or overall Phase 2 acceptance. The next gate is a
separate product/platform architecture packet comparing this optional Docker
candidate with the unresolved macOS Virtualization path; Phase 2 remains
`NOT_ASSIGNED`.
