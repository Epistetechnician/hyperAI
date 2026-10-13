# Phase 4 Fleet Inventory and Preflight Packet

State slice: `hyperai.bazaar.fleet_readonly.v1`.
Parent plan: [HyperAI Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Objective

Expose a HyperAI-owned Fleet contract through the existing application facade,
authenticated loopback service, and HyperAI browser workspace. The contract
demonstrates inventory, a synthetic health timeline, deterministic doctor
checks, and digest-bound operation previews. It is a local product interface
slice, not a provider adapter.

## Baseline and state boundary

Baseline revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the pre-existing dirty monorepo and Phase 1–3A files preserved. No commit or
frozen worktree digest is claimed.

Included files:

- `Cargo.toml` and `Cargo.lock`: register and lock the independently owned
  `hyperai-fleet` crate using existing workspace dependencies only.
- `crates/hyperai-fleet/Cargo.toml`, `src/lib.rs`, and
  `tests/fleet_contract.rs`: own versioned synthetic inventory, explicit
  health states and timeline, deterministic local doctor checks, and
  operation-preview contracts.
- `crates/hyperai-application/Cargo.toml` and `src/lib.rs`: compose typed
  Fleet reads and preview calculation without owning provider credentials,
  host access, or operations.
- `crates/hyperai-local-service/src/server.rs` and
  `tests/fleet_api.rs`: expose authenticated, bounded, versioned read and
  preview routes through the existing loopback service.
- `apps/hyperai-interface/app.js` and `app.css`: replace the static Fleet
  placeholder with the synthetic inventory, health, doctor, and non-executable
  plan workspace.
- `README.md`, `docs/state-slices.md`, this packet, and
  `docs/compute-bazaar-rust-port-subplan.md`: document the capability and
  report only evidence actually produced.

No nested product workspace, HyperIntel, JustAI, Simple Compute Market, or
ignored Bazaar source path is in this slice.

## Allowed effects

- Construct fixed, HyperAI-authored synthetic Fleet records in memory.
- Read those records through the application facade and authenticated local
  service.
- Compute deterministic doctor results and operation-preview digests in
  memory.
- Render the typed results in the local browser interface.
- Run focused local tests, root validation, and browser checks against this
  synthetic interface.

No durable Fleet storage or caller-imported inventory is added. No machine,
process, environment variable, socket, filesystem inventory, credential,
provider, SSH, agent, network service, or remote fleet is inspected or
contacted.

## Contract and safety rules

- Every node, observation, health state, and capacity detail is labeled as a
  synthetic scenario with HyperAI as the fixture author. Fixture identifiers
  are not hostnames, provider IDs, or owner identities.
- Health status values such as healthy, stale, unreachable, and unknown are
  example states only. They are never derived from a probe or current clock.
- Doctor checks validate only the synthetic contract and fixture consistency;
  they do not diagnose the user's machine or a provider fleet.
- An operation preview binds the exact synthetic target and intent to a stable
  digest. Recomputing against a changed intent reports `preview_changed`.
- Every preview reports `advisory_only`, `not_authorized`, and
  `does_not_invoke_or_reserve`. It contains no shell command, socket target,
  credential reference, reservation, or callable action.
- An absent, stale, unreachable, or unknown fixture status stays explicit and
  blocks any claim that an action could proceed. Even the healthy fixture is
  never an execution approval.
- All JSON request types reject unknown fields, and invalid node/intent values
  fail closed.

## Non-goals

This slice does not import inventory, persist Fleet state, schedule or reserve
capacity, attach nodes, launch instances, start/cancel workloads, terminate
resources, open sockets, use SSH, invoke provider APIs, manage credentials,
connect ACP, or enable live Fleet operations. Phase 5's simulator and all
owner-authorized adapters remain separate slices. Fleet telemetry is not
computation verification, outcome evidence, market truth, quote validity, or
payment entitlement.

## Proof gates

- Synthetic fixture output is deterministic and carries an explicit fixture
  provenance and claim ceiling.
- Healthy, stale, unreachable, and unknown states remain distinguishable in
  inventory/timeline and doctor output; unsafe or changed input fails closed.
- Identical preview inputs yield the same digest; changing a bound field yields
  a different digest and explicit changed-preview status.
- The service requires the existing local authentication and CSRF contracts;
  requests over limits and malformed requests fail closed.
- No operation preview can invoke a process, network call, reservation, or
  state mutation.
- Browser rendering escapes dynamic text and remains usable at desktop and
  narrow viewport widths without horizontal page overflow.
- Focused crate/service tests, root lint, JavaScript syntax, and diff hygiene
  pass. Browser evidence is reported separately from backend tests.

## Assurance report

Validation snapshot (2026-09-24): baseline revision
`dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with the pre-existing
dirty monorepo and Phases 1–3A preserved; no commit or frozen worktree digest
was created. Focused `hyperai-fleet`, application, and local-service tests
passed. `corepack pnpm run lint` passed after the final UI change, including
formatting, workspace Clippy, contract tests, and
`cargo test --workspace --all-features`. `node --check
apps/hyperai-interface/app.js` and `git diff --check` passed.

Browser evidence: the local synthetic Fleet screen was exercised at 1280 px
and 390 px. Desktop shows the inventory table; mobile shows stacked node cards.
Both had no page-level horizontal overflow or console errors. Stale-state
preview and changed-plan digest paths displayed their blocked/advisory status;
no execution or reservation action exists. The temporary service was stopped
and its temporary home removed.

Assurance overlay: `global-experiment-assurance-lifecycle-v1`. Assurance level:
`NOT_ASSIGNED`; the listed local proof gates passed, but no bounded adversarial
review, clean second-environment reproduction, or independent review was
performed. Reviewer status: `NOT_REQUESTED`. Reproducibility status:
`NOT_REPRODUCED`. Data and artifact custody: HyperAI-authored synthetic fixture
bytes in local process memory and tests, plus temporary browser state; no
imported or restricted data. Claim ceiling: this local build deterministically
renders and previews the named synthetic examples under this packet's
constraints. The next authority gate is a new exact owner-authorized slice for
any real Fleet observation or operation; this packet does not prepare
credentials or grant that authority.
