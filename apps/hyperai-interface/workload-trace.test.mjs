import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { runInNewContext } from "node:vm";

const source = await readFile(new URL("./workload-trace.js", import.meta.url), "utf8");
const appSource = await readFile(new URL("./app.js", import.meta.url), "utf8");
const indexSource = await readFile(new URL("./index.html", import.meta.url), "utf8");
const context = {};
runInNewContext(source, context, { filename: "workload-trace.js" });
const { renderPanel } = context.HyperAIWorkloadTraceView;

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>\"']/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "\"": "&quot;",
    "'": "&#39;",
  })[character]);
}

function accessibleText(html) {
  return html
    .replace(/<(span|code)\b[^>]*\baria-hidden="true"[^>]*>.*?<\/\1>/gs, " ")
    .replace(/<[^>]*>/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, " ")
    .trim();
}

const helpers = {
  escapeHtml,
  shortDigest: (digest) => digest ? `${digest.slice(0, 12)}…${digest.slice(-8)}` : "Not available",
  button: (label, action, className) => `<button class="${escapeHtml(className)}" data-action="${escapeHtml(action)}">${escapeHtml(label)}</button>`,
};

function reference(overrides = {}) {
  return {
    owner_id: "hyperai-data",
    schema_version: "hyperai.synthetic.data.v1",
    record_id: "synthetic-data-reference",
    digest: null,
    status: "missing",
    link_basis: "optional_buyer_selected_reference",
    claim_ceiling: "synthetic_input_provenance_only_not_job_execution",
    ...overrides,
  };
}

function trace(references, overrides = {}) {
  return {
    digest: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    intent: { body: { workload_id: "synthetic-workload-1", revision: 2 } },
    references,
    claim_ceiling: "Digest-bound local synthetic projection only",
    nonclaims: "No execution occurred; no funds moved.",
    ...overrides,
  };
}

test("renders unresolved and failed references as explicit readable states", () => {
  const html = renderPanel({
    trace: trace([
      reference({ status: "missing" }),
      reference({ record_id: "conflicting-input", status: "conflicting" }),
      reference({ record_id: "denied-input", status: "denied" }),
      reference({ record_id: "stale-input", status: "stale" }),
      reference({
        owner_id: "hyperai-market",
        schema_version: "hyperai.compute-market.fulfillment-failure.v1",
        record_id: "synthetic-attempt-1",
        digest: "failure-digest",
        status: "fulfillment_failed",
        link_basis: "seller_signed_failure_report_bound_to_approved_deal",
        claim_ceiling: "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence",
      }),
    ]),
    ...helpers,
  });

  const text = accessibleText(html);
  for (const status of ["missing", "conflicting", "denied", "stale", "fulfillment failed"]) {
    assert.ok(text.includes(`Status: ${status}`), `missing accessible status ${status}`);
  }
  assert.ok(html.includes("No separate source digest"));
  assert.doesNotMatch(html, /<(?:span|code)\b[^>]*\baria-label=/);
  assert.ok(html.includes("optional buyer selected reference"));
  assert.ok(html.includes("seller attributed failure report only not independent execution or payment evidence"));
  assert.ok(html.includes("No execution occurred; no funds moved."));
});

