# Native macOS Data Worker Resource-Limit Probe

State slice: [`hyperai.bazaar.data_macos_rlimit_probe.v1`](state-slices.md#hyperai-bazaar-data_macos_rlimit_probev1).

Status: one-run native platform probe, authorized for macOS 26.6.1 arm64 with
Apple clang 21.0.0. This does not authorize a DataFusion worker or alter the
Phase 2 API contract.

## Question and claim boundary

Does this exact macOS environment refuse a fixed allocation in a child process
after the child lowers its own `RLIMIT_AS` soft limit to 128 MiB? The helper
attempts one 192 MiB allocation and touches one byte per page. It captures the
requested and effective limits, allocation result, process peak RSS report,
and exit status.

The test concerns one address-space allocation. Apple's XNU headers define
`RLIMIT_AS` and alias `RLIMIT_RSS` to it, and Apple's manual page lists
`setrlimit`; neither source by itself establishes current runtime enforcement.
See [Apple's `setrlimit` manual page](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html)
and [Apple's XNU resource header](https://github.com/apple/darwin-xnu/blob/main/bsd/sys/resource.h).

An allocation refusal is limited evidence for this helper and boundary. A
successful allocation shows that this candidate did not block the tested
allocation. Neither outcome establishes a resident-memory ceiling, DataFusion
startup or query behavior, process-tree containment, a general sandbox, or
macOS platform qualification.

## Exact helper source

The command compiles this source from standard input into a temporary binary;
it does not write helper source into the repository.

```c
#include <errno.h>
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
    printf("rlimit_as_before_soft_bytes=%llu\n",
           (unsigned long long)before.rlim_cur);
    printf("rlimit_as_before_hard_bytes=%llu\n",
           (unsigned long long)before.rlim_max);

    const rlim_t limit = (rlim_t)128 * 1024 * 1024;
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

    const size_t bytes = (size_t)192 * 1024 * 1024;
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

## Run and interpretation

The authorized command compiles with `/usr/bin/cc -Wall -Wextra -Werror -O2`,
hashes the binary, runs it once, records its output and exit status, and removes
the temporary directory. Setup failures, signals, timeout, or ambiguous output
are `INCONCLUSIVE`; the slice does not permit a retry. `ru_maxrss` is retained
as the platform's reported raw value and is not treated as a memory-limit
proof.

## Execution result (2026-10-01)

This attempt is `INCONCLUSIVE`; it does not establish allocation enforcement.
The binary compiled with exit 0 and SHA-256
`1b69d166e09e460e4dbb69da9f09d4f1ecad85f0360b2e65e8ad957fbcc52e09`.
The child reported pre-change soft and hard limits of
`9223372036854775807` bytes, then `setrlimit(RLIMIT_AS, ...)` returned
`EINVAL` and the child exited 70 before attempting the allocation. No peak RSS
was measured.

The compiled helper's format strings emitted literal `\\n` text instead of
line breaks, so the executed source did not exactly match the source recorded
above. The captured output still identifies the failing call, but this source
mismatch also fails the packet's exact-source gate. The temporary binary was
removed. The Simple Compute Market and HyperIntel imports matched their pins
before and after; JustAI remained at 177 files versus 176 recorded, with the
same observed tree digest. Git status remained 99 entries with the same
SHA-256 before and after. No product data, service, provider, external write,
or persistent artifact was involved.

Apple's current XNU source routes `RLIMIT_AS` through
`vm_map_set_size_limit` and can return `EINVAL` when the current address-space
usage already exceeds the requested limit. This attempt did not record the
child's virtual-map size, so that is a plausible explanation rather than a
confirmed cause. A fresh slice would need to record the baseline and choose a
limit above it before making another allocation attempt. The v1 slice permits
no retry.

Assurance: `INCONCLUSIVE` for this attempted setup only. Reviewer status:
`NOT_REQUESTED`. Reproduction: `NOT_REPRODUCED`. Custody is limited to the
fixed probe source, a temporary compiled binary, bounded output, and local
host metadata. The result does not qualify macOS resource enforcement, a
DataFusion worker, a resident-memory ceiling, or a supported platform.
