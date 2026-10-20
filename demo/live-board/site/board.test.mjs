import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
const { validate, freshness, render } = createRequire(import.meta.url)("./board.js");

const board = {
  schema_version: "hyperai.demo.live-board.v1", generated_at: "2026-10-20T12:00:00.000Z", board_digest: "d".repeat(64),
  observed_at_min: "2026-10-20T10:00:00Z", observed_at_max: "2026-10-20T11:00:00Z", row_count: 1,
  source: { snapshot_digest: "s".repeat(64), total_feed_rows: 10 }, claim_ceiling: "ceiling", nonclaims: ["<img src=x onerror=alert(1)>"],
  summary: [{ gpu_model: "<b>X</b>", pricing_type: "spot", offer_count: 1, min_usd_per_hour: "1", median_usd_per_hour: "1", max_usd_per_hour: "1" }],
  rows: [{ provider: "p", gpu_model: "<b>X</b>", pricing_type: "spot", region: "r", price_usd_per_hour: "1", observed_at: "t", source_url: "https://example.com/p" }],
};

test("validate rejects wrong schema and non-https sources", () => {
  assert.equal(validate(board), true);
  assert.equal(validate({ ...board, schema_version: "x" }), false);
  assert.equal(validate({ ...board, rows: [{ ...board.rows[0], source_url: "javascript:alert(1)" }] }), false);
});

test("freshness flags stale data after 24 hours", () => {
  const fresh = freshness(board, Date.parse("2026-10-20T20:00:00Z"));
  const stale = freshness(board, Date.parse("2026-10-22T12:00:00Z"));
  assert.equal(fresh.stale, false);
  assert.equal(stale.stale, true);
  assert.match(stale.text, /may be stale/);
});

function element() {
  return { children: [], textContent: "", className: "", classList: { toggle() {} }, appendChild(c) { this.children.push(c); },
    replaceChildren(...c) { this.children = c; } };
}

test("render writes untrusted strings only via textContent", () => {
  const nodes = { freshness: element(), nonclaims: element(), ceiling: element(), provenance: element(), summary: element(), rows: element() };
  const created = [];
  const doc = {
    getElementById: (id) => nodes[id],
    querySelector: (sel) => (sel.startsWith("#summary") ? nodes.summary : nodes.rows),
    createElement: (tag) => { const e = element(); e.tag = tag; created.push(e); return e; },
  };
  render(board, doc, Date.parse("2026-10-20T13:00:00Z"));
  assert.equal(nodes.nonclaims.children[0].textContent, "<img src=x onerror=alert(1)>");
  assert.ok(created.every((e) => !("innerHTML" in e)));
  assert.equal(nodes.summary.children[0].children[0].textContent, "<b>X</b>");
});
