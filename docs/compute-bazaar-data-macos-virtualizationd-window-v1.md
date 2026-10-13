# HyperAI Phase 2 macOS Virtualization Daemon Window v1

State slice: [`hyperai.bazaar.data_macos_virtualizationd_window.v1`](state-slices.md#hyperai-bazaar-data_macos_virtualizationd_windowv1).

Status: prepared read-only diagnostic following two identical
`VZErrorDomain` code `1` guest-start results. This packet authorizes one
bounded query of `virtualizationd` records for the exact second instrumented
attempt. It authorizes no helper build or VM action.

## Question and frozen window

The helper `guest-start-instrumented-v1` requested its only VM start at
`2026-10-01T12:58:08.782Z`; `start()` failed at
`2026-10-01T12:58:08.868Z` with `VZErrorDomain` code `1` and the generic
description “Internal Virtualization error. The virtual machine failed to
start.” Its VM state was `VZVirtualMachineState(rawValue: 3)` and
`canStop=false`. The previous one-second filtered query returned no
`virtualizationd` rows. This query checks whether the daemon emitted a
correlated record that did not contain the helper name or the error string.

Run exactly once:

```sh
log show --style compact --timezone UTC \
  --start '2026-10-01 12:58:07+0000' \
  --end '2026-10-01 12:58:10+0000' \
  --process virtualizationd
```

The fixed interval encloses the helper's recorded start and completion with
one second of log-delivery margin on each side. Allow at most 30 seconds and
128 KiB of stdout. Keep the raw output transient; do not save, copy, or paste
it into the repository. Report exit code, byte count, record count, and only
a short sanitized summary of records that correlate to the exact VM start.
Do not infer causality from unrelated daemon records or from an absence of
records.

## Exact scope

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the existing dirty and untracked worktree. Pre-change SHA-256 values:

- `docs/state-slices.md`:
  `7965adca3a7ba2a995fc345dd69ee128cb41aaf3d6205f776d1fa9c919585f4a`
- `docs/compute-bazaar-phase-2-data-packet.md`:
  `1883adb6975e2f46db21bce3ad1b8081d39ec088479a8688890cab48e1e5d77b`
- `docs/compute-bazaar-rust-port-subplan.md`:
  `b0ce3b3a7db4de9133a6ab8e81dddca18cdb9a8881f743a294884e34d56f8ee9`
- `docs/compute-bazaar-data-macos-guest-start-instrumented-v1.md`:
  `352eb1b2312d13d3e69c48153bb69ae2432c06cf5866bef95fc1d972934a9757`

Exact writable files:

- `docs/compute-bazaar-data-macos-virtualizationd-window-v1.md`
- `docs/state-slices.md`
- `docs/compute-bazaar-phase-2-data-packet.md`
- `docs/compute-bazaar-rust-port-subplan.md`

Allowed effects: execute the exact read-only query once, inspect its
transient filtered output, and record the bounded result in those four files.
No helper, VM, image, service, provider, dataset, external source, user data,
network request, external write, publication, or deployment may be accessed
or modified.

Claim ceiling: at most `INTERNAL_PASS` for this exact daemon-log query and
output-handling contract. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. The query cannot prove a VM cause from
unrelated events, establish guest boot or isolation, or close Phase 2.
Publish boundary: local Markdown only.

## Research notes

Apple documents that `VZError.Code.internalError` can indicate that the VM
unexpectedly stopped; its public description does not identify why. Apple
documents RAW as a supported disk-image format, so the DiskImages2 error 45
about unsupported RAW user-data retrieval is not by itself evidence that RAW
caused the VM error. The existing `VZLinuxBootLoader` plus
`VZGenericPlatformConfiguration` and the single virtualization entitlement
match Apple's documented API requirements. Output-only serial attachment is
also an explicitly supported Apple API shape. Plausible remaining causes
include an early guest stop, insufficient guest memory during boot, an
unreported framework/device failure, or an entitlement/runtime issue not
shown by strict local signature inspection. The daemon-window query is the
lowest-effect next diagnostic; it changes no VM or guest input.

Sources: Apple [`VZError.internalError`](https://developer.apple.com/documentation/virtualization/vzerror/internalerror), [`VZDiskImageStorageDeviceAttachment`](https://developer.apple.com/documentation/virtualization/vzdiskimagestoragedeviceattachment), [`VZLinuxBootLoader`](https://developer.apple.com/documentation/virtualization/vzlinuxbootloader), [Virtualization entitlement](https://developer.apple.com/documentation/virtualization/adding-the-virtualization-entitlement-to-your-project), and [`VZFileSerialPortAttachment`](https://developer.apple.com/documentation/virtualization/vzfileserialportattachment).

## Execution result

The exact `virtualizationd`-only query exited 0 in under one second with 44
bytes of stdout and no stderr. It returned only its header and zero daemon
records. No raw output was retained. This is `INTERNAL_PASS` for the bounded
query and output handling only; it does not explain the repeated
`VZErrorDomain` code `1`. Reviewer status is `NOT_REQUESTED` and reproduction
is `NOT_REPRODUCED`. The start cause remains `INCONCLUSIVE`; no further VM
start is authorized by this packet.
