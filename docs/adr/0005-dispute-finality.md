# ADR 0005: dispute and finality

Status: accepted for implementation.

Truth, finality, and settlement lifecycle are separate state machines. Active
disputes and operator pauses block finalization and settlement. Corrections are
signed superseding records; they never silently replace a final resolution or
drive the original binding.

Counterexample correction is append-only and acyclic:

1. retain the provisional FALSE resolution;
2. append an immutable OPEN dispute before its configured close;
3. build recomputation evidence that binds the original lineage, dispute, and
   eligible counterexample executions;
4. append an authority decision that binds that recomputation evidence;
5. require replacement attestations to sign the original non-settling digest;
6. append provisional and final TRUE records, then an explicit supersession
   edge from the original record to the replacement.

The terminal decision is exported beside the recomputation bundle rather than
inside it. Including it would create a digest cycle because the decision binds
the bundle. Already-FINAL resolutions cannot enter this correction path.
