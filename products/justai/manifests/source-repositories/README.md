# Source repository inventory and migration boundary

Inventory state slice: `justai-governed-discovery-monorepo-v1`.

Audit date: 2026-09-09. The inventory records the repository checkout, branch, exact HEAD, working-tree state, declared validation commands, rights/provenance status, allowlisted reference paths, excluded material, and JustAI owner slice. Source repositories are not modified, deleted, archived, or treated as migrated code.

`manifest.json` is the machine-readable source of truth for this inventory. A path in `reference_paths` is a provenance pointer, not a permission to copy. Code is reimplemented clean-room unless the manifest explicitly records a later rights gate. Generated runs, datasets, traces, weights, credentials, caches, nested repositories, provider runners, deployment state, and active services remain excluded.

Important limitations:

- The audit is a local checkout audit. Remote synchronization, independent review, scientific assessment, hosted enforcement, and production readiness were not established.
- A source commit does not transfer its authorization, reviewer identity, custody root, signer, policy, release authority, or kill path.
- The `composed-zk-benchmark-os/semantica` directory is a nested untracked repository and is explicitly excluded.
- The Statebook source is the documentation-only archive `statefulai`; no `/Users/shaanp/Documents/GitHub/statebook` checkout exists.
