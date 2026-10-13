/* HyperAI's dependency-free local interface. All dynamic text is escaped before rendering. */
const state = { page: "overview", status: null, csrfToken: null, demo: null, workloadTrace: null, workloadTraceMode: "success", workloadTraceError: "", supplyComparison: null, supplyComparisonError: "", supplyComparisonLoading: true, supplyQuotePreview: null, supplyQuoteLoading: false, supplyQuoteError: "", marketHistory: null, marketHistoryError: "", marketHistoryLoading: false, quotePreview: null, quoteApproval: null, quoteCompletion: null, verification: null, hyperintelVerification: null, hyperintelReconciliation: null, simpleMarketCapture: null, scmListingQuery: { search: "", resource: "all", status: "all" }, dataCatalog: null, dataModels: null, dataViews: null, dataSyntheticRelease: null, dataImportReceipt: null, gpudiffSchedule: null, gpudiffScheduleBusy: false, dataDatasetId: "", dataSql: "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model", dataResult: null, dataBusy: false, dataQueryStatus: "", dataQueryError: false, activeDataQueryId: null, dataModelDraft: null, dataModelPreview: null, dataModelSaved: null, dataModelStatus: "", dataModelStatusError: false, dataViewDraft: null, dataViewPreview: null, dataViewSaved: null, dataViewStatus: "", dataViewStatusError: false, fleetInventory: null, fleetDoctor: null, fleetPlan: null, fleetNodeId: "fixture-node-atlas", fleetOperation: "launch", fleetSimulation: null, fleetSimulationTransition: null, fleetSimulationOperation: "attach", fleetSimulationFault: "none", fleetSimulationGpuUnits: 1, fleetSimulationRetry: null, fleetSimulationBusy: false, evalCatalog: null, evalReport: null, evalRetry: null, evalBusy: false, tradeCatalog: null, tradeSeries: null, tradeScenario: null, tradePaper: null, tradeBusy: false, tradeDraft: { primary: "a100-us-east", comparison: "a100-us-west", baseline: "a100-us-west", priceShockBps: 0, availabilityDeltaUnits: 0, requestedUnits: 1 }, agentSnapshot: null, agentActionErrorCode: "", agentBusy: false, agentControlBusy: false, fleetBusy: false, busy: false, apiError: "" };
const pageRoot = document.getElementById("page-root");
let connectionInventory = null;
let connectionInventoryError = "";
const banner = document.getElementById("connection-banner");
const toast = document.getElementById("toast");
const shell = document.getElementById("shell");
const sidebar = document.getElementById("sidebar");
const mobileNavToggle = document.getElementById("mobile-nav-toggle");
const mobileNavigation = window.matchMedia("(max-width: 840px)");
const pageNames = { overview: "OVERVIEW", market: "MARKET", deals: "DEALS", data: "DATA", inference: "INFERENCE", fleet: "FLEET", eval: "EVAL", trade: "TRADE RESEARCH", evidence: "EVIDENCE", access: "AGENTS & ACCESS", activity: "ACTIVITY" };
const fleetSimulationFaultLabels = { none: "No injected failure", timeout_before_effect: "Timeout before simulated effect", timeout_after_effect: "Timeout after simulated effect", partial_effect: "Partial simulated effect" };

function syncMobileNavigation(open = shell.classList.contains("sidebar-open"), focusNavigation = false) {
  const mobile = mobileNavigation.matches;
  const expanded = mobile && open;
  shell.classList.toggle("sidebar-open", expanded);
  mobileNavToggle.setAttribute("aria-expanded", String(expanded));
  mobileNavToggle.setAttribute("aria-label", expanded ? "Close navigation" : "Open navigation");
  mobileNavToggle.setAttribute("aria-controls", "sidebar");
  sidebar.toggleAttribute("inert", mobile && !expanded);
  if (mobile && !expanded) sidebar.setAttribute("aria-hidden", "true");
  else sidebar.removeAttribute("aria-hidden");

  if (focusNavigation && expanded) {
    const currentPage = sidebar.querySelector('.primary-nav [aria-current="page"]');
    (currentPage || sidebar.querySelector(".primary-nav [data-page]"))?.focus({ preventScroll: true });
  }
}

mobileNavigation.addEventListener("change", () => {
  const focusWasInSidebar = sidebar.contains(document.activeElement);
  syncMobileNavigation(false);
  if (mobileNavigation.matches && focusWasInSidebar) mobileNavToggle.focus({ preventScroll: true });
});
syncMobileNavigation(false);

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>\"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;" })[character]);
}

function money(microusd) {
  return new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(Number(microusd) / 1_000_000);
}

function shortDigest(digest) {
  const value = String(digest || "");
  return value ? `${value.slice(0, 12)}…${value.slice(-8)}` : "Not available";
}

function showToast(message, isError = false) {
  toast.textContent = message;
  toast.classList.toggle("is-error", isError);
  toast.hidden = false;
  window.clearTimeout(showToast.timeout);
  showToast.timeout = window.setTimeout(() => { toast.hidden = true; }, 4200);
}

function focusAgentWorkspace(targetSelector) {
  if (state.page !== "access") return;
  const preferred = pageRoot.querySelector(targetSelector);
  const fallback = pageRoot.querySelector("#agent-workspace-title") || pageRoot;
  const target = preferred instanceof HTMLElement && !preferred.matches(":disabled")
    ? preferred
    : fallback;
  target.focus({ preventScroll: true });
}

function focusPageElement(selector) {
  const preferred = pageRoot.querySelector(selector);
  const target = preferred instanceof HTMLElement && !preferred.matches(":disabled")
    ? preferred
    : pageRoot;
  target.focus();
}

async function requestJson(path, options = {}) {
  const method = String(options.method || "GET").toUpperCase();
  const csrfHeaders = state.csrfToken && !["GET", "HEAD"].includes(method) ? { "X-CSRF-Token": state.csrfToken } : {};
  const response = await fetch(path, { cache: "no-store", credentials: "same-origin", ...options, headers: { "Accept": "application/json", ...(["POST", "PUT", "PATCH"].includes(method) ? { "Content-Type": "application/json" } : {}), ...csrfHeaders, ...options.headers } });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    const normalized = window.HyperAIErrorEnvelope.normalize(response.status, body);
    const error = new Error(normalized.message);
    error.status = normalized.status;
    error.code = normalized.code;
    throw error;
  }
  return body;
}

async function refreshFleetWorkspace() {
  const [inventory, doctor, simulation] = await Promise.all([
    requestJson("/api/v1/fleet/inventory"),
    requestJson("/api/v1/fleet/doctor"),
    requestJson("/api/v1/fleet/simulation"),
  ]);
  state.fleetInventory = inventory;
  state.fleetDoctor = doctor;
  state.fleetSimulation = simulation;
}

async function refreshEvalWorkspace() {
  state.evalCatalog = await requestJson("/api/v1/eval/catalog");
}

async function refreshTradeWorkspace() {
  state.tradeCatalog = await requestJson("/api/v1/trade/catalog");
}

async function refreshMarketSupplyComparison() {
  state.supplyComparisonLoading = true;
  state.supplyComparisonError = "";
  try {
    state.supplyComparison = await requestJson("/api/v1/market/supply-comparison");
  } catch (error) {
    state.supplyComparison = null;
    state.supplyComparisonError = error.message;
  } finally {
    state.supplyComparisonLoading = false;
  }
}

async function refreshMarketHistory(cursor = null, append = false) {
  state.marketHistoryLoading = true;
  state.marketHistoryError = "";
  try {
    const query = new URLSearchParams({ limit: "20" });
    if (cursor !== null) query.set("cursor", cursor);
    const page = await requestJson(`/api/v1/market/deal-history?${query.toString()}`);
    if (page?.body?.schema_version !== "hyperai.compute-market.local-history-page.v1"
      || page.body.state_slice !== "hyperai.compute_market.local_deal_history.v1"
      || !Array.isArray(page.body.entries)
      || typeof page.digest !== "string"
      || !/^[0-9a-f]{64}$/.test(page.digest)
      || page.body.next_cursor !== null && typeof page.body.next_cursor !== "string") {
      throw new Error("The local service returned an unsupported Market history page.");
    }
    const previous = append ? state.marketHistory : null;
    state.marketHistory = {
      entries: [...(previous?.entries || []), ...page.body.entries],
      pageDigests: [...(previous?.pageDigests || []), page.digest],
      nextCursor: page.body.next_cursor,
      claimCeiling: page.body.claim_ceiling,
      nonclaims: page.body.nonclaims,
      effects: page.body.effects,
    };
  } catch (error) {
    state.marketHistoryError = error.message;
    if (!append) state.marketHistory = null;
  } finally {
    state.marketHistoryLoading = false;
  }
}

async function loadMoreMarketHistory() {
  const cursor = state.marketHistory?.nextCursor;
  if (state.busy || state.marketHistoryLoading || typeof cursor !== "string" || !cursor) return;
  await refreshMarketHistory(cursor, true);
  render();
}

async function retryMarketHistory() {
  if (state.busy || state.marketHistoryLoading) return;
  const request = refreshMarketHistory();
  render();
  await request;
  render();
}

function verifySupplyQuotePreview(preview, comparison) {
  const body = preview?.body;
  const nested = body?.quote_preview;
  const quote = nested?.body?.signed_quote?.quote;
  const selected = comparison?.body?.offer_assessments?.find((assessment) =>
    assessment.selected && assessment.offer_id === comparison.body.selected_offer_id,
  );
  const isDigest = (value) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
  if (!body || body.schema_version !== "hyperai.compute-market.supply-quote-preview.v1"
    || body.state_slice !== "hyperai.bazaar.supply_to_quote.v1"
    || !isDigest(preview.digest)
    || !isDigest(nested?.digest)
    || body.comparison_digest !== comparison?.digest
    || body.catalog_digest !== comparison?.body?.catalog?.digest
    || body.catalog_digest !== nested?.body?.catalog?.digest
    || body.selected_offer_id !== comparison?.body?.selected_offer_id
    || !selected
    || body.selected_offer_digest !== selected.offer_digest
    || body.selected_offer_digest !== quote?.offer_digest
    || nested?.body?.selected_offer_id !== body.selected_offer_id
    || JSON.stringify(nested?.body?.buyer_request) !== JSON.stringify(comparison?.body?.buyer_request)
    || body.buyer_request_digest !== quote?.request_digest
    || !Number.isSafeInteger(quote?.quoted_total_microusd)
    || !Number.isSafeInteger(quote?.amount_cap_microusd)
    || quote.quoted_total_microusd > quote.amount_cap_microusd) {
    throw new Error("The quote preview did not bind to the displayed comparison and selected signed offer.");
  }
  return preview;
}

async function previewSelectedSupplyQuote() {
  if (state.busy || state.supplyQuoteLoading) return;
  const comparison = state.supplyComparison;
  if (!comparison?.digest || !comparison.body?.selected_offer_id) {
    state.supplyQuoteError = "No selected eligible offer is available for a quote preview.";
    render();
    return;
  }
  state.supplyQuoteLoading = true;
  state.supplyQuoteError = "";
  setBusy(true);
  render();
  try {
    const preview = verifySupplyQuotePreview(await requestJson(
      "/api/v1/market/supply-quote-preview",
      {
        method: "POST",
        body: JSON.stringify({ comparison_digest: comparison.digest }),
      },
    ), comparison);
    state.supplyQuotePreview = preview;
    state.quotePreview = preview.body.quote_preview;
    state.quoteApproval = null;
    state.quoteCompletion = null;
    state.page = "deals";
    state.apiError = "";
    showToast("Quote preview matches the selected offer. Approval remains a separate step.");
  } catch (error) {
    state.supplyQuotePreview = null;
    state.quotePreview = null;
    state.supplyQuoteError = error.message;
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    state.supplyQuoteLoading = false;
    setBusy(false);
    render();
    if (state.page === "market" && state.supplyQuoteError) {
      pageRoot.querySelector(".supply-error")?.focus({ preventScroll: true });
    } else if (state.page === "deals") {
      pageRoot.focus({ preventScroll: true });
    }
  }
}

async function refreshWorkloadTrace(mode = state.workloadTraceMode) {
  state.workloadTraceMode = mode;
  try {
    const dataStatus = mode.startsWith("data-status-")
      ? mode.slice("data-status-".length)
      : "";
    const path = mode === "seller-failure-preview"
      ? "/api/v1/market/workload-trace/failure-preview"
      : mode === "inconclusive-preview"
        ? "/api/v1/market/workload-trace/inconclusive-preview"
        : mode === "duplicate-disposition-preview"
          ? "/api/v1/market/workload-trace/duplicate-disposition-preview"
        : ["missing", "conflicting", "denied"].includes(dataStatus)
          ? `/api/v1/market/workload-trace/data-status-preview/${encodeURIComponent(dataStatus)}`
          : "/api/v1/market/workload-trace";
    state.workloadTrace = await requestJson(path);
    state.workloadTraceError = "";
  } catch (error) {
    state.workloadTrace = null;
    state.workloadTraceError = error.message;
  }
}

async function loadWorkloadTraceAndFocus(mode = state.workloadTraceMode) {
  await refreshWorkloadTrace(mode);
  render();
  pageRoot.querySelector(".workload-trace-panel h2")?.focus({ preventScroll: true });
}

async function refreshAgentWorkspace() {
  state.agentSnapshot = await requestJson("/api/v1/agent/status");
}

