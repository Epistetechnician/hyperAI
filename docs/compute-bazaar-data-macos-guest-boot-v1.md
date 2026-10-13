# HyperAI Phase 2 macOS Guest Boot v1

State slice: [`hyperai.bazaar.data_macos_guest_boot.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_bootv1).

Status: closed `INCONCLUSIVE`. The one helper compilation failed before VM
configuration validation; no VM was instantiated or started. The compile
allowance is consumed. Any further attempt requires a fresh named continuation
slice and packet.

## Baseline and frozen inputs

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree. Preserve every pre-existing change.

Use only these already retained, authenticated files:

| Role | Local path | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| RAW ext4 root disk | `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/resolute-rootfs-arm64.raw.ext4` | 4,294,967,296 | `6342a12d3fb4484d7a340fa647e574754d72aa39d535fb4a15ec61c0072b7fce` |
| Linux arm64 kernel | `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-inputs/resolute-server-cloudimg-arm64-vmlinuz-generic` | 23,831,432 | `0bad41327e67c7c79d79b7e5f0ee833d0202be2ddf0f7cbbb984b4cb9c94e108` |
| Linux arm64 initrd | `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-inputs/resolute-server-cloudimg-arm64-initrd-generic` | 91,123,407 | `65fa960053965fbcbba70a36e060965f2f4eb46124e8f0d9c0984250edc61117` |

Their source authentication and the RAW image integrity inspection are
recorded in the linked image and boot-input packets. Do not download, replace,
modify, copy, host-mount, or convert these inputs under this slice. The disk is a
single ext4 filesystem exposed as the only virtio block device. The proposed
kernel root argument is `/dev/vda`; any root-device or filesystem failure
closes this one attempt as `INCONCLUSIVE`.

## Frozen VM and lifecycle contract

- Host: macOS 26.6.1 (`25G76`), arm64 Apple Silicon, Xcode 27.0
  (`27A266a`), Swift 6.4.0 (`swiftlang-6.4.0.34.1`), and macOS 27.0 SDK.
  Before instantiation, require
  `kern.hv_support=1`, at least 8 GiB physical memory, and at least 25% free
  memory from one `memory_pressure -Q` reading. Stop before VM creation if any
  condition fails.
- Use one standalone Swift helper compiled once against the system
  Virtualization framework and ad-hoc-signed once with only
  `com.apple.security.virtualization = true`. Inspect and strictly verify the
  signature, then call configuration validation once before VM creation.
- Configure exactly one vCPU and 1 GiB guest memory, one generic Linux
  platform, one read-only RAW virtio block attachment, and one one-way serial
  console that writes guest output to a local log. Set no network, shared
  directory, socket, USB, graphics, audio, input, or other device.
- Boot the frozen kernel and initrd with exactly this command line:
  `console=hvc0 root=/dev/vda rootfstype=ext4 ro rootwait systemd.unit=emergency.target systemd.show_status=1`.
  Do not inject guest input or execute workload code. The emergency target
  keeps this attempt at boot-only scope; the disk attachment and root mount
  are both read-only.
- Before the start call, verify the exact byte counts and SHA-256 values in
  the table. After the VM is stopped, hash the RAW disk once more and require
  the original digest. Do not rehash or retry under this slice.
- Allow exactly one VM start, for at most 120 seconds. The only success marker
  is the literal `Welcome to emergency mode!` in the serial log. Stop as soon
  as that marker appears. Stop immediately on framework failure, a kernel
  panic, an output log over 1 MiB, or the 120-second deadline. Call VM stop
  once and allow up to 10 seconds for its completion callback. Do not restart
  the VM or retry any failed operation.
- Maximum serial log size is 1 MiB; record its byte count, SHA-256, and only
  the bounded lines surrounding the marker. Keep the log at
  `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-v1-console.log`.
  Store helper source, entitlement, compiler cache, and binary only under
  `/private/tmp/hyperai-bazaar-vm-boot-v1/`; record their hashes, then remove
  that directory.
- Before and after the attempt, inspect the process list for the helper and
  host capacity once. Require the VM stop callback, no remaining helper, and
  the unchanged RAW digest for a clean local lifecycle result. If stop or
  cleanup cannot be confirmed, record `INCONCLUSIVE` and stop.

## Allowed effects and non-goals

Allowed effects are local hashing and read-only metadata checks of the three
frozen inputs; one helper compile, ad-hoc signature operation, signature
inspection, and framework configuration validation; one VM construction and
start under the exact contract above; one bounded boot console output file;
and edits to this packet, `docs/state-slices.md`, the Phase 2 Data packet, and
the Rust-port subplan.

No network device or guest network, host path sharing, provider, seller,
worker, dataset, user payload, HyperIntel/JustAI connection, payment,
external write, production service, or remote process is allowed. Do not
execute an evaluation workload, install packages, run cloud-init provisioning
against an external source, alter the root image, change product source or
dependencies, or publish/release/redistribute any Ubuntu artifact. Remove
the helper and its build artifacts after evidence capture. Retain only the
previously authorized Ubuntu inputs and the bounded local console log outside
the repository.

Ubuntu package and image rights remain subject to their package licenses and
Canonical's local internal-use boundary from the source packets. No guest
artifact or log enters tracked HyperAI files.

## Assurance target and claim ceiling

Assurance is `INCONCLUSIVE` for this closed slice. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.

A pass establishes one local boot path for these exact bytes and this host.
It does not establish that hostile code is isolated, that cgroups or memory
limits constrain DataFusion, that cancellation or crash/OOM recovery is safe,
that a bounded IPC exists, that service-plus-worker capacity is acceptable,
that the image is generally bootable, or that macOS is a supported product
platform. It does not authorize paid compute, providers, federation, testnet,
mainnet, or production execution. Overall Phase 2 remains `NOT_ASSIGNED`.

## Result record

The single compile command was
`xcrun swiftc -sdk /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk -module-cache-path /private/tmp/hyperai-bazaar-vm-boot-v1/module-cache -framework Virtualization /private/tmp/hyperai-bazaar-vm-boot-v1/main.swift -o /private/tmp/hyperai-bazaar-vm-boot-v1/guest-boot`.
It exited 1 in 1.9 seconds with two compiler diagnostics: Swift required
`-parse-as-library` for the `@main` declaration in `main.swift`; and the SDK
typed `VZVirtioConsoleDeviceSerialPortConfiguration` did not match the
`VZVirtioConsolePortConfiguration` subscript on
`VZVirtioConsoleDeviceConfiguration`. Read-only SDK header inspection showed
that `VZVirtualMachineConfiguration.serialPorts` is the compatible property
for `VZVirtioConsoleDeviceSerialPortConfiguration`.

The failed source SHA-256 was
`1ae6cbbec16214f313fbfa105987352060ec25a5cdc74835dae2bc8f77e0c84a`.
No entitlement was created, no binary was produced, no configuration was
validated, no VM was instantiated or started, and no console log was created.
The temporary source and compiler cache were removed after capturing the
diagnostic and source hash. No guest or product bytes changed. The pre-launch
artifact hashes matched; host preflight was `kern.hv_support=1`, 36 GiB
physical RAM, and 76% free memory. The retained Ubuntu artifact directory was
4,575,676 KiB; no additional boot copy was made.

Assurance: `INCONCLUSIVE`. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody: authenticated Ubuntu bytes remain
in `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`; no helper artifacts
remain. Claim ceiling: host/compiler/API preflight only. No VM or guest-boot
claim follows. The one compile is consumed; a new continuation packet is the
next gate before any further helper build or VM start.

## References

- Apple: [Running Linux in a Virtual Machine](https://developer.apple.com/documentation/virtualization/running-linux-in-a-virtual-machine)
- Apple: [VZLinuxBootLoader](https://developer.apple.com/documentation/virtualization/vzlinuxbootloader)
- Apple: [RAW disk attachment and read-only mode](https://developer.apple.com/documentation/virtualization/vzdiskimagestoragedeviceattachment)
- Apple: [Serial console configuration](https://developer.apple.com/documentation/virtualization/vzvirtioconsoledeviceserialportconfiguration)
- Apple: [VZVirtualMachine start and stop](https://developer.apple.com/documentation/virtualization/vzvirtualmachine)
