# HyperAI Phase 2 macOS Guest Start Instrumented v1

State slice: [`hyperai.bazaar.data_macos_guest_start_instrumented.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_start_instrumentedv1).

Status: one-run local synthetic Virtualization.framework continuation under
the user-directed Compute Bazaar Rust port subplan. The earlier boot start
closed `INCONCLUSIVE`; this packet is a fresh identity and permits one
instrumented attempt with the same authenticated image and VM configuration.
No HyperAI product code is changed.

## Objective

Determine whether the prior `VZErrorDomain` code `1` start result can be
classified using the exact thrown `NSError` fields, VM state, guest serial
output, and a correctly bounded host-log query. Keep the guest disk, kernel,
initrd, CPU, memory, devices, boot arguments, signing entitlement, and timeout
unchanged from the previous boot packet. The previous RAW user-data log entry
was not causally linked to startup; do not treat it as the cause.

## Frozen baseline and inputs

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
preserved dirty and untracked worktree. Pre-change SHA-256 values:

- `docs/state-slices.md`:
  `4ef4b4ffbab80e47be9291abcb39d78b2aea2cf043441e34d35016b7090ddb00`
- `docs/compute-bazaar-phase-2-data-packet.md`:
  `1883adb6975e2f46db21bce3ad1b8081d39ec088479a8688890cab48e1e5d77b`
- `docs/compute-bazaar-rust-port-subplan.md`:
  `b0ce3b3a7db4de9133a6ab8e81dddca18cdb9a8881f743a294884e34d56f8ee9`

The only guest inputs are already authenticated and retained under
`/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`:

| Input | Bytes | SHA-256 |
| --- | ---: | --- |
| `resolute-rootfs-arm64.raw.ext4` | 4,294,967,296 | `6342a12d3fb4484d7a340fa647e574754d72aa39d535fb4a15ec61c0072b7fce` |
| `boot-inputs/resolute-server-cloudimg-arm64-vmlinuz-generic` | 23,831,432 | `0bad41327e67c7c79d79b7e5f0ee833d0202be2ddf0f7cbbb984b4cb9c94e108` |
| `boot-inputs/resolute-server-cloudimg-arm64-initrd-generic` | 91,123,407 | `65fa960053965fbcbba70a36e060965f2f4eb46124e8f0d9c0984250edc61117` |

These bytes remain within the existing local internal-use boundary recorded
in the source-check packets. Do not download, copy, convert, modify, unpack,
mount, redistribute, or add any guest bytes to the repository. Recheck all
three sizes and hashes immediately before VM construction and after shutdown.
Any mismatch stops the attempt.

## Exact run contract

- Host: macOS 26.6.1 arm64, Xcode 27.0 build `27A266a`, SDK
  `/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk`.
- Require `kern.hv_support=1`, at least 8 GiB physical memory, at least 25%
  free memory, and at least 20 GiB available on `/private/tmp` before helper
  compilation and again immediately before VM construction.
- Create one temporary root:
  `/private/tmp/hyperai-bazaar-vm-boot-instrumented-v1/`. Refuse to use or
  overwrite it if it already exists.
- Frozen Swift helper: 6,743 bytes, SHA-256
  `a6c7341a17a2772c1c90a5590ef9e335636c2fae954c1454856e2a88ecc49784` at
  `/private/tmp/hyperai-bazaar-vm-boot-instrumented-v1/main.swift`. Verify
  those exact bytes immediately before compilation. Compile once with
  `xcrun swiftc -parse-as-library -sdk /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk -module-cache-path /private/tmp/hyperai-bazaar-vm-boot-instrumented-v1/module-cache -framework Virtualization /private/tmp/hyperai-bazaar-vm-boot-instrumented-v1/main.swift -o /private/tmp/hyperai-bazaar-vm-boot-instrumented-v1/guest-start-instrumented-v1`,
  sign once with only
  `com.apple.security.virtualization = true`, inspect the entitlement, and
  require strict signature verification. No retry is allowed under this
  identity.
- Compile result: exited 0 with no diagnostics in 3.8 seconds. Entitlement
  plist SHA-256 is
  `5d95521eb14d3cce9af386b484a3c01d767df8681e56d277d999a8f2fa1aa85b`.
  The signed helper is 152,016 bytes with SHA-256
  `8fbc9d4ed92b6e64bdf9d048ee79cb58615de72bcbdf04cb5fd497f5d269fb2d`;
  strict verification passed and inspection showed only the virtualization
  entitlement. Do not modify or resign it.
- Require one successful configuration validation, then construct one VM
  with one vCPU, 1 GiB guest memory, the authenticated generic arm64 kernel
  and initrd, the exact command line
  `console=hvc0 root=/dev/vda rootfstype=ext4 ro rootwait systemd.unit=emergency.target systemd.show_status=1`,
  one read-only RAW virtio block device, and one output-only serial port.
- Attach no network, host directory, host socket, USB, graphics, audio,
  keyboard, pointer, credential, HyperAI home, dataset, or other device. Run
  no workload and send no guest input.
- Emit the VM state and `canStart` before the sole `start()` call. On thrown
  errors, emit only bounded `NSError` domain, code, localized description,
  underlying-error domain/code/description to depth 3, sorted `userInfo` key
  names, and post-error VM state/`canStart`; do not print arbitrary object
  descriptions or full `userInfo` values. Capture exact UTC start and end
  timestamps to millisecond precision.
- Permit at most one VM start and 120 seconds after that call begins. The
  success marker is the literal `Welcome to emergency mode!`; serial output
  is capped at 1 MiB. If the VM reaches running state, stop once after the
  marker or deadline and allow 10 seconds for the stop result. If start throws,
  do not retry; record state and destroy the VM object. A supervisor timeout
  or failed cleanup is `INCONCLUSIVE` and requires a process check.
- After the attempt, convert the captured UTC timestamps to enclosing UTC
  whole-second bounds, then run exactly one query with the fixed predicate:
  `log show --style compact --timezone UTC --start "$START_UTC_BOUND" --end "$END_UTC_BOUND" --predicate 'process == "guest-start-instrumented-v1" OR (process == "virtualizationd" AND (eventMessage CONTAINS[c] "guest-start-instrumented-v1" OR eventMessage CONTAINS[c] "VZErrorInternal"))'`.
  Keep raw output transient, cap it at 128 KiB, and record only query status,
  matching count, and a sanitized causal summary. A log query error or
  ambiguous correlation is `INCONCLUSIVE`.
- Hash the root disk after the attempt and require the same frozen digest.
  Confirm the helper exited, capture stop/cleanup outcomes, remove the helper,
  entitlement, compiler cache, console log, and temporary run root after
  recording the necessary digests and bounded results.

## Exact writable files and allowed effects

Writable repository files are limited to:

- `docs/compute-bazaar-data-macos-guest-start-instrumented-v1.md`
- `docs/state-slices.md`
- `docs/compute-bazaar-phase-2-data-packet.md`
- `docs/compute-bazaar-rust-port-subplan.md`

Allowed non-repository effects are limited to the single temporary helper
build/sign/verification, one VM construction and at most one start/stop,
one bounded serial log, one bounded exact-window host-log query, the specified
input/hash/capacity checks, and cleanup. No network, provider, seller, remote
worker, external data source, user payload, external write, source import,
publication, release, or deployment is included.

## Acceptance, assurance, and limits

Record the literal execution result, all input/helper/binary/log digests and
sizes, exact commands, host capacity readings, start/end UTC timestamps,
sanitized thrown-error details, VM state transitions, helper exit status, and
cleanup. Preserve failures. Do not retry compile, sign, VM construction, VM
start, or log query under this identity.

Assurance target: `INTERNAL_PASS` only for the exact host/configuration and
instrumented startup diagnostic if the frozen gates complete. Reviewer status:
`NOT_REQUESTED`. Reproducibility: `NOT_REPRODUCED`. Claim ceiling: the exact
local start behavior and diagnostics for these frozen inputs only. Even a
successful guest boot does not establish cgroup/DataFusion enforcement,
hostile-code isolation, bounded IPC, cancellation, crash/OOM recovery,
combined service/guest capacity, general macOS support, independent review,
or production readiness. Overall Phase 2 remains `NOT_ASSIGNED`.

## References

- [Apple `VZVirtualMachine.start()`](https://developer.apple.com/documentation/virtualization/vzvirtualmachine/start%28%29): reports startup through an async thrown error.
- [Apple `VZVirtualMachine.State`](https://developer.apple.com/documentation/virtualization/vzvirtualmachine/state-swift.enum): enumerates stopped, running, error, and transition states.
- [Apple `VZDiskImageStorageDeviceAttachment`](https://developer.apple.com/documentation/virtualization/vzdiskimagestoragedeviceattachment): documents RAW and ASIF image support.
- [Apple `VZLinuxBootLoader`](https://developer.apple.com/documentation/virtualization/vzlinuxbootloader): documents Linux kernel and optional initrd configuration.

## Execution result

Execution date: 2026-10-01, macOS 26.6.1 build `25G76`, arm64, Xcode 27.0
build `27A266a`, Swift compiler from that Xcode, Python 3.14.5. Pre-start
gates passed: `kern.hv_support=1`, 36 GiB physical memory, 68% free memory,
and about 301 GiB available on `/private/tmp`. All three frozen input sizes
and hashes matched immediately before VM construction and after the attempt.

The helper compiled once with exit 0 and no diagnostics in 3.8 seconds. It
was ad-hoc signed once; strict signature verification passed and inspection
showed only `com.apple.security.virtualization = true`. Configuration
validation passed for one vCPU, 1 GiB memory, one read-only storage device,
one output-only serial device, and zero network/directory-sharing devices.
The VM reported its pre-start state as `rawValue: 0`, `canStart=true`, and
`canStop=false`.

The one start request was
`2026-10-01T12:58:08.782Z`. It returned at
`2026-10-01T12:58:08.868Z` with exit status 30 and
`VZErrorDomain` code `1`; the localized description was “Internal
Virtualization error. The virtual machine failed to start.” The bounded
`userInfo` key list was `NSLocalizedFailure, NSLocalizedFailureReason`; the
packet intentionally did not print their values. Post-error state was
`rawValue: 3`, `canStart=true`, and `canStop=false`, so `stop()` was not
called. No serial marker appeared; `serial.log` was 0 bytes with SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
The supervisor did not time out and the helper exited. This attempt is
`INCONCLUSIVE`; it establishes no guest boot or resource boundary.

The exact one-second helper-process query exited 0 with 2,967 bytes of stdout
and 14 event records. It observed the RAW-image user-data error 45 and XPC
connection activity; no `virtualizationd` record linked those events to the
startup error. The separate daemon-only query reported in
[`compute-bazaar-data-macos-virtualizationd-window-v1.md`](compute-bazaar-data-macos-virtualizationd-window-v1.md)
returned only its 44-byte header with zero records. The broader framework
query is recorded in
[`compute-bazaar-data-macos-virtualization-subsystem-window-v1.md`](compute-bazaar-data-macos-virtualization-subsystem-window-v1.md).

Post-run sizes and SHA-256 values for the RAW disk, kernel, and initrd exactly
matched their pre-run values. Process checks found neither helper PID `24461`
nor Virtualization service PID `24489` alive. The exact temporary run root,
helper, entitlement, compiler cache, and empty serial log were removed. Raw
system-log output was not saved. Assurance for this VM attempt is
`INCONCLUSIVE`; reviewer status is `NOT_REQUESTED`, and reproducibility is
`NOT_REPRODUCED`. No root `pnpm run lint` or product tests were run because
no product source changed. Overall Phase 2 remains `NOT_ASSIGNED`.
