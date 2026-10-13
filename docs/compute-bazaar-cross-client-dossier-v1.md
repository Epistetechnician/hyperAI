# HyperAI Cross-Client Wiring and Evidence Dossier v1

- **State slice:** `hyperai.bazaar.cross_client_compatibility_dossier.v1`
- **Governing overlay:** `global-experiment-assurance-lifecycle-v1`
- **Source revision:** `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`
- **Pre-edit worktree status digest:** `8a6f2b776cddacaf103eee2bb83509c38468f544a273ff59ec2cc76c5fcc003c`
- **Route/auth/effect baseline:** [route inventory](compute-bazaar-route-auth-effect-inventory-v1.md)

## Purpose and assurance

This dossier consolidates source-visible client paths and existing bounded
workflow evidence. It does not treat different clients as equivalent merely
because they reach the same local service, and it does not merge packet results
that used different frozen inputs or state slices.

- **Assurance:** `NOT_RUN` for complete cross-client route/schema parity.
- **Reviewer:** `NOT_REQUESTED`.
- **Reproduction:** `NOT_REPRODUCED` for this dossier.
- **Evidence custody:** checked-out HyperAI source, fixed synthetic fixtures,
  and disposable local test homes.
- **Claim ceiling:** local route wiring and the specific packet/test scopes
  cited below. No claim of live provider behavior, real compute, settlement,
  full platform support, or production readiness.

## Client boundaries

| Client | Service boundary | Auth/capability | What the client does not inherit |
| --- | --- | --- | --- |
| Browser | Same-origin `/api/v1` calls through the local service | Volatile browser session; CSRF token and `Origin` required for writes | Process-owner bearer authority; external service access |
| CLI | Loopback local-service HTTP for its routed operations; workload-trace command also composes through `hyperai-application` directly | Process owner bearer | Browser route coverage; direct trace composition does not prove HTTP-route parity |
| TUI | Loopback local-service HTTP for its selected workspace operations | Process owner bearer | Browser route coverage; interaction and accessibility parity beyond tested frames |
| MCP | Separate `/api/v1/agent/tools/...` path through the local service | Volatile capability limited to the exact 15 method/path entries | Quote approval/completion, Data saves/seeding/cancellation, Fleet transitions, Eval run, agent session control |
| ACP | Bundled MCP bridge and its fixed tool list | ACP session uses the scoped MCP capability | Full owner API, third-party-agent behavior, or authority outside the MCP allowlist |

The exact 65 API registrations and each row's source-visible client calls are
in the [route inventory](compute-bazaar-route-auth-effect-inventory-v1.md).
The browser, CLI, and TUI share the service and owner contracts but do not
call the same route set. MCP/ACP use separate alias paths and preserve a
strictly smaller capability.

## Workflow wiring matrix

“Source calls” records routes visible in client source, not a claim that every
call has passed a runtime workflow test. The MCP/ACP column describes the
scoped tool routes, not the direct owner-route aliases.

| Workflow | Browser | CLI | TUI | MCP / ACP | State and effect boundary |
| --- | --- | --- | --- | --- | --- |
| Supply comparison → quote → receipt | Source calls comparison, quote preview, approval, completion, history, and verification; browser lifecycle packet passes selected-offer approval and mock completion | Source calls comparison, quote preview, approval, completion, and history; installed CLI lifecycle passed | Source calls comparison, quote preview, approval, completion, and history; local terminal lifecycle passed; installed startup-sync attempt remains `INCONCLUSIVE`, while a later installed receipt-layout slice passed its exact scope | Source calls comparison and quote preview only; no approval or completion route | Local synthetic quote, approval, mock fulfillment, verifier/disposition, and receipt; no real seller, compute, or payment |
| Workload trace | Source calls success, seller-failure, inconclusive, Data-status, and duplicate-disposition previews | Same projection is also available via the application path; selected owner-route parity is covered by focused tests | Source calls default, failure, and inconclusive owner routes | Scoped routes expose default, failure, and inconclusive projections | Read-only composition; fixed previews do not establish actual execution failure, dispute, missing data, or payment |
| Data catalog and query | Source calls catalog, release/seed, query/cancel, and model/view preview/save | Source calls catalog, describe, seed/query, and model/view operations | Source calls catalog, model/view catalogs, and query | Scoped catalog/query only | DataFusion is in-process; no isolated worker. Phase 2 Data remains `NOT_ASSIGNED` |
| Fleet | Source calls inventory, doctor, advisory preview, and local simulation/transition | Source calls inventory, doctor, preview, and local simulation/transition | Source calls inventory, doctor, preview, and local simulation/transition | Scoped inventory, doctor, and preview only | Simulation and previews do not contact or reserve remote Fleet resources |
| Eval | Source calls catalog and bounded local run | Source calls catalog and bounded local run | Source calls catalog and bounded local run | Scoped catalog and report reads only | Fixed local scorer; reports are process-held, not paid/provider evaluations |
| Trade | Source calls catalog, series, scenario, and paper replay | Source calls catalog, series, scenario, and paper replay | Source calls catalog, series, scenario, and paper replay | Scoped catalog, series, and scenario only | Synthetic research and paper replay; no broker or financial action |
| Offline source adapters | Source calls caller-supplied SCM capture verification and HyperIntel verify/reconcile | No corresponding local-service route callsite found | No corresponding local-service route callsite found | No corresponding scoped tool route | Caller-selected/offline evidence only; no live registry, provider, or HyperIntel routing |
| Agent session control | Source calls connect/status/prompt/cancel/disconnect | Source calls connect/status/prompt/cancel/disconnect | Source calls status/connect/prompt/cancel/disconnect | Not granted through tool routes | Starts and controls the configured agent path; model/provider/network behavior is not qualified here |

