# HyperAI Phase 2 macOS Virtualization Host Preflight v1

State slice: [`hyperai.bazaar.data_macos_virtualization_preflight.v1`](state-slices.md#hyperai-bazaar-data_macos_virtualization_preflightv1).

Status: scoped local synthetic host preflight. This checks whether a
standalone, locally ad-hoc-signed process can validate a minimal
Virtualization.framework configuration with the required entitlement. It
does not boot a VM or qualify a Data worker.

## Baseline and purpose

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked working tree. The selected host is macOS 26.6.1
(`25G76`), Darwin 25.6.0, Apple Silicon `arm64`; the pinned compiler is Swift
6.4.0 from Xcode's macOS 26.0 SDK. The read-only host check reported
`kern.hv_support: 1`. The bounded search of Downloads and checked-out GitHub
repositories found no local Linux guest image.

The preceding [`RLIMIT_DATA` probe](compute-bazaar-data-macos-rlimit-data-probe-v1.md)
was `INCONCLUSIVE`; the options assessment recommends a Linux guest as the
next candidate. Before sourcing a guest image, this preflight checks the host
framework and local entitlement path with a minimal configuration that never
starts. No guest image or third-party data is needed for this test.

## Frozen contract

- Compile one standalone Swift helper against the system Virtualization
  framework. Embed only `com.apple.security.virtualization = true` in its
  ad-hoc code signature; inspect the embedded entitlement before execution.
- The helper checks `VZVirtualMachine.isSupported`, configures one virtual
  CPU and 512 MiB guest-visible memory, uses a placeholder readable kernel
  URL, and calls `VZVirtualMachineConfiguration.validate()` once.
- Do not instantiate or start `VZVirtualMachine`; do not add storage, network,
  filesystem-share, or socket devices. The helper prints only framework
  support and configuration-validation status/error domain and code.
- Use one compile and one execution with a 60-second wall limit. A compile,
  signing, or validation failure is retained as failure or inconclusive; no
  retry occurs under this slice.
- Temporary source, entitlement file, compiler cache, and binary live only
  under `scratch/bazaar-vz-host-preflight-v1/` and are removed after evidence
  capture. No repository source, API, dependency, package, or platform
  support declaration changes.

## Source, data, and rights decision

This preflight uses no downloaded guest image, dataset, provider data, or
Bazaar source. HyperAI Phase 2 continues to use its HyperAI-authored synthetic
snapshot. A later guest-boot packet must identify the exact official Linux
kernel and root filesystem source, architecture, release, cryptographic
signature/checksum verification, component-license record, local retention,
and no-redistribution boundary before download or launch. The unpinned local
Bazaar license remains irrelevant to this candidate.

## Proof gates and claim ceiling

Record the helper source and binary SHA-256 digests, exact compile/sign/run
commands, embedded entitlement result, host details, and literal framework
response. A pass proves only that this host accepted the entitlement and
minimal configuration for this ad-hoc helper. It does not establish a
distribution signing path, VM boot, cgroup enforcement, no-network guest
isolation, cancellation, crash/OOM cleanup, host-plus-guest capacity,
DataFusion resource enforcement, hostile-code safety, or macOS product
support.

If the ad-hoc entitlement is rejected, stop and record the blocker; do not
try another signer or launch a VM. If it passes, the next separately named
packet must freeze and rights-check the exact arm64 guest image, IPC and
snapshot handoff, cgroup configuration, lifecycle failures, budgets, and
cleanup evidence before any VM is started.

## Execution result (2026-10-01)

The one locked compile, ad-hoc signing, signature inspection, signature
verification, and one helper execution all passed. The signature contained
only `com.apple.security.virtualization = true`; `codesign --verify --strict`
reported a valid signature satisfying its designated requirement. The helper
returned exactly:

```text
VZ_SUPPORTED=true
CONFIG_VALIDATE=PASS
```

It did not instantiate or start a VM. No Linux image, dataset, network device,
external source, provider, or remote worker was used. `kern.hv_support=1` was
observed before the helper run. This is `INTERNAL_PASS` only for host support,
the ad-hoc entitlement, and the placeholder configuration check on this host.

Commands:

```sh
xcrun swiftc -module-cache-path scratch/bazaar-vz-host-preflight-v1/module-cache -framework Virtualization scratch/bazaar-vz-host-preflight-v1/main.swift -o scratch/bazaar-vz-host-preflight-v1/preflight
codesign --force --sign - --entitlements scratch/bazaar-vz-host-preflight-v1/entitlements.plist scratch/bazaar-vz-host-preflight-v1/preflight
codesign -d --entitlements :- scratch/bazaar-vz-host-preflight-v1/preflight
codesign --verify --strict --verbose=2 scratch/bazaar-vz-host-preflight-v1/preflight
scratch/bazaar-vz-host-preflight-v1/preflight
```

SHA-256: helper source
`ae8968bf0d042e5f429e4788cc2bf4378a90c0e589e1e34b3754126e2fa9b093`;
entitlement plist
`5d95521eb14d3cce9af386b484a3c01d767df8681e56d277d999a8f2fa1aa85b`;
binary
`d3335da3e408009fe23418779a89dbd8be0d7bbc4582ae3c751a69dd6d392da2`.
The temporary helper, plist, binary, and compiler cache were removed after
recording their digests. Reviewer status is `NOT_REQUESTED`, reproducibility
is `NOT_REPRODUCED`, and the next blocked authority gate is the separately
scoped, rights-checked Linux guest lifecycle packet. Distribution signing,
guest memory enforcement, DataFusion enforcement, and macOS support remain
unproven.

### Exact helper source

The fenced source and entitlement content below are the exact bytes hashed
above, each with one final LF. They are retained here as evidence; no helper
file or binary remains in the worktree.

`main.swift`:

```swift
import Foundation
import Virtualization

func finish(_ message: String, code: Int32) -> Never {
    print(message)
    exit(code)
}

let supported = VZVirtualMachine.isSupported
print("VZ_SUPPORTED=\(supported)")
guard supported else {
    finish("PREFLIGHT=UNSUPPORTED", code: 20)
}

let configuration = VZVirtualMachineConfiguration()
configuration.cpuCount = 1
configuration.memorySize = 512 * 1024 * 1024
configuration.bootLoader = VZLinuxBootLoader(
    kernelURL: URL(fileURLWithPath: "/dev/null")
)

do {
    try configuration.validate()
    finish("CONFIG_VALIDATE=PASS", code: 0)
} catch let error as NSError {
    finish(
        "CONFIG_VALIDATE=FAIL domain=\(error.domain) code=\(error.code)",
        code: 21
    )
}
```

`entitlements.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>com.apple.security.virtualization</key>
	<true/>
</dict>
</plist>
```

## References

- Apple's [Virtualization entitlement setup](https://developer.apple.com/documentation/virtualization/adding-the-virtualization-entitlement-to-your-project) requires `com.apple.security.virtualization` for Virtualization APIs.
- Apple's [entitlement reference](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.virtualization) describes checking system support and using configuration validation to check the required entitlement.
- Apple's [Linux VM sample](https://developer.apple.com/documentation/virtualization/running-linux-in-a-virtual-machine) documents configuring guest CPU/memory and validating a Linux boot configuration before starting it.
