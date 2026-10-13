# HyperAI Phase 2 macOS Guest Image Construction v1

State slice: [`hyperai.bazaar.data_macos_guest_image_construction.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_image_constructionv1).

Status: one-run local synthetic guest-artifact acquisition and construction
scope. It does not authorize VM instantiation or boot.

## Baseline and purpose

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree. The source/format check identified the
date-specific official Ubuntu 26.04 arm64 root filesystem archive as the
candidate input, but did not authenticate or download it. This slice verifies
the exact source bytes, constructs a RAW ext4 filesystem candidate using an
already available pinned tool image, and inspects whether the rootfs includes
the expected boot files. It stops short of proving a bootable VM.

## Frozen inputs and custody

- Official date-specific directory:
  `https://cloud-images.ubuntu.com/resolute/20260927/`
- Archive: `resolute-server-cloudimg-arm64-root.tar.xz` (directory listing:
  241 MiB).
- Signature inputs: `SHA256SUMS` and `SHA256SUMS.gpg` from the same directory.
- Public signing key source:
  `https://keyserver.ubuntu.com/pks/lookup?op=get&search=0xD2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`.
  Accept it only if its full fingerprint is exactly
  `D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`.
- Verification/build image: existing local
  `rust:1.93.1-bookworm`, repo digest
  `sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
  invoked with `--pull=never --network=none --platform linux/amd64`.
- Temporary host directory:
  `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`; do not place any
  downloaded or derived artifact in the repository.

The only permitted network reads are HTTPS GETs for the three named files and
the exact key URL above, with redirects disabled. Total transfer is capped at
300 MiB. No other host, package repository, keyserver, provider, or user data
is permitted. Retain the verified archive and derived RAW candidate in the
named temporary directory for the separately gated boot-lifecycle design;
never copy them into tracked files or a release artifact.

## Allowed local processing

1. Download the named files once, without automatic retries, into the
   temporary directory. Record HTTP status, byte count, and final URL.
2. Import the public key into a fresh temporary GPG home, print and compare
   its full fingerprint, verify `SHA256SUMS.gpg` against `SHA256SUMS`, then
   verify the root archive against that authenticated checksum list. Stop on
   any mismatch; do not substitute another key, source, or release.
3. Inspect the tar member list before extraction. Reject absolute paths,
   parent traversal, unexpected top-level roots, or unapproved special files.
   The only approved special entries are `dev/console`, `dev/full`,
   `dev/mapper/control`, `dev/null`, `dev/ptmx`, `dev/random`, `dev/tty`,
   `dev/urandom`, and `dev/zero`, the nine standard `/dev` entries recorded
   by the inspection. Extract only into a new empty directory in
   the disposable no-network container, without restoring source ownership or
   permissions. Run that container as root only inside the container, with
   `--cap-drop=ALL --cap-add=MKNOD`, read-only `/inputs`, and no host device
   mounts. Do not grant host privileges or mount any writable host path into
   the container. Keep the extraction tree and candidate image in disposable
   container storage; export only the completed image with `docker cp` to the
   named temporary host directory after read-only inspection.
4. Record extracted size and inspect `/boot` and the root filesystem layout.
   If the archive lacks the required kernel or boot files, record that fact;
   do not fetch replacements under this slice.
5. If the source tree fits within the frozen limit, create one 4-GiB RAW ext4
   filesystem image in disposable container storage from that tree using the
   pinned image's existing `mkfs.ext4`. Run read-only filesystem inspection
   and record the output digest before exporting the image with `docker cp`.
   Do not mount it on the host or attach it to a VM.
6. Retain the source archive and derived image only in the temporary directory
   until a later VM-lifecycle decision; record hashes and custody. Remove the
   container, GPG home, extracted tree, and transient verification files after
   evidence capture. Preserve the archive and RAW image only if both digests
   verify and total retained bytes remain below 5 GiB.

## Resource and stop limits

- Maximum network transfer: 300 MiB.
- Maximum extracted tree: 2 GiB; stop before creating the RAW image if exceeded.
- Maximum RAW virtual size: 4 GiB.
- Maximum retained artifact bytes: 5 GiB.
- One download attempt per file; one checksum/signature verification; one
  extraction; one RAW image construction; one read-only image inspection.
- Do not install host or container packages, run source binaries, boot a guest,
  read a dataset, attach networking/shared folders/sockets, contact a provider,
  or change any product code or dependency.

## Rights and claim ceiling

Canonical's current IPRights Policy permits internal use and modification of
Ubuntu; modified Ubuntu redistribution has additional conditions, and package
licenses remain applicable. The artifact is solely for local internal
synthetic qualification and must not be redistributed. No Bazaar source,
fixture, or data is involved.

Successful verification proves only that the local archive bytes match an
authenticated Ubuntu checksum list under the documented key fingerprint.
Successful RAW construction proves only that the local tool produced a
readable ext4 filesystem from that root tree. Neither result proves kernel
compatibility, boot, cgroup/resource enforcement, DataFusion isolation,
cancellation, cleanup, host-plus-guest capacity, security, or product support.

The next VM packet must freeze kernel/initramfs identity, root-device boot
arguments, guest memory/vCPU, cgroup limits, no-network device topology,
bounded IPC, cancellation/OOM/crash behavior, cleanup, and combined capacity
before a VM is instantiated.

## Result record

Status: source authentication and archive inspection passed; the single RAW
build attempt failed before a complete image was produced. This slice
authorizes no retry. Assurance is `INCONCLUSIVE` for guest-image construction.
Reviewer status is `NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`.

The partial image is not a candidate. Detailed digests, tar inspection,
failure, custody, limitations, and next gate are recorded below.

HTTP 200 responses, with no redirects, came from the exact approved URLs:
`SHA256SUMS` (8,492 bytes), `SHA256SUMS.gpg` (833 bytes), signing key
(3,325 bytes), and root archive (252,430,860 bytes). Total transfer was
252,445,510 bytes, under 300 MiB.

- `SHA256SUMS` SHA-256:
  `5733fd3e705bf581bd9dac60e5e592c5e55128f7dd835f5ed0af0c3fbbecbeb5`.
- `SHA256SUMS.gpg` SHA-256:
  `8ddf43a55e2fca04864aba2d277c93da01adf9c9a1bb389770d310d0407c44d2`.
- Signing key SHA-256:
  `927138de292fac4d861105a052cf56e9c5921987da385c844d88f34d19b39ab0`.
- Root archive SHA-256:
  `6ed455eb3a048eab4ff7cc08fe9f4126875b82c06260117c2ec1cddc391ad0d7`.

GPG reported full fingerprint
`D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`; the signature was good and the
archive checksum matched. GPG also emitted an unsafe-home-ownership warning
for `/work/gnupg`. The container inspection found `/work` root-owned and the
`gnupg` child user-owned; the precise cause of GPG's warning was not isolated.
Record these checks with that warning, not as warning-free.

Tar inspection found 54,807 members and 1,187,879,532 regular-file logical
bytes. All member paths were relative, had no parent traversal, and were
rooted at standard Linux directories: `bin`, `boot`, `dev`, `etc`, `home`,
`lib`, `media`, `mnt`, `opt`, `proc`, `root`, `run`, `sbin`, `snap`, `srv`,
`sys`, `tmp`, `usr`, and `var`. The nine special entries were exactly
`dev/console`, `dev/full`, `dev/mapper/control`, `dev/null`, `dev/ptmx`,
`dev/random`, `dev/tty`, `dev/urandom`, and `dev/zero`. `/boot` contains zero
regular files; the archive supplies no kernel or initramfs, and none were
fetched.

Extraction completed inside the no-network container. `du` reported
1,211,021,884 apparent bytes and 1,319,845,888 allocated bytes, with a
permission warning for `/var/lib/snapd/void`; treat those as incomplete
extracted-tree measurements. The RAW attempt used `mkfs.ext4` 1.47.0 and
failed at `Copying files into the device` with `Permission denied while
scanning directory "void"`. The container process had `MKNOD` only and could
not traverse that mode-restricted directory. The partial 4-GiB image was not
inspected or exported as a candidate.

Tool image: `rust:1.93.1-bookworm`, repo digest
`sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
platform `linux/amd64`. The authenticated archive remains in
`/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`; no derived image is
retained. Remove the failed container and its partial image after capturing
this evidence.

Claim ceiling: authenticated Ubuntu bytes and bounded archive inspection
only. No ext4 candidate, boot, isolation, resource enforcement, cgroup,
DataFusion, capacity, platform support, or production claim is established.
Any continuation needs its own named slice and must preserve this failed
attempt in the record.

## References

- [Ubuntu 26.04 dated arm64 snapshot](https://cloud-images.ubuntu.com/resolute/20260927/)
- [Ubuntu public-image checksum verification](https://ubuntu.com/docs/public-images/public-images-how-to/verify-image-checksum/)
- [Canonical Intellectual Property Rights Policy](https://canonical.com/legal/intellectual-property-policy)
- [Apple Virtualization disk-image formats](https://developer.apple.com/documentation/virtualization/vzdiskimagestoragedeviceattachment)
