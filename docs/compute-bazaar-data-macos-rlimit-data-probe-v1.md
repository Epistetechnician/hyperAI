# Native macOS `RLIMIT_DATA` Data Worker Probe v1

State slice: [`hyperai.bazaar.data_macos_rlimit_data_probe.v1`](state-slices.md#hyperai-bazaar-data_macos_rlimit_data_probev1).

Status: the one authorized synthetic macOS `26.6.1` arm64 probe completed
`INCONCLUSIVE`. It changed no HyperAI runtime code.

## Question and claim boundary

Apple's archived `setrlimit(2)` reference defines `RLIMIT_DATA` as the data
segment limit and describes it as controlling how far a process may extend
its break with `sbrk`. The current XNU implementation routes this limit to
`vm_map_set_data_limit`. macOS defines `RLIMIT_RSS` as an alias for
`RLIMIT_AS`, and the prior `RLIMIT_AS` probe did not establish a useful worker
memory budget. This probe screens whether `RLIMIT_DATA` refuses one oversized
`sbrk`, anonymous `mmap`, and system-allocator `malloc` request in separate
child processes. See [Apple's `setrlimit(2)` reference](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html),
[XNU `RLIMIT_DATA` handling](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c),
and [XNU resource-limit definitions](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/resource.h).

Each child sets its own soft `RLIMIT_DATA` to 64 MiB, preserves its inherited
hard limit, reads the limit back, performs one allocation case, captures raw
`ru_maxrss`, and exits. The cases use separate processes so one operation
does not consume the next case's budget. A result applies only to these three
allocation forms; it does not establish DataFusion or whole-process memory
enforcement.

## Exact helper source

The helper is compiled from this document into an automatically removed
temporary directory and is not retained elsewhere.

```c
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static int touch_pages(volatile unsigned char *memory, size_t bytes) {
    long page_size = sysconf(_SC_PAGESIZE);
    if (page_size <= 0) return -1;
    for (size_t offset = 0; offset < bytes; offset += (size_t)page_size) {
        memory[offset] = 1;
    }
    memory[bytes - 1] = 1;
    return 0;
}

static int run_case(const char *name, int mode) {
    struct rlimit before;
    struct rlimit requested;
    struct rlimit after;
    if (getrlimit(RLIMIT_DATA, &before) != 0) {
        perror("getrlimit_before");
        return 70;
    }
    requested = before;
    requested.rlim_cur = (rlim_t)64 * 1024 * 1024;
    if (before.rlim_max < requested.rlim_cur) {
        puts("setup=hard_limit_below_64_mib");
        return 71;
    }
    if (setrlimit(RLIMIT_DATA, &requested) != 0) {
        printf("setup=setrlimit_failed errno=%d\n", errno);
        return 72;
    }
    if (getrlimit(RLIMIT_DATA, &after) != 0) {
        perror("getrlimit_after");
        return 73;
    }
    printf("case=%s limit_soft=%llu limit_hard=%llu\n", name,
           (unsigned long long)after.rlim_cur,
           (unsigned long long)after.rlim_max);
    errno = 0;
    if (mode == 0) {
        const size_t bytes = (size_t)80 * 1024 * 1024;
#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
#endif
        void *old_break = sbrk(0);
        if (old_break == (void *)-1) {
            printf("operation=sbrk_base_break_unavailable errno=%d\n", errno);
            return 84;
        }
        void *memory = sbrk((intptr_t)bytes);
        if (memory == (void *)-1) {
            printf("operation=sbrk_refused errno=%d\n", errno);
        } else {
            if (touch_pages((volatile unsigned char *)memory, bytes) != 0) {
                puts("operation=sbrk_touch_failed");
                return 74;
            }
            printf("operation=sbrk_succeeded bytes=%llu\n",
                   (unsigned long long)bytes);
            if (brk(old_break) != 0) {
                printf("cleanup=brk_failed errno=%d\n", errno);
                return 75;
            }
        }
#if defined(__clang__)
#pragma clang diagnostic pop
#endif
    } else if (mode == 1) {
        const size_t bytes = (size_t)128 * 1024 * 1024;
        void *memory = mmap(NULL, bytes, PROT_READ | PROT_WRITE,
                            MAP_PRIVATE | MAP_ANON, -1, 0);
        if (memory == MAP_FAILED) {
            printf("operation=mmap_refused errno=%d\n", errno);
        } else {
            if (touch_pages((volatile unsigned char *)memory, bytes) != 0) {
                puts("operation=mmap_touch_failed");
                return 76;
            }
            printf("operation=mmap_succeeded bytes=%llu\n",
                   (unsigned long long)bytes);
            if (munmap(memory, bytes) != 0) {
                printf("cleanup=munmap_failed errno=%d\n", errno);
                return 77;
            }
        }
    } else {
        const size_t bytes = (size_t)128 * 1024 * 1024;
        volatile unsigned char *memory = malloc(bytes);
        if (memory == NULL) {
            printf("operation=malloc_refused errno=%d\n", errno);
        } else {
            if (touch_pages(memory, bytes) != 0) {
                puts("operation=malloc_touch_failed");
                free((void *)memory);
                return 78;
            }
            printf("operation=malloc_succeeded bytes=%llu\n",
                   (unsigned long long)bytes);
            free((void *)memory);
        }
    }
    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) != 0) {
        perror("getrusage");
        return 79;
    }
    printf("peak_rss_reported_raw=%ld\n", usage.ru_maxrss);
    return 0;
}

static int launch_case(const char *name, int mode) {
    if (fflush(stdout) != 0) {
        perror("fflush_before_fork");
        return 83;
    }
    pid_t child = fork();
    if (child < 0) {
        perror("fork");
        return 80;
    }
    if (child == 0) {
        int result = run_case(name, mode);
        if (fflush(stdout) != 0) _exit(83);
        _exit(result);
    }
    int status = 0;
    for (;;) {
        pid_t waited = waitpid(child, &status, 0);
        if (waited == child) break;
        if (waited < 0 && errno == EINTR) continue;
        perror("waitpid");
        return 81;
    }
    if (WIFEXITED(status)) {
        printf("case=%s child_exit=%d\n", name, WEXITSTATUS(status));
        if (fflush(stdout) != 0) {
            perror("fflush_parent_status");
            return 83;
        }
        return WEXITSTATUS(status);
    }
    if (WIFSIGNALED(status)) {
        printf("case=%s child_signal=%d\n", name, WTERMSIG(status));
        if (fflush(stdout) != 0) {
            perror("fflush_parent_signal");
            return 83;
        }
    }
    return 82;
}

int main(void) {
    int status = launch_case("sbrk_80_mib", 0);
    if (status != 0) return status;
    status = launch_case("mmap_128_mib", 1);
    if (status != 0) return status;
    return launch_case("malloc_128_mib", 2);
}
```

## Authorized run

Compile exactly once with `/usr/bin/cc -Wall -Wextra -Werror -O2 -x c`,
reading the source from standard input. Hash the binary, execute it exactly
once with a 15-second timeout, and capture bounded output and parent/child
statuses. The helper makes no network calls and reads no files. Each child
changes only its own soft `RLIMIT_DATA`; the parent and hard limits remain
unchanged. Use a temporary directory that is removed on exit.

Run `python3 scripts/verify_imports.py` before and after; the three source
tree digests must match. Do not retry if setup, build, child status, or output
gates fail.

## Result and interpretation

Execution (2026-10-01): on macOS `26.6.1` arm64, the helper source SHA-256 was
`dcf8ce2eb6a76aebda2df272cf707f55fa1a87be9e44fa5fa15526c4ffbf90da`.
`/usr/bin/cc -Wall -Wextra -Werror -O2 -x c` compiled it once with exit 0 and
empty stdout/stderr. The temporary binary SHA-256 was
`a66ee421b7b30f9068fbf92464cb81007fdfb133538e7963a96d0e7735a4e68e`.
The one execution completed before its 15-second timeout with helper exit 72,
stdout `setup=setrlimit_failed errno=22` followed by
`case=sbrk_80_mib child_exit=72`, and empty stderr. The first child failed to
set its soft `RLIMIT_DATA`; no allocation was attempted, no limit readback or
`ru_maxrss` was emitted, and the `mmap` and `malloc` children did not run. The
temporary directory was removed.

The result is `INCONCLUSIVE`: public XNU source states that `RLIMIT_DATA`
returns `EINVAL` when the requested limit is below current data usage. That is
a plausible explanation here, but this helper did not measure the child's
current data usage, so the cause is unconfirmed. No allocation-path or
DataFusion worker conclusion follows. Do not retry under this slice. The import
verifier passed before and after with unchanged pins: Simple Compute Market
2,189 files / `afe6d9363a24fa627d9530240c190b6e68cdd1b7d6425d54d0b7547d829207ab`,
HyperIntel 99 files / `62d9669279d177946a3cefe03211757cd6a48fb3bcbbdc9836d97b2d35b3b99b`,
and JustAI 176 files / `e853b7319aa9c1e1f4a49d5717b48e634ac2d39cd551621aa0324c0708e290c4`.

Assurance: `INCONCLUSIVE` for this candidate probe; reviewer status:
`NOT_REQUESTED`; reproducibility: `NOT_REPRODUCED`. Custody was limited to the
helper source in this packet and the temporary binary; no dataset, service,
network, or external source was used. The next gate is a fresh named,
documentation-first slice selecting another macOS worker-isolation mechanism
and defining its proof protocol. No runtime mechanism is selected by this
result.
