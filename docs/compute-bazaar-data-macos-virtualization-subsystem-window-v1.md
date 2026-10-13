# HyperAI Phase 2 macOS Virtualization Subsystem Window v1

State slice: [`hyperai.bazaar.data_macos_virtualization_subsystem_window.v1`](state-slices.md#hyperai-bazaar-data_macos_virtualization_subsystem_windowv1).

Status: prepared read-only framework diagnostic following two identical
`VZErrorDomain` code `1` VM start failures and a daemon-only query with no
records. This packet authorizes one exact-window query across Apple
Virtualization framework subsystems. It authorizes no helper or VM action.

## Question and fixed query

The instrumented helper's only start request was
`2026-10-01T12:58:08.782Z`; it returned the generic error at
`2026-10-01T12:58:08.868Z`. The bounded `virtualizationd`-only query for
`12:58:07` through `12:58:10` UTC returned no records. Inspect framework and
daemon event sources within the same fixed three-second window for a specific
startup, device, guest-stop, or entitlement error.

Run exactly once:

```sh
log show --style compact --info --debug --timezone UTC \
  --start '2026-10-01 12:58:07+0000' \
  --end '2026-10-01 12:58:10+0000' \
  --predicate '(process == "virtualizationd" OR process == "vmd" OR subsystem CONTAINS[c] "Virtualization")'
```

Allow at most 30 seconds and 128 KiB of stdout. Keep raw output transient.
Report exit code, stdout bytes, record count, and a short sanitized summary
of only records that can be temporally and technically correlated to the
frozen start. Do not treat unrelated events or absence of events as causal.

## Frozen research and selected action

After the same generic error occurred twice, the following candidate causes
and mitigations were checked against current Apple documentation:

1. **Unsupported RAW disk format.** Apple documents both RAW and ASIF for
   Virtualization disk attachments, so the recorded DiskImages2 user-data
   error alone does not prove the RAW root disk is invalid. Keep the exact
   input bytes unchanged.
2. **Insufficient guest memory.** Apple defines `VZError.Code.internalError`
   as an unexpected VM stop, so early guest exit remains plausible. Changing
   from 1 GiB now would confound the first boot comparison; defer until a
   diagnostic identifies a guest-resource failure or a fresh packet compares
   one justified budget change.
3. **Invalid Linux platform/boot-loader pairing.** Apple's contract requires
   `VZLinuxBootLoader` with `VZGenericPlatformConfiguration`; the current
   helper matches this pair and configuration validation passed.
4. **Unsupported output-only serial attachment.** Apple documents a
   file-based one-way guest-to-host serial attachment. The current
   file-handle output attachment is also a documented API shape; no source
   evidence identifies it as the cause. Keep it fixed while inspecting logs.
5. **Missing Virtualization entitlement.** Apple requires
   `com.apple.security.virtualization`; the signed helper's exact embedded
   entitlement was inspected and strict signature verification passed. This
   does not qualify a distribution signing path, but makes entitlement
   absence a weaker local-run hypothesis.

The selected next action is this no-effect framework log query because it
can separate a guest stop or framework/device failure from the uninformative
API error without changing inputs or consuming another VM start. If the query
does not identify a cause, stop this diagnostic chain. Any experiment that
changes memory, serial, storage, boot inputs, or signing requires its own
packet and explicit user authorization before a new VM start.

Sources: Apple [`VZError.internalError`](https://developer.apple.com/documentation/virtualization/vzerror/internalerror), [`VZDiskImageStorageDeviceAttachment`](https://developer.apple.com/documentation/virtualization/vzdiskimagestoragedeviceattachment), [`VZLinuxBootLoader`](https://developer.apple.com/documentation/virtualization/vzlinuxbootloader), [Virtualization entitlement](https://developer.apple.com/documentation/virtualization/adding-the-virtualization-entitlement-to-your-project), and [`VZFileSerialPortAttachment`](https://developer.apple.com/documentation/virtualization/vzfileserialportattachment).

## Exact scope

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the preserved dirty and untracked worktree. Pre-change SHA-256 values:

- `docs/state-slices.md`:
  `ba192c840a51a704b1e978eae9a94d0d2a220a64e9a9db7ab03d1b3a59b82ed9`
- `docs/compute-bazaar-phase-2-data-packet.md`:
  `1883adb6975e2f46db21bce3ad1b8081d39ec088479a8688890cab48e1e5d77b`
- `docs/compute-bazaar-rust-port-subplan.md`:
  `b0ce3b3a7db4de9133a6ab8e81dddca18cdb9a8881f743a294884e34d56f8ee9`
- `docs/compute-bazaar-data-macos-guest-start-instrumented-v1.md`:
  `352eb1b2312d13d3e69c48153bb69ae2432c06cf5866bef95fc1d972934a9757`
- `docs/compute-bazaar-data-macos-virtualizationd-window-v1.md`:
  `7fc623075bf1b16e47854299102f71782ea6785f935e0a660abab69a58e6a885`

Exact writable files:

- `docs/compute-bazaar-data-macos-virtualization-subsystem-window-v1.md`
- `docs/state-slices.md`
- `docs/compute-bazaar-phase-2-data-packet.md`
- `docs/compute-bazaar-rust-port-subplan.md`

No files or services are read or modified beyond the named bounded system
log query and those documentation files. No VM start, helper execution, disk
access, network, user payload, provider, seller, or external action is
authorized by this packet.

Claim ceiling: at most `INTERNAL_PASS` for this exact read-only query and
output-handling scope. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. No VM cause, guest boot, resource isolation, worker bound,
platform support, or Phase 2 acceptance follows without directly correlated
evidence. Publish boundary: local Markdown only.

## Execution result

The exact query exited 0 in under one second with 3,728 bytes of stdout and
no stderr, below the 128-KiB cap. It returned 22 event records. `launchd`
correlated helper PID `24461` with a `com.apple.Virtualization.VirtualMachine`
service instance, spawned service PID `24489`, and reported the service
running. A framework `breadcrumb` error record followed; the service then
exited with status 0 after 84 ms. No record states why the VM service exited,
and the empty guest serial log provides no boot marker. This is a direct
temporal/process correlation to the exact start, not a causal explanation.
Raw system-log output was not retained.

Assurance is `INCONCLUSIVE` for cause identification and `INTERNAL_PASS` for
the exact bounded query/output handling. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. The cause remains unknown. This packet
authorizes no further VM attempt.
