# Inspired inefficiency and separate measures

State slice: `justai-discovery-v1`.

JustAI predeclares budgets before assessment. The discovery loop uses frozen proposal, alternative-sampling, intuition-time, and recovery-attempt budgets. Callers report proposal and recovery usage explicitly; the loop rejects usage above the frozen maxima and records used-versus-budget values. Alternative candidates are caller-supplied data; the local loop does not claim independent sampling or select a candidate. Results cannot edit those budgets. Exploration, dissent, reflection, and manual-recovery budgets are protected resources.

The system records separate dimensions, never a single efficiency score:

- outcome quality
- cost and latency
- novelty yield
- candidate diversity
- calibration
- useful human overrides
- skill retention
- recovery success
- reversibility
- unresolved Unknown rate
- disagreement preserved and resolved
- common-mode failure exposure

The seven failure modes have explicit controls:

| Failure mode | Control |
| --- | --- |
| Counter-serendipity | Locked alternative sampling, at least two distinct candidate IDs, and preserved candidates |
| Hyperfocus | Confidence/reversibility gate and slow-path escalation |
| Self-amplifying cascades | Distinct intuition/validator identities when external independence is asserted, three-instance local quorum, dissent record, and no self-signing |
| Skill erosion | Required human intuition and manual fallback budgets |
| Perverse feedback | Later outcome and useful-override fields are recorded separately |
| Data deluge | Enforced proposal/alternative/recovery limits, digest references, exclusions, and aggregate-only platform export |
| Monoculture | At least two distinct candidate IDs, rejected alternatives, and caller-supplied validation context; local-quorum machine diversity is structural, not organizational independence |
