# HyperAI Phase 2 macOS Guest Boot Continuation v1

State slice: [`hyperai.bazaar.data_macos_guest_boot_continuation.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_boot_continuationv1).

Status: closed `INCONCLUSIVE`. The corrected helper compiled, signed, and
passed configuration validation, but its single VM start call returned
`VZErrorDomain` code `1` before guest output. No guest boot marker was reached.
This packet's one start allowance is consumed; no further start is authorized
under this slice.

## Baseline and frozen inputs

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree. The prior packet's exact inputs remain in
local temporary custody:

| Role | Local path | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| RAW ext4 root disk | `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/resolute-rootfs-arm64.raw.ext4` | 4,294,967,296 | `6342a12d3fb4484d7a340fa647e574754d72aa39d535fb4a15ec61c0072b7fce` |
| Linux arm64 kernel | `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-inputs/resolute-server-cloudimg-arm64-vmlinuz-generic` | 23,831,432 | `0bad41327e67c7c79d79b7e5f0ee833d0202be2ddf0f7cbbb984b4cb9c94e108` |
| Linux arm64 initrd | `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-inputs/resolute-server-cloudimg-arm64-initrd-generic` | 91,123,407 | `65fa960053965fbcbba70a36e060965f2f4eb46124e8f0d9c0984250edc61117` |

Verify each size and digest once before VM instantiation. Do not download,
replace, copy, host-mount, convert, or modify any input. The RAW filesystem
is the only virtio block device; kernel root is `/dev/vda`.

## Corrected helper and VM contract

- Host is macOS 26.6.1 (`25G76`), arm64, Xcode 27.0 (`27A266a`), Swift 6.4.0
  (`swiftlang-6.4.0.34.1`), SDK 27.0. Before VM instantiation require
  `kern.hv_support=1`, at least 8 GiB physical memory, and at least 25% free
  memory in one `memory_pressure -Q` reading.
- Freeze the corrected helper source digest in this packet before compiling.
  Compile once with `-parse-as-library`; configure the one
  `VZVirtioConsoleDeviceSerialPortConfiguration` through
  `VZVirtualMachineConfiguration.serialPorts`. These changes address the two
  compiler diagnostics from the prior closed packet. Do not change other
  helper behavior or retry compilation.
- Frozen source: `/private/tmp/hyperai-bazaar-vm-boot-continuation-v1/main.swift`,
  4,320 bytes, SHA-256
  `70ce9a9a8a2e40b71d8f09880650c7f180445771565abcb1d7018b68308b61ac`.
- Exact compile command:
  `xcrun swiftc -parse-as-library -sdk /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk -module-cache-path /private/tmp/hyperai-bazaar-vm-boot-continuation-v1/module-cache -framework Virtualization /private/tmp/hyperai-bazaar-vm-boot-continuation-v1/main.swift -o /private/tmp/hyperai-bazaar-vm-boot-continuation-v1/guest-boot`.
- Ad-hoc-sign once with only
  `com.apple.security.virtualization = true`; inspect the embedded entitlement
  and verify the signature strictly. Require a single successful
  configuration validation before instantiating the VM.
- Configure exactly one vCPU, 1 GiB memory, generic Linux platform, one
  read-only RAW virtio block device, and one output-only serial port writing
  to `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-v1-console.log`.
  Keep network, directory sharing, sockets, USB, graphics, audio, keyboard,
  pointer, and every other device absent.
- Use the frozen kernel and initrd with command line:
  `console=hvc0 root=/dev/vda rootfstype=ext4 ro rootwait systemd.unit=emergency.target systemd.show_status=1`.
  Send no input to the guest and run no workload.
- Allow at most one VM start and 120 seconds from the start request. Stop at
  the literal `Welcome to emergency mode!` marker. Stop on framework failure,
  `Kernel panic`, a serial log over 1 MiB, or the deadline. Call VM stop once;
  allow 10 seconds for completion. A Python supervisor may bound the helper
  process to 150 seconds total; supervisor timeout means cleanup is
  `INCONCLUSIVE`, followed by a process check and post-run root-disk hash.
- Keep the serial log under 1 MiB and record its hash, size, and only the
  bounded marker context. Record helper source, entitlement, and binary hashes.
  Remove the helper source, binary, compiler cache, and entitlement file after
  evidence capture. No duplicate disk image is permitted.
- Hash the root disk once after stop and require the frozen digest. Confirm
  the helper exited, capture stop result/timing, and measure host capacity once
  after the run. Stop if any identity or cleanup gate fails.

## Allowed effects and non-goals

Allowed effects: three frozen-input pre-run hashes; one corrected helper source
creation and digest freeze; one compile, one ad-hoc signature operation,
signature inspection, one configuration validation, one VM construction and
at most one VM start/stop; one bounded local console log; one post-run root
disk hash; and edits only to this packet, `docs/state-slices.md`, the Phase 2
Data packet, and the Rust-port subplan.

No guest network, host path sharing, provider, seller, worker, dataset,
external source, user payload, HyperIntel/JustAI connection, evaluation,
payment, external write, production service, publication, release, or
redistribution. No rootfs change, package installation, second compile,
second start, or retry is authorized by this slice.

Ubuntu package/image rights remain under Canonical's local internal-use
boundary and package licenses recorded in the source packets. No guest bytes
or logs enter tracked HyperAI files.

## Assurance target and claim ceiling

Target is `INTERNAL_PASS` only for one successful local boot to the exact
serial marker with clean VM stop, exited helper, bounded log, and unchanged
RAW digest. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`.

This cannot establish hostile-code isolation, cgroup or DataFusion resource
enforcement, bounded IPC, cancellation, crash/OOM recovery, service-plus-worker
capacity, general image bootability, macOS product support, or production
readiness. Overall Phase 2 remains `NOT_ASSIGNED`.

## Result record

Frozen helper source: 4,320 bytes, SHA-256
`70ce9a9a8a2e40b71d8f09880650c7f180445771565abcb1d7018b68308b61ac`.
The exact `xcrun swiftc -parse-as-library ...` command is recorded above and
exited 0 in 2.1 seconds. The ad-hoc signature carried only
`com.apple.security.virtualization = true`; strict verification reported
`valid on disk` and `satisfies its Designated Requirement`. The entitlement
file SHA-256 was
`5d95521eb14d3cce9af386b484a3c01d767df8681e56d277d999a8f2fa1aa85b`; the
signed binary SHA-256 was
`b68efc934f1daff8d40df452757511a9f3a74f19db8c94baa0e0f245e7ee3b35`.
Entitlement inspection showed exactly the virtualization entitlement. The
inspection command emitted Apple's deprecation warning for the `:-`
entitlement-output path; it still printed the exact single-key plist. The
compiler reported no diagnostics.

Pre-run host gates passed: `kern.hv_support=1`, physical memory reported by
`sysctl` as `38654705664` bytes (36 GiB), and 75% free memory. All three
artifact sizes and SHA-256 values matched the frozen table. The helper emitted
`VZ_SUPPORTED=true`, `CONFIG_VALIDATE=PASS`, and
`CONFIG_COUNTS cpu=1 memory_bytes=1073741824 storage=1 serial_ports=1 network=0 directory_sharing=0 socket=0`.
Its single `start()` call failed with literal output
`VM_HELPER=FAIL domain=VZErrorDomain code=1`; process wall time was under one
second. The local SDK maps code 1 to `VZErrorInternal`; Apple describes it as
a generic internal VM error that can include an unexpected stop
([VZErrorCode](https://developer.apple.com/documentation/virtualization/vzerror/code)).
The helper did not emit the localized error description, so the underlying
cause remains unknown. The VM did not reach the serial marker; the retained
serial file is 0 bytes with SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

Because `start()` threw, the helper did not receive a running VM and did not
call `stop()`; there is no successful stop callback. The helper process exited
and `pgrep` found no remaining helper. The RAW disk post-run SHA-256 remains
`6342a12d3fb4484d7a340fa647e574754d72aa39d535fb4a15ec61c0072b7fce`, equal
to the pre-run digest. Post-run host memory remained 36 GiB with 75% free.
Temporary source, binary, entitlement, and compiler cache were removed; the
empty console log remains in the authorized temporary directory. No input or
product source bytes changed.

Assurance: `INCONCLUSIVE`. Reviewer status: `NOT_REQUESTED`.
Reproducibility: `NOT_REPRODUCED`. Custody is limited to the authenticated
Ubuntu artifacts and empty console log under
`/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`, plus these Markdown
records. Claim ceiling: local host/configuration checks and an inconclusive
Virtualization start call only; no guest boot, isolation, resource, or product
support claim follows. Next blocked gate: a separately named, read-only
diagnostic slice must capture the underlying cause beyond the generic
`VZErrorInternal` classification and determine whether a new VM start is
justified and authorized. Overall Phase 2 remains `NOT_ASSIGNED`.

## References

- Apple: [Running Linux in a Virtual Machine](https://developer.apple.com/documentation/virtualization/running-linux-in-a-virtual-machine)
- Apple: [VZVirtualMachineConfiguration](https://developer.apple.com/documentation/virtualization/vzvirtualmachineconfiguration)
- Apple: [Serial ports](https://developer.apple.com/documentation/virtualization/serial-ports)
- Apple: [VZVirtualMachine start and stop](https://developer.apple.com/documentation/virtualization/vzvirtualmachine)
