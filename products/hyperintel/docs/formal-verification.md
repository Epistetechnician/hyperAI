# Local formal verification boundary

HyperIntel R0 includes one bounded, machine-checked property: an abstract
two-candidate routing selector cannot select an ineligible candidate. This is
local formal-model evidence only. It is not a proof of the Rust implementation,
provider behavior, inference correctness, replay persistence, settlement, or
production safety.

## Exact artifacts

- Model: `crates/hyperintel-market/formal/policy_non_widening_v1.smt2`
- Process test: `crates/hyperintel-market/tests/formal_policy.rs`
- Bound implementation source: `crates/hyperintel-market/src/routing.rs`
- Bound source SHA-256:
  `a0ea4bc9129b509692ac3d4b55e215ba3514056cb09c270cdbaf95142d1bb425`
- Repository resolver: `tools/pinned-z3`
- Required local solver output for `z3 --version`:
  `Z3 version 4.15.0 - 64 bit`
- Expected solver verdicts, in order: `unsat`, then `sat`

The model was added at `661cdc3`; the repository-controlled exact-version
resolver and corrected source binding were added at `9703256`, and the source
binding was advanced with the compatibility correction at `6887ac1`. The model
is HyperIntel-specific and handwritten; no source-repository domain or formal-model
code was copied. The work reuses only the pinned-source process and conservative
claim discipline recorded in [`provenance.md`](provenance.md).

## Property

The QF_LIA model represents the canonical report order with exactly two
candidates. Each candidate has:

- an eligibility Boolean;
- a Boolean indicating whether cost is available; and
- a nonnegative integer rank abstracting the complete deterministic routing
  order.

A candidate is admissible only when it is eligible and has a cost. The selected
candidate is the lowest-ranked admissible candidate with deterministic
candidate-zero tie-breaking. The primary query asks whether a selected
candidate can nevertheless be ineligible. Z3 returns `unsat`.

The second query deliberately weakens selection by ignoring eligibility. It
fixes candidate zero as ineligible, cost-bearing, and cheaper than eligible
candidate one. Z3 returns `sat`, retaining a counterexample replay that detects
the policy-widening defect the primary model excludes.

## Process gate

The Rust contract test:

1. hashes the exact `routing.rs` bytes and rejects any source drift;
2. invokes `tools/pinned-z3`, which resolves an operator-installed solver and
   requires the exact Z3 4.15.0 version string;
3. sends the committed SMT2 bytes to the resolver with `-in -smt2 -T:5`;
4. requires a successful process with empty stderr; and
5. requires stdout to equal `unsat\nsat\n`.

The check is included in both:

```text
pnpm run verify:contracts
pnpm run verify:full
```

`Z3 version 4.15.0 - 64 bit` is therefore a local prerequisite for those gates.
The repository pins the reported version and invocation path, not the solver
binary hash or installation provenance. Z3 is not a HyperIntel runtime
dependency.

## Claim ceiling

The source digest detects that the routed Rust file changed; it does not prove
that the SMT definitions correspond semantically to the Rust program. The
model also omits canonical serialization, cryptography, arithmetic beyond the
abstract rank, unbounded candidate sets, concurrency, networking, persistence,
buyer authentication, external replay consumption, Solidity, EVM execution,
and provider behavior.

Accordingly, the supported claim is only:

> Z3 4.15.0 returned `unsat` for an ineligible selection in the committed
> two-candidate abstraction and `sat` for the committed weakened-selector
> counterexample, while the separately checked `routing.rs` bytes matched the
> recorded SHA-256 digest.

Any routing-source edit must update the model or explicitly confirm that its
abstraction remains appropriate, then update the source hash and re-run both
verdicts. A changed digest must never be bypassed as formatting-only drift.