## Evidence by surface

| Surface | Current bounded checks | Recorded packet evidence | Remaining limit |
| --- | --- | --- | --- |
| Route/auth policy | Static comparison matched 8 static/health, 50 direct API, and 15 scoped-tool route/method pairs. Local-service tests passed exact tool allowlisting, wrong-method/path denial, and denial on owner routes | [Route policy packet](compute-bazaar-local-service-route-policy-packet.md) | No complete route-by-route typed-schema, error, and negative-case matrix |
| Browser | `corepack pnpm run verify:interface`: 34 JavaScript tests passed. This command does not launch a browser | [Supply-quote browser lifecycle](compute-bazaar-supply-quote-lifecycle-browser-packet.md) records a same-host Chrome run: 33 workspace route/viewport checks, 33 reduced-motion checks, selected-offer quote, approval, mock completion, receipt, and cleanup passed | No OS-level screen-reader/physical-keyboard, cross-browser, or native-platform proof |
| CLI | Current package tests passed. The installed buyer quote packet passed compare, preview, exact-digest approval, mock completion, stale-digest rejection, and cleanup | [Installed CLI lifecycle](compute-bazaar-installed-quote-lifecycle-packet.md) | Does not prove all route schemas or that CLI's direct application trace path equals every service error/policy behavior |
| TUI | Current TUI unit and terminal-restoration tests passed, including service query and quote-action flows. A separate installed receipt-layout qualification passed its exact synthetic lifecycle/display scope | [Terminal quote lifecycle](compute-bazaar-terminal-quote-lifecycle-packet.md), [installed receipt-layout packet](compute-bazaar-installed-tui-receipt-layout-packet.md) | The earlier installed quote startup-sync packet remains `INCONCLUSIVE`; its consumed attempt is preserved. Broader physical keyboard, assistive technology, and platform parity remain open |
| MCP/ACP | Current MCP and ACP package tests passed. The installed V6 tool-list alignment passed for one HyperAI synthetic peer; tests retain capability denial and error-code-only forwarding | [Phase 8 ACP packet](compute-bazaar-phase-8-acp-packet.md) | ACP cancel-keyboard attempts v1–v5 remain `INCONCLUSIVE`; no third-party agent/provider behavior is qualified |
| Market history | Current route/client packages passed selected history digest and cursor-forwarding checks | [Market-history client packet](compute-bazaar-market-history-clients-packet.md) | Browser run used one history row; multi-row HTTP cursor traversal remains open |

The Rust checks run for this dossier used pinned Rust with `--locked
--offline` across the HyperAI foundation, Market, ACP, application, Data, Eval,
Fleet, HyperIntel/SCM adapters, local service, MCP, Trade, TUI, and CLI
packages. All executed tests passed; three existing browser/install tests
were ignored because their browser or staged-binary prerequisites were absent.
Exact command and per-gate output are
recorded in the [route inventory](compute-bazaar-route-auth-effect-inventory-v1.md).
The Simple Compute Market, HyperIntel, and JustAI import digests also passed
`python3 scripts/verify_imports.py`; those checks prove snapshot integrity
only.

## Open compatibility and authority gates

1. Map every route/tool to exact typed request and response schema versions,
   persistence touched, and stable error codes.
2. Add complete route/client checks for ordinary browser/CLI/TUI calls and the
   separate MCP/ACP allowlist. Exercise denied access, stale digests,
   malformed/unsupported schemas, and failure recovery at each relevant
   boundary.
3. Keep the installed TUI and ACP cancellation `INCONCLUSIVE` outcomes intact;
   a new claim requires a fresh named slice and packet rather than a retry of
   a consumed identity.
4. Qualify native target platforms and accessibility separately. macOS-local
   or emulated results do not qualify Windows, native Linux, Intel macOS, or
   physical assistive technology.
5. Keep Data worker isolation `NOT_ASSIGNED`. Keep live source/provider calls,
   remote execution, paid evaluation, financial Trade, custody, and settlement
   closed until their owner-specific contract, packet, and authority gates
   pass.

No global assurance level is assigned by this dossier. A cited `INTERNAL_PASS`
applies only to its exact packet scope. Reviewer status is
`NOT_REQUESTED`; no reviewer or deployment authorization is inferred.
