# Phase 5 Fleet Simulation Packet

State slice: `hyperai.bazaar.fleet_simulation.v1`.
Parent plan: [HyperAI Compute Desk Rust port subplan](compute-bazaar-rust-port-subplan.md).
Predecessor: [Phase 4 Fleet inventory and preflight](compute-bazaar-phase-4-fleet-readonly-packet.md).
Assurance overlay: `global-experiment-assurance-lifecycle-v1`.

## Objective

Add one shared, deterministic Fleet lifecycle simulator to the existing
HyperAI-owned Fleet domain. The authenticated local service owns its volatile
simulation state. The browser, CLI, and TUI call the same versioned API and
facade. This slice exercises ownership, virtual resource leases, operation
idempotency, uncertain outcomes, reconciliation, and cleanup without invoking
an operating-system process or external resource.

## Baseline and state boundary

Baseline revision: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty monorepo and Phases 1–4 preserved. No commit or frozen
worktree digest is claimed. No new dependency or Cargo workspace member is
required.

Included files:

- `crates/hyperai-fleet/src/lib.rs` and
  `crates/hyperai-fleet/tests/fleet_contract.rs`: versioned simulation state,
  transition rules, bounded idempotency cache, recovery paths, and contract
  coverage.
- `crates/hyperai-application/src/lib.rs`: expose the simulator's typed
  snapshot and transition operations through the existing facade.
- `crates/hyperai-local-service/src/server.rs` and
  `crates/hyperai-local-service/tests/fleet_api.rs`: own one volatile simulator
  per service process and expose authenticated, bounded, versioned routes.
- `crates/outcomes-cli/src/lib.rs` and
  `crates/outcomes-cli/tests/cli_compat.rs`: add `hyperai fleet simulation`
  `show` and explicit `step` commands over that service.
- `crates/hyperai-tui/src/lib.rs` and
  `crates/hyperai-tui/tests/terminal_restore.rs`: show Fleet simulator state
  and advance one clearly labelled synthetic happy-path step from the Fleet
  workspace.
- `apps/hyperai-interface/app.js` and `apps/hyperai-interface/app.css`: add
  lifecycle controls, fault scenarios, current virtual lease/task state, and
  recovery controls to the existing Fleet workspace.
- `README.md`, this packet, `docs/state-slices.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`: document the implemented
  capability and evidence actually produced.

No files under `references/`, `products/hyperintel/`, `products/justai/`, or
`products/simple-compute-market/` are included. No other build graph, source
manifest, or product workspace is modified.

## Frozen simulator contract

- The sole resource is `fixture-node-atlas`, with a fixed pool of four
  **virtual** GPU units. These values are simulator parameters, not inventory,
  capacity, availability, or performance claims about hardware.
- One local service process owns one simulator. Ownership is labelled
  `local_service_simulation`; access uses the service's existing local
  authentication and request protections.
- The normal lifecycle is `detached → attached → launched → workload_running
  → launched → terminated`. Launch reserves virtual units; cancel returns the
  virtual task to `launched`; terminate releases the virtual lease.
- `workload_start` creates a deterministic virtual task identifier. It is not
  an OS PID, process handle, command, or externally addressable resource.
- A transition binds the operation, fault scenario, requested virtual units,
  expected state digest, and idempotency key. The service caches at most 128
  distinct keys without eviction. Repeating the same key and request returns
  the original response; reusing the key with changed content or submitting a
  stale state digest fails closed. Once the cache is full, new keys are
  rejected until the service restarts; existing keys remain replayable.
- `timeout_before_effect` and `timeout_after_effect` enter
  `reconciliation_required` and conservatively retain any possibly affected
  virtual capacity. `reconcile` resolves only the simulator's stored outcome;
  it does not inspect an external resource.
- `partial_effect` is supported for launch and terminate. It enters
  `cleanup_required`; the simulator retains any remaining virtual lease until
  `cleanup` releases it. A task or lease in a recovery state cannot be used for
  another normal transition.
- The store is volatile and resets when the local service process exits. No
  durable Fleet journal or cross-restart recovery claim is introduced.
- Response bodies label all operations `simulation_only_no_process_no_network`
  and state their synthetic claim ceiling and nonclaims.

## Allowed effects

Create, inspect, and transition the one fixed simulator state in local service
memory. Run the local Rust/API/CLI/TUI checks and browser interactions listed
in this packet. Temporary test homes may hold the existing service's auth and
runtime files, subject to its established cleanup behavior.

## Non-goals and hard limits

This slice does not create, inspect, signal, or reap an OS process; launch a
container, VM, workload, shell, or command; open a socket beyond the existing
loopback service; read arbitrary files, host metadata, environment variables,
credentials, or machine inventory; call SSH, ACP, a provider, cloud, registry,
or remote Fleet; persist simulation state; or reserve real capacity. It does
not add real Fleet adapters or authority, settle funds, validate completed
computation, or establish actual cleanup/reaper behavior. Live access and
operations still require a new exact owner-authorized slice.

## Proof gates

- The complete normal lifecycle preserves owner labels and virtual capacity
  accounting; termination and cleanup release all virtual leases and task
  references.
- Identical idempotency retries do not add events, leases, or tasks; changed
  content under a used key and stale digests fail closed. Capacity and cache
  ceilings are enforced.
- Timeout-before, timeout-after, and partial-effect scenarios remain explicit;
  reconciliation and cleanup restore a safe simulator state, and disallowed
  transitions from recovery states fail closed.
- API reads and transitions require existing authentication/CSRF protections;
  malformed and unknown fields fail closed. No response contains a command,
  PID, socket target, provider identifier, credential reference, or reservation
  outside the simulator's explicitly virtual lease record.
- CLI and TUI use the same service routes and schemas as the browser. Existing
  CLI command compatibility and terminal restoration remain intact.
- Browser scenarios are usable at desktop and narrow widths, clearly separate
  simulation from the Phase 4 advisory preview, and report failures and
  recovery states without implying real execution.
- Focused crate/service/CLI/TUI checks, root lint, JavaScript syntax, browser
  checks, and diff hygiene pass. Browser and backend evidence are reported
  separately.

## Assurance report

Assurance level: `NOT_ASSIGNED`. Validation ran against the dirty working tree
based on `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`; no commit or
frozen worktree digest is claimed. `corepack pnpm run lint` passed, including
formatting, workspace Clippy with warnings denied, workspace contract tests,
and `cargo test --workspace --all-features`. Focused Fleet, service, CLI, and
TUI tests passed. `node --check apps/hyperai-interface/app.js` and `git diff
--check` passed.

The local browser flow attached the fixture, recorded a timeout-after-launch,
reconciled the applied effect, injected a partial termination, and cleaned up
to a terminated state with zero allocated or quarantined virtual units. At
390 px and 820 px, the document had no horizontal overflow. The browser
console had no errors or warnings. No browser test contacted an external
service or real Fleet resource.

Reviewer status: `NOT_REQUESTED`. Reproducibility status: `NOT_REPRODUCED`.
Data and artifact custody: fixed HyperAI-authored synthetic records and
volatile local simulator state only; temporary local service state was
stopped and removed after the browser check. Claim ceiling: deterministic
state-transition and recovery behavior for the single named virtual scenario
inside one local service process. This does not establish host process
management, real resource ownership, remote operation safety, workload
correctness, outcome truth, or payment entitlement. The next authority gate is
a new exact, owner-authorized adapter packet before any non-synthetic Fleet
observation or operation. Phase 6 Local Eval remains separate and requires its
own bounded packet before implementation.