test("uses named semantic structure and exposes full digests accessibly", () => {
  const sourceDigest = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
  const html = renderPanel({
    trace: trace([reference({ digest: sourceDigest, status: "bound" })]),
    ...helpers,
  });
  const text = accessibleText(html);

  assert.match(html, /<section class="panel workload-trace-panel" aria-labelledby="workload-trace-heading">/);
  assert.match(html, /<h2 id="workload-trace-heading" tabindex="-1">One workload, source-owned records<\/h2>/);
  assert.match(html, /<dl class="trace-summary"><div><dt><span>Workload<\/span><\/dt><dd><strong>synthetic-workload-1<\/strong><\/dd>/);
  assert.match(html, /<dt><span>Intent revision<\/span><\/dt><dd><strong>2<\/strong><\/dd>/);
  assert.ok(text.includes(`Source-provided digest: ${sourceDigest}`));
  assert.ok(text.includes("Trace digest"));
  assert.ok(text.includes("Claim ceiling: Digest-bound local synthetic projection only"));
  const traceDigest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
  assert.ok(html.includes(`<dd id="workload-trace-digest-value" aria-describedby="workload-trace-digest-description"><span id="workload-trace-digest-description" hidden>${traceDigest}</span><code aria-hidden="true">${traceDigest.slice(0, 12)}…${traceDigest.slice(-8)}</code></dd>`));
  assert.ok(html.includes(`<li class="trace-reference" aria-describedby="workload-trace-source-digest-0-0">`));
  assert.ok(html.includes(`<span id="workload-trace-source-digest-0-0" hidden>Source-provided digest: ${sourceDigest}</span><code aria-hidden="true">${sourceDigest.slice(0, 12)}…${sourceDigest.slice(-8)}</code>`));
  assert.doesNotMatch(html, /<code[^>]*title=/);
  assert.doesNotMatch(html, /<(?:span|code)\b[^>]*\baria-label=/);

  const empty = renderPanel({ trace: null, ...helpers });
  assert.match(empty, /<section class="panel workload-trace-panel" aria-labelledby="workload-trace-heading">/);
  assert.match(empty, /<h2 id="workload-trace-heading" tabindex="-1">Workload trace<\/h2>/);
});

test("links each known trace owner to its workspace and labels static inventory state", () => {
  const html = renderPanel({
    trace: trace([
      reference({ owner_id: "hyperai-data" }),
      reference({ owner_id: "hyperai-eval" }),
      reference({ owner_id: "hyperai-fleet" }),
      reference({ owner_id: "hyperai-market" }),
      reference({ owner_id: "hyperai-trade" }),
    ]),
    connections: [
      { module_id: "compute_market", state: "local_synthetic" },
      { module_id: "eval", state: "local_synthetic_harness_only" },
      { module_id: "fleet", state: "local_synthetic_fixture_only" },
      { module_id: "trade", state: "local_synthetic_research_only" },
      { module_id: "data", state: "local_synthetic_workspace" },
    ],
    ...helpers,
  });

  const text = accessibleText(html);
  for (const page of ["data", "eval", "fleet", "market", "trade"]) {
    assert.ok(html.includes(`data-page="${page}"`), `missing ${page} workspace link`);
  }
  assert.ok(html.includes("Open Data workspace"));
  assert.ok(text.includes("Static inventory · not a live probe local synthetic"));
  assert.ok(text.includes("Static inventory · not a live probe local synthetic workspace"));
  assert.ok(html.includes("local synthetic harness only"));
  assert.ok(html.includes("local synthetic fixture only"));
  assert.ok(html.includes("local synthetic research only"));
  assert.ok(!html.includes("No inventory entry"));
});

test("keeps unavailable and source-controlled inventory states explicit and escaped", () => {
  const unavailable = renderPanel({
    trace: trace([reference({ owner_id: "hyperai-market" })]),
    connections: undefined,
    ...helpers,
  });
  assert.ok(accessibleText(unavailable).includes("Static inventory · not a live probe Unavailable"));

  const escaped = renderPanel({
    trace: trace([reference({ owner_id: "hyperai-market" })]),
    connections: [{ module_id: "compute_market", state: "<img src=x>" }],
    ...helpers,
  });
  assert.ok(escaped.includes("&lt;img src=x&gt;"));
  assert.ok(!escaped.includes("<img src=x>"));

  const missingDataEntry = renderPanel({
    trace: trace([reference({ owner_id: "hyperai-data" })]),
    connections: [],
    ...helpers,
  });
  assert.ok(accessibleText(missingDataEntry).includes("Static inventory · not a live probe No inventory entry"));
});

test("labels a seller-failure preview and states its execution and payment limits", () => {
  const html = renderPanel({
    trace: trace([
      reference({
        owner_id: "hyperai-market",
        schema_version: "hyperai.compute-market.fulfillment-failure.v1",
        record_id: "synthetic-attempt-1",
        digest: "failure-digest",
        status: "fulfillment_failed",
        link_basis: "seller_signed_failure_report_bound_to_approved_deal",
        claim_ceiling: "seller_attributed_failure_report_only_not_independent_execution_or_payment_evidence",
      }),
    ], {
      state_slice: "hyperai.bazaar.workload_trace_seller_failure.v1",
      claim_ceiling: "digest-bound local synthetic seller-attributed failure projection",
      nonclaims: "No execution or payment entitlement is established.",
    }),
    ...helpers,
  });

  assert.ok(html.includes("Seller-reported failure preview"));
  assert.ok(html.includes("Synthetic failure preview"));
  assert.ok(html.includes("HyperAI did not execute work to create this preview"));
  assert.ok(html.includes("does not prove whether a real workload ran"));
  assert.ok(html.includes("Return to successful trace"));
  assert.ok(html.includes('data-action="load-workload-trace"'));
  assert.ok(html.includes('data-action="preview-seller-failure"'));
  assert.ok(appSource.includes("/api/v1/market/workload-trace/failure-preview"));
});

