/* Pure rendering for the shared, digest-bound local buyer supply projection. */
(() => {
  const SCHEMA = "hyperai.compute-market.supply-comparison.v1";

  function escapeHtml(value) {
    return String(value ?? "").replace(/[&<>\"']/g, (character) => ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      "\"": "&quot;",
      "'": "&#39;",
    })[character]);
  }

  function formatMoney(value) {
    if (!Number.isSafeInteger(value) || value < 0) return "Unknown";
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value / 1_000_000);
  }

  function humanize(value) {
    return String(value ?? "unknown").replaceAll("_", " ");
  }

  function renderPanel({ comparison, error = "", loading = false, quoteLoading = false, quoteError = "", quotePreviewReady = false } = {}) {
    if (loading) {
      return `<section class="panel supply-panel" aria-labelledby="supply-heading"><p class="eyebrow">BUYER SUPPLY</p><h2 id="supply-heading">Loading local comparison</h2><p role="status" aria-live="polite">Reading the fixed synthetic market projection. No seller or registry is contacted.</p></section>`;
    }
    if (error) {
      return `<section class="panel supply-panel" aria-labelledby="supply-heading"><p class="eyebrow">BUYER SUPPLY</p><h2 id="supply-heading">Comparison unavailable</h2><p class="supply-error" role="status" aria-live="polite">${escapeHtml(error)}</p><p>The read-only local comparison could not be loaded. No market action was attempted.</p></section>`;
    }
    const body = comparison?.body;
    if (!body || body.schema_version !== SCHEMA
      || !body.catalog?.body || !Array.isArray(body.offer_assessments)
      || !Array.isArray(body.advisory_observations)) {
      return `<section class="panel supply-panel" aria-labelledby="supply-heading"><p class="eyebrow">BUYER SUPPLY</p><h2 id="supply-heading">Comparison unavailable</h2><p role="status" aria-live="polite">The service returned an unsupported or incomplete comparison contract.</p></section>`;
    }

    const request = body.buyer_request || {};
    const catalog = body.catalog.body;
    const entries = new Map((catalog.entries || []).map((entry) => [entry.signed_offer?.offer?.offer_id, entry]));
    const observations = body.advisory_observations;
    const selectedOfferId = body.selected_offer_id;
    const rows = body.offer_assessments.map((assessment) => {
      const offer = entries.get(assessment.offer_id)?.signed_offer?.offer;
      if (!offer) return "";
      const reasons = (assessment.ineligibility_reasons || []).map(humanize);
      const eligibility = assessment.eligible
        ? `<span class="tag tag-good">Eligible</span>`
        : `<span class="tag tag-caution">Not eligible</span><small>${escapeHtml(reasons.join(" · ") || "Constraint not met")}</small>`;
      const selected = assessment.selected && assessment.offer_id === selectedOfferId
        ? `<span class="supply-selected">Selected by asking price</span>`
        : "";
      return `<tr><td><strong>${escapeHtml(offer.offer_id)}</strong><small>${escapeHtml((assessment.source_ids || []).join(" · ") || "Source unknown")}</small><small>Signed offer digest ${escapeHtml(assessment.offer_digest || "not supplied")}</small></td><td><strong>${escapeHtml(offer.resource_model)}</strong><small>${escapeHtml(offer.region)} · ${escapeHtml(offer.capacity_units)} seller-declared units</small></td><td class="number-cell">${formatMoney(assessment.asking_rate_microusd_per_unit_hour)}<small>asking / unit-hour</small></td><td class="number-cell">${formatMoney(assessment.asking_total_microusd)}<small>checked request total · not a quote</small></td><td>${eligibility}${selected}</td></tr>`;
    }).join("");
    const selectedSummary = selectedOfferId
      ? `Selected offer: ${escapeHtml(selectedOfferId)} · lowest asking rate among offers that satisfy the buyer request.`
      : "No signed offer satisfies every buyer constraint.";
    const quoteAction = selectedOfferId
      ? `<div class="import-actions supply-quote-action"><button class="button button-primary" type="button" data-action="preview-supply-quote" aria-label="Preview quote for ${escapeHtml(selectedOfferId)}" ${quoteLoading ? "disabled aria-busy=\"true\"" : ""}>${quoteLoading ? "Building quote preview…" : quotePreviewReady ? "Refresh quote preview" : "Preview seller quote"}</button><span>Uses this comparison digest · synthetic preview only · approval stays separate</span>${quoteError ? `<p class="supply-error" role="status" aria-live="polite" aria-label="${escapeHtml(quoteError)}" tabindex="-1">${escapeHtml(quoteError)}</p>` : ""}</div>`
      : "";
    const observationCards = observations.length
      ? observations.map((observation) => {
        const item = observation.body || {};
        return `<article class="supply-observation"><div><span class="signal-label">ADVISORY OBSERVATION</span><span class="tag tag-caution">Not an offer</span></div><strong>${escapeHtml(item.offer_id || "Offer unknown")}</strong><dl><div><dt>Estimated rate</dt><dd>${item.estimated_rate_microusd_per_unit_hour == null ? "Unknown" : `${formatMoney(item.estimated_rate_microusd_per_unit_hour)} / unit-hour`}</dd></div><div><dt>Estimated availability</dt><dd>${item.estimated_available_units == null ? "Unknown" : `${escapeHtml(item.estimated_available_units)} units`}</dd></div><div><dt>Source and observed time</dt><dd>${escapeHtml(item.source_id || "Source unknown")} · ${escapeHtml(item.observed_at || "Time unknown")}</dd></div></dl><small>Digest ${escapeHtml(observation.digest || "not supplied")}</small></article>`;
      }).join("")
      : `<p class="supply-muted">No advisory observations were supplied. Their absence does not change offer eligibility.</p>`;

    return `<section class="supply-panel" aria-labelledby="supply-heading">
      <div class="supply-intro"><div><p class="eyebrow">BUYER SUPPLY / LOCAL SYNTHETIC</p><h2 id="supply-heading">Compare the request with signed offers</h2><p>Eligibility comes from the verified listing and buyer constraints. Market observations stay advisory and separate.</p></div><span class="tag tag-caution">No live supply</span></div>
      <div class="supply-request panel"><div><p class="eyebrow">BUYER CONSTRAINTS</p><h3>${escapeHtml(request.resource_model || "Resource unknown")} · ${escapeHtml(request.region || "Region unknown")}</h3></div><dl><div><dt>Requested</dt><dd>${escapeHtml(request.units)} units · ${escapeHtml(request.duration_hours)} hours</dd></div><div><dt>Maximum asking rate</dt><dd>${formatMoney(request.max_rate_microusd_per_unit_hour)} / unit-hour</dd></div><div><dt>Total spend cap</dt><dd>${formatMoney(request.max_total_microusd)}</dd></div></dl></div>
      <p class="supply-selection" role="status">${selectedSummary}</p>
      ${quoteAction}
      <div class="table-panel supply-table"><div class="table-toolbar"><div><p class="eyebrow">SIGNED CATALOG</p><h3>${body.offer_assessments.length} assessed offers <span>· ${escapeHtml(catalog.source_count)} sources · ${escapeHtml(catalog.duplicate_count)} duplicates collapsed</span></h3></div><span class="snapshot-time">Compared at<br /><strong>${escapeHtml(body.compared_at || "Unknown")}</strong></span></div><div class="table-scroll"><table><caption>Offer eligibility and seller asking terms for the fixed buyer request</caption><thead><tr><th scope="col">Offer / source</th><th scope="col">Resource</th><th scope="col">Seller asking</th><th scope="col">Request total</th><th scope="col">Buyer fit</th></tr></thead><tbody>${rows || `<tr><td colspan="5">No signed offers in this catalog.</td></tr>`}</tbody></table></div><div class="table-foot"><code class="supply-comparison-digest">COMPARISON ${escapeHtml(comparison.digest || "digest unavailable")}</code><code>CATALOG ${escapeHtml(catalog.digest || "digest unavailable")}</code><span>Freshness: ${escapeHtml(body.freshness_status || "unknown")}</span></div></div>
      <div class="section-heading"><div><p class="eyebrow">ADVISORY CONTEXT</p><h3>Market estimates never set offer eligibility</h3></div></div><div class="supply-observation-grid">${observationCards}</div>
      <p class="supply-claim">${escapeHtml(body.claim_ceiling || "Synthetic comparison only")}. ${escapeHtml(body.nonclaims || "No execution, availability, quote, or payment claim.")}</p>
      <p class="supply-effects">${escapeHtml(body.effects || "Read-only local projection.")}</p>
    </section>`;
  }

  globalThis.HyperAIMarketSupplyComparison = Object.freeze({ renderPanel });
})();
