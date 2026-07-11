# Corpus adapters

The local fixture corpus is executable and hermetic. External corpora use one
`generic-git-corpus.v1` profile engine; repository-specific business logic is
data, not copy-pasted adapters. `config/corpus-profiles.json` pins immutable
commits observed on 2026-07-10 for NYU CTF Bench, CryptoHack, CTF Archives,
pwn.college, and the three public-author repositories.

The catalog test requires unique repositories and profile IDs, 40-hex commits,
reviewed roots, solver-view exclusion patterns, a shared engine, and a readiness
ceiling below `SETTLEMENT_READY`. Repository-level license statements never
promote ambiguous item-level rights.

Current external profiles validate catalog, pin, exclusion, rights, and
readiness metadata. Sanitized normalized recorded path manifests cover every
profile in `fixtures/corpora/recorded-profile-manifests.json`; they exercise
solver-view separation without copying upstream challenge content. The local
materializer can read deterministic regular-file manifests from an explicitly
provided absolute checkout whose expected and observed commits match the profile.
It rejects unsafe paths, symlinks, protected content, filesystem changes, and
resource-limit overflow without invoking Git, network, or executables. Profiles
also receive bounded inert inspection of `.gitmodules`, strict Git LFS pointer
metadata, archive format signatures, and static Dockerfile/Containerfile base
image references. Unsafe paths or URLs, dynamic or mutable image references,
missing submodule trees, unverified submodule commits, and unmaterialized LFS
objects become digest-bound readiness blockers. Archives and container recipes
are classified only; nothing is extracted or executed. Profiles do not yet
clone upstream bytes, retrieve submodule/LFS objects, extract archives,
materialize images, or prove a runnable verifier. They therefore
remain `EVALUATION_ONLY` or `CATALOG_ONLY` and cannot drive settlement.
