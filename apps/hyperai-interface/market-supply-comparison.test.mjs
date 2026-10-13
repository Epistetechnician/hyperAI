import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { runInNewContext } from "node:vm";

const source = await readFile(new URL("./market-supply-comparison.js", import.meta.url), "utf8");
const context = {};
runInNewContext(source, context, { filename: "market-supply-comparison.js" });
const { renderPanel } = context.HyperAIMarketSupplyComparison;

function comparison(overrides = {}) {
  return {
    digest: "comparison-digest",
    body: {
      schema_version: "hyperai.compute-market.supply-comparison.v1",
      buyer_request: {
        resource_model: "GPU-H100",
        region: "us-east",
        units: 2,
        duration_hours: 3,
        max_rate_microusd_per_unit_hour: 2_500_000,
        max_total_microusd: 15_000_000,
      },
      compared_at: "2026-09-24T12:00:00Z",
      selected_offer_id: "offer-gpu-b",
      catalog: {
        digest: "catalog-digest",
        body: {
          source_count: 2,
          duplicate_count: 1,
          entries: [{
            source_ids: ["synthetic-registry-beta"],
            signed_offer: { offer: {
              offer_id: "offer-gpu-b",
              resource_model: "GPU-H100",
              region: "us-east",
              capacity_units: 8,
            } },
          }],
        },
      },
      offer_assessments: [{
        offer_id: "offer-gpu-b",
        asking_rate_microusd_per_unit_hour: 1_800_000,
        asking_total_microusd: 10_800_000,
        eligible: true,
        ineligibility_reasons: [],
        selected: true,
        source_ids: ["synthetic-registry-beta"],
      }],
      advisory_observations: [{
        digest: "observation-digest",
        body: {
          offer_id: "offer-gpu-b",
          source_id: "synthetic-market-observation-feed",
          observed_at: "2026-09-24T12:00:00Z",
          estimated_rate_microusd_per_unit_hour: 1_600_000,
          estimated_available_units: 4,
        },
      }],
      freshness_status: "not_evaluated",
      claim_ceiling: "fixed synthetic comparison only",
      nonclaims: "No execution or payment claim.",
      effects: "Read-only in-memory synthetic projection.",
      ...overrides,
    },
  };
}

test("renders buyer constraints, signed eligibility, and advisory provenance separately", () => {
  const html = renderPanel({ comparison: comparison() });
  for (const label of [
    "BUYER CONSTRAINTS",
    "SIGNED CATALOG",
    "Eligible",
    "Selected by asking price",
    "ADVISORY CONTEXT",
    "Not an offer",
    "synthetic-market-observation-feed",
    "Freshness: not_evaluated",
    "No execution or payment claim.",
    'data-action="preview-supply-quote"',
    "approval stays separate",
  ]) {
    assert.ok(html.includes(label), `expected rendered comparison to include ${label}`);
  }
  assert.equal((html.match(/data-action=/g) || []).length, 1);
});

test("shows quote-preview progress and errors only beside an eligible selected offer", () => {
  const loading = renderPanel({ comparison: comparison(), quoteLoading: true });
  assert.ok(loading.includes("Building quote preview…"));
  assert.ok(loading.includes('aria-busy="true"'));
  const failed = renderPanel({ comparison: comparison(), quoteError: "Comparison changed" });
  assert.ok(failed.includes("Comparison changed"));
  assert.ok(failed.includes('role="status"'));
  assert.ok(failed.includes('aria-live="polite"'));
  assert.ok(failed.includes('aria-label="Comparison changed"'));
  assert.ok(failed.includes('tabindex="-1"'));

  const noSelection = renderPanel({
    comparison: comparison({ selected_offer_id: null }),
  });
  assert.ok(!noSelection.includes('data-action="preview-supply-quote"'));
});

test("escapes source supplied labels and displays malformed service contracts explicitly", () => {
  const body = comparison();
  body.body.advisory_observations[0].body.source_id = "<img src=x onerror=alert(1)>";
  const html = renderPanel({ comparison: body });
  assert.ok(html.includes("&lt;img src=x onerror=alert(1)&gt;"));
  assert.ok(!html.includes("<img src=x onerror=alert(1)>"));

  const malformed = renderPanel({ comparison: { body: { schema_version: "future.v1" } } });
  assert.ok(malformed.includes("unsupported or incomplete comparison contract"));
});

test("loading and service errors stay visible without market actions", () => {
  assert.ok(renderPanel({ loading: true }).includes("Loading local comparison"));
  const failed = renderPanel({ error: "Service unavailable" });
  assert.ok(failed.includes("Service unavailable"));
  assert.ok(failed.includes("No market action was attempted"));
});
