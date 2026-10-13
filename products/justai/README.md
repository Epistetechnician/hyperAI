# JustAI

JustAI is a public-interest, nonviolent AI observatory and federation. It publishes signed, content-addressed evidence bundles about AI incidents, bounded claims, and institutional commitments. The public product is read-only and static.

The system is not an offensive cyber network, surveillance system, coercive enforcement body, autonomous agent swarm, or general capability-scaling platform. It does not authorize access to third-party systems or turn local tests into proof of alignment, safety, or production readiness.

## Repository status

The first implementation slice is `justai-observatory-contracts-v1`:

- Rust owns canonical JSON parsing, duplicate-key rejection, digests, signatures, release validation, and state transitions.
- Python performs bounded, read-only local source normalization.
- TypeScript recomputes release digests, verifies Ed25519 release signatures, and renders valid bundles into deterministic static HTML.
- The Rust assembler creates an immutable `<content_digest>/release.json` bundle; the independent Python/OpenSSL validator checks it before the renderer runs.
- Protected submissions, live model execution, open-internet evaluation, and distributed compute are intentionally not implemented in v1.

## Commands

```text
pnpm run test:focused
pnpm run verify:full
pnpm run lint
```

The Rust CLI is available with Cargo:

```text
cargo run -p justai-contracts -- validate path/to/release.json
cargo run -p justai-contracts -- digest path/to/record.json
cargo run -p justai-contracts -- sign input.json output.json key-id seed-hex
cargo run -p justai-contracts -- assemble input-dir output-dir publisher-id created-at release-id key-id seed-hex
```

The signing command is for controlled release preparation. Private seed material must never be committed, logged, or placed in a public bundle.

Content digests exclude only generated `content_digest` and `signature_refs` fields. This prevents circular signing while keeping all substantive release content digest-bound. Raw source-byte digests remain separately recorded in `source_digests`.

The integrated publisher is `python3 tools/publish_release.py`. It assembles with Rust, validates independently with Python/OpenSSL, and renders only after both checks pass.

The `justai-evidence-bridge-v1` slice extends this path with metadata-only artifact manifests, mechanically equivalent public/restricted evaluation receipts, signed reviewer-manifest bindings, disclosure decisions, bounded claim projections, and artifact-to-policy traceability. See [docs/evidence-bridge.md](docs/evidence-bridge.md).

The repeatable public-only pilot is under `fixtures/pilot/input`; it is locator-only and explicitly carries no external incident or safety claim.

The governed monorepo slice is `justai-governed-discovery-monorepo-v1`. Its canonical loop and package ownership are documented in [`docs/architecture/monorepo.md`](docs/architecture/monorepo.md) and [`docs/architecture/decision-loop.md`](docs/architecture/decision-loop.md). Applications and packages are non-authorizing: the model proposes, validation and human disposition remain explicit, bounded local quorum may replace external-independent validation for local replay only, and `Unknown`/`quarantine` are valid terminal dispositions.

The source inventory and migration manifest are under [`manifests/source-repositories`](manifests/source-repositories) and [`manifests/migration`](manifests/migration). Source repositories remain separate and preserved. No private implementation details, credentials, generated outputs, traces, datasets, weights, provider runners, or active-service state are vendored.

## Claim ceiling

Every publication states its evidence status and claim ceiling. `Unknown`, `NoCandidate`, `Contested`, and `Withdrawn` are valid outcomes. Signed records are immutable; corrections are new linked records.

## Existing repositories

Existing research and runtime repositories remain independently governed. Future adapters may consume exact, pinned, licensed source manifests, but this repository does not copy their code, data, prompts, traces, or generated artifacts.
