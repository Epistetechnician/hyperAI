# End-to-end acceptance status

Status date: 2026-07-12. `COMPOSED_TESTED` means the scenario crosses its
required application boundaries in one automated path. `CONTRACT_TESTED` means
the relevant invariants have focused tests but are not yet one composed path.

1. Valid solution before deadline — `COMPOSED_TESTED`. Local fixture and opt-in
   Docker paths cover evidence, quorum, finality, one mock effect, reconciliation,
   and standalone bundle verification logic.
2. Invalid candidate followed by valid candidate — `COMPOSED_TESTED`. A real
   ledger snapshot leaves candidate one unresolved after FAIL; candidate two
   produces two PASS replays, FINAL TRUE, and one confirmed mock effect. Both
   candidates remain evidence; only candidate two is qualifying evidence.
3. No solution — `COMPOSED_TESTED`. Complete empty observation after deadline
   and grace reaches FINAL FALSE without settlement publication.
4. Incomplete observation — `COMPOSED_TESTED`. A post-grace local snapshot with
   an injected pagination gap remains UNRESOLVED with `ObservationIncomplete`.
   Portable evidence verifies, resolution proposal is refused, typed settlement
   eligibility requires a final resolution, and no intent or effect is created.
5. Runner error — `PARTIAL_COMPOSED`. Scripted ERROR reaches unresolved/no
   effect; live Docker timeout/output-flood classification is runner-level only.
6. Nondeterministic validator — `COMPOSED_TESTED`. PASS/FAIL executions for the
   same candidate and frozen environment remain UNRESOLVED with
   `ReplayDivergence`. Both manifests remain evidence, resolution proposal and
   settlement eligibility fail closed, and no intent or effect is created.
7. Dispute with counterexample — `COMPOSED_TESTED`. A provisional FALSE is
   refused by settlement preflight and blocked from finalization by an immutable
   counterexample dispute. A candidate observed and received before cutoff then
   produces two PASS replays, an accepted terminal decision, superseding quorum,
   FINAL TRUE, one idempotent mock effect, and confirmed reconciliation. The
   original record and both evidence views remain append-only artifacts.
8. Tampered evidence — `CONTRACT_TESTED`. Standalone verification identifies
   digest failure; exported-file mutation through the CLI remains open.
9. Stale market binding — `CONTRACT_TESTED`. Metadata drift fails preflight
   without rebinding.
10. Settlement crash recovery — `COMPOSED_TESTED`. One file-backed contract
    persists the intent before a real mock submission, discards the returned
    receipt, reopens SQLite, selects authoritative status lookup, recovers the
    exact receipt without resubmission, persists reconciliation, reopens again,
    and reaches complete durable state with one submission and one effect.
11. Wrong network/mainnet mutation — `CONTRACT_TESTED`. Backend, API, and CLI
    reject mainnet mutation before signer or network processing.
12. Unauthorized operation — `PARTIAL_CONTRACT`. Role-gated operations deny
    public readers without response leakage; durable denial audit is open.
13. Backward compatibility — `TESTED` by the workspace contract and full gates.
14. NYU agent-to-verifier path — `CONTRACT_TESTED` with pinned recorded harness
    data; no hosted or authenticated interaction occurs.
15. Corpus update isolation — `CONTRACT_TESTED` at claim identity, immutable pin,
    materialization digest, and overlay digest boundaries; a composed refresh
    workflow remains open.
16. Rights and capability gating — `CONTRACT_TESTED`. Missing item permission
    remains catalog/evaluation-only and cannot reach settlement readiness.

The highest-impact remaining API security gap is scenario 12 because
authorization denial is not yet connected to durable audit persistence. The
next compact functional gap is scenario 8 because standalone tamper detection
has not yet been exercised against an exported bundle through the CLI.
