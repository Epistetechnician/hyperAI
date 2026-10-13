# Installed ACP V4 Capability Alignment Packet

Status: authorized local synthetic fixture correction under the user-directed
Rust compute-desk subplan and the accepted `desk_runtime.v1` and `acp.v1`
implementation slices.

State slice: `hyperai.bazaar.acp_v4_capability_fixture_alignment.v1`.
Parents: [Rust port subplan](compute-bazaar-rust-port-subplan.md),
[Phase 1 contract packet](compute-bazaar-phase-1-contract-packet.md), and
[Phase 8 ACP packet](compute-bazaar-phase-8-acp-packet.md).

Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, with
the existing dirty and untracked worktree preserved. The relevant source
hashes below freeze the V4 capability/tool contract and installed integration
test before this correction.

Frozen source SHA-256:

- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`:
  `3a29e5fcf1455f3d9892aef6072fbe3923a93ed09d3610521add627b00bc03ee`
- `crates/hyperai-acp/tests/synthetic_peer.rs`:
  `dc886c9f4bdd886b7c7636d3d6b6c0c41b1c0701751a07fd2104a9c8d8ccfc6c`
- `crates/hyperai-acp/src/tools.rs`:
  `e8406f28137afeb26854a77758a09469dae77868c49feb72fc8753c074aac1ac`
- `crates/hyperai-local-service/src/route_policy.rs`:
  `137cddf675449fcae9531bc9245dfd09b2d3dc8c0b7faa226a15909e1f22806a`
- `crates/hyperai-local-service/src/server.rs`:
  `75e918d2d08ebdf88c5b4df8db566e1d8e1bb24146d103ef159f004141f09a74`
- `crates/outcomes-cli/tests/cli_compat.rs`:
  `924e929e375f23c93f27f88802ea3211ededc5ca92a646a6a688dd281efad4ff`
- `scripts/install-hyperai-local.mjs`:
  `1e195cbb618d34cc521b11944e269d3a124d56f5289621d53bd44a7b9ddf49b3`
- `Cargo.toml`: `22f7c776a269e335521724dc7f45c7eeab8a39f5a645d9d7140ee795d439c487`
- `Cargo.lock`: `84895b9a6a8c077dba860ea6a14d5324182d1007b5bff4f9115b9753b935746a`
- `crates/hyperai-acp/Cargo.toml`:
  `86db2fa85795ffcf7d89876526f8eb21948353fa5bdd48bc490aad14c75a726c`
- `crates/hyperai-local-service/Cargo.toml`:
  `b5f6831cf1b75c19aa035e7f785a81519ebcf33092aea4a04c72cf3fb60d02af`
- `crates/hyperai-mcp/Cargo.toml`:
  `6141d8f52750fab10aabdd33347a82176f3b238c6fbe0707947682046ea304bf`
- `crates/outcomes-cli/Cargo.toml`:
  `6b733497c27e1d59a525b1c1ac8ee2958567c230990a406462ad03a106c99648`
- `docs/compute-bazaar-phase-8-acp-packet.md`:
  `81eb0ec64bca9ddc425a6b5bf1b2b638c1d645645611b7b7f857a9bb5faf3c6a`
- `docs/compute-bazaar-rust-port-subplan.md`:
  `d1486a26266df309f2681e03b54c06d3de8e6ab0bf0898e4c3e5d83f9f7779a1`
- `docs/state-slices.md`:
  `d446a8d31f9dcb26e090e90ee9b5ae24ed659b70d7c3143d6da837d9bd74dc53`

## Objective and contract

Align the HyperAI-authored ACP fixture's route probes with the fixed V4
MCP allowlist. V4 preserves the V3 seller-failure GET tool and appends the
inconclusive-preview GET tool. Their corresponding scoped routes are
read-only and are already in the typed route grant. The fixture's negative
probe incorrectly treated the seller-failure GET alias as denied, so the
current installed source-integration fails before exercising the complete
sidecar flow.

The correction is test-fixture-only. It must assert that the scoped capability
can GET both fixed preview aliases without exposing the capability, and still
cannot GET their owner routes, POST to either scoped alias, or use an unknown
route. It must preserve ACP protocol v1, MCP V1-V4 arrays, route methods,
handler behavior, schemas, and product permissions.

Pre-correction observation: the locked/offline workspace-bin build passed.
The exact ignored source-install integration then failed after its installed
CLI/TUI lifecycle with `AGENT_SESSION_FAILED`; the synthetic peer reported
that the scoped capability reached its supposedly denied
`GET /api/v1/agent/tools/market/workload-trace/failure-preview` route. The
fixture returned no route status in that diagnostic. The service route table
and V4 allowlist show that this is an allowed GET; no runtime route or grant
change is authorized here. After correcting that stale route assertion, the
installed run reached the MCP tools and exposed a second stale fixture check:
it expected a non-null digest on the mock disposition, while the application
contract intentionally leaves that reference digestless and links it through
the enclosing receipt. The fixture correction must assert that link basis and
the `no_payment_or_entitlement_claim` ceiling.

## Exact writable files and effects

- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`: correct positive/negative
  route probes for the existing V4 grant.
- `crates/hyperai-acp/tests/synthetic_peer.rs`: make the local HTTP test
  harness answer the corrected positive and negative route probes.
