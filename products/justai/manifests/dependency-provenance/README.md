# Dependency provenance

State slice: `justai-contracts-v1`.

The canonical implementation uses the existing Rust dependency set already pinned by `Cargo.lock`: `serde`, `serde_json`, `serde_jcs`, `sha2`, `hex`, `ed25519-dalek`, and `thiserror`. The new broker uses the same data/cryptographic primitives. The subprocess harness uses only `serde` and `thiserror`. The TypeScript package directories have no runtime dependencies and export type-only projections or small deterministic guards.

No source repository implementation, model, dataset, trace, generated output, credential, provider SDK, or vendored dependency was copied into JustAI. Rights remain source-specific and are recorded in [`../source-repositories/manifest.json`](../source-repositories/manifest.json).
