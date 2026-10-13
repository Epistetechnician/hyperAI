# Runtime configuration

`hyperintel-runtime-config` validates one explicit local JSON document for the
`LOCAL_HERMETIC` profile. It does not supply defaults, merge environment
variables, interpolate values, watch for changes, fetch remote configuration,
or create directories or databases.

## V1 document

The committed example is [`config/examples/local-hermetic.json`](../config/examples/local-hermetic.json).
Every field is required and unknown fields fail. `runtime_id` uses the canonical
HyperIntel protocol-ID grammar. The two database paths must be absolute,
lexically safe, and distinct. Each connection count is limited to `1..=16`.

The example paths are operator-selected deployment examples. Validation does
not establish that a path exists, is writable, has safe ownership or
permissions, is encrypted, is backed up, or is rollback resistant. The loader
does not open either database.

The input budget is 65,536 bytes. `load_runtime_configuration_v1` reads only the
explicit path supplied by its caller and reads at most one byte beyond that
budget before rejecting oversize input. `parse_runtime_configuration_v1` has no
filesystem effect.

After validation, the crate computes a canonical, domain-separated
configuration digest. The digest binds the document, not the filesystem,
runtime process, operator, or external authority.

## Secret references

`SecretReferenceV1` is a reusable logical reference for later service-specific
configuration:

```json
{
  "schema_version": "hyperintel.secret-reference.v1",
  "provider_id": "fixture",
  "handle_id": "provider-receipt-signer-alpha",
  "version_digest": "1111111111111111111111111111111111111111111111111111111111111111"
}
```

It stores only canonical provider and handle identifiers plus a nonzero SHA-256
digest of an immutable provider-issued version identifier or version-metadata
record. The digest must never be computed from secret bytes or other secret
material. Logical aliases such as `latest`, `current`, `active`, `default`, or
provider-specific names cannot be decoded as the fixed digest. Unknown fields
such as `value`, `token`, `private_key`, or `material` fail decoding. Debug
output redacts the handle and version digest.

This crate contains no resolver. It reads no environment variable, secret file,
credential, KMS, or HSM. A reference neither establishes that a secret exists
nor grants authority to use it.

## Claim boundary

A validated V1 document establishes only that the supplied local JSON matches
the bounded schema and invariants at parse time. It grants no network,
provider, credential, wallet, chain, settlement, custody, funds, production, or
deployment authority.