test("labels the inconclusive mock preview and avoids real dispute or payment claims", () => {
  const html = renderPanel({
    trace: trace([
      reference({
        owner_id: "hyperai-market",
        record_id: "synthetic-mock-verifier-decision-v1",
        status: "mock_verdict_inconclusive",
        claim_ceiling: "local_fixture_verdict_only_not_independent_correctness_evidence",
      }),
      reference({
        owner_id: "hyperai-market",
        record_id: "synthetic-mock-disposition-v1",
        status: "mock_disposition_hold_for_dispute",
        claim_ceiling: "no_payment_or_entitlement_claim",
      }),
    ], {
      state_slice: "hyperai.bazaar.workload_trace_inconclusive_served.v1",
      claim_ceiling: "digest-bound local synthetic inconclusive mock-outcome preview",
      nonclaims: "No actual verifier, dispute, funds hold, or payment is established.",
    }),
    ...helpers,
  });

  assert.ok(html.includes("Inconclusive verifier preview"));
  assert.ok(html.includes("Synthetic mock outcome"));
  assert.ok(accessibleText(html).includes("Status: mock verdict inconclusive"));
  assert.ok(html.includes("mock disposition hold for dispute"));
  assert.ok(html.includes("not an actual verifier decision, dispute, funds hold, or payment authorization"));
  assert.ok(html.includes('data-action="load-workload-trace"'));
  assert.ok(html.includes('data-action="preview-seller-failure"'));
  assert.ok(html.includes('data-action="preview-inconclusive-outcome"'));
  assert.ok(appSource.includes("/api/v1/market/workload-trace/inconclusive-preview"));
});

test("labels duplicate disposition replay refusal without a second receipt claim", () => {
  const html = renderPanel({
    trace: trace([
      reference({
        owner_id: "hyperai-market",
        schema_version: "hyperai.compute-market.mock-settlement.v1",
        record_id: "synthetic-mock-disposition-v1",
        status: "mock_disposition_release_to_seller",
        link_basis: "covered_by_the_enclosing_local_market_receipt_digest",
        claim_ceiling: "no_payment_or_entitlement_claim",
      }),
      reference({
        owner_id: "hyperai-market",
        schema_version: "hyperai.compute-market.mock-settlement.v1",
        record_id: "synthetic-mock-disposition-replay-attempt-v1",
        status: "mock_settlement_replay_refused",
        link_basis: "same_deal_retry_rejected_by_in_memory_mock_ledger",
        claim_ceiling: "replay_refusal_only_no_second_mock_receipt_or_funds_move",
      }),
      reference({
        owner_id: "hyperai-market",
        schema_version: "hyperai.compute-market.local-market-receipt.v1",
        record_id: "synthetic-local-market-receipt-v1",
        digest: "receipt-digest",
        status: "synthetic_source_record",
      }),
    ], {
      state_slice: "hyperai.bazaar.workload_trace_duplicate_disposition_served.v1",
      claim_ceiling: "digest-bound local synthetic duplicate disposition replay refusal preview",
      nonclaims: "No second mock settlement receipt, durable idempotency, payment, or funds movement.",
    }),
    ...helpers,
  });

  assert.ok(html.includes("Duplicate settlement retry preview"));
  assert.ok(html.includes("Synthetic replay refusal"));
  assert.ok(html.includes("mock disposition release to seller"));
  assert.ok(html.includes("mock settlement replay refused"));
  assert.ok(html.includes("no second mock receipt or funds move"));
  assert.ok(html.includes("creates no second settlement receipt"));
  assert.ok(html.includes("writes no durable ledger"));
  assert.ok(html.includes('data-action="load-workload-trace"'));
  assert.ok(html.includes('data-action="preview-duplicate-disposition"'));
  assert.ok(html.includes("Preview synthetic edge cases"));
  assert.ok(appSource.includes("/api/v1/market/workload-trace/duplicate-disposition-preview"));
  assert.ok(appSource.includes('target.dataset.action === "preview-duplicate-disposition"'));
});

