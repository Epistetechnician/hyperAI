# Provenance and custody

State slice: `justai-custody-v1`.

Every governed record carries an exact state slice, proposal digest, claim ceiling, repository-relative source path, source commit, source-byte digest, rights basis, custody reference, disposition, and evidence references. Source identities and authorizations do not transfer with a file.

The source inventory is a read-only audit of the named checkouts. Dirty work, nested repositories, generated runs, traces, datasets, weights, credentials, caches, provider runners, and deployment outputs are excluded. `statefulai` is retained as a documentation-only Statebook archive under the `statebook` alias.

Canonical serialization is Rust-owned and uses RFC 8785-compatible JCS. JSON is parsed with recursive duplicate-key rejection before typed mapping. Integration handoff seals are accepted only when an externally supplied Ed25519 signature verifies against the exact result digest and seal identifier.
