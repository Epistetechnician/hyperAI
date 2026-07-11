# End-to-end acceptance status

Status date: 2026-07-11. `COMPOSED_TESTED` means the scenario crosses its
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
4. Incomplete observation — `CONTRACT_TESTED`. Source and resolution tests block
   FALSE; composed unresolved-to-preflight refusal remains open.
5. Runner error — `PARTIAL_COMPOSED`. Scripted ERROR reaches unresolved/no
   effect; live Docker timeout/output-flood classification is runner-level only.
6. Nondeterministic validator — `CONTRACT_TESTED`. Divergent replay contracts
   block resolution; composed evidence and settlement refusal remain open.
7. Dispute with counterexample — `PARTIAL_CONTRACT`. Counterexample disputes
   block finalization, but pre-cutoff candidate ingestion, recomputation, and
   superseding resolution are not implemented as one path.
8. Tampered evidence — `CONTRACT_TESTED`. Standalone verification identifies
   digest failure; exported-file mutation through the CLI remains open.
9. Stale market binding — `CONTRACT_TESTED`. Metadata drift fails preflight
   without rebinding.
10. Settlement crash recovery — `SPLIT_CONTRACTS`. Durable restart recovery and
    mock backend lost-receipt recovery pass separately; one joined path remains
    open.
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

The highest-impact remaining functional gap is scenario 7 because the domain
does not yet provide terminal dispute decision, recomputation, and supersession
as one append-only workflow. The highest-impact API security gap is scenario 12
because authorization denial is not yet connected to durable audit persistence.
