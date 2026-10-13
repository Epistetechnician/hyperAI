# Native macOS Data Worker Resource-Limit Probe v2

State slice: [`hyperai.bazaar.data_macos_rlimit_probe.v2`](state-slices.md#hyperai-bazaar-data_macos_rlimit_probev2).

Status: one-run native probe on macOS 26.6.1 arm64 with Apple clang 21.0.0.
The [v1 attempt](compute-bazaar-data-macos-resource-limit-probe.md) was
inconclusive: its 128 MiB limit request returned `EINVAL` before allocation,
and the executed format strings did not exactly match its frozen source.

## Question and claim boundary

The current XNU `RLIMIT_AS` path delegates to `vm_map_set_size_limit`, which
can reject a requested limit below current address-space use. This probe reads
the child's `TASK_VM_INFO.virtual_size`, sets the child's `RLIMIT_AS` soft
limit to that value plus 64 MiB, and attempts one 128 MiB allocation while
touching one byte per page. The hard limit is unchanged.

This probes one virtual-address allocation boundary. It does not establish a
resident-memory ceiling, a general sandbox, DataFusion startup or query
behavior, process-tree containment, or platform qualification. Sources:
[XNU `task_vm_info_data_t`](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/task_info.h),
[XNU resource-limit implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c),
and [Apple's `task_vm_info_data_t` reference](https://developer.apple.com/documentation/kernel/task_vm_info_data_t).

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
        struct rusage usage;
        if (getrusage(RUSAGE_SELF, &usage) != 0) {
            perror("getrusage");
            free((void *)memory);
            return 70;
        }
        puts("allocation=succeeded");
        printf("peak_rss_reported_raw=%ld\n", usage.ru_maxrss);
        free((void *)memory);
    }
    return 0;
}
```

## Run

The slice's command uses Python `TemporaryDirectory` to create and remove an
isolated temp directory. It extracts this exact fenced source, compiles once
with `/usr/bin/cc -Wall -Wextra -Werror -O2 -x c`, hashes the binary, and runs
the helper once with a 10-second timeout. It captures output, exit status or
signal, and binary hash. Before and after, it records the Git-status digest
and runs the read-only import verifier. No test retry is allowed. An RSS value
is reported only if the allocation succeeds.

## Execution result (2026-10-01)

The native environment was macOS `26.6.1` arm64 with Apple clang
`21.0.0 (clang-2100.3.34.2)`. The helper source was extracted from this
packet, so the compiled bytes match the frozen C block. Compilation exited 0;
binary SHA-256 was
`57852ca3e4274aff16030218df92115fd60f7741380023f613624b8d812eb84d`.

`TASK_VM_INFO.virtual_size` was `445746184192` bytes. The helper set and read
back `RLIMIT_AS` soft limit `445813293056` bytes, exactly 67,108,864 bytes
(64 MiB) above that baseline; the hard limit stayed at
`9223372036854775807` bytes. One 134,217,728-byte allocation was refused with
`errno=12` (`ENOMEM`), and the helper exited 0. It did not report peak RSS
because the source only calls `getrusage` after a successful allocation.

The Simple Compute Market and HyperIntel imports matched their pins before
and after. JustAI remained 177 files versus 176 recorded with observed digest
`08051f92b2c577583b09b921c7ecd87f892a3d6307291f23161928386cd5cfa4`. Git
status stayed at 100 entries with SHA-256
`9ffcfb6c14725dfa6eb72a2587d97e8c3decb6096c16522ed55f5111c16a31de` before
and after. Python `TemporaryDirectory` removed the compiled binary.

This execution is `INCONCLUSIVE` under its frozen state-slice gates because
the exact helper reports `ru_maxrss` only after a successful allocation. The
128 MiB allocation was refused, so the required RSS report is absent. The raw
observation remains: this helper's `RLIMIT_AS` value was accepted and the
allocation was refused under that setting. The baseline virtual size was
about 415 GiB; that is virtual address space, not resident memory. This does
not establish an RSS ceiling, a useful whole-process memory budget, DataFusion
startup/query behavior, process-tree containment, or macOS platform
qualification. Reviewer status is `NOT_REQUESTED`; reproduction is
`NOT_REPRODUCED`. Custody is limited to the packet source, temporary binary,
bounded output, and host metadata. No product data, service, network,
provider, or external write is in scope. A fresh slice is required to capture
RSS whether allocation succeeds or is refused; do not retry under v2.
