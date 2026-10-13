# JustAI governed monorepo architecture

State slice: `justai-governed-discovery-monorepo-v1`.

JustAI is a boundary-preserving monorepo. Rust owns canonical pure-data contracts, duplicate-key rejection, content digests, signatures, and external-seal verification. Python owns bounded local orchestration and recovery demonstrations. TypeScript packages are type-only projections or deterministic presentation guards. Applications do not become authorities by importing a contract.

## Ownership

| Area | Owner | State slice | Constraint |
| --- | --- | --- | --- |
| Decision, evidence, promotion, Unknown, and handoff contracts | `crates/justai-contracts` | `justai-contracts-v1` | Pure data; model is proposal-only |
| Proposal, alternative, intuition, validation, and retrospective loop | `apps/discovery` | `justai-discovery-v1` | External-independent or bounded local-quorum validation; no model/evaluator/policy/custody selection |
| Project isolation, aggregate-only export, and synthetic hosted status | `apps/platform` | `justai-platform-v1` | Export omits record payloads; hosted runtime is non-authorizing |
| Replay-safe receipts and custody journals | `crates/security-broker` | `justai-custody-v1` | No accepted-evidence write |
| Fresh-directory subprocess quarantine | `crates/subprocess-harness` | `justai-operations-v1` | Rejected outcomes remain observable |
| Manual fallback and rollback rehearsal | `apps/operations` | `justai-operations-v1` | Unknown/quarantine on ambiguity |
| Multi-instance local quorum and bounded self-verification | `tools/local_quorum.py`, `tools/multi_instance_self_verify.py` | `justai-local-quorum-v1`, `justai-infrastructure-v1` | Same-operator replay only; fixed `local-replay` ceiling |
| Infrastructure reachability classification | `HostedRuntime.status` | `justai-infrastructure-v1` | Reachability, connector health, auth, app correctness, and deployment are separate |

The source mapping, exact checkout commits, file-level reference paths, dirty-state notes, and exclusions are in [`../../manifests/source-repositories/manifest.json`](../../manifests/source-repositories/manifest.json). The migration manifest assigns one owner to each capability.

## Authority rule

The model may provide a proposal and confidence. It cannot select or modify the evaluator, validator, policy, evidence ledger, custody root, signer, release authority, or kill path. Validation and human disposition are caller-supplied records; the local loop supports either external-independent validation or bounded same-operator local quorum. Local quorum requires distinct instance and machine evidence, remains capped at `local-replay`, and cannot establish organizational independence. Local green tests, synthetic replay, observer health, or completed agent work do not upgrade any claim ceiling.
