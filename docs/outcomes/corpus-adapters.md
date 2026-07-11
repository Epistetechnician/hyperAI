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
resource-limit overflow without invoking Git, network, or executables. The v3
materialization manifest digest-binds conservative media types and observed
execute bits. Profiles
also receive bounded inert inspection of `.gitmodules`, strict Git LFS pointer
metadata, archive format signatures, and static Dockerfile/Containerfile base
image references. Unsafe paths or URLs, dynamic or mutable image references,
missing submodule trees, unverified submodule commits, and unmaterialized LFS
objects become digest-bound readiness blockers. Archives and container recipes
are classified only; nothing is extracted or executed. Profiles do not yet
clone upstream bytes, retrieve submodule/LFS objects, extract archives,
materialize images, or prove a runnable verifier.

`fixtures/corpora/recorded-challenge-overlays.json` supplies a sanitized,
deny-unknown reviewed overlay bound to an exact v3 materialization digest. The
compiler requires a complete one-to-one path partition across solver, runtime,
verifier, contamination, build, and descriptive roles; checks declared media
type and execute bits; carries special-object blockers forward; prevents
solver/excluded digest aliasing; and emits digest-bound package, solver-view,
and item-rights records. Repository-level licensing cannot promote missing
item-level permission. Compilation performs no execution and structurally
rejects `SETTLEMENT_READY`. External profiles therefore
remain `EVALUATION_ONLY` or `CATALOG_ONLY` and cannot drive settlement.
