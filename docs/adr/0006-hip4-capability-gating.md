# ADR 0006: HIP-4 capability gating

Status: accepted for implementation.

HIP-4 is an outer settlement adapter. Bindings use network plus outcome ID,
side order and labels, quote token, raw metadata digest, and frozen HSAI claim
digest. Active and settled metadata reads are supported. Mock settlement and a
manual HSAI handoff are supported. Testnet resolution publication is
`UNSUPPORTED_UNVERIFIED`; mainnet mutation is structurally disabled.
