# HyperAI Phase 2 macOS Guest Start Diagnostic v1

State slice: [`hyperai.bazaar.data_macos_guest_start_diagnostic.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_start_diagnosticv1).

Status: closed `INCONCLUSIVE`. The one read-only query returned relevant
records, but its execution end time was not captured against the packet's
fixed time-window boundary. The log observations remain diagnostic leads.
The query allowance is consumed. The query targeted the single failed helper
execution recorded in
[`hyperai.bazaar.data_macos_guest_boot_continuation.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_boot_continuationv1).
No helper compile, VM construction, or VM start is authorized.

## Baseline and frozen query

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree. The target was the single helper execution
at approximately 2026-10-01 09:36–09:51 UTC, which returned
`VZErrorDomain` code `1` (`VZErrorInternal`) with no serial output. Apple
defines that code as a generic internal VM error; the framework cause was not
returned by the helper.

Run exactly one macOS Unified Log query for the last 15 minutes ending at
2026-10-01 09:51:03 UTC, using `log show --style compact` and this predicate:

```text
process == "guest-boot" OR (process == "virtualizationd" AND (eventMessage CONTAINS[c] "hyperai-bazaar-vm-boot-continuation-v1" OR eventMessage CONTAINS[c] "guest-boot" OR eventMessage CONTAINS[c] "VZErrorInternal"))
```

Allow one query, 30 seconds, and at most 128 KiB of returned text. If the
query fails, times out, exceeds the cap, or returns ambiguous/unrelated
records, stop and record `INCONCLUSIVE`. Inspect only whether messages
explicitly correlate the exact helper or VM start, and whether they name a
specific cause. Do not infer a cause from absence of matching records.

## Allowed effects and non-goals

Allowed effects: execute the exact read-only Unified Log query above; record
the command, exit code, matching-record count, and a short sanitized summary
in this packet, `docs/state-slices.md`, the Phase 2 Data packet, and the Rust
port subplan. Keep raw log output transient and do not copy it into repository
files or another artifact.

No helper build or execution, VM construction/start/stop, system setting or
entitlement change, log deletion, provider/worker/data access, external write,
or product source change. The previous VM-start allowance remains consumed.

## Claim ceiling

Assurance target is `INTERNAL_PASS` only for the bounded read-only query and
its recorded output handling. Reviewer status is `NOT_REQUESTED`; reproducibility
is `NOT_REPRODUCED`. This can identify a cause only if the host log explicitly
records one. It cannot establish guest boot, VM cleanup beyond the prior
process/hash checks, isolation, resource enforcement, or product support.

## Result record

The executed command was `log show --last 15m --style compact --predicate 'process == "guest-boot" OR (process == "virtualizationd" AND (eventMessage CONTAINS[c] "hyperai-bazaar-vm-boot-continuation-v1" OR eventMessage CONTAINS[c] "guest-boot" OR eventMessage CONTAINS[c] "VZErrorInternal"))'`.
It exited 0 in under 2 seconds with 2,729 bytes of stdout and no stderr, below
the 128 KiB cap. It returned 14 event records for the `guest-boot` process;
there were no matching `virtualizationd` records. Raw output was inspected
transiently and not saved. The query used the relative `--last 15m` option;
the exact execution end time was not recorded against the packet's fixed
09:51:03 UTC boundary, so the frozen window match cannot be confirmed.

The records confirm that DiskImages2 opened the 4-GiB image and classified it
as RAW. One record says that retrieving Disk Image user data failed because
user data is not supported in that image format (DiskImages2 error 45). The
log then records creation of the Virtualization VM XPC connection followed
by cancellation from the current process. It does not report
`VZErrorInternal`, a localized Virtualization error, or a causal link between
the RAW user-data message and the failed `start()` call. The underlying cause
therefore remains unknown; treat the RAW metadata message as a lead only.

Assurance is `INCONCLUSIVE` for this packet because the exact window end was
not captured. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. No VM was created or started by this diagnostic slice. No
raw logs were retained in the repository or elsewhere. The query allowance is
consumed. Any additional VM launch requires a separate exact packet and
explicit user authorization.
