# HyperAI Phase 2 macOS Guest Source and Format Check v1

State slice: [`hyperai.bazaar.data_macos_guest_source_check.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_source_checkv1).

Status: documentation-only source, rights, and disk-format qualification. It
does not authorize downloading a guest image, installing conversion tooling,
converting a disk, or starting a VM.

## Baseline and purpose

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree. The preceding macOS Virtualization host
preflight passed on macOS 26.6.1 arm64, but did not use a guest image. This
packet determines whether a specific official arm64 Linux image has a rights
basis and a disk format that can be attached by Apple's Virtualization
framework, and whether the required verification/conversion tools already
exist locally.

## Frozen scope

- Inspect the current Phase 2 packet, Rust port subplan, host-preflight packet,
  and local tool availability without changing system packages or images.
- Read only official Canonical/Ubuntu release, image-verification, and
  intellectual-property pages, plus official Apple Virtualization storage
  attachment documentation.
- Record a date-specific candidate release/build path, architecture, image
  format, published checksum/signature locations, signer fingerprint, and
  rights limits when the official sources expose them.
- Compare the image format with Apple's documented attachable formats. If a
  conversion is required, record that as a blocker; do not install or run a
  converter in this slice.
- Retain no guest image, downloaded checksum file, key, or other source
  artifact. Record only public source links and the findings needed for the
  next packet.

## Included files

- `docs/compute-bazaar-data-macos-guest-source-check-v1.md`: this scope and
  findings.
- `docs/state-slices.md`: authorization, proof gates, and claim ceiling.
- `docs/compute-bazaar-phase-2-data-packet.md`: Phase 2 status and next gate.
- `docs/compute-bazaar-rust-port-subplan.md`: current Data qualification.

## Non-goals

No guest image or key download; package installation; use of a keyserver;
image conversion; VM instantiation or launch; network, disk, shared-folder,
or socket attachment; dataset access; runtime, dependency, API, signing,
packaging, or supported-platform changes; provider calls; external writes;
publication; or deployment.

## Proof gates and claim ceiling

The source lookup identifies a date-specific official build, but only a
verified signed SHA-256 digest can freeze the exact artifact bytes. The signer
and checksum method, local internal-use rights and redistribution limits,
Apple attachment formats, and current local tools are recorded. The bootable
QCOW2 candidate is incompatible with the direct attachment path, and a
bootable conversion from the root archive has not been demonstrated. A source
listing or authenticated checksum alone does not prove guest boot, security,
memory enforcement, cgroup behavior, DataFusion isolation, cancellation,
cleanup, combined capacity, or product support.

Claim ceiling: `PLANNING_ONLY` for source selection and format compatibility.
Reviewer status: `NOT_REQUESTED`. Reproducibility: `NOT_APPLICABLE`. No image
bytes are placed in repository or scratch custody.

## Execution result (2026-10-01)

The dated official Ubuntu 26.04 daily snapshot at
[`resolute/20260927/`](https://cloud-images.ubuntu.com/resolute/20260927/)
lists an arm64 root filesystem archive
`resolute-server-cloudimg-arm64-root.tar.xz` (241 MiB) and a 902 MiB
`resolute-server-cloudimg-arm64.img` described as a QCOW2 UEFI/GPT disk image.
The date directory identifies the selected build instead of relying on the
mutable `current/` alias, but it is not an artifact identity until its signed
SHA256 digest is verified. The listing provides `SHA256SUMS` and
`SHA256SUMS.gpg`; no image, checksum, signature, or key bytes were downloaded
in this slice.

Ubuntu's official verification instructions say to authenticate `SHA256SUMS`
with GPG, then check the image against that signed list. The documented UEC
Image Automatic Signing Key fingerprint is
`D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`. This is a published identity
reference only; the key and snapshot signature were not fetched or verified.

Canonical's current IPRights Policy permits internal use and modification of
Ubuntu, limits redistribution of modified Ubuntu images, and says package
licenses still apply. The proposed use is local internal synthetic
qualification only, with no image redistribution. Package-level license
review remains required before any built guest artifact is distributed.

Apple documents `VZDiskImageStorageDeviceAttachment` for RAW and ASIF disk
images; the listed QCOW2 image cannot be attached directly. The dated
root-filesystem archive is not itself a disk image. Read-only inspection of
the existing local `rust:1.93.1-bookworm` image (repo digest
`sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`)
found GPG/GPGV, SHA-256, `mkfs.ext4`, `mke2fs`, and `debugfs`; `qemu-img` is
not installed. The tool image is Linux/amd64 under local Apple Silicon
emulation. Constructing an ext4 RAW disk from the signed root archive is a
possible future path, but this slice did not download or inspect that archive,
construct a filesystem, or establish a bootable kernel/initramfs/root-device
combination.

Disposition: `INCONCLUSIVE` for a ready-to-boot guest artifact path; the
date-specific source location, documented signature method, internal-use
rights, Apple format requirements, and local tool availability are recorded,
but the source bytes are not cryptographically frozen. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_APPLICABLE`. Artifact custody is
empty: no image, key, checksum, or package bytes were acquired. Phase 2 stays
`NOT_ASSIGNED`.

Next gate: a new exact packet may authorize retrieval of only the dated
arm64 root archive and its official checksum/signature metadata, obtain the
public signing key from an explicitly pinned source and verify its fingerprint,
authenticate the checksum file, and verify the archive digest. It must also
freeze a no-network recipe for converting the root filesystem into a RAW disk
and resolving kernel/initramfs/root-device boot requirements. Do not launch a
VM until the archive is authenticated, the exact derived image is hashed, and
a separate guest-lifecycle packet binds network, memory/cgroup, IPC,
cancellation/OOM/crash behavior, cleanup, and host-plus-guest capacity.

## References

- Ubuntu's [dated Resolute 20260927 daily snapshot](https://cloud-images.ubuntu.com/resolute/20260927/) lists arm64 artifacts and signed SHA256 metadata.
- Ubuntu's [image checksum verification instructions](https://ubuntu.com/docs/public-images/public-images-how-to/verify-image-checksum/) document GPG verification and the UEC image-signing fingerprint.
- Canonical's [Intellectual Property Rights Policy](https://canonical.com/legal/intellectual-property-policy) covers internal use, modification, redistribution, trademarks, and component licenses.
- Apple's [`VZDiskImageStorageDeviceAttachment`](https://developer.apple.com/documentation/virtualization/vzdiskimagestoragedeviceattachment) documentation lists RAW and ASIF as supported disk image formats.
