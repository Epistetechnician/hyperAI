# HyperAI Phase 2 macOS Guest Image Continuation v1

State slice: [`hyperai.bazaar.data_macos_guest_image_continuation.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_image_continuationv1).

Status: one-run, local, offline continuation for the failed RAW image build in
[`hyperai.bazaar.data_macos_guest_image_construction.v1`](state-slices.md#hyperai-bazaar-data_macos_guest_image_constructionv1).
It does not authorize VM instantiation or boot.

## Baseline and purpose

Base revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus the
existing dirty/untracked worktree and the recorded failed v1 slice. Preserve
that failure as evidence. The source is the already authenticated Ubuntu
26.04 arm64 root archive at
`/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/resolute-server-cloudimg-arm64-root.tar.xz`,
SHA-256
`6ed455eb3a048eab4ff7cc08fe9f4126875b82c06260117c2ec1cddc391ad0d7`.
Its signature verification succeeded with the GPG ownership warning recorded
in the v1 packet; do not download or re-import its signing key under this
slice.

The sole v1 construction attempt stopped because the root process lacked
permission to traverse `/var/lib/snapd/void`. Use only the container's
`CAP_DAC_READ_SEARCH` to read/search such source paths while retaining their
archived mode bits. Do not chmod the root tree, grant `DAC_OVERRIDE`, or omit
the inaccessible path. Docker documents `DAC_READ_SEARCH` as bypassing file
read and directory read/search checks, while `DAC_OVERRIDE` also bypasses
write checks. This is a narrower permission correction than changing tree
modes or adding broad container capabilities.

## Frozen scope and custody

- Exact archive and SHA-256 are frozen above and in the v1 result record. One
  offline digest comparison is allowed before extraction. No network access,
  signature retry, key fetch, source substitution, or checksum-list rewrite.
- Tool image: existing local `rust:1.93.1-bookworm`, repo digest
  `sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
  `linux/amd64`; use `--pull=never --network=none`.
- Temporary host directory:
  `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`. Mount only the source
  directory read-only. Do not mount host devices, sockets, shared folders, or
  any writable host path. Keep extraction and image files in disposable
  container storage; export only the inspected complete image with `docker
  cp`.
- Run as container root with `--cap-drop=ALL --cap-add=MKNOD
  --cap-add=DAC_READ_SEARCH --security-opt no-new-privileges:true`. Do not use
  `--privileged`, `DAC_OVERRIDE`, or host root. The nine `/dev` special paths
  and complete safe member-path inventory are recorded in the v1 packet; do
  not repeat or broaden the archive listing under this slice.

## Allowed local processing

1. Compare the existing archive SHA-256 and byte size to the frozen v1 values.
   Stop if either differs or the file is absent.
2. Extract the archive once into a new empty directory in the pinned
   no-network container using `--no-same-owner --no-same-permissions`. Do not
   restore source ownership or chmod any extracted path. The added capability
   exists only so read-only traversal and population can preserve the
   mode-restricted tree.
3. Measure the fully readable extracted tree. Require no permission errors
   and no more than 2 GiB apparent size. Confirm `/boot` still contains no
   kernel or initramfs; do not fetch replacements.
4. Create one 4-GiB RAW ext4 candidate in disposable container storage with
   the pinned image's existing `mkfs.ext4`. Perform one read-only filesystem
   inspection, record logical and allocated sizes and SHA-256, then export the
   complete image with `docker cp` to the named temporary directory. Verify
   the exported byte size and SHA-256 on the host.
5. Retain the verified source archive and complete image only if their
   combined logical size is below 5 GiB. Remove the container and temporary
   GPG/build artifacts. Never attach, mount, or boot the image.

## Stop limits and non-goals

- One archive digest comparison, one extraction, one RAW image creation, and
  one read-only inspection. Any failure ends this slice; no fallback or retry.
- Maximum source archive: the exact 252,430,860-byte v1 file; no transfer.
- Maximum extracted apparent size: 2 GiB. Maximum RAW virtual size: 4 GiB.
- No package installation, source binary execution, provider calls, worker
  calls, datasets, code/dependency changes, VM boot, networking, IPC, external
  writes, publication, redistribution, or deployment.

## Rights and claim ceiling

The exact Ubuntu source and local-only rights boundary are inherited from the
v1 packet. This slice can establish only that the frozen authenticated archive
was read into a locally constructed ext4 candidate and that the candidate
passed the named read-only inspection. It cannot establish bootability (the
archive has no kernel/initramfs), isolation, cgroup enforcement, DataFusion
resource limits, cancellation, crash recovery, combined capacity, product
support, security, or production readiness.

Assurance target: `INTERNAL_PASS` only for the stated local construction and
filesystem-inspection scope if every gate succeeds. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. A failure remains
`INCONCLUSIVE` and does not authorize another attempt.

## Result record

Status: `INTERNAL_PASS` for the exact local extraction, ext4 construction, and
read-only filesystem-inspection scope. Reviewer status is `NOT_REQUESTED`;
reproducibility is `NOT_REPRODUCED`.

- The one offline source comparison passed: 252,430,860 bytes and SHA-256
  `6ed455eb3a048eab4ff7cc08fe9f4126875b82c06260117c2ec1cddc391ad0d7`,
  matching the already authenticated v1 archive. No network read or
  signature/key retry occurred.
- The v1 archive inventory was reused without relisting: 54,807 members,
  relative safe paths, 1,187,879,532 regular-file logical bytes, and the nine
  allowlisted `/dev` special entries. Extraction completed with
  `MKNOD` and `DAC_READ_SEARCH`, without a permission warning or mode rewrite.
  Apparent tree size was 1,211,021,884 bytes; allocated size was
  1,319,845,888 bytes. Both are below 2 GiB.
- `/boot` contains zero regular files, so no kernel or initramfs is present.
- `mkfs.ext4` 1.47.0 created one RAW ext4 image of exactly 4,294,967,296
  logical bytes (4 GiB). `dumpe2fs -h` reported a clean ext4 superblock with
  1,048,576 blocks at 4,096 bytes each. `e2fsck -f -n` completed all five
  passes and reported 54,696 files and 338,815 of 1,048,576 blocks used.
- In-container SHA-256 and exported-host SHA-256 both equal
  `6342a12d3fb4484d7a340fa647e574754d72aa39d535fb4a15ec61c0072b7fce`.
  The host file size is 4,294,967,296 bytes. Source plus image logical size is
  4,547,398,156 bytes, below the 5-GiB retained-artifact ceiling.
- Tool image: `rust:1.93.1-bookworm`, repo digest
  `sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31`,
  `linux/amd64`. The archive and image are retained only under
  `/private/tmp/hyperai-bazaar-ubuntu-arm64-image-v1/`; the container and
  in-container extracted tree are transient and must be removed after this
  record is captured.

Claim ceiling: `INTERNAL_PASS` establishes only local construction of a
readable ext4 filesystem from the authenticated root tree and the named
read-only integrity check. It does not establish kernel compatibility, guest
boot, isolation, cgroup/resource enforcement, DataFusion limits, cancellation,
cleanup, host-plus-guest capacity, security, or product support. Phase 2
remains `NOT_ASSIGNED`. The next blocked gate is a separate packet for a
rights-cleared kernel/initramfs and guest-lifecycle path; no VM may be started
under this slice.

## References

- [Docker container capabilities](https://docs.docker.com/engine/containers/run/)
- [Linux path resolution and DAC capabilities](https://www.man7.org/linux/man-pages/man7/path_resolution.7.html)
- [GNU tar extraction permissions](https://www.gnu.org/software/tar/manual/html_node/Setting-Access-Permissions.html)