async function runAgentAction(path, body, successMessage) {
  if (state.agentBusy) return;
  state.agentBusy = true;
  state.agentActionErrorCode = "";
  state.apiError = "";
  render();
  const connecting = path.endsWith("/connect");
  showToast(connecting ? "Connecting to the selected local process." : "Waiting for a response from the local ACP process.");
  focusAgentWorkspace(connecting ? "#agent-workspace-title" : '[data-action="agent-cancel"]');
  try {
    state.agentSnapshot = await requestJson(path, {
      method: "POST",
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    if (successMessage) showToast(successMessage);
  } catch (error) {
    state.agentActionErrorCode = String(error.code || "UNKNOWN_ERROR").slice(0, 80);
    state.apiError = error.message;
    showToast(error.message, true);
    await refreshAgentWorkspace().catch(() => {});
  } finally {
    state.agentBusy = false;
    render();
    focusAgentWorkspace(connecting ? "#agent-workspace-title" : "#agent-prompt");
  }
}

async function controlAgent(path, successMessage) {
  if (state.agentControlBusy) return;
  state.agentControlBusy = true;
  state.agentActionErrorCode = "";
  render();
  showToast(path.endsWith("/cancel")
    ? "Requesting cancellation. HyperAI will report process exit only after it is confirmed."
    : "Disconnecting the local ACP session.");
  focusAgentWorkspace("#agent-workspace-title");
  try {
    state.agentSnapshot = await requestJson(path, { method: "POST", body: "{}" });
    if (successMessage) showToast(successMessage);
  } catch (error) {
    state.agentActionErrorCode = String(error.code || "UNKNOWN_ERROR").slice(0, 80);
    state.apiError = error.message;
    showToast(error.message, true);
    await refreshAgentWorkspace().catch(() => {});
  } finally {
    state.agentControlBusy = false;
    render();
    focusAgentWorkspace("#agent-workspace-title");
  }
}

async function runTradeResearch(draft) {
  if (state.tradeBusy || !state.tradeCatalog) return;
  state.tradeDraft = draft;
  state.tradeBusy = true;
  state.apiError = "";
  render();
  const expectedDatasetDigest = state.tradeCatalog.dataset_digest;
  let completed = false;
  try {
    const [series, scenario, paper] = await Promise.all([
      requestJson(`/api/v1/trade/series/${encodeURIComponent(draft.primary)}`),
      requestJson("/api/v1/trade/scenario", {
        method: "POST",
        body: JSON.stringify({
          expected_dataset_digest: expectedDatasetDigest,
          primary_instrument_id: draft.primary,
          comparison_instrument_id: draft.comparison,
          price_shock_bps: draft.priceShockBps,
          availability_delta_units: draft.availabilityDeltaUnits,
        }),
      }),
      requestJson("/api/v1/trade/paper", {
        method: "POST",
        body: JSON.stringify({
          expected_dataset_digest: expectedDatasetDigest,
          baseline_instrument_id: draft.baseline,
          requested_units: draft.requestedUnits,
        }),
      }),
    ]);
    state.tradeSeries = series;
    state.tradeScenario = scenario;
    state.tradePaper = paper;
    completed = true;
    showToast("Local synthetic Trade research updated. No orders or external calls were made.");
  } catch (error) {
    state.apiError = error.message;
    if (error.status === 409) {
      state.tradeCatalog = await requestJson("/api/v1/trade/catalog").catch(() => state.tradeCatalog);
    }
    showToast(error.message, true);
  } finally {
    state.tradeBusy = false;
    render();
    if (state.page === "trade") {
      focusPageElement(completed ? "#trade-result-heading" : "#trade-form button[type='submit']");
    }
  }
}

function tradeMoney(value) {
  const amount = Number(value);
  if (!Number.isFinite(amount)) return "Unavailable";
  return `${amount < 0 ? "−" : amount > 0 ? "+" : ""}${money(Math.abs(amount))}`;
}

function tradeAvailability(value) {
  return value === null || value === undefined ? "Unknown" : `${escapeHtml(value)} GPUs`;
}

async function runEvalQualification() {
  if (state.evalBusy || !state.evalCatalog) return;
  state.evalBusy = true;
  state.apiError = "";
  const catalog = state.evalCatalog;
  const retry = state.evalRetry;
  const request = retry && retry.expected_suite_digest === catalog.suite_digest
    ? retry
    : {
      suite_id: catalog.suite_id,
      expected_suite_digest: catalog.suite_digest,
      pair_id: catalog.pair_id,
      idempotency_key: localRequestKey("eval-run"),
    };
  state.evalRetry = request;
  let completed = false;
  render();
  try {
    state.evalReport = await requestJson("/api/v1/eval/run", {
      method: "POST",
      body: JSON.stringify(request),
    });
    state.evalRetry = null;
    completed = true;
    showToast("Synthetic harness check recorded. No agent or model ran.");
  } catch (error) {
    state.apiError = error.message;
    if (error.status === 409) {
      state.evalRetry = null;
      await refreshEvalWorkspace().catch(() => {});
    }
    showToast(error.message, true);
  } finally {
    state.evalBusy = false;
    render();
    if (state.page === "eval") {
      focusPageElement(completed ? "#eval-report-heading" : "[data-action='run-eval']");
    }
  }
}

async function previewFleetOperation(nodeId, operation) {
  if (state.fleetBusy) return;
  state.fleetNodeId = nodeId;
  state.fleetOperation = operation;
  state.fleetBusy = true;
  state.apiError = "";
  render();
  let completed = false;
  try {
    const request = { node_id: nodeId, operation };
    if (state.fleetPlan?.preview_digest) request.expected_preview_digest = state.fleetPlan.preview_digest;
    state.fleetPlan = await requestJson("/api/v1/fleet/operation-preview", {
      method: "POST",
      body: JSON.stringify(request),
    });
    completed = true;
    showToast(state.fleetPlan.preview_status === "preview_changed"
      ? "The intent changed. Review the updated digest; no action was taken."
      : "Advisory preview ready. No resource was contacted or reserved.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    state.fleetBusy = false;
    render();
    if (state.page === "fleet") {
      focusPageElement(completed ? "#fleet-plan-heading" : "#fleet-plan-form button[type='submit']");
    }
  }
}

async function transitionFleetSimulation(operation, fault, gpuUnits) {
  if (state.fleetSimulationBusy || !state.fleetSimulation) return;
  state.fleetSimulationOperation = operation;
  state.fleetSimulationFault = fault;
  state.fleetSimulationGpuUnits = gpuUnits;
  state.fleetSimulationBusy = true;
  state.apiError = "";
  const expected = state.fleetSimulation.state_digest;
  const retry = state.fleetSimulationRetry;
  const request = retry
    && retry.expected_state_digest === expected
    && retry.operation === operation
    && retry.fault === fault
    && retry.gpu_units === (operation === "launch" ? gpuUnits : null)
    ? retry
    : {
      expected_state_digest: expected,
      idempotency_key: localRequestKey("fleet-sim"),
      operation,
      gpu_units: operation === "launch" ? gpuUnits : null,
      fault,
    };
  state.fleetSimulationRetry = request;
  render();
  try {
    const transition = await requestJson("/api/v1/fleet/simulation/transition", {
      method: "POST",
      body: JSON.stringify(request),
    });
    state.fleetSimulationTransition = transition;
    state.fleetSimulation = await requestJson("/api/v1/fleet/simulation");
    state.fleetSimulationRetry = null;
    if (transition.transition_status === "outcome_unknown_reconciliation_required") {
      showToast("Outcome is uncertain in the scenario. Reconcile before the next lifecycle step.");
    } else if (transition.transition_status === "partial_effect_cleanup_required") {
      showToast("Partial virtual effect recorded. Cleanup is required before continuing.");
    } else {
      showToast(`Simulator recorded ${transition.transition_status.replaceAll("_", " ")}. No real resource was touched.`);
    }
  } catch (error) {
    if (error.status === 409) {
      try {
        state.fleetSimulation = await requestJson("/api/v1/fleet/simulation");
        state.fleetSimulationRetry = null;
      } catch {}
    }
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    state.fleetSimulationBusy = false;
    render();
  }
}

function setBusy(busy) {
  state.busy = busy;
  document.querySelectorAll("[data-action='run-demo'], [data-action='review-quote'], [data-action='preview-supply-quote'], [data-action='approve-quote'], [data-action='complete-quote'], [data-action='verify-receipt'], [data-action='verify-hyperintel'], [data-action='reconcile-hyperintel'], [data-action='verify-registry-capture']").forEach((button) => {
    button.disabled = busy;
    button.setAttribute("aria-busy", String(busy));
  });
}

function localRequestKey(prefix) {
  const random = globalThis.crypto?.randomUUID?.().replaceAll("-", "") || `${Date.now()}${Math.random().toString(16).slice(2)}`;
  return `${prefix}-${random}`.slice(0, 64);
}

function defaultDataSql(datasetId) {
  return datasetId === "hyperai.gpudiff_public_gpu_offers.v1"
    ? "SELECT provider, gpu_model, price_decimal, unit, region, observed_at, provenance_url FROM dataset LIMIT 100"
    : "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
}

function focusDataQueryStatus() {
  if (state.page === "data") pageRoot.querySelector("#data-query-status")?.focus({ preventScroll: true });
}

function setDataActionStatus(kind, message, isError = false) {
  if (kind === "model") {
    state.dataModelStatus = message;
    state.dataModelStatusError = isError;
  } else {
    state.dataViewStatus = message;
    state.dataViewStatusError = isError;
  }
}

function focusDataActionStatus(kind) {
  if (state.page === "data") pageRoot.querySelector(`#data-${kind}-status`)?.focus({ preventScroll: true });
}

async function refreshLocalDataCatalog() {
  const [catalog, models, views, gpudiffSchedule] = await Promise.all([
    requestJson("/api/v1/data/catalog"),
    requestJson("/api/v1/data/models"),
    requestJson("/api/v1/data/views"),
    requestJson("/api/v1/data/gpudiff/schedule"),
  ]);
  state.dataCatalog = catalog;
  state.dataModels = models;
  state.dataViews = views;
  state.gpudiffSchedule = gpudiffSchedule;
  const sourceIndicator = document.getElementById("data-source-indicator");
  if (sourceIndicator) {
    sourceIndicator.textContent = catalog.datasets.some(
      (dataset) => dataset.dataset_id === "hyperai.gpudiff_public_gpu_offers.v1",
    ) ? "LOCAL · GPUDIFF SNAPSHOT" : "LOCAL · SYNTHETIC";
  }
  if (!catalog.datasets.some((dataset) => dataset.dataset_id === state.dataDatasetId)) {
    state.dataDatasetId = catalog.datasets[0]?.dataset_id || "";
    state.dataSql = defaultDataSql(state.dataDatasetId);
  }
  state.dataSyntheticRelease = catalog.datasets.some(
    (dataset) => dataset.dataset_id === "hyperai.synthetic_market_observations_json.v1",
  )
    ? await requestJson("/api/v1/data/synthetic-release")
    : null;
}

async function seedSyntheticData() {
  state.dataBusy = true;
  state.apiError = "";
  render();
  try {
    const preview = await requestJson("/api/v1/data/synthetic/preview");
    const manifest = await requestJson("/api/v1/data/synthetic/seed", {
      method: "POST",
      headers: { "Idempotency-Key": localRequestKey("seed") },
      body: JSON.stringify({ preview_digest: preview.preview_digest }),
    });
    await refreshLocalDataCatalog();
    await refreshWorkloadTrace();
    showToast(`Local synthetic snapshot ready: ${manifest.dataset_id}`);
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    state.dataBusy = false;
    render();
  }
}

async function refreshGpudiffData() {
  if (state.dataBusy) return;
  state.dataBusy = true;
  state.apiError = "";
  render();
  let imported = null;
  try {
    imported = await requestJson("/api/v1/data/gpudiff/refresh", {
      method: "POST",
      body: "{}",
    });
    state.dataImportReceipt = imported;
    await refreshLocalDataCatalog();
    state.dataDatasetId = imported.manifest.dataset_id;
    state.dataSql = defaultDataSql(imported.manifest.dataset_id);
    showToast(`Imported ${imported.receipt.row_count} GPUDiff rows · ${shortDigest(imported.receipt.raw_source_digest)}`);
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    state.dataBusy = false;
    render();
  }
  if (imported) await runLocalDataQuery(imported.manifest.dataset_id, state.dataSql);
}

async function updateGpudiffSchedule(action) {
  if (state.gpudiffScheduleBusy) return;
  state.gpudiffScheduleBusy = true;
  state.apiError = "";
  render();
  try {
    if (action === "status") {
      state.gpudiffSchedule = await requestJson("/api/v1/data/gpudiff/schedule");
    } else {
      const path = action === "start"
        ? "/api/v1/data/gpudiff/schedule"
        : "/api/v1/data/gpudiff/schedule/stop";
      state.gpudiffSchedule = await requestJson(path, { method: "POST", body: "{}" });
    }
    if (action === "start" && state.gpudiffSchedule.state === "active") {
      await refreshLocalDataCatalog();
      const datasetId = "hyperai.gpudiff_public_gpu_offers.v1";
      state.dataDatasetId = datasetId;
      state.dataSql = defaultDataSql(datasetId);
      await runLocalDataQuery(datasetId, state.dataSql);
    }
    const stateLabel = state.gpudiffSchedule.state.replaceAll("_", " ");
    showToast(`GPUDiff schedule ${stateLabel}${state.gpudiffSchedule.last_error_code ? ` · ${state.gpudiffSchedule.last_error_code}` : ""}`, state.gpudiffSchedule.state === "paused_capacity");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    state.gpudiffScheduleBusy = false;
    render();
  }
}

async function runLocalDataQuery(datasetId, sql = state.dataSql) {
  if (!datasetId || state.dataBusy) return;
  state.dataDatasetId = datasetId;
  state.dataSql = String(sql).trim().slice(0, 4096);
  state.dataBusy = true;
  state.dataQueryError = false;
  state.dataResult = null;
  state.dataModelDraft = null;
  state.dataModelPreview = null;
  state.dataModelStatus = "";
  state.dataModelStatusError = false;
  state.dataViewDraft = null;
  state.dataViewPreview = null;
  state.dataViewStatus = "";
  state.dataViewStatusError = false;
  state.dataQueryStatus = "Query running. Results are limited to 1,000 rows and 256 KiB.";
  state.apiError = "";
  const queryId = localRequestKey("web-query");
  state.activeDataQueryId = queryId;
  render();
  focusDataQueryStatus();
  let feedbackMessage = "";
  let feedbackIsError = false;
  try {
    state.dataResult = await requestJson("/api/v1/data/query", {
      method: "POST",
      body: JSON.stringify({
        query_id: queryId,
        request: { dataset_id: datasetId, sql: state.dataSql, format: "json", max_rows: 100 },
      }),
    });
    state.dataModelPreview = null;
    state.dataModelDraft = null;
    state.dataQueryStatus = `Query complete: ${state.dataResult.row_count} rows across ${state.dataResult.columns.length} columns from ${datasetId}.`;
    feedbackMessage = `Query returned ${state.dataResult.row_count} local rows.`;
  } catch (error) {
    state.apiError = error.message;
    state.dataQueryError = true;
    const errorCode = String(error.code || "UNKNOWN_ERROR").slice(0, 80);
    const httpStatus = Number.isInteger(error.status) ? ` (HTTP ${error.status})` : "";
    state.dataQueryStatus = `Query failed: ${errorCode}${httpStatus}: ${error.message}`;
    feedbackMessage = `${errorCode}${httpStatus}: ${error.message}`;
    feedbackIsError = true;
  } finally {
    state.activeDataQueryId = null;
    state.dataBusy = false;
    render();
    if (state.page === "data") focusDataQueryStatus();
    else if (feedbackMessage) showToast(feedbackMessage, feedbackIsError);
  }
}

async function cancelLocalDataQuery() {
  if (!state.activeDataQueryId) return;
  try {
    await requestJson(`/api/v1/data/query/${encodeURIComponent(state.activeDataQueryId)}/cancel`, {
      method: "POST",
      body: "{}",
    });
    showToast("Cancellation requested for this local query.");
  } catch (error) {
    if (!/not found|conflict/i.test(error.message)) showToast(error.message, true);
  }
}

async function previewDataModel(modelId) {
  if (state.dataBusy) return;
  const dataset = state.dataCatalog?.datasets.find((item) => item.dataset_id === state.dataDatasetId);
  if (!dataset || !state.dataSql.trim()) return;
  const model = {
    schema_version: "hyperai.data.model.v1",
    model_id: String(modelId).trim().slice(0, 64),
    dataset_id: dataset.dataset_id,
    snapshot_digest: dataset.snapshot_digest,
    sql: state.dataSql.trim(),
  };
  state.dataBusy = true;
  state.apiError = "";
  state.dataModelDraft = model;
  state.dataModelPreview = null;
  setDataActionStatus("model", "Preparing a digest-bound model preview…");
  render();
  focusDataActionStatus("model");
  try {
    state.dataModelPreview = await requestJson("/api/v1/data/models/preview", {
      method: "POST",
      body: JSON.stringify(model),
    });
    setDataActionStatus("model", `Preview ready. Review digest ${state.dataModelPreview.preview_digest}, then confirm the local save.`);
  } catch (error) {
    state.apiError = error.message;
    setDataActionStatus("model", `Preview failed: ${error.message}. Edit the model name or query, then retry.`, true);
    showToast(error.message, true);
  } finally {
    state.dataBusy = false;
    render();
    if (state.page === "data") focusDataActionStatus("model");
  }
}

async function savePreviewedDataModel() {
  if (!state.dataModelDraft || !state.dataModelPreview || state.dataBusy) return;
  state.dataBusy = true;
  state.apiError = "";
  setDataActionStatus("model", "Saving the model definition in this HyperAI home…");
  render();
  focusDataActionStatus("model");
  try {
    state.dataModelSaved = await requestJson("/api/v1/data/models/save", {
      method: "POST",
      headers: { "Idempotency-Key": localRequestKey("model") },
      body: JSON.stringify({ model: state.dataModelDraft, preview_digest: state.dataModelPreview.preview_digest }),
    });
    await refreshLocalDataCatalog();
    state.dataModelPreview = null;
    setDataActionStatus("model", `Saved model ${state.dataModelSaved.definition_id}. Definition digest ${state.dataModelSaved.definition_digest}.`);
    showToast(`Saved local model ${state.dataModelSaved.definition_id}.`);
  } catch (error) {
    state.apiError = error.message;
    state.dataModelPreview = null;
    setDataActionStatus("model", `Model save failed: ${error.message}. The model name is retained so you can edit and preview again.`, true);
    showToast(error.message, true);
  } finally {
    state.dataBusy = false;
    render();
    if (state.page === "data") focusDataActionStatus("model");
  }
}

async function previewDataView(viewId, presentation, columns) {
  if (state.dataBusy || !state.dataModelSaved || !columns.length) return;
  const view = {
    schema_version: "hyperai.data.view.v1",
    view_id: String(viewId).trim().slice(0, 64),
    model_digest: state.dataModelSaved.definition_digest,
    presentation,
    columns,
  };
  state.dataBusy = true;
  state.apiError = "";
  state.dataViewDraft = view;
  state.dataViewPreview = null;
  setDataActionStatus("view", "Preparing a digest-bound view preview…");
  render();
  focusDataActionStatus("view");
  try {
    state.dataViewPreview = await requestJson("/api/v1/data/views/preview", {
      method: "POST",
      body: JSON.stringify(view),
    });
    setDataActionStatus("view", `Preview ready. Review digest ${state.dataViewPreview.preview_digest}, then confirm the local save.`);
  } catch (error) {
    state.apiError = error.message;
    setDataActionStatus("view", `Preview failed: ${error.message}. Edit the view fields, then retry.`, true);
    showToast(error.message, true);
  } finally {
    state.dataBusy = false;
    render();
    if (state.page === "data") focusDataActionStatus("view");
  }
}

async function savePreviewedDataView() {
  if (!state.dataViewDraft || !state.dataViewPreview || state.dataBusy) return;
  state.dataBusy = true;
  state.apiError = "";
  setDataActionStatus("view", "Saving view metadata in this HyperAI home…");
  render();
  focusDataActionStatus("view");
  try {
    state.dataViewSaved = await requestJson("/api/v1/data/views/save", {
      method: "POST",
      headers: { "Idempotency-Key": localRequestKey("view") },
      body: JSON.stringify({ view: state.dataViewDraft, preview_digest: state.dataViewPreview.preview_digest }),
    });
    await refreshLocalDataCatalog();
    state.dataViewPreview = null;
    setDataActionStatus("view", `Saved view ${state.dataViewSaved.definition_id}. Definition digest ${state.dataViewSaved.definition_digest}.`);
    showToast(`Saved local view ${state.dataViewSaved.definition_id}.`);
  } catch (error) {
    state.apiError = error.message;
    state.dataViewPreview = null;
    setDataActionStatus("view", `View save failed: ${error.message}. The view fields are retained so you can edit and preview again.`, true);
    showToast(error.message, true);
  } finally {
    state.dataBusy = false;
    render();
    if (state.page === "data") focusDataActionStatus("view");
  }
}

async function runDemo() {
  state.quotePreview = null;
  state.supplyQuotePreview = null;
  state.quoteApproval = null;
  state.quoteCompletion = null;
  setBusy(true);
  try {
    state.demo = await requestJson("/api/v1/market/demo", { method: "POST", body: "{}" });
    state.verification = null;
    state.apiError = "";
    showToast("Synthetic market flow completed in memory.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
  }
}

async function reviewSyntheticQuote() {
  state.quotePreview = null;
  state.supplyQuotePreview = null;
  state.quoteApproval = null;
  state.quoteCompletion = null;
  state.apiError = "";
  setBusy(true);
  try {
    state.quotePreview = await requestJson("/api/v1/market/quote-preview", { method: "POST", body: "{}" });
    state.page = "deals";
    showToast("Synthetic quote ready for review. No approval has been created.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
    if (state.page === "deals") pageRoot.focus({ preventScroll: true });
  }
}

async function approveSyntheticQuote() {
  if (!state.quotePreview?.digest || state.quoteApproval) return;
  setBusy(true);
  let completed = false;
  try {
    const approval = await requestJson("/api/v1/market/quote-approve", {
      method: "POST",
      body: JSON.stringify({ preview_digest: state.quotePreview.digest }),
    });
    if (approval.preview_digest !== state.quotePreview.digest) {
      throw new Error("The returned approval does not bind to the quote shown here.");
    }
    state.quoteApproval = approval;
    await refreshMarketHistory();
    completed = true;
    state.apiError = "";
    showToast("Synthetic approval recorded. No seller was contacted and no funds moved.");
  } catch (error) {
    state.quoteApproval = null;
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
    if (state.page === "deals") {
      focusPageElement(completed ? "#quote-approval-heading" : "[data-action='approve-quote']");
    }
  }
}

async function completeSyntheticQuote() {
  if (!state.quotePreview?.digest || !state.quoteApproval?.deal_digest || state.quoteCompletion) return;
  setBusy(true);
  let completed = false;
  try {
    const completion = await requestJson("/api/v1/market/quote-complete", {
      method: "POST",
      body: JSON.stringify({
        preview_digest: state.quotePreview.digest,
        deal_digest: state.quoteApproval.deal_digest,
      }),
    });
    if (completion.preview_digest !== state.quotePreview.digest
      || completion.deal_digest !== state.quoteApproval.deal_digest
      || completion.demo?.report?.deal_digest !== state.quoteApproval.deal_digest) {
      throw new Error("The returned receipt does not bind to the reviewed quote and approval.");
    }
    state.quoteCompletion = completion;
    state.demo = completion.demo;
    await refreshMarketHistory();
    state.verification = null;
    completed = true;
    state.apiError = "";
    showToast("Synthetic mock fulfillment and receipt completed. No funds moved.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
    if (state.page === "deals") {
      focusPageElement(completed ? "#quote-completion-heading" : "[data-action='complete-quote']");
    }
  }
}

async function verifyReceipt() {
  if (!state.demo?.receipt) return;
  setBusy(true);
  try {
    state.verification = await requestJson("/api/v1/market/verify", { method: "POST", body: JSON.stringify(state.demo.receipt) });
    showToast("Local contract integrity check completed.");
  } catch (error) {
    state.verification = null;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
  }
}

async function verifyHyperIntelBundle() {
  const evidenceFile = document.getElementById("hyperintel-evidence-file")?.files?.[0];
  const trustedKeysFile = document.getElementById("hyperintel-trust-file")?.files?.[0];
  if (!evidenceFile || !trustedKeysFile) {
    state.apiError = "Choose both the HyperIntel evidence input and caller-controlled trusted-key JSON.";
    showToast(state.apiError, true);
    render();
    return;
  }
  if (evidenceFile.size + trustedKeysFile.size > 150_000) {
    state.apiError = "The selected files exceed the 150 KB local verification limit.";
    showToast(state.apiError, true);
    render();
    return;
  }

  state.hyperintelVerification = null;
  state.apiError = "";
  setBusy(true);
  try {
    const [evidenceText, trustedKeysText] = await Promise.all([
      evidenceFile.text(),
      trustedKeysFile.text(),
    ]);
    try {
      JSON.parse(evidenceText);
      JSON.parse(trustedKeysText);
    } catch {
      throw new Error("Both selected files must contain valid JSON.");
    }
    state.hyperintelVerification = await requestJson("/api/v1/inference/verify-bundle", {
      method: "POST",
      body: JSON.stringify({ evidence_input: evidenceText, trusted_keys_input: trustedKeysText }),
    });
    showToast("Local HyperIntel contract verification passed under the supplied trust registry.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
  }
}

async function reconcileHyperIntelExecution() {
  const executionFile = document.getElementById("hyperintel-execution-file")?.files?.[0];
  const trustedKeysFile = document.getElementById("hyperintel-reconciliation-trust-file")?.files?.[0];
  if (!executionFile || !trustedKeysFile) {
    state.apiError = "Choose both the complete execution-lineage JSON and caller-controlled public-key registry.";
    showToast(state.apiError, true);
    render();
    return;
  }
  if (executionFile.size + trustedKeysFile.size > 150_000) {
    state.apiError = "The selected lineage and trust files exceed the 150 KB local limit.";
    showToast(state.apiError, true);
    render();
    return;
  }

  state.hyperintelReconciliation = null;
  state.apiError = "";
  setBusy(true);
  try {
    const [executionText, trustedKeysText] = await Promise.all([
      executionFile.text(),
      trustedKeysFile.text(),
    ]);
    state.hyperintelReconciliation = await requestJson("/api/v1/inference/reconcile-execution", {
      method: "POST",
      body: JSON.stringify({ execution_input: executionText, trusted_keys_input: trustedKeysText }),
    });
    showToast("HyperIntel's local execution lineage was reconstructed under the supplied trust keys.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
  }
}

async function verifySimpleMarketCapture() {
  const captureFile = document.getElementById("scm-capture-file")?.files?.[0];
  const trustFile = document.getElementById("scm-trust-file")?.files?.[0];
  if (!captureFile || !trustFile) {
    state.apiError = "Choose both the signed listings capture and caller-controlled trust pins.";
    showToast(state.apiError, true);
    render();
    return;
  }
  if (captureFile.size + trustFile.size > 400_000) {
    state.apiError = "The selected capture and trust files exceed the 400 KB local limit.";
    showToast(state.apiError, true);
    render();
    return;
  }

  state.simpleMarketCapture = null;
  state.apiError = "";
  setBusy(true);
  try {
    const [captureText, trustText] = await Promise.all([captureFile.text(), trustFile.text()]);
    try {
      JSON.parse(captureText);
      JSON.parse(trustText);
    } catch {
      throw new Error("The capture and trust files must contain valid JSON.");
    }
    state.simpleMarketCapture = await requestJson("/api/v1/market/registry-capture/verify", {
      method: "POST",
      body: JSON.stringify({ capture_json: captureText, trust_json: trustText }),
    });
    showToast("The registry response verified under the caller-supplied pins.");
  } catch (error) {
    state.apiError = error.message;
    showToast(error.message, true);
  } finally {
    setBusy(false);
    render();
  }
}

function connectionStateLabel(value) {
  if (value === "local_offline_verification_and_reconciliation_available") return "OFFLINE EVIDENCE READY";
  if (value === "local_offline_verifier_available") return "OFFLINE IMPORT READY";
  if (value === "local_offline_capture_verifier_available") return "CAPTURE VERIFY READY";
  return String(value || "unknown").replaceAll("_", " ");
}

function statusClass(value) {
  if (value === "local_offline_verification_and_reconciliation_available") return "state-pill state-offline";
  if (value === "local_synthetic") return "state-pill state-local";
  if (value === "local_offline_verifier_available") return "state-pill state-offline";
  if (value === "local_offline_capture_verifier_available") return "state-pill state-offline";
  if (value === "disabled" || value === "planned_not_in_runtime") return "state-pill state-muted";
  if (value === "reference_only_rights_unresolved") return "state-pill state-reference";
  return "state-pill state-pending";
}

function heading(kicker, title, description, actions = "") {
  return `<div class="page-heading"><div><p class="eyebrow">${escapeHtml(kicker)}</p><h1 id="workspace-page-title" tabindex="-1">${escapeHtml(title)}</h1><p class="page-description">${escapeHtml(description)}</p></div>${actions ? `<div class="heading-actions">${actions}</div>` : ""}</div>`;
}

function button(label, action, className = "button button-primary", disabled = false) {
  return `<button class="${className}" data-action="${escapeHtml(action)}" ${disabled || state.busy ? "disabled" : ""}>${escapeHtml(label)}</button>`;
}

function demoReceipt() { return state.demo?.receipt || null; }

function renderOverview() {
  const report = state.demo?.report;
  const receipt = demoReceipt();
  const stats = report ? `<div class="metric-row"><div class="metric"><span>Registry sources</span><strong>${escapeHtml(report.source_count)}</strong><small>${escapeHtml(report.duplicate_count)} duplicate listing collapsed</small></div><div class="metric"><span>Selected offer</span><strong class="metric-compact">${escapeHtml(report.selected_offer_id)}</strong><small>Deterministic buyer constraints</small></div><div class="metric"><span>Deal amount</span><strong>${money(receipt.body.approved_deal.signed_quote.quote.quoted_total_microusd)}</strong><small>Local mock only · no funds move</small></div></div>` : `<div class="metric-row"><div class="metric"><span>External connectors</span><strong>0</strong><small>None are active in this shell</small></div><div class="metric"><span>Market mode</span><strong class="metric-compact">Synthetic</strong><small>Deterministic local contracts</small></div><div class="metric"><span>Trade</span><strong class="metric-compact">Research</strong><small>Synthetic paper scenarios · no orders</small></div></div>`;
  return `${heading("COMPUTE WORKSPACE", "Compute, with context.", "Move from a market signal to a reviewed deal while keeping source, evidence, and authority visible.")}
    <section class="hero-panel"><div class="hero-copy"><span class="hero-kicker"><i></i> A composed HyperAI workspace</span><h2>One clear path from<br /><em>signal to receipt.</em></h2><p>Explore the local market flow and see exactly which records are observations, approvals, seller reports, and mock decisions.</p><div class="hero-actions">${button(state.busy ? "Preparing quote…" : "Review synthetic quote", "review-quote")}${button(state.busy ? "Running lifecycle…" : "Run full synthetic lifecycle", "run-demo", "button button-quiet")}${report ? `<button class="button button-quiet" data-page="deals">Review this deal <span aria-hidden="true">↗</span></button>` : ""}</div></div><div class="hero-map" aria-label="Offer, deal, and evidence remain separate records"><div class="map-orbit orbit-one"></div><div class="map-orbit orbit-two"></div><div class="map-node node-market"><span>01</span><strong>Market</strong><small>Observed</small></div><div class="map-node node-deal"><span>02</span><strong>Deal</strong><small>Approved</small></div><div class="map-node node-proof"><span>03</span><strong>Evidence</strong><small>Attributed</small></div><div class="map-link"></div></div><div class="hero-foot"><span>LOCAL FIXTURE</span><span>NO PROVIDER CALLS</span><span>NO FUNDS MOVE</span></div></section>
    ${stats}
    ${report ? renderLifecycle(receipt) : renderIntegrationStrip()}
    <div class="section-heading"><div><p class="eyebrow">SYSTEM MAP</p><h2>Connected by contract, not by assumption.</h2></div><button class="text-button" data-page="access">Review boundaries <span aria-hidden="true">→</span></button></div>
    ${renderConnectionCards(4)}`;
}

function renderLifecycle(receipt) {
  const quote = receipt.body.approved_deal.signed_quote.quote;
  return `<section class="panel lifecycle-panel"><div class="panel-heading"><div><p class="eyebrow">LOCAL SYNTHETIC LIFECYCLE</p><h2>Deal path in this run</h2></div><span class="tag tag-caution">Mock only</span></div><div class="lifecycle-steps"><div class="lifecycle-step"><span class="step-index">01</span><span><strong>Offer selected</strong><small>${escapeHtml(receipt.body.selected_offer_id)} · signed fixture</small></span><b>OBSERVED</b></div><div class="lifecycle-step"><span class="step-index">02</span><span><strong>Buyer terms bound</strong><small>${quote.units} units · ${quote.duration_hours} hours · ${money(quote.amount_cap_microusd)} cap</small></span><b>APPROVED</b></div><div class="lifecycle-step"><span class="step-index">03</span><span><strong>Seller reported complete</strong><small>${money(receipt.body.fulfillment.billed_microusd)} billed · seller-signed receipt</small></span><b>ATTRIBUTED</b></div><div class="lifecycle-step"><span class="step-index">04</span><span><strong>Local mock decision</strong><small>${escapeHtml(receipt.body.verifier_decision.reason_code)} · no independent verifier</small></span><b>MOCK</b></div></div><div class="digest-line"><span>RECEIPT DIGEST</span><code title="${escapeHtml(receipt.digest)}">${escapeHtml(shortDigest(receipt.digest))}</code><button class="text-button" data-page="evidence">View evidence →</button></div></section>`;
}

function renderIntegrationStrip() {
  return `<section class="integration-strip"><div><span class="strip-index">A</span><span><strong>Observe</strong><small>Seller listings and market analysis stay distinct.</small></span></div><span class="strip-arrow">→</span><div><span class="strip-index">B</span><span><strong>Authorize</strong><small>Buyer terms bind to an exact quote and cap.</small></span></div><span class="strip-arrow">→</span><div><span class="strip-index">C</span><span><strong>Review</strong><small>Receipts retain attribution and claim limits.</small></span></div></section>`;
}

function renderMarket() {
  const capturePanel = renderSimpleMarketCapture();
  const comparison = window.HyperAIMarketSupplyComparison.renderPanel({
    comparison: state.supplyComparison,
    error: state.supplyComparisonError,
    loading: state.supplyComparisonLoading,
    quoteLoading: state.supplyQuoteLoading,
    quoteError: state.supplyQuoteError,
    quotePreviewReady: Boolean(state.supplyQuotePreview),
  });
  return `${heading("DISCOVERY / BUYER SUPPLY", "Market", "Compare a buyer request with signed local offers. Source provenance, seller asking terms, and advisory signals remain separate.")}${comparison}<section class="market-capture-section"><div class="section-heading"><div><p class="eyebrow">OFFLINE SOURCE EXPLORER</p><h2>Simple Compute Market captures</h2><p>Caller-selected signed captures remain exploratory and do not enter this comparison.</p></div></div>${capturePanel}</section>${renderConnectionFootnote("Synthetic comparison only; live registry and seller connections are not configured.")}`;
}

function simpleMarketEntries(body) {
  if (Array.isArray(body)) return body;
  if (!body || typeof body !== "object") return [];
  for (const key of ["items", "listings", "data"]) {
    const value = body[key];
    if (Array.isArray(value)) {
      if (value.length > 0) return value;
      continue;
    }
    if (value == null || value === false || value === "") continue;
    return [];
  }
  return [];
}

function registryPublisherSummary(entry) {
  const identities = entry.publisher_principals?.identities;
  if (!Array.isArray(identities) || identities.length === 0) {
    return escapeHtml(entry.publisher_id ?? "Not supplied");
  }
  return identities.map((identity) =>
    `${escapeHtml(identity?.scheme ?? "unknown")} · ${escapeHtml(shortDigest(identity?.identifier))}`
  ).join("<br />");
}

function sourceScalar(value, fallback = "Unknown") {
  if (value == null) return fallback;
  if (typeof value === "string" || typeof value === "number") return String(value);
  return JSON.stringify(value);
}

function resourceClassificationLabel(classification) {
  return ({
    compute_resource: "Compute resource shape",
    api_credits_resource: "API credits shape",
    token_resource_opaque: "Token resource · opaque",
    unknown_opaque: "Unknown resource · opaque",
  })[classification] || "Unknown resource · opaque";
}

function renderScmListingExplorer(result) {
  const entries = simpleMarketEntries(result.registry_body);
  const views = result.listing_views || [];
  const query = state.scmListingQuery;
  const statuses = [...new Set(entries.map((entry) => entry.status).filter((value) => typeof value === "string"))].sort();
  const rows = entries.map((entry, index) => ({ entry, view: views[index] || { source_index: index, resource: { classification: "unknown_opaque", classification_basis: "No typed view was returned; original payload remains opaque.", fields: [], other_payload_field_count: 0 } } }));
  const matching = rows.filter(({ entry, view }) => {
    const listingId = sourceScalar(entry.listing_id ?? entry.id, "");
    const status = typeof entry.status === "string" ? entry.status : "";
    const haystack = `${listingId} ${status} ${JSON.stringify(entry)}`.toLocaleLowerCase();
    return (!query.search || haystack.includes(query.search.toLocaleLowerCase()))
      && (query.resource === "all" || view.resource.classification === query.resource)
      && (query.status === "all" || status === query.status);
  });
  const cards = matching.map(({ entry, view }) => {
    const listingId = sourceScalar(entry.listing_id ?? entry.id, `Listing ${view.source_index + 1}`);
    const resource = view.resource;
    const resourceFields = resource.fields.length
      ? `<dl class="registry-resource-fields">${resource.fields.map((field) => `<div><dt>${escapeHtml(field.label)}</dt><dd>${escapeHtml(sourceScalar(field.value))}</dd></div>`).join("")}</dl>`
      : `<p class="registry-opaque-note">No resource fields were projected. The original source object is available below.</p>`;
    const preview = JSON.stringify(entry, null, 2) || "{}";
    const listingPreview = preview.length > 2400 ? `${preview.slice(0, 2400)}…` : preview;
    return `<article class="registry-listing-card">
      <div class="registry-listing-top"><div><p class="eyebrow">SOURCE LISTING · ${escapeHtml(sourceScalar(entry.status, "status unknown"))}</p><h3>${escapeHtml(listingId)}</h3></div><span class="tag tag-neutral">Registry attributed</span></div>
      <div class="registry-listing-meta">
        <div><span>Resource view</span><strong>${escapeHtml(resourceClassificationLabel(resource.classification))}</strong></div>
        <div><span>Listing updated</span><strong>${escapeHtml(sourceScalar(entry.updated_at))}</strong></div>
        <div><span>Registry-listed principals</span><strong>${registryPublisherSummary(entry)}</strong></div>
        <div><span>Seller storefront</span><strong>${escapeHtml(sourceScalar(entry.storefront_url, "Not supplied"))}</strong></div>
      </div>
      <section class="registry-resource-summary"><div class="registry-resource-heading"><h4>Resource summary</h4><span>${escapeHtml(resource.other_payload_field_count)} other payload fields retained</span></div><p>${escapeHtml(resource.classification_basis)}</p>${resourceFields}</section>
      <details><summary>Inspect complete source listing</summary><pre>${escapeHtml(listingPreview)}</pre></details>
    </article>`;
  }).join("");
  const resourceOptions = [
    ["all", "All resource shapes"],
    ["compute_resource", "Compute resource shape"],
    ["api_credits_resource", "API credits shape"],
    ["token_resource_opaque", "Token resource · opaque"],
    ["unknown_opaque", "Unknown or unsupported · opaque"],
  ];
  return `<section class="panel registry-explorer">
    <div class="panel-heading"><div><p class="eyebrow">SIGNED CAPTURE / LOCAL QUERY</p><h2>Explore registry listings</h2></div><span class="tag tag-caution">Read-only</span></div>
    <p class="result-explanation">These filters run in this browser over the capture already verified under caller pins. They do not change signed bytes, assess listing freshness, select an offer, or alter a deal.</p>
    <form class="registry-query" id="scm-listing-query">
      <label><span>Search source fields</span><input id="scm-listing-search" name="search" type="search" maxlength="120" value="${escapeHtml(query.search)}" placeholder="Listing ID, status, resource fields" /></label>
      <label><span>Resource view</span><select name="resource">${resourceOptions.map(([value, label]) => `<option value="${value}" ${query.resource === value ? "selected" : ""}>${label}</option>`).join("")}</select></label>
      <label><span>Source status</span><select name="status"><option value="all" ${query.status === "all" ? "selected" : ""}>All statuses</option>${statuses.map((value) => `<option value="${escapeHtml(value)}" ${query.status === value ? "selected" : ""}>${escapeHtml(value)}</option>`).join("")}</select></label>
      <div class="registry-query-actions"><button class="button button-primary" type="submit">Apply filters</button><button class="button button-quiet" type="button" data-action="clear-scm-query">Reset</button></div>
    </form>
    <div class="registry-query-status" role="status" aria-live="polite">Showing ${matching.length} of ${rows.length} signed-capture listings · listing freshness not assessed.</div>
    <p class="registry-opaque-note">Recognized fields are a bounded display summary, not complete source-domain validation. Unsupported resources, settlement terms, demands, and extra fields remain source data.</p>
    ${rows.length === 0 ? `<div class="empty-inline">The signed response contains no listing records.</div>` : cards || `<div class="empty-inline">No listings match these filters. Reset the query to show the full capture.</div>`}
  </section>`;
}

function renderSimpleMarketCapture() {
  const result = state.simpleMarketCapture;
  let resultPanel = "";
  if (result) {
    resultPanel = `<section class="panel registry-capture-result">
      <div class="panel-heading"><div><p class="eyebrow">SIMPLE COMPUTE MARKET / V2 SIGNATURES</p><h2>Capture authenticated under caller pins</h2></div><span class="tag tag-caution">Exploration only</span></div>
      <p class="result-explanation">The buyer request and registry response signatures verified. The registry response does not sign the query-body hash; query application is trusted to the pinned registry implementation. Re-importing this still-fresh capture can verify again.</p>
      <div class="verification-metrics registry-metrics">
        <div><span>Listings</span><strong>${escapeHtml(result.listing_count)}</strong></div>
        <div><span>Request ID</span><strong>${escapeHtml(result.request_id)}</strong></div>
        <div><span>Buyer pin</span><strong>${escapeHtml(result.buyer_identity.scheme)} · ${escapeHtml(shortDigest(result.buyer_identity.identifier))}</strong></div>
        <div><span>Registry pin</span><strong>${escapeHtml(result.registry_identity.scheme)} · ${escapeHtml(shortDigest(result.registry_identity.identifier))}</strong></div>
      </div>
      <dl class="detail-list registry-capture-details">
        <div><dt>Capture schema</dt><dd><code>${escapeHtml(result.source_capture_schema_version)}</code></dd></div>
        <div><dt>Verification schema</dt><dd><code>${escapeHtml(result.schema_version)}</code></dd></div>
        <div><dt>Query body SHA-256</dt><dd><code class="digest-full">${escapeHtml(result.query_body_sha256)}</code></dd></div>
        <div><dt>Response body SHA-256</dt><dd><code class="digest-full">${escapeHtml(result.response_body_sha256)}</code></dd></div>
        <div><dt>Response time</dt><dd>${escapeHtml(result.response_timestamp)} Unix seconds · 300-second auth skew only</dd></div>
        <div><dt>Caller pins</dt><dd>${escapeHtml(result.buyer_trust_pin_count)} buyer · ${escapeHtml(result.registry_trust_pin_count)} registry</dd></div>
      </dl>
      <div class="capture-next-step"><p>${escapeHtml(result.listing_count)} authenticated listing objects are ready for a local, read-only query.</p><button class="button button-secondary" data-page="data" type="button">Explore listings in Data</button></div>
      <div class="claim-box"><span class="eyebrow">CLAIM CEILING</span><p>${escapeHtml(result.claim_ceiling)}</p><ul>${result.explicit_nonclaims.map((claim) => `<li>${escapeHtml(claim)}</li>`).join("")}</ul></div>
      <p class="retention-note">Capture and result stay in this page’s memory. No registry read, seller call, deal selection, or file write occurred.</p>
    </section>`;
  }

  return `<section class="panel import-panel registry-import">
    <div class="panel-heading"><div><p class="eyebrow">READ-ONLY / LOCAL FILES</p><h2>Verify a signed listings capture</h2></div><span class="tag tag-neutral">No network access</span></div>
    <p class="result-explanation">Assemble the source client’s signed <code>GET /listings</code> request envelope and query body with the response status, signature headers, and raw response body in HyperAI’s V1 capture schema. Add your buyer and registry public-key pins; HyperAI does not create or govern them.</p>
    <div class="import-grid">
      <label class="file-field" for="scm-capture-file"><span>Listings capture</span><small>HyperAI V1 envelope; the original response body stays raw JSON text</small><input id="scm-capture-file" type="file" accept=".json,application/json" /></label>
      <label class="file-field" for="scm-trust-file"><span>Buyer and registry pins</span><small>Caller-controlled public identities; one pin or an explicit two-key rotation pair per role</small><input id="scm-trust-file" type="file" accept=".json,application/json" /></label>
    </div>
    <div class="import-actions"><button class="button button-primary" data-action="verify-registry-capture" type="button" ${state.busy ? "disabled aria-busy=\"true\"" : ""}>${state.busy ? "Verifying signatures…" : "Verify capture offline"}</button><span>Maximum combined file size: 400 KB · memory only</span></div>
  </section>${resultPanel}`;
}

function renderQuoteReview() {
  const preview = state.quotePreview;
  const body = preview.body;
  const request = body.buyer_request;
  const quote = body.signed_quote.quote;
  const sourceEntry = body.catalog.body.entries.find((entry) => entry.signed_offer.offer.offer_id === body.selected_offer_id);
  const sourceLabels = sourceEntry?.source_ids?.join(" · ") || "Source label unavailable";
  const approval = state.quoteApproval;
  const supplyBinding = state.supplyQuotePreview?.body
    ? `<section class="panel quote-digest-panel"><div><p class="eyebrow">MARKET SELECTION BINDING</p><h2>Supply comparison → seller quote</h2><p>${escapeHtml(state.supplyQuotePreview.body.selected_offer_id)} was selected by the displayed buyer constraints; this digest links the two local projections.</p></div><dl class="detail-list"><div><dt>Supply-to-quote digest</dt><dd><code class="digest-full">${escapeHtml(state.supplyQuotePreview.digest)}</code></dd></div><div><dt>Comparison digest</dt><dd><code class="digest-full">${escapeHtml(state.supplyQuotePreview.body.comparison_digest)}</code></dd></div><div><dt>Catalog digest</dt><dd><code class="digest-full">${escapeHtml(state.supplyQuotePreview.body.catalog_digest)}</code></dd></div><div><dt>Signed offer digest</dt><dd><code class="digest-full">${escapeHtml(state.supplyQuotePreview.body.selected_offer_digest)}</code></dd></div><div><dt>Buyer request digest</dt><dd><code class="digest-full">${escapeHtml(state.supplyQuotePreview.body.buyer_request_digest)}</code></dd></div></dl><p class="retention-note">${escapeHtml(state.supplyQuotePreview.body.effects)}. ${escapeHtml(state.supplyQuotePreview.body.nonclaims)}.</p><p class="retention-note">No approval or funds action occurred. The existing approval control below remains separate.</p></section>`
    : "";
  const approvalPanel = approval
    ? `<section class="panel quote-approval-result" role="status" aria-live="polite"><div class="panel-heading"><div><p class="eyebrow">SYNTHETIC APPROVAL RECORDED</p><h2 id="quote-approval-heading" tabindex="-1">Quote bound to a local deal</h2></div><span class="tag tag-caution">Fixture key only</span></div><dl class="detail-list"><div><dt>Accepted at</dt><dd>${escapeHtml(approval.approved_deal.accepted_at)} · simulated fixture time</dd></div><div><dt>Deal digest</dt><dd><code class="digest-full">${escapeHtml(approval.deal_digest)}</code></dd></div><div><dt>Bound preview</dt><dd><code class="digest-full">${escapeHtml(approval.preview_digest)}</code></dd></div></dl><div class="notice notice-caution"><span class="notice-icon">i</span><div><strong>This is not purchase authorization</strong><p>The signature uses HyperAI’s deterministic synthetic buyer key. No real buyer identity, seller call, fulfillment, verifier, settlement, or funds movement is involved.</p></div></div><div class="import-actions">${button(state.busy ? "Completing mock…" : "Continue to mock fulfillment", "complete-quote")}<span>This separate step creates only a synthetic seller receipt and local mock decision; no funds move.</span></div><p class="retention-note">Approval remains in this page’s memory. The next action is still a local simulation.</p></section>`
    : `<section class="panel quote-approval-panel"><div class="panel-heading"><div><p class="eyebrow">YOUR NEXT ACTION</p><h2>Approve these exact terms?</h2></div><span class="tag tag-caution">Synthetic only</span></div><p class="result-explanation">The button signs this exact preview digest using the local synthetic buyer fixture. It demonstrates the approval contract; it does not authorize a real purchase.</p><div class="import-actions">${button(state.busy ? "Signing fixture…" : "Approve synthetic quote", "approve-quote")}<span>Only the preview digest is submitted; the market contract rebuilds the fixture before signing.</span></div></section>`;

  return `${heading("PROCUREMENT / STAGED SYNTHETIC", "Review seller quote", "Inspect the buyer constraints, seller-owned terms, provenance, and expiry before the separate approval step.")}
    <div class="notice notice-caution"><span class="notice-icon">i</span><div><strong>Simulated fixture · not a live seller quote</strong><p>This page uses a fixed synthetic market clock (${escapeHtml(body.simulated_at)}), deterministic test keys, and in-memory records. The displayed expiry is part of that simulation and is not current market freshness.</p></div></div>
    <div class="deal-banner"><div><span class="tag tag-caution">${approval ? "SYNTHETIC APPROVAL RECORDED" : "AWAITING SYNTHETIC APPROVAL"}</span><h2>${escapeHtml(body.selected_offer_id)}</h2><p>${escapeHtml(quote.seller_id)} · ${escapeHtml(request.resource_model)} · ${escapeHtml(request.region)}</p></div><div class="deal-amount"><small>Quoted total</small><strong>${money(quote.quoted_total_microusd)}</strong><span>Buyer cap ${money(quote.amount_cap_microusd)}</span></div></div>
    <div class="detail-grid"><section class="panel detail-panel"><div class="panel-heading"><div><p class="eyebrow">BUYER REQUEST</p><h2>Constraints to approve</h2></div><span class="tag tag-neutral">Synthetic buyer</span></div><dl class="detail-list"><div><dt>Resource</dt><dd>${escapeHtml(request.resource_model)} · ${request.units} units</dd></div><div><dt>Region</dt><dd>${escapeHtml(request.region)}</dd></div><div><dt>Duration</dt><dd>${request.duration_hours} hours</dd></div><div><dt>Maximum rate</dt><dd>${money(request.max_rate_microusd_per_unit_hour)} / unit-hour</dd></div><div><dt>Total spend cap</dt><dd>${money(request.max_total_microusd)}</dd></div><div><dt>Approval deadline</dt><dd>${escapeHtml(request.valid_until)} · simulated</dd></div></dl></section>
    <section class="panel detail-panel"><div class="panel-heading"><div><p class="eyebrow">SELLER QUOTE</p><h2>Terms and provenance</h2></div><span class="tag tag-neutral">Seller-owned in model</span></div><dl class="detail-list"><div><dt>Seller label</dt><dd>${escapeHtml(quote.seller_id)} · fixture identity</dd></div><div><dt>Rate</dt><dd>${money(quote.rate_microusd_per_unit_hour)} / unit-hour</dd></div><div><dt>Exact total</dt><dd>${money(quote.quoted_total_microusd)}</dd></div><div><dt>Quote expiry</dt><dd>${escapeHtml(quote.expires_at)} · simulated</dd></div><div><dt>Catalog sources</dt><dd>${escapeHtml(sourceLabels)}</dd></div><div><dt>Offer digest</dt><dd><code class="digest-full">${escapeHtml(quote.offer_digest)}</code></dd></div><div><dt>Request digest</dt><dd><code class="digest-full">${escapeHtml(quote.request_digest)}</code></dd></div></dl></section></div>
    <section class="panel quote-digest-panel"><div><p class="eyebrow">COMPLETE REVIEW BINDING</p><h2>Preview digest</h2><p>This digest covers the catalog snapshot, selected offer, buyer request, seller quote, clock, and claim boundary.</p></div><code class="digest-full">${escapeHtml(preview.digest)}</code></section>
    ${supplyBinding}
    ${approvalPanel}`;
}

function renderWorkloadTracePanel() {
  return window.HyperAIWorkloadTraceView.renderPanel({
    trace: state.workloadTrace,
    mode: state.workloadTraceMode,
    error: state.workloadTraceError,
    connections: state.status?.connections,
    escapeHtml,
    shortDigest,
    button,
  });
}

function renderDeals() {
  if (state.quotePreview && !state.quoteCompletion) return `${renderQuoteReview()}${renderMarketHistory()}`;
  const receipt = demoReceipt();
  if (!receipt) return `${heading("PROCUREMENT / LOCAL ONLY", "Deals", "Review buyer constraints, the seller quote, and the attributed fulfillment receipt as separate records.")}<section class="empty-panel"><div class="empty-mark">⇄</div><p class="eyebrow">NO DEAL IN THIS SESSION</p><h2>Start with a quote review</h2><p>Inspect a synthetic seller quote first, then choose a separate approval action. A complete mock lifecycle remains available for walkthroughs.</p>${button("Review synthetic quote", "review-quote")}${button("Run full synthetic lifecycle", "run-demo", "button button-secondary")}</section>${renderMarketHistory()}${renderWorkloadTracePanel()}`;
  const body = receipt.body;
  const quote = body.approved_deal.signed_quote.quote;
  const completionLink = state.quoteCompletion ? `<section class="panel quote-completion-link"><div class="panel-heading"><div><p class="eyebrow">STAGED FLOW COMPLETE · LOCAL MOCK ONLY</p><h2 id="quote-completion-heading" tabindex="-1">Receipt continues the quote you reviewed</h2></div><span class="tag tag-caution">No funds move</span></div><dl class="detail-list"><div><dt>Preview digest</dt><dd><code class="digest-full">${escapeHtml(state.quoteCompletion.preview_digest)}</code></dd></div><div><dt>Approved deal</dt><dd><code class="digest-full">${escapeHtml(state.quoteCompletion.deal_digest)}</code></dd></div><div><dt>Receipt</dt><dd><code class="digest-full">${escapeHtml(state.quoteCompletion.demo.receipt.digest)}</code></dd></div></dl><p class="retention-note">The seller completion is a deterministic fixture statement; the verifier and disposition are local mocks, not independent correctness or payment evidence.</p></section>` : "";
  return `${heading("PROCUREMENT / LOCAL SYNTHETIC", "Deal review", "A digest-linked walkthrough of the buyer constraints, signed quote, seller statement, and local mock decision.", button("View receipt", "evidence", "button button-secondary"))}
    ${completionLink}
    ${renderWorkloadTracePanel()}
    <div class="deal-banner"><div><span class="tag tag-caution">SYNTHETIC · NO FUNDS MOVE</span><h2>${escapeHtml(body.selected_offer_id)}</h2><p>${escapeHtml(quote.seller_id)} · ${escapeHtml(body.buyer_request.resource_model)} · ${escapeHtml(body.buyer_request.region)}</p></div><div class="deal-amount"><small>Quoted total</small><strong>${money(quote.quoted_total_microusd)}</strong><span>Cap ${money(quote.amount_cap_microusd)}</span></div></div>
    <div class="detail-grid"><section class="panel detail-panel"><div class="panel-heading"><div><p class="eyebrow">BUYER-APPROVED TERMS</p><h2>Bound request</h2></div><span class="tag tag-good">Buyer signature checked</span></div><dl class="detail-list"><div><dt>Resource</dt><dd>${escapeHtml(body.buyer_request.resource_model)} · ${body.buyer_request.units} units</dd></div><div><dt>Region</dt><dd>${escapeHtml(body.buyer_request.region)}</dd></div><div><dt>Duration</dt><dd>${body.buyer_request.duration_hours} hours</dd></div><div><dt>Max rate</dt><dd>${money(body.buyer_request.max_rate_microusd_per_unit_hour)} / unit-hour</dd></div><div><dt>Request deadline</dt><dd>${escapeHtml(body.buyer_request.valid_until)}</dd></div><div><dt>Deal digest</dt><dd><code title="${escapeHtml(state.demo.report.deal_digest)}">${escapeHtml(shortDigest(state.demo.report.deal_digest))}</code></dd></div></dl></section>
    <section class="panel detail-panel"><div class="panel-heading"><div><p class="eyebrow">SELLER-OWNED TERMS</p><h2>Quote &amp; fulfillment</h2></div><span class="tag tag-neutral">Attribution only</span></div><dl class="detail-list"><div><dt>Seller</dt><dd>${escapeHtml(quote.seller_id)}</dd></div><div><dt>Rate</dt><dd>${money(quote.rate_microusd_per_unit_hour)} / unit-hour</dd></div><div><dt>Quote expiry</dt><dd>${escapeHtml(quote.expires_at)}</dd></div><div><dt>Seller-reported billed</dt><dd>${money(body.fulfillment.billed_microusd)}</dd></div><div><dt>Reported complete</dt><dd>${escapeHtml(body.fulfillment.completed_at)}</dd></div><div><dt>Fulfillment digest</dt><dd><code title="${escapeHtml(state.demo.report.fulfillment_digest)}">${escapeHtml(shortDigest(state.demo.report.fulfillment_digest))}</code></dd></div></dl></section></div>
    <section class="panel verdict-panel"><div class="verdict-symbol">!</div><div><p class="eyebrow">LOCAL MOCK VERIFIER</p><h2>Fixture verdict · ${escapeHtml(body.verifier_decision.verdict.replaceAll("_", " "))}</h2><p>${escapeHtml(body.verifier_decision.reason_code)}. This is a fixture verdict, not independent evidence of correct computation.</p></div><div class="mock-disposition"><small>MOCK DISPOSITION</small><strong>${escapeHtml(body.mock_settlement.disposition.replaceAll("_", " "))}</strong><span>Unsigned local mock ledger · no funds moved</span></div></section>
    ${renderMarketHistory()}`;
}

function renderMarketHistory() {
  const history = state.marketHistory;
  const entries = Array.isArray(history?.entries) ? history.entries : [];
  const rows = entries.map((entry) => {
    const approval = entry?.approval || {};
    const deal = approval.approved_deal || {};
    const quote = deal.signed_quote?.quote || {};
    const receipt = entry?.completion_receipt || null;
    const status = receipt ? "MOCK COMPLETED" : "APPROVED LOCALLY";
    return `<li><article class="market-history-row"><div class="market-history-row-top"><div><span class="eyebrow">MARKET · LOCAL SYNTHETIC</span><h3>${escapeHtml(quote.seller_id || "Seller unavailable")}</h3><p>${escapeHtml(quote.units ?? "?")} units · ${escapeHtml(quote.duration_hours ?? "?")} hours · ${quote.quoted_total_microusd == null ? "Amount unavailable" : money(quote.quoted_total_microusd)}</p></div><span class="tag tag-caution">${status}</span></div><dl class="detail-list"><div><dt>Deal digest</dt><dd><code title="${escapeHtml(approval.deal_digest || "")}">${escapeHtml(shortDigest(approval.deal_digest))}</code></dd></div><div><dt>Quote digest</dt><dd><code title="${escapeHtml(approval.preview_digest || "")}">${escapeHtml(shortDigest(approval.preview_digest))}</code></dd></div>${receipt ? `<div><dt>Mock receipt</dt><dd><code title="${escapeHtml(receipt.digest || "")}">${escapeHtml(shortDigest(receipt.digest))}</code></dd></div>` : ""}</dl></article></li>`;
  }).join("");
  const stateText = state.marketHistoryLoading
    ? `<p class="market-history-status" role="status" aria-live="polite">Loading owner-only Market history…</p>`
    : state.marketHistoryError
      ? `<div class="market-history-read-error"><p class="market-history-status is-error" role="status" aria-live="polite">History could not be refreshed: ${escapeHtml(state.marketHistoryError)}${history ? " Previously loaded pages remain visible; this is not a complete history." : " No history is presented as complete."}</p>${history ? "" : `<button class="button button-secondary" type="button" data-action="refresh-market-history" ${state.marketHistoryLoading || state.busy ? "disabled" : ""}>Retry history read</button>`}</div>`
      : !history
        ? `<p class="market-history-status" role="status" aria-live="polite">Waiting for the local Market history service.</p>`
        : entries.length
          ? `<ul class="market-history-list" aria-label="Market-owned local synthetic deal history">${rows}</ul>`
          : `<p class="market-history-empty">No locally approved synthetic deals are recorded yet.</p>`;
  const pageDigests = (history?.pageDigests || []).map((digest) => `<li><code title="${escapeHtml(digest)}">${escapeHtml(shortDigest(digest))}</code></li>`).join("");
  return `<section class="panel market-history" aria-labelledby="market-history-title"><div class="panel-heading"><div><p class="eyebrow">MARKET-OWNED · DURABLE IN THIS HYPERAI HOME</p><h2 id="market-history-title">Local deal history</h2></div><span class="tag tag-neutral">No shared ledger</span></div><p class="market-history-intro">Approved quotes and mock receipts stay with the Market record. They do not establish executed compute, correct results, payment, or settlement.</p>${stateText}<div class="market-history-footer"><div><span class="eyebrow">PAGE DIGESTS · VERIFIED BY LOCAL SERVICE</span><ul>${pageDigests || "<li>Not available</li>"}</ul></div>${history?.nextCursor ? `<button class="button button-secondary" type="button" data-action="load-more-market-history" ${state.marketHistoryLoading || state.busy ? "disabled" : ""}>${state.marketHistoryLoading ? "Loading…" : "Load more"}</button>` : ""}</div><p class="retention-note">This is Market-owned deal history. The Activity page remains a separate, session-only view.</p></section>`;
}

function renderLocalDataWorkspace() {
  const pipeline = state.dataSyntheticRelease;
  const pipelineView = pipeline
    ? `<section class="data-pipeline panel"><div class="panel-heading"><div><p class="eyebrow">REPRODUCIBLE DERIVATION · SYNTHETIC INPUT</p><h2>Bronze to Silver to Gold</h2></div><span class="tag tag-caution">No live market claim</span></div><div class="data-pipeline-line"><article><span class="data-layer-label">BRONZE · VERIFIED INPUT</span><strong>${escapeHtml(pipeline.bronze.row_count)} source rows</strong><p>${escapeHtml(pipeline.bronze.source_label)}</p><code title="${escapeHtml(pipeline.bronze.snapshot_digest)}">${escapeHtml(shortDigest(pipeline.bronze.snapshot_digest))}</code></article><article><span class="data-layer-label">SILVER · NORMALIZED</span><strong>${escapeHtml(pipeline.silver.length)} observations</strong><p>USD micros per hour · unknown availability stays unknown</p><span>${escapeHtml(pipeline.method_version)}</span></article><article><span class="data-layer-label">GOLD · GROUP SUMMARY</span><strong>${escapeHtml(pipeline.gold.length)} model/region groups</strong><p>Asking-price range and availability coverage</p><code title="${escapeHtml(pipeline.output_digest)}">${escapeHtml(shortDigest(pipeline.output_digest))}</code></article></div><div class="data-table-scroll" role="region" aria-label="Derived market summary table" tabindex="0"><table class="data-result-table data-pipeline-table"><caption class="visually-hidden">Synthetic observations grouped by GPU model and region.</caption><thead><tr><th scope="col">GPU / region</th><th scope="col">Records</th><th scope="col">Asking range · USD/hour</th><th scope="col">Availability coverage</th><th scope="col">Known units</th></tr></thead><tbody>${pipeline.gold.map((group) => `<tr><td>${escapeHtml(group.gpu_model)} · ${escapeHtml(group.region)}</td><td>${escapeHtml(group.record_count)}</td><td>${money(group.min_asking_usd_micros_per_hour)} – ${money(group.max_asking_usd_micros_per_hour)}</td><td>${escapeHtml(group.known_availability_records)} known · ${escapeHtml(group.unknown_availability_records)} unknown</td><td>${group.available_units_total == null ? `<span class="data-null">Unknown</span>` : escapeHtml(group.available_units_total)}</td></tr>`).join("")}</tbody></table></div><div class="data-pipeline-foot"><span>Input SHA-256 <code>${escapeHtml(shortDigest(pipeline.bronze.snapshot_digest))}</code></span><span>Output SHA-256 <code>${escapeHtml(shortDigest(pipeline.output_digest))}</code></span><span>Freshness ${escapeHtml(pipeline.bronze.freshness)}</span></div><p class="data-claim">${escapeHtml(pipeline.claim_ceiling)}. ${escapeHtml(pipeline.effect_boundary.replaceAll("_", " ").toLowerCase())}.</p></section>`
    : `<section class="data-pipeline panel"><div class="panel-heading"><div><p class="eyebrow">REPRODUCIBLE DERIVATION</p><h2>Bronze to Silver to Gold</h2></div><span class="tag tag-neutral">Waiting for local input</span></div><p>Create the fixed HyperAI synthetic snapshot to inspect the deterministic, source-bound transform. GPUDiff public snapshots are cataloged separately and do not enter this synthetic derivation.</p></section>`;
  const catalog = state.dataCatalog;
  if (!catalog) return `<section class="empty-panel compact-empty"><div class="empty-mark">⌁</div><h2>Connecting to the local data service</h2><p>The catalog is read from the HyperAI home through the authenticated loopback API.</p></section>`;
  const datasets = Array.isArray(catalog.datasets) ? catalog.datasets : [];
  const selected = datasets.find((dataset) => dataset.dataset_id === state.dataDatasetId) || datasets[0];
  const datasetPicker = datasets.length
    ? `<label class="data-field"><span>Local snapshot</span><select id="data-dataset-id" name="dataset_id" form="data-query-form">${datasets.map((dataset) => `<option value="${escapeHtml(dataset.dataset_id)}" ${dataset.dataset_id === selected?.dataset_id ? "selected" : ""}>${escapeHtml(dataset.format.toUpperCase())} · ${escapeHtml(dataset.dataset_id)}</option>`).join("")}</select></label>`
    : `<div class="data-empty"><strong>Your local catalog is empty.</strong><span>Create the fixed HyperAI JSON and Parquet sample. It stays local and becomes a verified source reference in the workload trace.</span><button class="button button-secondary" type="button" data-action="seed-synthetic-data" ${state.dataBusy ? "disabled" : ""}>${state.dataBusy ? "Creating local snapshot…" : "Create local synthetic snapshot"}</button></div>`;
  const result = state.dataResult
    ? `<section class="data-result panel"><div class="panel-heading"><div><p class="eyebrow">LOCAL QUERY RESULT</p><h2>${escapeHtml(state.dataResult.row_count)} rows · ${escapeHtml(state.dataResult.columns.length)} columns</h2></div><span class="tag tag-caution">Snapshot-bound</span></div><div class="data-result-meta"><span>Dataset <code>${escapeHtml(state.dataResult.dataset_id)}</code></span><span>SHA-256 <code title="${escapeHtml(state.dataResult.snapshot_digest)}">${escapeHtml(shortDigest(state.dataResult.snapshot_digest))}</code></span><span>Output ${escapeHtml(state.dataResult.output_bytes)} bytes</span></div>${renderDataResultTable(state.dataResult)}</section>`
    : `<div class="data-result-empty">Run a query to see a bounded preview. Results are held in this browser session only.</div>`;
  const models = state.dataModels?.models || [];
  const views = state.dataViews?.views || [];
  const modelList = models.length
    ? `<div class="data-definition-list">${models.map((record) => `<article class="data-definition-row"><div><strong>${escapeHtml(record.model.model_id)}</strong><span>${escapeHtml(record.model.dataset_id)} · ${escapeHtml(record.input_state)} input</span><code>${escapeHtml(shortDigest(record.definition_digest))}</code></div><button class="button button-quiet" type="button" data-action="run-saved-model" data-model-id="${escapeHtml(record.model.model_id)}" aria-label="Run saved model ${escapeHtml(record.model.model_id)} on dataset ${escapeHtml(record.model.dataset_id)}" ${record.input_state !== "current" || state.dataBusy ? "disabled" : ""}>Run model</button></article>`).join("")}</div>`
    : `<p class="data-muted">No saved query models in this HyperAI home.</p>`;
  const viewList = views.length
    ? `<div class="data-definition-list">${views.map((record) => `<article class="data-definition-row"><div><strong>${escapeHtml(record.view.view_id)}</strong><span>${escapeHtml(record.view.presentation)} · ${escapeHtml(record.model_state)} model</span><code>${escapeHtml(shortDigest(record.definition_digest))}</code></div><span class="tag ${record.model_state === "linked" ? "tag-good" : "tag-caution"}">${escapeHtml(record.model_state)}</span></article>`).join("")}</div>`
    : `<p class="data-muted">Views store presentation metadata and a model digest; they contain no rows.</p>`;
  const importedReceipt = selected?.dataset_id
    && state.dataImportReceipt?.manifest?.dataset_id
    && selected.dataset_id === state.dataImportReceipt.manifest.dataset_id
    ? `<p class="data-claim">GPUDiff source SHA-256 <code>${escapeHtml(state.dataImportReceipt.receipt.raw_source_digest)}</code> · observed ${escapeHtml(state.dataImportReceipt.receipt.observed_at_min)} to ${escapeHtml(state.dataImportReceipt.receipt.observed_at_max)} · CC BY 4.0; cite gpudiff.com.</p>`
    : "";
  const schedule = state.gpudiffSchedule;
  const scheduleActive = ["starting", "refreshing", "active"].includes(schedule?.state);
  const scheduleState = schedule?.state?.replaceAll("_", " ") || "status not loaded";
  const scheduleSuccess = schedule?.last_success_unix_seconds
    ? new Date(schedule.last_success_unix_seconds * 1000).toLocaleString()
    : "No successful scheduled refresh yet";
  const scheduleDigest = schedule?.last_raw_source_digest
    ? `<code>${escapeHtml(shortDigest(schedule.last_raw_source_digest))}</code>`
    : "Not available";
  const scheduleObservedRange = schedule?.last_observed_at_min && schedule?.last_observed_at_max
    ? `${escapeHtml(schedule.last_observed_at_min)} to ${escapeHtml(schedule.last_observed_at_max)}`
    : "No source observation range yet";
  const scheduleError = schedule?.last_error_code
    ? `<p class="data-schedule-error" role="status">Last attempt: ${escapeHtml(schedule.last_error_code)}. The current snapshot remains available.</p>`
    : "";
  const schedulePanel = `<section class="data-schedule panel" aria-labelledby="gpudiff-schedule-title"><div class="panel-heading"><div><p class="eyebrow">OWNER-CONTROLLED · PUBLIC DATA</p><h2 id="gpudiff-schedule-title">Scheduled GPUDiff refresh</h2></div><span class="tag ${scheduleActive ? "tag-good" : schedule?.state === "paused_capacity" ? "tag-caution" : "tag-neutral"}">${escapeHtml(scheduleState)}</span></div><p>Start to fetch now, then refresh every six hours while this local service stays open. It keeps at most 16 distinct captures and stops at capacity without deleting older data.</p><div class="data-schedule-meta"><span>Last successful refresh: ${escapeHtml(scheduleSuccess)}</span><span>Source digest: ${scheduleDigest}</span><span>Source observations: ${scheduleObservedRange}</span><span>Attribution: CC BY 4.0 · cite gpudiff.com</span></div>${scheduleError}<div class="data-schedule-actions"><button class="button button-secondary" type="button" data-action="start-gpudiff-schedule" ${state.gpudiffScheduleBusy || scheduleActive || schedule?.state === "paused_capacity" ? "disabled" : ""}>${state.gpudiffScheduleBusy && !scheduleActive ? "Starting schedule…" : "Start six-hour refresh"}</button><button class="button button-secondary" type="button" data-action="stop-gpudiff-schedule" ${state.gpudiffScheduleBusy || !scheduleActive ? "disabled" : ""}>Stop schedule</button><button class="button button-secondary" type="button" data-action="refresh-gpudiff-schedule-status" ${state.gpudiffScheduleBusy ? "disabled" : ""}>Check status</button></div></section>`;
  return `<section class="data-workspace">
    ${pipelineView}
    <section class="data-hero panel"><div><p class="eyebrow">HYPERAI-OWNED · LOCAL CATALOG</p><h2>Inspect a snapshot. Keep its provenance attached.</h2><p>Query synthetic fixtures or import a bounded, source-attributed GPUDiff public offer snapshot. The imported feed remains advisory and separate from provider execution and Market records.</p><button class="button button-secondary" type="button" data-action="refresh-gpudiff-data" ${state.dataBusy ? "disabled" : ""}>${state.dataBusy ? "Fetching and checking GPUDiff…" : "Import current GPUDiff snapshot"}</button></div><div class="data-hero-stat"><span>${escapeHtml(datasets.length)}</span><small>${datasets.length === 1 ? "verified snapshot" : "verified snapshots"}</small><em>${escapeHtml(catalog.status)}</em></div></section>
    ${schedulePanel}
    <section class="data-query-panel panel"><div class="panel-heading"><div><p class="eyebrow">READ-ONLY SQL · 2 SECOND LIMIT</p><h2>Query a local dataset</h2></div><span class="tag tag-neutral">No external scan paths</span></div>
      ${datasetPicker}
      ${selected ? `<dl class="data-provenance"><div><dt>Source</dt><dd>${escapeHtml(selected.source_label)}</dd></div><div><dt>Freshness</dt><dd>${escapeHtml(selected.freshness)}</dd></div><div><dt>Rows</dt><dd>${escapeHtml(selected.row_count)}</dd></div><div><dt>Snapshot</dt><dd><code title="${escapeHtml(selected.snapshot_digest)}">${escapeHtml(shortDigest(selected.snapshot_digest))}</code></dd></div></dl><p class="data-claim">${escapeHtml(selected.claim_ceiling)}</p>${importedReceipt}` : ""}
      ${state.dataQueryStatus ? `<p id="data-query-status" class="data-query-status${state.dataQueryError ? " is-error" : ""}" role="status" aria-live="polite" tabindex="-1">${escapeHtml(state.dataQueryStatus)}</p>` : ""}
      <form id="data-query-form" class="data-query-form"><label class="data-field" for="data-sql"><span>SQL</span><textarea id="data-sql" name="sql" maxlength="4096" spellcheck="false" required>${escapeHtml(state.dataSql)}</textarea></label><div class="data-query-actions"><button class="button button-primary" type="submit" ${!selected || state.dataBusy ? "disabled" : ""}>${state.dataBusy && !state.activeDataQueryId ? "Saving locally…" : state.activeDataQueryId ? "Query running…" : "Run bounded query"}</button>${state.activeDataQueryId ? `<button class="button button-secondary" type="button" data-action="cancel-data-query">Cancel query</button>` : `<span>64 MiB memory · one partition · max 1,000 rows · max 256 KiB output</span>`}</div></form>
    </section>
    ${result}
    ${state.dataResult ? `<section class="data-save-grid">
      <section class="panel data-save-panel">
        <div class="panel-heading"><div><p class="eyebrow">REUSABLE QUERY</p><h2>Save as a model</h2></div><span class="tag tag-neutral">Definition only</span></div>
        <p>Model saves bind this SQL to the selected snapshot digest. They store no input or result rows.</p>
        ${state.dataModelStatus ? `<p id="data-model-status" class="data-action-status${state.dataModelStatusError ? " is-error" : ""}" role="status" aria-live="polite" aria-atomic="true" tabindex="-1">${escapeHtml(state.dataModelStatus)}</p>` : ""}
        ${state.dataModelPreview ? `<div class="data-preview-digest"><span>Preview digest</span><code>${escapeHtml(state.dataModelPreview.preview_digest)}</code><button class="button button-primary" type="button" data-action="confirm-data-model" ${state.dataBusy ? "disabled" : ""}>${state.dataBusy ? "Saving model…" : "Confirm local save"}</button></div>` : `<form id="data-model-form" class="data-definition-form"><label class="data-field"><span>Model name</span><input name="model_id" value="${escapeHtml(state.dataModelDraft?.model_id || "")}" maxlength="64" pattern="[A-Za-z0-9][A-Za-z0-9_\\x2d]*" placeholder="hourly-costs" required /></label><button class="button button-secondary" type="submit" ${state.dataBusy ? "disabled" : ""}>${state.dataBusy && state.dataModelStatus ? "Preparing preview…" : "Preview model"}</button></form>`}
      </section>
      <section class="panel data-save-panel">
        <div class="panel-heading"><div><p class="eyebrow">PRESENTATION</p><h2>Save a view</h2></div><span class="tag tag-neutral">No row storage</span></div>
        ${state.dataModelSaved ? `<p>Linked to <strong>${escapeHtml(state.dataModelSaved.definition_id)}</strong> · <code>${escapeHtml(shortDigest(state.dataModelSaved.definition_digest))}</code></p>` : `<p class="data-muted">Save a model first; views bind to its definition digest.</p>`}
        ${state.dataModelSaved && state.dataViewStatus ? `<p id="data-view-status" class="data-action-status${state.dataViewStatusError ? " is-error" : ""}" role="status" aria-live="polite" aria-atomic="true" tabindex="-1">${escapeHtml(state.dataViewStatus)}</p>` : ""}
        ${state.dataModelSaved ? state.dataViewPreview ? `<div class="data-preview-digest"><span>Preview digest</span><code>${escapeHtml(state.dataViewPreview.preview_digest)}</code><button class="button button-primary" type="button" data-action="confirm-data-view" ${state.dataBusy ? "disabled" : ""}>${state.dataBusy ? "Saving view…" : "Confirm view save"}</button></div>` : `<form id="data-view-form" class="data-definition-form"><label class="data-field"><span>View name</span><input name="view_id" value="${escapeHtml(state.dataViewDraft?.view_id || "")}" maxlength="64" pattern="[A-Za-z0-9][A-Za-z0-9_\\x2d]*" placeholder="cost-chart" required /></label><label class="data-field"><span>Presentation</span><select name="presentation"><option value="table" ${state.dataViewDraft?.presentation === "table" ? "selected" : ""}>Table</option><option value="bar" ${state.dataViewDraft?.presentation === "bar" ? "selected" : ""}>Bar</option><option value="line" ${state.dataViewDraft?.presentation === "line" ? "selected" : ""}>Line</option></select></label><label class="data-field"><span>Columns</span><input name="columns" value="${escapeHtml((state.dataViewDraft?.columns || state.dataResult.columns || []).join(", "))}" required /></label><button class="button button-secondary" type="submit" ${state.dataBusy ? "disabled" : ""}>${state.dataBusy && state.dataViewStatus ? "Preparing preview…" : "Preview view"}</button></form>` : ""}
      </section>
    </section>` : ""}
    <section class="panel data-definitions"><div class="panel-heading"><div><p class="eyebrow">LOCAL DEFINITIONS</p><h2>Models &amp; views</h2></div><span class="tag tag-neutral">Durable in this HyperAI home</span></div><div class="data-definition-columns"><div><h3>Query models</h3>${modelList}</div><div><h3>Views</h3>${viewList}</div></div></section>
    ${state.dataModelSaved ? `<div class="notice notice-good"><span class="notice-icon">✓</span><div><strong>Saved model ${escapeHtml(state.dataModelSaved.definition_id)}</strong><p>Definition digest ${escapeHtml(state.dataModelSaved.definition_digest)} · local user-confirmed write.</p></div></div>` : ""}
    ${state.dataViewSaved ? `<div class="notice notice-good"><span class="notice-icon">✓</span><div><strong>Saved view ${escapeHtml(state.dataViewSaved.definition_id)}</strong><p>Definition digest ${escapeHtml(state.dataViewSaved.definition_digest)} · metadata only.</p></div></div>` : ""}
  </section>`;
}

function renderDataResultTable(result) {
  const columns = Array.isArray(result.columns) ? result.columns : [];
  const rows = Array.isArray(result.rows) ? result.rows : [];
  return `<div class="data-table-scroll" role="region" aria-label="Query results table" tabindex="0"><table class="data-result-table"><caption class="visually-hidden">Query results for dataset ${escapeHtml(result.dataset_id)} from snapshot ${escapeHtml(shortDigest(result.snapshot_digest))}.</caption><thead><tr>${columns.map((column) => `<th scope="col">${escapeHtml(column)}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${row.map((cell) => `<td>${cell === null ? `<span class="data-null">Unknown</span>` : escapeHtml(cell)}</td>`).join("")}</tr>`).join("") || `<tr><td colspan="${Math.max(columns.length, 1)}">No rows returned.</td></tr>`}</tbody></table></div>`;
}

function renderData() {
  const observations = demoReceipt()?.body.advisory_observations || [];
  const capture = state.simpleMarketCapture;
  const captureView = capture
    ? renderScmListingExplorer(capture)
    : `<section class="empty-panel compact-empty"><div class="empty-mark">⌕</div><h2>No signed registry capture in this session</h2><p>Import a caller-provided Simple Compute Market capture in Market, verify it under caller pins, then explore its source fields here.</p><button class="button button-secondary" data-page="market" type="button">Open Market import</button></section>`;
  const observationView = observations.length
    ? `<section class="panel data-observations"><div class="panel-heading"><div><p class="eyebrow">HYPERAI SYNTHETIC FIXTURE</p><h2>Advisory observations</h2></div><span class="tag tag-caution">Not Bazaar data</span></div><div class="signal-grid">${observations.map((item) => `<article class="signal-card"><div><span class="signal-label">MARKET OBSERVATION</span><span class="tag tag-caution">Advisory</span></div><strong>${item.body.estimated_rate_microusd_per_unit_hour == null ? "Unknown" : money(item.body.estimated_rate_microusd_per_unit_hour)}</strong><p>${escapeHtml(item.body.offer_id)} · estimated availability: ${item.body.estimated_available_units == null ? "Unknown" : `${escapeHtml(item.body.estimated_available_units)} units`}</p><small>${escapeHtml(item.body.source_id)} · observed ${escapeHtml(item.body.observed_at)}</small><code>${escapeHtml(shortDigest(item.digest))}</code></article>`).join("")}</div><p class="retention-note">This local fixture is separate from the caller-authenticated registry capture above; no cross-source join is performed.</p></section>`
    : `<section class="empty-panel compact-empty"><div class="empty-mark">▥</div><h2>No synthetic observations loaded</h2><p>Run the local market fixture to inspect advisory-only observations. They are generated by HyperAI and are not Bazaar data.</p>${button("Run full synthetic lifecycle", "run-demo")}</section>`;
  return `${heading("ANALYSIS / SOURCE-LABELLED DATA", "Data", "Compare provenance, inspect bounded results, and preserve each source boundary through the HyperAI interface.")}
    ${renderLocalDataWorkspace()}
    ${captureView}
    ${observationView}
    <div class="notice notice-neutral"><span class="notice-icon">↗</span><div><strong>Analysis is not authority</strong><p>Source fields and modeled price or availability do not determine offer eligibility, seller quote terms, verifier decisions, or settlement. Listing freshness remains unknown unless a source-specific policy evaluates it.</p></div></div>`;
}

function renderInference() {
  const result = state.hyperintelVerification;
  const resultPanel = result ? `<section class="panel hyperintel-result"><div class="panel-heading"><div><p class="eyebrow">SOURCE-OWNED VERIFIER</p><h2>Contract verification passed</h2></div><span class="tag tag-caution">Caller trust · local only</span></div><p class="result-explanation">HyperIntel's V1–V4 verifier accepted this bundle using the public keys you supplied. This does not establish current provider state, execution, output correctness, payment, or independent review.</p><div class="verification-metrics"><div><span>Bundle schema</span><strong>${escapeHtml(result.bundle_schema_version)}</strong></div><div><span>Candidate offers</span><strong>${escapeHtml(result.candidate_count)}</strong></div><div><span>Caller trust keys</span><strong>${escapeHtml(result.trusted_key_count)}</strong></div></div><dl class="detail-list"><div><dt>Source slice</dt><dd><code>${escapeHtml(result.state_slice)}</code></dd></div><div><dt>Evaluation time</dt><dd>${escapeHtml(result.evaluated_at)} · no freshness judgment</dd></div><div><dt>Bundle digest</dt><dd><code class="digest-full">${escapeHtml(result.bundle_digest)}</code></dd></div><div><dt>Assurance blobs</dt><dd>${escapeHtml(result.assurance_blob_count)}</dd></div><div><dt>Reliability artifacts</dt><dd>${escapeHtml(result.reliability_artifact_count)}</dd></div></dl><div class="claim-box"><span class="eyebrow">CLAIM CEILING</span><p>${escapeHtml(result.claim_ceiling)}</p><span class="eyebrow">EXPLICIT NONCLAIMS</span><ul>${result.explicit_nonclaims.map((claim) => `<li>${escapeHtml(claim)}</li>`).join("")}</ul></div>${result.assumptions.length ? `<div class="assumption-box"><span class="eyebrow">BUNDLE ASSUMPTIONS</span><ul>${result.assumptions.map((assumption) => `<li>${escapeHtml(assumption)}</li>`).join("")}</ul></div>` : ""}<p class="retention-note">Inputs and verification results are not persisted by this interface.</p></section>` : "";
  const reconciliation = state.hyperintelReconciliation;
  const reconciliationPanel = reconciliation ? `<section class="panel hyperintel-result"><div class="panel-heading"><div><p class="eyebrow">SOURCE-OWNED RECONCILER</p><h2>Execution lineage reconstructed</h2></div><span class="tag tag-caution">Caller trust · local only</span></div><p class="result-explanation">This report confirms that the supplied admission, route, replay consumption, journal, execution outcome, provider receipt, metering, and settlement-intent records reconstruct consistently. It does not independently meter work or prove correct computation.</p><div class="verification-metrics"><div><span>Reconciled status</span><strong>${escapeHtml(reconciliation.report.status)}</strong></div><div><span>Caller trust keys</span><strong>${escapeHtml(reconciliation.trusted_key_count)}</strong></div><div><span>Signed offers</span><strong>${escapeHtml(reconciliation.signed_offer_count)}</strong></div><div><span>Assurance blobs</span><strong>${escapeHtml(reconciliation.assurance_blob_count)}</strong></div><div><span>Reliability artifacts</span><strong>${escapeHtml(reconciliation.reliability_artifact_count)}</strong></div></div><dl class="detail-list"><div><dt>Report schema</dt><dd><code>${escapeHtml(reconciliation.report.schema_version)}</code></dd></div><div><dt>Source slice</dt><dd><code>${escapeHtml(reconciliation.report.state_slice)}</code></dd></div><div><dt>Buyer</dt><dd><code>${escapeHtml(reconciliation.report.buyer_id)}</code></dd></div><div><dt>Provider</dt><dd><code>${escapeHtml(reconciliation.report.provider_id)}</code></dd></div><div><dt>Admission</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.admission_digest)}</code></dd></div><div><dt>Execution outcome</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.execution_outcome_digest)}</code></dd></div><div><dt>Stream commitment</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.stream_digest)}</code></dd></div><div><dt>Provider receipt</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.provider_receipt_digest || "Not present for this incomplete execution")}</code></dd></div><div><dt>Metering receipt</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.metering_receipt_digest || "Not present for this incomplete execution")}</code></dd></div><div><dt>Settlement intent</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.settlement_intent_digest || "Not present for this incomplete execution")}</code></dd></div><div><dt>Report digest</dt><dd><code class="digest-full">${escapeHtml(reconciliation.report.report_digest)}</code></dd></div><div><dt>Authority</dt><dd>${reconciliation.authority_granted ? "Granted" : "None"}</dd></div></dl><div class="claim-box"><span class="eyebrow">CLAIM CEILING</span><p>${escapeHtml(reconciliation.claim_ceiling)}</p></div><p class="retention-note">The selected execution stream is not shown or stored by this interface. Only the compact identity report remains in this page session.</p></section>` : "";
  return `${heading("HYPERINTEL / OFFLINE EVIDENCE", "Inference", "Verify portable market evidence and reconcile a complete local execution lineage without routing requests or contacting a provider.")}
    <div class="notice notice-caution"><span class="notice-icon">i</span><div><strong>Caller-controlled trust · local contract check</strong><p>Select the evidence input and its public-key registry. The verifier checks bundle versions 1–4; version-specific nonclaims remain visible below.</p></div></div>
    <section class="panel import-panel"><div class="panel-heading"><div><p class="eyebrow">LOCAL FILES ONLY</p><h2>Verify a market-evidence bundle</h2></div><span class="tag tag-neutral">No provider call</span></div><div class="import-grid"><label class="file-field" for="hyperintel-evidence-file"><span>HyperIntel evidence input</span><small>CLI-compatible JSON with bundle, assurance blobs, and V3/V4 reliability artifacts</small><input id="hyperintel-evidence-file" type="file" accept=".json,application/json" /></label><label class="file-field" for="hyperintel-trust-file"><span>Trusted-key registry</span><small>Caller-controlled public keys; the interface does not create or select trust roots</small><input id="hyperintel-trust-file" type="file" accept=".json,application/json" /></label></div><div class="import-actions"><button class="button button-primary" data-action="verify-hyperintel" type="button" ${state.busy ? "disabled aria-busy=\"true\"" : ""}>${state.busy ? "Verifying locally…" : "Verify bundle offline"}</button><span>Maximum combined file size: 150 KB · memory only</span></div></section>
    ${resultPanel || `<section class="empty-panel compact-empty"><div class="empty-mark">◉</div><h2>No HyperIntel bundle checked</h2><p>Choose the exact evidence and trust files used by HyperIntel's offline verifier. A report from another tool is not substituted for contract verification.</p></section>`}
    <section class="panel import-panel"><div class="panel-heading"><div><p class="eyebrow">LOCAL FILES ONLY · REPORT-ONLY</p><h2>Reconcile an execution lineage</h2></div><span class="tag tag-neutral">No provider or store access</span></div><p class="result-explanation">Select a complete <code>hyperintel.receipt-reconciliation-input.v1</code> JSON document with admission, routing context, replay transition, signed policy, journal start and terminal, and execution outcome; then select its caller-controlled public-key registry.</p><div class="notice notice-caution reconciliation-privacy"><span class="notice-icon">!</span><div><strong>Execution streams can contain private text</strong><p>Use synthetic or explicitly authorized non-sensitive evidence only. The interface sends the selected file to its loopback API, holds it in memory for this request, and renders only identity digests and counts. It does not fetch records from HyperIntel.</p></div></div><div class="import-grid"><label class="file-field" for="hyperintel-execution-file"><span>Complete execution lineage</span><small>Versioned JSON; may include the full execution stream</small><input id="hyperintel-execution-file" type="file" accept=".json,application/json" /></label><label class="file-field" for="hyperintel-reconciliation-trust-file"><span>Trusted-key registry</span><small>Caller-controlled public keys for buyer, provider, and evidence verification</small><input id="hyperintel-reconciliation-trust-file" type="file" accept=".json,application/json" /></label></div><div class="import-actions"><button class="button button-primary" data-action="reconcile-hyperintel" type="button" ${state.busy ? "disabled aria-busy=\"true\"" : ""}>${state.busy ? "Reconciling locally…" : "Reconcile lineage offline"}</button><span>Maximum combined file size: 150 KB · no source store or provider call</span></div></section>
    ${reconciliationPanel || `<section class="empty-panel compact-empty"><div class="empty-mark">⌁</div><h2>No execution lineage reconciled</h2><p>A positive source-owned report checks internal record consistency only. It grants no execution, settlement, or value authority.</p></section>`}
    <div class="notice notice-neutral"><span class="notice-icon">↗</span><div><strong>Inference execution remains owner-controlled</strong><p>The interface verifies portable evidence and reconciles caller-supplied records. It does not fetch HyperIntel records, route requests, invoke a provider, execute inference, resolve a secret, or submit settlement.</p></div></div>`;
}

function fleetHealthLabel(value) {
  return String(value || "unknown").replaceAll("_", " ");
}

function allowedFleetSimulationFaults(operation) {
  if (["reconcile", "cleanup"].includes(operation)) return ["none"];
  if (["launch", "terminate"].includes(operation)) return ["none", "timeout_before_effect", "timeout_after_effect", "partial_effect"];
  return ["none", "timeout_before_effect", "timeout_after_effect"];
}

function renderFleet() {
  const inventory = state.fleetInventory;
  if (!inventory) {
    return `${heading("FLEET / READ-ONLY SCENARIO", "Fleet", "Inspect owner-labelled inventory, review health states, and bind an exact advisory plan.")}
      <section class="empty-panel compact-empty"><div class="empty-mark">▦</div><h2>Loading local scenario</h2><p>The Fleet view uses a fixed HyperAI-authored fixture. It does not connect to a provider or inspect this machine.</p></section>`;
  }
  const healthClass = (value) => `fleet-health fleet-health-${escapeHtml(value)}`;
  const nodes = inventory.nodes || [];
  const timeline = inventory.health_timeline || [];
  const checks = state.fleetDoctor?.checks || inventory.doctor?.checks || [];
  const nodeRows = nodes.map((node) => `<tr>
    <td><strong>${escapeHtml(node.display_name)}</strong><small>${escapeHtml(node.node_id)}</small></td>
    <td>${escapeHtml(node.resource_profile)}</td>
    <td><span class="${healthClass(node.health_state)}">${escapeHtml(fleetHealthLabel(node.health_state))}</span><small>${escapeHtml(node.scenario_note)}</small></td>
    <td>${escapeHtml(node.owner_label)}</td>
  </tr>`).join("");
  const nodeCards = nodes.map((node) => `<article class="fleet-node-card">
    <div class="fleet-node-card-top"><div><strong>${escapeHtml(node.display_name)}</strong><small>${escapeHtml(node.node_id)}</small></div><span class="${healthClass(node.health_state)}">${escapeHtml(fleetHealthLabel(node.health_state))}</span></div>
    <dl><div><dt>Resource profile</dt><dd>${escapeHtml(node.resource_profile)}</dd></div><div><dt>Fixture owner</dt><dd>${escapeHtml(node.owner_label)}</dd></div></dl>
    <p>${escapeHtml(node.scenario_note)}</p>
  </article>`).join("");
  const timelineRows = timeline.map((event) => {
    const node = nodes.find((item) => item.node_id === event.node_id);
    return `<li><span class="fleet-step">${escapeHtml(event.scenario_step)}</span><div><div class="fleet-timeline-title"><strong>${escapeHtml(node?.display_name || event.node_id)}</strong><span class="${healthClass(event.health_state)}">${escapeHtml(fleetHealthLabel(event.health_state))}</span></div><p>${escapeHtml(event.summary)}</p><small>Scenario step ${escapeHtml(event.scenario_step)} · not a wall-clock observation</small></div></li>`;
  }).join("");
  const doctorRows = checks.map((check) => `<article class="fleet-doctor-check">
    <div><span class="fleet-check-status fleet-check-${escapeHtml(check.status)}">${escapeHtml(check.status)}</span><strong>${escapeHtml(check.check_id.replaceAll("_", " "))}</strong></div>
    <p>${escapeHtml(check.summary)}</p>
  </article>`).join("");
  const operationNames = {
    attach: "Attach",
    launch: "Launch",
    workload_start: "Start workload",
    cancel: "Cancel",
    terminate: "Terminate",
  };
  const plan = state.fleetPlan;
  const planPanel = plan
    ? `<section class="panel fleet-plan-result" aria-live="polite">
      <div class="panel-heading"><div><p class="eyebrow">DIGEST-BOUND PREVIEW</p><h3 id="fleet-plan-heading" tabindex="-1">${escapeHtml(operationNames[plan.operation] || plan.operation)} · ${escapeHtml(plan.node_id)}</h3></div><span class="fleet-check-status ${plan.preview_status === "preview_changed" ? "fleet-check-blocked" : "fleet-check-unknown"}">${escapeHtml(plan.preview_status.replaceAll("_", " "))}</span></div>
      <div class="fleet-plan-status"><span>Scenario health</span><strong class="${healthClass(plan.observed_health_state)}">${escapeHtml(fleetHealthLabel(plan.observed_health_state))}</strong><span>Authority</span><strong>${escapeHtml(plan.authority.replaceAll("_", " "))}</strong><span>Preview digest</span><code title="${escapeHtml(plan.preview_digest)}">${escapeHtml(plan.preview_digest)}</code></div>
      <p class="fleet-plan-boundary">${escapeHtml(plan.health_gate.replaceAll("_", " "))}. ${escapeHtml(plan.effect_boundary.replaceAll("_", " "))}.</p>
      <div class="fleet-gates"><strong>Required for any future live operation</strong><ul>${(plan.required_gates || []).map((gate) => `<li>${escapeHtml(gate.replaceAll("_", " "))}</li>`).join("")}</ul></div>
      <div class="fleet-claims"><strong>Claim ceiling</strong><p>${escapeHtml(plan.claim_ceiling)}</p><strong>Nonclaims</strong><p>${escapeHtml(plan.nonclaims)}</p></div>
    </section>`
    : `<div class="fleet-plan-empty">Choose a fixture node and intent to see a deterministic preview. The preview cannot invoke or reserve anything.</div>`;
  const simulation = state.fleetSimulation;
  const simulationOperations = {
    attach: "Attach fixture",
    launch: "Acquire virtual lease",
    workload_start: "Start virtual task",
    cancel: "Cancel virtual task",
    terminate: "Terminate virtual lease",
    reconcile: "Reconcile outcome",
    cleanup: "Clean up virtual state",
  };
  const allowedFaults = new Set(allowedFleetSimulationFaults(state.fleetSimulationOperation));
  const simulationTransition = state.fleetSimulationTransition;
  const simulationPanel = simulation
    ? `<section class="panel fleet-simulator">
      <div class="panel-heading"><div><p class="eyebrow">LOCAL TEST ADAPTER / VOLATILE STATE</p><h2>Fleet lifecycle simulator</h2></div><span class="tag tag-caution">Virtual only</span></div>
      <p class="fleet-simulator-intro">Exercise ownership, virtual leases, workload transitions, retries, ambiguous outcomes, reconciliation, and cleanup through the same service contract. State resets when this local service exits.</p>
      <div class="fleet-sim-metrics">
        <article><span>Scenario phase</span><strong>${escapeHtml(simulation.phase.replaceAll("_", " "))}</strong></article>
        <article><span>Virtual capacity</span><strong>${escapeHtml(simulation.allocated_units)} allocated · ${escapeHtml(simulation.quarantined_units)} quarantined / ${escapeHtml(simulation.virtual_capacity_units)}</strong></article>
        <article><span>Lease state</span><strong>${escapeHtml(simulation.lease_state.replaceAll("_", " "))}</strong><code>${escapeHtml(simulation.virtual_lease_id || "No virtual lease")}</code></article>
        <article><span>Virtual task</span><strong>${escapeHtml(simulation.virtual_task_id || "No virtual task")}</strong><small>Never an OS process ID</small></article>
      </div>
      <form class="fleet-sim-form" id="fleet-simulation-form">
        <label class="data-field"><span>Lifecycle transition</span><select name="operation">${Object.entries(simulationOperations).map(([value, label]) => `<option value="${escapeHtml(value)}" ${value === state.fleetSimulationOperation ? "selected" : ""}>${escapeHtml(label)}</option>`).join("")}</select></label>
        <label class="data-field"><span>Injected scenario</span><select name="fault">${Object.entries(fleetSimulationFaultLabels).map(([value, label]) => `<option value="${escapeHtml(value)}" ${value === state.fleetSimulationFault ? "selected" : ""} ${allowedFaults.has(value) ? "" : "disabled"}>${escapeHtml(label)}</option>`).join("")}</select></label>
        <label class="data-field"><span>Virtual units <small>(launch only)</small></span><input name="gpu_units" type="number" min="1" max="4" step="1" value="${escapeHtml(state.fleetSimulationGpuUnits)}" ${state.fleetSimulationOperation === "launch" ? "" : "disabled"} required /></label>
        <button class="button button-primary" type="submit" ${state.fleetSimulationBusy ? "disabled aria-busy=\"true\"" : ""}>${state.fleetSimulationBusy ? "Recording virtual step…" : state.fleetSimulationRetry ? "Retry exact transition" : "Apply simulator step"}</button>
      </form>
      <div class="fleet-sim-receipt" aria-live="polite"><span>${simulationTransition ? escapeHtml(simulationTransition.transition_status.replaceAll("_", " ")) : "No simulator transition recorded in this page"}</span><code>State ${escapeHtml(shortDigest(simulation.state_digest))} · ${escapeHtml(simulation.transition_count)} transitions</code><small>${escapeHtml(simulation.effect_boundary)} · ${escapeHtml(simulation.persistence)}</small></div>
      <div class="fleet-sim-recovery"><strong>Recovery rules</strong><span>Uncertain outcomes require reconcile or cleanup. Partial lease effects remain held until cleanup. Every value here is synthetic; no process, command, network, provider, or real capacity is involved.</span></div>
      <div class="fleet-claims"><strong>Claim ceiling</strong><p>${escapeHtml(simulation.claim_ceiling)}</p><strong>Nonclaims</strong><p>${escapeHtml(simulation.nonclaims)}</p></div>
    </section>`
    : `<section class="panel fleet-simulator"><div class="panel-heading"><div><p class="eyebrow">LOCAL TEST ADAPTER</p><h2>Fleet lifecycle simulator</h2></div></div><p class="fleet-simulator-intro">The authenticated local service has not returned a simulator snapshot. No action was taken.</p></section>`;
  return `${heading("FLEET / INVENTORY + LOCAL SIMULATOR", "Fleet", "Inspect the fixed scenario, build advisory previews, and exercise virtual lifecycle recovery through the shared local service.")}
    <div class="fleet-workspace">
      <section class="panel fleet-hero">
        <div><p class="eyebrow">HYPERAI-AUTHORED FIXTURE</p><h2>Fleet signals, with their limits attached.</h2><p>This is a fixed teaching scenario. Node names and health states are fictional; no machine or provider was checked.</p></div>
        <div class="fleet-snapshot"><span>SCENARIO SNAPSHOT</span><code title="${escapeHtml(inventory.snapshot_digest)}">${escapeHtml(shortDigest(inventory.snapshot_digest))}</code><small>${escapeHtml(inventory.provenance.scenario_id)}</small></div>
      </section>
      <div class="fleet-metrics"><article><strong>${escapeHtml(nodes.length)}</strong><span>fictional nodes</span></article><article><strong>${escapeHtml(timeline.length)}</strong><span>scenario samples</span></article><article><strong>Not run</strong><span>live health probe</span></article><article><strong>None</strong><span>operation authority</span></article></div>
      <section class="panel fleet-inventory">
        <div class="panel-heading"><div><p class="eyebrow">INVENTORY / LOCAL FIXTURE</p><h2>Illustrative resources</h2></div><span class="tag tag-caution">No capacity claim</span></div>
        <div class="fleet-node-cards">${nodeCards}</div>
        <div class="table-scroll"><table><thead><tr><th>Fixture node</th><th>Resource profile</th><th>Scenario health</th><th>Fixture owner</th></tr></thead><tbody>${nodeRows}</tbody></table></div>
        <div class="fleet-table-foot"><span>Source: ${escapeHtml(inventory.provenance.source_kind)}</span><span>Observation mode: ${escapeHtml(inventory.provenance.observation_mode)}</span></div>
      </section>
      <div class="fleet-lower-grid">
        <section class="panel fleet-timeline"><div class="panel-heading"><div><p class="eyebrow">HEALTH / SCENARIO STEPS</p><h2>Sample timeline</h2></div><span class="tag tag-neutral">Not live telemetry</span></div><ol>${timelineRows}</ol></section>
        <section class="panel fleet-doctor"><div class="panel-heading"><div><p class="eyebrow">DOCTOR / FIXTURE CHECKS</p><h2>Preflight checks</h2></div><span class="tag tag-caution">No host probe</span></div><div class="fleet-doctor-list">${doctorRows}</div></section>
      </div>
      <section class="panel fleet-plan-builder">
        <div class="panel-heading"><div><p class="eyebrow">PLAN / EXACT INTENT</p><h2>Preview a future operation</h2></div><span class="tag tag-neutral">Preview only</span></div>
        <p>Choose a fictional target and operation. The local service binds the exact intent to a stable digest. It does not emit a command, reserve capacity, or authorize execution.</p>
        <form class="fleet-plan-form" id="fleet-plan-form">
          <label class="data-field"><span>Fixture node</span><select name="node_id">${nodes.map((node) => `<option value="${escapeHtml(node.node_id)}" ${node.node_id === state.fleetNodeId ? "selected" : ""}>${escapeHtml(node.display_name)} · ${escapeHtml(fleetHealthLabel(node.health_state))}</option>`).join("")}</select></label>
          <label class="data-field"><span>Intent</span><select name="operation">${Object.entries(operationNames).map(([value, label]) => `<option value="${escapeHtml(value)}" ${value === state.fleetOperation ? "selected" : ""}>${escapeHtml(label)}</option>`).join("")}</select></label>
          <button class="button button-primary" type="submit" ${state.fleetBusy ? "disabled" : ""}>${state.fleetBusy ? "Building preview…" : "Build advisory preview"}</button>
        </form>
        ${planPanel}
      </section>
      ${simulationPanel}
      <div class="notice notice-caution"><span class="notice-icon">!</span><div><strong>${escapeHtml(inventory.claim_ceiling)}</strong><p>${escapeHtml(inventory.nonclaims)} A future live adapter, credentials, operator approval, and any operation each need separate authorization.</p></div></div>
    </div>`;
}

function renderEvidence() {
  const receipt = demoReceipt();
  if (!receipt) return `${heading("TRUST / RECEIPTS", "Evidence", "Inspect contract records and their limits. This workspace does not publish evidence to another system.")}${emptyState("No receipt is available", "Run the full synthetic lifecycle to inspect and locally recheck its digest-bound receipt.", "run-demo")}`;
  const check = state.verification;
  return `${heading("TRUST / LOCAL CONTRACT CHECK", "Receipt review", "A passing local check establishes contract integrity and signature consistency only.", button(state.busy ? "Checking…" : "Verify receipt locally", "verify-receipt", "button button-secondary"))}
    ${check ? `<div class="notice notice-good"><span class="notice-icon">✓</span><div><strong>Local contract integrity pass</strong><p>No independent verifier or external authority was used.</p></div></div>` : ""}
    <div class="receipt-card"><div class="receipt-top"><div><p class="eyebrow">HYPERAI COMPUTE-MARKET RECEIPT</p><h2>Local synthetic record</h2></div><span class="tag tag-caution">NO FUNDS MOVE</span></div><dl class="detail-list"><div><dt>Receipt digest</dt><dd><code class="digest-full">${escapeHtml(receipt.digest)}</code></dd></div><div><dt>Catalog digest</dt><dd><code>${escapeHtml(receipt.body.catalog.digest)}</code></dd></div><div><dt>Approved deal</dt><dd><code>${escapeHtml(state.demo.report.deal_digest)}</code></dd></div><div><dt>Seller receipt</dt><dd><code>${escapeHtml(state.demo.report.fulfillment_digest)}</code></dd></div><div><dt>Schema</dt><dd><code>${escapeHtml(receipt.body.schema_version)}</code></dd></div><div><dt>Producing slice</dt><dd><code>${escapeHtml(receipt.body.state_slice)}</code></dd></div></dl><div class="claim-box"><span class="eyebrow">CLAIM CEILING</span><p>${escapeHtml(receipt.body.claim_ceiling)}</p><span class="eyebrow">NONCLAIMS</span><p>${escapeHtml(receipt.body.nonclaims)}</p></div></div>
    <div class="notice notice-neutral"><span class="notice-icon">i</span><div><strong>Receipt provenance</strong><p>Seller signatures attribute statements to synthetic keys. They do not establish seller identity, hardware, correct computation, or payment entitlement.</p></div></div>`;
}

function renderEval() {
  const catalog = state.evalCatalog;
  const report = state.evalReport;
  const cases = catalog?.cases || [];
  const caseList = cases.length
    ? `<ol class="eval-case-list">${cases.map((item) => `<li><span class="eval-case-index">${escapeHtml(item.category.replaceAll("_", " "))}</span><strong>${escapeHtml(item.case_id.replaceAll("_", " "))}</strong><p>${escapeHtml(item.task)}</p></li>`).join("")}</ol>`
    : `<div class="empty-inline">The fixed synthetic catalog is unavailable. No evaluation ran.</div>`;
  const reportPanel = report
    ? `<section class="panel eval-report"><div class="panel-heading"><div><p class="eyebrow">LOCAL / SYNTHETIC REPORT</p><h2 id="eval-report-heading" tabindex="-1">Harness fixture comparison</h2></div><span class="tag tag-caution">No model run</span></div>
      <p class="result-explanation">These are two HyperAI-authored response fixtures scored against the same six local cases. They are not agents or model candidates and do not support a quality or improvement claim.</p>
      <div class="eval-report-grid"><article><span>Control fixture</span><strong>${escapeHtml(report.control.matching_cases)}/${escapeHtml(report.control.case_count)}</strong></article><article><span>Alternate fixture</span><strong>${escapeHtml(report.alternate.matching_cases)}/${escapeHtml(report.alternate.case_count)}</strong></article><article><span>Sequential repeats</span><strong>${escapeHtml(report.child_wall_time_ms.length)}</strong></article><article><span>Input / output bytes per scorer</span><strong>${escapeHtml(report.scorer_input_bytes)} / ${escapeHtml(report.scorer_output_bytes)}</strong></article></div>
      <dl class="detail-list"><div><dt>Suite digest</dt><dd><code class="digest-full">${escapeHtml(report.suite_digest)}</code></dd></div><div><dt>Repeated result digest</dt><dd><code class="digest-full">${escapeHtml(report.repeated_result_digest)}</code></dd></div><div><dt>Scorer child wall time</dt><dd>${report.child_wall_time_ms.map((value) => `${escapeHtml(value)} ms`).join(" · ")}</dd></div><div><dt>Measurement</dt><dd>${escapeHtml(report.measurement_type)}</dd></div></dl>
      <div class="eval-case-results"><h3>Per-case fixture matches</h3><ul>${report.cases.map((item) => `<li><strong>${escapeHtml(item.case_id.replaceAll("_", " "))}</strong><span>Control: ${item.control_matches ? "matches" : "differs"} · alternate: ${item.alternate_matches ? "matches" : "differs"}</span></li>`).join("")}</ul></div>
      <div class="claim-box"><span class="eyebrow">CLAIM CEILING</span><p>${escapeHtml(report.claim_ceiling)}</p><span class="eyebrow">NONCLAIMS</span><p>${escapeHtml(report.nonclaims)}</p></div><p class="retention-note">No model, agent, provider, task upload, or holdout was used. The report remains in this local service process and resets when it exits.</p></section>`
    : "";
  return `${heading("EVAL / HARNESS QUALIFICATION", "Eval", "Inspect the fixed synthetic task suite and verify that the local scorer returns stable paired fixture results.")}
    <div class="notice notice-caution"><span class="notice-icon">i</span><div><strong>Harness check only</strong><p>This workspace does not run an agent or model. The paired records are fixed answer fixtures; their scores are not candidate-quality or improvement evidence.</p></div></div>
    <section class="panel eval-catalog"><div class="panel-heading"><div><p class="eyebrow">HYPERAI-AUTHORED SUITE</p><h2>Six bounded synthetic cases</h2></div><span class="tag tag-neutral">No external data</span></div>
      <div class="eval-suite-meta"><div><span>Suite</span><strong>${escapeHtml(catalog?.suite_id || "Unavailable")}</strong></div><div><span>Fixed pair</span><strong>${escapeHtml(catalog?.pair_id || "Unavailable")}</strong></div><div><span>Repeat policy</span><strong>${escapeHtml(catalog?.repeat_count ?? "—")} sequential runs</strong></div></div>
      ${catalog ? `<div class="digest-line"><span>SUITE DIGEST</span><code class="digest-full">${escapeHtml(catalog.suite_digest)}</code></div>` : ""}${caseList}
      <div class="eval-run-controls"><p>Each run starts three bounded scorer subprocesses with the same fixture pair and suite digest. Only aggregate matches and process-time measurements are returned.</p><button class="button" type="button" data-action="run-eval" ${!catalog || state.evalBusy ? "disabled" : ""}>${state.evalBusy ? "Running local scorer…" : report ? "Run a new synthetic repeat" : "Run harness qualification"}</button></div>
    </section>${reportPanel}`;
}

function renderTrade() {
  const catalog = state.tradeCatalog;
  if (!catalog) {
    return `${heading("TRADE / LOCAL SYNTHETIC RESEARCH", "Trade", "Explore bounded price and capacity assumptions over a fixed HyperAI-authored fixture.")}
      <div class="empty-inline">The Trade research catalog is unavailable. No scenario or replay ran.</div>`;
  }
  const instruments = catalog.instruments || [];
  const optionList = (selected) => instruments.map((item) => `<option value="${escapeHtml(item.instrument_id)}" ${selected === item.instrument_id ? "selected" : ""}>${escapeHtml(item.region)} · ${escapeHtml(item.gpu_model)} · ${escapeHtml(item.instrument_id)}</option>`).join("");
  const draft = state.tradeDraft;
  const composition = (catalog.fixture_composition || []).map((item) => `<article><span>${escapeHtml(item.instrument_id)}</span><strong>${escapeHtml(item.observed_rows)} / ${escapeHtml(catalog.expected_period_count)} slots</strong><small>${escapeHtml(item.fixture_row_share_bps)} bps of fixture rows</small></article>`).join("");
  const series = state.tradeSeries;
  const seriesRows = (series?.observations || []).map((point) => `<tr><td><time>${escapeHtml(point.observed_at)}</time></td><td>${money(point.asking_usd_micros_per_hour)}</td><td>${tradeAvailability(point.available_count)}</td></tr>`).join("");
  const scenario = state.tradeScenario;
  const paper = state.tradePaper;
  const report = scenario && paper ? `<section class="trade-results-grid" aria-label="Trade research results">
      <article class="panel trade-result-card"><div class="panel-heading"><div><p class="eyebrow">REGIONAL ASKING-PRICE BASIS</p><h2 id="trade-result-heading" tabindex="-1">${escapeHtml(scenario.primary_instrument.region)} vs ${escapeHtml(scenario.comparison_instrument.region)}</h2></div><span class="tag tag-caution">Assumption only</span></div>
        <div class="trade-result-stats"><div><span>Observed basis</span><strong>${tradeMoney(scenario.regional_basis_usd_micros_per_hour)}<small> / GPU-hour</small></strong></div><div><span>Scenario basis</span><strong>${tradeMoney(scenario.scenario_regional_basis_usd_micros_per_hour)}<small> / GPU-hour</small></strong></div><div><span>Common points</span><strong>${escapeHtml(scenario.common_period_count)} / ${escapeHtml(catalog.expected_period_count)}</strong></div></div>
        <p class="trade-result-note">The assumed primary price shock is ${escapeHtml(scenario.assumed_price_shock_bps)} bps. Capacity stays ${tradeAvailability(scenario.scenario_available_units)} after the assumed change.</p>
        <div class="trade-sensitivity"><strong>Fixture sensitivity</strong><span>Doubling the highest latest quote changes the regional median by ${tradeMoney(scenario.sensitivity.outlier_median_change_usd_micros_per_hour)} per GPU-hour.</span><span>Using the prior complete period changes the median by ${tradeMoney(scenario.sensitivity.one_period_lag_change_usd_micros_per_hour)}.</span><small>${escapeHtml(scenario.sensitivity.method_label)}</small></div>
        <dl class="detail-list"><div><dt>Scenario digest</dt><dd><code class="digest-full">${escapeHtml(scenario.scenario_digest)}</code></dd></div><div><dt>Sample time</dt><dd>${escapeHtml(scenario.observed_at)}</dd></div></dl>
      </article>
      <article class="panel trade-result-card"><div class="panel-heading"><div><p class="eyebrow">PAPER PROCUREMENT COST</p><h2>Known-capacity replay</h2></div><span class="tag tag-caution">No simulated fill</span></div>
        <div class="trade-result-stats"><div><span>Comparable periods</span><strong>${escapeHtml(paper.comparable_period_count)} / ${escapeHtml(catalog.expected_period_count)}</strong></div><div><span>Baseline asking cost</span><strong>${money(paper.baseline_asking_cost_microusd)}</strong></div><div><span>Selected asking cost</span><strong>${money(paper.strategy_asking_cost_microusd)}</strong></div></div>
        <p class="trade-cost-delta">Baseline minus selected asking cost: <strong>${tradeMoney(paper.baseline_minus_strategy_asking_cost_microusd)}</strong></p>
        <div class="trade-skip-counts"><span>Unknown capacity records <strong>${escapeHtml(paper.unknown_capacity_observation_count)}</strong></span><span>Insufficient capacity records <strong>${escapeHtml(paper.insufficient_capacity_observation_count)}</strong></span><span>No eligible capacity periods <strong>${escapeHtml(paper.no_eligible_capacity_count)}</strong></span></div>
        <div class="table-scroll trade-paper-table"><table><thead><tr><th>Period</th><th>Status</th><th>Selected</th><th>Baseline − selected</th></tr></thead><tbody>${(paper.periods || []).map((period) => `<tr><td>${escapeHtml(period.observed_at)}</td><td><span class="trade-status">${escapeHtml(period.status.replaceAll("_", " "))}</span></td><td>${escapeHtml(period.selected_instrument_id || "—")}</td><td>${period.baseline_minus_strategy_asking_cost_microusd === null ? "—" : tradeMoney(period.baseline_minus_strategy_asking_cost_microusd)}</td></tr>`).join("")}</tbody></table></div>
        <small class="trade-method-label">${escapeHtml(paper.measurement_type)} · ties: ${escapeHtml(paper.tie_breaking)}</small>
        <div class="digest-line trade-replay-digest"><span>PAPER REPLAY DIGEST</span><code class="digest-full">${escapeHtml(paper.replay_digest)}</code></div>
      </article>
    </section>` : `<section class="empty-panel compact-empty"><div class="empty-mark">⌁</div><h2>No research scenario yet</h2><p>Choose a synthetic regional pair, bound the assumptions, then calculate through the shared local service.</p></section>`;
  return `${heading("TRADE / LOCAL SYNTHETIC RESEARCH", "Trade", "Compare regional asking-price observations and replay bounded procurement-cost assumptions over one fixed synthetic release.")}
    <div class="notice notice-caution trade-boundary"><span class="notice-icon">i</span><div><strong>Synthetic research only · no financial actions</strong><p>These results are not a live index, forecast, order, fill, saving, P&amp;L, payment entitlement, or provider availability claim. Requests calculate in memory and create no durable record.</p></div></div>
    <section class="panel trade-source"><div class="trade-source-heading"><div><p class="eyebrow">DATA-OWNED RELEASE</p><h2>${escapeHtml(catalog.dataset_revision)}</h2><p>${escapeHtml(catalog.source_label)} · ${escapeHtml(catalog.freshness)} · ${escapeHtml(catalog.interval_seconds)} second interval</p></div><span class="tag tag-neutral">${escapeHtml(catalog.observed_row_count)} / ${escapeHtml(catalog.expected_row_count)} observations</span></div>
      <div class="trade-source-metrics"><article><span>Grid coverage</span><strong>${escapeHtml((catalog.fixture_coverage_bps / 100).toFixed(2))}%</strong></article><article><span>Regions</span><strong>${escapeHtml(instruments.length)}</strong></article><article><span>Expected slots</span><strong>${escapeHtml(catalog.expected_period_count)} hourly</strong></article><article><span>Fixture composition HHI</span><strong>${escapeHtml(catalog.fixture_composition_hhi_bps)} bps</strong><small>Sample composition only</small></article></div>
      <div class="trade-composition">${composition}</div><div class="digest-line"><span>DATASET DIGEST</span><code class="digest-full">${escapeHtml(catalog.dataset_digest)}</code></div>
    </section>
    <section class="panel trade-controls"><div class="panel-heading"><div><p class="eyebrow">BOUNDED SCENARIO INPUTS</p><h2>Choose a regional comparison</h2></div><span class="tag tag-neutral">±50% price · ±10 units</span></div>
      <form id="trade-form" class="trade-form">
        <label>Primary region<select name="primary_instrument_id" required>${optionList(draft.primary)}</select></label>
        <label>Comparison region<select name="comparison_instrument_id" required>${optionList(draft.comparison)}</select></label>
        <label>Price shock (basis points)<input name="price_shock_bps" type="number" min="-5000" max="5000" step="1" value="${escapeHtml(draft.priceShockBps)}" required></label>
        <label>Availability change (units)<input name="availability_delta_units" type="number" min="-10" max="10" step="1" value="${escapeHtml(draft.availabilityDeltaUnits)}" required></label>
        <label>Paper baseline<select name="baseline_instrument_id" required>${optionList(draft.baseline)}</select></label>
        <label>Requested units<input name="requested_units" type="number" min="1" max="8" step="1" value="${escapeHtml(draft.requestedUnits)}" required></label>
        <button class="button trade-run-button" type="submit" ${state.tradeBusy ? "disabled aria-busy=\"true\"" : ""}>${state.tradeBusy ? "Calculating local research…" : "Run synthetic research"}</button>
      </form>
      <p class="trade-control-note">Price changes are assumptions, not predictions. Unknown availability remains unknown; missing hours break the series comparison.</p>
    </section>
    ${series ? `<section class="panel trade-series"><div class="panel-heading"><div><p class="eyebrow">GAP-AWARE TIME SERIES</p><h2>${escapeHtml(series.instrument.gpu_model)} · ${escapeHtml(series.instrument.region)}</h2></div><span class="tag tag-neutral">${escapeHtml((series.price_coverage_bps / 100).toFixed(2))}% coverage</span></div>
      <div class="trade-series-summary"><span>Mean absolute contiguous change <strong>${series.mean_absolute_contiguous_change_bps === null ? "Insufficient pairs" : `${escapeHtml(series.mean_absolute_contiguous_change_bps)} bps`}</strong></span><span>Unknown availability <strong>${escapeHtml(series.unknown_availability_count)}</strong></span><span>Known zero capacity <strong>${escapeHtml(series.zero_availability_count)}</strong></span><span>Metric <strong>Not forecast volatility</strong></span></div>
      ${series.missing_periods.length ? `<div class="trade-gap-note">Missing price interval${series.missing_periods.length === 1 ? "" : "s"}: ${series.missing_periods.map(escapeHtml).join(" · ")}. The calculation does not bridge these gaps.</div>` : ""}
      <div class="table-scroll"><table><thead><tr><th>Observed at (UTC)</th><th>Synthetic asking price / GPU-hour</th><th>Availability</th></tr></thead><tbody>${seriesRows}</tbody></table></div>
    </section>` : ""}
    ${report}
    <section class="claim-box trade-claims"><span class="eyebrow">CLAIM CEILING</span><p>${escapeHtml(catalog.claim_ceiling)}</p><span class="eyebrow">NONCLAIMS</span><p>${escapeHtml(catalog.nonclaims)}</p><small>Methods: ${(catalog.method_versions || []).map(escapeHtml).join(" · ")}</small></section>`;
}

function renderConnections() {
  const agent = state.agentSnapshot;
  const activeAgent = agent && !["disconnected", "failed"].includes(agent.status);
  const profile = agent?.profile;
  const agentErrorCode = agent?.error_code || state.agentActionErrorCode;
  const sessionTools = (agent?.tools || []).map((tool) => `<li><code>${escapeHtml(tool)}</code></li>`).join("");
  const sessionEvents = (agent?.events || []).map((item) => `<li><span>${String(item.sequence).padStart(2, "0")}</span><p>${escapeHtml(item.text)}</p></li>`).join("");
  const agentForm = !activeAgent ? `<section class="agent-connect-panel">
      <div class="agent-panel-heading"><div><p class="eyebrow">ACP V1 / LOCAL PROCESS</p><h2 id="agent-workspace-title" tabindex="-1">Connect a local agent</h2><p>Choose the exact executable and working directory. HyperAI keeps the profile in service memory for this session.</p></div><span class="tag tag-neutral">User initiated</span></div>
      ${agentErrorCode ? `<p class="agent-error" role="status" aria-live="polite">${escapeHtml(String(agentErrorCode).slice(0, 80))}</p>` : ""}
      <form id="agent-connect-form" class="agent-profile-form">
        <label>Profile name<input name="name" maxlength="80" value="Local ACP agent" required autocomplete="off" /></label>
        <label>Absolute executable path<input name="executable" maxlength="4096" placeholder="/absolute/path/to/agent" required autocomplete="off" /></label>
        <label>Working directory<input name="working_directory" maxlength="4096" placeholder="/absolute/path/to/workspace" required autocomplete="off" /></label>
        <label class="agent-args-field">Arguments <span>one argument per line</span><textarea name="args" rows="3" maxlength="8192" placeholder="--mode&#10;interactive" autocomplete="off"></textarea></label>
        <button class="button button-primary" type="submit" ${state.agentBusy ? "disabled aria-busy=\"true\"" : ""}>${state.agentBusy ? "Connecting…" : "Review and connect"}</button>
      </form>
      <p class="agent-process-disclosure"><strong>Process boundary:</strong> HyperAI exposes only the listed read-only tools and passes the agent a temporary, read-only tool capability. ACP does not sandbox this executable; it may independently access files, network, tools, or credentials available to it.</p>
    </section>` : `<section class="agent-session-panel">
      <div class="agent-panel-heading"><div><p class="eyebrow">ACTIVE LOCAL SESSION</p><h2 id="agent-workspace-title" tabindex="-1">${escapeHtml(profile?.name || "ACP agent")}</h2><p>${escapeHtml(agent.status.replaceAll("_", " "))} · ACP ${escapeHtml(agent.protocol_version ? `v${agent.protocol_version}` : "negotiating")}</p></div><span class="tag ${agent.status === "running" ? "tag-caution" : "tag-neutral"}">${escapeHtml(agent.status.replaceAll("_", " ").toUpperCase())}</span></div>
      <dl class="agent-profile-summary"><div><dt>Executable</dt><dd><code>${escapeHtml(profile?.executable || "Not available")}</code></dd></div><div><dt>Working directory</dt><dd><code>${escapeHtml(profile?.working_directory || "Not available")}</code></dd></div><div><dt>Local session</dt><dd><code>${escapeHtml(agent.local_session_id || "Negotiating")}</code></dd></div><div><dt>Profile digest</dt><dd><code>${escapeHtml(agent.profile_digest || "Not available")}</code></dd></div><div><dt>Tool scope</dt><dd><code>${escapeHtml(agent.tool_set_version || "Not available")}</code> · ${escapeHtml(agent.data_scope || "")}</dd></div></dl>
      ${agentErrorCode ? `<p class="agent-error" role="status" aria-live="polite">${escapeHtml(String(agentErrorCode).slice(0, 80))}</p>` : ""}
      <div class="agent-session-columns"><section><h3>Read-only HyperAI tools</h3><ul class="agent-tool-list">${sessionTools}</ul><p>Data, Fleet, Eval and Trade research calls stay behind the local service policy.</p></section><section><h3>Negotiated capabilities</h3><p>${escapeHtml((agent.capabilities || []).join(" · ") || "No optional capabilities reported")}</p></section></div>
      <form id="agent-prompt-form" class="agent-prompt-form"><label for="agent-prompt">Send a prompt to this local process</label><textarea id="agent-prompt" name="prompt" maxlength="16384" rows="4" placeholder="Prompt text is sent once and is not stored by HyperAI." required ${state.agentBusy ? "disabled" : ""}></textarea><div><button class="button button-primary" type="submit" ${state.agentBusy ? "disabled aria-busy=\"true\"" : ""}>${state.agentBusy ? "Waiting for agent…" : "Send prompt"}</button><button class="button button-secondary" data-action="agent-cancel" type="button" ${agent.status !== "running" && !state.agentBusy || state.agentControlBusy ? "disabled" : ""}>${state.agentControlBusy ? "Stopping…" : "Cancel active request"}</button><button class="button button-secondary" data-action="agent-disconnect" type="button" ${state.agentControlBusy ? "disabled" : ""}>Disconnect</button></div></form>
      <section class="agent-events"><div><h3>Session messages</h3><small>Recent assistant text only · held in memory</small></div>${sessionEvents ? `<ol>${sessionEvents}</ol>` : `<p class="empty-inline">No assistant messages yet.</p>`}</section>
      <p class="agent-process-disclosure"><strong>Process boundary:</strong> the temporary capability is limited to these HyperAI routes. It does not restrict the executable's own filesystem, network, tools, or credentials.</p>
    </section>`;
  return `${heading("COMPOSITION / OWNERSHIP", "Agents & access", "One interface for navigation; each module keeps its own records, runtime, and authority.")}
    <div class="notice notice-caution"><span class="notice-icon">i</span><div><strong>Static inventory · not a health check</strong><p>Owners, revisions, surfaces, and gates are descriptive local metadata. They do not probe external services, establish rights, or grant authority.</p></div></div>
    <div class="section-heading"><div><h2>Connection inventory</h2><p>Current module boundaries and local readiness labels.</p></div></div>
    ${connectionInventoryError ? `<p class="empty-inline" role="status">${escapeHtml(connectionInventoryError)}</p>` : ""}
    ${window.HyperAIConnectionInventoryView.renderList(connectionInventory)}
    ${agentForm}
    <div class="section-heading"><div><p class="eyebrow">COMPOSITION MODEL</p><h2>Records stay with their owner.</h2></div></div><div class="ownership-grid"><article><span>01 / DISCOVERY</span><h3>Market</h3><p>Simple Compute Market patterns and advisory data feed a future read-only registry surface.</p></article><article><span>02 / EXECUTION</span><h3>Inference &amp; fleet</h3><p>HyperIntel owns inference routing. Fleet operations remain separately authorized.</p></article><article><span>03 / OUTCOMES</span><h3>Evidence &amp; settlement</h3><p>Outcome resolution, evidence custody, and any payment each retain distinct contracts.</p></article></div>`;
}

function renderConnectionCards(limit = Infinity) {
  return window.HyperAIConnectionInventoryView.renderCards(connectionInventory, limit);
}

function renderConnectionFootnote(text) { return `<div class="notice notice-neutral"><span class="notice-icon">i</span><div><strong>${escapeHtml(text)}</strong><p>The local demo does not call an external system.</p></div></div>`; }

function emptyState(title, description, action) {
  return `<section class="empty-panel"><div class="empty-mark">◇</div><h2>${escapeHtml(title)}</h2><p>${escapeHtml(description)}</p>${button("Run full synthetic lifecycle", action)}</section>`;
}

const informationalPages = {
  inference: { kicker: "HYPERINTEL / SEPARATE AUTHORITY", title: "Inference", description: "Verify offline market-evidence bundles and reconcile caller-selected execution records under caller-supplied trust.", owner: "HyperIntel", boundary: "No inference router, execution path, provider adapter, or store connection is exposed here.", state: "local_offline_verification_and_reconciliation_available" },
  fleet: { kicker: "OPERATIONS / READ ONLY FIRST", title: "Fleet", description: "A future fleet surface will begin with owner-labelled inventory and telemetry.", owner: "Fleet operator", boundary: "No allocation, SSH, launch, workload, or termination control is available in this local shell.", state: "not_connected" },
  eval: { kicker: "RESEARCH / CONTROLLED EVALUATION", title: "Eval", description: "Evaluation views will bind results to a frozen model, harness, data, controls, budget, and stopping rule.", owner: "Evaluation operator", boundary: "No agent runs or benchmark fixtures are loaded by the interface.", state: "not_connected" },
  access: { kicker: "PERMISSIONS / SOURCE OWNERSHIP", title: "Agents & access", description: "Connect one explicitly selected local ACP v1 process to a fixed, read-only HyperAI tool surface.", owner: "HyperAI application policy", boundary: "The local agent capability cannot approve external actions or grant provider authority.", state: "local_only" },
  activity: { kicker: "ACTIVITY / LOCAL SESSION", title: "Activity", description: "Review results already loaded in this browser session across source-owned workspaces. Records stay with their owners; this view does not create a shared history.", owner: "HyperAI local interface", boundary: "Session-only presentation; not a durable or cross-product audit log.", state: "not_connected" }
};

function renderActivityList(steps) {
  return `<ol class="activity-list" aria-label="Synthetic quote lifecycle">${steps.map((step, index) => `<li><span class="activity-index">0${index + 1}</span><div><h3>${escapeHtml(step.title)}</h3><p>${escapeHtml(step.detail)}</p><code title="${escapeHtml(step.digest)}">${escapeHtml(shortDigest(step.digest))}</code></div><span class="tag tag-caution">${escapeHtml(step.status)}</span></li>`).join("")}</ol>`;
}

function sessionActivityRecords() {
  const records = [];
  const hasDigest = (value) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
  const capture = state.simpleMarketCapture;
  if (capture && hasDigest(capture.response_body_sha256)) {
    records.push({
      source: "SIMPLE COMPUTE MARKET",
      title: "Signed listings capture",
      detail: `${capture.listing_count} listings · response ${capture.response_status} · request ${capture.request_id} · caller pins supplied`,
      digest: capture.response_body_sha256,
      digestLabel: "Response body SHA-256",
      status: "SIGNATURES VERIFIED",
      note: "The query-body hash is not signed; listing freshness and replay protection are not established.",
      destination: "Market",
      page: "market",
    });
  }
  const evidence = state.hyperintelVerification;
  if (evidence && hasDigest(evidence.bundle_digest)) {
    records.push({
      source: "HYPERINTEL",
      title: "Market-evidence bundle",
      detail: `${evidence.bundle_schema_version} · ${evidence.candidate_count} candidates · ${evidence.trusted_key_count} caller keys · evaluated ${evidence.evaluated_at}`,
      digest: evidence.bundle_digest,
      digestLabel: "Bundle digest",
      status: "OFFLINE CONTRACT PASS",
      note: "Source-contract reconstruction under caller trust; evaluation time is not a freshness judgment.",
      destination: "Inference",
      page: "inference",
    });
  }
  const reconciliation = state.hyperintelReconciliation;
  if (reconciliation && hasDigest(reconciliation.report?.report_digest)) {
    records.push({
      source: "HYPERINTEL",
      title: "Execution-lineage report",
      detail: `${reconciliation.report.status} · ${reconciliation.signed_offer_count} signed offers · ${reconciliation.trusted_key_count} caller keys · authority none`,
      digest: reconciliation.report.report_digest,
      digestLabel: "Report digest",
      status: "LOCAL RECONCILIATION",
      note: "The report checks record consistency; it does not establish correct computation, payment, or settlement.",
      destination: "Inference",
      page: "inference",
    });
  }
  const dataResult = state.dataResult;
  if (dataResult && hasDigest(dataResult.snapshot_digest)
    && typeof dataResult.dataset_id === "string" && Number.isSafeInteger(dataResult.row_count)
    && Array.isArray(dataResult.columns)) {
    records.push({
      source: "HYPERAI DATA",
      title: "Local query result",
      detail: `${dataResult.row_count} rows · ${dataResult.columns.length} columns · ${dataResult.dataset_id}`,
      digest: dataResult.snapshot_digest,
      digestLabel: "Source snapshot digest",
      status: "LOCAL QUERY RESULT",
      note: "The digest identifies the input snapshot, not this query result. SQL and result cells stay in the Data workspace.",
      destination: "Data",
      page: "data",
    });
  }
  const fleetPlan = state.fleetPlan;
  if (fleetPlan && hasDigest(fleetPlan.preview_digest)
    && ["preview_ready", "preview_changed"].includes(fleetPlan.preview_status)
    && typeof fleetPlan.operation === "string" && typeof fleetPlan.node_id === "string") {
    records.push({
      source: "HYPERAI FLEET",
      title: "Advisory operation preview",
      detail: `${fleetPlan.operation.replaceAll("_", " ")} · ${fleetPlan.node_id}`,
      digest: fleetPlan.preview_digest,
      digestLabel: "Preview digest",
      status: fleetPlan.preview_status === "preview_changed" ? "PREVIEW CHANGED" : "ADVISORY PREVIEW",
      note: "This digest binds a synthetic plan only. It does not invoke, reserve, or inspect a host or provider resource.",
      destination: "Fleet",
      page: "fleet",
    });
  }
  const evalReport = state.evalReport;
  if (evalReport && hasDigest(evalReport.suite_digest) && hasDigest(evalReport.repeated_result_digest)
    && typeof evalReport.suite_id === "string" && evalReport.control && evalReport.alternate) {
    records.push({
      source: "HYPERAI EVAL",
      title: "Harness fixture comparison",
      detail: `${evalReport.suite_id} · control ${evalReport.control.matching_cases}/${evalReport.control.case_count} · alternate ${evalReport.alternate.matching_cases}/${evalReport.alternate.case_count}`,
      digest: evalReport.repeated_result_digest,
      digestLabel: "Repeated fixture result digest",
      status: "SYNTHETIC FIXTURE",
      note: "A fixed response-fixture comparison only; no model or agent ran, and no quality claim follows.",
      destination: "Eval",
      page: "eval",
    });
  }
  const tradePaper = state.tradePaper;
  if (tradePaper && hasDigest(tradePaper.dataset_digest) && hasDigest(tradePaper.replay_digest)
    && Number.isSafeInteger(tradePaper.comparable_period_count)) {
    records.push({
      source: "HYPERAI TRADE",
      title: "Paper procurement-cost replay",
      detail: `${tradePaper.comparable_period_count} comparable periods · ${tradePaper.requested_units} hypothetical units · asking-cost delta ${tradeMoney(tradePaper.baseline_minus_strategy_asking_cost_microusd)}`,
      digest: tradePaper.replay_digest,
      digestLabel: "Paper replay digest",
      status: "LOCAL RESEARCH",
      note: "A bounded asking-price calculation, not a live index, forecast, order, fill, P&L, savings, or payment.",
      destination: "Trade Research",
      page: "trade",
    });
  }
  return records;
}

function renderSessionActivityRecords(records) {
  if (!records.length) return "";
  return `<section class="session-activity"><div class="section-heading"><div><p class="eyebrow">SOURCE-OWNED RESULTS</p><h2>Current session records</h2></div><span class="tag tag-neutral">No shared ledger</span></div><ul class="session-record-list" aria-label="Current source result summaries">${records.map((record) => `<li><article class="session-record"><p class="eyebrow">${escapeHtml(record.source)}</p><h3>${escapeHtml(record.title)}</h3><p>${escapeHtml(record.detail)}</p><p class="eyebrow session-record-identity">${escapeHtml(record.digestLabel || "Source digest")}</p><code title="${escapeHtml(record.digest)}">${escapeHtml(record.digest)}</code><small>${escapeHtml(record.note)}</small><div class="session-record-footer"><span class="tag tag-caution">${escapeHtml(record.status)}</span><button class="button button-secondary" type="button" data-page="${escapeHtml(record.page)}">Open ${escapeHtml(record.destination)}</button></div></article></li>`).join("")}</ul><p class="retention-note">These cards summarize result objects currently held in this page's memory. They are not chronological history and are not combined into another product's record.</p></section>`;
}

function renderInformational(page) {
  const item = informationalPages[page];
  const receipt = demoReceipt();
  const sessionRecords = page === "activity" ? renderSessionActivityRecords(sessionActivityRecords()) : "";
  let content = "";
  if (page === "activity" && state.quotePreview) {
    const previewBody = state.quotePreview.body;
    const request = previewBody.buyer_request;
    const steps = [{
      title: "Seller quote preview",
      detail: `${previewBody.selected_offer_id} · ${request.resource_model} · ${request.units} units for ${request.duration_hours} hours · cap ${money(request.max_total_microusd)}`,
      digest: state.quotePreview.digest,
      status: "PREVIEW",
    }];
    if (state.quoteApproval) {
      steps.push({
        title: "Synthetic buyer approval",
        detail: `Fixture key · simulated ${state.quoteApproval.approved_deal.accepted_at} · binds preview ${shortDigest(state.quoteApproval.preview_digest)}`,
        digest: state.quoteApproval.deal_digest,
        status: "SYNTHETIC",
      });
    }
    if (state.quoteCompletion) {
      const completedReceipt = state.quoteCompletion.demo.receipt;
      steps.push({
        title: "Mock fulfillment and disposition",
        detail: `${completedReceipt.body.fulfillment.seller_id} fixture report · deal ${shortDigest(state.quoteCompletion.deal_digest)} · ${completedReceipt.body.mock_settlement.disposition.replaceAll("_", " ")} · no funds moved`,
        digest: completedReceipt.digest,
        status: "MOCK",
      });
    }
    content = `${renderActivityList(steps)}${sessionRecords}`;
  } else if (page === "activity" && receipt) {
    const steps = [
      { title: "Synthetic catalog", detail: `${receipt.body.catalog.body.source_count} sources · ${receipt.body.catalog.body.duplicate_count} duplicate collapsed`, digest: receipt.body.catalog.digest, status: "OBSERVED" },
      { title: "Synthetic buyer approval", detail: `${receipt.body.buyer_request.units} units · ${receipt.body.buyer_request.duration_hours} hours · cap ${money(receipt.body.buyer_request.max_total_microusd)}`, digest: state.demo.report.deal_digest, status: "SYNTHETIC" },
      { title: "Seller-reported fulfillment", detail: `${receipt.body.fulfillment.seller_id} fixture report · ${money(receipt.body.fulfillment.billed_microusd)} billed`, digest: state.demo.report.fulfillment_digest, status: "ATTRIBUTED" },
      { title: "Mock disposition", detail: `${receipt.body.mock_settlement.disposition.replaceAll("_", " ")} · no funds moved`, digest: receipt.digest, status: "MOCK" },
    ];
    content = `${renderActivityList(steps)}${sessionRecords}`;
  } else if (page === "activity" && sessionRecords) {
    content = sessionRecords;
  } else if (page === "activity") {
    content = `<section class="empty-panel compact-empty"><div class="empty-mark">≋</div><p class="eyebrow">NO SESSION RESULTS YET</p><h2>Start with a local record</h2><p>Review a synthetic quote or run the full walkthrough. Results loaded from Market, Data, Fleet, Eval, Trade, and HyperIntel can also appear here as source-linked summaries.</p>${button("Review synthetic quote", "review-quote")}${button("Run full synthetic lifecycle", "run-demo", "button button-secondary")}</section>`;
  } else {
    content = `<section class="module-preview"><div class="preview-top"><span class="${statusClass(item.state)}">${escapeHtml(connectionStateLabel(item.state))}</span><span class="preview-owner">OWNER / ${escapeHtml(item.owner)}</span></div><h2>${escapeHtml(item.boundary)}</h2><p>When an adapter is added, it will expose a named, versioned contract through the HyperAI facade. The interface will show the source, freshness, permissions, and claim limits at the point of use.</p><button class="button button-secondary" data-page="access">View connection map <span aria-hidden="true">→</span></button></section>`;
  }
  return `${heading(item.kicker, item.title, item.description)}${content}`;
}

function render() {
  const crumb = document.getElementById("page-crumb");
  crumb.textContent = pageNames[state.page] || "OVERVIEW";
  document.querySelectorAll(".nav-item[data-page]").forEach((item) => {
    const active = item.dataset.page === state.page;
    item.classList.toggle("is-active", active);
    if (active) item.setAttribute("aria-current", "page"); else item.removeAttribute("aria-current");
  });
  if (state.apiError) {
    banner.hidden = false;
    banner.innerHTML = `<span class="banner-dot"></span><span><strong>Local interface error</strong> ${escapeHtml(state.apiError)}</span><button aria-label="Dismiss" data-action="dismiss-banner">×</button>`;
  } else if (!state.status) {
    banner.hidden = false;
    banner.innerHTML = `<span class="banner-dot"></span><span><strong>Local API unavailable.</strong> Start the interface with <code>corepack pnpm run interface:local</code>.</span>`;
  } else { banner.hidden = true; }
  if (state.page === "overview") pageRoot.innerHTML = renderOverview();
  else if (state.page === "market") pageRoot.innerHTML = renderMarket();
  else if (state.page === "deals") pageRoot.innerHTML = renderDeals();
  else if (state.page === "data") pageRoot.innerHTML = renderData();
  else if (state.page === "inference") pageRoot.innerHTML = renderInference();
  else if (state.page === "fleet") pageRoot.innerHTML = renderFleet();
  else if (state.page === "eval") pageRoot.innerHTML = renderEval();
  else if (state.page === "trade") pageRoot.innerHTML = renderTrade();
  else if (state.page === "evidence") pageRoot.innerHTML = renderEvidence();
  else if (state.page === "access") pageRoot.innerHTML = renderConnections();
  else if (state.page === "connections") pageRoot.innerHTML = renderConnections();
  else pageRoot.innerHTML = renderInformational(state.page);
}

async function initialize() {
  try {
    const session = await requestJson("/api/v1/session");
    state.csrfToken = session.csrf_token;
    state.status = await requestJson("/api/v1/status");
    try {
      connectionInventory = await requestJson("/api/v1/connections");
      connectionInventoryError = "";
    } catch (error) {
      connectionInventory = null;
      connectionInventoryError = error.message;
    }
    await Promise.all([refreshLocalDataCatalog(), refreshFleetWorkspace(), refreshEvalWorkspace(), refreshTradeWorkspace(), refreshAgentWorkspace(), refreshWorkloadTrace(), refreshMarketSupplyComparison(), refreshMarketHistory()]);
  }
  catch (error) { if (!state.status) state.status = null; state.apiError = error.message; }
  render();
}

window.setInterval(async () => {
  try {
    const session = await requestJson("/api/v1/session");
    state.csrfToken = session.csrf_token;
  } catch (error) {
    state.apiError = error.message;
    render();
  }
}, 15 * 60 * 1000);

document.addEventListener("click", (event) => {
  const target = event.target instanceof Element ? event.target.closest("[data-page], [data-action]") : null;
  if (!target || target.hasAttribute("disabled")) return;
  if (target.dataset.page) {
    state.page = target.dataset.page;
    syncMobileNavigation(false);
    render();
    (pageRoot.querySelector("#workspace-page-title") || pageRoot).focus({ preventScroll: true });
  } else if (target.dataset.action === "run-demo") { void runDemo(); }
  else if (target.dataset.action === "review-quote") { void reviewSyntheticQuote(); }
  else if (target.dataset.action === "preview-supply-quote") { void previewSelectedSupplyQuote(); }
  else if (target.dataset.action === "approve-quote") { void approveSyntheticQuote(); }
  else if (target.dataset.action === "complete-quote") { void completeSyntheticQuote(); }
  else if (target.dataset.action === "load-more-market-history") { void loadMoreMarketHistory(); }
  else if (target.dataset.action === "refresh-market-history") { void retryMarketHistory(); }
  else if (target.dataset.action === "verify-receipt") { void verifyReceipt(); }
  else if (target.dataset.action === "verify-hyperintel") { void verifyHyperIntelBundle(); }
  else if (target.dataset.action === "reconcile-hyperintel") { void reconcileHyperIntelExecution(); }
  else if (target.dataset.action === "verify-registry-capture") { void verifySimpleMarketCapture(); }
  else if (target.dataset.action === "load-workload-trace") { void loadWorkloadTraceAndFocus("success"); }
  else if (target.dataset.action === "preview-seller-failure") { void loadWorkloadTraceAndFocus("seller-failure-preview"); }
  else if (target.dataset.action === "preview-inconclusive-outcome") { void loadWorkloadTraceAndFocus("inconclusive-preview"); }
  else if (target.dataset.action === "preview-duplicate-disposition") { void loadWorkloadTraceAndFocus("duplicate-disposition-preview"); }
  else if (target.dataset.action.startsWith("preview-data-status-")) {
    const status = target.dataset.action.slice("preview-data-status-".length);
    if (["missing", "conflicting", "denied"].includes(status)) {
      void loadWorkloadTraceAndFocus(`data-status-${status}`);
    }
  }
  else if (target.dataset.action === "seed-synthetic-data") { void seedSyntheticData(); }
  else if (target.dataset.action === "refresh-gpudiff-data") { void refreshGpudiffData(); }
  else if (target.dataset.action === "start-gpudiff-schedule") { void updateGpudiffSchedule("start"); }
  else if (target.dataset.action === "stop-gpudiff-schedule") { void updateGpudiffSchedule("stop"); }
  else if (target.dataset.action === "refresh-gpudiff-schedule-status") { void updateGpudiffSchedule("status"); }
  else if (target.dataset.action === "cancel-data-query") { void cancelLocalDataQuery(); }
  else if (target.dataset.action === "confirm-data-model") { void savePreviewedDataModel(); }
  else if (target.dataset.action === "confirm-data-view") { void savePreviewedDataView(); }
  else if (target.dataset.action === "run-saved-model") {
    const record = (state.dataModels?.models || []).find((item) => item.model.model_id === target.dataset.modelId);
    if (record) void runLocalDataQuery(record.model.dataset_id, record.model.sql);
  }
  else if (target.dataset.action === "clear-scm-query") {
    state.scmListingQuery = { search: "", resource: "all", status: "all" };
    render();
    pageRoot.querySelector("#scm-listing-search")?.focus({ preventScroll: true });
  }
  else if (target.dataset.action === "evidence") { state.page = "evidence"; render(); }
  else if (target.dataset.action === "dismiss-banner") { state.apiError = ""; state.status = state.status || {}; render(); }
  else if (target.dataset.action === "run-eval") { void runEvalQualification(); }
  else if (target.dataset.action === "agent-cancel") { void controlAgent("/api/v1/agent/cancel", "Cancellation requested from the local agent."); }
  else if (target.dataset.action === "agent-disconnect") { void controlAgent("/api/v1/agent/disconnect", "Local ACP session disconnected."); }
});

document.addEventListener("submit", (event) => {
  if (!(event.target instanceof HTMLFormElement)) return;
  event.preventDefault();
  const form = new FormData(event.target);
  if (event.target.id === "agent-connect-form") {
    const args = String(form.get("args") || "").split(/\r?\n/).filter((arg) => arg.length > 0);
    void runAgentAction("/api/v1/agent/connect", {
      profile: {
        name: String(form.get("name") || "").trim(),
        executable: String(form.get("executable") || "").trim(),
        working_directory: String(form.get("working_directory") || "").trim(),
        args,
        environment: {},
      },
    }, "ACP v1 session connected. Review its tool scope before sending a prompt.");
  } else if (event.target.id === "agent-prompt-form") {
    const prompt = String(form.get("prompt") || "");
    if (!prompt.trim() || new TextEncoder().encode(prompt).length > 16384) {
      showToast("Prompt must contain 1–16,384 UTF-8 bytes.", true);
      return;
    }
    void runAgentAction("/api/v1/agent/prompt", { prompt }, "The local ACP request completed.");
  } else if (event.target.id === "scm-listing-query") {
    state.scmListingQuery = {
      search: String(form.get("search") || "").trim().slice(0, 120),
      resource: String(form.get("resource") || "all"),
      status: String(form.get("status") || "all"),
    };
    render();
    pageRoot.querySelector("#scm-listing-search")?.focus({ preventScroll: true });
  } else if (event.target.id === "data-query-form") {
    const datasetId = String(form.get("dataset_id") || "");
    const sql = String(form.get("sql") || "");
    void runLocalDataQuery(datasetId, sql);
  } else if (event.target.id === "data-model-form") {
    void previewDataModel(String(form.get("model_id") || ""));
  } else if (event.target.id === "data-view-form") {
    const columns = String(form.get("columns") || "").split(",").map((column) => column.trim()).filter(Boolean);
    void previewDataView(String(form.get("view_id") || ""), String(form.get("presentation") || "table"), columns);
  } else if (event.target.id === "fleet-plan-form") {
    void previewFleetOperation(
      String(form.get("node_id") || ""),
      String(form.get("operation") || ""),
    );
  } else if (event.target.id === "fleet-simulation-form") {
    const operation = String(form.get("operation") || "attach");
    const gpuUnits = Number(form.get("gpu_units") || state.fleetSimulationGpuUnits || 1);
    void transitionFleetSimulation(
      operation,
      String(form.get("fault") || "none"),
      gpuUnits,
    );
  } else if (event.target.id === "trade-form") {
    const draft = {
      primary: String(form.get("primary_instrument_id") || ""),
      comparison: String(form.get("comparison_instrument_id") || ""),
      baseline: String(form.get("baseline_instrument_id") || ""),
      priceShockBps: Number(form.get("price_shock_bps")),
      availabilityDeltaUnits: Number(form.get("availability_delta_units")),
      requestedUnits: Number(form.get("requested_units")),
    };
    if (!Number.isInteger(draft.priceShockBps) || draft.priceShockBps < -5000 || draft.priceShockBps > 5000
      || !Number.isInteger(draft.availabilityDeltaUnits) || draft.availabilityDeltaUnits < -10 || draft.availabilityDeltaUnits > 10
      || !Number.isInteger(draft.requestedUnits) || draft.requestedUnits < 1 || draft.requestedUnits > 8) {
      showToast("Trade inputs are outside the local research bounds.", true);
      return;
    }
    void runTradeResearch(draft);
  }
});

document.addEventListener("change", (event) => {
  if (event.target instanceof HTMLSelectElement && event.target.id === "data-dataset-id") {
    state.dataDatasetId = event.target.value;
    state.dataSql = defaultDataSql(state.dataDatasetId);
    render();
    pageRoot.querySelector("#data-sql")?.focus({ preventScroll: true });
    return;
  }
  const formElement = event.target instanceof Element ? event.target.closest("#fleet-simulation-form") : null;
  if (!(formElement instanceof HTMLFormElement)) return;
  const form = new FormData(formElement);
  const operationSelect = formElement.elements.namedItem("operation");
  const faultSelect = formElement.elements.namedItem("fault");
  const gpuInput = formElement.elements.namedItem("gpu_units");
  state.fleetSimulationOperation = String(form.get("operation") || "attach");
  const allowedFaults = new Set(allowedFleetSimulationFaults(state.fleetSimulationOperation));
  for (const option of faultSelect.options) option.disabled = !allowedFaults.has(option.value);
  if (!allowedFaults.has(faultSelect.value)) faultSelect.value = "none";
  state.fleetSimulationFault = faultSelect.value;
  state.fleetSimulationGpuUnits = Number(form.get("gpu_units") || state.fleetSimulationGpuUnits || 1);
  gpuInput.disabled = state.fleetSimulationOperation !== "launch";
  operationSelect.value = state.fleetSimulationOperation;
});

mobileNavToggle.addEventListener("click", () => {
  syncMobileNavigation(!shell.classList.contains("sidebar-open"), true);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && shell.classList.contains("sidebar-open")) {
    syncMobileNavigation(false);
    mobileNavToggle.focus({ preventScroll: true });
  }
});

initialize();