test("labels fixed unresolved Data previews as synthetic and keeps source reads closed", () => {
  for (const status of ["missing", "conflicting", "denied"]) {
    const html = renderPanel({
      trace: trace([
        reference({
          status,
          link_basis: "fixed_synthetic_status_fixture_no_data_source_read",
          claim_ceiling: "synthetic_data_status_fixture_only_no_source_read_conflict_or_access_decision",
        }),
      ], {
        state_slice: "hyperai.bazaar.workload_trace_data_status_served.v1",
        claim_ceiling: "digest-bound local synthetic unresolved Data status preview",
        nonclaims: "This is a fixed synthetic Data status; no Data source was read.",
      }),
      ...helpers,
    });

    assert.ok(html.includes(`Data reference preview: ${status}`));
    assert.ok(html.includes("Synthetic status fixture"));
    assert.ok(accessibleText(html).includes(`Status: ${status}`));
    assert.ok(html.includes("fixed synthetic status fixture no data source read"));
    assert.ok(html.includes("No Data catalog or file was read"));
    assert.ok(html.includes("Preview synthetic edge cases"));
    assert.ok(html.includes('data-action="load-workload-trace"'));
    for (const option of ["missing", "conflicting", "denied"]) {
      assert.ok(html.includes(`data-action="preview-data-status-${option}"`));
    }
  }

  assert.ok(appSource.includes("/api/v1/market/workload-trace/data-status-preview/"));
  assert.ok(appSource.includes('target.dataset.action.startsWith("preview-data-status-")'));
});

test("keeps inconclusive verdict, dispute hold, and claim limits visible", () => {
  const html = renderPanel({
    trace: trace([
      reference({ owner_id: "hyperai-market", record_id: "verifier", status: "mock_verdict_inconclusive", claim_ceiling: "local_fixture_verdict_only_not_independent_correctness_evidence" }),
      reference({ owner_id: "hyperai-market", record_id: "disposition", status: "mock_disposition_hold_for_dispute", claim_ceiling: "no_payment_or_entitlement_claim" }),
    ]),
    ...helpers,
  });

  assert.ok(accessibleText(html).includes("Status: mock verdict inconclusive"));
  assert.ok(html.includes("mock disposition hold for dispute"));
  assert.ok(html.includes("local fixture verdict only not independent correctness evidence"));
  assert.ok(html.includes("no payment or entitlement claim"));
  assert.equal((html.match(/No separate source digest/g) || []).length, 2);
  assert.ok(!html.includes("Source-provided digest undefined"));
});

test("keeps rejected mock verdict, buyer refund, and claim limits visible", () => {
  const html = renderPanel({
    trace: trace([
      reference({ owner_id: "hyperai-market", record_id: "verifier", status: "mock_verdict_rejected", claim_ceiling: "local_fixture_verdict_only_not_independent_correctness_evidence" }),
      reference({ owner_id: "hyperai-market", record_id: "disposition", status: "mock_disposition_refund_to_buyer", claim_ceiling: "no_payment_or_entitlement_claim" }),
    ]),
    ...helpers,
  });

  assert.ok(accessibleText(html).includes("Status: mock verdict rejected"));
  assert.ok(html.includes("mock disposition refund to buyer"));
  assert.ok(html.includes("local fixture verdict only not independent correctness evidence"));
  assert.ok(html.includes("no payment or entitlement claim"));
  assert.equal((html.match(/No separate source digest/g) || []).length, 2);
});

test("states when a trace has no linked source references", () => {
  const html = renderPanel({ trace: trace([]), ...helpers });

  assert.ok(html.includes('role="status"'));
  assert.ok(html.includes("No source references are linked. Availability and outcomes remain unknown."));
});