- This packet, `docs/state-slices.md`,
  `docs/compute-bazaar-phase-8-acp-packet.md`, and
  `docs/compute-bazaar-rust-port-subplan.md`: record scope, execution evidence,
  and open gates.

Allowed effects: offline locked Rust workspace-bin build; local source install
to an automatically removed temporary root; temporary HyperAI home and fixed
HyperAI-authored synthetic data; test-owned service on IPv4 loopback port `0`;
the HyperAI ACP fixture and installed MCP sidecar; focused tests and root
validation; cleanup of test-owned files and processes.

Non-goals: any product route, permission, tool-list, schema, CLI, TUI, or ACP
protocol change; Bazaar source or data access; real-user data; model or
third-party agent calls; provider, seller, registry, fleet, network, wallet,
chain, or reviewer access; paid execution, publication, commit, PR, or
deployment. Preserve existing user data and the service at
`127.0.0.1:4317`.

## Proof gates and claim ceiling

- The ACP synthetic-peer suite passes with the fixed V4 list and scoped route
  checks.
- The exact ignored installed lifecycle passes after staging workspace bins:
  CLI compatibility entrypoints, TUI PTY start/cleanup, local service launch,
  ACP v1 connection, MCP V4 list and read-only calls, denial boundaries,
  capability revocation, and process cleanup.
- Run pinned Rust formatting, `python3 scripts/verify_imports.py`, root
  `CARGO_NET_OFFLINE=true corepack pnpm run lint`, and `git diff --check`.
- Confirm the fixture is absent from the install root, owned service and
  sidecar processes exit, test homes are removed, and port `4317` is not used.

Maximum claim: current-source local install-to-ACP/MCP capability alignment
for the fixed synthetic V4 surface on the tested Unix host. This does not
establish native support on other targets, third-party ACP compatibility,
complete accessibility or policy parity, independent review, clean
reproduction, package signing, or production readiness.

Assurance is `INCONCLUSIVE` unless the global assurance overlay's local-test,
static-check, fuzzing, threat-model, and bounded red-team requirements are all
met for this scope. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. Data/artifact custody is checked-out HyperAI source and
removed disposable synthetic test data. Publish boundary: local source and
docs only; no commit, PR, release, external circulation, or deployment.

## Execution record (2026-10-01)

The fixture correction was limited to the two files above. The product V4 tool
list, route registration, capability grants, schemas, handlers, and ACP
protocol were unchanged. The route-check list was factored into a named
constant after root Clippy reported that the synthetic-peer test exceeded its
function-size lint.

Passed on the checked-out macOS arm64 host with Rust/Cargo 1.93.1:

- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo build --workspace --bins --locked --offline`.
- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p hyperai-acp --test synthetic_peer`: 9 passed.
- The exact ignored `outcomes-cli` source-install integration after the workspace-bin build: 1 passed. It launched the installed CLI/TUI, started the local service on an OS-assigned loopback port, connected ACP v1 to the synthetic peer, exercised the installed MCP V4 sidecar and its positive/negative routes, then revoked the capability and cleaned up the service, sidecar, PTY state, and temporary home. The installed TUI transcript digest was `ea7a04352e983e760ddc64791d141edacc2fd6703520fb3f89574f99b9dbac44`.
- `CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo fmt --all -- --check`.
- `python3 scripts/verify_imports.py` for the pinned Simple Compute Market, HyperIntel, and JustAI inventories.
- `CARGO_NET_OFFLINE=true corepack pnpm run lint`: exit 0, including root contract tests, all-target/all-feature Clippy, and workspace/doc tests.

Resulting corrected source SHA-256:

- `crates/hyperai-acp/src/bin/acp-fixture-peer.rs`:
  `756237fd87b44b96399790288ea63168bf47621fe5687ecff19afa736701597b`
- `crates/hyperai-acp/tests/synthetic_peer.rs`:
  `72f1ef1707c154b1ac18683956db454afc6046a4efc14c4c8c5db0c018de7fe3`

The installed integration invocation was
`CARGO_NET_OFFLINE=true sh scripts/with-pinned-rust.sh cargo test --locked --offline -p outcomes-cli --test cli_compat source_install_bundles_mcp_sidecar_and_connects_synthetic_peer -- --ignored --exact --nocapture`.

Two initial assertions failed because the fixture expected an already-granted
seller-failure GET to be denied and a deliberately digestless mock disposition
to carry its own digest. Both were corrected to match the current route and
receipt-link contracts. The root lint's `too_many_lines` finding was corrected
by extracting the scoped route cases into a constant; the focused suite and
root lint then passed. The integration used port `0`; port `4317` was not
contacted. The successful integration reported its temporary home removed and
no test-owned child process remains.

Assurance remains `INCONCLUSIVE`: this slice has focused and root tests,
formatting, import verification, and static lint, but no slice-specific fuzz
run, threat model, or bounded red-team review. Reviewer status is
`NOT_REQUESTED`; reproducibility is `NOT_REPRODUCED`. Custody remains local
HyperAI source plus removed disposable synthetic test data. The result does
not prove correct compute, provider behavior, third-party ACP interoperability,
payment entitlement, production readiness, or support on other platforms.
