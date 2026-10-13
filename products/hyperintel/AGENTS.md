# HyperIntel Agent Contract

## Product boundary

HyperIntel is an inference-native marketplace and assurance foundation. It is
not an outcome market, prediction market, token issuer, production exchange, or
proof of model correctness.

Every mutation must name exactly one state slice from `docs/state-slices.md`.
Do not infer mutation scope from imports or file names.

## Required workflow

1. Measure twice, cut once: inspect the current state, name the slice, and list
   the intended files before editing.
2. Preserve unrelated work. Never copy uncommitted source-repository content.
3. Port only from the exact commits recorded in `docs/provenance.md`.
4. Keep inference-native names and semantics. Do not import outcome resolution,
   binary market binding, HIP-4, ClaimTruth, or outcome settlement types.
5. Unsupported inputs fail closed. Budget exhaustion, missing evidence, stale
   state, or an inconclusive verifier never imply acceptance.
6. Keep the repository clean: no temporary files, dead code, dead files, or
   unnecessary folders.
7. Run `pnpm run lint:fast` during iteration, `pnpm run test:focused` for the
   vertical slice, `pnpm run verify:contracts` for protocol contracts, and
   `pnpm run lint` plus `pnpm run verify:full` before release handoff.

## Evidence ceiling

Local tests, signed fixtures, digests, attestations, and handwritten formal
models are scoped local evidence only. They do not establish production
readiness, semantic correctness, model truthfulness, confidential execution,
independent verification, external settlement, or value movement.

## External effects

Normal tests must be hermetic. Network access, provider credentials, wallet
keys, EVM submission, HyperCore actions, custody, and mainnet mutation require a
separate explicitly authorized state slice and operator acknowledgement.