test("uses an error fallback without stale reference markup", () => {
  const html = renderPanel({ trace: null, error: "The local HyperAI contract operation could not complete.", ...helpers });

  assert.ok(html.includes("The read-only trace could not be loaded"));
  assert.ok(html.includes('data-action="load-workload-trace"'));
  assert.ok(!html.includes("synthetic-data-reference"));
  assert.ok(!html.includes("One workload, source-owned records"));

  const inconclusiveError = renderPanel({
    trace: null,
    mode: "inconclusive-preview",
    error: "The local HyperAI contract operation could not complete.",
    ...helpers,
  });
  assert.ok(inconclusiveError.includes('data-action="preview-inconclusive-outcome"'));
  assert.ok(inconclusiveError.includes('data-action="load-workload-trace"'));
  assert.ok(!inconclusiveError.includes("mock disposition hold for dispute"));

  const dataStatusError = renderPanel({
    trace: null,
    mode: "data-status-denied",
    error: "The local HyperAI contract operation could not complete.",
    ...helpers,
  });
  assert.ok(dataStatusError.includes('data-action="preview-data-status-denied"'));
  assert.ok(dataStatusError.includes("This fixed Data status preview performs no Data read"));
  assert.ok(!dataStatusError.includes("synthetic-data-reference"));

  const duplicateError = renderPanel({
    trace: null,
    mode: "duplicate-disposition-preview",
    error: "The local HyperAI contract operation could not complete.",
    ...helpers,
  });
  assert.ok(duplicateError.includes('data-action="preview-duplicate-disposition"'));
  assert.ok(duplicateError.includes("creates no second receipt or funds movement"));
  assert.ok(!duplicateError.includes("synthetic-mock-disposition-replay-attempt-v1"));
});

test("escapes source-controlled reference and trace text", () => {
  const html = renderPanel({
    trace: trace([
      reference({
        record_id: "<script>alert(1)</script>",
        status: "denied_<img src=x>",
        link_basis: "<b>untrusted</b>",
        claim_ceiling: "<svg onload=alert(1)>",
      }),
    ], { intent: { body: { workload_id: "<iframe>id</iframe>", revision: 1 } } }),
    ...helpers,
  });

  assert.ok(html.includes("&lt;script&gt;alert(1)&lt;/script&gt;"));
  assert.ok(html.includes("denied &lt;img src=x&gt;"));
  assert.ok(html.includes("&lt;b&gt;untrusted&lt;/b&gt;"));
  assert.ok(html.includes("&lt;svg onload=alert(1)&gt;"));
  assert.ok(html.includes("&lt;iframe&gt;id&lt;/iframe&gt;"));
  assert.ok(!html.includes("<script>alert(1)</script>"));
  assert.ok(!html.includes("<svg onload=alert(1)>"));
});

test("the application delegates to the renderer loaded before app.js", () => {
  const rendererScript = indexSource.indexOf('src="/workload-trace.js"');
  const applicationScript = indexSource.indexOf('src="/app.js"');
  assert.ok(rendererScript >= 0 && applicationScript > rendererScript);

  function stubElement() {
    return {
      classList: { contains: () => false, toggle: () => {} },
      setAttribute: () => {},
      removeAttribute: () => {},
      toggleAttribute: () => {},
      addEventListener: () => {},
      contains: () => false,
    };
  }

  const document = {
    activeElement: null,
    getElementById: () => stubElement(),
    addEventListener: () => {},
    querySelectorAll: () => [],
  };
  const matchMedia = { matches: false, addEventListener: () => {} };
  const appContext = {
    document,
    fetch: () => new Promise(() => {}),
    window: {
      HyperAIWorkloadTraceView: context.HyperAIWorkloadTraceView,
      matchMedia: () => matchMedia,
      setInterval: () => 1,
    },
  };
  runInNewContext(appSource, appContext, { filename: "app.js" });
  appContext.fixtureTrace = trace([
    reference({
      owner_id: "hyperai-market",
      status: "mock_verdict_inconclusive",
      claim_ceiling: "no_payment_or_entitlement_claim",
    }),
  ]);
  const rendered = runInNewContext(
    "state.status = { connections: [{ module_id: 'compute_market', state: 'local_synthetic' }] }; state.workloadTrace = fixtureTrace; renderWorkloadTracePanel()",
    appContext,
  );

  assert.ok(accessibleText(rendered).includes("Status: mock verdict inconclusive"));
  assert.ok(rendered.includes("no payment or entitlement claim"));
  assert.ok(accessibleText(rendered).includes("Static inventory · not a live probe local synthetic"));
});
