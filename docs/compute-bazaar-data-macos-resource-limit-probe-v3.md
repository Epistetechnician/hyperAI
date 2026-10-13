# Native macOS Data Worker Resource-Limit Probe v3

State slice: [`hyperai.bazaar.data_macos_rlimit_probe.v3`](state-slices.md#hyperai-bazaar-data_macos_rlimit_probev3).

Status: one-run native probe on macOS 26.6.1 arm64 with Apple clang 21.0.0.
The [v1 packet](compute-bazaar-data-macos-resource-limit-probe.md) recorded an
inconclusive `setrlimit` request below the unmeasured virtual-size baseline.
The [v2 packet](compute-bazaar-data-macos-resource-limit-probe-v2.md)
recorded the baseline-relative allocation result, but its frozen proof gate
also required an RSS report, which its refusal branch omitted. This v3 packet
preserves that gate and reports `ru_maxrss` after either branch.

## Question and claim boundary

The helper reads its own `TASK_VM_INFO.virtual_size`, sets its own soft
`RLIMIT_AS` limit to that value plus 64 MiB without raising the hard limit,
then attempts to allocate and touch one 128 MiB buffer. It calls
`getrusage(RUSAGE_SELF)` whether the allocation succeeds or is refused.

This probes one address-space allocation and records the platform's raw
`ru_maxrss` value. It does not establish an RSS ceiling, a general sandbox,
DataFusion startup or query behavior, process-tree containment, or platform
qualification. The 64 MiB headroom applies to virtual address space, not RSS.
See [XNU `task_vm_info_data_t`](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/task_info.h)
and the [XNU resource-limit implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c).

## Exact helper source

The helper is compiled from standard input; its source is not written into the
repository.

```c
#include <errno.h>
#include <mach/mach.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <sys/time.h>
#include <unistd.h>

int main(void) {
    struct rlimit before;
    if (getrlimit(RLIMIT_AS, &before) != 0) {
        perror("getrlimit_before");
        return 70;
    }

    task_vm_info_data_t vm_info;
    mach_msg_type_number_t count = TASK_VM_INFO_COUNT;
    kern_return_t task_result = task_info(
        mach_task_self(), TASK_VM_INFO, (task_info_t)&vm_info, &count);
    if (task_result != KERN_SUCCESS) {
        printf("setup=task_info_failed code=%d\n", task_result);
        return 70;
    }
    printf("virtual_size_before_bytes=%llu\n",
           (unsigned long long)vm_info.virtual_size);
    printf("rlimit_as_before_soft_bytes=%llu\n",
           (unsigned long long)before.rlim_cur);
    printf("rlimit_as_before_hard_bytes=%llu\n",
           (unsigned long long)before.rlim_max);

    const rlim_t headroom = (rlim_t)64 * 1024 * 1024;
    if ((uint64_t)vm_info.virtual_size > (uint64_t)RLIM_INFINITY - headroom) {
        puts("setup=limit_overflow");
        return 70;
    }
    const rlim_t limit = (rlim_t)vm_info.virtual_size + headroom;
    if (before.rlim_max != RLIM_INFINITY && before.rlim_max < limit) {
        puts("setup=insufficient_hard_limit");
        return 70;
    }
    struct rlimit requested = before;
    requested.rlim_cur = limit;
    if (setrlimit(RLIMIT_AS, &requested) != 0) {
        perror("setrlimit");
        return 70;
    }

    struct rlimit after;
    if (getrlimit(RLIMIT_AS, &after) != 0) {
        perror("getrlimit_after");
        return 70;
    }
    printf("rlimit_as_after_soft_bytes=%llu\n",
           (unsigned long long)after.rlim_cur);
    printf("rlimit_as_after_hard_bytes=%llu\n",
           (unsigned long long)after.rlim_max);
    fflush(stdout);

    const size_t bytes = (size_t)128 * 1024 * 1024;
    volatile unsigned char *memory = malloc(bytes);
    if (memory == NULL) {
        printf("allocation=refused errno=%d\n", errno);
    } else {
        long page_size = sysconf(_SC_PAGESIZE);
        if (page_size <= 0) {
            puts("setup=invalid_page_size");
            free((void *)memory);
            return 70;
        }
        for (size_t offset = 0; offset < bytes; offset += (size_t)page_size) {
            memory[offset] = 1;
        }
        memory[bytes - 1] = 1;
        puts("allocation=succeeded");
        free((void *)memory);
    }

    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) != 0) {
        perror("getrusage");
        return 70;
    }
    printf("peak_rss_reported_raw=%ld\n", usage.ru_maxrss);
    return 0;
}
```

## Run and interpretation

The authorized command extracts this exact source from the packet, compiles
once with `/usr/bin/cc -Wall -Wextra -Werror -O2 -x c`, hashes the binary,
and runs the helper once with a 10-second timeout in an automatically cleaned
Python `TemporaryDirectory`. It records output, child status, Git-status digest,
and the read-only import verifier before and after. No retry is allowed.

## Execution result (2026-10-01)

The exact helper source was extracted from this packet and compiled on macOS
`26.6.1` arm64 with Apple clang `21.0.0 (clang-2100.3.34.2)`. Compilation
exited 0; binary SHA-256 was
`f5da4e73db45f15abc88acf1db6fa2dd1781957615159946c8534414ab5b2e7e`.

`TASK_VM_INFO.virtual_size` was `445746020352` bytes. The child set and read
back `RLIMIT_AS` soft limit `445813129216` bytes, exactly 67,108,864 bytes
(64 MiB) above the baseline; the hard limit stayed at
`9223372036854775807` bytes. The single 134,217,728-byte allocation was
refused with `errno=12` (`ENOMEM`). `getrusage` then reported
`ru_maxrss=1392640` in the platform's raw units; the child exited 0.

The Simple Compute Market and HyperIntel imports matched their pins before
and after. JustAI remained at 177 files versus 176 recorded with the same
observed digest
`08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`. Git
status remained at 101 entries with SHA-256
`d0906da43bcb1999447021fe1eed0a2b0843d2338f753d6a7c5e2201f38621c0` before
and after. `TemporaryDirectory` removed the binary.

Assurance is `INTERNAL_PASS` for this exact address-space allocation probe
and raw `ru_maxrss` report only. The result does not establish an RSS ceiling,
a useful whole-process memory budget, DataFusion startup/query behavior,
process-tree containment, or macOS platform qualification. The baseline is
about 415 GiB of virtual address space, not resident memory. Reviewer status
is `NOT_REQUESTED`; reproduction is `NOT_REPRODUCED`. Custody is limited to
the packet source, temporary binary, bounded output, and host metadata. No
product data, service, network, provider, or external write is in scope.
