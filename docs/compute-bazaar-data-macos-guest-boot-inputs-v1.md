# HyperAI Phase 2 macOS Guest Boot Inputs v1

State slice: [`hyperai.bazaar.data_macos_guest_boot_inputs.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_boot_inputsv1).

Status: one-run local acquisition and authentication of the exact dated Ubuntu
arm64 kernel and initrd inputs. This is a preparation slice only; it does not
authorize VM instantiation or boot.

## Baseline and purpose

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree and completed
`hyperai.bazaar.data_macos_guest_image_continuation.v1` record. The local RAW
root disk was built from the authenticated Ubuntu 26.04 arm64 root archive for
snapshot `20260927`; its SHA-256 is
`6342a12d3fb4484d7a340fa647e574754d72aa39d535fb4a15ec61c0072b7fce`.
The root archive contains no `/boot` files. Canonical publishes generic arm64
`vmlinuz` and `initrd` artifacts in the same dated snapshot's `unpacked/`
directory. Authenticate and retain only those boot inputs and their signed
checksum metadata to prepare a later exact VM packet.

## Frozen source and custody

The only network reads are direct HTTPS GETs, with redirects disabled, from
`https://cloud-images.ubuntu.com/resolute/20260927/unpacked/` for:

- `SHA256SUMS` (maximum 4 KiB)
- `SHA256SUMS.gpg` (maximum 2 KiB)
- `resolute-server-cloudimg-arm64-vmlinuz-generic` (maximum 24 MiB)
- `resolute-server-cloudimg-arm64-initrd-generic` (maximum 92 MiB)

Maximum total transfer is 125 MiB. Do not use the `current/` alias or another
mirror. The existing public key
`/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/UEC-image-signing-key.asc`
is the only accepted key input; its SHA-256 and prior provenance are recorded
in the v1 image-construction packet. Require the full signing fingerprint to
equal `D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`; do not fetch another key.

Download into
`/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-inputs/`. Use the
existing local `rust:1.93.1-bookworm` image, repo digest
`sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
with `--pull=never --network=none` for GPG and SHA-256 verification. Put the
fresh GPG home in the container's disposable `/tmp`, not in a host mount.
Mount the downloaded files and existing public key read-only; use no writable
host mount, device, or socket.

## Allowed local processing

1. Download each of the four named files once, with no automatic retries,
   redirects, or alternate URLs. Record HTTP status, actual byte count, and
   URL for each; stop if the combined transfer exceeds 125 MiB.
2. In a fresh container-local GPG home, import only the existing public key,
   print and compare its full fingerprint, verify `SHA256SUMS.gpg` against
   `SHA256SUMS`, then verify both boot files against that signed list. Stop on
   a signature, fingerprint, path, or digest mismatch. The earlier root
   archive signature result does not substitute for this subdirectory's
   checksum signature.
3. Read file metadata only. Do not execute, unpack, modify, mount, or attach
   the kernel or initrd. Record their SHA-256 hashes, sizes, image/build
   identity, and exact temporary custody paths.
4. Remove the container and its GPG home after capturing the results. Keep the
   already verified root archive and RAW image, and retain these boot inputs
   only if the combined logical size of all retained artifacts remains below
   5 GiB.

## Limits, rights, and claim ceiling

- One GET per named file; at most 125 MiB total network transfer; one GPG
  signature verification and one checksum verification.
- No other host, mirror, keyserver, package repository, or user data; no
  package installation; no source-binary execution; no rootfs modification;
  no image conversion; no VM creation or boot; no networking, shared-folder,
  device, or socket attachment; no provider or worker calls.
- Canonical's local internal-use boundary from the prior packet applies.
  Ubuntu and package bytes are not authorized for redistribution; no artifact
  enters tracked files or a release.
- Success establishes only the authenticated identity and local custody of
  the selected kernel and initrd bytes. It does not establish their
  compatibility with the RAW disk, Apple Virtualization boot behavior,
  cgroup/resource enforcement, no-network operation, cancellation, cleanup,
  capacity, platform support, or production readiness.

Assurance target: `INTERNAL_PASS` for the exact source authentication and
metadata scope if every gate succeeds. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`. Any failure remains `INCONCLUSIVE`, with
no retry under this slice.

The next gate requires a separate packet binding these exact digests to one
local Apple Virtualization boot attempt, with fixed guest memory/vCPU,
root-device and console contracts, no network or shared paths, bounded
lifecycle, cancellation/crash cleanup, and host-plus-guest capacity limits.

## Result record

Status: `INTERNAL_PASS` for the exact local source-authentication and metadata
scope. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`.

- Four exact HTTPS GETs returned HTTP 200 without redirects. `SHA256SUMS`:
  1,587 bytes; `SHA256SUMS.gpg`: 833 bytes; arm64 vmlinuz: 23,831,432 bytes;
  arm64 initrd: 91,123,407 bytes. Total transfer was 114,957,259 bytes,
  below 125 MiB.
- The existing public key matched full fingerprint
  `D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`. The checksum-list signature
  verified as good. GPG noted that this fresh temporary keyring had no
  ultimately trusted keys; the exact fingerprint match is the accepted trust
  check. No unsafe-home warning was emitted.
- `SHA256SUMS` SHA-256:
  `5acb8213afd5d74ddf35044d618f04f989f6daaac77ff6f15c5c2fd03a1272a8`.
- `SHA256SUMS.gpg` SHA-256:
  `58e7b61a7e0d8bcdadb321696d99ec0d8232c5a661ca85c5f0a064e1fa74558d`.
- vmlinuz SHA-256:
  `0bad41327e67c7c79d79b7e5f0ee833d0202be2ddf0f7cbbb984b4cb9c94e108`.
- initrd SHA-256:
  `65fa960053965fbcbba70a36e060965f2f4eb46124e8f0d9c0984250edc61117`.
- Both boot-file hashes matched the signed checksum list. Tool image:
  `rust:1.93.1-bookworm`, repo digest
  `sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
  `linux/amd64`. The four downloaded files remain only in
  `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/boot-inputs/`; the key
  remains in the parent temp directory. No kernel or initrd was executed.

Claim ceiling: authentication and local custody of the dated boot-input
bytes only. The next blocked gate is a new exact local VM-boot packet binding
these digests to the RAW root image and fixing resource, device, no-network,
cancellation, cleanup, and capacity limits. This slice does not authorize VM
instantiation or boot.

## References

- [Ubuntu dated arm64 cloud-image snapshot](https://cloud-images.ubuntu.com/resolute/20260927/)
- [Ubuntu cloud-image checksum verification](https://ubuntu.com/docs/public-images/public-images-how-to/verify-image-checksum/)
- [Ubuntu cloud-image artifact formats](https://ubuntu.com/docs/public-images/public-images-reference/artifacts/)
- [Canonical Intellectual Property Rights Policy](https://canonical.com/legal/intellectual-property-policy)
