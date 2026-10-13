/* Pure, dependency-free workload trace panel rendering for the local interface. */
(() => {
  const DATA_STATUS_PREVIEW_SLICE = "hyperai.bazaar.workload_trace_data_status_served.v1";
  const DUPLICATE_DISPOSITION_PREVIEW_SLICE = "hyperai.bazaar.workload_trace_duplicate_disposition_served.v1";
  const DATA_STATUS_OPTIONS = ["missing", "conflicting", "denied"];

  function humanize(value, fallback) {
    return String(value ?? fallback).replaceAll("_", " ");
  }

  function dataStatusFromMode(mode) {
    if (!String(mode).startsWith("data-status-")) return "";
    const status = String(mode).slice("data-status-".length);
    return DATA_STATUS_OPTIONS.includes(status) ? status : "";
  }

  function traceScenarioActions(button) {
    const options = DATA_STATUS_OPTIONS.map((status) => button(
      `Preview ${status} Data reference`,
      `preview-data-status-${status}`,
      "button button-secondary",
    )).join("");
    return `<details class="trace-scenario-details"><summary>Preview synthetic edge cases</summary><div class="trace-scenario-actions"><p class="trace-scenario-note">Data-status fixtures · no Data source read</p>${options}<p class="trace-scenario-note">Same-deal settlement retry · local mock ledger only</p>${button("Preview duplicate disposition replay refusal", "preview-duplicate-disposition", "button button-secondary")}</div></details>`;
  }

  function renderPanel({ trace, mode = "success", error, connections, escapeHtml, shortDigest, button }) {
    if (!trace) {
      const failurePreview = mode === "seller-failure-preview";
      const inconclusivePreview = mode === "inconclusive-preview";
      const duplicateDispositionPreview = mode === "duplicate-disposition-preview";
      const dataStatus = dataStatusFromMode(mode);
      const dataStatusPreview = Boolean(dataStatus);
      const detail = error
        ? `The read-only trace could not be loaded: ${escapeHtml(error)}${dataStatusPreview ? " This fixed Data status preview performs no Data read." : duplicateDispositionPreview ? " The duplicate retry preview creates no second receipt or funds movement." : ""}`
        : failurePreview
          ? "Load the fixed seller-reported failure preview. It is synthetic and performs no workload or deal action."
          : inconclusivePreview
          ? "Load the fixed inconclusive mock-outcome preview. It does not create a dispute or hold funds."
          : duplicateDispositionPreview
            ? "Load the fixed same-deal retry preview. The mock ledger refuses the repeated settlement in memory; it creates no second receipt or funds movement."
          : dataStatusPreview
            ? `Load the fixed synthetic ${dataStatus} Data-reference preview. It performs no Data read and reports no real source or access condition.`
            : "The interface composes owner-labelled references to the synthetic request, deal, receipt, Data input, Eval suite, Fleet fixture, and Trade research catalog.";
      const retryLabel = failurePreview
        ? "Retry failure preview"
        : inconclusivePreview
          ? "Retry inconclusive preview"
          : duplicateDispositionPreview
            ? "Retry duplicate disposition preview"
          : dataStatusPreview
            ? `Retry ${dataStatus} Data preview`
            : "Load successful trace";
      const retryAction = failurePreview
        ? "preview-seller-failure"
        : inconclusivePreview
          ? "preview-inconclusive-outcome"
          : duplicateDispositionPreview
            ? "preview-duplicate-disposition"
          : dataStatusPreview
            ? `preview-data-status-${dataStatus}`
            : "load-workload-trace";
      const alternate = failurePreview || inconclusivePreview || duplicateDispositionPreview || dataStatusPreview
        ? button("Load successful trace", "load-workload-trace", "button button-secondary")
        : `${button("Preview seller-reported failure", "preview-seller-failure", "button button-secondary")}${button("Preview inconclusive mock outcome", "preview-inconclusive-outcome", "button button-secondary")}`;
      return `<section class="panel workload-trace-panel" aria-labelledby="workload-trace-heading"><div class="panel-heading"><div><p class="eyebrow">CROSS-MODULE RECORD MAP</p><h2 id="workload-trace-heading" tabindex="-1">Workload trace</h2></div><span class="tag tag-caution">Local synthetic</span></div><p class="trace-intro" role="status" aria-live="polite">${detail}</p><div class="trace-actions">${button(retryLabel, retryAction, "button button-secondary")}${alternate}</div>${traceScenarioActions(button)}</section>`;
    }

    const ownerWorkspaces = {
      "hyperai-data": { name: "Data", page: "data", moduleId: "data" },
      "hyperai-eval": { name: "Eval", page: "eval", moduleId: "eval" },
      "hyperai-fleet": { name: "Fleet", page: "fleet", moduleId: "fleet" },
      "hyperai-market": { name: "Market", page: "market", moduleId: "compute_market" },
      "hyperai-trade": { name: "Trade research", page: "trade", moduleId: "trade" },
    };
    const grouped = new Map();
    const references = Array.isArray(trace.references) ? trace.references : [];
    for (const reference of references) {
      const owner = reference.owner_id || "unknown";
      if (!grouped.has(owner)) grouped.set(owner, []);
      grouped.get(owner).push(reference);
    }

    const sources = [...grouped.entries()].map(([owner, ownerReferences], index) => {
      const className = owner === "hyperai-market" ? " trace-source-card-market" : "";
      const workspace = ownerWorkspaces[owner];
      const ownerName = workspace?.name || owner;
      const headingId = `workload-trace-owner-${index}`;
      const inventoryState = !Array.isArray(connections)
        ? "Unavailable"
        : workspace?.moduleId
          ? humanize(connections.find((connection) => connection.module_id === workspace.moduleId)?.state, "No inventory entry")
          : "No inventory entry";
      const ownerNavigation = workspace
        ? `<button class="text-button trace-owner-link" type="button" data-page="${workspace.page}" aria-label="Open ${escapeHtml(ownerName)} workspace">Open workspace <span aria-hidden="true">→</span></button>`
        : "";
      const items = ownerReferences.map((reference, referenceIndex) => {
        const status = humanize(reference.status, "unknown");
        const linkBasis = humanize(reference.link_basis, "Link basis unavailable");
        const claimCeiling = humanize(reference.claim_ceiling, "Claim ceiling unavailable");
        const digestDescriptionId = `workload-trace-source-digest-${index}-${referenceIndex}`;
        const digest = reference.digest
          ? `<span id="${digestDescriptionId}" hidden>Source-provided digest: ${escapeHtml(reference.digest)}</span><code aria-hidden="true">${escapeHtml(shortDigest(reference.digest))}</code>`
          : `<span id="${digestDescriptionId}" hidden>Source-provided digest unavailable.</span><code aria-hidden="true">No separate source digest</code>`;
        return `<li class="trace-reference" aria-describedby="${digestDescriptionId}"><div class="trace-reference-row"><div><strong>${escapeHtml(reference.record_id)}</strong><span class="trace-status"><span class="visually-hidden">Status: </span>${escapeHtml(status)}</span></div>${digest}</div><details class="trace-reference-details"><summary>Link and claim details</summary><dl><div><dt>Record schema</dt><dd><code>${escapeHtml(reference.schema_version || "Schema unavailable")}</code></dd></div><div><dt>Why linked</dt><dd>${escapeHtml(linkBasis)}</dd></div><div><dt>Claim ceiling</dt><dd>${escapeHtml(claimCeiling)}</dd></div></dl></details></li>`;
      }).join("");
      return `<article class="trace-source-card${className}" aria-labelledby="${headingId}"><div class="trace-source-heading"><div class="trace-source-heading-top"><h3 id="${headingId}">${escapeHtml(ownerName)}</h3><small>${ownerReferences.length} ${ownerReferences.length === 1 ? "reference" : "references"}</small></div><div class="trace-owner-actions"><div class="trace-connection-state"><span>Static inventory · not a live probe</span><strong>${escapeHtml(inventoryState)}</strong></div>${ownerNavigation}</div></div><ul class="trace-reference-list" aria-label="${escapeHtml(ownerName)} source records">${items}</ul></article>`;
    }).join("");

    const sellerFailure = trace.state_slice === "hyperai.bazaar.workload_trace_seller_failure.v1";
    const inconclusivePreview = trace.state_slice === "hyperai.bazaar.workload_trace_inconclusive_served.v1";
    const duplicateDispositionPreview = trace.state_slice === DUPLICATE_DISPOSITION_PREVIEW_SLICE;
    const dataStatusPreview = trace.state_slice === DATA_STATUS_PREVIEW_SLICE;
    const dataReference = dataStatusPreview
      ? references.find((reference) => reference.owner_id === "hyperai-data")
      : null;
    const dataStatus = dataStatusPreview && DATA_STATUS_OPTIONS.includes(dataReference?.status)
      ? dataReference.status
      : "unknown";
    const traceDigest = String(trace.digest || "");
    const traceDigestMarkup = traceDigest
      ? `<dd id="workload-trace-digest-value" aria-describedby="workload-trace-digest-description"><span id="workload-trace-digest-description" hidden>${escapeHtml(traceDigest)}</span><code aria-hidden="true">${escapeHtml(shortDigest(traceDigest))}</code></dd>`
      : "<dd><span>Unavailable</span></dd>";
    const sourceContent = sources || `<p class="trace-intro" role="status">No source references are linked. Availability and outcomes remain unknown.</p>`;
    const title = sellerFailure
      ? "Seller-reported failure preview"
      : inconclusivePreview
        ? "Inconclusive verifier preview"
        : duplicateDispositionPreview
          ? "Duplicate settlement retry preview"
        : dataStatusPreview
          ? `Data reference preview: ${dataStatus}`
          : "One workload, source-owned records";
    const tag = sellerFailure
      ? "Synthetic failure preview"
      : inconclusivePreview
        ? "Synthetic mock outcome"
        : duplicateDispositionPreview
          ? "Synthetic replay refusal"
        : dataStatusPreview
          ? "Synthetic status fixture"
          : "Synthetic trace";
    const interpretation = sellerFailure
      ? "This fixed seller-signed example is linked to the approved synthetic deal. HyperAI did not execute work to create this preview; the report does not prove whether a real workload ran and establishes no refund or payment entitlement."
      : inconclusivePreview
        ? "This fixed synthetic trace maps an inconclusive local mock verdict to an in-memory hold-for-dispute disposition. It is not an actual verifier decision, dispute, funds hold, or payment authorization."
        : duplicateDispositionPreview
          ? "This fixed in-memory preview shows one existing mock disposition and the same-deal retry refusal. It creates no second settlement receipt, writes no durable ledger, and moves no funds."
        : dataStatusPreview
          ? `This fixed synthetic ${dataStatus} status is a display fixture. No Data catalog or file was read; it does not report actual availability, a source conflict, or an access decision.`
          : "Market owns the request and receipt contracts. Data is linked only when its verified local manifest is present. Eval, Fleet, and Trade are advisory context; their presence does not show that this workload ran.";
    const sellerFailureAction = sellerFailure || duplicateDispositionPreview || dataStatusPreview
      ? button("Return to successful trace", "load-workload-trace", "button button-secondary")
      : button("Preview seller-reported failure", "preview-seller-failure", "button button-secondary");
    const inconclusiveAction = inconclusivePreview
      ? button("Return to successful trace", "load-workload-trace", "button button-secondary")
      : button("Preview inconclusive mock outcome", "preview-inconclusive-outcome", "button button-secondary");
    const refreshAction = sellerFailure
      ? button("Refresh failure preview", "preview-seller-failure", "button button-secondary")
      : inconclusivePreview
        ? button("Refresh inconclusive preview", "preview-inconclusive-outcome", "button button-secondary")
        : duplicateDispositionPreview
          ? button("Refresh replay refusal preview", "preview-duplicate-disposition", "button button-secondary")
        : dataStatusPreview
          ? button(`Refresh ${dataStatus} Data preview`, `preview-data-status-${dataStatus}`, "button button-secondary")
          : button("Refresh trace", "load-workload-trace", "button button-secondary");
    return `<section class="panel workload-trace-panel" aria-labelledby="workload-trace-heading"><div class="panel-heading"><div><p class="eyebrow">CROSS-MODULE RECORD MAP</p><h2 id="workload-trace-heading" tabindex="-1">${escapeHtml(title)}</h2></div><span class="tag tag-caution">${escapeHtml(tag)}</span></div><dl class="trace-summary"><div><dt><span>Workload</span></dt><dd><strong>${escapeHtml(trace.intent?.body?.workload_id || "Unavailable")}</strong></dd></div><div><dt><span>Intent revision</span></dt><dd><strong>${escapeHtml(trace.intent?.body?.revision ?? "Unknown")}</strong></dd></div><div><dt><span>Trace digest</span></dt>${traceDigestMarkup}</div></dl><div class="trace-source-grid">${sourceContent}</div><div class="trace-claim"><strong><span class="visually-hidden">Claim ceiling: </span>${escapeHtml(trace.claim_ceiling || "Claim ceiling unavailable")}</strong><p>${escapeHtml(trace.nonclaims || "Nonclaims unavailable")}</p><small>${escapeHtml(interpretation)}</small></div><div class="trace-actions">${sellerFailureAction}${inconclusiveAction}${refreshAction}</div>${traceScenarioActions(button)}</section>`;
  }

  Object.defineProperty(globalThis, "HyperAIWorkloadTraceView", {
    configurable: false,
    enumerable: false,
    value: Object.freeze({ renderPanel }),
    writable: false,
  });
})();
