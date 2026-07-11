# Hyperliquid HIP-4 adapter

Status dated 2026-07-10. HIP-4 is a settlement backend, not claim-domain logic.

## Capability boundary

- Recorded metadata: implemented and fixture-tested. Exact side order and labels,
  quote token, raw bytes, and network-scoped outcome ID are retained.
- Live metadata: opt-in `live-readonly` Cargo feature. It performs the documented
  bounded `{"type":"outcomeMeta"}` POST, then selects an outcome ID locally; it
  never signs or posts an Exchange action. The official page says testnet-only,
  while dated direct observations found both networks responding. This rollout
  mismatch remains explicit capability uncertainty. Default tests use no network.
  The build does not implement the observed `settledOutcome` response client or
  a process-wide rate limiter, so live mode remains evaluation-only.
- Asset IDs: checked `10 * outcome_id + side`, `#encoding`, `+encoding`, and
  `100000000 + encoding`; only binary side indices 0 and 1 are accepted.
- Binding: explicit claim and metadata digests plus operator-supplied TRUE/FALSE
  side mapping. Name matching is forbidden.
- Mock publish: implemented, idempotent, and reconcilable after the caller loses
  the receipt.
- Manual handoff: implemented as a signed handoff artifact. It is not settlement.
- Testnet publish: `UNSUPPORTED_UNVERIFIED`. Current primary sources did not
  document an exact resolution action, signing domain, authorized broadcaster
  role, and authorization/replay model.
- Mainnet publish: structurally disabled.

Preflight re-reads the metadata digest and blocks stale bindings, non-final
resolutions, invalidated bindings, and unsupported capabilities. Outcome IDs are
never rotated or rebound automatically. HSAI attests its frozen claim; it does
not thereby become Hyperliquid’s authorized broadcaster.

Primary sources and access dates are in [references](../references.md).
