# JustAI governed discovery monorepo v1 task list

State slice: `justai-governed-discovery-monorepo-v1`

This is the pre-mutation task list. Source repositories remain read-only and are not deleted or archived by this migration.

## Inventory and custody

- [x] Record current branch, commit, dirty/untracked state, validation commands, licenses, provenance, generated artifacts, credentials, traces, datasets, weights, and active-service indicators for every named source.
- [x] Create a source-repository inventory and migration manifest with exact source commits, file-level provenance, safe dispositions, excluded material, and one owner per migrated capability.
- [x] Preserve the existing target dirty slice in `crates/justai-contracts/src/records.rs`; reconcile it without overwriting unrelated work.

## Contracts and serialization (`justai-contracts-v1`)

- [x] Define pure-data DecisionRecord, SerendipityRecord, IntuitionReview, DissentRecord, PromotionDecision, UnknownRecord, EvidenceManifest, IntegrationResult, and IntegrationHandoffSeal records.
- [x] Require state slice, proposal digest, claim ceiling, provenance, disposition, and evidence references on every governed record.
- [x] Enforce canonical serialization, recursive duplicate-key rejection, digest validation, external-seal verification, claim ceilings, proposal-only model roles, and fail-closed dispositions.

## Governed loop and paths

- [x] Implement proposal -> serendipity -> intuition -> external-independent or local-quorum validation -> promote/Unknown/quarantine -> retrospective as pure deterministic orchestration.
- [x] Implement fast-path eligibility and slow-path escalation for disagreement, novelty, missing/stale evidence, irreversible actions, safety concerns, ambiguous rollback, and infrastructure uncertainty.
- [x] Persist intuition and dissent as falsifiable evidence-bearing records; preserve alternatives and never force slow-path acceptance.

## Monorepo boundaries

- [x] Add the requested applications, packages, crates, protocols, documentation, manifests, and focused tests with explicit ownership and state-slice labels.
- [x] Add non-authorizing platform/operations adapters, custody/replay/rollback controls, subprocess quarantine controls, project isolation, reproducible export, and infrastructure-state classification.
- [x] Add inspired-inefficiency budgets and separate dimensions without collapsing them into a single optimization score.
- [x] Document controls for counter-serendipity, hyperfocus, self-amplifying cascades, skill erosion, perverse feedback, data deluge, and monoculture.

## Verification and reporting

- [x] Add contract, custody, integration, regression, slow/Unknown, intuition, isolation, manual fallback, recovery, rollback, and focused migrated-package tests.
- [x] Run focused validation first, then `pnpm --ignore-workspace run lint`, then full repository validation only after focused validation passes.
- [x] Report implementation, local validation, independent review, scientific evidence, hosted enforcement, production readiness, and authorization as separate statuses.
